//! CV02: exercise limit configuration and execution through a real process.
//! Existing outputs must survive failures before any accepted compilation output.
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

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
        command.current_dir(&self.dir).output().unwrap()
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
