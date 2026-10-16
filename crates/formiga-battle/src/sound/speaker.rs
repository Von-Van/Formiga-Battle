//! The speaker: the only part of Battle that touches an audio device. It opens the default output
//! on a thread of its own, so the window never waits on it, and feeds it the mixer for as long as
//! the window is open. Nothing else here knows rodio is there.

use super::mixer::{Command, Mixer};
use rodio::{ChannelCount, SampleRate, Source};
use std::num::NonZero;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Receiver;
use std::time::Duration;

/// How many samples are made between looks for new commands: about a millisecond and a half.
const LOOK_EVERY: u32 = 64;

/// Opens the default output and plays the mixer on it until `hush` is dropped. Says why if there
/// is nothing to play on; once open, a device that fails later is reported once and otherwise
/// left be.
pub fn speak(commands: Receiver<Command>, hush: Receiver<()>) -> Result<(), String> {
    let said = Arc::new(AtomicBool::new(false));
    let complain = move |error: rodio::cpal::StreamError| {
        if !said.swap(true, Ordering::Relaxed) {
            eprintln!("formiga-battle: something went wrong with the sound: {error}");
        }
    };
    let mut sink = rodio::DeviceSinkBuilder::from_default_device()
        .and_then(|device| device.with_error_callback(complain).open_sink_or_fallback())
        .map_err(|error| error.to_string())?;
    sink.log_on_drop(false);
    let rate = sink.config().sample_rate();
    sink.mixer().add(Feed {
        mixer: Mixer::new(rate.get() as f32),
        commands,
        rate,
        until_look: 0,
    });
    // Nothing more to do here but keep the device open: the window says when it closes.
    let _ = hush.recv();
    Ok(())
}

/// The mixer as rodio plays it: one channel at the device's own rate, so nothing is resampled.
struct Feed {
    mixer: Mixer,
    commands: Receiver<Command>,
    rate: SampleRate,
    until_look: u32,
}

impl Iterator for Feed {
    type Item = rodio::Sample;

    fn next(&mut self) -> Option<rodio::Sample> {
        if self.until_look == 0 {
            self.until_look = LOOK_EVERY;
            while let Ok(command) = self.commands.try_recv() {
                self.mixer.command(command);
            }
        }
        self.until_look -= 1;
        Some(self.mixer.next())
    }
}

impl Source for Feed {
    fn current_span_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> ChannelCount {
        NonZero::<u16>::MIN
    }

    fn sample_rate(&self) -> SampleRate {
        self.rate
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}
