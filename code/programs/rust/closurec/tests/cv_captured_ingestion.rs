//! D2: start at the replacement and require the exact consumed physical IDs.
//! A second tokenizer with identical text/locations cannot satisfy this test.
use coding_adventures_correlation_vector::{CVLog, GraphLimits};
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

const NS: &str = "closurec_source_v1";
type Pos = [usize; 6];
type Span = [Pos; 2];

struct Fixture {
    dir: PathBuf,
    inputs: Vec<PathBuf>,
    paths: [PathBuf; 4],
}
impl Fixture {
    fn new(sources: &[&str]) -> Self {
        static SEQ: AtomicU64 = AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "closurec-d2-{}-{}",
            std::process::id(),
            SEQ.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&dir).unwrap();
        let inputs = sources
            .iter()
            .enumerate()
            .map(|(i, source)| {
                let path = dir.join(format!("input-{i}.js"));
                std::fs::write(&path, source).unwrap();
                path
            })
            .collect();
        let paths = ["out.js", "out.map", "manifest.txt", "trace.json"].map(|p| dir.join(p));
        Self { dir, inputs, paths }
    }
    fn run(&self, level: &str, traced: bool) -> Output {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_closurec"));
        for input in &self.inputs {
            cmd.arg("--js").arg(input);
        }
        cmd.args(["--compilation_level", level])
            .arg("--js_output_file")
            .arg(&self.paths[0])
            .arg("--create_source_map")
            .arg(&self.paths[1])
            .arg("--output_manifest")
            .arg(&self.paths[2]);
        if traced {
            cmd.arg("--correlation_vector")
                .arg("--correlation_vector_output")
                .arg(&self.paths[3]);
        }
        cmd.current_dir(&self.dir)
            .env_remove("PSModulePath")
            .output()
            .unwrap()
    }
    fn outputs(&self) -> Vec<Vec<u8>> {
        self.paths[..3]
            .iter()
            .map(|p| std::fs::read(p).unwrap())
            .collect()
    }
    fn wire(&self) -> Value {
        let text = std::fs::read_to_string(&self.paths[3]).unwrap();
        let checked = CVLog::from_checked_json(&text, GraphLimits::default()).unwrap();
        assert_eq!(
            checked.to_json_string().unwrap(),
            CVLog::from_json_string(&text)
                .unwrap()
                .to_json_string()
                .unwrap()
        );
        serde_json::from_str(&text).unwrap()
    }
    fn compile(&self, level: &str) -> Value {
        // These legacy success/byte/determinism guards execute BEFORE lineage.
        // Thus a bad fixture is not mistaken for a producer semantic red.
        success(self.run(level, false));
        let plain = self.outputs();
        success(self.run(level, true));
        assert_eq!(self.outputs(), plain, "trace changed JS/map/manifest");
        let first = std::fs::read(&self.paths[3]).unwrap();
        success(self.run(level, true));
        assert_eq!(self.outputs(), plain);
        assert_eq!(
            std::fs::read(&self.paths[3]).unwrap(),
            first,
            "unstable graph/journal"
        );
        println!("D2_BASELINE_OK {level}");
        self.wire()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        assert_eq!(self.dir.parent(), Some(std::env::temp_dir().as_path()));
        std::fs::remove_dir_all(&self.dir).unwrap();
    }
}
fn success(out: Output) {
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(out.stdout.is_empty());
}
fn ancestry(w: &Value, start: &str) -> BTreeSet<String> {
    fn visit(w: &Value, id: &str, active: &mut BTreeSet<String>, seen: &mut BTreeSet<String>) {
        assert!(!active.contains(id), "cycle");
        if !seen.insert(id.to_owned()) {
            return;
        }
        let entry = w["entries"].get(id).expect("no dangling parents");
        active.insert(id.to_owned());
        for p in entry["parent_ids"].as_array().unwrap() {
            visit(w, p.as_str().unwrap(), active, seen);
        }
        active.remove(id);
    }
    let mut seen = BTreeSet::new();
    visit(w, start, &mut BTreeSet::new(), &mut seen);
    seen
}
fn root<'a>(w: &'a Value, input: &Path, source: &str) -> &'a str {
    let roots: Vec<_> = w["entries"]
        .as_object()
        .unwrap()
        .iter()
        .filter(|(_, e)| {
            e["origin"]["source"] == "input_file"
                && e["origin"]["location"] == input.to_string_lossy().as_ref()
        })
        .collect();
    assert_eq!(roots.len(), 1);
    let (id, entry) = roots[0];
    assert_eq!(entry["parent_ids"], json!([]));
    assert_eq!(
        entry["origin"]["meta"][NS],
        json!({"schema":"1","role":"input", "buffer_domain":"read_to_string_utf8","encoding":"utf8","grammar":"javascript_standard","edition":"es2025", "sha256":coding_adventures_sha256::sha256_hex(source.as_bytes()),"byte_length":source.len().to_string()})
    );
    id
}
fn pos(p: Pos) -> Value {
    json!({"byte_offset":p[0].to_string(),"scalar_offset":p[1].to_string(),"utf16_offset":p[2].to_string(),"line":p[3].to_string(),"scalar_column":p[4].to_string(),"utf16_column":p[5].to_string()})
}
fn ascii(a: usize, b: usize) -> Span {
    [[a, a, a, 0, a, a], [b, b, b, 0, b, b]]
}
fn inventory<'a>(w: &'a Value, root: &str) -> Vec<&'a str> {
    let events: Vec<_> = w["entries"][root]["contributions"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["source"] == "source.inventory" && c["tag"] == "finalized")
        .collect();
    assert_eq!(events.len(), 1);
    let descriptor = &events[0]["meta"][NS];
    assert_eq!(descriptor["status"], "available");
    let ids: Vec<_> = descriptor["piece_ids"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert_eq!(ids.len(), ids.iter().collect::<BTreeSet<_>>().len());
    ids
}
fn physical(w: &Value, root: &str, span: Span) -> String {
    let matches: Vec<_> = inventory(w, root)
        .into_iter()
        .filter(|id| {
            let d = &w["entries"][*id]["origin"]["meta"][NS];
            d["kind"] == "token" && d["start"] == pos(span[0]) && d["end"] == pos(span[1])
        })
        .collect();
    assert_eq!(
        matches.len(),
        1,
        "exact independently authored physical span"
    );
    let id = matches[0];
    let entry = &w["entries"][id];
    assert_eq!(entry["parent_ids"], json!([root]));
    assert_eq!(entry["origin"]["source"], "lexer_token");
    assert_eq!(entry["origin"]["meta"][NS]["input_id"], root);
    id.to_owned()
}
fn folds(w: &Value, after: &str) -> Vec<String> {
    w["entries"]
        .as_object()
        .unwrap()
        .iter()
        .filter(|(id, e)| {
            e["contributions"].as_array().unwrap().iter().any(|c| {
                c["source"] == "constant-fold"
                    && c["tag"] == "folded"
                    && c["meta"]["after"] == after
                    && c["meta"]["new_cv"] == id.as_str()
            })
        })
        .map(|(id, _)| id.clone())
        .collect()
}
fn single_fold(w: &Value, after: &str) -> String {
    let found = folds(w, after);
    assert_eq!(found.len(), 1);
    found[0].clone()
}
fn operands(w: &Value, result: &str, root: &str, expected: &[Span]) -> Vec<String> {
    let expected_ids: Vec<_> = expected.iter().map(|s| physical(w, root, *s)).collect();
    let reached = ancestry(w, result);
    assert!(reached.contains(root), "fold must reach actual input root");
    let actual_roots: BTreeSet<_> = reached
        .iter()
        .filter(|id| w["entries"][*id]["origin"]["source"] == "input_file")
        .cloned()
        .collect();
    assert_eq!(
        actual_roots,
        BTreeSet::from([root.to_owned()]),
        "cross-input contamination"
    );
    let actual: BTreeSet<_> = reached
        .iter()
        .filter(|id| {
            let origin = &w["entries"][*id]["origin"];
            origin["source"] == "lexer_token"
                || origin["source"] == "lexer_trivia"
                || origin["meta"][NS]["role"] == "piece"
        })
        .cloned()
        .collect();
    assert_eq!(
        actual,
        expected_ids.iter().cloned().collect(),
        "exact operand IDs, excluding unrelated spelling/trivia/EOF"
    );
    let inv = inventory(w, root);
    for id in &actual {
        assert!(inv.contains(&id.as_str()));
    }
    // A private compiler capture must never mint a second rootless lexer set.
    let input_path = w["entries"][root]["origin"]["location"].as_str().unwrap();
    assert!(!w["entries"]
        .as_object()
        .unwrap()
        .values()
        .any(|e| e["origin"]["source"] == input_path));
    expected_ids
}
fn merge_parents(w: &Value, result: &str, expected: &[String]) {
    let entry = &w["entries"][result];
    let parents = entry["parent_ids"].as_array().unwrap();
    assert_eq!(parents.len(), 1);
    let parent = parents[0].as_str().unwrap();
    assert_eq!(
        w["entries"][parent]["parent_ids"],
        json!(expected),
        "exact immediate ordered operand merge"
    );
    let events: Vec<_> = entry["contributions"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["source"] == "constant-fold" && c["tag"] == "folded")
        .collect();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["meta"]["parent_cv"], parent);
}

#[test]
fn d2_repeated_spelling_uses_two_exact_ingestion_operands() {
    for level in ["SIMPLE", "ADVANCED"] {
        let source = "report(1+1);report(1);";
        let f = Fixture::new(&[source]);
        let w = f.compile(level);
        let root = root(&w, &f.inputs[0], source);
        let result = single_fold(&w, "2");
        let ids = operands(&w, &result, root, &[ascii(7, 8), ascii(9, 10)]);
        merge_parents(&w, &result, &ids);
        println!("D2_LINEAGE_OK repeated {level}");
    }
}
#[test]
fn d2_nested_fold_keeps_exact_intermediate_and_ingestion_ids() {
    let source = "report(1+2*3);";
    let f = Fixture::new(&[source]);
    let w = f.compile("SIMPLE");
    let root = root(&w, &f.inputs[0], source);
    let outer = single_fold(&w, "7");
    let inner = single_fold(&w, "6");
    let ids = operands(
        &w,
        &outer,
        root,
        &[ascii(7, 8), ascii(9, 10), ascii(11, 12)],
    );
    merge_parents(&w, &outer, &[ids[0].clone(), inner.clone()]);
    let children = operands(&w, &inner, root, &[ascii(9, 10), ascii(11, 12)]);
    merge_parents(&w, &inner, &children);
    println!("D2_LINEAGE_OK nested");
}
#[test]
fn d2_asi_preserves_crlf_astral_prefix_and_original_leaf_ids() {
    let source = "/*😀*/\r\nreport(1+1)";
    let expected = [
        [[17, 14, 15, 1, 7, 7], [18, 15, 16, 1, 8, 8]],
        [[19, 16, 17, 1, 9, 9], [20, 17, 18, 1, 10, 10]],
    ];
    for level in ["SIMPLE", "ADVANCED"] {
        let f = Fixture::new(&[source]);
        let w = f.compile(level);
        let root = root(&w, &f.inputs[0], source);
        let result = single_fold(&w, "2");
        let ids = operands(&w, &result, root, &expected);
        merge_parents(&w, &result, &ids);
        println!("D2_LINEAGE_OK asi {level}");
    }
}
#[test]
fn d2_string_and_unary_folds_reach_raw_input_pieces() {
    for (source, after, expected) in [
        (
            "report(\"😀\".length);",
            "2",
            [[7, 7, 7, 0, 7, 7], [13, 10, 11, 0, 10, 11]],
        ),
        ("report(-2);", "-2", ascii(8, 9)),
    ] {
        let f = Fixture::new(&[source]);
        let w = f.compile("SIMPLE");
        let root = root(&w, &f.inputs[0], source);
        let result = single_fold(&w, after);
        let ids = operands(&w, &result, root, &[expected]);
        assert_eq!(
            w["entries"][result]["parent_ids"],
            json!(ids),
            "single-operand direct derivation"
        );
        println!("D2_LINEAGE_OK single {after}");
    }
}
#[test]
fn d2_identical_inputs_have_disjoint_fold_roots_and_operand_ids() {
    let source = "report(1+1);";
    let f = Fixture::new(&[source, source]);
    let w = f.compile("SIMPLE");
    let results = folds(&w, "2");
    assert_eq!(results.len(), 2);
    let mut prior = BTreeSet::new();
    for input in &f.inputs {
        let root = root(&w, input, source);
        let result: Vec<_> = results
            .iter()
            .filter(|id| ancestry(&w, id).contains(root))
            .collect();
        assert_eq!(result.len(), 1);
        let ids = operands(&w, result[0], root, &[ascii(7, 8), ascii(9, 10)]);
        assert!(ids.iter().all(|id| prior.insert(id.clone())));
    }
    println!("D2_LINEAGE_OK two-input");
}
#[test]
fn d2_typed_lex_and_parse_failures_preserve_every_destination() {
    for source in ["\0", "report(1+);"] {
        for level in ["SIMPLE", "ADVANCED"] {
            let f = Fixture::new(&[source]);
            for path in &f.paths {
                std::fs::write(path, b"sentinel").unwrap();
            }
            let out = f.run(level, true);
            assert_ne!(out.status.code(), Some(0));
            for path in &f.paths {
                assert_eq!(std::fs::read(path).unwrap(), b"sentinel");
            }
            println!("D2_REFUSAL_OK {level}");
        }
    }
}
