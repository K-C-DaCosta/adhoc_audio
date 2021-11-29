use super::{
    collections::{BitStream, CircularStack, NibbleList},
    math::FixedParabola,
};
pub use std::io::{Seek, SeekFrom};

pub mod adhoc;
pub use adhoc::*;
use serde::{Deserialize, Serialize};

pub mod wav;

const NORMALIZE_FACTOR: f32 = 1.0 / i16::MAX as f32;

fn normalize_sample<T>(samp: T) -> f32
where
    T: Clone,
    i32: From<T>,
{
    i32::from(samp) as f32 * NORMALIZE_FACTOR
}

fn truncate_sample(samp: f32) -> i16 {
    (samp.clamp(-1.0, 1.0) * (i16::MAX as f32)) as i16
}

#[derive(Copy, Clone, Default, Serialize, Deserialize)]
pub struct StreamInfo {
    pub sample_rate: u32,
    pub channels: u32,
}
impl StreamInfo {
    pub fn new(sample_rate: u32, channels: u32) -> Self {
        Self {
            sample_rate,
            channels,
        }
    }

    pub fn channels(&self) -> usize {
        self.channels as usize
    }

    pub fn frequency(&self) -> usize {
        self.sample_rate as usize
    }

    pub fn as_bytes(&self) -> &[u8] {
        let data = self as *const Self as *const u8;
        unsafe { std::slice::from_raw_parts(data, std::mem::size_of::<StreamInfo>()) }
    }
}

/// # Description
/// Designed for use in the browser so all audio data is assumed to be PCM INTERLEAVED with IEE754 values ranging from -1.0 to 1.0
/// ## Comments
/// - Some people may think it unusual to do audio stuff in f32 but WEBAUDIO API pretty much forces me to use them
pub trait Streamable {
    /// # Description
    /// encodes `samples` and returns number of samples encoded
    fn encode(&mut self, samples: &[f32]) -> Option<usize>;

    /// # Description
    /// Decodes the stream and write it out into `samples`
    /// ## Returns
    /// Number of samples decoded
    fn decode(&mut self, samples: &mut [f32]) -> Option<usize>;

    /// # Description
    /// Seeks to a certain spot in the stream
    /// # Parameters
    /// -`dt` is in milliseconds
    fn seek(&mut self, dt: SeekFrom);
}
