# Rustle

Introducing Rustle, a lightweight, Rust-based audio stream generator for
Linux, inspired by Sound Keeper. It generates periodic, inaudible sine wave
pulses to prevent speakers from going into sleep mode. Why would this ever
be necessary you ask? Well, I've bought my first set of proper speakers for
my TV and infuriatingly, they kept turning off after 15 minutes. I emailed
the manufacturer about the issue and they responded:

> _I understand the situation, it can be frustrating when the speakers power
> down simply because a movie was paused for a while. However, the automatic
> standby function you're referring to is required by EU regulation, specifically
> Commission Regulation (EU) No 801/2013 as far as I remember. This regulation
> mandates that electronic devices like active speakers automatically switch to
> standby mode after a maximum of 20 minutes of inactivity (no audio signal),
> and most manufacturers, including us, configure this to occur after 15
> minutes to ensure compliance._
> 
> _Unfortunately, this feature cannot be disabled, as it is a legal requirement
> aimed at reducing energy consumption across the EU._

This is of course despite the fact that HDMI-CEC _already_ turns the speaker
off automatically, when the TV is turned off.

> _"I'm not mad at you, **I'm mad at the system**" - Dennis_

Seeing as the EU has made proper speaker integration with your TV _illegal_,
and I couldn't find a proper library for this on linux, I created this small
rust script in an afternoon.

> **Note:** Requires Pulseaudio and Alsa, but this should be standard on most Linux distros

## Features

Generates a configurable sine wive in periods of silence. See the options below:

```
  Usage: rustle [OPTIONS]
  
  Options:
    -d, --pulse-duration <PULSE_DURATION>
            Duration of each tone in seconds (0 for continual playback) [default: 10]
    -f, --frequency <FREQUENCY>
            Frequency of the sine wave during pulses in Hz [default: 20]
    -a, --amplitude <AMPLITUDE>
            Amplitude of the sine wave (e.g., 0.01 for 1%) [default: 0.01]
    -s, --minutes-of-silence <MINUTES_OF_SILENCE>
            Minutes of undetected sound until the tone plays [default: 10]
    -t, --threshold <THRESHOLD>
            Threshold sound level that counts as "undetected sound" [default: 0.001]
    -i, --check-interval <CHECK_INTERVAL>
            How often to check for sound in seconds [default: 1]
    -h, --help
            Print help
    -V, --version
            Print version
```

So, by default if there has been no sound playing for 10 minutes, generate
a sine wave of 20Hz for 10 seconds, resetting the silence period. Depending
on the model of your speakers, you may have to tweak these options, but they
work for mine. To enable debugging set the following environment variables:

```bash
  export RUST_LOG=debug # Enable debugging
  export DEBUG_INTERVAL=10 # Interval of debug messages, in seconds
  rustle
```

## installation

**Nix (recommended):**

```bash
  nix profile install github:rasmus-kirk/rustle
```

**Cargo:**

```bash
  cargo install rustle
```

If you install with cargo you might need some alsa/pulseaudio dependencies,
I suggest using Nix, since it will handle the non-rust dependencies for you.

## Systemd Service in Nix

I personally run this as a systemd service using Nix Home Manager. The
service can be seen [here](https://github.com/rasmus-kirk/nix-config/blob/main/modules/home-manager/rustle/default.nix)
