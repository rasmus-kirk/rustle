use std::fs::File;
use std::io::BufReader;
use std::time::Duration;
use rodio::{Decoder, OutputStream, Sink};
use rodio::source::{SineWave, Source};

fn main() {
    // _stream must live as long as the sink
    let (_stream, stream_handle) = OutputStream::try_default().unwrap();
    let sink = Sink::try_new(&stream_handle).unwrap();

    // Add a dummy source of the sake of the example.

    let tones = [ 261.63, 277.18, 293.66, 311.13, 329.63, 349.23, 369.99, 391.99, 415.30, 440.00, 466.16, 493.88 ];
    for tone in tones {
        let source = SineWave::new(tone).take_duration(Duration::from_secs_f32(1.0)).amplify(0.40);
        sink.append(source);
    }

    // The sound plays in a separate thread. This call will block the current thread until the sink
    // has finished playing all its queued sounds.
    sink.sleep_until_end();
}
