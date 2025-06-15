use anyhow::{anyhow, bail, Context};
use clap::Parser;
use libpulse_binding::mainloop::standard::IterateResult;
use libpulse_binding::sample::{Format, Spec};
use libpulse_simple_binding::Simple;
use log::{debug, error, info};
use rodio::buffer::SamplesBuffer;
use rodio::cpal::traits::{HostTrait, StreamTrait};
use rodio::source::SineWave;
use rodio::{cpal, DeviceTrait, OutputStream, Sink, Source};
use std::process::Command;
use std::thread::sleep;
use std::time::{Duration, SystemTime};
use std::sync::mpsc::{channel, Receiver};

use libpulse_binding::context::Context as LibpulseContext;
use libpulse_binding::{
    context::State,
    mainloop::standard::Mainloop,
    proplist::Proplist,
};
use std::rc::Rc;
use std::cell::RefCell;

// In seconds
const DEBUG_INTERVAL_DEFAULT: u64 = 60;

#[derive(Parser, Debug)]
#[command(author="Rasmus Kirk", version, about = "Rustle - Keep your digital speakers from sleeping, using low sound signals", long_about = None)]
struct Args {
    /// Duration of each tone in seconds (0 for continual playback)
    #[arg(short = 'd', long, default_value_t = 120.0)]
    pulse_duration: f32,

    /// Frequency of the sine wave during pulses in Hz
    #[arg(short = 'f', long, default_value_t = 20.0)]
    frequency: f32,

    /// Amplitude of the sine wave (e.g., 0.01 for 1%)
    #[arg(short = 'a', long, default_value_t = 0.01)]
    amplitude: f32,

    /// Minutes of undetected sound until the tone plays
    #[arg(short = 's', long, default_value_t = 10)]
    minutes_of_silence: u64,
}

macro_rules! handle_err {
    ($result:expr) => {
        match $result {
            Ok(value) => value,
            Err(e) => {
                error!("Error: {e}");
                continue;
            }
        }
    };
}

fn is_playing_pipewire() -> anyhow::Result<bool> {
    let x = Command::new("sh")
        .arg("-c")
        .arg("pw-dump | grep '\"state\": \"running\"' | wc -l")
        .output()?
        .stdout;
    Ok(str::from_utf8(&x)?.trim() != "0")
}

fn get_default_sink() -> anyhow::Result<String> {
    // Create a new mainloop
    let mainloop = Rc::new(RefCell::new(
        Mainloop::new().expect("Failed to create mainloop"),
    ));

    // Create a new context
    let context = Rc::new(RefCell::new(
        LibpulseContext::new(&*mainloop.borrow(), "PulseContext").with_context(|| "Failed to create context")?,
    ));

    context
        .borrow_mut()
        .connect(None, libpulse_binding::context::FlagSet::NOFLAGS, None)
        .expect("Failed to connect context");

    // Wait for the context to be ready
    loop {
        match mainloop.borrow_mut().iterate(true) {
            IterateResult::Success(_) => (),
            IterateResult::Err(e) => panic!("Mainloop iteration failed: {}", e),
            IterateResult::Quit(_) => panic!("Mainloop quit unexpectedly"),
        }

        match context.borrow().get_state() {
            State::Ready => break,
            State::Failed | State::Terminated => panic!("Context connection failed"),
            _ => continue,
        }
    }

    // Flag to track when server info is retrieved
    let server_info_received = Rc::new(RefCell::new(false));
    let server_info_received_clone = server_info_received.clone();

    let default_sink_received = Rc::new(RefCell::new(None));
    let default_sink_received_clone = default_sink_received.clone();

    // Get server information (includes default sink)
    context
        .borrow_mut()
        .introspect()
        .get_server_info(move |server_info| {
            if let Some(default_sink) = &server_info.default_sink_name {
                *default_sink_received_clone.borrow_mut() = Some(default_sink.to_string());
                println!("Default Output Sink: {}", default_sink);
            } else {
                info!("No default output sink found");
            }
            *server_info_received_clone.borrow_mut() = true;
        });

    // Wait until server info is received
    while !*server_info_received.borrow() {
        match mainloop.borrow_mut().iterate(true) {
            IterateResult::Success(_) => (),
            IterateResult::Err(e) => bail!("Mainloop iteration failed: {}", e),
            IterateResult::Quit(_) => bail!("Mainloop quit unexpectedly"),
        }
    }

    // Clean up
    context.borrow_mut().disconnect();

    default_sink_received.borrow().clone().with_context(|| "No default sink found")
}

// fn get_pulse_sink() -> anyhow::Result<()> {
//     // Create a mainloop and context
//     let mut mainloop = Mainloop::new().unwrap();
//     let api = mainloop.get_api();
//     let context = Context::new(api, "default_sink_query").unwrap();

//     // Connect to PulseAudio
//     context.connect(None, FlagSet::NOFLAGS, None)?;
//     context.set_state_callback(Some(Box::new(|| {})));

//     // Wait for the context to be ready
//     loop {
//         match context.get_state() {
//             libpulse_binding::context::State::Ready => break,
//             libpulse_binding::context::State::Failed
//             | libpulse_binding::context::State::Terminated => {
//                 return Err("PulseAudio connection failed".into());
//             }
//             _ => {
//                 mainloop.iterate(true);
//             }
//         }
//     }

//     // Request server info
//     let mut done = false;
//     let mut default_sink = None;

//     context.get_server_info(|info| {
//         default_sink = Some(info.default_sink_name.clone().unwrap_or_default());
//         // Signal to exit mainloop after callback
//         done = true;
//     });

//     // Wait for callback
//     while !done {
//         mainloop.iterate(true);
//     }

//     println!(
//         "Default Sink: {}",
//         default_sink.unwrap_or_else(|| "<none>".to_string())
//     );

//     Ok(())
// }

fn is_playing_native2() -> anyhow::Result<()> {
    // Audio sample spec: CD quality, stereo, 16-bit little endian
    let spec = Spec {
        format: Format::S16NE,
        channels: 2,
        rate: 44100,
    };

    assert!(spec.is_valid());

    // Use a known monitor source — customize this based on `pactl list sources`
    //let device = "alsa_output.pci-0000_00_1f.3.analog-stereo.monitor".to_string();
    let device  = format!("{}.monitor", get_default_sink()?);


    let s = Simple::new(
        None,                 // Use default server
        "pulse-rms",          // Our app name
        libpulse_binding::stream::Direction::Record,
        Some(&device),         // Monitor source
        "record",             // Stream description
        &spec,
        None,
        None,
    )?;

    println!("Capturing from {}", device);

    // Buffer for ~0.5 second of audio (44100 * 0.5 * 2ch * 2 bytes/sample)
    let mut buf = vec![0u8; 44100 * 2 * 2 / 2];

    loop {
        s.read(&mut buf)?;
        let samples: Vec<i16> = buf
            .chunks_exact(2)
            .map(|b| i16::from_le_bytes([b[0], b[1]]))
            .collect();

        // Normalize and convert to f32
        let float_samples: Vec<f32> = samples
            .iter()
            .map(|&s| s as f32 / i16::MAX as f32)
            .collect();

        let rms = (float_samples.iter().map(|s| s * s).sum::<f32>() / float_samples.len() as f32).sqrt();

        println!("RMS amplitude: {:.6}", rms);

        std::thread::sleep(Duration::from_millis(100));
    }
}

fn is_playing_native() -> anyhow::Result<()> {
    let host = cpal::default_host();
    
    // Find the default input device (or monitor source)
    let device = host
        .default_input_device()
        .with_context(|| "No input device available")?;

    host
        .input_devices()?
        .for_each(|x| println!("{:?}", x.name()));


    println!("Using device: {}", device.name()?);

    // Get the default input config
    let config = device.default_input_config()?;

    // Create a channel to receive audio samples
    let (tx, rx) = channel::<Vec<f32>>();

    // Build input stream
    let stream_config = config.config();
    let stream = device.build_input_stream(
        &stream_config,
        move |data: &[f32], _: &cpal::InputCallbackInfo| {
            // Send captured samples to the main thread
            tx.send(data.to_vec()).unwrap();
        },
        |err| eprintln!("Stream error: {:?}", err),
        None,
    )?;

    // Start the stream
    stream.play()?;

    // Process samples and compute RMS amplitude
    loop {
        if let Ok(samples) = rx.recv_timeout(Duration::from_secs(1)) {
            // Convert samples to a Rodio source
            let source = SamplesBuffer::new(2, stream_config.sample_rate.0, samples.clone());

            // Compute RMS amplitude
            let rms = samples
                .iter()
                .map(|&sample| sample * sample)
                .sum::<f32>()
                .sqrt()
                / samples.len() as f32;

            println!("RMS Amplitude: {:.6}", rms);
        } else {
            println!("No samples received in 1 second");
        }
    }
}

fn play_sound(args: &Args) -> anyhow::Result<()> {
    debug!(
        "Playing {} Hz sine wave for {} seconds",
        args.frequency, args.pulse_duration
    );
    let (_stream, stream_handle) = OutputStream::try_default()?;
    let sink = Sink::try_new(&stream_handle)?;
    if args.pulse_duration != 0.0 {
        let src = SineWave::new(args.frequency)
            .amplify(args.amplitude)
            .take_duration(Duration::from_secs_f32(args.pulse_duration));
        sink.append(src);
    } else {
        let src = SineWave::new(args.frequency).amplify(args.amplitude);
        sink.append(src);
    };
    sink.sleep_until_end();
    debug!("Playing of wave stopped");

    Ok(())
}

fn main() {
    get_default_sink().unwrap();
    is_playing_native2().unwrap();
    
    let args = Args::parse();
    env_logger::init();

    let debug_interval = match std::env::var("DEBUG_INTERVAL") {
        Ok(val) => val.parse().unwrap_or_else(|e| {
            error!("{e}");
            DEBUG_INTERVAL_DEFAULT
        }),
        Err(e) => {
            info!("{e}");
            DEBUG_INTERVAL_DEFAULT
        }
    };

    let mut silence_start = SystemTime::now();
    let program_start = SystemTime::now();
    loop {
        sleep(Duration::new(1, 0));

        let is_playing = handle_err!(is_playing_pipewire());
        let secs_of_silence = handle_err!(silence_start.elapsed()).as_secs();
        let mins_of_silence = secs_of_silence / 60;

        if mins_of_silence >= args.minutes_of_silence {
            handle_err!(play_sound(&args));
            silence_start = SystemTime::now();
        } else if is_playing {
            silence_start = SystemTime::now();
        }

        if handle_err!(program_start.elapsed()).as_secs() % debug_interval == 0 {
            if is_playing {
                debug!("Sound is currently playing")
            } else {
                debug!(
                    "Period of silence: {:02}:{:02}",
                    mins_of_silence,
                    secs_of_silence % 60
                )
            }
        }
    }
}
