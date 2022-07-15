# Adhoc Audio
Audio compression written in pure rust. 
**It Doesn't** link to any bindings so its buildable for wasm32-unknown-unknown. 

## What is this? 
Its a collection of audio codecs I'm cobbling together to compress audio. 
I'm currently using it to: 
- compress microphone data on client's browser (using this repo compiled to wasm)
- send data to server (POST request multipart)
- re-encode compressed data to a standard format (here i'll use rust bindings to libavcodec or something)

Currently, there's only one codec that actually compresses audio [`AdhocCodec`] and a WAVE Reader/Writer Utility I wrote [`WavCodec`]. 

## Why?
The need arose to compress microphone data coming from the WEBAUDIO api during the development of a WASM application I was writing. A **pure rust** solution is needed to keep the build steps of the project simple. AFAIK, there are a few pure rust audio **decoders** for things like VORBIS(lewton) ,MP3(puremp3) etc but most of those crates do not support **encoding**. 

## Performance 
Probably not very fast but I haven't really tested this. The encoding/decoding algorithm is O(N) so it should be fast enough. And I will definitely make optimizations if I can't meet my speed requirements. the `Vec` implementation does allocation, so there should be log(N) allocations. 

## Compression
Compression savings seems to be anywhere from 20%-70% but I haven't done extensive testing to say concretely.  The codec is not lossy, however, it does quantize the audio on higher "compression-levels" to make significant space savings. Quantization doesn't effect audio quality too badly, I was pretty suprised at that discovery.  


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

## Decode Example
```rust
use adhoc_audio::{codec::Streamable, AdhocCodec, WavCodec};
use std::fs::File;

fn main() {
    println!("decompressing file from 'compress' example...");

    //set up a buffer for reading/writing samples
    let mut samples = [0.0; 1024];

    //open wav file
    let mut adhoc = AdhocCodec::load(
        File::open("./resources/taunt.adhoc").expect("run example 'compress' before this one"),
    )
    .unwrap();
    
    let mut wav_writer = WavCodec::new(adhoc.info());

    //decode adhoc stream a chunk of samples at a time
    while let Some(samples_read) = adhoc.decode(&mut samples) {
        //encode wav data bit-by-bit
        //memory is allocated as needed
        wav_writer.encode(&samples[0..samples_read]);
    }

    //write compressed audio back to disk
    wav_writer
        .save_to(File::create("./resources/taunt_decompressed.wav").unwrap())
        .unwrap();

    println!("taunt.adhoc written to: ./resources");
}
```


# Command line interface 
A this package has a simple command line tool to convert back and forth between `.wav` and the `.adhoc` format.

In terminal simply do: 

```
cargo install --path .
```




then to flags options do:
```
adhoc_audio -h
```

## Compress
the simplest way to compress a wav is like so:
```
adhoc_audio ./resources/taunt.wav 
```

and it will create a `taunt.codec` in the current directory 

## Decompress 
building of the first example to decompress `taunt.codec` just:
```
adhoc_audio ./taunt.codec
```

and decompressed wav file will be written to your cwd
