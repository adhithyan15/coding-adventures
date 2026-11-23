//! Actual WIR front-end attempt for the fixed-point public exports.
//!
//! This target deliberately reports source-IR availability separately from the
//! generated Warrant model and the hand-constructed SAT add/sub circuits.  A
//! front-end refusal is recorded as `not-live`; parser acceptance alone is not
//! treated as semantic equivalence evidence.

use std::collections::BTreeMap;

use coding_adventures_sha256::sha256_hex;
use coding_adventures_test_bootstrap::manifest::{self, Json};
use coding_adventures_warrant_grammar::binding::parse_evidence_bindings;
use coding_adventures_warrant_grammar::layout::{capture_layout, LayoutLimits};
use coding_adventures_warrant_grammar::parser::{parse_document, Declaration, ParserLimits};
use coding_adventures_warrant_grammar::source::{SourceDocument, SourceLimits};
use coding_adventures_warrant_ir::lower_source_file;
use coding_adventures_warrant_ir::rust_subset::parse_rust_file;
use coding_adventures_warrant_ir::source_equivalence::{
    probe_export_from_snapshot, FrontendOutcome, MAX_CARGO_MANIFEST_BYTES,
};

const SPEC: &str = include_str!("../../../../specs/fixed-point.warrant");
const BINDINGS: &str = include_str!("../../fixed-point/evidence.bindings");
const SPEC_PATH: &str = "code/specs/fixed-point.warrant";
const SOURCE_PATH: &str = "code/packages/rust/fixed-point/src/lib.rs";
const MODULE: &str = "fixed_point_q16";

fn package_src_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../fixed-point/src")
}

fn frontend_attempt(source: &str) -> Result<(), String> {
    let file = parse_rust_file(source).map_err(|e| {
        let line = source
            .lines()
            .nth(e.span.line.saturating_sub(1))
            .unwrap_or("")
            .trim();
        format!(
            "parse refused at {}:{}: {} (line: `{}`)",
            e.span.line, e.span.col, e.message, line
        )
    })?;
    lower_source_file(&file, &[])
        .map(|_| ())
        .map_err(|e| format!("WIR lowering refused: {}", e.message))
}

fn per_export_attempt(
    source_frame: &[u8],
    cargo: &[u8],
    name: &str,
) -> Result<(bool, String), String> {
    match probe_export_from_snapshot(source_frame, cargo, name)? {
        FrontendOutcome::Refused(reason) => Ok((false, format!("isolated export attempt: {reason}"))),
        FrontendOutcome::Accepted => Err(format!(
            "source-equivalence refusal disposition is stale: frontend accepts direct export `{name}`"
        )),
    }
}

fn main() {
    let source_frame =
        manifest::source_tree_framing(&package_src_dir()).expect("frame fixed-point/src");
    let source = coding_adventures_warrant_ir::source_equivalence::crate_root_source_from_snapshot(
        &source_frame,
    )
    .expect("decode source snapshot");
    use std::io::Read as _;
    let cargo_path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../fixed-point/Cargo.toml");
    let mut cargo = Vec::new();
    std::fs::File::open(&cargo_path)
        .expect("open fixed-point Cargo.toml")
        .take((MAX_CARGO_MANIFEST_BYTES + 1) as u64)
        .read_to_end(&mut cargo)
        .expect("read bounded fixed-point Cargo.toml");
    assert!(
        cargo.len() <= MAX_CARGO_MANIFEST_BYTES,
        "fixed-point Cargo.toml exceeds source-probe limit"
    );
    let bindings = parse_evidence_bindings(BINDINGS).expect("bindings parse");
    let document =
        SourceDocument::admit(SPEC.as_bytes(), SourceLimits::default()).expect("admit spec");
    let layout = capture_layout(&document, LayoutLimits::default()).expect("capture spec layout");
    let warrant = parse_document(&layout, &document, ParserLimits::default()).expect("parse spec");

    let mut ops = BTreeMap::<String, String>::new();
    for declaration in &warrant.declarations {
        if let Declaration::Op(op) = declaration {
            let binding = bindings
                .manifest
                .bindings
                .get(op.name)
                .expect("each Warrant op has a binding");
            ops.insert(
                op.name.to_string(),
                binding
                    .export
                    .clone()
                    .expect("proof diagnostics need an export; driver ops unsupported"),
            );
        }
    }
    assert_eq!(
        ops.len(),
        10,
        "the source attempt covers every current fixed-point Warrant operation"
    );

    let whole = frontend_attempt(source);
    let attempts: BTreeMap<String, (bool, String)> = ops
        .iter()
        .map(|(op, export)| {
            (
                op.clone(),
                per_export_attempt(&source_frame, &cargo, export).expect(
                    "every proof operation must have an identity-resolved frontend refusal",
                ),
            )
        })
        .collect();
    let per_export = ops
        .iter()
        .map(|(op, export)| format!("{op} ({export}): {}", attempts[op].1))
        .collect::<Vec<_>>()
        .join("; ");
    let whole_note = match &whole {
        Err(reason) => format!("actual whole-file Rust-subset/WIR attempt refused: {reason}; per-export probes: {per_export}"),
        Ok(()) => format!("actual whole-file Rust-subset/WIR lowering succeeded; per-export probes: {per_export}"),
    };

    // This artifact is a disposition of the actual source frontend attempt,
    // never a semantic-equivalence proof.  The assurance checker requires a
    // matching `proofs_not_live` binding for each not-live operation.
    let proofs: Vec<Json> = ops
        .iter()
        .map(|(op, export)| {
            let (lowered, detail) = &attempts[op];
            // The established symbolic API proves source functions against a
            // WIR reference function, but there is no canonical Warrant-model
            // to WIR bridge for these operations yet.  A successful frontend
            // attempt is therefore `unchecked`, not a semantic pass.  A real
            // parse/lowering refusal is recorded as `not-live`.
            let verdict = if *lowered { "unchecked" } else { "not-live" };
            let note = format!("{detail}; whole-file probe: {}. No source-to-Warrant equivalence proof is claimed.",
                whole.as_ref().map(|_| "lowered successfully").unwrap_or("refused"));
            Json::obj([
                ("id", Json::str(format!("warrant/{MODULE}/source-equivalence/{op}"))),
                ("op", Json::str(op.clone())),
                ("function", Json::str(export.clone())),
                ("kind", Json::str("source-equivalence")),
                ("label", Json::str(verdict)),
                ("verdict", Json::str(verdict)),
                ("max_len", Json::Num(0)),
                ("certificates", Json::Arr(Vec::new())),
                ("note", Json::str(note)),
            ])
        })
        .collect();

    if let Some(dir) = manifest::output_path("WARRANT_PROOF_OUT") {
        let output = Json::obj([
            ("schema", Json::str("warrant-proof-evidence-v1")),
            ("module", Json::str(MODULE)),
            ("spec", Json::str(SPEC_PATH)),
            ("spec_sha256", Json::str(sha256_hex(SPEC.as_bytes()))),
            ("source", Json::str(SOURCE_PATH)),
            ("source_sha256", Json::str(sha256_hex(&source_frame))),
            ("max_len", Json::Num(0)),
            ("proofs", Json::Arr(proofs)),
        ]);
        manifest::write(&dir.join("proof-manifest.json"), &output)
            .expect("write proof disposition manifest");
    }

    println!("whole-file source-IR attempt: {whole_note}");
    println!(
        "recorded {} per-operation source-equivalence dispositions",
        ops.len()
    );
}
