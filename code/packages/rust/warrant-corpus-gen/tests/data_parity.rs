//! The S11b acceptance test: the generator lowers, types and costs `ir.data` exactly as the
//! independent oracle does, byte for byte (VER04r section 13, slice S11b; sections 16.14 and 16.16).
//!
//! `python/warrant-oracle` committed, in slice S11a, a synthetic document that uses every form of
//! Appendix A.2 (`data_forms_v1.warrant`: records, variants with and without payload, `Option` as a real
//! type, the four tags `some none ctor field`, `match` on any scrutinee with the four data patterns) and
//! two dumps of what the oracle computes from it, in `tests/fixtures/data-forms-v1/`:
//!
//! | dump | schema | what this test reproduces it from |
//! |---|---|---|
//! | `.oracle-lowering.json` | `warrant-lowering-parity-v1` | `lower_document` and `view::oracle_view_v1`, **including the conditional `types` object** (R59: records and variants only, keys sorted as canonical JSON sorts them) |
//! | `.oracle-refusals.json` | `warrant-refusals-parity-v1` | 121 documents the oracle refuses: each is refused by the generator with the generator code of its row, in the order of R35 and R53 |
//!
//! The files are read from the oracle's directory (no copy), so an edit to either one reruns this test
//! (the build tool's repository-input row `rust/warrant-corpus-gen`).  There is no evaluation, values,
//! stream, TFV or faults dump: both tools lower, type and cost `ir.data` here and still refuse to evaluate
//! it (`level ir.data`, R37), which the last tests pin on the generator's side: `generate` stays a
//! fail-closed stub for a document that needs it.
//!
//! The negative controls at the bottom show the comparisons can fail.

mod support;

use coding_adventures_warrant_corpus_gen::error::{code, GenError};
use coding_adventures_warrant_corpus_gen::ir::Ir;
use coding_adventures_warrant_corpus_gen::lower::lower_document;
use coding_adventures_warrant_corpus_gen::view::oracle_view_v1;
use support::minijson::{parse, J};
use support::parity::first_difference;
use support::*;

const DIR: &str = "../../../python/warrant-oracle/tests/fixtures/data-forms-v1";

const WARRANT: &str = include_str!(
    "../../../python/warrant-oracle/tests/fixtures/data-forms-v1/data_forms_v1.warrant"
);
const LOWERING: &str = include_str!(
    "../../../python/warrant-oracle/tests/fixtures/data-forms-v1/data_forms_v1.oracle-lowering.json"
);
const REFUSALS: &str = include_str!(
    "../../../python/warrant-oracle/tests/fixtures/data-forms-v1/data_forms_v1.oracle-refusals.json"
);

/// The bound the oracle's `dump-lowering` costs under by default (`MAX_IR_COST`).
const LIMIT: u64 = 2_000;

// ---- the lowering dump -------------------------------------------------------------------------

fn lowering_text(src: &str) -> String {
    with_file(src, |f| {
        let l = lower_document(f).expect("lowers");
        oracle_view_v1(&l, LIMIT)
            .expect("view")
            .render()
            .expect("render")
    })
}

#[test]
fn the_data_forms_document_lowers_to_the_oracle_dump_byte_for_byte() {
    let text = lowering_text(WARRANT);
    assert!(text == LOWERING, "{}", first_difference(&text, LOWERING));
}

#[test]
fn the_dumps_are_canonical_json_of_their_schemas() {
    for (name, d, schema) in [
        ("lowering", LOWERING, "warrant-lowering-parity-v1"),
        ("refusals", REFUSALS, "warrant-refusals-parity-v1"),
    ] {
        assert!(!d.contains('\r'), "{name}");
        assert!(d.ends_with("}\n") && !d.ends_with("\n\n"), "{name}");
        assert!(d.is_ascii(), "{name}");
        assert!(d.contains(&format!("\"schema\": \"{schema}\"")), "{name}");
    }
    assert!(DIR.ends_with("data-forms-v1"));
}

#[test]
fn the_document_really_has_every_data_form_and_every_data_pattern() {
    // not a vacuous parity: the lowered document is full of the new nodes and patterns
    let dump = parse(LOWERING).expect("the dump parses");
    assert_eq!(dump.get("types").keys().len(), 7, "seven records and variants");
    // (the dump's README counts 86 models, 88 operations and 94 claims; the committed dump holds these)
    assert_eq!(dump.get("models").keys().len(), 95);
    assert_eq!(dump.get("ops").keys().len(), 97);
    assert_eq!(dump.get("suites").a().len(), 103);
    assert_eq!(dump.get("examples").a().len(), 11);
    let text = lowering_text(WARRANT);
    for (needle, at_least) in [
        (r#""some""#, 50),
        (r#""none""#, 30),
        (r#""ctor""#, 60),
        (r#""field""#, 10),
        (r#""bool""#, 15),
        (r#""match""#, 35),
        (r#""ir.data""#, 150),
    ] {
        assert!(text.matches(needle).count() >= at_least, "{needle}");
    }
    let mut counts = [0usize; 4];
    with_file(WARRANT, |f| {
        let l = lower_document(f).unwrap();
        for m in &l.models {
            m.body.walk(&mut |n| {
                counts[0] += usize::from(matches!(n, Ir::Some(_)));
                counts[1] += usize::from(matches!(n, Ir::None(_)));
                counts[2] += usize::from(matches!(n, Ir::Ctor { .. }));
                counts[3] += usize::from(matches!(n, Ir::Field(..)));
            });
        }
    });
    assert!(counts.iter().all(|c| *c > 8), "{counts:?}");
}

#[test]
fn the_types_object_is_conditional_records_and_variants_only() {
    // R59: a document that declares a record or a variant has `types` (no error type in it); one that
    // declares neither dumps byte for byte as it did before the level existed
    let dump = parse(LOWERING).unwrap();
    let types = dump.get("types");
    for (name, kind) in [
        ("Point", "record"),
        ("Wrapped", "record"),
        ("Shape", "variant"),
        ("Msg", "variant"),
    ] {
        assert_eq!(types.get(name).get("kind").s(), kind, "{name}");
    }
    assert!(!types.has("DataError"), "an error type stays under `errors`");
    assert!(dump.get("errors").has("DataError"));
    // the keys of `types` are sorted as canonical JSON sorts them (the rows are written sorted)
    let keys = types.keys();
    let mut sorted = keys.clone();
    sorted.sort_unstable();
    assert_eq!(keys, sorted);
    // a document with neither has no `types` key
    let plain = lowering_text(CT_COMPARE);
    assert!(!plain.contains("\"types\""));
}

// ---- the refusals dump ---------------------------------------------------------------------------

/// The contract boilerplate (`assume`, `labels`, `input`, `bound`) for every operation of a refusal
/// document, so the S2 checker reaches the unit under test and not a missing contract.  A parameter name
/// that repeats (the refusal rows hold a duplicate-parameter document) is declared once.
fn contracts(source: &str) -> String {
    let mut s = String::from("  assume platform:\n    none\n  assume environment:\n    none\n");
    let mut ops: Vec<(String, Vec<(String, String)>)> = Vec::new();
    for line in source.lines() {
        let Some(rest) = line.trim().strip_prefix("op ") else {
            continue;
        };
        let name = rest.split('(').next().expect("an op name").to_string();
        let params = &rest[rest.find('(').expect("(") + 1..rest.find(')').expect(")")];
        let mut ps: Vec<(String, String)> = Vec::new();
        for p in params.split(',').filter(|p| p.contains(':')) {
            let (n, t) = p.split_once(':').expect(":");
            let (n, t) = (n.trim().to_string(), t.trim().to_string());
            if !ps.iter().any(|(m, _)| *m == n) {
                ps.push((n, t));
            }
        }
        ops.push((name, ps));
    }
    for (n, ps) in &ops {
        s += &format!("  labels {n}:\n");
        for (p, t) in ps {
            if t.starts_with("Bytes") {
                s += &format!("    {p}.contents secret\n    {p}.length public\n");
            } else {
                s += &format!("    {p} secret\n");
            }
        }
    }
    for (n, ps) in &ops {
        for (p, _) in ps {
            s += &format!("  input {n}.{p}:\n    hostile\n    ensures no_panic, terminates\n");
        }
    }
    for (n, _) in &ops {
        s += &format!("  bound {n}:\n    steps <= 64\n    heap <= 0\n    depth <= 8\n");
    }
    s
}

fn refused(source: &str) -> GenError {
    let src = format!("{source}{}", contracts(source));
    with_file(&src, |f| {
        lower_document(f).expect_err("the document is refused")
    })
}

fn refusals() -> Vec<J> {
    let dump = parse(REFUSALS).expect("the refusals dump parses");
    assert_eq!(dump.get("schema").s(), "warrant-refusals-parity-v1");
    dump.get("refusals").a().to_vec()
}

#[test]
fn every_refused_document_has_the_generator_code_of_its_row_in_the_order_of_r35() {
    let (mut level, mut ir, mut limit, mut typing) = (0, 0, 0, 0);
    let mut wrong: Vec<String> = Vec::new();
    for r in refusals() {
        let (name, source) = (r.get("name").s(), r.get("source").s());
        let e = refused(source);
        let ok = match r.get("generator_code").s() {
            // a reserved level or feature (R27, R28): the same name as the oracle
            "level" => {
                level += 1;
                assert_eq!(r.get("oracle_code").s(), "level", "{name}");
                e.code == code::LEVEL
                    && e.level == Some(leak(r.get("oracle_level").s()))
                    && e.message == r.get("message").s()
            }
            // a corpus error: the same sentence as the oracle, in the oracle's order
            "ir" => {
                ir += 1;
                assert_eq!(r.get("oracle_code").s(), "corpus", "{name}");
                e.code == code::IR && e.message == r.get("message").s()
            }
            // a corpus limit (R34, R56)
            "limit" => {
                limit += 1;
                assert_eq!(r.get("oracle_code").s(), "limit", "{name}");
                e.code == code::LIMIT && e.message == r.get("message").s()
            }
            // R13 (a): the generator's own typing error (its checker or its typer), pinned to `check`: never a level,
            // a corpus error or a limit, and never an acceptance
            "" => {
                typing += 1;
                assert_eq!(r.get("oracle_code").s(), "type", "{name}");
                e.code == code::CHECK
            }
            other => panic!("{name}: an unknown generator code {other}"),
        };
        if !ok {
            wrong.push(format!("{name}: {e} (oracle {})", r.get("message").s()));
        }
    }
    assert!(wrong.is_empty(), "{} rows differ:\n{}", wrong.len(), wrong.join("\n"));
    assert_eq!(level + ir + limit + typing, 121);
    // 22 `ir.list` + 10 `ir.seq` + 4 `fixed-cast` + 3 `err-value`, 28 corpus errors, 4 limits, 50 typing errors
    assert_eq!((level, ir, limit, typing), (39, 28, 4, 50));
}

/// A `&'static str` for a level name read from the dump (the error carries `&'static str`).
fn leak(s: &str) -> &'static str {
    match s {
        "ir.list" => "ir.list",
        "ir.seq" => "ir.seq",
        "fixed-cast" => "fixed-cast",
        "err-value" => "err-value",
        other => panic!("a level name this tool does not know: {other}"),
    }
}

// ---- since S11c-2 `generate` no longer refuses ir.data (R37 lifted) ---------------------------------------

#[test]
fn generate_no_longer_refuses_the_data_forms_document_naming_ir_data() {
    // S11b lowered, typed and costed the level and kept `generate` fail-closed; S11c-2 evaluates it.  The
    // data-forms document is a *lowering* fixture (some of its claims have no enumeration the resolver can make
    // under the default budget), so it is refused by the resolver, never for the level.
    use coding_adventures_warrant_corpus_gen::{generate, GenInput};
    let r = with_file(WARRANT, |f| {
        generate(&GenInput {
            file: f,
            spec_path: "data_forms_v1.warrant",
            spec_bytes: WARRANT.as_bytes(),
            profile_bytes: b"",
            binding_faults: None,
        })
    });
    // pinned: the refusal is the resolver's (W0701 at the first claim it cannot enumerate), not a level
    let e = r.expect_err("the lowering fixture is not a generation fixture");
    assert_eq!((e.code, e.level, e.at.as_str()), (code::W0701, None, "c_opt_widen_eq"), "{e}");
    assert!(e.message.contains("binder domains cannot be resolved"), "{e}");
}

// ---- negative controls ---------------------------------------------------------------------------------

#[test]
fn a_mis_lowered_constructor_argument_loses_lowering_parity() {
    // swap the two arguments of `mk_point`: a one-node difference in one model
    let mutated = WARRANT.replacen("model mk_point(a: U8, b: U8) -> Point = Point(a, b)", "model mk_point(a: U8, b: U8) -> Point = Point(b, a)", 1);
    assert_ne!(mutated, WARRANT);
    assert_ne!(lowering_text(&mutated), LOWERING);
}

#[test]
fn a_mis_lowered_pattern_loses_lowering_parity() {
    let mutated = WARRANT.replacen(
        "model m_opt_wild(o: Option<U8>) -> Bool = match o with Some(_) -> true | None -> false",
        "model m_opt_wild(o: Option<U8>) -> Bool = match o with Some(_) -> false | None -> false",
        1,
    );
    assert_ne!(mutated, WARRANT);
    assert_ne!(lowering_text(&mutated), LOWERING);
}

#[test]
fn a_dropped_requires_or_a_dropped_types_row_loses_parity() {
    let text = lowering_text(WARRANT);
    let broken = text.replacen("\"ir.data\"", "\"ir.core\"", 1);
    assert_ne!(broken, LOWERING);
    // the conditional `types` object is part of the bytes (R59): removing one field loses parity
    let mutated = WARRANT.replacen("record Point(x: U8, y: U8)", "record Point(x: U8, y: U8, z: U8)", 1);
    assert_ne!(mutated, WARRANT);
    // (the other units no longer type: the document is refused, which is no parity either)
    let text = with_file(&mutated, |f| {
        lower_document(f).map(|l| oracle_view_v1(&l, LIMIT).unwrap().render().unwrap())
    });
    if let Ok(text) = text {
        assert_ne!(text, LOWERING);
    }
}

#[test]
fn the_refusal_comparison_can_fail() {
    // a document the oracle refuses is not accepted by the generator, and the codes of two rows differ:
    // the check is not satisfied by "any refusal"
    let rows = refusals();
    let level_row = rows.iter().find(|r| r.get("generator_code").s() == "level").unwrap();
    let ir_row = rows.iter().find(|r| r.get("generator_code").s() == "ir").unwrap();
    let (a, b) = (refused(level_row.get("source").s()), refused(ir_row.get("source").s()));
    assert_ne!(a.code, b.code);
    assert!(a.is_level() && !b.is_level());
    // an accepted document is not a refusal row: the oracle's own document lowers
    assert!(with_file(WARRANT, |f| lower_document(f).is_ok()));
}

