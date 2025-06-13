#![feature(iterator_try_collect)]

use clap::Parser;
use log::{debug, error};
use rodio::source::SineWave;
use rodio::{OutputStream, Sink, Source};
use std::fs::DirEntry;
use std::path::Path;
use std::thread::sleep;
use std::time::{Duration, SystemTime};
use std::{fs, io};

// In seconds
const DEBUG_INTERVAL: u64 = 5;

#[derive(Parser, Debug)]
#[command(author="Rasmus Kirk", version, about = "Rustle - Keep your digital speakers from sleeping, using low sound signals", long_about = None)]
struct Args {
    /// Duration of each tone in seconds
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
    mins_of_silence: u64,
}

macro_rules! handle_err_cont {
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

// This function is horrible no matter what I do lol
fn is_sound_playing() -> io::Result<bool> {
    let base_path = Path::new("/proc/asound");

    let handle_err = |e| {
        error!("{e}");
        false
    };

    let f = |entry: io::Result<DirEntry>, starts_with| {
        let entry = entry.ok()?;
        let filename_starts_with = entry.file_name().to_string_lossy().starts_with(starts_with);
        let is_dir = entry
            .file_type()
            .map(|ft| ft.is_dir())
            .unwrap_or_else(handle_err);
        if !(filename_starts_with && is_dir) {
            return None;
        };
        Some(entry)
    };

    let check_sub = |path| {
        fs::read_dir(path)
            .into_iter()
            .flatten()
            .filter(|entry| {
                entry
                    .as_ref()
                    .is_ok_and(|x| x.file_name().to_str().is_some_and(|x| x == "status"))
            })
            .any(|status| {
                status.is_ok_and(|status| {
                    fs::read_to_string(status.path())
                        .map(|content| content.contains("state: RUNNING"))
                        .unwrap_or_else(handle_err)
                })
            })
    };

    let check_pcm = |path| {
        fs::read_dir(path)
            .into_iter()
            .flatten()
            .filter_map(|x| f(x, "sub"))
            .any(|sub| check_sub(sub.path()))
    };

    let check_card = |path: &Path| {
        fs::read_dir(path)
            .into_iter()
            .flatten()
            .filter_map(|x| f(x, "pcm"))
            .any(|pcm| check_pcm(pcm.path()))
    };

    Ok(fs::read_dir(base_path)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_name().to_string_lossy().starts_with("card"))
        .filter(|entry| {
            entry
                .file_type()
                .map(|ft| ft.is_dir())
                .unwrap_or_else(handle_err)
        })
        .any(|card| check_card(&card.path())))
}

fn play_sound(args: &Args) -> anyhow::Result<()> {
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

    Ok(())
}

fn main() {
    let args = Args::parse();
    env_logger::init();

    let mut silence_start = SystemTime::now();
    loop {
        sleep(Duration::new(1, 0));

        let is_playing = handle_err_cont!(is_sound_playing());
        let secs_of_silence = handle_err_cont!(silence_start.elapsed()).as_secs();
        let mins_of_silence = secs_of_silence / 60;

        if mins_of_silence >= args.mins_of_silence {
            handle_err_cont!(play_sound(&args));
            silence_start = SystemTime::now();
        } else if is_playing {
            silence_start = SystemTime::now();
        }

        if secs_of_silence % DEBUG_INTERVAL == 0 {
            debug!(
                "Period of silence: {:02}:{:02}",
                mins_of_silence,
                secs_of_silence % 60
            );
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_dummy() {}
}
