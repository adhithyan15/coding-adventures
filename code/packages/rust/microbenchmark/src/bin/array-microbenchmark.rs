//! Run the VER07 indexed-array comparison and emit a replayable JSON report.

use coding_adventures_microbenchmark::{
    collect, indexed_array, indexed_array_by_index, indexed_array_by_iteration, Config,
    ARRAY_CHECKSUM,
};
use serde_json::json;
use std::env;
use std::io::{self, Write};
use std::process::ExitCode;
use std::time::Instant;

const MAX_RUSTC_VERSION_BYTES: usize = 256;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    let start = Instant::now();
    // The collector treats a negative tick as an invalid clock observation.
    // Never clamp an out-of-range elapsed duration into a plausible timestamp.
    let mut clock = || i64::try_from(start.elapsed().as_nanos()).unwrap_or(-1);
    match run(&args, &mut io::stdout(), &mut clock) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("array-microbenchmark: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run(
    args: &[String],
    stdout: &mut impl Write,
    now: &mut impl FnMut() -> i64,
) -> Result<(), String> {
    let mut config = Config::default();
    let mut revision = "unknown".to_owned();
    let mut rustc_version = "unknown".to_owned();
    let mut index = 1;
    while index < args.len() {
        let flag = &args[index];
        if flag == "--help" || flag == "-h" {
            writeln!(stdout, "Usage: array-microbenchmark [--pairs N] [--confidence P] [--factor F] [--seed N] [--target-ns N] [--max-batch N] [--revision ID] [--rustc-version LABEL]")
                .map_err(|error| error.to_string())?;
            return Ok(());
        }
        let value = args
            .get(index + 1)
            .ok_or_else(|| format!("{flag} requires a value"))?;
        match flag.as_str() {
            "--pairs" => config.pairs = value.parse().map_err(|_| "invalid --pairs".to_owned())?,
            "--confidence" => {
                config.confidence_percent = value
                    .parse()
                    .map_err(|_| "invalid --confidence".to_owned())?
            }
            "--factor" => {
                config.minimum_factor = value.parse().map_err(|_| "invalid --factor".to_owned())?
            }
            "--seed" => config.seed = value.parse().map_err(|_| "invalid --seed".to_owned())?,
            "--target-ns" => {
                config.target_ns = value
                    .parse()
                    .map_err(|_| "invalid --target-ns".to_owned())?
            }
            "--max-batch" => {
                config.max_batch = value
                    .parse()
                    .map_err(|_| "invalid --max-batch".to_owned())?
            }
            "--revision" => revision = value.clone(),
            "--rustc-version" => {
                validate_rustc_version(value)?;
                rustc_version = value.clone();
            }
            _ => return Err(format!("unknown option or positional argument: {flag}")),
        }
        index += 2;
    }

    let values = indexed_array();
    if values.len() != 4096 || values.iter().sum::<i64>() != ARRAY_CHECKSUM {
        return Err("internal indexed-array checksum mismatch".to_owned());
    }
    let mut baseline = |batch| indexed_array_by_index(&values, batch);
    let mut candidate = |batch| indexed_array_by_iteration(&values, batch);
    let report = collect(config, ARRAY_CHECKSUM, &mut baseline, &mut candidate, now)
        .map_err(|error| error.to_string())?;

    let mut document = serde_json::to_value(report).map_err(|error| error.to_string())?;
    let object = document
        .as_object_mut()
        .ok_or("report was not a JSON object")?;
    object.insert("workload".to_owned(), json!("indexed-array-sum-4096"));
    object.insert("invocation".to_owned(), json!(args));
    object.insert("revision".to_owned(), json!(revision));
    object.insert("runtime".to_owned(), json!(rustc_version));
    object.insert(
        "profile".to_owned(),
        json!(if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        }),
    );
    object.insert("os".to_owned(), json!(env::consts::OS));
    object.insert("arch".to_owned(), json!(env::consts::ARCH));
    serde_json::to_writer_pretty(&mut *stdout, &document).map_err(|error| error.to_string())?;
    writeln!(stdout).map_err(|error| error.to_string())?;
    Ok(())
}

/// Accept a bounded one-line label shaped like `rustc --version` output.
/// This validates caller-supplied descriptive metadata; it does not prove
/// which compiler built this executable.
fn validate_rustc_version(value: &str) -> Result<(), String> {
    if value.len() > MAX_RUSTC_VERSION_BYTES {
        return Err(format!(
            "--rustc-version must be at most {MAX_RUSTC_VERSION_BYTES} UTF-8 bytes"
        ));
    }
    if value.is_empty()
        || value.trim() != value
        || value
            .chars()
            .any(|character| character.is_control() || matches!(character, '\u{2028}' | '\u{2029}'))
    {
        return Err(
            "--rustc-version must be a nonblank single line without control characters or line separators".to_owned(),
        );
    }
    let mut words = value.split_ascii_whitespace();
    let version = words
        .next()
        .and_then(|program| (program == "rustc").then(|| words.next()).flatten());
    if !version.is_some_and(|token| {
        token.as_bytes().first().is_some_and(u8::is_ascii_digit) && token.contains('.')
    }) {
        return Err("--rustc-version must look like one-line `rustc --version` output".to_owned());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    #[test]
    fn command_emits_report_metadata_and_fake_clock_observations() {
        let args = vec![
            "array-microbenchmark".to_owned(),
            "--pairs".to_owned(),
            "1".to_owned(),
            "--confidence".to_owned(),
            "90".to_owned(),
            "--factor".to_owned(),
            "1.1".to_owned(),
            "--seed".to_owned(),
            "42".to_owned(),
            "--target-ns".to_owned(),
            "1".to_owned(),
            "--max-batch".to_owned(),
            "1".to_owned(),
            "--revision".to_owned(),
            "abc123".to_owned(),
            "--rustc-version".to_owned(),
            "rustc 0.0.0-test (synthetic metadata fixture)".to_owned(),
        ];
        let mut ticks = [0, 2, 3, 5, 6, 8, 9, 11].into_iter();
        let mut output = Vec::new();
        run(&args, &mut output, &mut || ticks.next().unwrap()).unwrap();
        let report: Value = serde_json::from_slice(&output).unwrap();
        assert_eq!(report["workload"], "indexed-array-sum-4096");
        assert_eq!(report["revision"], "abc123");
        assert_eq!(
            report["runtime"],
            "rustc 0.0.0-test (synthetic metadata fixture)"
        );
        assert_eq!(
            report["profile"],
            if cfg!(debug_assertions) {
                "debug"
            } else {
                "release"
            }
        );
        assert_eq!(report["config"]["pairs"], 1);
        assert_eq!(report["config"]["confidence_percent"], 90);
        assert_eq!(report["config"]["minimum_factor"], 1.1);
        assert_eq!(report["config"]["seed"], 42);
        assert_eq!(report["pairs"][0]["baseline_ns"], 2);
        assert_eq!(report["pairs"][0]["candidate_ns"], 2);
        assert_eq!(report["invocation"].as_array().unwrap().len(), args.len());
        let replay = coding_adventures_microbenchmark::Report::from_json(
            &serde_json::to_string(&report).unwrap(),
        )
        .unwrap();
        assert_eq!(replay.pairs.len(), 1);
    }

    #[test]
    fn command_rejects_bad_flags_and_accepts_help_without_timing() {
        let mut output = Vec::new();
        let mut unused_clock = || panic!("help must not read clock");
        assert!(run(
            &["bench".into(), "positional".into()],
            &mut output,
            &mut unused_clock
        )
        .is_err());
        assert!(run(
            &["bench".into(), "--pairs".into(), "nope".into()],
            &mut output,
            &mut unused_clock
        )
        .is_err());
        output.clear();
        run(
            &["bench".into(), "--help".into()],
            &mut output,
            &mut unused_clock,
        )
        .unwrap();
        assert!(String::from_utf8(output).unwrap().contains("Usage:"));
    }

    #[test]
    fn compiler_version_metadata_is_bounded_and_uses_rustc_version_shape() {
        for valid in ["rustc 1.100.0", "rustc 1.100.0-nightly (abc 2026-11-27)"] {
            assert!(validate_rustc_version(valid).is_ok(), "accepted {valid:?}");
        }
        for invalid in [
            "",
            "unknown",
            "rustc",
            "rustc test-version",
            " rustc 1.100.0",
            "rustc 1.100.0\nforged",
            "rustc 1.100.0\u{7f}forged",
            "rustc 1.100.0\u{2028}forged",
        ] {
            assert!(
                validate_rustc_version(invalid).is_err(),
                "rejected {invalid:?}"
            );
        }
        assert!(validate_rustc_version(&format!("rustc {}", "1".repeat(251))).is_err());
    }

    #[test]
    fn default_runtime_metadata_is_unknown() {
        let mut ticks = [0, 2, 3, 5, 6, 8, 9, 11].into_iter();
        let mut output = Vec::new();
        let config_args = [
            "array-microbenchmark".to_owned(),
            "--pairs".to_owned(),
            "1".to_owned(),
            "--target-ns".to_owned(),
            "1".to_owned(),
            "--max-batch".to_owned(),
            "1".to_owned(),
        ];
        run(&config_args, &mut output, &mut || ticks.next().unwrap()).unwrap();
        let report: Value = serde_json::from_slice(&output).unwrap();
        assert_eq!(report["runtime"], "unknown");
    }
}
