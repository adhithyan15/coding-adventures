//! CLOC41: exact ingested Math operands must reach their own replacement.
//!
//! This is a real CLI contract, not a synthetic call-ID fixture. The bridge
//! supplies a member-prefix identity and token identities, but no call root.
//! Missing-parent injection belongs to the private native tests: the CLI has
//! no interface for forging a parent ID. Here real budget/parse refusals must
//! preserve all four publication destinations and fail normally.
use coding_adventures_correlation_vector::{CVLog, GraphLimits};
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

const SOURCE: &str = "closurec_source_v1";
const NODE: &str = "closurec_node_v1";

struct Fixture {
    dir: PathBuf,
    input: PathBuf,
    paths: [PathBuf; 4],
}
impl Fixture {
    fn new(source: &str) -> Self {
        static SEQ: AtomicU64 = AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "closurec-c41-{}-{}",
            std::process::id(),
            SEQ.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&dir).unwrap();
        let dir = dir.canonicalize().unwrap();
        let input = dir.join("input.js");
        std::fs::write(&input, source.as_bytes()).unwrap();
        let paths = ["out.js", "out.map", "manifest.txt", "trace.json"].map(|p| dir.join(p));
        Self { dir, input, paths }
    }
    fn run(&self, level: &str, traced: bool, extra: &[&str]) -> Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_closurec"));
        command
            .args(["--compilation_level", level, "--js"])
            .arg(&self.input)
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
        command.args(extra).current_dir(&self.dir).output().unwrap()
    }
    fn artifacts(&self) -> Vec<Vec<u8>> {
        self.paths[..3]
            .iter()
            .map(|p| std::fs::read(p).unwrap())
            .collect()
    }
    fn compile(&self, level: &str) -> Value {
        // Old-output guards precede lineage assertions: setup/parse errors must
        // never masquerade as the intended missing-result semantic red.
        success(self.run(level, false, &[]));
        let plain = self.artifacts();
        success(self.run(level, true, &[]));
        assert_eq!(
            self.artifacts(),
            plain,
            "tracing changed JS/map/manifest bytes"
        );
        let trace = std::fs::read(&self.paths[3]).unwrap();
        success(self.run(level, true, &[]));
        assert_eq!(self.artifacts(), plain);
        assert_eq!(
            std::fs::read(&self.paths[3]).unwrap(),
            trace,
            "unstable trace"
        );
        let text = std::str::from_utf8(&trace).unwrap();
        CVLog::from_checked_json(text, GraphLimits::default()).unwrap();
        println!("C41_CLI_BASELINE {level}");
        serde_json::from_slice(&trace).unwrap()
    }
    fn seed(&self) -> Vec<Vec<u8>> {
        self.paths
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let bytes = format!("prior\0C41 artifact {i}").into_bytes();
                std::fs::write(p, &bytes).unwrap();
                bytes
            })
            .collect()
    }
    fn preserved(&self, before: &[Vec<u8>]) {
        for (path, bytes) in self.paths.iter().zip(before) {
            assert_eq!(&std::fs::read(path).unwrap(), bytes);
        }
        assert_eq!(std::fs::read_dir(&self.dir).unwrap().count(), 5);
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        assert_eq!(
            self.dir.parent(),
            Some(std::env::temp_dir().canonicalize().unwrap().as_path())
        );
        std::fs::remove_dir_all(&self.dir).unwrap();
    }
}
fn success(output: Output) {
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stdout.is_empty());
}
fn clock(value: &Value) -> u64 {
    u64::from_str_radix(value.as_str().unwrap(), 16).unwrap()
}
fn position(source: &str, byte: usize) -> Value {
    let (mut scalar, mut utf16, mut line, mut column, mut utf16_column) = (0, 0, 0, 0, 0);
    let mut cr = false;
    for ch in source[..byte].chars() {
        scalar += 1;
        utf16 += ch.len_utf16();
        if ch == '\n' && cr {
        } else if matches!(ch, '\r' | '\n' | '\u{2028}' | '\u{2029}') {
            line += 1;
            column = 0;
            utf16_column = 0;
        } else {
            column += 1;
            utf16_column += ch.len_utf16();
        }
        cr = ch == '\r';
    }
    json!({"byte_offset":byte.to_string(),"scalar_offset":scalar.to_string(),
        "utf16_offset":utf16.to_string(),"line":line.to_string(),
        "scalar_column":column.to_string(),"utf16_column":utf16_column.to_string()})
}
fn ancestry(wire: &Value, start: &str) -> BTreeSet<String> {
    fn visit(w: &Value, id: &str, active: &mut BTreeSet<String>, seen: &mut BTreeSet<String>) {
        assert!(!active.contains(id), "cycle");
        if !seen.insert(id.to_owned()) {
            return;
        }
        let entry = w["entries"].get(id).expect("dangling CV parent");
        active.insert(id.to_owned());
        for parent in entry["parent_ids"].as_array().unwrap() {
            visit(w, parent.as_str().unwrap(), active, seen);
        }
        active.remove(id);
    }
    let mut seen = BTreeSet::new();
    visit(wire, start, &mut BTreeSet::new(), &mut seen);
    seen
}
fn input_root(w: &Value, f: &Fixture) -> String {
    let found: Vec<_> = w["entries"]
        .as_object()
        .unwrap()
        .iter()
        .filter(|(_, e)| {
            e["origin"]["source"] == "input_file"
                && e["origin"]["location"] == f.input.to_string_lossy().as_ref()
        })
        .collect();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].1["parent_ids"], json!([]));
    let consumed = std::fs::read(&f.input).unwrap();
    assert_eq!(
        found[0].1["origin"]["meta"][SOURCE],
        json!({"schema":"1","role":"input",
            "buffer_domain":"read_to_string_utf8","encoding":"utf8",
            "grammar":"javascript_standard","edition":"es2025",
            "sha256":coding_adventures_sha256::sha256_hex(&consumed),
            "byte_length":consumed.len().to_string()})
    );
    found[0].0.clone()
}
fn inventory(w: &Value, root: &str) -> Vec<String> {
    let rows: Vec<_> = w["entries"][root]["contributions"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["source"] == "source.inventory" && c["tag"] == "finalized")
        .collect();
    assert_eq!(rows.len(), 1);
    rows[0]["meta"][SOURCE]["piece_ids"]
        .as_array()
        .unwrap()
        .iter()
        .map(|id| id.as_str().unwrap().to_owned())
        .collect()
}
fn token(w: &Value, root: &str, source: &str, start: usize, end: usize) -> String {
    let found: Vec<_> = inventory(w, root)
        .into_iter()
        .filter(|id| {
            let piece = &w["entries"][id]["origin"]["meta"][SOURCE];
            piece["kind"] == "token"
                && piece["start"] == position(source, start)
                && piece["end"] == position(source, end)
        })
        .collect();
    assert_eq!(found.len(), 1, "exact token {:?}", &source[start..end]);
    let id = &found[0];
    assert_eq!(w["entries"][id]["parent_ids"], json!([root]));
    assert_eq!(w["entries"][id]["origin"]["source"], "lexer_token");
    assert_eq!(w["entries"][id]["origin"]["meta"][SOURCE]["input_id"], root);
    id.clone()
}
fn member(w: &Value, root: &str, source: &str, start: usize, end: usize) -> String {
    let rows: Vec<_> = w["entries"]
        .as_object()
        .unwrap()
        .iter()
        .filter(|(_, e)| {
            let m = &e["origin"]["meta"][NODE];
            e["origin"]["source"] == "typed_member"
                && m["kind"] == "MemberExpression"
                && m["input_id"] == root
                && m["start"] == position(source, start)
                && m["end"] == position(source, end)
        })
        .collect();
    assert_eq!(rows.len(), 1);
    let (id, e) = rows[0];
    assert_eq!(e["origin"]["location"], "member_prefix");
    let m = &e["origin"]["meta"][NODE];
    let pieces = inventory(w, root);
    let first = pieces
        .iter()
        .position(|p| p == m["first_piece_id"].as_str().unwrap())
        .unwrap();
    let last = pieces
        .iter()
        .position(|p| p == m["last_piece_id"].as_str().unwrap())
        .unwrap();
    assert!(first < last);
    assert_eq!(e["parent_ids"], json!(pieces[first..=last]));
    let mut cursor = start;
    for p in &pieces[first..=last] {
        let entry = &w["entries"][p];
        let physical = &entry["origin"]["meta"][SOURCE];
        assert_eq!(entry["parent_ids"], json!([root]));
        assert_eq!(physical["start"], position(source, cursor));
        let finish: usize = physical["end"]["byte_offset"]
            .as_str()
            .unwrap()
            .parse()
            .unwrap();
        assert!(finish > cursor);
        assert_eq!(physical["end"], position(source, finish));
        cursor = finish;
    }
    assert_eq!(cursor, end);
    id.clone()
}
fn own_fold(w: &Value, member: &str, before: &str, after: &str) -> String {
    let rows: Vec<_> = w["entries"]
        .as_object()
        .unwrap()
        .iter()
        .filter(|(id, e)| {
            ancestry(w, id).contains(member)
                && e["contributions"].as_array().unwrap().iter().any(|c| {
                    c["source"] == "constant-fold"
                        && c["tag"] == "folded"
                        && c["meta"]["before"] == before
                        && c["meta"]["after"] == after
                        && c["meta"]["new_cv"] == id.as_str()
                })
        })
        .collect();
    assert_eq!(rows.len(), 1, "one result-owned {before} -> {after}");
    rows[0].0.clone()
}
fn event<'a>(w: &'a Value, kind: &str, id: &str) -> &'a Value {
    let rows: Vec<_> = w["journal"]["events"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|e| e["event"]["kind"] == kind && e["event"]["entity"] == id)
        .collect();
    assert_eq!(rows.len(), 1, "one {kind} event for {id}");
    rows[0]
}
fn joined(w: &Value, root: &str, result: &str, parents: &[String], children: &[String]) {
    let e = &w["entries"][result];
    let parent = e["parent_ids"].as_array().unwrap();
    assert_eq!(parent.len(), 1);
    let merged = parent[0].as_str().unwrap();
    assert_eq!(w["entries"][merged]["parent_ids"], json!(parents));
    let folds: Vec<_> = e["contributions"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["source"] == "constant-fold" && c["tag"] == "folded")
        .collect();
    assert_eq!(folds.len(), 1);
    assert_eq!(folds[0]["meta"]["parent_cv"], merged);
    assert_eq!(folds[0]["meta"]["new_cv"], result);
    let merge = event(w, "merge", merged);
    let derive = event(w, "derive", result);
    let own = event(w, "contribution", result);
    assert!(clock(&merge["sequence"]) < clock(&derive["sequence"]));
    assert!(clock(&derive["sequence"]) < clock(&own["sequence"]));
    assert_eq!(merge["context"], derive["context"]);
    assert_eq!(derive["context"], own["context"]);
    for child in children {
        let rewritten = event(w, "contribution", child);
        assert_eq!(rewritten["context"], merge["context"]);
        assert!(clock(&rewritten["sequence"]) < clock(&merge["sequence"]));
    }
    let events = w["journal"]["events"].as_array().unwrap();
    let pass = events
        .iter()
        .find(|e| e["sequence"] == merge["context"])
        .unwrap();
    assert_eq!(pass["event"]["kind"], "context_begin");
    assert_eq!(pass["event"]["scope"]["kind"], "pass");
    assert_eq!(clock(&pass["event"]["scope"]["sweep"]), 0);
    let schedule = events
        .iter()
        .find(|e| e["event"]["kind"] == "schedule" && e["context"] == pass["context"])
        .unwrap();
    let slot = clock(&pass["event"]["scope"]["slot"]) as usize;
    assert_eq!(schedule["event"]["passes"][slot]["name"], "constant-fold");
    let end = events
        .iter()
        .find(|e| e["event"]["kind"] == "context_end" && e["event"]["begin"] == pass["sequence"])
        .unwrap();
    assert_eq!(end["event"]["outcome"]["kind"], "accepted");
    assert_eq!(end["event"]["outcome"]["changed"], true);
    assert!(clock(&own["sequence"]) < clock(&end["sequence"]));
    // Compare all reached physical pieces, not just presence of the desired words.
    let reached = ancestry(w, result);
    let physical: BTreeSet<_> = inventory(w, root)
        .into_iter()
        .filter(|p| reached.contains(p))
        .collect();
    let expected: BTreeSet<_> = parents
        .iter()
        .flat_map(|p| ancestry(w, p))
        .filter(|p| w["entries"][p]["origin"]["meta"][SOURCE]["input_id"] == root)
        .collect();
    assert_eq!(physical, expected);
}

#[test]
fn c41_six_math_methods_keep_exact_repeated_unicode_crlf_source_parents() {
    let mut source = "/*😀 repeated Math.abs(5.75) is comment trivia*/\r\n".to_owned();
    let methods = [
        ("abs", "5.75"),
        ("floor", "5"),
        ("ceil", "6"),
        ("round", "6"),
        ("trunc", "5"),
        ("sign", "1"),
    ];
    let mut cases = Vec::new();
    let mut expected_js = String::new();
    for repeat in 0..2 {
        for (method, after) in methods {
            let start = source.len() + "report(".len();
            source.push_str(&format!("report(Math.{method}(5.75));\r\n"));
            cases.push((repeat, method, after, start));
            expected_js.push_str(&format!("report({after});\n"));
        }
    }
    for level in ["SIMPLE", "ADVANCED"] {
        let f = Fixture::new(&source);
        let w = f.compile(level);
        assert_eq!(std::fs::read_to_string(&f.paths[0]).unwrap(), expected_js);
        let root = input_root(&w, &f);
        let mut identities = BTreeSet::new();
        for (repeat, method, after, start) in &cases {
            let end = start + "Math.".len() + method.len();
            let m = member(&w, &root, &source, *start, end);
            let receiver = token(&w, &root, &source, *start, start + 4);
            let property = token(&w, &root, &source, start + 5, end);
            let argument = token(&w, &root, &source, end + 1, end + 5);
            let result = own_fold(&w, &m, &format!("Math.{method}(5.75)"), after);
            joined(
                &w,
                &root,
                &result,
                &[m.clone(), receiver, property, argument],
                &[],
            );
            assert!(
                identities.insert(m) && identities.insert(result),
                "same spelling reminted/aliased"
            );
            println!("C41_CLI_LINEAGE {level} {method} repeat={repeat}");
        }
    }
}

#[test]
fn c41_nested_unary_and_binary_history_precedes_own_math_event() {
    let source = "/*😀*/\r\nreport(Math.abs(-(2+3)));\r\n";
    for level in ["SIMPLE", "ADVANCED"] {
        let f = Fixture::new(source);
        let w = f.compile(level);
        assert_eq!(
            std::fs::read_to_string(&f.paths[0]).unwrap(),
            "report(5);\n"
        );
        let root = input_root(&w, &f);
        let start = source.find("Math.abs").unwrap();
        let m = member(&w, &root, source, start, start + 8);
        let receiver = token(&w, &root, source, start, start + 4);
        let property = token(&w, &root, source, start + 5, start + 8);
        let two = source.find('2').unwrap();
        let three = source.find('3').unwrap();
        let two_id = token(&w, &root, source, two, two + 1);
        let three_id = token(&w, &root, source, three, three + 1);
        let binary: Vec<_> = w["entries"]
            .as_object()
            .unwrap()
            .iter()
            .filter(|(id, e)| {
                ancestry(&w, id).contains(&two_id)
                    && ancestry(&w, id).contains(&three_id)
                    && e["contributions"].as_array().unwrap().iter().any(|c| {
                        c["source"] == "constant-fold"
                            && c["meta"]["before"] == "(2) + (3)"
                            && c["meta"]["after"] == "5"
                    })
            })
            .collect();
        assert_eq!(binary.len(), 1);
        let binary = binary[0].0.clone();
        let unary: Vec<_> = w["entries"]
            .as_object()
            .unwrap()
            .iter()
            .filter(|(id, e)| {
                ancestry(&w, id).contains(&binary)
                    && e["contributions"].as_array().unwrap().iter().any(|c| {
                        c["source"] == "constant-fold"
                            && c["meta"]["before"] == "-(5)"
                            && c["meta"]["after"] == "-5"
                    })
            })
            .collect();
        assert_eq!(unary.len(), 1);
        let unary = unary[0].0.clone();
        let binary_merge = w["entries"][&binary]["parent_ids"].as_array().unwrap();
        assert_eq!(binary_merge.len(), 1);
        assert_eq!(
            w["entries"][binary_merge[0].as_str().unwrap()]["parent_ids"],
            json!([two_id, three_id])
        );
        assert_eq!(w["entries"][&unary]["parent_ids"], json!([binary]));
        let result = own_fold(&w, &m, "Math.abs(-5)", "5");
        joined(
            &w,
            &root,
            &result,
            &[m, receiver, property, unary.clone()],
            &[binary, unary],
        );
        println!("C41_CLI_NESTED {level}");
    }
}

#[test]
fn c41_declined_calls_keep_effects_and_do_not_claim_a_math_rewrite() {
    let source = "report(Math.abs(1,side()));report(Math.floor(value));report(other.abs(5));report(Math.ceil(-0.4));report(Math.sign(NaN));";
    for level in ["SIMPLE", "ADVANCED"] {
        let f = Fixture::new(source);
        let w = f.compile(level);
        assert!(std::fs::read_to_string(&f.paths[0])
            .unwrap()
            .contains("side()"));
        for entry in w["entries"].as_object().unwrap().values() {
            for c in entry["contributions"].as_array().unwrap() {
                assert!(
                    !(c["source"] == "constant-fold"
                        && c["tag"] == "folded"
                        && c["meta"]["before"]
                            .as_str()
                            .is_some_and(|v| v.starts_with("Math.")))
                );
            }
        }
        println!("C41_CLI_DECLINED {level}");
    }
}

#[test]
fn c41_real_cli_refusals_preserve_js_map_manifest_and_trace() {
    // Node caps target the outer merge and derive from a complete positive
    // chronology, rather than hardcoding ingestion's current node count.
    // Zero event/metadata caps separately prove early public recording refusal;
    // they do NOT claim a missing-parent injection or outer-phase payload test.
    let source = "report(Math.floor(2+3.75));";
    for level in ["SIMPLE", "ADVANCED"] {
        let f = Fixture::new(source);
        let w = f.compile(level);
        let root = input_root(&w, &f);
        let start = source.find("Math.floor").unwrap();
        let m = member(&w, &root, source, start, start + 10);
        let result = own_fold(&w, &m, "Math.floor(5.75)", "5");
        let merged = w["entries"][&result]["parent_ids"][0].as_str().unwrap();
        let events = w["journal"]["events"].as_array().unwrap();
        let index = events
            .iter()
            .position(|e| e["event"]["kind"] == "merge" && e["event"]["entity"] == merged)
            .unwrap();
        let count = events[..index]
            .iter()
            .filter(|e| {
                matches!(
                    e["event"]["kind"].as_str(),
                    Some("create" | "derive" | "merge")
                )
            })
            .count();
        for cap in [count, count + 1] {
            for format in ["JSON", "NDJSON", "NONE"] {
                let before = f.seed();
                let cap = format!("max_nodes={cap}");
                let out = f.run(
                    level,
                    true,
                    &[
                        "--correlation_vector_limits",
                        &cap,
                        "--correlation_vector_format",
                        format,
                    ],
                );
                refusal(
                    out,
                    &format!("{level} compilation failed at pass stage:"),
                    "constant-fold",
                );
                f.preserved(&before);
                println!("C41_CLI_OUTER_REFUSAL {level} {format} {cap}");
            }
        }
        for limit in ["max_events=0", "max_metadata_bytes=0"] {
            let before = f.seed();
            let out = f.run(level, true, &["--correlation_vector_limits", limit]);
            refusal(out, "provenance failed at ", "limit");
            f.preserved(&before);
            println!("C41_CLI_EARLY_REFUSAL {level} {limit}");
        }
    }
    for (source, cause) in [
        ("\0", "JavaScript tokenization failed:"),
        ("Math.abs(1+);", "JavaScript parse failed:"),
    ] {
        for level in ["SIMPLE", "ADVANCED"] {
            let f = Fixture::new(source);
            let before = f.seed();
            refusal(
                f.run(level, true, &[]),
                &format!("{level} compilation failed at parse stage: {cause}"),
                cause,
            );
            f.preserved(&before);
            println!("C41_CLI_PARSE_REFUSAL {level}");
        }
    }
}
fn refusal(out: Output, prefix: &str, cause: &str) {
    assert_eq!(
        out.status.code(),
        Some(1),
        "normal typed refusal, not panic/setup failure"
    );
    assert!(out.stdout.is_empty());
    let error = String::from_utf8(out.stderr).unwrap();
    assert!(
        error.starts_with(prefix) && error.contains(cause),
        "{error}"
    );
    assert!(!error.contains("panicked") && !error.contains("abort"));
}
