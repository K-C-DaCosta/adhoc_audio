use super::*;
use std::{
    collections::VecDeque,
    ops::{Deref, DerefMut},
};

mod audio_stream;
pub use audio_stream::*;

mod frame;
pub use frame::*;

pub const MAX_DECODE_ATTEMPTS: usize = 10;

#[derive(Copy, Clone)]
pub enum CodecState {
    Init,
    Encoding,
    Decoding,
}

/// # Description
/// A compressed audio format cobbled together to handle audio on the web
pub struct AdhocCodec {
    frame_header_list: FrameHeaders,
    channel_state_list: Vec<FrameCodec>,
    stream: AudioStream,
    deinterleaved_channel: Vec<f32>,
}
impl AdhocCodec {
    pub fn new() -> Self {
        Self {
            stream: AudioStream::new(),
            frame_header_list: FrameHeaders::new(),
            deinterleaved_channel: Vec::new(),
            channel_state_list: Vec::new(),
        }
    }

    pub fn set_info(&mut self, info: StreamInfo) {
        let num_channels = info.channels as usize;
        self.channel_state_list
            .resize(num_channels, FrameCodec::new());
        self.stream.set_info(Some(info));
    }

    //switches to decode mode
    pub fn init(&mut self) {
        self.stream.seek_start();
        self.frame_header_list.reset();
        self.channel_state_list.iter_mut().for_each(|cs| cs.init())
    }


    fn encode(&mut self, interleaved_pcm: &[f32]) {
        let num_channels = self.stream.info().expect("info not set").channels as usize;
        let num_chunks = interleaved_pcm.len() / num_channels;

        //split borrows
        let channel_list = &mut self.channel_state_list;
        let stream = &mut self.stream;
        let block_info = &mut self.frame_header_list;
        let deinterleaved_channel = &mut self.deinterleaved_channel;

        deinterleaved_channel.resize(num_chunks, 0.0);

        //de-interleave pcm by channel
        for channel_idx in 0..num_channels {
            //fill buffer with channel number 'channel_idx'
            for chunk_idx in 0..num_chunks {
                deinterleaved_channel[chunk_idx] =
                    interleaved_pcm[(chunk_idx * num_channels) + channel_idx];
            }
            // println!("{:?}", deinterleaved_channel);
            channel_list[channel_idx].encode_frame(stream, block_info, &deinterleaved_channel);
        }
    }

    fn decode(&mut self, pcm_out: &mut [f32]) -> usize {
        let num_channels = self.stream.info().expect("info not set").channels as usize;
        let channel_list = &mut self.channel_state_list;
        let stream = &mut self.stream;
        let block_info = &mut self.frame_header_list;

        let is_buffers_empty = |channel_list: &mut Vec<FrameCodec>| {
            channel_list
                .iter()
                .all(|cs| cs.buffered_channel().is_empty())
        };

        let mut buffer_audio = |channel_list: &mut Vec<FrameCodec>| {
            let mut attempts = 0;
            //decode and load buffers if empty
            while attempts < MAX_DECODE_ATTEMPTS && is_buffers_empty(channel_list) {
                //decode more data
                for channel_idx in 0..num_channels {
                    channel_list[channel_idx].decode_frame(stream, block_info);
                }
                attempts += 1;
            }
            attempts
        };

        // number of samples that can be written has to number a multiple of `num_channels`
        let legal_output_len = (pcm_out.len() / num_channels) * num_channels;
        let mut pcm_out_cursor = 0;

        while pcm_out_cursor < legal_output_len {
            if buffer_audio(channel_list) >= MAX_DECODE_ATTEMPTS {
                break;
            }

            let mut channel_idx = 0;
            while (pcm_out_cursor < legal_output_len) && (channel_idx < num_channels) {
                let decoded_sample = channel_list[channel_idx]
                    .buffered_channel_mut()
                    .pop_front()
                    .unwrap_or_default();
                pcm_out[pcm_out_cursor] = decoded_sample;
                pcm_out_cursor += 1;
                channel_idx += 1;
            }
        }
        pcm_out_cursor
    }
}

impl Streamable for AdhocCodec {
    fn encode(&mut self, samples: &[f32]) -> Option<usize> {
        self.encode(samples);
        Some(samples.len())
    }
    fn decode(&mut self, samples: &mut [f32]) -> Option<usize> {
        let samples_read = self.decode(samples);
        (samples_read > 0).then(|| samples_read)
    }

    fn seek(&mut self, dt: SeekFrom) {
        let info = self.stream.info().expect("info not initalized");
        let channels = info.channels as u64;
        let frequency_in_millis = info.sample_rate as f32 / 1000.0;

        self.init();

        let frame_header_list = &mut self.frame_header_list;
        let channel_state_list = &mut self.channel_state_list;
        let stream = &mut self.stream;

        match dt {
            SeekFrom::Start(dt) => {
                let offset = (frequency_in_millis * (dt as f32)) as u64;
                let offset_in_blocks = offset * channels;

                let mut samples_skipped = 0;
                let mut header_block_index = 0;

                //number of headers should be a multiple of channels
                let header_block_len = frame_header_list.len() / channels as usize;

                //find frame that is within the seek interval
                while header_block_index < header_block_len && samples_skipped < offset_in_blocks {
                    let header = frame_header_list
                        .get(header_block_index * channels as usize)
                        .expect("error while fence seeking");

                    let header_size = header.size as u64 * channels;

                    if (samples_skipped + header_size) >= offset_in_blocks {
                        break;
                    }

                    samples_skipped += header_size;
                    header_block_index += 1;
                }

                //pinned down start header
                let start_header_index = header_block_index * channels as usize;
                let start_header = frame_header_list
                    .get(start_header_index)
                    .expect("start frame failed to fetch");

                //make sure cursor in bitstream is set properly
                stream.set_bit_cursor(start_header.bit_cursor as u128);

                //make sure cursor is set at the 'start header index'
                frame_header_list.set_cursor(start_header_index as u32);


                //for each frame in the frame-block
                for offset in 0..channels {
                    let offset = offset as usize;
                    let header = frame_header_list
                        .get(start_header_index + offset)
                        .expect("frame fence post");

                    //make sure stack history from the header is transferred to state
                    // 0..3 because sample history is supposed to have 3 samples 
                    for k in 0..3 {
                        channel_state_list[offset]
                            .sample_history_mut()
                            .push(header.stack_history[k]);
                    }

                    //make sure buffers are clear
                    let codec = &mut channel_state_list[offset];

                    codec.buffered_channel_mut().clear();

                    //transfer codec state from frame to codec
                    *codec.state_mut() = header
                        .is_init
                        .then(|| CodecState::Init)
                        .unwrap_or(CodecState::Decoding);

                    codec.decode_frame(stream, frame_header_list);
                }

                //prune samples in buffer until the number of samples skipped has been reached
                while samples_skipped < offset_in_blocks {
                    for offset in 0..channels {
                        let offset = offset as usize;
                        let codec = &mut channel_state_list[offset];
                        codec.buffered_channel_mut().pop_front();
                        samples_skipped += 1;
                    }
                }
            }
            _ => {
                unimplemented!("not implemented, SeekFrom::Start(..) is currently supported")
            }
        }
    }
}

mod test {
    #[allow(unused_imports)]
    use super::{AdhocCodec, StreamInfo, Streamable};

    #[allow(unused_imports)]
    use crate::codec::wav::WavCodec;
    
    #[allow(unused_imports)]
    use crate::math;

    #[allow(unused_imports)]
    use crate::signal;

    #[allow(unused_imports)]
    use std::{
        fs::File,
        io::{Cursor, Read, Write,SeekFrom},
    };

    #[test]
    fn sanity() {
        let mut codec = AdhocCodec::new();
        let data = [
            0.0, 0.0, 0.01, 0.01, 0.02, 0.02, 0.03, 0.03, 0.019, 0.020, 0.019, 0.018, 0.017,
        ];

        codec.set_info(StreamInfo {
            channels: 1,
            sample_rate: 44100,
        });

        codec.encode(&data);

        let mut out_buffer = data.clone();
        codec.init();
        let samples_read = codec.decode(&mut out_buffer[..]);
        let mean_squared_error = math::compute_mse(&out_buffer, &data);

        println!("data   :{:?}", data);
        println!("decoded:{:?}", &out_buffer[0..samples_read]);
        println!(
            "blocks allocated = {}\nbits written = {}\ncompression_ratio={}\nMean Squared Error={:.6}",
            codec.stream.binary.len(),
            codec.stream.capacity(),
            codec.stream.capacity() as f32 / (data.len() * 16) as f32,
            mean_squared_error
        );

        for (&input_sample, &decoded_sample) in data.iter().zip(out_buffer.iter()) {
            let close_enough = (input_sample - decoded_sample).abs() < 0.01;
            assert_eq!(close_enough, true, "accuracy threshold not met");
        }
    }

    #[test]
    fn sanity2() {
        let mut codec = AdhocCodec::new();
        let data = [
            0.0, 0.0, 0.1, 0.01, 0.2, 0.02, 0.3, 0.03, 0.4, 0.020, 0.5, 0.018, 0.6, 0.019,
        ];

        codec.set_info(StreamInfo {
            channels: 2,
            sample_rate: 44100,
        });

        codec.encode(&data[0..8]);
        codec.encode(&data[8..]);

        let mut out_buffer = data.clone();
        codec.init();
        let samples_read = codec.decode(&mut out_buffer[..]);
        let mean_squared_error = math::compute_mse(&out_buffer, &data);

        println!("data   :{:?}", data);
        println!("decoded:{:?}", &out_buffer[0..samples_read]);
        println!(
            "blocks allocated = {}\nbits written = {}\ncompression_ratio={}\nMean Squared Error={:.6}",
            codec.stream.binary.len(),
            codec.stream.capacity(),
            codec.stream.capacity() as f32 / (data.len() * 16) as f32,
            mean_squared_error
        );

        for (&input_sample, &decoded_sample) in data.iter().zip(out_buffer.iter()) {
            let close_enough = (input_sample - decoded_sample).abs() < 0.01;
            assert_eq!(close_enough, true, "accuracy threshold not met");
        }
    }

    #[test]
    fn re_encode_wav() {
        let mut wav_data =
            WavCodec::load(File::open("./resources/folly.wav").expect("file not found")).unwrap();

        let mut buffer = [0.0f32; 1024];
        let mut adhoc_codec = AdhocCodec::new();

        adhoc_codec.set_info(StreamInfo::new(44_100, 2));
        while let Some(samps_decoded) = wav_data.decode(&mut buffer) {
            adhoc_codec.encode(&buffer[0..samps_decoded]);
        }

       
        adhoc_codec.seek(SeekFrom::Start(0));

        let mut out_wav = WavCodec::new(StreamInfo::new(44_100, 2));
        while let Some(samps_decoded) =
            <AdhocCodec as Streamable>::decode(&mut adhoc_codec, &mut buffer)
        {
            out_wav.encode(&buffer[0..samps_decoded]);
        }

        out_wav
            .save_to(File::create("./resources/adhoc_folly.wav").unwrap())
            .unwrap();
    }
}
