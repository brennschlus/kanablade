//! Tiny xorshift32: plenty for visual and audio jitter, no dependency.

use bevy::prelude::Resource;

#[derive(Resource)]
pub struct Rng(u32);

impl Rng {
    /// `seed` must be non-zero.
    pub const fn new(seed: u32) -> Self {
        Rng(seed)
    }

    pub fn seeded() -> Self {
        let t = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(1, |d| d.subsec_nanos() ^ d.as_secs() as u32);
        Rng(t | 1)
    }

    pub fn next(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        x
    }

    /// Uniform in `0.0..=1.0`.
    pub fn unit(&mut self) -> f32 {
        self.next() as f32 / u32::MAX as f32
    }

    /// Uniform in `-1.0..=1.0`.
    pub fn noise(&mut self) -> f32 {
        self.unit() * 2.0 - 1.0
    }

    pub fn range(&mut self, a: f32, b: f32) -> f32 {
        a + (b - a) * self.unit()
    }

    pub fn below(&mut self, n: usize) -> usize {
        self.next() as usize % n
    }
}
