//! VER33 ADL-002b: the `realizes` bindings of the Rust logic-gates package.
//!
//! `code/packages/rust/logic-gates/evidence.bindings` says, for each of the eleven
//! scalar gates, that the op realizes the gate model function of the same name
//! (`embedding=int-by-value`) and declares the panic templates that count as the
//! modelled rejection.  This file pins three things:
//!
//! 1. the binding file validates against the real documents: `logic-gates.warrant`
//!    (the ops) and `logic-gates-model.warrant` (the model, `profile algorithm`),
//!    with no W0806 or W0807;
//! 2. **the declared templates are the panic sites of `validate_bit`**, read from
//!    `gates.rs`: the one `assert!` in `validate_bit` with its format string, times the
//!    argument names each gate reaches.  For a primitive the match is exact; for a
//!    NAND-built gate the static call graph gives an upper bound (the templates must be
//!    producible), and the exact set is pinned at runtime (`nand_or` reaches only the
//!    name `a`, because `nand_not(b)` forwards to `nand_gate(b, b)`, whose first check
//!    names its argument `a`);
//! 3. nothing compiler-generated is declared, and the only other panic site in a gate
//!    body is the `.unwrap()` after the validated CMOS call, which is deliberately NOT a
//!    modelled rejection (VER33 4.8: any other trap fails `Refines`).
//!
//! The runtime half (real panics are matched by the declared templates) is
//! `logic-gates/tests/realizes_panics.rs`.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use coding_adventures_warrant_grammar::binding::{
    check_panic_template, parse_evidence_bindings, validate_realizations, ModelModule,
};
use coding_adventures_warrant_grammar::checker::{check_warrant, CheckedWarrantModule};
use coding_adventures_warrant_grammar::layout::{capture_layout, LayoutLimits};
use coding_adventures_warrant_grammar::parser::{parse_document, ParserLimits, Profile};
use coding_adventures_warrant_grammar::source::{SourceDocument, SourceLimits};

const GATES: [&str; 11] = [
    "not_gate",
    "and_gate",
    "or_gate",
    "xor_gate",
    "nand_gate",
    "nor_gate",
    "xnor_gate",
    "nand_not",
    "nand_and",
    "nand_or",
    "nand_xor",
];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(4)
        .expect("repository root")
        .to_path_buf()
}

fn read(rel: &str) -> String {
    std::fs::read_to_string(root().join(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"))
}

fn load(rel: &str) -> (Profile, CheckedWarrantModule) {
    let text = read(rel);
    let doc = SourceDocument::admit(text.as_bytes(), SourceLimits::default()).expect("admit");
    let layout = capture_layout(&doc, LayoutLimits::default()).expect("layout");
    let file = parse_document(&layout, &doc, ParserLimits::default()).expect("parse");
    let checked = check_warrant(&file).expect("check");
    (file.profile, checked)
}

const BINDINGS: &str = "code/packages/rust/logic-gates/evidence.bindings";
const GATES_RS: &str = "code/packages/rust/logic-gates/src/gates.rs";

#[test]
fn the_bindings_validate_against_the_ops_and_the_model_document() {
    let bindings =
        parse_evidence_bindings(&read(BINDINGS)).expect("bindings parse (W0806/W0807 clean)");
    let (_, ops) = load("code/specs/logic-gates.warrant");
    let model_text = read("code/specs/logic-gates-model.warrant");
    let doc = SourceDocument::admit(model_text.as_bytes(), SourceLimits::default()).expect("admit");
    let layout = capture_layout(&doc, LayoutLimits::default()).expect("layout");
    let model_file = parse_document(&layout, &doc, ParserLimits::default()).expect("parse");
    assert_eq!(model_file.profile, Profile::Algorithm);
    let model = check_warrant(&model_file).expect("check");
    let models = [ModelModule::new(&model_file, &model)];
    validate_realizations(&ops, &bindings.realizations, &models).expect("realizations validate");

    // Exactly the eleven scalar gates realize, each its own model function, by value.
    let got: BTreeSet<&str> = bindings.realizations.keys().map(String::as_str).collect();
    assert_eq!(got, GATES.iter().copied().collect::<BTreeSet<_>>());
    for (op, r) in &bindings.realizations {
        assert_eq!(r.module, "logic_gates_model", "{op}");
        assert_eq!(
            &r.model, op,
            "{op} realizes the model function of its own name"
        );
        assert_eq!(r.embedding, "int-by-value", "{op}");
        for t in &r.panic_messages {
            check_panic_template(t).unwrap_or_else(|e| panic!("{op}: {t}: {e}"));
        }
    }
}

/// The body of `fn name(...)` in `src`: from its signature to the first line that is
/// exactly `}`.
fn body<'s>(src: &'s str, name: &str) -> &'s str {
    let sig = src
        .find(&format!("fn {name}("))
        .unwrap_or_else(|| panic!("no fn {name}"));
    let rest = &src[sig..];
    let end = rest.find("\n}\n").expect("end of fn");
    &rest[..end]
}

/// The argument names a gate reaches `validate_bit` with, in order, following calls to
/// the other gates (`callee(x, ..)` makes the callee's first-argument names reachable).
fn reachable_names(src: &str, gate: &str) -> BTreeSet<String> {
    let b = body(src, gate);
    let mut out = BTreeSet::new();
    for line in b.lines() {
        if let Some(i) = line.find("validate_bit(") {
            let args = &line[i + "validate_bit(".len()..];
            let name = args.split('"').nth(1).expect("a literal name");
            out.insert(name.to_string());
        }
    }
    // Calls: the callee panics with its own names, whatever we pass it.
    for callee in GATES {
        if callee != gate && b.contains(&format!("{callee}(")) {
            out.extend(reachable_names(src, callee));
        }
    }
    out
}

#[test]
fn the_declared_templates_are_exactly_the_panic_sites_of_validate_bit() {
    let src = read(GATES_RS);

    // The one authored panic site: a single `assert!` in `validate_bit`.
    let vb = body(&src, "validate_bit");
    assert_eq!(
        vb.matches("assert!(").count(),
        1,
        "validate_bit has one assert!"
    );
    assert_eq!(vb.matches("panic!(").count(), 0);
    let fmt = vb
        .split('"')
        .nth(1)
        .expect("the assert! has a format string")
        .to_string();
    assert_eq!(fmt, "{name} must be 0 or 1, got {value}");

    let bindings = parse_evidence_bindings(&read(BINDINGS)).expect("bindings");
    let mut checked = 0;
    for gate in GATES {
        let want: BTreeSet<String> = reachable_names(&src, gate)
            .into_iter()
            .map(|n| fmt.replace("{name}", &n).replace("{value}", "{}"))
            .collect();
        let got: BTreeSet<String> = bindings.realizations[gate]
            .panic_messages
            .iter()
            .cloned()
            .collect();
        // A primitive calls `validate_bit` itself, so its templates are exactly its
        // sites.  A NAND-built gate forwards its parameters to the primitives, so the
        // static call graph is only an upper bound: it cannot see that
        // `nand_gate(a, a)` panics on the first check, naming `a`, whatever `b` is.
        // The exact set for those is pinned at runtime by
        // `logic-gates/tests/realizes_panics.rs`.
        if body(&src, gate).contains("validate_bit(") {
            assert_eq!(
                got, want,
                "{gate}: declared templates differ from its sites"
            );
        } else {
            assert!(
                got.is_subset(&want),
                "{gate}: declares a template no panic site can produce"
            );
        }
        checked += got.len();
    }
    // 8 two-argument gates declare a and b; nand_or only a; not_gate and nand_not only a.
    assert_eq!(checked, 8 * 2 + 3, "the template count changed");
}

#[test]
fn nothing_compiler_generated_is_declared_and_other_panic_sites_are_named() {
    let src = read(GATES_RS);
    let bindings = parse_evidence_bindings(&read(BINDINGS)).expect("bindings");
    let mut all: BTreeMap<&str, usize> = BTreeMap::new();
    for r in bindings.realizations.values() {
        for t in &r.panic_messages {
            assert!(t.starts_with(['a', 'b']));
            for bad in [
                "attempt to",
                "unwrap",
                "index out of bounds",
                "called `",
                "overflow",
            ] {
                assert!(!t.contains(bad), "{t}");
            }
            *all.entry(t.as_str()).or_default() += 1;
        }
    }
    assert_eq!(
        all.keys().copied().collect::<Vec<_>>(),
        ["a must be 0 or 1, got {}", "b must be 0 or 1, got {}"]
    );

    // In a primitive's body the only other panic site is `.unwrap()` after the CMOS
    // call, which cannot fire on a validated bit and is not declared.  No `panic!`,
    // `expect(`, indexing or arithmetic appears in the eleven bodies.
    for gate in GATES {
        let b = body(&src, gate);
        for forbidden in ["panic!(", "unreachable!(", ".expect(", "assert", "["] {
            assert!(
                !b.contains(forbidden),
                "{gate} contains `{forbidden}`: a panic site the bindings do not declare"
            );
        }
        let unwraps = b.matches(".unwrap()").count();
        assert!(unwraps <= 1, "{gate}: {unwraps} unwraps");
    }
}
