//! Actual native CLI processes connect the multi-sweep inline/fold witness to
//! recorded invocation contexts. Equal output bytes and valid graph history do
//! not establish source-map or every-transform lineage completeness.
use coding_adventures_correlation_vector::{CVLog, GraphLimits};
use serde_json::Value;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

struct Fixture {
    dir: PathBuf,
    input: PathBuf,
    paths: [PathBuf; 4],
}
impl Fixture {
    fn new(source: &str) -> Self {
        static SEQ: AtomicU64 = AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "closurec-cv03-{}-{}",
            std::process::id(),
            SEQ.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&dir).unwrap();
        let input = dir.join("a.js");
        std::fs::write(&input, source).unwrap();
        let paths = ["out.js", "out.map", "manifest.txt", "trace.json"].map(|name| dir.join(name));
        Self { dir, input, paths }
    }
    fn run(&self, level: &str, traced: bool, options: &[&str]) -> Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_closurec"));
        command
            .arg("--js")
            .arg(&self.input)
            .args(["--compilation_level", level])
            .arg("--js_output_file")
            .arg(&self.paths[0])
            .arg("--create_source_map")
            .arg(&self.paths[1])
            .arg("--output_manifest")
            .arg(&self.paths[2]);
        if traced {
            command
                .arg("--correlation_vector")
                .arg("--correlation_vector_output")
                .arg(&self.paths[3]);
        }
        command
            .args(options)
            .env_remove("PSModulePath")
            .current_dir(&self.dir)
            .output()
            .unwrap()
    }
    fn sidecar(&self) -> String {
        std::fs::read_to_string(&self.paths[3]).unwrap()
    }
    fn outputs(&self) -> Vec<Vec<u8>> {
        self.paths[..3]
            .iter()
            .map(|p| std::fs::read(p).unwrap())
            .collect()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        assert_eq!(self.dir.parent(), Some(std::env::temp_dir().as_path()));
        std::fs::remove_dir_all(&self.dir).unwrap();
    }
}
fn success(output: Output) {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stdout.is_empty());
}
fn reload(text: &str) -> Value {
    let wire: Value = serde_json::from_str(text).unwrap();
    assert!(
        wire.get("journal").is_some(),
        "actual CLI snapshot has no chronology declaration"
    );
    let checked = CVLog::from_checked_json(text, GraphLimits::default()).unwrap();
    let compatibility = CVLog::from_json_string(text).unwrap();
    assert!(
        checked.to_json_string().unwrap() == compatibility.to_json_string().unwrap(),
        "checked and compatibility chronology imports disagree"
    );
    wire
}

#[test]
fn simple_and_advanced_trace_neutral_bytes_and_deterministic_complete_journal() {
    for level in ["SIMPLE", "ADVANCED"] {
        let fixture = Fixture::new("function twice(x){return x*2;} console.log(twice(7));");
        success(fixture.run(level, false, &[]));
        let plain = fixture.outputs();
        success(fixture.run(level, true, &[]));
        assert_eq!(fixture.outputs(), plain);
        let first = fixture.sidecar();
        let wire = reload(&first);
        assert_eq!(wire["journal"]["version"], "chronology-v1");
        assert_eq!(wire["journal"]["coverage"], "full");
        assert!(wire["journal"]["events"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["event"]["scope"]["kind"] == "pass"));
        success(fixture.run(level, true, &[]));
        assert_eq!(fixture.sidecar(), first);
        assert_eq!(fixture.outputs(), plain);
    }
}

#[test]
fn actual_inline_exposure_is_folded_in_a_later_sweep_context() {
    let fixture = Fixture::new("function twice(x){return x*2;} console.log(twice(7));");
    success(fixture.run("ADVANCED", true, &[]));
    assert_eq!(
        std::fs::read_to_string(&fixture.paths[0]).unwrap().trim(),
        "console.log(14);"
    );
    let wire = reload(&fixture.sidecar());
    let records = wire["journal"]["events"].as_array().unwrap();
    let (entity, index) = wire["entries"]
        .as_object()
        .unwrap()
        .iter()
        .find_map(|(id, entry)| {
            entry["contributions"]
                .as_array()
                .unwrap()
                .iter()
                .enumerate()
                .find_map(|(index, c)| {
                    (c["source"] == "constant-fold"
                        && c["tag"] == "folded"
                        && c["meta"]["after"] == "14")
                        .then_some((id, index))
                })
        })
        .expect("actual folded result contribution");
    let reference = records
        .iter()
        .find(|r| {
            r["event"]["kind"] == "contribution"
                && r["event"]["entity"] == entity.as_str()
                && r["event"]["index"] == format!("{index:016x}")
        })
        .unwrap();
    let pass = records
        .iter()
        .find(|r| r["sequence"] == reference["context"])
        .unwrap();
    assert_eq!(pass["event"]["scope"]["kind"], "pass");
    let sweep = u64::from_str_radix(pass["event"]["scope"]["sweep"].as_str().unwrap(), 16).unwrap();
    assert!(
        sweep > 0,
        "inline exposes multiplication after the earlier constant-fold pass"
    );
    let schedule = records
        .iter()
        .find(|r| r["event"]["kind"] == "schedule" && r["context"] == pass["context"])
        .unwrap();
    let slot = usize::from_str_radix(pass["event"]["scope"]["slot"].as_str().unwrap(), 16).unwrap();
    assert_eq!(schedule["event"]["passes"][slot]["name"], "constant-fold");
    let inline_slot = schedule["event"]["passes"]
        .as_array()
        .unwrap()
        .iter()
        .position(|p| p["name"] == "inline")
        .unwrap();
    assert!(slot < inline_slot);
    let initial_inline = records
        .iter()
        .find(|r| {
            r["event"]["scope"]["kind"] == "pass"
                && r["context"] == pass["context"]
                && r["event"]["scope"]["slot"] == format!("{inline_slot:016x}")
                && r["event"]["scope"]["sweep"] == "0000000000000000"
        })
        .unwrap();
    assert!(initial_inline["sequence"].as_str().unwrap() < reference["sequence"].as_str().unwrap());
}

#[test]
fn ndjson_reconstructs_full_cli_evidence_without_footer_event_duplication() {
    let fixture = Fixture::new("report(1+2);");
    success(fixture.run("SIMPLE", true, &["--correlation_vector_format", "NDJSON"]));
    let body = fixture.sidecar();
    let mut entries = serde_json::Map::new();
    let mut events = Vec::new();
    let mut footer = None;
    for line in body.lines() {
        let mut value: Value = serde_json::from_str(line).unwrap();
        if value.get("_event").is_some() {
            events.push(value["_event"].take());
        } else if value.get("_meta").is_some() {
            footer = Some(value["_meta"].take());
        } else {
            entries.insert(value["id"].as_str().unwrap().to_owned(), value);
        }
    }
    assert!(!events.is_empty());
    let mut wire = footer.unwrap();
    assert!(wire["journal"].get("events").is_none());
    wire["entries"] = Value::Object(entries);
    wire["journal"]["events"] = Value::Array(events);
    reload(&wire.to_string());
}

#[test]
fn journal_limit_failures_preserve_javascript_map_manifest_and_sidecar_in_all_formats() {
    let fixture = Fixture::new("report(1+2);");
    for (format, pretty) in [
        ("JSON", false),
        ("JSON", true),
        ("NDJSON", false),
        ("NONE", false),
    ] {
        for (index, path) in fixture.paths.iter().enumerate() {
            std::fs::write(path, format!("original {index}")).unwrap();
        }
        let before: Vec<_> = fixture
            .paths
            .iter()
            .map(|p| std::fs::read(p).unwrap())
            .collect();
        let mut options = vec![
            "--correlation_vector_limits",
            "max_events=1",
            "--correlation_vector_format",
            format,
        ];
        if pretty {
            options.push("--correlation_vector_pretty");
        }
        let output = fixture.run("SIMPLE", true, &options);
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(output.stderr.len() < 8192);
        assert!(String::from_utf8_lossy(&output.stderr).contains("provenance"));
        assert_eq!(
            fixture
                .paths
                .iter()
                .map(|p| std::fs::read(p).unwrap())
                .collect::<Vec<_>>(),
            before
        );
        assert_eq!(std::fs::read_dir(&fixture.dir).unwrap().count(), 5);
    }
}
