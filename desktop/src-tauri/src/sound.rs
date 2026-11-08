//! The app's sounds (design/sounds/), played here rather than in the window. WebKit only lets a
//! page make sound after the first click, which would silence the launch intro and requests that
//! arrive while the app sits in the tray. The window asks for each sound by name (`play_sound`);
//! the Sounds switch in Settings lives in the window too, so everything that reaches here plays.

use std::{
    collections::HashMap,
    num::NonZero,
    sync::mpsc,
    time::{Duration, Instant},
};

use rodio::{Source, buffer::SamplesBuffer};
use tauri::State;
use tracing::warn;

include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../design/sounds/build/sounds.rs"));

/// How long the speakers stay open after the last sound. An open stream keeps the audio device
/// and a thread busy mixing silence, so it closes when the app has been quiet this long.
const CLOSE_AFTER: Duration = Duration::from_secs(30);
/// After the speakers fail to open, they're tried again at the next sound after this long.
const RETRY_AFTER: Duration = Duration::from_secs(60);

pub struct Sounds {
    clips: HashMap<&'static str, SamplesBuffer>,
    /// To the thread that owns the speakers (the stream must stay on the thread that opened it).
    tx: Option<mpsc::Sender<(SamplesBuffer, f32)>>,
}

impl Sounds {
    pub fn new() -> Self {
        let clips = SOUNDS.iter().filter_map(|(name, bytes)| Some((*name, clip(bytes)?))).collect();
        let (tx, rx) = mpsc::channel::<(SamplesBuffer, f32)>();
        let spawned = std::thread::Builder::new().name("sounds".into()).spawn(move || speakers(rx));
        if let Err(e) = &spawned {
            warn!("no sound: couldn't start the sound thread: {e}");
        }
        Self {
            clips,
            tx: spawned.ok().map(|_| tx),
        }
    }

    /// `volume` is 1 for Loud and less for Soft (Settings > Appearance > Sounds).
    pub fn play(&self, name: &str, volume: f32) {
        let Some(clip) = self.clips.get(name) else {
            warn!("no sound called {name}");
            return;
        };
        if let Some(tx) = &self.tx {
            let _ = tx.send((clip.clone(), volume.clamp(0.0, 1.0)));
        }
    }
}

/// Opens the speakers for the first sound, plays everything that comes, and closes them after
/// `CLOSE_AFTER` of quiet.
fn speakers(rx: mpsc::Receiver<(SamplesBuffer, f32)>) {
    let mut sink = None;
    let mut failed_at: Option<Instant> = None;
    loop {
        let next = if sink.is_some() {
            match rx.recv_timeout(CLOSE_AFTER) {
                Ok(m) => m,
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    sink = None;
                    continue;
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => return,
            }
        } else {
            match rx.recv() {
                Ok(m) => m,
                Err(_) => return,
            }
        };
        if sink.is_none() && failed_at.is_none_or(|t| t.elapsed() >= RETRY_AFTER) {
            match rodio::DeviceSinkBuilder::open_default_sink() {
                Ok(mut s) => {
                    s.log_on_drop(false);
                    sink = Some(s);
                    failed_at = None;
                }
                Err(e) => {
                    warn!("no sound: couldn't open the speakers: {e}");
                    failed_at = Some(Instant::now());
                }
            }
        }
        if let Some(s) = &sink {
            let (clip, volume) = next;
            s.mixer().add(clip.amplify(volume));
        }
    }
}

/// Reads one of our own WAVs (design/sounds/build.mjs): 16-bit PCM, mono, any sample rate.
fn clip(bytes: &[u8]) -> Option<SamplesBuffer> {
    if bytes.len() < 44 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" || &bytes[36..40] != b"data" {
        return None;
    }
    let channels = u16::from_le_bytes([bytes[22], bytes[23]]);
    let rate = u32::from_le_bytes([bytes[24], bytes[25], bytes[26], bytes[27]]);
    let bits = u16::from_le_bytes([bytes[34], bytes[35]]);
    if bits != 16 {
        return None;
    }
    let samples: Vec<f32> = bytes[44..]
        .as_chunks::<2>()
        .0
        .iter()
        .map(|&s| f32::from(i16::from_le_bytes(s)) / 32768.0)
        .collect();
    let buffer = SamplesBuffer::new(NonZero::new(channels)?, NonZero::new(rate)?, samples);
    buffer.total_duration().is_some_and(|d| !d.is_zero()).then_some(buffer)
}

/// Async so the very first sound, which may wait for the speakers, never holds up the window.
#[tauri::command]
pub async fn play_sound(sounds: State<'_, Sounds>, name: String, volume: Option<f32>) -> Result<(), String> {
    sounds.play(&name, volume.unwrap_or(1.0));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_sound_reads() {
        assert!(SOUNDS.len() >= 20);
        for (name, bytes) in SOUNDS {
            let c = clip(bytes).unwrap_or_else(|| panic!("{name} didn't read"));
            let ms = c.total_duration().unwrap().as_millis();
            assert!((20..=1500).contains(&ms), "{name} is {ms} ms");
        }
        assert!(SOUNDS.iter().any(|(n, _)| *n == "tap"));
    }

    /// Plays the done chime through this computer's speakers: `cargo test -p nhako-desktop -- --ignored speakers`.
    #[test]
    #[ignore = "makes a sound"]
    fn speakers() {
        assert!(rodio::DeviceSinkBuilder::open_default_sink().is_ok(), "couldn't open the speakers");
        let sounds = Sounds::new();
        sounds.play("done", 1.0);
        std::thread::sleep(Duration::from_millis(1600));
    }
}
