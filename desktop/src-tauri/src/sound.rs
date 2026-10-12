//! The app's sounds (design/sounds/), played here rather than in the window. WebKit only lets a
//! page make sound after the first click, which would silence the launch intro and requests that
//! arrive while the app sits in the tray. The window asks for each sound by name (`play_sound`);
//! the Sounds switch in Settings lives in the window too, so everything that reaches here plays.

use std::{
    collections::HashMap,
    num::NonZero,
    sync::{Mutex, OnceLock, mpsc},
    time::Duration,
};

use rodio::{Source, buffer::SamplesBuffer, mixer::Mixer};
use tauri::State;
use tracing::warn;

include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../design/sounds/build/sounds.rs"));

pub struct Sounds {
    clips: HashMap<&'static str, SamplesBuffer>,
    /// The speakers, opened on a thread of their own at start (the stream must stay on it).
    pending: Mutex<Option<mpsc::Receiver<Option<Mixer>>>>,
    mixer: OnceLock<Option<Mixer>>,
}

impl Sounds {
    pub fn new() -> Self {
        let clips = SOUNDS.iter().filter_map(|(name, bytes)| Some((*name, clip(bytes)?))).collect();
        let (tx, rx) = mpsc::channel();
        std::thread::Builder::new()
            .name("sounds".into())
            .spawn(move || match rodio::DeviceSinkBuilder::open_default_sink() {
                Ok(mut sink) => {
                    sink.log_on_drop(false);
                    let _ = tx.send(Some(sink.mixer().clone()));
                    // Keeps the speakers open for as long as the app runs.
                    loop {
                        std::thread::park();
                    }
                }
                Err(e) => {
                    warn!("no sound: couldn't open the speakers: {e}");
                    let _ = tx.send(None);
                }
            })
            .ok();
        Self {
            clips,
            pending: Mutex::new(Some(rx)),
            mixer: OnceLock::new(),
        }
    }

    fn mixer(&self) -> Option<&Mixer> {
        self.mixer
            .get_or_init(|| {
                let rx = self.pending.lock().ok()?.take()?;
                // The first sound (the launch intro) can come before the speakers are open.
                rx.recv_timeout(Duration::from_millis(600)).ok().flatten()
            })
            .as_ref()
    }

    /// `volume` is 1 for Loud and less for Soft (Settings > Appearance > Sounds).
    pub fn play(&self, name: &str, volume: f32) {
        let Some(clip) = self.clips.get(name) else {
            warn!("no sound called {name}");
            return;
        };
        if let Some(mixer) = self.mixer() {
            mixer.add(clip.clone().amplify(volume.clamp(0.0, 1.0)));
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
        let sounds = Sounds::new();
        assert!(sounds.mixer().is_some(), "couldn't open the speakers");
        sounds.play("done", 1.0);
        std::thread::sleep(Duration::from_millis(1400));
    }
}
