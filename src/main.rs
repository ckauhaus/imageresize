#[macro_use]
extern crate clap;

use atty::Stream;
use clap::Arg;
use colored::Colorize;
use image::{DynamicImage, FilterType, GenericImageView, RgbImage};
use mozjpeg::{ColorSpace, Compress};
use rayon::prelude::*;
use rexiv2::{Metadata, Rexiv2Error};
use snafu::{OptionExt, ResultExt, Snafu};
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::exit;

#[derive(Debug, Snafu)]
enum Error {
    #[snafu(display("cannot open image: {}", source))]
    Open { source: io::Error },
    #[snafu(display("cannot load image: {}", source))]
    Load { source: image::ImageError },
    #[snafu(display("cannot parse EXIF metadata: {}", source))]
    ParseMeta { source: Rexiv2Error },
    #[snafu(display("cannot create target directory: {}", source))]
    LeadingDirectories { source: io::Error },
    #[snafu(display("'{}' does not look like a file", src.display()))]
    UglyFile { src: PathBuf },
    #[snafu(display("cannot write resized image '{}': {}", dst.display(), source))]
    Write { dst: PathBuf, source: io::Error },
    #[snafu(display("cannot write EXIF metadata to '{}': {}", dst.display(), source))]
    WriteMeta { dst: PathBuf, source: Rexiv2Error },
}

type Result<T, E = Error> = std::result::Result<T, E>;

struct Converter {
    max: u32,
    quality: u8,
    dir: PathBuf,
}

fn load(src: &Path) -> Result<(DynamicImage, Metadata)> {
    let buf = fs::read(src).context(Open)?;
    let img = image::load_from_memory(&buf).context(Load)?;
    let meta = Metadata::new_from_buffer(&buf).context(ParseMeta)?;
    Ok((img, meta))
}

// Returns true if the compressed size is less than 90% of the original size
fn should_rewrite(compressed: &[u8], orig: &Path) -> bool {
    if let Some(ext) = orig.extension() {
        if ext != OsStr::new("jpg") {
            return true;
        }
    }
    match fs::metadata(orig) {
        Ok(meta) => (compressed.len() as u64) < meta.len() * 9 / 10,
        Err(_) => true,
    }
}

impl Converter {
    fn conditional_resize(&self, img: DynamicImage) -> RgbImage {
        let res = if img.width() > self.max || img.height() > self.max {
            img.resize(self.max, self.max, FilterType::CatmullRom)
        } else {
            img
        };
        match res {
            DynamicImage::ImageRgb8(img) => img,
            _ => res.to_rgb(),
        }
    }

    fn compress(&self, img: RgbImage) -> Vec<u8> {
        let (w, h) = (img.width() as usize, img.height() as usize);
        let mut comp = Compress::new(ColorSpace::JCS_RGB);
        comp.set_size(w, h);
        comp.set_quality(f32::from(self.quality));
        comp.set_optimize_scans(true);
        comp.set_progressive_mode();
        comp.set_mem_dest();
        comp.start_compress();
        for line in img.into_vec().chunks(w * 3) {
            comp.write_scanlines(line);
        }
        comp.finish_compress();
        comp.data_to_vec()
            .expect("libjpeg internal compression error")
    }

    fn convert(&self, src: &Path) -> Result<PathBuf> {
        let (img, meta) = load(src)?;
        let dir = src.parent().unwrap_or_else(|| Path::new("."));
        let base = Path::new(src.file_stem().context(UglyFile { src })?);
        let dst = dir.join(&self.dir).join(base.with_extension("jpg"));
        if let Some(dir) = dst.parent() {
            fs::create_dir_all(dir).context(LeadingDirectories)?;
        }
        let resized = self.conditional_resize(img);
        let compressed = self.compress(resized);
        if should_rewrite(&compressed, src) {
            fs::write(&dst, &compressed).context(Write { dst: &dst })?;
            meta.save_to_file(&dst).context(WriteMeta { dst: &dst })?;
        } else {
            fs::hard_link(src, &dst)
                .or_else(|_| fs::copy(src, &dst).map(|_| ()))
                .context(Write { dst: &dst })?;
        }
        Ok(dst)
    }
}

fn cli() -> clap::App<'static, 'static> {
    let val_size = |val: String| match val.parse::<u32>() {
        Ok(s) if s > 0 => Ok(()),
        Ok(_) => Err("SIZE must be a positive integer".to_owned()),
        Err(e) => Err(e.to_string()),
    };

    let val_quality = |val: String| match val.parse::<usize>() {
        Ok(q) if q <= 100 && q > 0 => Ok(()),
        Ok(_) => Err("quality value must be between 1 and 100".to_owned()),
        Err(e) => Err(e.to_string()),
    };

    app_from_crate!()
        .arg(
            Arg::from_usage("[DIR] -o --output-dir 'Writes resized images to DIR'")
                .default_value("resized")
                .long_help("\
                Writes resized JPEG images to DIR. If DIR is a relative path, it will be created \
                relative to each file's containing directory. In case DIR is an absolute path, \
                the files' directories are not taken into account.")
        )
        .arg(
            Arg::from_usage(
                "[SIZE] -m --max-size \
                'Shrinks images so that the longest dimension is no more than SIZE pixels'",
            )
            .default_value("3840")
            .validator(val_size),
        )
        .arg(
            Arg::from_usage("[QUALITY] -q --quality 'JPEG compression quality (1..100)'")
                .default_value("80")
                .validator(val_quality),
        )
        .arg(Arg::from_usage("<FILE>... 'input image files (JPEG, PNG, or WEBP)'"))
}

fn main() {
    if atty::isnt(Stream::Stderr) {
        colored::control::set_override(false)
    }
    let m = cli().get_matches();
    let conv = Converter {
        max: m.value_of("SIZE").unwrap().parse().unwrap(),
        quality: m.value_of("QUALITY").unwrap().parse().unwrap(),
        dir: PathBuf::from(m.value_of_os("DIR").unwrap()),
    };
    let files = m.values_of_os("FILE").unwrap().collect::<Vec<&OsStr>>();
    let failed = files
        .par_iter()
        .map(|f| {
            let src = Path::new(f);
            match conv.convert(&src) {
                Err(e) => {
                    let src = src.to_string_lossy();
                    eprint!("{}: {}\n", src.yellow(), e);
                    Some(src)
                }
                Ok(dst) => {
                    println!("{}", dst.display());
                    None
                }
            }
        })
        .filter_map(|e| e)
        .collect::<Vec<_>>();
    if !failed.is_empty() {
        eprintln!(
            "{} {}",
            "Error while converting the following images:".red().bold(),
            failed.join(", ")
        );
        exit(2);
    }
}
