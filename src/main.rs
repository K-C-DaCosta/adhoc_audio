#[cfg(feature = "cli")]
use adhoc_audio::{AdhocCodec, Streamable, WavCodec};

#[cfg(feature = "cli")]
use clap::{App, Arg};

#[cfg(feature = "cli")]
use rayon::prelude::*;

#[cfg(feature = "cli")]
use std::{
    fs::File,
    path::{Path, PathBuf},
};

pub fn main() {
    #[cfg(feature = "cli")]
    {
        cli()
    }
}

#[cfg(feature = "cli")]
pub fn cli() {
    let m = App::new("Adhoc Audio Cli")
        .author("khadeem dacosta,khadeem.dacosta@gmail.com")
        .version("0.1.0")
        .about("Compress and decompress audio")
        
        .arg(
            Arg::with_name("INPUT")
                .short("i")
                .help("list of files you want encoded/decoded")
                .min_values(1)
                // .required(true)
                .index(1),

        )
        .arg(
            Arg::with_name("output")
                .long("outdir")
                .short("o")
                .multiple(false)
                .default_value("./")
                .help("specifies output directory")
                // .index(2),
        )
        .arg(
            Arg::with_name("compression_level")
                .short("c")
                .long("comp-level")
                .multiple(false)
                .default_value("5")
                .help("specify compression level [0-10]")
                // .index(3),
        )
        .after_help(
            "This is a simple utility tool for compressing WAVE files into a custom adhoc format. \
            the adhoc format is quick enough to do decoding real-time, while also being much smaller than a \
            raw WAVE file. This library portion of this crate is written in pure rust. If you wish to use the libray-only \
            don't forget to remove the freature 'cli'
            ",
        )
        .get_matches();

    let output_directory = m.value_of("output").unwrap_or("./");

    if Path::new(output_directory).is_dir() == false {
        eprintln!("directory specified is not valid!");
        return;
    }

    let input_files = m
        .values_of("INPUT")
        .map(|values| values.into_iter().collect::<Vec<_>>())
        .unwrap_or_default();

    let compression_level = m
        .value_of("compression_level")
        .and_then(|val| val.parse::<u32>().ok())
        .unwrap_or(5);


    // println!("input files {:?}",input_files);
    // println!("output_directory: {:?}",output_directory);

    input_files.par_iter().for_each(|file_path| {
        let input: &Path = file_path.as_ref();
        let output_dir: &Path = output_directory.as_ref();
        convert_file(input, output_dir, compression_level);
    });
}

pub fn convert_file(input: &Path, output_dir: &Path, compression_level: u32) -> Option<()> {
    let input_ext = input.extension()?.to_str()?;
    if input_ext.contains("wav") {
        convert_wav_to_adhoc(input, output_dir, compression_level)?;
    }
    if input_ext.contains("adhoc") {
        convert_adhoc_to_wav(input, output_dir)?;
    }
    Some(())
}

fn convert_wav_to_adhoc(input: &Path, output_dir: &Path, compression_level: u32) -> Option<()> {
    let file = File::open(input).ok()?;
    let file_name = input.file_stem()?;

    let mut parsed_wav = WavCodec::load(file).ok()?;

    let mut compressed_wav = AdhocCodec::new()
        .with_compression_level(compression_level)
        .with_info(parsed_wav.info());

    let mut buffer = [0.0; 1024];
    while let Some(samples_read) = parsed_wav.decode(&mut buffer[..]) {
        compressed_wav.encode(&buffer[0..samples_read]);
    }

    let mut file_dest = PathBuf::from(output_dir);
    // println!("dest = {:?}",file_dest);

    file_dest.push(file_name);

    // println!("dest = {:?}",file_dest);
    file_dest.set_extension("adhoc");
    
    // println!("final dest = {:?}",file_dest);

    compressed_wav.save_to(File::create(file_dest).ok()?);

    Some(())
}
fn convert_adhoc_to_wav(input: &Path, output_dir: &Path) -> Option<()> {
    let file = File::open(input).ok()?;
    let file_name = input.file_stem()?;

    let mut src_adhoc = AdhocCodec::load(file)?;
    let mut dst_wav = WavCodec::new(src_adhoc.info());

    let mut buffer = [0.0; 1024];

    while let Some(samples_read) = src_adhoc.decode(&mut buffer[..]) {
        dst_wav.encode(&buffer[0..samples_read]);
    }

    let mut file_dest = PathBuf::from(output_dir);
    file_dest.push(file_name);
    file_dest.set_extension("wav");

    dst_wav.save_to(File::create(file_dest).ok()?).ok()?;

    Some(())
}
