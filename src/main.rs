use clap::Parser;
use rodio::cpal::traits::HostTrait;
use rodio::source::SineWave;
use rodio::{cpal, DeviceTrait, OutputStream, Sink, Source};
use std::f32::consts::PI;
use std::thread::sleep;
use std::time::{Duration, SystemTime};

#[derive(Parser, Debug)]
#[command(author="Rasmus Kirk", version, about = "Rustle - Keep your digital speakers from sleeping, using low sound signals", long_about = None)]
struct Args {
    /// Pulse rate in Hz (e.g., 2.0 for a pulse every 0.5 seconds)
    #[arg(short = 'r', long, default_value_t = 1.0)]
    pulse_rate: f32,

    /// Frequency of the sine wave during pulses in Hz
    #[arg(short = 'f', long, default_value_t = 50.0)]
    signal_frequency: f32,

    /// Amplitude of the sine wave (e.g., 0.01 for 1%)
    #[arg(short = 'a', long, default_value_t = 0.001)]
    amplitude: f32,

    /// Sample rate in Hz (e.g., 44100)
    #[arg(short = 's', long, default_value_t = 44100)]
    sample_rate: u32,

    /// Duration of each pulse in seconds (e.g., 0.1)
    #[arg(short = 'd', long, default_value_t = 0.01)]
    pulse_duration: f32,
}

struct FluctuateSource {
    sample_rate: u32,              // Samples per second (e.g., 44.1kHz)
    signal_frequency: f32,         // Frequency of the sine wave during pulses (Hz, e.g., 600Hz)
    amplitude: f32,                // Amplitude of the sine wave (e.g., 0.01 for 1%)
    samples_since_last_pulse: u32, // Counter for samples since last pulse
    samples_in_current_pulse: u32, // Counter for samples within the current pulse
    pulse_interval: u32,           // Samples between pulses (sample_rate / pulse_rate)
    pulse_length: u32,             // Samples in a pulse (sample_rate * pulse_duration)
}

impl FluctuateSource {
    fn new(
        pulse_rate: f32,
        signal_frequency: f32,
        amplitude: f32,
        sample_rate: u32,
        pulse_duration: f32,
    ) -> Self {
        let pulse_interval = (sample_rate as f32 / pulse_rate) as u32;
        let pulse_length = (sample_rate as f32 * pulse_duration) as u32;
        FluctuateSource {
            sample_rate,
            signal_frequency,
            amplitude,
            samples_since_last_pulse: 0,
            samples_in_current_pulse: 0,
            pulse_interval,
            pulse_length,
        }
    }
}

impl Source for FluctuateSource {
    fn current_frame_len(&self) -> Option<usize> {
        None // Stream is infinite
    }

    fn channels(&self) -> u16 {
        1 // Mono audio
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn total_duration(&self) -> Option<Duration> {
        None // Stream is infinite
    }
}

impl Iterator for FluctuateSource {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        self.samples_since_last_pulse += 1;

        if self.samples_since_last_pulse >= self.pulse_interval {
            // Start a new pulse
            self.samples_since_last_pulse = 0;
            self.samples_in_current_pulse = 0;
        }
        
        if self.samples_in_current_pulse < self.pulse_length {
            // Generate sine wave sample for the current pulse
            self.samples_in_current_pulse += 1;
            let time = self.samples_in_current_pulse as f32 / self.sample_rate as f32;
            let sample = (2.0 * PI * self.signal_frequency * time).sin() * self.amplitude;
            Some(sample)
        } else {
            // Output zero outside of pulses
            Some(0.0)
        }
    }
}

fn main() {
    // Parse command-line arguments
    let args = Args::parse();

    // Initialize the audio output stream
    let (_stream, stream_handle) = OutputStream::try_default().unwrap();
    let sink = Sink::try_new(&stream_handle).unwrap();

    if args.pulse_rate == 0.0 || args.pulse_duration == 0.0 {
        let source = SineWave::new(args.signal_frequency).amplify(args.amplitude);
        sink.append(source);
    } else {
        let source = FluctuateSource::new(
            args.pulse_rate,
            args.signal_frequency,
            args.amplitude,
            args.sample_rate,
            args.pulse_duration,
        );
        sink.append(source);
    }

    // Keep the program running until interrupted (e.g., Ctrl+C)
    let start = SystemTime::now();
    let mut cur = 0;
    loop {
        sleep(Duration::new(1, 0));
        let now = start.elapsed().unwrap().as_secs();
        if cur != now {
            println!("{:02}:{:02}", now / 60, now % 60);
            cur = now;
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_dummy() {
        assert!(true)
    }
}
