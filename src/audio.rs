//! Procedurally synthesized sound, played through kira on a dedicated thread.
//!
//! The game only sends tiny `Cue`s over a channel, so a key press reaches the
//! mixer within microseconds; the cpal buffer is kept small for low latency.

use bevy::prelude::*;
use kira::{
    AudioManager, AudioManagerSettings, Decibels, DefaultBackend, Frame, Mix, Tween,
    backend::cpal::CpalBackendSettings,
    effect::{filter::FilterBuilder, reverb::ReverbBuilder},
    sound::static_sound::{StaticSoundData, StaticSoundHandle, StaticSoundSettings},
    track::{TrackBuilder, TrackHandle},
};
use std::{
    f32::consts::TAU,
    sync::{Arc, mpsc},
    time::Duration,
};

const RATE: u32 = 48_000;
/// Japanese "In" (miyako-bushi) scale on D, in semitones from D4.
const SCALE: [i32; 5] = [0, 1, 5, 7, 8];

#[derive(Debug, Clone, Copy)]
pub enum Cue {
    Key,
    /// Scale degree (any integer; wraps into octaves).
    Note(i32),
    Miss,
    LineDone,
    ToggleMute,
}

#[derive(Resource)]
pub struct Audio {
    tx: mpsc::Sender<Cue>,
    pub muted: bool,
}

impl Audio {
    pub fn play(&self, cue: Cue) {
        let _ = self.tx.send(cue);
    }

    pub fn toggle_mute(&mut self) {
        self.muted = !self.muted;
        self.play(Cue::ToggleMute);
    }
}

pub struct AudioPlugin;

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        let (tx, rx) = mpsc::channel();
        std::thread::Builder::new()
            .name("kanablade-audio".into())
            .spawn(move || audio_thread(rx))
            .expect("spawn audio thread");
        app.insert_resource(Audio { tx, muted: false });
    }
}

fn low_latency_manager() -> Option<AudioManager> {
    use kira::backend::cpal::cpal::{
        BufferSize, SupportedBufferSize,
        traits::{DeviceTrait, HostTrait},
    };
    let try_fixed = || -> Option<AudioManager> {
        let device = kira::backend::cpal::cpal::default_host().default_output_device()?;
        let supported = device.default_output_config().ok()?;
        let mut config = supported.config();
        let frames = match supported.buffer_size() {
            SupportedBufferSize::Range { min, max } => 512.clamp(*min, *max),
            SupportedBufferSize::Unknown => 512,
        };
        config.buffer_size = BufferSize::Fixed(frames);
        let settings = AudioManagerSettings {
            backend_settings: CpalBackendSettings { device: None, config: Some(config) },
            ..default()
        };
        AudioManager::<DefaultBackend>::new(settings).ok()
    };
    try_fixed().or_else(|| match AudioManager::<DefaultBackend>::new(default()) {
        Ok(m) => Some(m),
        Err(e) => {
            warn!("audio disabled: {e}");
            None
        }
    })
}

struct Bank {
    keys: Vec<StaticSoundData>,
    notes: Vec<StaticSoundData>,
    miss: StaticSoundData,
    bowl: StaticSoundData,
    ambient: StaticSoundData,
}

fn audio_thread(rx: mpsc::Receiver<Cue>) {
    let Some(mut manager) = low_latency_manager() else {
        // Drain silently so senders never block or error.
        while rx.recv().is_ok() {}
        return;
    };

    let mut room: Option<TrackHandle> = manager
        .add_sub_track(
            TrackBuilder::new()
                .with_effect(ReverbBuilder::new().feedback(0.88).damping(0.55).mix(Mix(0.32))),
        )
        .ok();
    let mut dry: Option<TrackHandle> = manager
        .add_sub_track(TrackBuilder::new().with_effect(FilterBuilder::new().cutoff(9000.0)))
        .ok();

    let bank = Bank::synthesize();
    let mut rng = Rng(0x9E37_79B9);
    let mut muted = false;

    let mut ambient: Option<StaticSoundHandle> = manager.play(bank.ambient.clone()).ok();

    while let Ok(cue) = rx.recv() {
        if let Cue::ToggleMute = cue {
            muted = !muted;
            let vol = if muted { Decibels::SILENCE } else { Decibels::IDENTITY };
            manager.main_track().set_volume(vol, Tween { duration: Duration::from_millis(250), ..default() });
            if let Some(a) = ambient.as_mut() {
                if muted { a.pause(Tween::default()) } else { a.resume(Tween::default()) }
            }
            continue;
        }
        if muted {
            continue;
        }
        let sound = match cue {
            Cue::Key => {
                let k = &bank.keys[rng.below(bank.keys.len())];
                k.playback_rate(0.94 + rng.unit() as f64 * 0.12).panning(rng.unit() * 0.3 - 0.15)
            }
            Cue::Note(degree) => {
                let idx = degree.clamp(0, bank.notes.len() as i32 - 1) as usize;
                bank.notes[idx].panning(rng.unit() * 0.4 - 0.2)
            }
            Cue::Miss => bank.miss.clone(),
            Cue::LineDone => bank.bowl.clone(),
            Cue::ToggleMute => unreachable!(),
        };
        let wet = matches!(cue, Cue::Note(_) | Cue::LineDone);
        let res = match (wet, room.as_mut(), dry.as_mut()) {
            (true, Some(t), _) | (false, _, Some(t)) => t.play(sound).map(|_| ()),
            _ => manager.play(sound).map(|_| ()),
        };
        if let Err(e) = res {
            warn!("play failed: {e}");
        }
    }
}

struct Rng(u32);

impl Rng {
    fn next(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        x
    }
    fn unit(&mut self) -> f32 {
        self.next() as f32 / u32::MAX as f32
    }
    fn noise(&mut self) -> f32 {
        self.unit() * 2.0 - 1.0
    }
    fn below(&mut self, n: usize) -> usize {
        self.next() as usize % n
    }
}

fn sound(samples: Vec<Frame>, volume_db: f32) -> StaticSoundData {
    StaticSoundData {
        sample_rate: RATE,
        frames: Arc::from(samples),
        settings: StaticSoundSettings::new().volume(Decibels(volume_db)),
        slice: None,
    }
}

fn mono(samples: impl IntoIterator<Item = f32>) -> Vec<Frame> {
    samples.into_iter().map(Frame::from_mono).collect()
}

fn len(secs: f32) -> usize {
    (secs * RATE as f32) as usize
}

impl Bank {
    fn synthesize() -> Self {
        let mut rng = Rng(12345);
        let keys = (0..4).map(|v| sound(key_click(&mut rng, v), -13.0)).collect();
        // Two and a half octaves of the scale: D3 .. A5.
        let notes = (-5..8)
            .map(|deg: i32| {
                let semis = SCALE[deg.rem_euclid(5) as usize] + 12 * deg.div_euclid(5);
                let freq = 293.66 * 2f32.powf(semis as f32 / 12.0);
                sound(pluck(freq), -11.0)
            })
            .collect();
        Self {
            keys,
            notes,
            miss: sound(miss(&mut rng), -9.0),
            bowl: sound(singing_bowl(), -8.0),
            ambient: sound(ambient(&mut rng), -21.0).loop_region(..),
        }
    }
}

/// A soft wooden tick: a damped resonance excited by a short noise burst.
fn key_click(rng: &mut Rng, variant: usize) -> Vec<Frame> {
    let n = len(0.06);
    let freq = [1850.0, 2100.0, 1650.0, 2350.0][variant % 4];
    let mut lp = 0.0f32;
    mono((0..n).map(|i| {
        let t = i as f32 / RATE as f32;
        let burst = if t < 0.004 { rng.noise() * (1.0 - t / 0.004) } else { 0.0 };
        lp += (burst - lp) * 0.35;
        let body = (TAU * freq * t).sin() * (-t * 140.0).exp();
        let low = (TAU * 420.0 * t).sin() * (-t * 90.0).exp() * 0.5;
        (lp * 0.6 + body * 0.55 + low) * 0.8
    }))
}

/// Koto/kalimba-like pluck with slightly inharmonic partials.
fn pluck(freq: f32) -> Vec<Frame> {
    let n = len(2.2);
    let partials = [(1.0, 1.0, 1.0), (2.0, 0.35, 1.8), (3.01, 0.12, 2.6), (4.18, 0.06, 4.0), (5.43, 0.03, 6.0)];
    let samples: Vec<f32> = (0..n)
        .map(|i| {
            let t = i as f32 / RATE as f32;
            let attack = (t / 0.004).min(1.0);
            let mut s = 0.0;
            for (ratio, amp, damp) in partials {
                s += (TAU * freq * ratio * t).sin() * amp * (-t * 2.4 * damp).exp();
            }
            s * attack * 0.5
        })
        .collect();
    // Tiny stereo detune for width.
    let d = (RATE as f32 * 0.0007) as usize;
    (0..n)
        .map(|i| Frame { left: samples[i], right: samples[i.saturating_sub(d)] })
        .collect()
}

/// Gentle, low, non-punishing "thup".
fn miss(rng: &mut Rng) -> Vec<Frame> {
    let n = len(0.18);
    let mut lp = 0.0f32;
    let mut phase = 0.0f32;
    mono((0..n).map(|i| {
        let t = i as f32 / RATE as f32;
        let f = 150.0 * (1.0 - t * 2.5).max(0.4);
        phase += TAU * f / RATE as f32;
        lp += (rng.noise() - lp) * 0.06;
        let env = (t / 0.003).min(1.0) * (-t * 28.0).exp();
        (phase.sin() * 0.8 + lp * 0.9) * env
    }))
}

/// Singing bowl: inharmonic partials, each a slowly beating pair.
fn singing_bowl() -> Vec<Frame> {
    let n = len(5.5);
    let base = 220.0;
    let partials = [(1.0, 1.0, 0.55), (2.76, 0.45, 0.9), (5.4, 0.22, 1.4), (8.9, 0.08, 2.2)];
    (0..n)
        .map(|i| {
            let t = i as f32 / RATE as f32;
            let attack = (t / 0.02).min(1.0);
            let (mut l, mut r) = (0.0, 0.0);
            for (k, (ratio, amp, damp)) in partials.iter().enumerate() {
                let f = base * ratio;
                let beat = 0.6 + k as f32 * 0.35;
                let env = amp * (-t * damp).exp();
                l += ((TAU * f * t).sin() + (TAU * (f + beat) * t).sin()) * env;
                r += ((TAU * f * t + 0.7).sin() + (TAU * (f - beat) * t).sin()) * env;
            }
            Frame { left: l * attack * 0.3, right: r * attack * 0.3 }
        })
        .collect()
}

/// A seamless 16 s drone: D + A with slow breathing, plus soft filtered wind.
fn ambient(rng: &mut Rng) -> Vec<Frame> {
    let secs = 16.0;
    let n = len(secs);
    // Frequencies chosen to complete whole cycles in the loop length.
    let tone = |f: f32| (f * secs).round() / secs;
    let voices = [(tone(73.42), 0.5), (tone(110.0), 0.32), (tone(146.83), 0.18), (tone(220.0), 0.06)];
    let mut wind_l = 0.0f32;
    let mut wind_r = 0.0f32;
    let mut out: Vec<Frame> = (0..n)
        .map(|i| {
            let t = i as f32 / RATE as f32;
            let breath = 0.75 + 0.25 * (TAU * t / secs * 2.0).sin();
            let mut s = 0.0;
            for (k, (f, a)) in voices.iter().enumerate() {
                let wobble = 1.0 + 0.15 * (TAU * t / secs * (k as f32 + 1.0)).sin();
                s += (TAU * f * t).sin() * a * wobble;
            }
            wind_l += (rng.noise() - wind_l) * 0.012;
            wind_r += (rng.noise() - wind_r) * 0.012;
            let gust = 0.5 + 0.5 * (TAU * t / secs * 3.0).sin().powi(2);
            Frame {
                left: s * breath * 0.6 + wind_l * 1.6 * gust,
                right: s * breath * 0.6 + wind_r * 1.6 * gust,
            }
        })
        .collect();
    // Crossfade the noise seam.
    let fade = len(0.5);
    for i in 0..fade {
        let a = i as f32 / fade as f32;
        let tail = out[n - fade + i];
        let head = out[i];
        out[i] = Frame { left: head.left * a + tail.left * (1.0 - a), right: head.right * a + tail.right * (1.0 - a) };
    }
    out
}

/// Maps a typed unit to a scale degree: the vowel picks the note,
/// position in the line lifts it gently so phrases rise and fall.
pub fn degree_for(romaji: &str, index: usize) -> i32 {
    let v = match romaji.chars().rev().find(|c| "aiueon".contains(*c)) {
        Some('a') => 0,
        Some('i') => 3,
        Some('u') => 1,
        Some('e') => 2,
        Some('o') => 4,
        Some('n') => -2,
        _ => 0,
    };
    let lift = [0, 0, 1, 1, 2, 1, 0, 1][index % 8];
    v + lift + 2
}
