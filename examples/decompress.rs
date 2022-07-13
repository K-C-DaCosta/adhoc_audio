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
