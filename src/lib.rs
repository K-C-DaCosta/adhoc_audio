//! # Adhoc Audio
//! A crate for compressing audio written in pure-rust
//! 
//! # Example - compressing a wav file 
//! ```
//! use adhoc_audio::{codec::Streamable, AdhocCodec, WavCodec};
//! use std::fs::File;
//!
//! fn main() {
//!     println!("compressing file example..");
//! 
//!     //set up a buffer for reading/writing samples
//!     let mut samples = [0.0; 1024];
//! 
//!     //open wav file
//!     let mut wav_reader = WavCodec::load(
//!         File::open("./resources/taunt.wav").unwrap()
//!     ).unwrap();
//! 
//!     let mut adhoc = AdhocCodec::new()
//!         // compression-level 0 means 'lossless'
//!         // while levels 1-10 does quantization + dithering (quality suffers)
//!         .with_compression_level(7)
//!         // AdhocCodec::with_info(.. ) MUST BE CALLED before calling 
//!         // encode/decode when you are creating a new instance of AdhocCodec
//!         .with_info(wav_reader.info());
//! 
//!     //'decode' wav stream bit-by-bit
//!     //Note:in this case we are just reading PCM info
//!     while let Some(samples_read) = wav_reader.decode(&mut samples) {
//!         //encode wav data bit-by-bit
//!         //memory is allocated as needed
//!         adhoc.encode(&samples[0..samples_read]);
//!     }
//! 
//!     //write compressed audio back to disk
//!     adhoc
//!         .save_to(File::create("./resources/taunt.adhoc").unwrap())
//!         .unwrap();
//! 
//!     println!("taunt.adhoc written to: ./resources");
//! }
//! ```

/// all the audio stuff is in here 
pub mod codec;
mod collections;
mod math;
pub use codec::{adhoc::AdhocCodec, wav::WavCodec, StreamInfo, Streamable};
pub use std::io::SeekFrom;
