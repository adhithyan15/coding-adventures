//! Actual native CLI processes connect the multi-sweep inline/fold witness to
//! recorded invocation contexts. Equal output bytes and valid graph history do
//! not establish source-map or every-transform lineage completeness.
use coding_adventures_correlation_vector::{CVLog, GraphLimits};
use serde_json::Value;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};
#[path = "common/chronology_ndjson.rs"]
mod chronology_ndjson;

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
    assert!(body.lines().any(|line| line.starts_with("{\"_event\":")));
    let footer: Value = serde_json::from_str(body.lines().last().unwrap()).unwrap();
    assert!(footer["_meta"]["journal"].get("events").is_none());
    let raw = chronology_ndjson::reconstruct(&body, &GraphLimits::default()).unwrap();
    reload(&raw);
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

#[test]
fn exact_serialization_cap_succeeds_and_one_less_preserves_all_artifacts() {
    let fixture = Fixture::new("report(1+2);");
    for options in [
        vec![],
        vec!["--correlation_vector_pretty"],
        vec!["--correlation_vector_format", "NDJSON"],
    ] {
        success(fixture.run("SIMPLE", true, &options));
        let cap = std::fs::metadata(&fixture.paths[3]).unwrap().len();
        let exact = format!("max_output_bytes={cap}");
        let mut exact_options = options.clone();
        exact_options.extend(["--correlation_vector_limits", exact.as_str()]);
        success(fixture.run("SIMPLE", true, &exact_options));
        assert_eq!(std::fs::metadata(&fixture.paths[3]).unwrap().len(), cap);
        for (index, path) in fixture.paths.iter().enumerate() {
            std::fs::write(path, format!("original {index}")).unwrap();
        }
        let before: Vec<_> = fixture
            .paths
            .iter()
            .map(|p| std::fs::read(p).unwrap())
            .collect();
        let too_small = format!("max_output_bytes={}", cap - 1);
        let mut too_small_options = options.clone();
        too_small_options.extend(["--correlation_vector_limits", too_small.as_str()]);
        let output = fixture.run("SIMPLE", true, &too_small_options);
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(output.stderr.len() < 8192);
        assert!(String::from_utf8_lossy(&output.stderr).contains("output bytes limit"));
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

#[test]
fn filtered_cli_journal_retains_all_contexts_and_original_watermark() {
    let fixture = Fixture::new("report(1+2);");
    success(fixture.run("SIMPLE", true, &[]));
    let full = reload(&fixture.sidecar());
    success(fixture.run(
        "SIMPLE",
        true,
        &["--correlation_vector_filter", "constant-fold"],
    ));
    let text = fixture.sidecar();
    let partial: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(partial["journal"]["coverage"], "partial");
    assert_eq!(
        partial["journal"]["last_sequence"],
        full["journal"]["last_sequence"]
    );
    let contexts = |wire: &Value| {
        wire["journal"]["events"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| {
                matches!(
                    r["event"]["kind"].as_str(),
                    Some("context_begin" | "context_end" | "schedule")
                )
            })
            .cloned()
            .collect::<Vec<_>>()
    };
    assert_eq!(contexts(&partial), contexts(&full));
    assert!(
        partial["journal"]["events"].as_array().unwrap().len()
            < full["journal"]["events"].as_array().unwrap().len()
    );
    assert!(CVLog::from_json_string(&text).is_err());
    assert!(CVLog::from_checked_json(&text, GraphLimits::default()).is_err());
}

#[test]
fn canonical_ndjson_adapter_preserves_duplicate_evidence_and_bounds_assembly() {
    let body = concat!(
        "{\"contributions\":[],\"deleted\":null,\"id\":\"cv1.0000000000000001\",\"origin\":{\"location\":\"1:1\",\"meta\":{\"id\":\"cv1.0000000000000002\"},\"source\":\"input\",\"timestamp\":null},\"parent_ids\":[]}\n",
        "{\"_event\":{\"context\":null,\"event\":{\"entity\":\"cv1.0000000000000001\",\"kind\":\"create\"},\"sequence\":\"0000000000000001\"}}\n",
        "{\"_meta\":{\"enabled\":true,\"identity\":{\"last_sequence\":\"0000000000000001\",\"scheme\":\"compact-v1\"},\"journal\":{\"coverage\":\"full\",\"last_sequence\":\"0000000000000001\",\"version\":\"chronology-v1\"},\"pass_order\":[]}}\n",
    );
    let limits = GraphLimits::default();
    let raw = chronology_ndjson::reconstruct(body, &limits).unwrap();
    let parsed = reload(&raw);
    assert_eq!(
        parsed["entries"]["cv1.0000000000000001"]["origin"]["meta"]["id"], "cv1.0000000000000002",
        "metadata id does not choose the entry key"
    );
    for attack in [
        body.replace(
            "\"context\":null",
            "\"context\":null,\"\\u0063ontext\":null",
        ),
        body.replace("\"meta\":{", "\"meta\":{\"dup\":0,\"\\u0064up\":1,"),
        body.replace(
            "\"coverage\":\"full\"",
            "\"coverage\":\"full\",\"\\u0063overage\":\"full\"",
        ),
        body.replace(
            "\"coverage\":\"full\"",
            "\"coverage\":\"full\",\"events\":[]",
        ),
    ] {
        let raw = chronology_ndjson::reconstruct(&attack, &limits).unwrap();
        assert!(CVLog::from_checked_json(&raw, limits.clone()).is_err());
        assert!(CVLog::from_json_string(&raw).is_err());
    }
    for limits in [
        GraphLimits {
            max_nodes: 0,
            ..Default::default()
        },
        GraphLimits {
            max_events: 0,
            ..Default::default()
        },
        GraphLimits {
            max_work: 0,
            ..Default::default()
        },
        GraphLimits {
            max_input_bytes: body.len() - 1,
            ..Default::default()
        },
    ] {
        assert!(chronology_ndjson::reconstruct(body, &limits).is_err());
    }
    let lines: Vec<_> = body.lines().collect();
    for malformed in [
        format!("{}\n{}\n{}\n", lines[1], lines[0], lines[2]),
        format!("{body}{}\n", lines[1]),
    ] {
        assert!(chronology_ndjson::reconstruct(&malformed, &GraphLimits::default()).is_err());
    }
}
