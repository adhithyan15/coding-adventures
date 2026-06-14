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

use instagram_filters::{apply_filter, run_bench, BenchOpts, Filter, FilterParamError};
use std::collections::HashMap;
use std::process::ExitCode;

fn main() -> ExitCode {
    let argv: Vec<String> = std::env::args().collect();
    if argv.iter().any(|a| a == "-h" || a == "--help") {
        print_help();
        return ExitCode::SUCCESS;
    }

    // Subcommand dispatch: if the first positional is a known
    // subcommand name, route to its handler.  Otherwise fall through
    // to the original `--input/--output/--filter` flag style so the
    // existing CLI surface is unchanged.
    if argv.len() >= 2 && argv[1] == "bench-specialisation" {
        return run_bench_subcommand(&argv[2..]);
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

    // Surface which backend the matrix execution layer routed to.  The
    // planner picks per graph based on cost model, so tiny images may
    // stay on CPU even on a Mac with Metal available.
    if let Some(name) = image_gpu_core::last_executor() {
        eprintln!("instagram-filters: executed on {}", name);
    }

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

/// Subcommand handler for `bench-specialisation`.
///
/// Argv layout (after the subcommand token has been stripped):
///   <input.ppm> <output_dir> [--iterations N] [--batch N]
///                            [--brightness N] [--contrast S]
///
/// We accept the two required positionals first to mirror typical Unix
/// tools (`cp src dst`), then optional flags for tuning.  All flags
/// have defaults from [`BenchOpts::with_paths`].
fn run_bench_subcommand(args: &[String]) -> ExitCode {
    let opts = match parse_bench_args(args) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("instagram-filters bench-specialisation: {}", e);
            eprintln!("(run with --help for usage)");
            return ExitCode::from(2);
        }
    };

    eprintln!(
        "instagram-filters: running bench: {} iterations, batch {} on {}",
        opts.iterations, opts.batch, opts.input
    );

    match run_bench(&opts) {
        Ok(summary) => {
            eprintln!(
                "instagram-filters: bench done — {} snapshots, image {}×{}, wrote {} bytes",
                summary.snapshots.len(),
                summary.image_width,
                summary.image_height,
                summary.final_result_bytes
            );
            if let Some(last) = summary.snapshots.last() {
                eprintln!(
                    "instagram-filters: final counters — cache={} installs={} dispatches={} deopts={}",
                    last.spec_cache_len,
                    last.specialised_install_count,
                    last.specialised_dispatch_count,
                    last.deoptimisation_count
                );
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("instagram-filters bench-specialisation: {}", e);
            ExitCode::from(6)
        }
    }
}

#[derive(Debug)]
enum BenchArgError {
    MissingInput,
    MissingOutputDir,
    MissingValue(&'static str),
    UnknownFlag(String),
    InvalidNumber { flag: &'static str, value: String },
}

impl core::fmt::Display for BenchArgError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            BenchArgError::MissingInput => write!(f, "missing <input.ppm> positional"),
            BenchArgError::MissingOutputDir => write!(f, "missing <output_dir> positional"),
            BenchArgError::MissingValue(flag) => write!(f, "flag --{} needs a value", flag),
            BenchArgError::UnknownFlag(s) => write!(f, "unknown argument '{}'", s),
            BenchArgError::InvalidNumber { flag, value } => {
                write!(f, "--{} expects a number, got '{}'", flag, value)
            }
        }
    }
}

fn parse_bench_args(args: &[String]) -> Result<BenchOpts, BenchArgError> {
    // Collect positionals (anything not starting with `--`) and flags.
    let mut positional: Vec<String> = Vec::new();
    let mut iterations: Option<usize> = None;
    let mut batch: Option<usize> = None;
    let mut brightness: Option<i16> = None;
    let mut contrast: Option<f32> = None;

    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        if let Some(stripped) = a.strip_prefix("--") {
            let v = args
                .get(i + 1)
                .ok_or(BenchArgError::MissingValue(match stripped {
                    "iterations" => "iterations",
                    "batch" => "batch",
                    "brightness" => "brightness",
                    "contrast" => "contrast",
                    _ => "value",
                }))?
                .clone();
            match stripped {
                "iterations" => {
                    iterations = Some(v.parse().map_err(|_| BenchArgError::InvalidNumber {
                        flag: "iterations",
                        value: v.clone(),
                    })?);
                }
                "batch" => {
                    batch = Some(v.parse().map_err(|_| BenchArgError::InvalidNumber {
                        flag: "batch",
                        value: v.clone(),
                    })?);
                }
                "brightness" => {
                    brightness = Some(v.parse().map_err(|_| BenchArgError::InvalidNumber {
                        flag: "brightness",
                        value: v.clone(),
                    })?);
                }
                "contrast" => {
                    contrast = Some(v.parse().map_err(|_| BenchArgError::InvalidNumber {
                        flag: "contrast",
                        value: v.clone(),
                    })?);
                }
                other => return Err(BenchArgError::UnknownFlag(format!("--{}", other))),
            }
            i += 2;
        } else {
            positional.push(a.clone());
            i += 1;
        }
    }

    let mut pos = positional.into_iter();
    let input = pos.next().ok_or(BenchArgError::MissingInput)?;
    let output_dir = pos.next().ok_or(BenchArgError::MissingOutputDir)?;
    if let Some(extra) = pos.next() {
        return Err(BenchArgError::UnknownFlag(extra));
    }

    let mut opts = BenchOpts::with_paths(input, output_dir);
    if let Some(n) = iterations {
        opts.iterations = n;
    }
    if let Some(n) = batch {
        opts.batch = n;
    }
    if let Some(d) = brightness {
        opts.brightness_delta = d;
    }
    if let Some(s) = contrast {
        opts.contrast_scale = s;
    }
    Ok(opts)
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
         \x20\x20         → matrix-runtime planner → matrix-cpu → PixelContainer → PPM bytes\n\
         \n\
         SUBCOMMANDS:\n\
         \x20\x20bench-specialisation <input.ppm> <output_dir>\n\
         \x20\x20    [--iterations N] [--batch N] [--brightness N] [--contrast S]\n\
         \n\
         \x20\x20    Runs a brightness → contrast → sepia chain repeatedly,\n\
         \x20\x20    snapshotting the MX05 specialisation pipeline counters\n\
         \x20\x20    (spec_cache_len, specialised_install_count,\n\
         \x20\x20    specialised_dispatch_count, deoptimisation_count) every\n\
         \x20\x20    --batch iterations.  Writes <output_dir>/result.ppm and\n\
         \x20\x20    <output_dir>/summary.json.  Defaults: 3000 iterations,\n\
         \x20\x20    batch of 1000.\n"
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

    #[test]
    fn bench_args_two_positionals_only() {
        let opts = parse_bench_args(&s(&["in.ppm", "out_dir"])).unwrap();
        assert_eq!(opts.input, "in.ppm");
        assert_eq!(opts.output_dir, "out_dir");
        assert_eq!(opts.iterations, 3000);
        assert_eq!(opts.batch, 1000);
    }

    #[test]
    fn bench_args_with_overrides() {
        let opts = parse_bench_args(&s(&[
            "in.ppm",
            "out",
            "--iterations",
            "10",
            "--batch",
            "5",
            "--brightness",
            "12",
            "--contrast",
            "1.25",
        ]))
        .unwrap();
        assert_eq!(opts.iterations, 10);
        assert_eq!(opts.batch, 5);
        assert_eq!(opts.brightness_delta, 12);
        assert!((opts.contrast_scale - 1.25).abs() < 1e-6);
    }

    #[test]
    fn bench_args_flags_before_positionals_ok() {
        // Order independence: flags can come first.
        let opts =
            parse_bench_args(&s(&["--iterations", "5", "in.ppm", "--batch", "2", "out"])).unwrap();
        assert_eq!(opts.input, "in.ppm");
        assert_eq!(opts.output_dir, "out");
        assert_eq!(opts.iterations, 5);
        assert_eq!(opts.batch, 2);
    }

    #[test]
    fn bench_args_missing_positional() {
        assert!(matches!(
            parse_bench_args(&s(&["only_input"])).unwrap_err(),
            BenchArgError::MissingOutputDir
        ));
        assert!(matches!(
            parse_bench_args(&s(&[])).unwrap_err(),
            BenchArgError::MissingInput
        ));
    }

    #[test]
    fn bench_args_unknown_flag_errors() {
        let err = parse_bench_args(&s(&["in", "out", "--bogus", "1"])).unwrap_err();
        assert!(matches!(err, BenchArgError::UnknownFlag(_)));
    }

    #[test]
    fn bench_args_invalid_number_errors() {
        let err = parse_bench_args(&s(&["in", "out", "--iterations", "lots"])).unwrap_err();
        assert!(matches!(err, BenchArgError::InvalidNumber { .. }));
    }

    #[test]
    fn bench_args_extra_positional_errors() {
        let err = parse_bench_args(&s(&["in", "out", "extra"])).unwrap_err();
        assert!(matches!(err, BenchArgError::UnknownFlag(_)));
    }
}
