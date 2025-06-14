use clap::Parser;
use log::{debug, error};
use rodio::source::SineWave;
use rodio::{OutputStream, Sink, Source};
use std::process::Command;
use std::thread::sleep;
use std::time::{Duration, SystemTime};

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
    let args = Args::parse();
    env_logger::init();

    let debug_interval = match std::env::var("DEBUG_INTERVAL") {
        Ok(val) => val.parse().unwrap_or(DEBUG_INTERVAL_DEFAULT),
        Err(e) => {
            error!("{}", e);
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
