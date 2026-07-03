// # image-convert — Universal Image Format Converter
//
// Converts any supported image format to any other, routing through the shared
// RGBA8 PixelContainer intermediate representation.
//
// ```
// image-convert photo.nef photo.png          # develop camera RAW → PNG
// image-convert banner.bmp banner.webp       # BMP → WebP
// image-convert icon.ico icon.png            # extract ICO → PNG
// image-convert --list-formats               # show all supported formats
// ```
//
// Exit codes:
//   0 — success
//   1 — input file not found / unreadable
//   2 — format detection failed
//   3 — output format not encodable (RAW output requested)
//   4 — decode error
//   5 — encode error
//   6 — output write error

use image_convert::{
    decode_image, detect_format, encode_image, extension_from_path,
    list_formats, ImageFormat,
};
use std::process;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let code = run(&args[1..]);
    process::exit(code);
}

fn run(args: &[String]) -> i32 {
    // ── Parse arguments ───────────────────────────────────────────────────

    if args.is_empty() || args.iter().any(|a| a == "-h" || a == "--help") {
        print_usage();
        return 0;
    }

    if args.iter().any(|a| a == "-V" || a == "--version") {
        println!("image-convert 0.1.0");
        return 0;
    }

    if args.iter().any(|a| a == "--list-formats") {
        print!("{}", list_formats());
        return 0;
    }

    // Collect positional arguments and flags.
    let mut positional: Vec<&str> = Vec::new();
    let mut quality: u8 = 85;
    let mut force_from: Option<ImageFormat> = None;
    let mut force_to:   Option<ImageFormat> = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-q" | "--quality" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("error: --quality requires a value");
                    return 1;
                }
                quality = match args[i].parse::<u8>() {
                    Ok(q) if q >= 1 => q,
                    _ => {
                        eprintln!("error: quality must be 1–100");
                        return 1;
                    }
                };
            }
            "--from" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("error: --from requires a format name");
                    return 1;
                }
                match image_convert::detect_by_extension(&args[i].to_lowercase()) {
                    Some(fmt) => force_from = Some(fmt),
                    None => {
                        eprintln!("error: unknown format '{}'. Run --list-formats.", args[i]);
                        return 2;
                    }
                }
            }
            "--to" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("error: --to requires a format name");
                    return 1;
                }
                match image_convert::detect_by_extension(&args[i].to_lowercase()) {
                    Some(fmt) => force_to = Some(fmt),
                    None => {
                        eprintln!("error: unknown format '{}'. Run --list-formats.", args[i]);
                        return 2;
                    }
                }
            }
            other if other.starts_with('-') => {
                eprintln!("error: unknown flag '{}'. Run --help.", other);
                return 1;
            }
            other => positional.push(other),
        }
        i += 1;
    }

    if positional.len() < 2 {
        eprintln!("error: expected <INPUT> and <OUTPUT> arguments.");
        print_usage();
        return 1;
    }

    let input_path  = positional[0];
    let output_path = positional[1];

    // ── Read input file ───────────────────────────────────────────────────
    //
    // Security: check metadata BEFORE reading to avoid two threats:
    //
    // 1. Unbounded memory allocation from special files.
    //    `std::fs::read` calls `read_to_end` which eagerly fills a Vec.
    //    A named pipe, `/dev/urandom`, or a Linux `/proc` pseudo-file returns
    //    data indefinitely — the process would OOM before the post-read size
    //    guard fires. Rejecting non-regular files prevents this.
    //
    // 2. Pre-read size guard.
    //    `metadata.len()` returns the reported file size from the OS without
    //    reading any bytes. This is cheap and rejects oversized regular files
    //    before any allocation occurs.
    //
    // Note: there is a TOCTOU window between the metadata check and the read,
    // but for a CLI tool invoked by the file owner the residual risk is
    // acceptable (the attacker would already need write access to the path).

    const MAX_INPUT_BYTES: u64 = 512 * 1024 * 1024; // 512 MB

    let meta = match std::fs::metadata(input_path) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("error: cannot stat '{}': {}", input_path, e);
            return 1;
        }
    };

    if !meta.is_file() {
        eprintln!("error: '{}' is not a regular file (pipes and device files are not supported)", input_path);
        return 1;
    }

    if meta.len() > MAX_INPUT_BYTES {
        eprintln!("error: input file is too large (> 512 MB)");
        return 1;
    }

    let input_bytes = match std::fs::read(input_path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("error: cannot read '{}': {}", input_path, e);
            return 1;
        }
    };

    // ── Detect input format ───────────────────────────────────────────────

    let input_fmt = match force_from {
        Some(fmt) => fmt,
        None => {
            let ext = extension_from_path(input_path);
            match detect_format(&input_bytes, ext.as_deref()) {
                Some(fmt) => fmt,
                None => {
                    eprintln!(
                        "error: cannot detect format of '{}'. \
                         Try --from <format>. Run --list-formats for supported formats.",
                        input_path
                    );
                    return 2;
                }
            }
        }
    };

    // ── Detect output format ──────────────────────────────────────────────

    let output_fmt = match force_to {
        Some(fmt) => fmt,
        None => {
            let ext = extension_from_path(output_path);
            match ext.as_deref().and_then(image_convert::detect_by_extension) {
                Some(fmt) => fmt,
                None => {
                    eprintln!(
                        "error: cannot determine output format from '{}'. \
                         Try --to <format>. Run --list-formats for supported formats.",
                        output_path
                    );
                    return 2;
                }
            }
        }
    };

    // Reject RAW output early with a helpful message.
    if !output_fmt.is_encodable() {
        eprintln!(
            "error: {} is a camera RAW format and cannot be used as output.\n\
             Supported output formats: png, jpg, bmp, tiff, webp, jxl, gif, ico, qoi, ppm",
            output_fmt.name()
        );
        return 3;
    }

    // ── Decode ────────────────────────────────────────────────────────────

    eprintln!(
        "Converting {} → {} ...",
        input_fmt.name(),
        output_fmt.name()
    );

    let pixels = match decode_image(&input_bytes, &input_fmt) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("error: decode failed: {}", e);
            return 4;
        }
    };

    eprintln!("  Decoded: {}×{} pixels", pixels.width, pixels.height);

    // ── Encode ────────────────────────────────────────────────────────────

    let output_bytes = match encode_image(&pixels, &output_fmt, quality) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("error: encode failed: {}", e);
            return 5;
        }
    };

    eprintln!(
        "  Encoded: {} bytes ({} KB)",
        output_bytes.len(),
        output_bytes.len() / 1024
    );

    // ── Write output (atomic: write to .tmp then rename) ─────────────────
    //
    // Security: use a nonce-suffixed temp file to prevent symlink attacks.
    //
    // A predictable name like `output.png.tmp` in a shared directory allows
    // an attacker to pre-create a symlink at that path pointing to an
    // arbitrary file, causing `std::fs::write` to overwrite the symlink
    // target. Adding a process-specific nonce makes the name unpredictable.
    //
    // We derive the nonce from the current thread's stack address (which
    // changes per process and per execution) combined with the process ID.
    // This is not cryptographic randomness, but it is sufficient to prevent
    // a casual pre-positioning attack in a shared directory.

    let nonce = {
        // Mix the PID with a stack address to get a runtime-varying value.
        let pid = std::process::id();
        let stack_var: usize = &pid as *const u32 as usize;
        // Fold into 32 bits for a short but non-guessable suffix.
        (pid as u64 ^ (stack_var as u64)).wrapping_mul(6364136223846793005)
    };
    let tmp_path = format!("{}.{:016x}.tmp", output_path, nonce);

    if let Err(e) = std::fs::write(&tmp_path, &output_bytes) {
        eprintln!("error: cannot write '{}': {}", tmp_path, e);
        return 6;
    }
    if let Err(e) = std::fs::rename(&tmp_path, output_path) {
        eprintln!("error: cannot rename '{}' to '{}': {}", tmp_path, output_path, e);
        let _ = std::fs::remove_file(&tmp_path);
        return 6;
    }

    eprintln!("  Written: {}", output_path);
    0
}

fn print_usage() {
    println!(
        r#"image-convert 0.1.0
Universal image format converter — the pandoc of image files.

USAGE:
    image-convert [OPTIONS] <INPUT> <OUTPUT>

ARGUMENTS:
    <INPUT>    Path to the input image file
    <OUTPUT>   Path to the output image file (extension determines format)

OPTIONS:
    -q, --quality <N>     Encode quality 1–100 for lossy formats (default: 85)
    --from <FORMAT>       Force input format (e.g. jpg, nef, dng)
    --to <FORMAT>         Force output format (e.g. png, tiff)
    --list-formats        Print all supported formats and exit
    -h, --help            Print this help
    -V, --version         Print version

EXAMPLES:
    image-convert photo.nef photo.png           # develop camera RAW → PNG
    image-convert photo.cr2 photo.tiff          # Canon RAW → TIFF
    image-convert photo.raf photo.jpg -q 90     # Fujifilm RAW → JPEG (90%)
    image-convert banner.gif banner.webp        # GIF → WebP
    image-convert icon.ico icon.png             # ICO → PNG
    image-convert logo.bmp logo.qoi             # BMP → QOI (fast lossless)
    image-convert --list-formats                # show all formats

SUPPORTED INPUTS: PNG BMP PPM QOI JPEG WebP JXL GIF ICO TIFF
                  DNG CR2 NEF ARW RAF ORF RW2 (camera RAW)
SUPPORTED OUTPUTS: PNG BMP PPM QOI JPEG WebP JXL GIF ICO TIFF
"#
    );
}
