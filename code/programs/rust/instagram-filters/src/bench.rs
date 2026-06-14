//! # `bench-specialisation` — end-to-end MX05 specialisation demo
//!
//! Drives a chain of filters (brightness → contrast → sepia) repeatedly
//! over a real PPM image and snapshots the MX05 specialisation pipeline
//! counters every `batch` iterations.  The point is to show that the
//! sampler → policy → router → cache → emitter → install → dispatch →
//! deopt loop is doing useful work on a realistic image-processing
//! workload, not just synthetic unit-test graphs.
//!
//! ## What the bench does, in order
//!
//! 1. Decode the input PPM into a [`PixelContainer`].
//! 2. Run `iterations` passes of brightness → contrast → sepia.  Each
//!    pass is three dispatches through `image-gpu-core`, so the
//!    [`DefaultPolicy`] (default `min_invocations = 1000`) fires for
//!    each op once enough samples accumulate.
//! 3. Every `batch` iterations, snapshot
//!    [`image_gpu_core::spec_cache_len`],
//!    [`image_gpu_core::specialised_install_count`],
//!    [`image_gpu_core::specialised_dispatch_count`], and
//!    [`image_gpu_core::deoptimisation_count`].
//! 4. After all iterations, write the final filtered image to
//!    `<output_dir>/result.ppm` and a JSON summary of every snapshot
//!    to `<output_dir>/summary.json`.
//!
//! ## Why this is not just `apply_filter` in a loop
//!
//! The bench cycles the image back into the input of the next
//! iteration, so the same MatrixIR subgraph is dispatched over and
//! over — which is exactly the access pattern that justifies kernel
//! specialisation.  A user looking at the summary should see
//! `specialised_install_count` jump from 0 to 3 (one per op) once the
//! threshold is reached, and `specialised_dispatch_count` grow with
//! every subsequent iteration.

use image_gpu_core::{
    deoptimisation_count, gpu_brightness, gpu_contrast, gpu_sepia, spec_cache_len,
    specialised_dispatch_count, specialised_install_count, GpuError,
};

/// Inputs to a benchmark run.  Construct from the CLI argv in `main.rs`
/// or directly from test code.
#[derive(Debug, Clone)]
pub struct BenchOpts {
    /// Path to the input PPM file.
    pub input: String,
    /// Directory the bench will create / write `result.ppm` and
    /// `summary.json` into.
    pub output_dir: String,
    /// Total number of (brightness, contrast, sepia) cycles to run.
    pub iterations: usize,
    /// Snapshot the counters every `batch` iterations.  Must be > 0.
    pub batch: usize,
    /// Brightness delta passed to each iteration.  Held constant so
    /// the specialiser can fold it into a `RangeClass::Constant`.
    pub brightness_delta: i16,
    /// Contrast scale.  Held constant for the same reason.
    pub contrast_scale: f32,
}

impl BenchOpts {
    /// Defaults exposed to the CLI when the user omits the optional
    /// flags.  3000 iterations × 3 ops = 9000 dispatches, comfortably
    /// past the default `min_invocations = 1000` threshold.
    pub fn with_paths(input: String, output_dir: String) -> Self {
        Self {
            input,
            output_dir,
            iterations: 3000,
            batch: 1000,
            brightness_delta: 10,
            contrast_scale: 1.1,
        }
    }
}

/// One snapshot of the specialisation pipeline counters at the end
/// of a batch of iterations.  Serialised into the JSON summary.
#[derive(Debug, Clone, PartialEq)]
pub struct Snapshot {
    pub iteration: usize,
    pub spec_cache_len: usize,
    pub specialised_install_count: usize,
    pub specialised_dispatch_count: usize,
    pub deoptimisation_count: usize,
}

/// What [`run_bench`] returns after a successful run.
#[derive(Debug, Clone)]
pub struct BenchSummary {
    pub input: String,
    pub output_dir: String,
    pub image_width: u32,
    pub image_height: u32,
    pub iterations: usize,
    pub batch: usize,
    pub brightness_delta: i16,
    pub contrast_scale: f32,
    pub snapshots: Vec<Snapshot>,
    pub final_result_bytes: usize,
}

/// Errors that the bench may produce.  Distinct from
/// [`crate::FilterParamError`] so the CLI can return distinct exit
/// codes for parse errors vs. runtime failures.
#[derive(Debug)]
pub enum BenchError {
    /// `iterations == 0` or `batch == 0`.
    InvalidConfig(&'static str),
    /// I/O reading the input PPM or writing the output files.
    Io { path: String, err: std::io::Error },
    /// Input file exceeded the 64 MiB cap.
    InputTooLarge { path: String, bytes: u64 },
    /// PPM decoder returned an error.
    Decode(String),
    /// `image-gpu-core` returned a `GpuError` during filtering.
    Gpu(GpuError),
}

impl core::fmt::Display for BenchError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            BenchError::InvalidConfig(s) => write!(f, "invalid bench config: {}", s),
            BenchError::Io { path, err } => write!(f, "i/o on {}: {}", path, err),
            BenchError::InputTooLarge { path, bytes } => {
                write!(f, "input {} is {} bytes (cap 64 MiB)", path, bytes)
            }
            BenchError::Decode(s) => write!(f, "decode: {}", s),
            BenchError::Gpu(e) => write!(f, "gpu: {}", e),
        }
    }
}

impl std::error::Error for BenchError {}

/// Cap input files at 64 MiB — same trust model as the main CLI's
/// `apply_filter` path.  Matches the per-tensor cap in matrix-runtime.
const MAX_INPUT_BYTES: u64 = 64 * 1024 * 1024;

/// Run the bench end to end.  Returns the summary so the caller can
/// either render it as JSON or assert on it in tests.
///
/// The function intentionally takes the input path rather than already-
/// decoded bytes so the bench owns the full read-decode-loop-encode
/// path the user will see at the CLI.
pub fn run_bench(opts: &BenchOpts) -> Result<BenchSummary, BenchError> {
    if opts.iterations == 0 {
        return Err(BenchError::InvalidConfig("iterations must be > 0"));
    }
    if opts.batch == 0 {
        return Err(BenchError::InvalidConfig("batch must be > 0"));
    }

    let meta = std::fs::metadata(&opts.input).map_err(|err| BenchError::Io {
        path: opts.input.clone(),
        err,
    })?;
    if meta.len() > MAX_INPUT_BYTES {
        return Err(BenchError::InputTooLarge {
            path: opts.input.clone(),
            bytes: meta.len(),
        });
    }

    let bytes = std::fs::read(&opts.input).map_err(|err| BenchError::Io {
        path: opts.input.clone(),
        err,
    })?;
    let mut image =
        image_codec_ppm::decode_ppm(&bytes).map_err(|e| BenchError::Decode(e.to_string()))?;

    // Hold image dimensions before the iteration loop consumes the
    // value — `image` rebinds each pass.
    let width = image.width;
    let height = image.height;

    let mut snapshots: Vec<Snapshot> = Vec::new();

    for iteration in 1..=opts.iterations {
        let bright = gpu_brightness(&image, opts.brightness_delta).map_err(BenchError::Gpu)?;
        let cont = gpu_contrast(&bright, opts.contrast_scale).map_err(BenchError::Gpu)?;
        image = gpu_sepia(&cont).map_err(BenchError::Gpu)?;

        // Sampling at the end of each batch keeps snapshot cost out of
        // the hot per-iteration loop while still giving the user enough
        // granularity to see installs land.
        if iteration % opts.batch == 0 {
            snapshots.push(Snapshot {
                iteration,
                spec_cache_len: spec_cache_len(),
                specialised_install_count: specialised_install_count(),
                specialised_dispatch_count: specialised_dispatch_count(),
                deoptimisation_count: deoptimisation_count(),
            });
        }
    }

    // Capture a final snapshot if the loop didn't end on a batch
    // boundary, so the user always sees the terminal counter values.
    if opts.iterations % opts.batch != 0 {
        snapshots.push(Snapshot {
            iteration: opts.iterations,
            spec_cache_len: spec_cache_len(),
            specialised_install_count: specialised_install_count(),
            specialised_dispatch_count: specialised_dispatch_count(),
            deoptimisation_count: deoptimisation_count(),
        });
    }

    // Make sure the output directory exists.  `create_dir_all` is
    // idempotent and a no-op if the directory already exists.
    std::fs::create_dir_all(&opts.output_dir).map_err(|err| BenchError::Io {
        path: opts.output_dir.clone(),
        err,
    })?;

    let result_path = join(&opts.output_dir, "result.ppm");
    let encoded = image_codec_ppm::encode_ppm(&image);
    std::fs::write(&result_path, &encoded).map_err(|err| BenchError::Io {
        path: result_path.clone(),
        err,
    })?;

    let summary = BenchSummary {
        input: opts.input.clone(),
        output_dir: opts.output_dir.clone(),
        image_width: width,
        image_height: height,
        iterations: opts.iterations,
        batch: opts.batch,
        brightness_delta: opts.brightness_delta,
        contrast_scale: opts.contrast_scale,
        snapshots,
        final_result_bytes: encoded.len(),
    };

    let json = render_summary_json(&summary);
    let summary_path = join(&opts.output_dir, "summary.json");
    std::fs::write(&summary_path, json.as_bytes()).map_err(|err| BenchError::Io {
        path: summary_path,
        err,
    })?;

    Ok(summary)
}

/// Render the bench summary as a JSON document.  We write JSON by
/// hand because pulling in `serde_json` for one fixed-shape object
/// is more dependency than the demo justifies.
///
/// The shape is documented in the README so downstream tools (e.g.
/// a future Grafana scraper) can rely on it.
pub fn render_summary_json(s: &BenchSummary) -> String {
    let mut out = String::with_capacity(256 + 96 * s.snapshots.len());
    out.push_str("{\n");
    push_json_kv_string(&mut out, "input", &s.input, true);
    push_json_kv_string(&mut out, "output_dir", &s.output_dir, true);
    push_json_kv_u64(&mut out, "image_width", s.image_width as u64, true);
    push_json_kv_u64(&mut out, "image_height", s.image_height as u64, true);
    push_json_kv_u64(&mut out, "iterations", s.iterations as u64, true);
    push_json_kv_u64(&mut out, "batch", s.batch as u64, true);
    push_json_kv_i64(&mut out, "brightness_delta", s.brightness_delta as i64, true);
    push_json_kv_f32(&mut out, "contrast_scale", s.contrast_scale, true);
    push_json_kv_u64(
        &mut out,
        "final_result_bytes",
        s.final_result_bytes as u64,
        true,
    );
    out.push_str("  \"snapshots\": [");
    for (i, snap) in s.snapshots.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str("\n    {");
        out.push_str(&format!("\"iteration\": {}", snap.iteration));
        out.push_str(&format!(
            ", \"spec_cache_len\": {}",
            snap.spec_cache_len
        ));
        out.push_str(&format!(
            ", \"specialised_install_count\": {}",
            snap.specialised_install_count
        ));
        out.push_str(&format!(
            ", \"specialised_dispatch_count\": {}",
            snap.specialised_dispatch_count
        ));
        out.push_str(&format!(
            ", \"deoptimisation_count\": {}",
            snap.deoptimisation_count
        ));
        out.push('}');
    }
    if s.snapshots.is_empty() {
        out.push_str("]\n");
    } else {
        out.push_str("\n  ]\n");
    }
    out.push_str("}\n");
    out
}

fn push_json_kv_string(out: &mut String, key: &str, value: &str, trailing_comma: bool) {
    out.push_str("  \"");
    out.push_str(key);
    out.push_str("\": \"");
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    if trailing_comma {
        out.push(',');
    }
    out.push('\n');
}

fn push_json_kv_u64(out: &mut String, key: &str, value: u64, trailing_comma: bool) {
    out.push_str(&format!(
        "  \"{}\": {}{}\n",
        key,
        value,
        if trailing_comma { "," } else { "" }
    ));
}

fn push_json_kv_i64(out: &mut String, key: &str, value: i64, trailing_comma: bool) {
    out.push_str(&format!(
        "  \"{}\": {}{}\n",
        key,
        value,
        if trailing_comma { "," } else { "" }
    ));
}

fn push_json_kv_f32(out: &mut String, key: &str, value: f32, trailing_comma: bool) {
    out.push_str(&format!(
        "  \"{}\": {}{}\n",
        key,
        value,
        if trailing_comma { "," } else { "" }
    ));
}

/// Tiny path join helper.  Doesn't pull in `std::path::PathBuf` for
/// one concatenation — the bench output paths are constructed once
/// per run, never compared, and not user-facing beyond display.
fn join(dir: &str, file: &str) -> String {
    if dir.ends_with('/') || dir.ends_with('\\') {
        format!("{}{}", dir, file)
    } else {
        format!("{}/{}", dir, file)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pixel_container::PixelContainer;

    fn tiny_ppm_bytes() -> Vec<u8> {
        // 2×2 RGB image with four distinct colours.  Small enough to
        // run many iterations of the bench without it taking minutes.
        let mut pc = PixelContainer::new(2, 2);
        pc.set_pixel(0, 0, 200, 100, 50, 255);
        pc.set_pixel(1, 0, 50, 100, 200, 255);
        pc.set_pixel(0, 1, 120, 220, 90, 255);
        pc.set_pixel(1, 1, 240, 30, 180, 255);
        image_codec_ppm::encode_ppm(&pc)
    }

    fn workdir(name: &str) -> std::path::PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!(
            "instagram-filters-bench-{}-{}",
            name,
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    fn write_input(dir: &std::path::Path) -> String {
        let p = dir.join("input.ppm");
        std::fs::write(&p, tiny_ppm_bytes()).unwrap();
        p.to_str().unwrap().to_string()
    }

    #[test]
    fn bench_rejects_zero_iterations() {
        let dir = workdir("zero-iters");
        let input = write_input(&dir);
        let opts = BenchOpts {
            input,
            output_dir: dir.to_str().unwrap().into(),
            iterations: 0,
            batch: 1,
            brightness_delta: 10,
            contrast_scale: 1.1,
        };
        let err = run_bench(&opts).unwrap_err();
        assert!(matches!(err, BenchError::InvalidConfig(_)));
    }

    #[test]
    fn bench_rejects_zero_batch() {
        let dir = workdir("zero-batch");
        let input = write_input(&dir);
        let opts = BenchOpts {
            input,
            output_dir: dir.to_str().unwrap().into(),
            iterations: 5,
            batch: 0,
            brightness_delta: 10,
            contrast_scale: 1.1,
        };
        let err = run_bench(&opts).unwrap_err();
        assert!(matches!(err, BenchError::InvalidConfig(_)));
    }

    #[test]
    fn bench_writes_outputs_and_snapshots() {
        let dir = workdir("happy-path");
        let input = write_input(&dir);
        let opts = BenchOpts {
            input,
            output_dir: dir.to_str().unwrap().into(),
            iterations: 6,
            batch: 2,
            brightness_delta: 5,
            contrast_scale: 1.0,
        };

        let summary = run_bench(&opts).unwrap();

        // 6 iterations / batch of 2 → 3 snapshots at iterations 2, 4, 6.
        assert_eq!(summary.snapshots.len(), 3);
        assert_eq!(summary.snapshots[0].iteration, 2);
        assert_eq!(summary.snapshots[1].iteration, 4);
        assert_eq!(summary.snapshots[2].iteration, 6);
        assert_eq!(summary.image_width, 2);
        assert_eq!(summary.image_height, 2);

        // Counters are monotonic non-decreasing across snapshots:
        // dispatch never goes down, install never goes down (in the
        // absence of deopts, which won't fire here — constants are
        // stable).
        for w in summary.snapshots.windows(2) {
            assert!(w[1].specialised_dispatch_count >= w[0].specialised_dispatch_count);
            assert!(w[1].specialised_install_count >= w[0].specialised_install_count);
        }

        // result.ppm and summary.json must exist and be non-empty.
        let result_path = dir.join("result.ppm");
        let summary_path = dir.join("summary.json");
        assert!(std::fs::metadata(&result_path).unwrap().len() > 0);
        assert!(std::fs::metadata(&summary_path).unwrap().len() > 0);
    }

    #[test]
    fn bench_appends_terminal_snapshot_on_partial_batch() {
        let dir = workdir("partial-batch");
        let input = write_input(&dir);
        let opts = BenchOpts {
            input,
            output_dir: dir.to_str().unwrap().into(),
            iterations: 7,
            batch: 3,
            brightness_delta: 5,
            contrast_scale: 1.0,
        };

        let summary = run_bench(&opts).unwrap();

        // 7 iterations / batch 3 → snapshots at 3, 6, plus terminal 7.
        assert_eq!(summary.snapshots.len(), 3);
        assert_eq!(summary.snapshots[0].iteration, 3);
        assert_eq!(summary.snapshots[1].iteration, 6);
        assert_eq!(summary.snapshots[2].iteration, 7);
    }

    #[test]
    fn bench_input_too_large_errors() {
        let dir = workdir("too-large");
        // Synthesise a fake file that exceeds the cap.  We don't
        // actually allocate 64 MiB — `set_len` on a file is sparse on
        // Linux/macOS and triggers the metadata check first.
        let p = dir.join("big.ppm");
        let f = std::fs::File::create(&p).unwrap();
        f.set_len(70 * 1024 * 1024).unwrap();
        drop(f);

        let opts = BenchOpts {
            input: p.to_str().unwrap().into(),
            output_dir: dir.to_str().unwrap().into(),
            iterations: 1,
            batch: 1,
            brightness_delta: 0,
            contrast_scale: 1.0,
        };
        let err = run_bench(&opts).unwrap_err();
        assert!(matches!(err, BenchError::InputTooLarge { .. }));
    }

    #[test]
    fn bench_missing_input_errors() {
        let dir = workdir("missing-input");
        let opts = BenchOpts {
            input: dir.join("does-not-exist.ppm").to_str().unwrap().into(),
            output_dir: dir.to_str().unwrap().into(),
            iterations: 1,
            batch: 1,
            brightness_delta: 0,
            contrast_scale: 1.0,
        };
        let err = run_bench(&opts).unwrap_err();
        assert!(matches!(err, BenchError::Io { .. }));
    }

    #[test]
    fn json_summary_round_trips_known_fields() {
        let s = BenchSummary {
            input: "in.ppm".into(),
            output_dir: "out".into(),
            image_width: 16,
            image_height: 16,
            iterations: 2,
            batch: 1,
            brightness_delta: 12,
            contrast_scale: 1.5,
            final_result_bytes: 100,
            snapshots: vec![
                Snapshot {
                    iteration: 1,
                    spec_cache_len: 0,
                    specialised_install_count: 0,
                    specialised_dispatch_count: 0,
                    deoptimisation_count: 0,
                },
                Snapshot {
                    iteration: 2,
                    spec_cache_len: 3,
                    specialised_install_count: 3,
                    specialised_dispatch_count: 4,
                    deoptimisation_count: 0,
                },
            ],
        };
        let json = render_summary_json(&s);
        // Quick sanity: every known field name shows up.
        assert!(json.contains("\"input\": \"in.ppm\""));
        assert!(json.contains("\"image_width\": 16"));
        assert!(json.contains("\"brightness_delta\": 12"));
        assert!(json.contains("\"snapshots\""));
        assert!(json.contains("\"spec_cache_len\": 3"));
        assert!(json.contains("\"specialised_dispatch_count\": 4"));
    }

    #[test]
    fn json_summary_escapes_quotes_and_backslashes() {
        let s = BenchSummary {
            input: "a\"b\\c".into(),
            output_dir: "d".into(),
            image_width: 1,
            image_height: 1,
            iterations: 1,
            batch: 1,
            brightness_delta: 0,
            contrast_scale: 1.0,
            final_result_bytes: 0,
            snapshots: vec![],
        };
        let json = render_summary_json(&s);
        // Both characters escaped — guards against an output path with
        // a quote in it producing invalid JSON.
        assert!(json.contains("\"input\": \"a\\\"b\\\\c\""));
    }

    #[test]
    fn json_empty_snapshots_renders_empty_array() {
        let s = BenchSummary {
            input: "i".into(),
            output_dir: "o".into(),
            image_width: 1,
            image_height: 1,
            iterations: 0,
            batch: 1,
            brightness_delta: 0,
            contrast_scale: 1.0,
            final_result_bytes: 0,
            snapshots: vec![],
        };
        let json = render_summary_json(&s);
        assert!(json.contains("\"snapshots\": []"));
    }
}
