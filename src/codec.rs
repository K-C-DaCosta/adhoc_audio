use super::{
    collections::{BitStream, CircularStack, NibbleList},
    math::FixedParabola,
};
use serde::{Deserialize, Serialize};
pub use std::io::{Seek, SeekFrom};

/// The first codec in this crate that actually compresses things
pub mod adhoc;
/// A utility for Reading/Writing wav files
pub mod wav;

pub use adhoc::*;

const NORMALIZE_FACTOR: f32 = 1.0 / i16::MAX as f32;

fn normalize_sample<T>(samp: T) -> f32
where
    T: Clone,
    i32: From<T>,
{
    i32::from(samp) as f32 * NORMALIZE_FACTOR
}

fn truncate_sample(samp: f32) -> i16 {
    (samp.clamp(-1.0, 1.0) * ((i16::MAX) as f32)) as i16
}

#[derive(Copy, Clone, Default, Serialize, Deserialize)]
/// A POD that contains basic information about an audio signal
/// ## Comments
/// If you instantiate a codec make sure this is set before you encode(..)/decode(..)
pub struct StreamInfo {
    /// `sample_rate` is in Hz so typically you set this to something like: `44_100` or `48_000`
    sample_rate: u32,
    /// number of channels in your stream
    channels: u32,
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
/// all codecs in this crate implement this trait to expose the `encode(..)`,`decode(..)` and `seek(..)` routines
/// # Description
/// Designed for use in the browser so all audio data is assumed to be PCM INTERLEAVED with IEE754 values ranging from -1.0 to 1.0
/// ## Comments
/// - Some people may think it unusual to do audio stuff in f32 but WEBAUDIO API pretty much forces me to use them
pub trait Streamable {
    /// # Description
    /// returns fundamental information about the stream
    fn info(&self) -> StreamInfo;

    /// # Description
    /// returns a **tight upperbound** of bits neeeded to store encoded data
    /// ## Comments
    /// - can be used to set filesize limits
    fn filesize_upperbound(&self) -> u64;

    /// # Description
    /// encodes `samples` and returns number of samples encoded
    fn encode(&mut self, samples: &[f32]) -> Option<usize>;

    /// # Description
    /// Decodes part of the stream and writes it out into the `samples` buffer
    /// ## Returns
    /// Number of samples decoded
    fn decode(&mut self, samples: &mut [f32]) -> Option<usize>;

    /// # Description
    /// Seeks to a certain spot in the stream
    /// ## Parameters
    /// - `dt` is change in time in milliseconds
    /// ## Comments
    /// - Notes about `AdhocCodec`:
    ///     - Currently only `SeekFrom::Start` is implemented 
    ///     - Intented to be used **ONLY AFTER** you've completely finished encoding \
    ///     audio, or you have just loaded the codec for the first time
    ///     - with `AdhocCodec` you can't just seek to a random spot and start encoding
    ///     - If you want to reuse the memory allocated call `AdhocCodec::init()` or `dt=SeekFrom::Start(0)` \
    ///     before encoding. This will reuse the stream memory allocated.
    /// - Notes about `WavCodec`:
    ///     - `SeekFrom::Start`, `SeekFrom::Current` and `SeekFrom::End` is implemented
    ///     - there no restrictions on how one should call this after encode/decode
    fn seek(&mut self, dt: SeekFrom);
}
