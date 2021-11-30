use serde::{Deserialize, Serialize};

use crate::collections::BitVec;

use super::*;

#[derive(Serialize, Deserialize)]
pub struct FrameHeader {
    pub exponent: u8,
    pub size: u16,
    pub bit_cursor: u128,
    pub is_init: bool,
    pub stack_history: [i16; 3],
}

/// Encoder compresses audio in 'blocks'
/// this struct stores compact infomation about every block in the stream
#[derive(Serialize, Deserialize)]
pub struct FrameHeaders {
    /// its stores `log_2(divisor)`, where `divisor =  2^k`, for some k
    divisor_exp_list: NibbleList,
    is_init_frame_list: BitVec,
    frame_size_list: Vec<u16>,
    bit_cursor_list: Vec<u64>,
    stack_history_list: Vec<[i16; 3]>,
    header_cursor: u32,
}

impl FrameHeaders {
    pub fn new() -> Self {
        Self {
            is_init_frame_list: BitVec::new(),
            divisor_exp_list: NibbleList::new(),
            frame_size_list: Vec::new(),
            bit_cursor_list: Vec::new(),
            stack_history_list: Vec::new(),
            header_cursor: 0,
        }
    }

    pub fn push(&mut self, header: FrameHeader) {
        let FrameHeader {
            exponent,
            size: num_samples,
            bit_cursor,
            is_init,
            stack_history,
        } = header;

        let header_cursor = self.header_cursor as usize;
        let is_init = is_init as u64;

        if header_cursor >= self.divisor_exp_list.len() {
            self.divisor_exp_list.push(exponent);
            self.frame_size_list.push(num_samples);
            self.bit_cursor_list.push(bit_cursor as u64);
            self.is_init_frame_list.push(is_init);
            self.stack_history_list.push(stack_history);
        } else {
            self.divisor_exp_list.set(header_cursor, exponent);
            self.is_init_frame_list.set(header_cursor, is_init);
            self.frame_size_list[header_cursor] = num_samples;
            self.bit_cursor_list[header_cursor] = bit_cursor as u64;
            self.stack_history_list[header_cursor] = stack_history;
        }

        self.header_cursor += 1;
    }

    pub fn get(&self, index: usize) -> Option<FrameHeader> {
        (index < self.len()).then(|| {
            let exponent = self.divisor_exp_list.get(index);
            let is_init = self.is_init_frame_list.get(index);
            let size = self.frame_size_list[index];
            let cursor = self.bit_cursor_list[index];
            let history = self.stack_history_list[index];
            FrameHeader {
                exponent,
                size,
                is_init: is_init == 1,
                bit_cursor: cursor as u128,
                stack_history: history,
            }
        })
    }

    pub fn set_cursor(&mut self, idx: u32) {
        self.header_cursor = idx;
    }

    pub fn len(&self) -> usize {
        self.divisor_exp_list.len()
    }

    pub fn reset(&mut self) {
        self.header_cursor = 0;
    }
}

impl Iterator for FrameHeaders {
    type Item = FrameHeader;
    fn next(&mut self) -> Option<Self::Item> {
        let header_cursor = self.header_cursor as usize;
        (header_cursor < self.divisor_exp_list.len()).then(|| {
            let header = FrameHeader {
                exponent: self.divisor_exp_list.get(header_cursor),
                size: self.frame_size_list[header_cursor],
                bit_cursor: self.bit_cursor_list[header_cursor] as u128,
                is_init: self.is_init_frame_list.get(header_cursor) == 1,
                stack_history: self.stack_history_list[header_cursor],
            };
            self.header_cursor += 1;
            header
        })
    }
}

#[derive(Clone)]
/// # Description
/// Codec for compressing a single frame of audio, where a frame is a collection of samples from a single channel
pub struct FrameCodec {
    state: CodecState,
    sample_history: CircularStack<i16>,
    buffered_channel: VecDeque<f32>,
}

impl FrameCodec {
    pub fn new() -> Self {
        Self {
            state: CodecState::Init,
            sample_history: CircularStack::new(),
            buffered_channel: VecDeque::new(),
        }
    }
    pub fn state_mut(&mut self) -> &mut CodecState {
        &mut self.state
    }
    pub fn sample_history_mut(&mut self) -> &mut CircularStack<i16> {
        &mut self.sample_history
    }

    pub fn init(&mut self) {
        self.state = CodecState::Init;
        self.buffered_channel.clear();
    }

    /// encodes a single channel
    pub fn encode_frame(
        &mut self,
        stream: &mut AudioStream,
        frame_headers: &mut FrameHeaders,
        pcm: &[f32],
    ) {
        let mut parabola = FixedParabola::new();
        let mut max_entropy: i16 = 0;
        let sample_history = &mut self.sample_history;

        match self.state {
            CodecState::Init => {
                // save bit cursor before modifying stream
                let bit_cursor = stream.bit_cursor();

                // write first three samples into the stream
                // and also record then in the history queue
                for k in 0..3 {
                    let samp = pcm[k];
                    let truncated = truncate_sample(samp);
                    stream.write::<u16>(truncated as u16);
                    sample_history.push(truncated);
                }

                let divisor_exp = Self::compute_optimal_divisor_exponent(sample_history, &pcm[3..]);

                frame_headers.push(FrameHeader {
                    exponent: divisor_exp as u8,
                    size: pcm.len() as u16,
                    bit_cursor,
                    is_init: true,
                    stack_history: [
                        sample_history.prev(3),
                        sample_history.prev(2),
                        sample_history.prev(1),
                    ],
                });

                //entropy encode
                for &sample in pcm[3..].iter() {
                    let current = truncate_sample(sample);

                    parabola.f = [
                        normalize_sample(sample_history.prev(3)),
                        normalize_sample(sample_history.prev(2)),
                        normalize_sample(sample_history.prev(1)),
                    ];
                    parabola.compute_coefs();
                    let predicted = truncate_sample(parabola.eval(3.0));

                    let entropy = (current as i32 - predicted as i32)
                        .clamp(i16::MIN as i32 + 1, i16::MAX as i32 - 1)
                        as i16;

                    max_entropy = max_entropy.max(entropy);
                    stream.write_compressed(divisor_exp, entropy);
                    sample_history.push(current);
                }
                // println!("MAX ENTROPY={}", max_entropy);
                self.state = CodecState::Encoding;
            }
            CodecState::Encoding => {
                let divisor_exp = Self::compute_optimal_divisor_exponent(&sample_history, &pcm[..]);

                frame_headers.push(FrameHeader {
                    exponent: divisor_exp as u8,
                    size: pcm.len() as u16,
                    bit_cursor: stream.bit_cursor(),
                    is_init: false,
                    stack_history: [
                        sample_history.prev(3),
                        sample_history.prev(2),
                        sample_history.prev(1),
                    ],
                });

                for &sample in pcm[..].iter() {
                    let current = truncate_sample(sample);

                    parabola.f = [
                        normalize_sample(sample_history.prev(3)),
                        normalize_sample(sample_history.prev(2)),
                        normalize_sample(sample_history.prev(1)),
                    ];
                    parabola.compute_coefs();
                    let predicted = truncate_sample(parabola.eval(3.0));

                    let entropy = (current as i32 - predicted as i32)
                        .clamp(i16::MIN as i32 + 1, i16::MAX as i32 - 1)
                        as i16;

                    max_entropy = max_entropy.max(entropy);
                    stream.write_compressed(divisor_exp, entropy);
                    sample_history.push(current);
                }
            }
            _ => panic!("invalid state"),
        }
    }

    /// decodes a single channel
    pub fn decode_frame(
        &mut self,
        stream: &mut AudioStream,
        frame_info: &mut FrameHeaders,
    ) -> Option<usize> {
        let mut parabola = FixedParabola::new();
        let sample_history = &mut self.sample_history;
        let sample_buffer = &mut self.buffered_channel;

        frame_info.next().map(|FrameHeader { exponent, size, .. }| {
            match self.state {
                CodecState::Init => {
                    let num_samples_pre_read = 3;
                    //write starting samples into the stream
                    for _ in 0..num_samples_pre_read {
                        let samp = stream.read::<u16>() as i16;
                        sample_history.push(samp);
                    }
                    for k in 0..num_samples_pre_read {
                        let offset = num_samples_pre_read as u32 - k as u32;
                        let decoded_sample = normalize_sample(sample_history.prev(offset));
                        sample_buffer.push_back(decoded_sample);
                    }
                    let mut samples_read = num_samples_pre_read as usize;
                    //entropy decode
                    for _ in 3..size {
                        let entropy = stream.read_compressed(exponent as i16);
                        parabola.f = [
                            normalize_sample(sample_history.prev(3)),
                            normalize_sample(sample_history.prev(2)),
                            normalize_sample(sample_history.prev(1)),
                        ];
                        parabola.compute_coefs();
                        let predicted = truncate_sample(parabola.eval(3.0));

                        //casted to i32 to avoid overflow issues
                        let current = (entropy as i32 + predicted as i32)
                            .clamp(i16::MIN as i32, i16::MAX as i32)
                            as i16;

                        let decoded_sample = normalize_sample(current).clamp(-1.0, 1.0);
                        sample_buffer.push_back(decoded_sample);
                        sample_history.push(current);
                        samples_read += 1;
                    }

                    self.state = CodecState::Decoding;
                    samples_read
                }
                CodecState::Decoding => {
                    let mut samples_read = 0;
                    //entropy decode
                    for _ in 0..size {
                        let entropy = stream.read_compressed(exponent as i16);
                        parabola.f = [
                            normalize_sample(sample_history.prev(3)),
                            normalize_sample(sample_history.prev(2)),
                            normalize_sample(sample_history.prev(1)),
                        ];
                        parabola.compute_coefs();
                        let predicted = truncate_sample(parabola.eval(3.0));

                        //casted to i32 to avoid overflow issues
                        let current = (entropy as i32 + predicted as i32)
                            .clamp(i16::MIN as i32, i16::MAX as i32)
                            as i16;

                        let decoded_sample = normalize_sample(current).clamp(-1.0, 1.0);
                        sample_buffer.push_back(decoded_sample);
                        sample_history.push(current);
                        samples_read += 1;
                    }
                    samples_read
                }
                _ => panic!("invalid state"),
            }
        })
    }

    fn compute_optimal_divisor_exponent(
        sample_history_ref: &CircularStack<i16>,
        remaining_samples: &[f32],
    ) -> i16 {
        let mut bit_sum_table = [0; 16];
        let mut parabola = FixedParabola::new();
        let mut sample_history = sample_history_ref.clone();
        for &sample in remaining_samples.iter() {
            let current = truncate_sample(sample);
            parabola.f = [
                normalize_sample(sample_history.prev(3)),
                normalize_sample(sample_history.prev(2)),
                normalize_sample(sample_history.prev(1)),
            ];
            parabola.compute_coefs();
            let predicted = truncate_sample(parabola.eval(3.0));
            let entropy = (current as i32 - predicted as i32).abs() as u32;
            for k in 1..16 {
                let quotient_bits = (entropy >> k) + 1;
                let remainder_bits = k as u32;
                bit_sum_table[k] += quotient_bits + remainder_bits + 1;
            }
            sample_history.push(current);
        }

        bit_sum_table
            .iter()
            .enumerate()
            .skip(1)
            .filter(|&(_, &b)| b > 0)
            .min_by(|(_, a), (_, b)| a.cmp(b))
            .map(|(idx, _)| idx as i16)
            .unwrap_or(1)
    }

    pub fn buffered_channel(&self) -> &VecDeque<f32> {
        &self.buffered_channel
    }

    pub fn buffered_channel_mut(&mut self) -> &mut VecDeque<f32> {
        &mut self.buffered_channel
    }
}
