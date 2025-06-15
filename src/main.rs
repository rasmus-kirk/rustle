#![feature(mpmc_channel)]

use anyhow::{Context, Result, bail, ensure};
use clap::Parser;
use libpulse_binding::mainloop::standard::IterateResult;
use libpulse_binding::sample::{Format, Spec};
use libpulse_simple_binding::Simple;
use log::{debug, error, info};
use rodio::source::SineWave;
use rodio::{OutputStream, Sink, Source};
use std::thread::sleep;
use std::time::{Duration, SystemTime};

use libpulse_binding::context::Context as LibpulseContext;
use libpulse_binding::{context::State, mainloop::standard::Mainloop};
use std::cell::RefCell;
use std::rc::Rc;

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

    /// Threshold sound level that counts as "undetected sound"
    #[arg(short = 't', long, default_value_t = 0.001)]
    threshold: f32,

    /// How often to check for sound in seconds
    #[arg(short = 'i', long, default_value_t = 1)]
    check_interval: u64,
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

fn get_default_sink() -> anyhow::Result<String> {
    // Create a new mainloop
    let mainloop = Rc::new(RefCell::new(
        Mainloop::new().expect("Failed to create mainloop"),
    ));

    // Create a new context
    let context = Rc::new(RefCell::new(
        LibpulseContext::new(&*mainloop.borrow(), "PulseContext")
            .with_context(|| "Failed to create context")?,
    ));

    context
        .borrow_mut()
        .connect(None, libpulse_binding::context::FlagSet::NOFLAGS, None)
        .expect("Failed to connect context");

    // Wait for the context to be ready
    loop {
        match mainloop.borrow_mut().iterate(true) {
            IterateResult::Success(_) => (),
            IterateResult::Err(e) => panic!("Mainloop iteration failed: {e}"),
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
                info!("Default Output Sink: {default_sink}");
            }
            *server_info_received_clone.borrow_mut() = true;
        });

    // Wait until server info is received
    while !*server_info_received.borrow() {
        match mainloop.borrow_mut().iterate(true) {
            IterateResult::Success(_) => (),
            IterateResult::Err(e) => bail!("Mainloop iteration failed: {e}"),
            IterateResult::Quit(_) => bail!("Mainloop quit unexpectedly"),
        }
    }

    // Clean up
    context.borrow_mut().disconnect();

    default_sink_received
        .borrow()
        .clone()
        .with_context(|| "No default sink found")
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

fn main() -> Result<()> {
    let args = Args::parse();
    env_logger::init();

    let debug_interval = match std::env::var("DEBUG_INTERVAL") {
        Ok(val) => val.parse()?,
        Err(e) => {
            info!("{e}");
            DEBUG_INTERVAL_DEFAULT
        }
    };

    let spec = Spec {
        format: Format::U8,
        channels: 1,
        rate: 256,
    };
    ensure!(spec.is_valid());

    let device = format!("{}.monitor", get_default_sink()?);
    let s = Simple::new(
        None,
        "rustle",
        libpulse_binding::stream::Direction::Record,
        Some(&device),
        "record",
        &spec,
        None,
        None,
    )?;

    let mut buf = vec![0u8; spec.rate as usize * spec.channels as usize];
    let mut silence_start = SystemTime::now();
    let program_start = SystemTime::now();
    loop {
        sleep(Duration::from_secs(args.check_interval));

        handle_err!(s.read(&mut buf));
        let sum_squares: f32 = buf
            .iter()
            .map(|b| ((*b as f32 - 128.0) / 128.0).powi(2))
            .sum();
        let rms = (sum_squares / buf.len() as f32).sqrt();
        let is_playing = rms >= args.threshold;
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
                debug!("Sound is currently playing ({rms} vol)")
            } else {
                debug!(
                    "Period of silence: {:02}:{:02} ({rms} vol)",
                    mins_of_silence,
                    secs_of_silence % 60
                )
            }
        }
    }
}
