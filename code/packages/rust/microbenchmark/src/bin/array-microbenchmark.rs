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

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    let start = Instant::now();
    // The collector treats a negative tick as an invalid clock observation.
    // Never clamp an out-of-range elapsed duration into a plausible timestamp.
    let mut clock = || i64::try_from(start.elapsed().as_nanos()).unwrap_or(-1);
    match run(
        &args,
        &mut io::stdout(),
        &mut clock,
        env!("CA_RUSTC_VERSION"),
    ) {
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
    compiler_version: &str,
) -> Result<(), String> {
    let mut config = Config::default();
    let mut revision = "unknown".to_owned();
    let mut index = 1;
    while index < args.len() {
        let flag = &args[index];
        if flag == "--help" || flag == "-h" {
            writeln!(stdout, "Usage: array-microbenchmark [--pairs N] [--confidence P] [--factor F] [--seed N] [--target-ns N] [--max-batch N] [--revision ID]")
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
    object.insert("runtime".to_owned(), json!(compiler_version));
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
        ];
        let mut ticks = [0, 2, 3, 5, 6, 8, 9, 11].into_iter();
        let mut output = Vec::new();
        run(
            &args,
            &mut output,
            &mut || ticks.next().unwrap(),
            "rustc-test",
        )
        .unwrap();
        let report: Value = serde_json::from_slice(&output).unwrap();
        assert_eq!(report["workload"], "indexed-array-sum-4096");
        assert_eq!(report["revision"], "abc123");
        assert_eq!(report["runtime"], "rustc-test");
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
            &mut unused_clock,
            "test"
        )
        .is_err());
        assert!(run(
            &["bench".into(), "--pairs".into(), "nope".into()],
            &mut output,
            &mut unused_clock,
            "test"
        )
        .is_err());
        output.clear();
        run(
            &["bench".into(), "--help".into()],
            &mut output,
            &mut unused_clock,
            "test",
        )
        .unwrap();
        assert!(String::from_utf8(output).unwrap().contains("Usage:"));
    }
}
