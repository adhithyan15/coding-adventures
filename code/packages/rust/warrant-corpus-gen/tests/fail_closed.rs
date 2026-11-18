//! Fail-closed behaviour (VER04r section 13, "Slice S3 in detail").
//!
//! The generator computes a digest for every suite it emits, so it cannot emit a suite it
//! cannot evaluate.  A document with any operation, model, claim or example beyond
//! `ir.core` is refused **as a whole**: `code == "level"`, the level named, nothing written.
//! One test per construct class, each with a control showing the same document minus the
//! construct lowers fine, so a refusal is the construct's doing and not the boilerplate's.

mod support;

use coding_adventures_warrant_corpus_gen::lower::{census, lower_document, ItemKind};
use coding_adventures_warrant_corpus_gen::profile::Profile;
use coding_adventures_warrant_corpus_gen::{analyse, code, generate, GenInput};
use support::*;

/// Lower `src` and return the refusal.
fn refusal(src: &str) -> coding_adventures_warrant_corpus_gen::GenError {
    with_file(src, |f| lower_document(f).expect_err("must be refused"))
}

fn assert_level(src: &str, level: &str, at: &str) {
    let e = refusal(src);
    assert_eq!(e.code, code::LEVEL, "{e}");
    assert_eq!(e.level, Some(level), "{e}");
    assert_eq!(e.at, at, "{e}");
    // The message is one of our sentences: it never echoes source text.
    assert!(!e.to_string().contains("synth"), "{e}");
}

const CLEAN_MODEL: &str = "  model ok(a: U8) -> U8 = a\n";

#[test]
fn the_control_document_lowers() {
    let src = doc_ops(" v1", &["op f(a: Bytes, b: Bytes) -> Bool"], CLEAN_MODEL);
    with_file(&src, |f| assert!(lower_document(f).is_ok()));
}

// ---- ir.data -----------------------------------------------------------------

#[test]
fn match_needs_ir_data() {
    let m = "  model mm(o: Bool) -> U8 = match o with true -> 1u8 | false -> 0u8\n";
    assert_level(
        &doc_ops(" v1", &["op f(a: Bytes, b: Bytes) -> Bool"], m),
        "ir.data",
        "mm",
    );
}

#[test]
fn a_tuple_projection_needs_ir_tuple() {
    // S10b: a tuple literal and `project` are `ir.tuple` (VER04r 4.1, D20), not `ir.data`.
    let m = "  model tp(a: U8) -> U8 = (a, a).0\n";
    assert_level(
        &doc_ops(" v1", &["op f(a: Bytes, b: Bytes) -> Bool"], m),
        "ir.tuple",
        "tp",
    );
}

#[test]
fn match_on_a_result_is_ir_tuple_and_on_anything_else_ir_data() {
    // D35: `match` is `ir.tuple` exactly when the scrutinee's static type is a `Result`.
    let ops = ["op f(a: Bytes, b: Bytes) -> Bool"];
    let r2 = "  error E = BAD\n  model mk(a: U8) -> Result<U8, E> = Ok(a)\n  model rr(a: U8) -> U8 = match mk(a) with Ok(x) -> x | Err(e) -> 0u8\n";
    assert_level(&doc_ops(" v1", &ops, r2), "ir.tuple", "rr");
    let o = "  model oo(o: Bool) -> U8 = match o with true -> 1u8 | false -> 0u8\n";
    assert_level(&doc_ops(" v1", &ops, o), "ir.data", "oo");
}

#[test]
fn a_list_literal_needs_ir_data() {
    let m = "  model ll(a: U8) -> Nat = len([a, a])\n";
    assert_level(
        &doc_ops(" v1", &["op f(a: Bytes, b: Bytes) -> Bool"], m),
        "ir.data",
        "ll",
    );
}

#[test]
fn constructors_and_option_built_ins_need_ir_data() {
    let m = "  model sm(a: U8) -> U8 = unwrap_or(Some(a), 0u8)\n";
    assert_level(
        &doc_ops(" v1", &["op f(a: Bytes, b: Bytes) -> Bool"], m),
        "ir.data",
        "sm",
    );
}

#[test]
fn a_tuple_type_in_a_model_signature_needs_ir_tuple() {
    let m = "  model tt(a: U8) -> (U8, U8) = (a, a)\n";
    assert_level(
        &doc_ops(" v1", &["op f(a: Bytes, b: Bytes) -> Bool"], m),
        "ir.tuple",
        "tt",
    );
}

#[test]
fn an_option_result_in_an_op_signature_needs_ir_data() {
    let src = doc_ops(
        " v1",
        &[
            "op f(a: Bytes, b: Bytes) -> Bool",
            "op o(a: Bytes, b: Bytes) -> Option<U8>",
        ],
        "",
    );
    assert_level(&src, "ir.data", "o");
}

#[test]
fn a_list_binder_needs_ir_data_and_names_the_claim() {
    let c = "  claim lb:\n    forall l: List<Bool>. f(b\"\", b\"\") == true\n";
    assert_level(
        &doc_ops(" v1", &["op f(a: Bytes, b: Bytes) -> Bool"], c),
        "ir.data",
        "lb",
    );
}

#[test]
fn record_and_variant_declarations_need_ir_data() {
    let r = "  record Span(start: U8, len: U8)\n";
    assert_level(
        &doc_ops(" v1", &["op f(a: Bytes, b: Bytes) -> Bool"], r),
        "ir.data",
        "Span",
    );
    let v = "  variant Shape = Circle(r: U8) | Empty\n";
    assert_level(
        &doc_ops(" v1", &["op f(a: Bytes, b: Bytes) -> Bool"], v),
        "ir.data",
        "Shape",
    );
}

// ---- ir.fixed ----------------------------------------------------------------

#[test]
fn integer_built_ins_need_ir_fixed() {
    for (i, body) in [
        "bit_and(a, b)",
        "bit_or(a, b)",
        "bit_xor(a, b)",
        "shr(a, 1)",
        "shl(a, 1)",
        "rotl(a, 1)",
    ]
    .iter()
    .enumerate()
    {
        let m = format!("  model fx{i}(a: U8, b: U8) -> U8 = {body}\n");
        assert_level(
            &doc_ops(" v1", &["op f(a: Bytes, b: Bytes) -> Bool"], &m),
            "ir.fixed",
            &format!("fx{i}"),
        );
    }
}

#[test]
fn width_changing_and_endian_built_ins_need_ir_fixed() {
    let m = "  model wd(a: U8) -> U64 = widen<U64>(a)\n";
    assert_level(
        &doc_ops(" v1", &["op f(a: Bytes, b: Bytes) -> Bool"], m),
        "ir.fixed",
        "wd",
    );
    let m = "  model be(a: U32) -> Bytes = be_bytes(a)\n";
    assert_level(
        &doc_ops(" v1", &["op f(a: Bytes, b: Bytes) -> Bool"], m),
        "ir.fixed",
        "be",
    );
}

#[test]
fn a_built_in_in_a_claim_names_the_claim() {
    let c = "  claim cb:\n    forall a: U8, b: U8. f(b\"\", b\"\") == (bit_and(a, b) == 0u8)\n";
    assert_level(
        &doc_ops(" v1", &["op f(a: Bytes, b: Bytes) -> Bool"], c),
        "ir.fixed",
        "cb",
    );
}

// ---- features ----------------------------------------------------------------

#[test]
fn an_inout_parameter_is_refused_naming_inout() {
    let src = doc_ops(
        " v1",
        &[
            "op f(a: Bytes, b: Bytes) -> Bool",
            "op wipe(a: inout Bytes, b: Bytes) -> Bytes",
        ],
        "",
    );
    assert_level(&src, "inout", "wipe");
}

#[test]
fn ensures_rows_are_refused_naming_ensures() {
    let src = doc_ops(
        " v1",
        &["op f(a: Bytes, b: Bytes) -> Bool"],
        "  ensures f:\n    result == true\n",
    );
    assert_level(&src, "ensures", "f");
}

#[test]
fn a_vectors_table_is_refused_naming_vectors() {
    let src = "warrant t v1\n  op f(m: Bytes) -> Bytes<4>\n  labels f:\n    m public\n  vectors a:\n    source \"a.tfv\"\n    sha256 b\"0000000000000000000000000000000000000000000000000000000000000000\"\n    rows 1\n    column msg: Bytes\n    holds f(msg) == f(msg)\n";
    assert_level(src, "vectors", "a");
}

// ---- W0701 ---------------------------------------------------------------------

#[test]
fn exists_has_no_model_ir_form() {
    let c = "  claim ex:\n    exists a: Bytes. f(a, a) == true\n";
    let e = refusal(&doc_ops("", &["op f(a: Bytes, b: Bytes) -> Bool"], c));
    assert_eq!(e.code, code::W0701);
    assert_eq!(e.at, "ex");
}

#[test]
fn a_subject_call_under_an_impure_guard_is_w0701() {
    let c =
        "  claim bad:\n    forall a: Bytes, b: Bytes. if f(a, b) then f(b, a) == true else true\n";
    let e = refusal(&doc_ops("", &["op f(a: Bytes, b: Bytes) -> Bool"], c));
    assert_eq!(e.code, code::W0701, "{e}");
    assert_eq!(e.at, "bad");
    assert!(e.message.contains("impure guard"));
    // the control: a pure guard is fine
    let ok = "  claim good:\n    forall a: Bytes, b: Bytes. if len(a) > 0 then f(b, a) == true else true\n";
    with_file(
        &doc_ops("", &["op f(a: Bytes, b: Bytes) -> Bool"], ok),
        |f| assert!(lower_document(f).is_ok()),
    );
}

#[test]
fn v0_refuses_fixed_width_arithmetic_over_a_subject_result_but_v1_does_not() {
    let op = "op u(a: Bytes, b: Bytes) -> U8";
    let c = "  claim wrapped:\n    forall a: Bytes, b: Bytes. u(a, b) + 1u8 == u(b, a)\n";
    let e = refusal(&doc_ops("", &[op], c));
    assert_eq!(e.code, code::W0701, "{e}");
    assert_eq!(e.at, "wrapped");
    assert!(e.message.contains("v0") && e.message.contains("rule 3"));
    // A v1 document wraps everywhere (VER04r 3.3 rule 5), so the same claim lowers.
    with_file(&doc_ops(" v1", &[op], c), |f| {
        assert!(lower_document(f).is_ok())
    });
}

#[test]
fn a_refinement_that_calls_the_subject_is_w0701() {
    let c = "  claim rs:\n    forall a: Bytes | f(a, a), b: Bytes. f(a, b) == true\n";
    let e = refusal(&doc_ops("", &["op f(a: Bytes, b: Bytes) -> Bool"], c));
    assert_eq!(e.code, code::W0701, "{e}");
    assert_eq!(e.at, "rs");
}

// ---- census, generate ------------------------------------------------------------

#[test]
fn census_reports_each_blocked_item_without_failing_the_document() {
    let extra = "  model ok(a: U8) -> U8 = a\n  model fx(a: U8) -> U8 = bit_not(a)\n  model dat(a: U8) -> U8 = (a, a).0\n  model uses_fx(a: U8) -> U8 = fx(a)\n  claim clean:\n    forall a: Bytes. f(a, a) == true\n  claim via_model:\n    forall a: U8. f(b\"\", b\"\") == (uses_fx(a) == 0u8)\n  claim via_data:\n    forall a: U8. f(b\"\", b\"\") == (dat(a) == 0u8)\n  example e_clean:\n    f(b\"\", b\"\") == true\n";
    let src = doc_ops(" v1", &["op f(a: Bytes, b: Bytes) -> Bool"], extra);
    let rows = with_file(&src, |f| census(f).expect("census"));
    let get = |kind, name: &str| {
        rows.iter()
            .find(|r| r.kind == kind && r.name == name)
            .unwrap_or_else(|| panic!("{name}"))
    };
    assert!(get(ItemKind::Op, "f").blocked.is_none());
    assert!(get(ItemKind::Model, "ok").blocked.is_none());
    assert_eq!(
        get(ItemKind::Model, "fx").blocked.as_ref().unwrap().level,
        Some("ir.fixed")
    );
    assert_eq!(
        get(ItemKind::Model, "dat").blocked.as_ref().unwrap().level,
        Some("ir.tuple")
    );
    // a model that calls a blocked model inherits the blocker
    assert_eq!(
        get(ItemKind::Model, "uses_fx")
            .blocked
            .as_ref()
            .unwrap()
            .level,
        Some("ir.fixed")
    );
    assert!(get(ItemKind::Claim, "clean").blocked.is_none());
    assert_eq!(
        get(ItemKind::Claim, "via_model")
            .blocked
            .as_ref()
            .unwrap()
            .level,
        Some("ir.fixed")
    );
    assert_eq!(
        get(ItemKind::Claim, "via_data")
            .blocked
            .as_ref()
            .unwrap()
            .level,
        Some("ir.tuple")
    );
    assert!(get(ItemKind::Example, "e_clean").blocked.is_none());
    // and the whole document is refused with the first blocker in declaration order
    let e = with_file(&src, |f| lower_document(f).unwrap_err());
    assert_eq!(e.level, Some("ir.fixed"));
    assert_eq!(e.at, "fx");
}

#[test]
fn census_marks_document_level_blockers_as_declarations() {
    let src = doc_ops(
        " v1",
        &["op f(a: Bytes, b: Bytes) -> Bool"],
        "  record Span(start: U8, len: U8)\n  ensures f:\n    result == true\n",
    );
    let rows = with_file(&src, |f| census(f).unwrap());
    let decls: Vec<_> = rows
        .iter()
        .filter(|r| r.kind == ItemKind::Declaration)
        .collect();
    assert_eq!(decls.len(), 2);
    assert_eq!(decls[0].blocked.as_ref().unwrap().level, Some("ir.data"));
    assert_eq!(decls[1].blocked.as_ref().unwrap().level, Some("ensures"));
}

#[test]
fn census_of_ct_compare_is_clean() {
    let rows = with_file(CT_COMPARE, |f| census(f).unwrap());
    assert!(rows.iter().all(|r| r.blocked.is_none()));
    assert_eq!(rows.iter().filter(|r| r.kind == ItemKind::Claim).count(), 9);
    assert_eq!(rows.iter().filter(|r| r.kind == ItemKind::Model).count(), 2);
    assert_eq!(rows.iter().filter(|r| r.kind == ItemKind::Op).count(), 4);
    assert_eq!(
        rows.iter().filter(|r| r.kind == ItemKind::Example).count(),
        12
    );
}

#[test]
fn a_document_the_checker_refuses_is_a_check_error_not_a_level_error() {
    // `nope` is not declared anywhere
    let src = doc_ops(
        "",
        &["op f(a: Bytes, b: Bytes) -> Bool"],
        "  claim c:\n    forall a: Bytes. f(a, a) == nope(a)\n",
    );
    let e = refusal(&src);
    assert_eq!(e.code, code::CHECK);
    assert!(!e.is_level());
}

#[test]
fn generate_still_refuses_what_is_beyond_ir_core_and_bad_profiles() {
    // ct-compare is `ir.core` all through: generate no longer fails closed for it (S3c), but a
    // bad profile is refused first and a document beyond `ir.core` is refused whole.
    with_file(CT_COMPARE, |f| {
        let e = generate(&GenInput {
            file: f,
            spec_path: "code/specs/ct-compare.warrant",
            spec_bytes: CT_COMPARE.as_bytes(),
            profile_bytes: b"chunk_size 3000\n",
            binding_faults: None,
        })
        .unwrap_err();
        assert_eq!(e.code, code::PROFILE);
        assert!(analyse(f, &Profile::default()).is_ok());
    });
    let beyond = doc_ops(
        " v1",
        &["op f(a: Bytes, b: Bytes) -> Bool"],
        "  model fx(a: U8) -> U8 = bit_not(a)\n",
    );
    with_file(&beyond, |f| {
        let e = generate(&GenInput {
            file: f,
            spec_path: "x.warrant",
            spec_bytes: beyond.as_bytes(),
            profile_bytes: b"",
            binding_faults: None,
        })
        .unwrap_err();
        assert_eq!(e.code, code::LEVEL);
        assert_eq!(e.level, Some("ir.fixed"));
    });
}

// ---- more document-level and ordering rules ----------------------------------------

#[test]
fn an_assume_row_that_assumes_something_is_refused_naming_assume() {
    // `none` is the empty block (ct-compare writes it); a row is a contract id (VER29).
    let src = "warrant s\n  op f(a: Bytes, b: Bytes) -> Bool\n  assume platform:\n    other.claim\n  assume environment:\n    none\n".to_string()
        + bytes_op_contracts(&["f"]).split_once("    none\n  assume environment:\n    none\n").unwrap().1;
    assert_level(&src, "assume", "platform");
}

#[test]
fn a_model_that_calls_a_later_model_is_w0701_with_rule_4_2_1() {
    let m = "  model m1(a: U8) -> U8 = m2(a)\n  model m2(a: U8) -> U8 = a\n";
    let e = refusal(&doc_ops("", &["op f(a: Bytes, b: Bytes) -> Bool"], m));
    assert_eq!(e.code, code::W0701);
    assert_eq!(e.at, "m1");
    assert!(e.message.contains("declared later"));
    // declared in the right order, the same pair lowers
    let ok = "  model m2(a: U8) -> U8 = a\n  model m1(a: U8) -> U8 = m2(a)\n";
    with_file(
        &doc_ops("", &["op f(a: Bytes, b: Bytes) -> Bool"], ok),
        |f| assert!(lower_document(f).is_ok()),
    );
}

#[test]
fn an_assume_environment_row_is_refused_naming_assume() {
    let src = "warrant s\n  op f(a: Bytes, b: Bytes) -> Bool\n  assume platform:\n    none\n  assume environment:\n    clock: true\n".to_string()
        + bytes_op_contracts(&["f"]).split_once("    none\n  assume environment:\n    none\n").unwrap().1;
    assert_level(&src, "assume", "environment");
}

#[test]
fn census_follows_blockers_through_records_constructors_ops_and_examples() {
    let extra = concat!(
        "  record Span(start: U8, len: U8)\n",
        "  variant Shape = Circle(r: U8) | Empty\n",
        "  model uses_span(s: Span) -> Bool = true\n",
        "  model builds(a: U8) -> Bool = Circle(a) == Empty\n",
        "  model bare_ctor(a: U8) -> Bool = Empty == Empty\n",
        "  claim via_blocked_op:\n    forall a: Bytes. o(a, a) == o(a, a)\n",
        "  claim via_blocked_model:\n    forall a: Bytes. f(a, a) == (builds(1u8))\n",
        "  example ex_fixed:\n    f(b\"\", b\"\") == (bit_not(1u8) == 0u8)\n",
    );
    let src = doc_ops(
        " v1",
        &[
            "op f(a: Bytes, b: Bytes) -> Bool",
            "op o(a: Bytes, b: Bytes) -> Option<U8>",
            "op shr_ref(a: Bytes, b: Bytes) -> Bool",
        ],
        extra,
    );
    let rows = with_file(&src, |f| census(f).expect("census"));
    let find = |kind, name: &str| {
        rows.iter()
            .find(|r| r.kind == kind && r.name == name)
            .unwrap()
    };
    let level = |kind, name: &str| find(kind, name).blocked.as_ref().and_then(|e| e.level);
    assert_eq!(level(ItemKind::Model, "uses_span"), Some("ir.data"));
    assert_eq!(level(ItemKind::Model, "builds"), Some("ir.data"));
    assert_eq!(level(ItemKind::Model, "bare_ctor"), Some("ir.data"));
    assert_eq!(level(ItemKind::Op, "o"), Some("ir.data"));
    assert_eq!(level(ItemKind::Claim, "via_blocked_op"), Some("ir.data"));
    assert_eq!(level(ItemKind::Claim, "via_blocked_model"), Some("ir.data"));
    assert_eq!(level(ItemKind::Example, "ex_fixed"), Some("ir.fixed"));
    assert!(find(ItemKind::Op, "shr_ref").blocked.is_none());
}

#[test]
fn an_op_refinement_beyond_ir_core_blocks_the_op_and_its_callers() {
    let src = doc_ops(
        " v1",
        &[
            "op f(a: Bytes, b: Bytes) -> Bool",
            "op g(a: Bytes, b: Bytes | shr(1u8, 1) == 0u8) -> Bool",
        ],
        "  claim cg:\n    forall a: Bytes, b: Bytes. g(a, b) == true\n",
    );
    let rows = with_file(&src, |f| census(f).expect("census"));
    let op = rows
        .iter()
        .find(|r| r.kind == ItemKind::Op && r.name == "g")
        .unwrap();
    assert_eq!(op.blocked.as_ref().unwrap().level, Some("ir.fixed"));
    // S10b: a claim that lowers within `ir.core` is not blocked by its callee's `admit`; the
    // oracle's `requires` does not read it either (R9 is on the unit's own tags and models).
    // The document is still refused, at the op, so `generate` stays closed.
    let claim = rows
        .iter()
        .find(|r| r.kind == ItemKind::Claim && r.name == "cg")
        .unwrap();
    assert!(claim.blocked.is_none());
    let e = with_file(&src, |f| lower_document(f).unwrap_err());
    assert_eq!((e.level, e.at.as_str()), (Some("ir.fixed"), "g"));
}

#[test]
fn an_alias_of_a_core_type_is_resolved_away_and_an_alias_of_a_tuple_is_refused() {
    let ok = doc_ops(
        " v1",
        &["op f(a: Bytes, b: Bytes) -> Bool"],
        "  model Word = Bytes<4>\n  model keep(a: Word) -> Word = a\n",
    );
    with_file(&ok, |f| {
        let l = lower_document(f).unwrap();
        // the alias is not a model, and the model's parameter is the resolved type
        assert_eq!(l.models.len(), 1);
        assert_eq!(l.models[0].params[0].1.name(), "Bytes<4>");
    });
    let tuple = doc_ops(
        " v1",
        &["op f(a: Bytes, b: Bytes) -> Bool"],
        "  model Pair = (U8, U8)\n  model uses(a: Pair) -> Bool = true\n",
    );
    assert_level(&tuple, "ir.tuple", "uses");
}

// ---- M1: the v0 guard through a model call; L4: examples; L3 ----------------------------

#[test]
fn v0_refuses_a_model_that_wraps_called_on_a_subject_result_but_v1_and_pure_calls_pass() {
    let op = "op u(a: Bytes, b: Bytes) -> U8";
    let model = "  model inc(x: U8) -> U8 = x + 1u8\n  model twice(x: U8) -> U8 = inc(inc(x))\n";
    let on_subject =
        format!("{model}  claim c:\n    forall a: Bytes, b: Bytes. inc(u(a, b)) == inc(1u8)\n");
    let e = refusal(&doc_ops("", &[op], &on_subject));
    assert_eq!(e.code, code::W0701, "{e}");
    assert_eq!(e.at, "c");
    assert!(
        e.message.contains("inc") && e.message.contains("rule 3"),
        "{e}"
    );
    // the flag follows the call closure: `twice` calls `inc`
    let via = format!("{model}  claim c2:\n    forall a: Bytes, b: Bytes. twice(u(a, b)) == 1u8\n");
    assert_eq!(refusal(&doc_ops("", &[op], &via)).code, code::W0701);
    // a v1 document wraps everywhere: the same claim lowers
    with_file(&doc_ops(" v1", &[op], &on_subject), |f| {
        assert!(lower_document(f).is_ok())
    });
    // pure arguments are fine in v0 (that is `wraps` territory), and so is a model with no
    // arithmetic on a subject result
    let pure = format!("{model}  model same(x: U8) -> U8 = x\n  claim c3:\n    forall a: U8. u(b\"\", b\"\") == inc(a)\n  claim c4:\n    forall a: Bytes, b: Bytes. same(u(a, b)) == 1u8\n");
    with_file(&doc_ops("", &[op], &pure), |f| {
        assert!(lower_document(f).is_ok())
    });
}

#[test]
fn an_example_must_be_one_subject_call_compared_with_a_pure_value() {
    let op = "op f(a: Bytes, b: Bytes) -> Bool";
    let two = "  example ex:\n    f(b\"00\", b\"00\") == f(b\"00\", b\"00\")\n";
    let e = refusal(&doc_ops("", &[op], two));
    assert_eq!(e.code, code::W0701);
    assert_eq!(e.at, "ex");
    let none = "  example ex2:\n    true == true\n";
    assert_eq!(refusal(&doc_ops("", &[op], none)).code, code::W0701);
    let ok = "  example ex3:\n    f(b\"00\", b\"00\") == true\n";
    with_file(&doc_ops("", &[op], ok), |f| {
        assert!(lower_document(f).is_ok())
    });
    // an impure guard in an example is refused by the pure-guard rule
    let guard = "  example ex4:\n    (if f(b\"\", b\"\") then f(b\"\", b\"\") else true) == true\n";
    assert_eq!(refusal(&doc_ops("", &[op], guard)).code, code::W0701);
}
