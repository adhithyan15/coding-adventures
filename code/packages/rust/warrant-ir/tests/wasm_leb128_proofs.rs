//! # wasm-leb128: Warrant `bound` rows -> proof status (VER04h section 7)
//!
//! `code/specs/wasm-leb128.warrant` labels every operand `public` (LEB128
//! encodes the public structure of a WebAssembly module), so VER04h asks for no
//! constant-time obligation.  It does declare `steps`, `depth` and, for the two
//! encoders, `heap` rows, so it wants a step-bound obligation per operation,
//! discharged on the *real lowered* `code/packages/rust/wasm-leb128/src/lib.rs`.
//! This target tries exactly that: it feeds the real source to the Rust-subset
//! front end and the WIR lowering.
//!
//! It is a copy of `gf256_proofs.rs` with the package constants changed and one
//! addition (the `heap` note below), deliberately not a shared, parameterised
//! target: the gf256 target and its checked-in manifest stay untouched.
//!
//! The front end does not accept the file.  What it refuses is recorded
//! verbatim (the front end's own message, position and the offending source
//! line), together with a per-function probe that says which exports lower on
//! their own and which construct stops each of the others.  Every steps-bound
//! obligation is then written to `proof-manifest.json` with label `not-live` and
//! verdict `not-live`, never `proved`, never omitted.  `heap`/`depth` rows are
//! `unchecked` (no prover tracks them).  The encoders' `heap` rows are
//! nevertheless *observed*: in a v1 document every instance of an operation
//! with a `heap` row runs under the generated harness's allocation observer
//! (VER04j section 6.3), and the test manifest records `max_observed_heap`; the
//! note says so rather than leaving "unchecked" to imply nothing ran.
//!
//! The `lowering-blocker` test passes only while the real source is refused:
//! the day the front end accepts it, it fails and says the obligations can now
//! be implemented, instead of leaving a stale "not-live" next to a source that
//! could be proved.
//!
//! With `WARRANT_PROOF_OUT=<dir>` it writes `<dir>/proof-manifest.json`
//! (schema `warrant-proof-evidence-v1`, no certificates).

use std::collections::BTreeMap;
use std::sync::Mutex;

use coding_adventures_sha256::sha256_hex;
use coding_adventures_test_bootstrap::manifest::{self, Json};
use coding_adventures_test_bootstrap::run::{self, Outcome, Suite};
use coding_adventures_test_bootstrap::Verdict;
use coding_adventures_warrant_grammar::binding::parse_evidence_bindings;
use coding_adventures_warrant_grammar::layout::{capture_layout, LayoutLimits};
use coding_adventures_warrant_grammar::parser::{parse_document, Declaration, ParserLimits};
use coding_adventures_warrant_grammar::source::{SourceDocument, SourceLimits};
use coding_adventures_warrant_ir::rust_subset::parse_rust_file;
use coding_adventures_warrant_ir::lower_source_file;

const SPEC: &str = include_str!("../../../../specs/wasm-leb128.warrant");
const SOURCE: &str = include_str!("../../wasm-leb128/src/lib.rs");
const BINDINGS: &str = include_str!("../../wasm-leb128/evidence.bindings");
const SPEC_PATH: &str = "code/specs/wasm-leb128.warrant";
const SOURCE_PATH: &str = "code/packages/rust/wasm-leb128/src/lib.rs";
const MODULE: &str = "wasm_leb128";

/// `wasm-leb128/src`, whose every `*.rs` file the manifest's `source_sha256`
/// covers (framing: `manifest::source_tree_framing`, shared with the test
/// manifest and the report checker).
fn package_src_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../wasm-leb128/src")
}

#[derive(Clone)]
struct Obligation {
    id: String,
    op: String,
    function: String,
    kind: &'static str,
    label: &'static str,
    note: String,
}

static OBLIGATIONS: Mutex<Vec<Obligation>> = Mutex::new(Vec::new());

/// What the front end says about the whole real file: `Ok(())` if it lowers,
/// otherwise the error with its position and source line.
fn whole_file_blocker() -> Result<(), String> {
    let file = parse_rust_file(SOURCE).map_err(|e| {
        let line = SOURCE.lines().nth(e.span.line.saturating_sub(1)).unwrap_or("").trim();
        format!(
            "parse refused at src/lib.rs:{}:{}: {} (line: `{}`)",
            e.span.line, e.span.col, e.message, line
        )
    })?;
    lower_source_file(&file, &[]).map(|_| ()).map_err(|e| format!("lowering refused: {}", e.message))
}

/// Source text of `pub fn <name>(...) { ... }` (brace matched).
fn function_text(name: &str) -> Option<String> {
    let start = SOURCE.find(&format!("pub fn {name}("))?;
    let rest = &SOURCE[start..];
    let open = rest.find('{')?;
    let mut depth = 0usize;
    for (i, c) in rest[open..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(rest[..open + i + 1].to_string());
                }
            }
            _ => {}
        }
    }
    None
}

/// Does this export lower when presented alone?  The answer is a note, not a
/// proof: the obligation is only ever discharged on the whole real file.
fn probe(function: &str) -> String {
    let Some(text) = function_text(function) else {
        return format!("`{function}` not found in src/lib.rs");
    };
    match parse_rust_file(&text) {
        Err(e) => format!("alone: parse refused ({})", e.message),
        Ok(file) => match lower_source_file(&file, &[]) {
            Ok(_) => "alone: lowers (loop-free, table-free body)".to_string(),
            Err(e) => format!("alone: lowering refused ({})", e.message),
        },
    }
}

fn record(o: Obligation) {
    OBLIGATIONS.lock().unwrap().push(o);
}

fn write_outputs(_outcomes: &[Outcome]) {
    let Some(dir) = manifest::output_path("WARRANT_PROOF_OUT") else {
        return;
    };
    let mut list = OBLIGATIONS.lock().unwrap().clone();
    list.sort_by(|a, b| a.id.cmp(&b.id));
    let proofs: Vec<Json> = list
        .into_iter()
        .map(|o| {
            Json::obj([
                ("id", Json::str(o.id)),
                ("op", Json::str(o.op)),
                ("function", Json::str(o.function)),
                ("kind", Json::str(o.kind)),
                ("label", Json::str(o.label)),
                ("verdict", Json::str(o.label)),
                ("max_len", Json::Num(0)),
                ("runs", Json::Num(0)),
                ("certificates", Json::Arr(Vec::new())),
                ("note", Json::str(o.note)),
            ])
        })
        .collect();
    let doc = Json::obj([
        ("schema", Json::str("warrant-proof-evidence-v1")),
        ("module", Json::str(MODULE)),
        ("spec", Json::str(SPEC_PATH)),
        ("spec_sha256", Json::str(sha256_hex(SPEC.as_bytes()))),
        ("source", Json::str(SOURCE_PATH)),
        (
            "source_sha256",
            Json::str(sha256_hex(
                &manifest::source_tree_framing(&package_src_dir()).expect("bind proofs to wasm-leb128/src"),
            )),
        ),
        ("max_len", Json::Num(0)),
        ("proofs", Json::Arr(proofs)),
    ]);
    manifest::write(&dir.join("proof-manifest.json"), &doc).expect("write proof manifest");
}

fn main() {
    let bindings = parse_evidence_bindings(BINDINGS).expect("evidence.bindings parses");
    let doc = SourceDocument::admit(SPEC.as_bytes(), SourceLimits::default()).expect("admit");
    let layout = capture_layout(&doc, LayoutLimits::default()).expect("layout");
    let file = parse_document(&layout, &doc, ParserLimits::default()).expect("parse");

    let blocker = whole_file_blocker();
    let blocker_text = match &blocker {
        Err(m) => m.clone(),
        Ok(()) => String::new(),
    };

    // op -> (has secret label, has steps/heap/depth rows)
    let mut rows: BTreeMap<String, (bool, Vec<String>)> = BTreeMap::new();
    for d in &file.declarations {
        match d {
            Declaration::Op(o) => {
                rows.entry(o.name.to_string()).or_default();
            }
            Declaration::Labels(l) => {
                if l.rows.iter().any(|r| r.is_secret) {
                    rows.entry(l.name.to_string()).or_default().0 = true;
                }
            }
            Declaration::Bound(b) => {
                let e = rows.entry(b.name.to_string()).or_default();
                for r in &b.rows {
                    e.1.push(r.name.to_string());
                }
            }
            _ => {}
        }
    }
    // One note for every obligation: the whole-file refusal, then what each
    // export does when lowered alone (so the report quotes a single, complete
    // blocker rather than one near-duplicate per operation).
    let probes: Vec<String> = rows
        .keys()
        .map(|op| {
            let function = &bindings.manifest.bindings[op].export;
            format!("{function}: {}", probe(function))
        })
        .collect();
    let note = format!("{blocker_text}; per export, {}", probes.join("; "));
    for (op, (has_secret, bounds)) in &rows {
        let function = bindings
            .manifest
            .bindings
            .get(op)
            .map(|b| b.export.clone())
            .expect("every op is bound");
        let status = if blocker.is_err() { "not-live" } else { "not-run" };
        if *has_secret {
            record(Obligation {
                id: format!("warrant/{MODULE}/proof/{op}/constant-time"),
                op: op.clone(),
                function: function.clone(),
                kind: "constant-time",
                label: status,
                note: note.clone(),
            });
        }
        if bounds.iter().any(|b| b == "steps") {
            record(Obligation {
                id: format!("warrant/{MODULE}/proof/{op}/steps-bound"),
                op: op.clone(),
                function: function.clone(),
                kind: "steps-bound",
                label: status,
                note: note.clone(),
            });
        }
        for (row, kind) in [("heap", "heap-bound"), ("depth", "depth-bound")] {
            if bounds.iter().any(|b| b == row) {
                // No prover tracks heap or depth, so the label is `unchecked`.
                // A v1 `heap` row is observed by the generated harness, though:
                // say where that evidence is instead of implying none exists.
                let note = if row == "heap" {
                    "no prover tracks this resource; the generated harness runs every instance of this \
                     operation under its allocation observer (VER04j section 6.3) and records \
                     max_observed_heap in evidence/test-manifest.json"
                } else {
                    "the pilot prover does not track this resource"
                };
                record(Obligation {
                    id: format!("warrant/{MODULE}/proof/{op}/{kind}"),
                    op: op.clone(),
                    function: function.clone(),
                    kind: if row == "heap" { "heap-bound" } else { "depth-bound" },
                    label: "unchecked",
                    note: note.into(),
                });
            }
        }
    }

    let mut suite = Suite::new();
    suite.add("warrant/wasm_leb128/proof/lowering-blocker", move |_| match &blocker {
        Err(_) => Verdict::Pass,
        Ok(()) => Verdict::fail(
            "corpus-mismatch",
            "the Rust-subset front end now accepts wasm-leb128/src/lib.rs: implement the steps-bound \
             obligations (see warrant_proofs.rs) instead of recording them not-live",
        ),
    });
    std::process::exit(run::main_recorded(&suite, write_outputs));
}
