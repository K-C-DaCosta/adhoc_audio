# Adhoc Audio
Audio compression written in pure rust. 
**It Doesn't** link to any bindings so its buildable for wasm32-unknown-unknown. 

## What is this? 
Its an audio codec I cobbled together for compressing audio. 
I am by no means a compression expert so don't expect much from this.

## Why?
The need arose to compress microphone data coming from the WEBAUDIO api during the development of a WASM application I was writing. To simplify the process of compiling the project, I needed the  encoder to be written in *pure rust*. AFAIK, there are a few pure rust audio **decoders** for things like VORBIS(lewton) ,MP3(puremp3) etc but most of those crates do not support **encoding**. 

## Performance 
Probably not very fast but I haven't really tested this. Should be real-time though. And I will definitely make optimizations if I can't meet my speed requirements.

## Compression
Compression savings seems to be anywhere from 20%-70% but i haven't dont extensive testing to say concretely.  The codec is not lossy, however, it does quantize the audio on higher "compression-levels" to make significant space savings. Quantization doesn't effect audio quality too badly, I was pretty suprised at that discovery.  




## Encode Example
```rust
use adhoc_audio::{codec::Streamable, AdhocCodec, WavCodec};
use std::fs::File;

fn main() {
    println!("compressing file example..");


    //set up a buffer for reading/writing samples
    let mut samples = [0.0; 1024];

    //open wav file
    let mut wav_reader = WavCodec::load(File::open("./resources/taunt.wav")
        .unwrap()).unwrap();
    
    let mut adhoc = AdhocCodec::new()
        // level 0 means no quantization ,so its basically lossless at level 0
        // levels 1-10 means quantization so compression is better but 
        // quality suffers (dithering is added to compensate)
        .with_compression_level(7)
        // AdhocCodec::with_info(.. ) MUST BE CALLED 
        // before calling encode/decode when you are 
        // creating a new instance of AdhocCodec
        .with_info(wav_reader.info());

    //'decode' wav stream bit-by-bit
    //Note:in this case we are just reading PCM info
    while let Some(samples_read) = wav_reader.decode(&mut samples) {
        //encode wav data bit-by-bit
        //memory is allocated as needed
        adhoc.encode(&samples[0..samples_read]);
    }

    //write compressed audio back to disk
    adhoc
        .save_to(File::create("./resources/taunt.adhoc").unwrap())
        .unwrap();

    println!("taunt.adhoc written to: ./resources");
}
```

## Decode 
check 'decompress.rs' in example folder 