//! `instagram-filters` CLI binary.
//!
//! Thin wrapper around the [`instagram_filters`] library:
//! - Parse argv into `--input`, `--output`, `--filter`, and filter args
//! - Read PPM file → PixelContainer
//! - Call `apply_filter` (which dispatches through the matrix execution layer)
//! - Encode PixelContainer → PPM file
//!
//! ## Usage
//!
//! ```text
//! instagram-filters --input photo.ppm --output sepia.ppm --filter sepia
//! instagram-filters --input photo.ppm --output bright.ppm --filter brightness --amount 30
//! instagram-filters --input photo.ppm --output gamma.ppm --filter gamma --gamma 0.7
//! instagram-filters --input photo.ppm --output high.ppm --filter contrast --scale 1.5
//! instagram-filters --input photo.ppm --output post.ppm --filter posterize --levels 4
//! instagram-filters --input photo.ppm --output grey.ppm --filter greyscale
//! instagram-filters --input photo.ppm --output inv.ppm --filter invert
//! ```
//!
//! ## Path safety
//!
//! Paths come from CLI args.  We don't follow symlinks across boundaries
//! and don't write outside what `std::fs::File::create` allows for the
//! invoking user.  The program treats `--input` and `--output` as
//! literal paths — same trust model as `cp`.

use instagram_filters::{apply_filter, Filter, FilterParamError};
use std::collections::HashMap;
use std::process::ExitCode;

fn main() -> ExitCode {
    let argv: Vec<String> = std::env::args().collect();
    if argv.iter().any(|a| a == "-h" || a == "--help") {
        print_help();
        return ExitCode::SUCCESS;
    }

    let parsed = match parse_args(&argv[1..]) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("instagram-filters: {}", e);
            eprintln!("(run with --help for usage)");
            return ExitCode::from(2);
        }
    };

    // Cap input file size at 64 MiB to prevent OOM from massive inputs.
    // PPM files at 64 MiB are huge — that's roughly a 4000×4000 RGB image,
    // which exceeds the matrix execution layer's per-tensor cap anyway.
    const MAX_INPUT_BYTES: u64 = 64 * 1024 * 1024;

    let bytes = match std::fs::metadata(&parsed.input) {
        Ok(m) if m.len() > MAX_INPUT_BYTES => {
            eprintln!(
                "instagram-filters: input file is {} bytes, exceeds the {}-byte cap",
                m.len(),
                MAX_INPUT_BYTES
            );
            return ExitCode::from(3);
        }
        Ok(_) => match std::fs::read(&parsed.input) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("instagram-filters: read {}: {}", parsed.input, e);
                return ExitCode::from(4);
            }
        },
        Err(e) => {
            eprintln!("instagram-filters: stat {}: {}", parsed.input, e);
            return ExitCode::from(4);
        }
    };

    let image = match image_codec_ppm::decode_ppm(&bytes) {
        Ok(img) => img,
        Err(e) => {
            eprintln!("instagram-filters: decode {}: {}", parsed.input, e);
            return ExitCode::from(5);
        }
    };

    eprintln!(
        "instagram-filters: applying {} to {}×{} image…",
        parsed.filter.name(),
        image.width,
        image.height
    );

    let out_image = match apply_filter(parsed.filter, &image) {
        Ok(out) => out,
        Err(e) => {
            eprintln!("instagram-filters: filter failed: {}", e);
            return ExitCode::from(6);
        }
    };

    let encoded = image_codec_ppm::encode_ppm(&out_image);
    if let Err(e) = std::fs::write(&parsed.output, &encoded) {
        eprintln!("instagram-filters: write {}: {}", parsed.output, e);
        return ExitCode::from(7);
    }

    eprintln!(
        "instagram-filters: wrote {} bytes to {}",
        encoded.len(),
        parsed.output
    );
    ExitCode::SUCCESS
}

#[derive(Debug)]
struct ParsedArgs {
    input: String,
    output: String,
    filter: Filter,
}

#[derive(Debug)]
enum ArgError {
    Missing(&'static str),
    DuplicateFlag(String),
    Filter(FilterParamError),
    Bare(String),
}

impl core::fmt::Display for ArgError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ArgError::Missing(name) => write!(f, "missing required --{}", name),
            ArgError::DuplicateFlag(s) => write!(f, "flag {} given more than once", s),
            ArgError::Filter(fpe) => write!(f, "{}", fpe),
            ArgError::Bare(s) => write!(f, "unexpected argument {}", s),
        }
    }
}

fn parse_args(args: &[String]) -> Result<ParsedArgs, ArgError> {
    let mut input: Option<String> = None;
    let mut output: Option<String> = None;
    let mut filter_name: Option<String> = None;
    let mut filter_args: HashMap<String, String> = HashMap::new();

    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        let consume = |key: &str, slot: &mut Option<String>| -> Result<usize, ArgError> {
            if slot.is_some() {
                return Err(ArgError::DuplicateFlag(format!("--{}", key)));
            }
            let v = args
                .get(i + 1)
                .ok_or(ArgError::Missing(match key {
                    "input" => "input VALUE",
                    "output" => "output VALUE",
                    "filter" => "filter VALUE",
                    _ => "VALUE",
                }))?
                .clone();
            *slot = Some(v);
            Ok(2)
        };

        let step = match a.as_str() {
            "--input" => consume("input", &mut input)?,
            "--output" => consume("output", &mut output)?,
            "--filter" => consume("filter", &mut filter_name)?,
            // Filter-specific args go into the args map.
            "--amount" | "--gamma" | "--scale" | "--levels" => {
                let key = a.trim_start_matches("--").to_string();
                if filter_args.contains_key(&key) {
                    return Err(ArgError::DuplicateFlag(a.clone()));
                }
                let v = args
                    .get(i + 1)
                    .ok_or(ArgError::Missing("filter argument value"))?
                    .clone();
                filter_args.insert(key, v);
                2
            }
            other => return Err(ArgError::Bare(other.to_string())),
        };
        i += step;
    }

    let input = input.ok_or(ArgError::Missing("input"))?;
    let output = output.ok_or(ArgError::Missing("output"))?;
    let filter_name = filter_name.ok_or(ArgError::Missing("filter"))?;
    let filter = Filter::parse_with_args(&filter_name, &filter_args).map_err(ArgError::Filter)?;

    Ok(ParsedArgs {
        input,
        output,
        filter,
    })
}

fn print_help() {
    println!(
        "instagram-filters — apply Instagram-style filters via the matrix execution layer\n\
         \n\
         USAGE:\n\
         \x20\x20instagram-filters --input PATH --output PATH --filter NAME [filter args]\n\
         \n\
         FILTERS:\n\
         \x20\x20invert                              Invert RGB channels (alpha unchanged)\n\
         \x20\x20greyscale | grayscale               Rec.709 luminance, linear light\n\
         \x20\x20sepia                               Classic 3×3 sepia matrix\n\
         \x20\x20brightness   --amount N             Add N ∈ [-255, 255] to each channel\n\
         \x20\x20gamma        --gamma G              Power-law gamma in linear light\n\
         \x20\x20contrast     --scale S              Stretch around mid-grey 128\n\
         \x20\x20posterize    --levels L             Reduce to L distinct values per channel\n\
         \n\
         FILE FORMAT:\n\
         \x20\x20Input and output are PPM (P6) files — see image-codec-ppm.\n\
         \n\
         The pipeline:\n\
         \x20\x20PPM bytes → PixelContainer → image-gpu-core (MatrixIR builder)\n\
         \x20\x20         → matrix-runtime planner → matrix-cpu → PixelContainer → PPM bytes\n"
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use instagram_filters::Filter;

    fn s(strs: &[&str]) -> Vec<String> {
        strs.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn parse_simple_invert() {
        let argv = s(&["--input", "in.ppm", "--output", "out.ppm", "--filter", "invert"]);
        let p = parse_args(&argv).unwrap();
        assert_eq!(p.input, "in.ppm");
        assert_eq!(p.output, "out.ppm");
        assert_eq!(p.filter, Filter::Invert);
    }

    #[test]
    fn parse_brightness_with_amount() {
        let argv = s(&[
            "--input", "i.ppm", "--output", "o.ppm", "--filter", "brightness", "--amount", "42",
        ]);
        let p = parse_args(&argv).unwrap();
        assert_eq!(p.filter, Filter::Brightness { delta: 42 });
    }

    #[test]
    fn parse_gamma_with_value() {
        let argv = s(&[
            "--input", "i.ppm", "--output", "o.ppm", "--filter", "gamma", "--gamma", "0.5",
        ]);
        let p = parse_args(&argv).unwrap();
        assert_eq!(p.filter, Filter::Gamma { gamma: 0.5 });
    }

    #[test]
    fn missing_input_errors() {
        let argv = s(&["--output", "o.ppm", "--filter", "invert"]);
        let err = parse_args(&argv).unwrap_err();
        assert!(matches!(err, ArgError::Missing(_)));
    }

    #[test]
    fn missing_filter_errors() {
        let argv = s(&["--input", "i.ppm", "--output", "o.ppm"]);
        assert!(matches!(parse_args(&argv).unwrap_err(), ArgError::Missing(_)));
    }

    #[test]
    fn unknown_flag_errors() {
        let argv = s(&[
            "--input", "i.ppm", "--output", "o.ppm", "--filter", "invert", "--bogus", "x",
        ]);
        assert!(matches!(parse_args(&argv).unwrap_err(), ArgError::Bare(_)));
    }

    #[test]
    fn duplicate_input_errors() {
        let argv = s(&[
            "--input", "a.ppm", "--input", "b.ppm", "--output", "o.ppm", "--filter", "invert",
        ]);
        assert!(matches!(
            parse_args(&argv).unwrap_err(),
            ArgError::DuplicateFlag(_)
        ));
    }

    #[test]
    fn brightness_missing_amount_errors() {
        let argv = s(&[
            "--input", "i.ppm", "--output", "o.ppm", "--filter", "brightness",
        ]);
        let err = parse_args(&argv).unwrap_err();
        assert!(matches!(err, ArgError::Filter(_)));
    }

    #[test]
    fn posterize_with_levels() {
        let argv = s(&[
            "--input", "i.ppm", "--output", "o.ppm", "--filter", "posterize", "--levels", "8",
        ]);
        let p = parse_args(&argv).unwrap();
        assert_eq!(p.filter, Filter::Posterize { levels: 8 });
    }
}
