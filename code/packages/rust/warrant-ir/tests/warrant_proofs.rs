//! # Warrant `labels` / `bound` rows -> proof obligations (VER04h section 7)
//!
//! This target reads `code/specs/ct-compare.warrant`, lowers the *real*
//! `ct-compare/src/lib.rs`, inlines helper calls so each public export is the
//! object proved, and discharges, for every operation:
//!
//! * a 2-safety (constant-time) obligation for every `secret` label, and
//! * a step-bound obligation for every `steps <= f(len(a))` row, evaluated at
//!   each concrete length with the Warrant model evaluator,
//!
//! over every buffer-length *shape* up to `MAX_LEN`.  Labels are therefore
//! `bounded(MAX_LEN)`, and `proved` only for the loop-free, buffer-free
//! `ct_eq_u64`.  Negative controls lower mutated sources that must be
//! `Falsified` / `Exceeded`.  `WARRANT_PROOF_FAULT=secret-branch` makes the main
//! obligations use the mutated source, which must then fail.
//!
//! With `WARRANT_PROOF_OUT=<dir>` it writes `<dir>/proof-manifest.json` and
//! `<dir>/proofs/<id>.{cnf,drat}`, where `<id>` is the SHA-256 of
//! `dimacs ++ 0x00 ++ drat` so a certificate's name can be re-derived from its
//! bytes.

use std::collections::{BTreeMap, HashMap};
use std::sync::Mutex;

use bit_blaster::bv::BitVec;
use coding_adventures_sha256::sha256_hex;
use coding_adventures_test_bootstrap::manifest::{self, Json};
use coding_adventures_test_bootstrap::run::{self, Outcome, Suite};
use coding_adventures_test_bootstrap::Verdict;
use coding_adventures_warrant_grammar::codegen::{CodegenLimits, EvalValue, Evaluator};
use coding_adventures_warrant_grammar::layout::{capture_layout, LayoutLimits};
use coding_adventures_warrant_grammar::parser::{
    parse_document, BoundDecl, Declaration, LabelsDecl, OpDecl, ParserLimits, TypeExpr,
    WarrantFile, Width,
};
use coding_adventures_warrant_grammar::source::{SourceDocument, SourceLimits};
use coding_adventures_warrant_ir::inline::inline_calls;
use coding_adventures_warrant_ir::rust_subset::parse_rust_file;
use coding_adventures_warrant_ir::sym::SymMemory;
use coding_adventures_warrant_ir::*;

const SPEC: &str = include_str!("../../../../specs/ct-compare.warrant");
const SOURCE: &str = include_str!("../../ct-compare/src/lib.rs");
const SPEC_PATH: &str = "code/specs/ct-compare.warrant";
const SOURCE_PATH: &str = "code/packages/rust/ct-compare/src/lib.rs";

/// Largest buffer length for which every length shape is proved.
const MAX_LEN: usize = 128;

/// Warrant op -> exported Rust function (same table as the evidence generator).
const EXPORTS: [(&str, &str); 4] = [
    ("eq", "ct_eq"),
    ("eq_fixed", "ct_eq_fixed"),
    ("select_bytes", "ct_select_bytes"),
    ("eq_u64", "ct_eq_u64"),
];

/// Mutations used as negative controls: (op, name, original text, replacement).
const MUTATIONS: [(&str, &str, &str, &str); 4] = [
    (
        "eq",
        "secret-branch",
        "        volatile_barrier::accumulate(&mut acc, &a[i], &b[i]);\n    }\n    volatile_barrier::read(&acc) == 0",
        "        if a[i] != b[i] {\n            acc = 1;\n        };\n    }\n    volatile_barrier::read(&acc) == 0",
    ),
    (
        "eq_fixed",
        "secret-branch",
        "        volatile_barrier::accumulate(&mut acc, &a[i], &b[i]);\n    }\n    volatile_barrier::read(&acc) == 0",
        "        if a[i] != b[i] {\n            acc = 1;\n        };\n    }\n    volatile_barrier::read(&acc) == 0",
    ),
    (
        "select_bytes",
        "secret-branch",
        "        out[i] = b[i] ^ ((a[i] ^ b[i]) & mask);",
        "        if choice {\n            out[i] = a[i];\n        } else {\n            out[i] = b[i];\n        };",
    ),
    (
        "eq_u64",
        "secret-branch",
        "    black_box(folded) == 0\n}",
        "    if diff == 0 {\n        return true;\n    };\n    black_box(folded) == 0\n}",
    ),
];

// ---------------------------------------------------------------------------
// Results collected while the suite runs
// ---------------------------------------------------------------------------

#[derive(Clone, Default)]
struct Record {
    id: String,
    op: String,
    function: String,
    kind: &'static str,
    label: String,
    max_len: u64,
    runs: u64,
    /// Shapes whose public length check traps before any secret is read.
    traps: u64,
    max_steps: u64,
    /// certificate id -> (kind, count)
    certs: BTreeMap<String, (&'static str, u64)>,
    note: String,
}

static RECORDS: Mutex<Vec<Record>> = Mutex::new(Vec::new());
static CERTS: Mutex<BTreeMap<String, (String, String)>> = Mutex::new(BTreeMap::new());

fn store_certs(rec: &mut Record, certs: &[ProofCertificate]) {
    for c in certs {
        let mut bytes = c.dimacs.clone().into_bytes();
        bytes.push(0);
        bytes.extend_from_slice(c.drat.as_bytes());
        let id = sha256_hex(&bytes);
        CERTS
            .lock()
            .unwrap()
            .insert(id.clone(), (c.dimacs.clone(), c.drat.clone()));
        let kind = if c.trivial { "trivial-miter-collapsed" } else { "miter" };
        rec.certs.entry(id).or_insert((kind, 0)).1 += 1;
    }
}

// ---------------------------------------------------------------------------
// Reading the Warrant document
// ---------------------------------------------------------------------------

struct OpInfo<'a> {
    decl: &'a OpDecl<'a>,
    labels: Option<&'a LabelsDecl<'a>>,
    bound: Option<&'a BoundDecl<'a>>,
}

fn ops<'a>(file: &'a WarrantFile<'a>) -> Vec<OpInfo<'a>> {
    let mut out = Vec::new();
    for d in &file.declarations {
        if let Declaration::Op(o) = d {
            let labels = file.declarations.iter().find_map(|x| match x {
                Declaration::Labels(l) if l.name == o.name => Some(l),
                _ => None,
            });
            let bound = file.declarations.iter().find_map(|x| match x {
                Declaration::Bound(b) if b.name == o.name => Some(b),
                _ => None,
            });
            out.push(OpInfo { decl: o, labels, bound });
        }
    }
    out
}

/// Is parameter `p`'s content (or the scalar itself) labelled `secret`?
fn is_secret(labels: Option<&LabelsDecl<'_>>, p: &str) -> bool {
    labels.is_some_and(|l| {
        l.rows.iter().any(|r| {
            let segs = &r.name.segments;
            r.is_secret
                && ((segs.len() == 1 && segs[0] == p) || (segs.len() == 2 && segs[0] == p && segs[1] == "contents"))
        })
    })
}

/// Lengths must be public: a `secret` length would make shape-splitting unsound.
fn lengths_are_public(labels: Option<&LabelsDecl<'_>>) -> bool {
    labels.is_none_or(|l| {
        !l.rows
            .iter()
            .any(|r| r.is_secret && r.name.segments.len() == 2 && r.name.segments[1] == "length")
    })
}

// ---------------------------------------------------------------------------
// Proof drivers
// ---------------------------------------------------------------------------

fn lower(source: &str, op: &str, export: &str) -> Result<Function, String> {
    let file = parse_rust_file(source).map_err(|e| format!("parse: {e:?}"))?;
    // Secret parameter names are applied by the driver itself (custom inputs),
    // so the lowering configuration only needs to know the function exists.
    let program = lower_source_file(
        &file,
        &[(export, &["a", "b", "choice"]), ("ct_eq_equal_length", &["a", "b"])],
    )
    .map_err(|e| format!("lower {op}: {e:?}"))?;
    inline_calls(&program, export).map_err(|e| format!("inline {op}: {e}"))
}

fn config(max_len: usize) -> SymbolicConfig {
    SymbolicConfig {
        max_steps: 200_000,
        max_paths: 512,
        unroll_bound: max_len + 2,
        ..SymbolicConfig::default()
    }
}

/// Every (len_a, len_b, ...) shape for a function with `ptrs` buffer params.
fn shapes(ptrs: usize, equal_only: bool) -> Vec<Vec<usize>> {
    match (ptrs, equal_only) {
        (0, _) => vec![vec![]],
        (_, true) => (0..=MAX_LEN).map(|n| vec![n; ptrs]).collect(),
        (2, false) => (0..=MAX_LEN)
            .flat_map(|a| (0..=MAX_LEN).map(move |b| vec![a, b]))
            .collect(),
        (n, _) => (0..=MAX_LEN).map(|k| vec![k; n]).collect(),
    }
}

fn ct_once(f: &Function, secret: &[bool], sizes: &[usize]) -> (CtProofVerdict, Vec<ProofCertificate>) {
    let mut prover = ConstantTimeProver::with_config(config(MAX_LEN));
    let (mut m1, mut m2) = (SymMemory::new(), SymMemory::new());
    let (mut a1, mut a2) = (Vec::new(), Vec::new());
    let mut next_size = sizes.iter();
    for ((_, ty, _), sec) in f.params.iter().zip(secret) {
        match ty {
            WirType::Ptr(_) => {
                let n = *next_size.next().expect("one size per buffer parameter");
                let (p1, o1) = m1.allocate_symbolic(n, &mut prover.aig);
                let (p2, o2) = m2.allocate_symbolic(n, &mut prover.aig);
                a1.push(SymVal::Ptr(p1, o1));
                a2.push(SymVal::Ptr(p2, o2));
            }
            WirType::Bool => {
                let b1 = BitVec::input(&mut prover.aig, 1);
                let b2 = if *sec { BitVec::input(&mut prover.aig, 1) } else { b1.clone() };
                a1.push(SymVal::Bool(b1.bits[0]));
                a2.push(SymVal::Bool(b2.bits[0]));
            }
            other => {
                let w = other.bit_width().unwrap_or(64) as usize;
                let b1 = BitVec::input(&mut prover.aig, w);
                let b2 = if *sec { BitVec::input(&mut prover.aig, w) } else { b1.clone() };
                a1.push(SymVal::Bits(b1));
                a2.push(SymVal::Bits(b2));
            }
        }
    }
    let verdict = prover.prove_with_custom_inputs(
        f, a1, m1, a2, m2, Vec::new(), HashMap::new(), HashMap::new(),
    );
    (verdict, prover.certificates)
}

fn bound_once(f: &Function, sizes: &[usize], bound: usize) -> (BoundVerdict, Vec<ProofCertificate>) {
    let mut prover = BoundProver::with_config(config(MAX_LEN));
    let mut mem = SymMemory::new();
    let mut args = Vec::new();
    let mut next_size = sizes.iter();
    for (var, ty, _) in &f.params {
        match ty {
            WirType::Ptr(_) => {
                let n = *next_size.next().expect("one size per buffer parameter");
                let (p, o) = mem.allocate_symbolic(n, &mut prover.executor.aig);
                args.push(SymVal::Ptr(p, o));
            }
            WirType::Bool => {
                let b = BitVec::input(&mut prover.executor.aig, 1);
                args.push(SymVal::Bool(b.bits[0]));
            }
            other => {
                let w = other.bit_width().unwrap_or(64) as usize;
                args.push(SymVal::Bits(prover.executor.create_input_bits(&var.0, w)));
            }
        }
    }
    let verdict = prover.prove_step_bound(f, args, mem, bound);
    (verdict, prover.certificates)
}

/// Evaluate the Warrant `steps` bound for concrete buffer lengths.
fn steps_bound(file: &WarrantFile<'_>, info: &OpInfo<'_>, sizes: &[usize]) -> Result<usize, String> {
    let row = info
        .bound
        .and_then(|b| b.rows.iter().find(|r| r.name == "steps"))
        .ok_or("no `steps` bound row")?;
    let mut env: HashMap<&str, EvalValue> = HashMap::new();
    let mut next = sizes.iter();
    for p in &info.decl.params {
        if matches!(p.param_type, TypeExpr::Bytes { .. }) {
            let n = *next.next().ok_or("shape shorter than parameters")?;
            env.insert(p.name, EvalValue::Bytes(vec![0; n]));
        }
    }
    let mut ev = Evaluator::new(file, CodegenLimits::default());
    let v = ev.eval(&row.bound, &env).map_err(|e| format!("{e}"))?;
    v.as_u64().map(|n| n as usize).ok_or_else(|| "bound is not a number".into())
}

fn ptr_count(f: &Function) -> usize {
    f.params.iter().filter(|(_, t, _)| matches!(t, WirType::Ptr(_))).count()
}

/// A shape on which the export's own `assert_eq!` on the *public* lengths
/// fails.  The real function panics before reading any byte, so there is no
/// secret-dependent trace to compare; the prover reports the trap as an error
/// rather than a path outcome, so these shapes are counted, not proved.  They
/// are covered by the `select_rejects_mismatch` claim (exhaustive/property/fuzz).
fn is_public_trap(msg: &str, sizes: &[usize]) -> bool {
    msg.contains("Execution trap: assertion failed") && sizes.windows(2).any(|w| w[0] != w[1])
}

fn failure(msg: impl Into<String>, counterexample: bool) -> Verdict {
    Verdict::fail(
        if counterexample { "property-counterexample" } else { "corpus-mismatch" },
        msg,
    )
}

fn label_for(ptrs: usize, bounded_any: bool) -> String {
    if ptrs == 0 && !bounded_any {
        "proved".into()
    } else {
        format!("bounded({MAX_LEN})")
    }
}

fn ct_obligation(
    file: &WarrantFile<'_>,
    info: &OpInfo<'_>,
    source: &str,
    export: &str,
    id: String,
    expect_falsified: bool,
    kind: &'static str,
) -> (Verdict, Record) {
    let op = info.decl.name;
    let mut rec = Record {
        id,
        op: op.into(),
        function: export.into(),
        kind,
        ..Record::default()
    };
    if !lengths_are_public(info.labels) {
        return (failure("a `secret` length is outside the shape-splitting model", false), rec);
    }
    let f = match lower(source, op, export) {
        Ok(f) => f,
        Err(e) => return (failure(e, false), rec),
    };
    let secret: Vec<bool> = f
        .params
        .iter()
        .map(|(v, _, _)| is_secret(info.labels, &v.0))
        .collect();
    if !secret.iter().any(|s| *s) {
        return (failure("operation has no secret label: vacuous", false), rec);
    }
    let equal_only = !info.decl.static_params.is_empty();
    let all_shapes = shapes(ptr_count(&f), equal_only);
    rec.max_len = MAX_LEN as u64;
    let mut bounded_any = false;
    let mut first_bad: Option<String> = None;
    for sizes in &all_shapes {
        let (verdict, certs) = ct_once(&f, &secret, sizes);
        rec.runs += 1;
        match verdict {
            CtProofVerdict::Proved { max_steps, .. } => {
                rec.max_steps = rec.max_steps.max(max_steps as u64);
                store_certs(&mut rec, &certs);
            }
            CtProofVerdict::ProvedBounded { max_steps, .. } => {
                bounded_any = true;
                rec.max_steps = rec.max_steps.max(max_steps as u64);
                store_certs(&mut rec, &certs);
            }
            CtProofVerdict::Unknown(ref m) if is_public_trap(m, sizes) => {
                rec.traps += 1;
            }
            other => {
                first_bad.get_or_insert(format!("shape {sizes:?}: {other:?}"));
                if expect_falsified && matches!(other, CtProofVerdict::Falsified { .. }) {
                    break;
                }
            }
        }
    }
    let _ = file;
    if expect_falsified {
        return match first_bad {
            Some(m) if m.contains("Falsified") => {
                rec.label = "falsified-as-expected".into();
                (Verdict::Pass, rec)
            }
            Some(m) => (failure(format!("mutation was not falsified: {m}"), false), rec),
            None => (failure("mutated source was proved constant-time", true), rec),
        };
    }
    match first_bad {
        None => {
            rec.label = label_for(ptr_count(&f), bounded_any);
            (Verdict::Pass, rec)
        }
        Some(m) => {
            rec.label = if m.contains("Falsified") { "falsified".into() } else { "unknown".into() };
            (failure(m, rec.label == "falsified"), rec)
        }
    }
}

fn steps_obligation(
    file: &WarrantFile<'_>,
    info: &OpInfo<'_>,
    export: &str,
    id: String,
) -> (Verdict, Record) {
    let op = info.decl.name;
    let mut rec = Record {
        id,
        op: op.into(),
        function: export.into(),
        kind: "steps-bound",
        ..Record::default()
    };
    let f = match lower(SOURCE, op, export) {
        Ok(f) => f,
        Err(e) => return (failure(e, false), rec),
    };
    rec.max_len = MAX_LEN as u64;
    let equal_only = !info.decl.static_params.is_empty();
    let mut bounded_any = false;
    for sizes in shapes(ptr_count(&f), equal_only) {
        let bound = match steps_bound(file, info, &sizes) {
            Ok(b) => b,
            Err(e) => return (failure(e, false), rec),
        };
        let (verdict, certs) = bound_once(&f, &sizes, bound);
        rec.runs += 1;
        match verdict {
            BoundVerdict::Proved { max_path_steps, .. } => {
                rec.max_steps = rec.max_steps.max(max_path_steps as u64);
                store_certs(&mut rec, &certs);
            }
            BoundVerdict::ProvedBounded { max_path_steps, .. } => {
                bounded_any = true;
                rec.max_steps = rec.max_steps.max(max_path_steps as u64);
                store_certs(&mut rec, &certs);
            }
            BoundVerdict::Unknown(ref m) if is_public_trap(m, &sizes) => {
                rec.traps += 1;
            }
            other => {
                rec.label = if matches!(other, BoundVerdict::Exceeded { .. }) { "exceeded" } else { "unknown" }.into();
                return (failure(format!("shape {sizes:?}: {other:?}"), matches!(other, BoundVerdict::Exceeded { .. })), rec);
            }
        }
    }
    rec.label = label_for(ptr_count(&f), bounded_any);
    (Verdict::Pass, rec)
}

/// A bound that is deliberately too small must be `Exceeded`.
fn bound_negative_control(info: &OpInfo<'_>, export: &str, id: String) -> (Verdict, Record) {
    let mut rec = Record {
        id,
        op: info.decl.name.into(),
        function: export.into(),
        kind: "negative-control",
        ..Record::default()
    };
    let f = match lower(SOURCE, info.decl.name, export) {
        Ok(f) => f,
        Err(e) => return (failure(e, false), rec),
    };
    let sizes = vec![4; ptr_count(&f)];
    let (verdict, _) = bound_once(&f, &sizes, 1);
    rec.runs = 1;
    match verdict {
        BoundVerdict::Exceeded { .. } => {
            rec.label = "falsified-as-expected".into();
            (Verdict::Pass, rec)
        }
        other => (failure(format!("bound 1 was not exceeded: {other:?}"), false), rec),
    }
}

// ---------------------------------------------------------------------------
// Suite and manifest
// ---------------------------------------------------------------------------

fn push(rec: Record) {
    RECORDS.lock().unwrap().push(rec);
}

fn write_outputs(outcomes: &[Outcome]) {
    let Some(dir) = manifest::output_path("WARRANT_PROOF_OUT") else {
        return;
    };
    if std::env::var("WARRANT_PROOF_FAULT").is_ok() {
        eprintln!("refusing to write a proof manifest from a sabotaged run");
        return;
    }
    let mut entries = Vec::new();
    let mut records = RECORDS.lock().unwrap().clone();
    records.sort_by(|a, b| a.id.cmp(&b.id));
    for r in records {
        let verdict = if r.label == "unchecked" {
            "unchecked"
        } else {
            outcomes
                .iter()
                .find(|o| o.id == r.id)
                .map(|o| if o.passed() { "pass" } else { "fail" })
                .unwrap_or("not-run")
        };
        let certs: Vec<Json> = r
            .certs
            .iter()
            .map(|(id, (kind, count))| {
                Json::obj([
                    ("id", Json::str(id.clone())),
                    ("kind", Json::str(*kind)),
                    ("count", Json::Num(*count)),
                ])
            })
            .collect();
        entries.push(Json::obj([
            ("id", Json::str(r.id)),
            ("op", Json::str(r.op)),
            ("function", Json::str(r.function)),
            ("kind", Json::str(r.kind)),
            ("label", Json::str(r.label)),
            ("verdict", Json::str(verdict)),
            ("max_len", Json::Num(r.max_len)),
            ("runs", Json::Num(r.runs)),
            ("trap_shapes", Json::Num(r.traps)),
            ("max_steps", Json::Num(r.max_steps)),
            ("certificates", Json::Arr(certs)),
            ("note", Json::str(r.note)),
        ]));
    }
    for (id, (dimacs, drat)) in CERTS.lock().unwrap().iter() {
        let write = |ext: &str, text: &str| {
            let path = dir.join("proofs").join(format!("{id}.{ext}"));
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            std::fs::write(path, text).expect("write certificate file");
        };
        write("cnf", dimacs);
        write("drat", drat);
    }
    let doc = Json::obj([
        ("schema", Json::str("warrant-proof-evidence-v1")),
        ("module", Json::str("ct_compare")),
        ("spec", Json::str(SPEC_PATH)),
        ("spec_sha256", Json::str(sha256_hex(SPEC.as_bytes()))),
        ("source", Json::str(SOURCE_PATH)),
        ("source_sha256", Json::str(sha256_hex(SOURCE.as_bytes()))),
        ("max_len", Json::Num(MAX_LEN as u64)),
        ("proofs", Json::Arr(entries)),
    ]);
    manifest::write(&dir.join("proof-manifest.json"), &doc).expect("write proof manifest");
}

fn reg(suite: &mut Suite, id: String, body: impl Fn(&str) -> (Verdict, Record) + 'static) {
    let id2 = id.clone();
    suite.add(id, move |_| {
        let (verdict, rec) = body(&id2);
        push(rec);
        verdict
    });
}

fn main() {
    // The parsed document borrows from `doc` and `layout`; leak both so the
    // suite closures can hold a `'static` reference for the life of the process.
    let doc: &'static SourceDocument =
        Box::leak(Box::new(SourceDocument::admit(SPEC.as_bytes(), SourceLimits::default()).expect("admit")));
    let layout = Box::leak(Box::new(capture_layout(doc, LayoutLimits::default()).expect("layout")));
    let file = Box::leak(Box::new(
        parse_document(layout, doc, ParserLimits::default()).expect("parse"),
    ));
    let file: &'static WarrantFile<'static> = file;
    let fault = std::env::var("WARRANT_PROOF_FAULT").ok();
    let mutated: HashMap<(String, String), String> = MUTATIONS
        .iter()
        .map(|(op, name, from, to)| {
            assert!(SOURCE.contains(from), "mutation `{name}` for {op} must apply");
            ((op.to_string(), name.to_string()), SOURCE.replacen(from, to, 1))
        })
        .collect();
    if let Some(f) = &fault {
        if f != "secret-branch" {
            eprintln!("unknown WARRANT_PROOF_FAULT {f:?}");
            std::process::exit(2);
        }
    }

    let mut suite = Suite::new();
    for (op, export) in EXPORTS {
        let info = ops(file).into_iter().find(|i| i.decl.name == op).expect("op in spec");
        let has_secret = info.labels.is_some_and(|l| l.rows.iter().any(|r| r.is_secret));
        if has_secret {
            // Under a sabotage run the main obligation uses the mutated source.
            let source: String = match (&fault, mutated.get(&(op.to_string(), "secret-branch".into()))) {
                (Some(_), Some(m)) => m.clone(),
                _ => SOURCE.to_string(),
            };
            reg(&mut suite, format!("warrant/ct_compare/proof/{op}/constant-time"), move |id| {
                let info = ops(file).into_iter().find(|i| i.decl.name == op).unwrap();
                ct_obligation(file, &info, &source, export, id.to_string(), false, "constant-time")
            });
            if let Some(m) = mutated.get(&(op.to_string(), "secret-branch".into())) {
                let m = m.clone();
                reg(&mut suite, format!("warrant/ct_compare/proofneg/{op}/secret-branch"), move |id| {
                    let info = ops(file).into_iter().find(|i| i.decl.name == op).unwrap();
                    ct_obligation(file, &info, &m, export, id.to_string(), true, "negative-control")
                });
            }
        }
        if info.bound.is_some_and(|b| b.rows.iter().any(|r| r.name == "steps")) {
            reg(&mut suite, format!("warrant/ct_compare/proof/{op}/steps-bound"), move |id| {
                let info = ops(file).into_iter().find(|i| i.decl.name == op).unwrap();
                steps_obligation(file, &info, export, id.to_string())
            });
            reg(&mut suite, format!("warrant/ct_compare/proofneg/{op}/bound-too-small"), move |id| {
                let info = ops(file).into_iter().find(|i| i.decl.name == op).unwrap();
                bound_negative_control(&info, export, id.to_string())
            });
        }
        // Rows the pilot prover cannot discharge are recorded, never implied.
        for (row, kind) in [("heap", "heap-bound"), ("depth", "depth-bound")] {
            if info.bound.is_some_and(|b| b.rows.iter().any(|r| r.name == row)) {
                push(Record {
                    id: format!("warrant/ct_compare/proof/{op}/{kind}"),
                    op: op.into(),
                    function: export.into(),
                    kind: if row == "heap" { "heap-bound" } else { "depth-bound" },
                    label: "unchecked".into(),
                    note: "the pilot prover does not track this resource".into(),
                    ..Record::default()
                });
            }
        }
    }
    let _ = Width::Literal(0);
    std::process::exit(run::main_recorded(&suite, write_outputs));
}
