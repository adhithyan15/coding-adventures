//! CV01: actual compiler sidecars keep compact allocation state in every format.
use coding_adventures_correlation_vector::CVLog;
use serde_json::{Map, Value};
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

struct Fixture {
    dir: PathBuf,
    input: PathBuf,
    output: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        static SEQ: AtomicU64 = AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "closurec-cvcompact-{}-{}",
            std::process::id(),
            SEQ.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&dir).unwrap();
        let input = dir.join("a.js");
        std::fs::write(&input, "report(\"abc\".length+1);").unwrap();
        let output = dir.join("out.js");
        Self { dir, input, output }
    }
    fn run(&self, level: &str, flags: &[&str], output: &std::path::Path) -> Output {
        let result = Command::new(env!("CARGO_BIN_EXE_closurec"))
            .args(["--compilation_level", level, "--js"])
            .arg(&self.input)
            .arg("--js_output_file")
            .arg(output)
            .args(flags)
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        result
    }
    fn sidecar(&self) -> PathBuf {
        PathBuf::from(format!("{}.cv.json", self.output.display()))
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        assert_eq!(self.dir.parent(), Some(std::env::temp_dir().as_path()));
        std::fs::remove_dir_all(&self.dir).unwrap();
    }
}

fn read_log(fixture: &Fixture, ndjson: bool) -> Value {
    let text = std::fs::read_to_string(fixture.sidecar()).unwrap();
    if !ndjson {
        return serde_json::from_str(&text).unwrap();
    }
    let mut entries = Map::new();
    let mut metadata = None;
    for line in text.lines() {
        let value: Value = serde_json::from_str(line).unwrap();
        if let Some(meta) = value.get("_meta") {
            assert!(metadata.is_none());
            metadata = Some(meta.as_object().unwrap().clone());
        } else {
            assert!(metadata.is_none(), "entry after metadata footer");
            let id = value["id"].as_str().unwrap().to_string();
            assert!(
                entries.insert(id, value).is_none(),
                "duplicate streamed identity"
            );
        }
    }
    let mut root = metadata.unwrap();
    root.insert("entries".into(), Value::Object(entries));
    Value::Object(root)
}

#[test]
fn all_sidecar_formats_preserve_compact_state_and_reload_unique_allocations() {
    for level in ["SIMPLE", "ADVANCED"] {
        let f = Fixture::new();
        let plain = f.dir.join("plain.js");
        f.run(level, &[], &plain);
        let expected = std::fs::read(&plain).unwrap();
        let mut first = None;
        for (format, pretty) in [("JSON", false), ("JSON", true), ("NDJSON", false)] {
            let mut flags = vec![
                "--correlation_vector",
                "--correlation_vector_format",
                format,
            ];
            if pretty {
                flags.push("--correlation_vector_pretty");
            }
            f.run(level, &flags, &f.output);
            assert_eq!(std::fs::read(&f.output).unwrap(), expected);
            let log = read_log(&f, format == "NDJSON");
            assert_eq!(log["identity"]["scheme"], "compact-v1");
            let last = u64::from_str_radix(log["identity"]["last_sequence"].as_str().unwrap(), 16)
                .unwrap();
            let entries = log["entries"].as_object().unwrap();
            assert!(!entries.is_empty());
            for (id, entry) in entries {
                assert!(id.starts_with("cv1.") && id.len() == 20);
                assert_eq!(entry["id"], id.as_str());
                assert!(u64::from_str_radix(&id[4..], 16).unwrap() <= last);
                for parent in entry["parent_ids"].as_array().unwrap() {
                    assert!(entries.contains_key(parent.as_str().unwrap()));
                }
            }
            let mut loaded = CVLog::from_json_string(&log.to_string()).unwrap();
            let next = loaded.try_create(None).unwrap();
            assert_eq!(u64::from_str_radix(&next[4..], 16).unwrap(), last + 1);
            assert!(!entries.contains_key(&next));
            if let Some(previous) = &first {
                assert_eq!(&log, previous);
            } else {
                first = Some(log);
            }
        }
        // Fresh process, same paths/configuration: identities/history agree even
        // though canonical object-key serialization is a separate pending slice.
        f.run(level, &["--correlation_vector"], &f.output);
        assert_eq!(read_log(&f, false), first.unwrap());
    }
}

#[test]
fn filtered_formats_declare_partial_views_and_cannot_be_reloaded_as_full_logs() {
    for format in ["JSON", "NDJSON"] {
        let f = Fixture::new();
        f.run(
            "SIMPLE",
            &[
                "--correlation_vector",
                "--correlation_vector_format",
                format,
                "--correlation_vector_filter",
                "constant-fold",
                "--correlation_vector_summary",
            ],
            &f.output,
        );
        let log = read_log(&f, format == "NDJSON");
        assert_eq!(log["identity"]["scheme"], "compact-v1");
        assert_eq!(log["view"]["filtered"], true);
        assert_eq!(log["view"]["complete"], false);
        assert!(CVLog::from_json_string(&log.to_string())
            .err()
            .expect("reject partial view")
            .contains("filtered"));
    }
}

#[test]
fn none_format_computes_summary_without_writing_sidecar() {
    let f = Fixture::new();
    let result = f.run(
        "SIMPLE",
        &[
            "--correlation_vector",
            "--correlation_vector_format",
            "NONE",
            "--correlation_vector_summary",
        ],
        &f.output,
    );
    assert_eq!(
        std::fs::read_to_string(&f.output).unwrap().trim(),
        "report(4);"
    );
    assert!(!f.sidecar().exists());
    assert!(String::from_utf8(result.stdout)
        .unwrap()
        .contains("skipped (format=NONE)"));
}
