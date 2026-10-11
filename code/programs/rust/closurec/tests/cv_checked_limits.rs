//! CV02: exercise limit configuration and execution through a real process.
//! Existing outputs must survive failures before any accepted compilation output.
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};
#[path = "common/chronology_ndjson.rs"]
mod chronology_ndjson;

struct Fixture {
    dir: PathBuf,
    input: PathBuf,
    output: PathBuf,
    sidecar: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        static SEQ: AtomicU64 = AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "closurec-cvlimits-{}-{}",
            std::process::id(),
            SEQ.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&dir).unwrap();
        let input = dir.join("a.js");
        std::fs::write(&input, "report(1+2);").unwrap();
        let output = dir.join("out.js");
        let sidecar = dir.join("out.js.cv.json");
        Self {
            dir,
            input,
            output,
            sidecar,
        }
    }
    fn run(&self, level: &str, raw_limits: &str, traced: bool, stdout: bool) -> Output {
        self.run_options(level, raw_limits, traced, stdout, &[])
    }
    fn run_options(
        &self,
        level: &str,
        raw_limits: &str,
        traced: bool,
        stdout: bool,
        options: &[&str],
    ) -> Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_closurec"));
        command
            .args(["--js"])
            .arg(&self.input)
            .args(["--compilation_level", level]);
        if !raw_limits.is_empty() {
            command.args(["--correlation_vector_limits", raw_limits]);
        }
        if !stdout {
            command.arg("--js_output_file").arg(&self.output);
        }
        if traced {
            command.arg("--correlation_vector");
        }
        command
            .args(options)
            .current_dir(&self.dir)
            .output()
            .unwrap()
    }
    fn preserve_existing(&self) {
        std::fs::write(&self.output, "original JS").unwrap();
        std::fs::write(&self.sidecar, "original provenance").unwrap();
    }
    fn assert_preserved(&self) {
        assert_eq!(
            std::fs::read_to_string(&self.output).unwrap(),
            "original JS"
        );
        assert_eq!(
            std::fs::read_to_string(&self.sidecar).unwrap(),
            "original provenance"
        );
        assert_eq!(
            std::fs::read_dir(&self.dir).unwrap().count(),
            3,
            "unexpected success or temporary artifact"
        );
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        assert_eq!(self.dir.parent(), Some(std::env::temp_dir().as_path()));
        std::fs::remove_dir_all(&self.dir).unwrap();
    }
}

#[test]
fn checked_limits_are_real_cli_configuration_with_tracing_neutral_success() {
    for level in [
        "WHITESPACE_ONLY",
        "SIMPLE",
        "ADVANCED",
        "BUNDLE",
        "TRANSPILE_ONLY",
    ] {
        let fixture = Fixture::new();
        let plain = fixture.run(level, "", false, false);
        assert!(
            plain.status.success(),
            "{}",
            String::from_utf8_lossy(&plain.stderr)
        );
        let expected = std::fs::read(&fixture.output).unwrap();
        let traced = fixture.run(level, " max_nodes = 10000 ,max_edges=10000,max_events=10000,max_metadata_values=10000,max_metadata_depth=64,max_metadata_bytes=1000000,max_input_bytes=1000000,max_work=1000000,max_output_bytes=1000000", true, false);
        assert!(
            traced.status.success(),
            "{}",
            String::from_utf8_lossy(&traced.stderr)
        );
        assert_eq!(std::fs::read(&fixture.output).unwrap(), expected);
        let body = std::fs::read_to_string(&fixture.sidecar).unwrap();
        let graph = coding_adventures_correlation_vector::CVLog::from_checked_json(
            &body,
            Default::default(),
        )
        .unwrap();
        assert!(!graph.entries().is_empty());
    }
}

#[test]
fn checked_limits_reject_bad_override_syntax_even_without_tracing() {
    for (raw, detail) in [
        (" ", "empty override pair"),
        ("\t \n", "empty override pair"),
        ("\u{2003}", "empty override pair"),
        ("max_nodes=-1", "unsigned"),
        ("max_nodes=+1", "unsigned"),
        ("max_nodes=1.0", "unsigned"),
        ("max_nodes=١", "unsigned"),
        ("max_nodes=", "unsigned"),
        ("=2", "unknown"),
        ("max_nodes=1, max_nodes = 2", "duplicate"),
        ("unknown=1", "unknown"),
        ("max_work=999999999999999999999999999999", "overflow"),
        ("max_metadata_depth=65", "cannot exceed 64"),
        ("max_nodes=1,", "empty"),
        ("max_nodes", "name=value"),
    ] {
        let fixture = Fixture::new();
        fixture.preserve_existing();
        let result = fixture.run("SIMPLE", raw, false, false);
        assert_eq!(result.status.code(), Some(1));
        assert!(result.stdout.is_empty());
        let message = String::from_utf8_lossy(&result.stderr);
        assert!(
            message.contains("--correlation_vector_limits:"),
            "{raw}: {message}"
        );
        assert!(message.contains(detail), "{raw}: {message}");
        fixture.assert_preserved();
    }
}

#[test]
fn checked_recording_limit_errors_preserve_files_and_successful_stdout() {
    for (raw, detail) in [
        ("max_nodes=0", "nodes limit"),
        ("max_edges=0", "edges limit"),
        ("max_events=0", "events limit"),
        ("max_metadata_values=0", "metadata values limit"),
        ("max_metadata_depth=0", "metadata depth limit"),
        ("max_metadata_bytes=0", "metadata bytes limit"),
        ("max_work=0", "work limit"),
    ] {
        for stdout in [false, true] {
            let fixture = Fixture::new();
            fixture.preserve_existing();
            let result = fixture.run("SIMPLE", raw, true, stdout);
            assert_eq!(
                result.status.code(),
                Some(1),
                "{raw}: {}",
                String::from_utf8_lossy(&result.stderr)
            );
            assert!(result.stdout.is_empty(), "success-shaped stdout for {raw}");
            let message = String::from_utf8_lossy(&result.stderr);
            assert!(message.contains("provenance failed at"), "{message}");
            assert!(message.contains(detail), "{message}");
            fixture.assert_preserved();
        }
    }
}

#[test]
fn checked_export_limit_errors_preserve_files_before_every_materialized_view() {
    for options in [
        vec![],
        vec!["--correlation_vector_pretty"],
        vec!["--correlation_vector_format", "NDJSON"],
        vec!["--correlation_vector_filter", "constant-fold"],
        vec![
            "--correlation_vector_format",
            "NDJSON",
            "--correlation_vector_filter",
            "lex",
            "--correlation_vector_filter_invert",
        ],
        vec![
            "--correlation_vector_format",
            "NONE",
            "--correlation_vector_summary",
        ],
        vec![
            "--correlation_vector_format",
            "NONE",
            "--correlation_vector_summary",
            "--correlation_vector_summary_format",
            "JSON",
        ],
        vec![
            "--correlation_vector_format",
            "NONE",
            "--correlation_vector_summary_only",
            "--correlation_vector_summary",
            "--correlation_vector_summary_format",
            "KV",
        ],
    ] {
        for stdout in [false, true] {
            let fixture = Fixture::new();
            fixture.preserve_existing();
            let result =
                fixture.run_options("SIMPLE", "max_output_bytes=0", true, stdout, &options);
            assert_eq!(
                result.status.code(),
                Some(1),
                "{options:?}: {}",
                String::from_utf8_lossy(&result.stderr)
            );
            assert!(result.stdout.is_empty());
            let message = String::from_utf8_lossy(&result.stderr);
            assert!(message.contains("provenance failed at"), "{message}");
            assert!(message.contains("output bytes limit"), "{message}");
            fixture.assert_preserved();
        }
    }
}

#[test]
fn checked_none_validates_graph_without_materializing_a_sidecar() {
    let fixture = Fixture::new();
    fixture.preserve_existing();
    let accepted = fixture.run_options(
        "SIMPLE",
        "max_output_bytes=0",
        true,
        false,
        &["--correlation_vector_format", "NONE"],
    );
    assert!(
        accepted.status.success(),
        "{}",
        String::from_utf8_lossy(&accepted.stderr)
    );
    assert_eq!(
        std::fs::read_to_string(&fixture.sidecar).unwrap(),
        "original provenance"
    );
    fixture.preserve_existing();
    let rejected = fixture.run_options(
        "SIMPLE",
        "max_work=100",
        true,
        false,
        &["--correlation_vector_format", "NONE"],
    );
    assert_eq!(rejected.status.code(), Some(1));
    assert!(rejected.stdout.is_empty());
    let error = String::from_utf8_lossy(&rejected.stderr);
    assert!(error.contains("validate stage"), "{error}");
    assert!(error.contains("work limit"), "{error}");
    fixture.assert_preserved();
}

#[test]
fn checked_export_rejection_preserves_the_entire_requested_artifact_set() {
    let fixture = Fixture::new();
    fixture.preserve_existing();
    let map = fixture.dir.join("out.map");
    let manifest = fixture.dir.join("manifest.txt");
    std::fs::write(&map, "original map").unwrap();
    std::fs::write(&manifest, "original manifest").unwrap();
    let map_arg = map.to_str().unwrap();
    let manifest_arg = manifest.to_str().unwrap();
    let result = fixture.run_options(
        "SIMPLE",
        "max_output_bytes=0",
        true,
        false,
        &[
            "--create_source_map",
            map_arg,
            "--output_manifest",
            manifest_arg,
        ],
    );
    assert_eq!(result.status.code(), Some(1));
    assert!(result.stdout.is_empty());
    assert_eq!(std::fs::read(&fixture.output).unwrap(), b"original JS");
    assert_eq!(
        std::fs::read(&fixture.sidecar).unwrap(),
        b"original provenance"
    );
    assert_eq!(std::fs::read(&map).unwrap(), b"original map");
    assert_eq!(std::fs::read(&manifest).unwrap(), b"original manifest");
    assert_eq!(std::fs::read_dir(&fixture.dir).unwrap().count(), 5);
}

#[test]
fn checked_process_exports_are_deterministic_across_formats_filters_and_summaries() {
    use coding_adventures_correlation_vector::{CVLog, GraphLimits};
    for level in ["SIMPLE", "ADVANCED"] {
        for options in [
            vec![],
            vec!["--correlation_vector_pretty"],
            vec!["--correlation_vector_format", "NDJSON"],
            vec!["--correlation_vector_filter", "constant-fold"],
            vec![
                "--correlation_vector_pretty",
                "--correlation_vector_filter",
                "lexer_token",
                "--correlation_vector_filter_includes_origin",
            ],
            vec![
                "--correlation_vector_format",
                "NDJSON",
                "--correlation_vector_filter",
                "lex",
                "--correlation_vector_filter_invert",
            ],
            vec!["--correlation_vector_summary"],
            vec![
                "--correlation_vector_summary",
                "--correlation_vector_summary_format",
                "JSON",
            ],
            vec![
                "--correlation_vector_summary",
                "--correlation_vector_summary_format",
                "KV",
                "--correlation_vector_summary_stderr",
            ],
            vec![
                "--correlation_vector_format",
                "NONE",
                "--correlation_vector_summary",
            ],
            vec![
                "--correlation_vector_format",
                "NONE",
                "--correlation_vector_summary",
                "--correlation_vector_summary_only",
            ],
            vec![
                "--correlation_vector_summary",
                "--correlation_vector_summary_only",
                "--correlation_vector_summary_format",
                "JSON",
            ],
        ] {
            let fixture = Fixture::new();
            let plain = fixture.run(level, "", false, false);
            assert!(plain.status.success());
            let expected_js = std::fs::read(&fixture.output).unwrap();
            let mut previous = None;
            for _ in 0..3 {
                let result = fixture.run_options(level, "", true, false, &options);
                assert!(
                    result.status.success(),
                    "{level} {options:?}: {}",
                    String::from_utf8_lossy(&result.stderr)
                );
                assert_eq!(std::fs::read(&fixture.output).unwrap(), expected_js);
                let none = options.contains(&"NONE");
                let sidecar = if none {
                    assert!(!fixture.sidecar.exists());
                    None
                } else {
                    Some(std::fs::read(&fixture.sidecar).unwrap())
                };
                if let Some(body) = &sidecar {
                    let text = std::str::from_utf8(body).unwrap();
                    let json = if options.contains(&"NDJSON") {
                        chronology_ndjson::reconstruct(text, &GraphLimits::default()).unwrap()
                    } else {
                        text.to_owned()
                    };
                    let root: serde_json::Value = serde_json::from_str(&json).unwrap();
                    if options.contains(&"--correlation_vector_filter") {
                        assert_eq!(
                            root["view"],
                            serde_json::json!({"complete":false,"filtered":true})
                        );
                        assert!(CVLog::from_checked_json(&json, GraphLimits::default()).is_err());
                    } else {
                        CVLog::from_checked_json(&json, GraphLimits::default()).unwrap();
                    }
                }
                let current = (result.stdout, result.stderr, sidecar);
                if let Some(previous) = &previous {
                    assert_eq!(&current, previous, "{level} {options:?}");
                }
                previous = Some(current);
            }
        }
    }
}
