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

This is of course despite the fact that HDMI-CEC already turns the speaker
off automatically, when the TV is turned off.

> _"I'm not mad at you, I'm mad at the system" - Dennis_

Seeing as the EU has made proper speaker integration with your TV _illegal_,
and I couldn't find a proper library for this on linux, I created this small
rust script in an afternoon.

## Features

Generates a stream of zeroes with periodic 0.1-second bursts of a sine
wave (default: 50Hz, 0.1% amplitude, every 0.1 seconds).  Configurable via
command-line arguments for pulse rate, signal frequency, amplitude, sample
rate, and pulse duration. Keeps audio outputs alive, similar to Sound
Keeper’s Fluctuate mode.

## Installation

TODO

View help for all options:
```bash
  cargo run -- --help
```
