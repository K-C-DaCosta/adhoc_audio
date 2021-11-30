use std::{
    fmt::{Debug, Display},
    io::{Read, Seek, SeekFrom, Write},
    mem::{self},
    slice, str,
};

use super::{normalize_sample, truncate_sample, StreamInfo, Streamable};

#[derive(Debug)]
#[repr(C, align(1))]
struct RawWavHeader {
    riff: [u8; 4],
    file_size: u32,
    wave: [u8; 4],
    fmt: [u8; 4],
    cksize: u32,
    audio_format: i16,
    num_channels: i16,
    frequncy: i32,
    bytes_per_sec: i32,
    block_align: i16,
    bits_per_sample: i16,
    data_header: [u8; 4],
    file_size_data: u32,
}
impl Default for RawWavHeader {
    fn default() -> Self {
        Self {
            riff: [b'R', b'I', b'F', b'F'],
            file_size: 0,
            wave: [b'W', b'A', b'V', b'E'],
            fmt: [b'f', b'm', b't', b' '],
            cksize: 0,
            audio_format: 0,
            num_channels: 0,
            frequncy: 0,
            bytes_per_sec: 0,
            block_align: 0,
            bits_per_sample: 0,
            data_header: [b'd', b'a', b't', b'a'],
            file_size_data: 0,
        }
    }
}
impl Display for RawWavHeader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "riff:{}\n", str::from_utf8(&self.riff).ok().unwrap())?;
        write!(f, "file_size:{}\n", self.file_size)?;
        write!(f, "wave:{}\n", str::from_utf8(&self.wave).ok().unwrap())?;
        write!(f, "fmt:{}\n", str::from_utf8(&self.fmt).ok().unwrap())?;
        write!(f, "data_len:{}\n", self.cksize)?;
        write!(f, "sample_type:{}\n", self.audio_format)?;
        write!(f, "frequency:{}\n", self.frequncy)?;
        write!(f, "bytes_per_sec:{}\n", self.bytes_per_sec)?;
        write!(f, "block_align:{}\n", self.block_align)?;
        write!(f, "bits_per_sample:{}\n", self.bits_per_sample)?;
        write!(
            f,
            "data_header:{}\n",
            str::from_utf8(&self.data_header).ok().unwrap()
        )?;
        write!(f, "file_size(data):{}\n", self.file_size_data)?;

        Ok(())
    }
}

impl RawWavHeader {
    pub fn as_bytes(&self) -> &[u8] {
        unsafe {
            let bytes: *const u8 = mem::transmute(self);
            slice::from_raw_parts(bytes, mem::size_of::<RawWavHeader>())
        }
    }
    pub fn as_bytes_mut(&mut self) -> &mut [u8] {
        unsafe {
            let bytes: *mut u8 = mem::transmute(self);
            slice::from_raw_parts_mut(bytes, mem::size_of::<RawWavHeader>())
        }
    }
}

pub struct WavCodec {
    info: StreamInfo,
    /// PCM data in bytes (internally it is i16 )
    pcm: Vec<u8>,
    /// cursor is index by sample (i16) not byte position
    short_cursor: u64,
}

impl WavCodec {
    pub fn new(info: StreamInfo) -> Self {
        Self {
            info,
            pcm: Vec::new(),
            short_cursor: 0,
        }
    }
    
    pub fn info(&self) -> StreamInfo {
        self.info
    }

    /// # Description
    /// Writes wav file to `Resource`
    /// ## Comments
    /// - `Resource` is usually `Vec<u8>` or  `fs::File`
    /// look at the tests for examples on how to use it
    pub fn save_to<Resource>(&self, mut dst: Resource) -> Result<(), &'static str>
    where
        Resource: Write,
    {
        let mut header = RawWavHeader::default();
        header.file_size = mem::size_of::<RawWavHeader>() as u32 - 8 + self.pcm.len() as u32;
        header.frequncy = self.info.sample_rate as i32;
        header.audio_format = 1;
        header.num_channels = self.info.channels as i16;
        header.cksize = 16;
        header.bytes_per_sec = (header.frequncy * header.num_channels as i32 * 16) / 8;
        header.block_align = (header.num_channels * 16) / 8;
        header.bits_per_sample = 16;
        header.file_size_data = self.pcm.len() as u32;
        // println!("header:\n{}\n",header);
        dst.write(header.as_bytes())
            .map_err(|_| "failed to write header")?;
        dst.write_all(&self.pcm)
            .map_err(|_| "failed to write pcm data")
    }

    /// parses and loads wav file
    pub fn load<Resource>(mut wav_res: Resource) -> Result<Self, &'static str>
    where
        Resource: Read + Seek,
    {
        let mut header_binary = [0u8; 256];

        wav_res
            .read_exact(&mut header_binary)
            .map_err(|_| "failed to read header bytes")?;

        let mut header_slice = &header_binary[..];
        let mut header = RawWavHeader::default();

        // println!("buffer size ={}", wav_binary.len());

        let (data_start, _) = header_binary
            .windows(4)
            .enumerate()
            .find(|(_, window)| {
                let number = [b'd', b'a', b't', b'a'];
                window == &number
            })
            .ok_or("data region not found")?;

        header_slice
            .clone()
            .read(header.as_bytes_mut())
            .map_err(|_| "header failed to read")?;

        header
            .data_header
            .iter_mut()
            .zip(header_slice[data_start..data_start + 4].iter())
            .for_each(|(data_header_byte, &b)| {
                *data_header_byte = b;
            });

        header_slice = &header_slice[data_start + 4..];
        header.file_size_data = u32::from_le_bytes([
            header_slice[0],
            header_slice[1],
            header_slice[2],
            header_slice[3],
        ]);

        if header.bits_per_sample != 16 && header.bits_per_sample != 8 {
            return Err("invalid bits per sample, either 16 or 8 bits per sample is supported ");
        }
        if header.audio_format != 1 {
            return Err("formats other than PCM liner isn't supported");
        }

        // println!("parsed header: {}",header);
        // println!("pcm data = {} byles", wav_binary.len());

        wav_res
            .seek(SeekFrom::Start(data_start as u64 + 8))
            .map_err(|_| "failed to seek to pcm data")?;

        let mut pcm = Vec::<u8>::new();
        wav_res
            .read_to_end(&mut pcm)
            .map_err(|_| "failed to read to end")?;

        if header.bits_per_sample == 8 {
            //convert 8bit stream to 16bit
            let len = pcm.len();
            pcm.resize(pcm.len() * 2, 0);
            for k in (0..len).rev() {
                let sample = pcm[k];
                let scaled_sample = ((sample as f32 / 255.0) * 2.0 - 1.0).clamp(-1.0, 1.0)
                    * ((i16::MAX - 1) as f32);
                let sample = scaled_sample as i16;
                pcm[2 * k + 0] = ((sample >> 0) & 0xff) as u8;
                pcm[2 * k + 1] = ((sample >> 8) & 0xff) as u8;
            }
        }

        // println!("header:\n{}\n", header);

        Ok(Self {
            info: StreamInfo {
                sample_rate: header.frequncy as u32,
                channels: header.num_channels as u32,
            },
            pcm,
            short_cursor: 0,
        })
    }

    fn num_samples(&self) -> usize {
        self.pcm.len() / 2
    }

    fn get_pcm(pcm: &Vec<u8>) -> &[i16] {
        unsafe { slice::from_raw_parts(pcm.as_ptr() as *const i16, pcm.len() / 2) }
    }

    #[allow(dead_code)]
    fn get_pcm_mut(pcm: &mut Vec<u8>) -> &mut [i16] {
        unsafe { slice::from_raw_parts_mut(pcm.as_ptr() as *mut i16, pcm.len() / 2) }
    }

    /// writes a sample at current cursor positions (will offset the cursor)
    fn write_sample(&mut self, truncated_sample: i16) {
        let upper_byte = ((truncated_sample >> 8) & 0xff) as u8;
        let lower_byte = ((truncated_sample >> 0) & 0xff) as u8;
        if (self.short_cursor as usize) < self.num_samples() {
            let byte_index = (self.short_cursor as usize) << 1;
            self.pcm[byte_index + 0] = lower_byte;
            self.pcm[byte_index + 1] = upper_byte;
        } else {
            self.pcm.push(lower_byte);
            self.pcm.push(upper_byte);
        }
        self.offset_cursor(1);
    }

    fn offset_cursor(&mut self, offset: i64) {
        self.short_cursor = (self.short_cursor as i64 + offset).max(0) as u64;
    }
}
impl Streamable for WavCodec {
    fn encode(&mut self, samples: &[f32]) -> Option<usize> {
        let num_channels = self.info.channels();
        let valid_len = (samples.len() / num_channels) * num_channels;
        for &samp in &samples[0..valid_len] {
            let sample_i16 = truncate_sample(samp);
            self.write_sample(sample_i16);
        }
        Some(valid_len)
    }
    fn decode(&mut self, out: &mut [f32]) -> Option<usize> {
        let mut out_cursor = 0;

        let num_channels = self.info().channels();
        let stream_length = self.num_samples() as u64;
        let cursor = &mut self.short_cursor;
        let samples_list = Self::get_pcm(&self.pcm);

        //makes sure we can't write partial PCM 'blocks'
        let valid_length = (out.len()/num_channels)*num_channels;

        while *cursor < stream_length && out_cursor < valid_length {
            let sample_i16 = samples_list[*cursor as usize];
            let normalized_sample = normalize_sample(sample_i16);
            out[out_cursor] = normalized_sample;
            *cursor += 1;
            out_cursor += 1;
        }

        (out_cursor > 0).then(|| out_cursor)
    }

    fn seek(&mut self, dt: SeekFrom) {
        let sample_rate = self.info.sample_rate as f32;
        let num_channels = self.info.channels() as i64;
        let num_samples = self.num_samples() as i64;

        let sample_rate_in_milliseconds = sample_rate / 1000.0;

        match dt {
            SeekFrom::Current(dt) => {
                let sample_offset = (sample_rate_in_milliseconds * (dt as f32)) as i64;
                let block_index = (self.short_cursor as i64 / num_channels) * num_channels;
                self.short_cursor =
                    (block_index + sample_offset * num_channels).clamp(0, num_samples - 1) as u64;
            }
            SeekFrom::Start(dt) => {
                let sample_offset = (sample_rate_in_milliseconds * (dt as f32)) as i64;
                self.short_cursor = (sample_offset * num_channels).clamp(0, num_samples - 1) as u64;
            }
            SeekFrom::End(dt) => {
                let sample_offset = (sample_rate_in_milliseconds * (dt as f32)) as i64;
                let block_index = num_samples - num_samples;
                self.short_cursor =
                    (block_index + sample_offset * num_channels).clamp(0, num_samples - 1) as u64;
            }
        }
    }
}

mod test {
    #[allow(unused_imports)]
    use super::{StreamInfo, Streamable, WavCodec};
    #[allow(unused_imports)]
    use std::{
        fs,
        io::{Cursor, Read, Seek, SeekFrom, Write},
    };

    #[test]
    fn parse_wav_then_re_export_copy_to_disk() {
        let file_pointer = fs::File::open("./resources/folly.wav").expect("misisng file");
        let wav_codec = WavCodec::load(file_pointer).unwrap();
        let new_file =
            fs::File::create("./resources/folly_out.wav").expect("failed to create file");
        wav_codec.save_to(new_file).expect("failed to write");
    }

    #[test]
    fn parse_wav_then_re_export_to_main_memory() {
        let mut file_pointer = fs::File::open("./resources/folly.wav").expect("misisng file");
        //write the wav file into the wav_binary buffer
        let mut wav_binary = Vec::new();
        file_pointer
            .read_to_end(&mut wav_binary)
            .expect("failed to write into a buffer");

        //parse the wav file and extract audio and properties of the audio stream
        let wav_codec = WavCodec::load(Cursor::new(wav_binary)).unwrap();

        //save wav file binary to vector living in main-memory
        let mut new_wav_file = Vec::new();
        wav_codec
            .save_to(&mut new_wav_file)
            .expect("failed to write");
    }

    #[test]
    fn parse_wav_then_re_export_by_encode_then_copy_to_disk() {
        let file_pointer = fs::File::open("./resources/taunt.wav").expect("misisng file");
        let mut wav_codec = WavCodec::load(file_pointer).unwrap();
        let mut new_wav = WavCodec::new(wav_codec.info());
        let mut buffer = [0.0; 1024];

        //seek 1min into the track
        // wav_codec.seek(SeekFrom::Start(60_000));

        //start writing from 1minute mark
        while let Some(samples_read) = wav_codec.decode(&mut buffer) {
            new_wav.encode(&buffer[0..samples_read]);
        }

        // wav_codec.seek(SeekFrom::Start(0));
        // new_wav.seek(SeekFrom::Start(0));
        // while let Some(samples_read) = wav_codec.decode(&mut buffer) {
        //     new_wav.encode(&buffer[0..samples_read]);
        // }

        let new_file =
            fs::File::create("./resources/taunt_copy.wav").expect("failed to create file");
        new_wav.save_to(new_file).expect("failed to write");
    }
}
