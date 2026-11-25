//! The structural rules of VER32 5.3: versions, features, imports, the index tables
//! (dense, backward-only, bound-variable range), names, counts, feature use and the
//! trailer. One negative test per rule, each asserting the exact error kind, and the
//! unsound-acceptance attempts (forward references, index reuse, truncation).

mod pif_common;

use lean_kernel::pif::{FileClass, GateInputs, PifError as E};
use pif_common::{err, file, lib, lib_err, ok, B};

fn hdr(pif: &str, logic: &str, features: &str) -> String {
    format!("pif {pif}\nlogic {logic}\nprofile native \"t\"\nfeatures{features}\n")
}

fn named(features: &[&str]) -> GateInputs<'static> {
    GateInputs {
        allowed_features: features.iter().map(|s| s.to_string()).collect(),
        ..lib()
    }
}

// ---- R-VER ------------------------------------------------------------------------------

#[test]
fn r_ver_1_unknown_and_withdrawn_versions() {
    assert_eq!(
        lib_err(&format!("{}end 0\n", hdr("2", "1", ""))),
        E::UnknownVersion
    );
    assert_eq!(
        lib_err(&format!("{}end 0\n", hdr("0", "1", ""))),
        E::UnknownVersion
    );
    assert_eq!(
        lib_err(&format!("{}end 0\n", hdr("1", "2", ""))),
        E::UnknownVersion
    );
    let g = GateInputs {
        withdrawn_logic: vec![1],
        ..lib()
    };
    // a withdrawn version is its own error, even though 1 is otherwise known
    assert_eq!(
        err(format!("{}end 0\n", hdr("1", "1", "")).as_bytes(), &g),
        E::WithdrawnLogicVersion
    );
}

#[test]
fn r_ver_2_features_known_allowed_and_implemented() {
    let text = |f: &str| format!("{}end 0\n", hdr("1", "1", f));
    // not a feature of this logic version at all
    assert_eq!(
        err(text(" magic").as_bytes(), &named(&["magic"])),
        E::UnknownFeature
    );
    // known, but the gate did not allow it for this environment
    for f in ["lits", "quotient", "mutual", "nested"] {
        assert_eq!(
            lib_err(&text(&format!(" {f}"))),
            E::FeatureNotAllowed,
            "{f}"
        );
    }
    // allowed by the gate, but this kernel (slices 1 to 3a) implements none: fail closed
    for f in ["lits", "quotient", "mutual", "nested"] {
        let e = err(text(&format!(" {f}")).as_bytes(), &named(&[f]));
        assert!(matches!(e, E::Unsupported(_)), "{f}: {e:?}");
    }
}

#[test]
fn r_ver_3_features_are_distinct_and_ascending() {
    let g = named(&["lits", "mutual"]);
    let text = |f: &str| format!("{}end 0\n", hdr("1", "1", f));
    assert_eq!(err(text(" mutual lits").as_bytes(), &g), E::FeatureOrder);
    assert_eq!(err(text(" lits lits").as_bytes(), &g), E::FeatureOrder);
}

// ---- R-IMP ------------------------------------------------------------------------------

#[test]
fn r_imp_1_imports_equal_the_gates_chain_in_order() {
    let d1 = "a".repeat(64);
    let d2 = "b".repeat(64);
    let text = |imports: &[&str]| {
        let mut s = hdr("1", "1", "");
        for d in imports {
            s.push_str(&format!("import sha256:{d}\n"));
        }
        s + "end 0\n"
    };
    let chain = |v: &[&str]| GateInputs {
        expected_imports: v.iter().map(|s| s.to_string()).collect(),
        ..lib()
    };
    ok(text(&[&d1, &d2]).as_bytes(), &chain(&[&d1, &d2]));
    // none expected but one named, one expected but none named, wrong order, wrong digest,
    // a missing one, an extra one
    assert_eq!(err(text(&[&d1]).as_bytes(), &chain(&[])), E::ImportMismatch);
    assert_eq!(err(text(&[]).as_bytes(), &chain(&[&d1])), E::ImportMismatch);
    assert_eq!(
        err(text(&[&d2, &d1]).as_bytes(), &chain(&[&d1, &d2])),
        E::ImportMismatch
    );
    assert_eq!(
        err(text(&[&d2]).as_bytes(), &chain(&[&d1])),
        E::ImportMismatch
    );
    assert_eq!(
        err(text(&[&d1]).as_bytes(), &chain(&[&d1, &d2])),
        E::ImportMismatch
    );
    assert_eq!(
        err(text(&[&d1, &d2]).as_bytes(), &chain(&[&d1])),
        E::ImportMismatch
    );
}

// ---- R-IDX ------------------------------------------------------------------------------

#[test]
fn r_idx_1_indices_are_dense_from_one() {
    // a gap, a restart, a repeat (index reuse), and the predefined index 0
    let cases = [
        file(&["nstr 2 0 \"a\""]),
        file(&["nstr 1 0 \"a\"", "nstr 1 0 \"b\""]),
        file(&["nstr 1 0 \"a\"", "nstr 3 0 \"b\""]),
        file(&["nstr 0 0 \"a\""]),
        file(&["lsucc 2 0"]),
        file(&["lsucc 0 0"]),
        file(&["esort 2 0"]),
        file(&["esort 0 0"]),
    ];
    for c in &cases {
        assert_eq!(lib_err(c), E::IndexNotDense, "{c}");
    }
}

#[test]
fn index_reuse_cannot_rebind_a_name_to_a_different_meaning() {
    // the second record tries to redefine expression 1 (Prop) as Type
    let text = file(&["lsucc 1 0", "esort 1 0", "esort 1 1"]);
    assert_eq!(lib_err(&text), E::IndexNotDense);
}

#[test]
fn r_idx_2_operands_refer_only_backwards() {
    let cases = [
        file(&["nstr 1 1 \"a\""]),                    // self reference
        file(&["nstr 1 2 \"a\"", "nstr 2 0 \"b\""]),  // forward reference
        file(&["nstr 1 0 \"a\"", "nstr 2 5 \"b\""]),  // never defined
        file(&["lsucc 1 1"]),                         // level self reference
        file(&["lsucc 1 2", "lsucc 2 0"]),            // level forward
        file(&["lmax 1 0 1"]),                        // level self reference, second operand
        file(&["lparam 1 1"]),                        // no name 1 yet
        file(&["esort 1 1"]),                         // no level 1 yet
        file(&["eapp 1 1 1"]),                        // expression self reference
        file(&["esort 1 0", "eapp 2 1 2"]),           // expression self reference as operand
        file(&["esort 1 0", "eapp 2 1 3"]),           // expression forward reference
        file(&["esort 1 0", "eapp 2 0 1"]),           // expression 0 does not exist
        file(&["econst 1 0 [1]"]),                    // level 1 not defined
        file(&["esort 1 0", "elam 2 default 1 1 1"]), // binder name 1 not defined
    ];
    for c in &cases {
        assert_eq!(lib_err(c), E::ForwardReference, "{c}");
    }
}

#[test]
fn r_idx_2_a_theorem_cannot_name_an_expression_defined_after_it() {
    // the declaration refers to a value index that the next record would define
    let mut b = B::new();
    let t = b.name("t");
    let prop = b.sort(0);
    b.line(&format!("theorem {t} [] {prop} {}", b.ne + 1));
    b.sort(0);
    assert_eq!(err(&b.bytes(), &lib()), E::ForwardReference);
}

#[test]
fn r_idx_3_a_bound_variable_index_is_range_checked() {
    // MAX_BVAR_INDEX is u32::MAX / 2; the unchecked constructor would panic here
    let top = u32::MAX / 2;
    assert_eq!(
        lib_err(&file(&[&format!("evar 1 {top}")])),
        E::BVarIndexTooLarge
    );
    assert_eq!(lib_err(&file(&["evar 1 4294967295"])), E::BVarIndexTooLarge);
    // the largest index below the ceiling is a legal term (a loose one: the kernel's job)
    ok(file(&[&format!("evar 1 {}", top - 1)]).as_bytes(), &lib());
}

// ---- R-NAM ------------------------------------------------------------------------------

#[test]
fn r_nam_1_name_components() {
    let bad = [
        "\"\"",            // empty
        "\"a.b\"",         // contains a dot
        "\"a\\u{1}b\"",    // control character
        "\"\\u{7f}\"",     // DEL
        "\"\\u{85}\"",     // C1 control
        "\"\\u{200b}\"",   // zero-width space (format character)
        "\"\\u{202e}\"",   // right-to-left override
        "\"\\u{2066}\"",   // bidi isolate
        "\"\\u{feff}\"",   // byte order mark
        "\"\\u{e000}\"",   // private use
        "\"\\u{f0000}\"",  // supplementary private use
        "\"\\u{10fffd}\"", // supplementary private use
    ];
    for s in bad {
        let text = file(&[&format!("nstr 1 0 {s}")]);
        assert_eq!(lib_err(&text), E::BadNameComponent, "{s}");
    }
    // legal non-ASCII components are fine in a library; a space and a quote are fine too
    for s in [
        "\"\\u{e9}\"",
        "\"\\u{20}\"",
        "\"\\\"\"",
        "\"\\u{1f600}\"",
        "\"\\u{2065}\"",
    ] {
        ok(file(&[&format!("nstr 1 0 {s}")]).as_bytes(), &lib());
    }
}

#[test]
fn r_nam_1_a_numeric_component_is_not_a_string_component() {
    // nnum 3 and nstr "3" are different names: both can be declared side by side
    let mut b = B::new();
    let p = b.nstr(0, "p");
    b.line(&format!("nnum {} {p} 3", b.nn + 1));
    let num = b.nn + 1;
    b.nn += 1;
    let s3 = b.nstr(p, "3");
    let prop = b.sort(0);
    b.axiom(num, &[], prop);
    b.axiom(s3, &[], prop);
    let r = ok(&b.bytes(), &lib());
    assert_eq!(r.decls.len(), 2);
    assert_ne!(r.decls[0].name, r.decls[1].name);
}

#[test]
fn r_nam_2_the_anonymous_name_cannot_be_declared_or_be_a_parameter() {
    let mut b = B::new();
    let prop = b.sort(0);
    b.axiom(0, &[], prop);
    assert_eq!(err(&b.bytes(), &lib()), E::AnonymousName);
    let mut b = B::new();
    let prop = b.sort(0);
    let n = b.name("a");
    b.axiom(n, &[0], prop);
    assert_eq!(err(&b.bytes(), &lib()), E::AnonymousName);
    // nor can level 1 be a parameter named by the anonymous name
    assert_eq!(lib_err(&file(&["lparam 1 0"])), E::AnonymousName);
    // an inductive type or constructor named anonymous
    let mut b = B::new();
    let one = b.lsucc(0);
    let ty = b.sort(one);
    b.line("inductive [] 0 1");
    b.line(&format!("itype 0 {ty} 0"));
    assert_eq!(err(&b.bytes(), &lib()), E::AnonymousName);
}

#[test]
fn r_nam_3_reviewed_files_use_the_ascii_class_for_every_name() {
    let reviewed = GateInputs {
        class: FileClass::Reviewed,
        ..GateInputs::default()
    };
    // a declared name with a non-ASCII component (Cyrillic a, a homoglyph of Latin a)
    let homoglyph = |build: &dyn Fn(&mut B, usize)| {
        let mut b = B::new();
        let bad = b.line("nstr 1 0 \"\\u{430}\"").nn + 1;
        b.nn = 1;
        let _ = bad;
        build(&mut b, 1);
        b
    };
    let b = homoglyph(&|b, n| {
        let prop = b.sort(0);
        b.axiom(n, &[], prop);
    });
    assert_eq!(err(&b.bytes(), &reviewed), E::NameNotAscii);
    // the same file is fine as a library
    ok(&b.bytes(), &lib());
    // components outside the class: digit first, dash, quote inside is allowed
    for comp in ["1a", "a-b", "a b"] {
        let mut b = B::new();
        let n = b.nstr(0, &comp.replace(' ', "\\u{20}"));
        let prop = b.sort(0);
        b.axiom(n, &[], prop);
        assert_eq!(err(&b.bytes(), &reviewed), E::NameNotAscii, "{comp}");
    }
    let mut b = B::new();
    let n = b.nstr(0, "a'b_1");
    let prop = b.sort(0);
    b.axiom(n, &[], prop);
    ok(&b.bytes(), &reviewed);
    // a numeric component is outside the class
    let mut b = B::new();
    b.line("nnum 1 0 7");
    b.nn = 1;
    let prop = b.sort(0);
    b.axiom(1, &[], prop);
    assert_eq!(err(&b.bytes(), &reviewed), E::NameNotAscii);
    // the anonymous name is not a name a person can read: a binder named by it is refused
    let mut b = B::new();
    let n = b.name("f");
    let prop = b.sort(0);
    let pi = b.pi(0, prop, prop);
    b.axiom(n, &[], pi);
    assert_eq!(err(&b.bytes(), &reviewed), E::NameNotAscii);
    ok(&b.bytes(), &lib());
}

#[test]
fn r_nam_3_binder_and_universe_parameter_names_are_checked_too() {
    let reviewed = GateInputs {
        class: FileClass::Reviewed,
        ..GateInputs::default()
    };
    // a binder named with a homoglyph, in a file whose declared name is fine
    let mut b = B::new();
    let f = b.name("f");
    let h = b.line("nstr 2 0 \"\\u{430}\"").nn + 1;
    b.nn = h;
    let prop = b.sort(0);
    let pi = b.pi(h, prop, prop);
    b.axiom(f, &[], pi);
    assert_eq!(err(&b.bytes(), &reviewed), E::NameNotAscii);
    // a universe parameter with a homoglyph name
    let mut b = B::new();
    let f = b.name("f");
    b.line("nstr 2 0 \"\\u{430}\"");
    b.nn = 2;
    let l = b.lparam(2);
    let s = b.sort(l);
    b.axiom(f, &[2], s);
    assert_eq!(err(&b.bytes(), &reviewed), E::NameNotAscii);
    ok(&b.bytes(), &lib());
}

#[test]
fn name_depth_is_limited_to_32_components() {
    let chain = |n: usize| {
        let mut b = B::new();
        (0..n).fold(0, |p, _| b.nstr(p, "a"));
        b.text()
    };
    ok(chain(32).as_bytes(), &lib());
    assert_eq!(lib_err(&chain(33)), E::LimitExceeded("name components"));
}

// ---- R-CNT, R-FEA, trailer ----------------------------------------------------------------

#[test]
fn r_cnt_1_universe_parameters_are_distinct() {
    let mut b = B::new();
    let u = b.name("u");
    let a = b.name("a");
    let l = b.lparam(u);
    let s = b.sort(l);
    b.axiom(a, &[u, u], s);
    assert_eq!(err(&b.bytes(), &lib()), E::CountMismatch);
}

#[test]
fn r_cnt_1_inductive_counts_are_validated_not_trusted() {
    // build a valid unit type, then lie about each count in turn
    let good = {
        let mut b = B::new();
        b.unit_type("T");
        b
    };
    ok(&good.bytes(), &lib());
    let lie = |from: &str, to: &str| {
        let text = good.text().replace(from, to);
        assert_ne!(text, good.text());
        text
    };
    // one itype declared, but zero or two written; one ictor declared, but zero or two written
    // (two declared is a mutual block, which needs a feature before counts are even looked at)
    assert_eq!(
        lib_err(&lie("inductive [] 0 1", "inductive [] 0 2")),
        E::FeatureNotDeclared
    );
    assert_eq!(
        lib_err(&lie("inductive [] 0 1", "inductive [] 0 0")),
        E::CountMismatch
    );
    assert_eq!(
        lib_err(&lie("itype 1 1 1", "itype 1 1 2")),
        E::CountMismatch
    );
    assert_eq!(
        lib_err(&lie("itype 1 1 1", "itype 1 1 0")),
        E::CountMismatch
    );
    let mut b = B::new();
    let (_, mk) = b.unit_type("T");
    b.line(&format!("ictor {mk} 2"));
    assert_eq!(err(&b.bytes(), &lib()), E::CountMismatch);
    let mut b = B::new();
    b.unit_type("T");
    b.line("itype 1 3 0");
    assert_eq!(err(&b.bytes(), &lib()), E::CountMismatch);
    // an inductive head with no itype at all
    let mut b = B::new();
    b.line("inductive [] 0 1");
    assert_eq!(err(&b.bytes(), &lib()), E::CountMismatch);
}

#[test]
fn an_itype_or_ictor_outside_a_block_is_a_lexical_error() {
    assert_eq!(lib_err(&file(&["itype 1 1 0"])), E::BadLexicalForm);
    assert_eq!(lib_err(&file(&["ictor 1 1"])), E::BadLexicalForm);
}

#[test]
fn r_cnt_2_the_trailer_counts_every_record_line() {
    let mut b = B::new();
    b.prop_axiom("A");
    let good = b.text();
    ok(good.as_bytes(), &lib());
    let n = b.body.len();
    for wrong in [n - 1, n + 1, 0, n + 1000] {
        let text = good.replace(&format!("end {n}\n"), &format!("end {wrong}\n"));
        assert_eq!(lib_err(&text), E::TrailerMismatch, "end {wrong}");
    }
    // a missing `end` line is truncation, with or without a partial last line
    let cut = good.replace(&format!("end {n}\n"), "");
    assert_eq!(lib_err(&cut), E::TrailerMismatch);
    assert_eq!(lib_err(&good[..good.len() - 2]), E::BadLexicalForm);
    // truncation in the middle of the records leaves a count that no longer matches
    let lines: Vec<&str> = good.lines().collect();
    let truncated = format!(
        "{}\n{}\n",
        lines[..lines.len() - 3].join("\n"),
        lines[lines.len() - 1]
    );
    assert_eq!(lib_err(&truncated), E::TrailerMismatch);
}

#[test]
fn r_cnt_2_an_inductive_block_counts_every_physical_line() {
    let mut b = B::new();
    b.unit_type("T");
    // name, name, level, sort, const, inductive, itype, ictor: 8 physical lines
    assert_eq!(b.body.len(), 8);
    ok(&b.bytes(), &lib());
    let text = b.text().replace("end 8", "end 6");
    assert_eq!(lib_err(&text), E::TrailerMismatch);
}

#[test]
fn r_fea_1_records_that_need_a_feature_need_it_declared() {
    let mut b = B::new();
    b.sort(0);
    b.line("enat 2 5");
    assert_eq!(err(&b.bytes(), &lib()), E::FeatureNotDeclared);
    let mut b = B::new();
    b.line("estr 1 \"abc\"");
    assert_eq!(err(&b.bytes(), &lib()), E::FeatureNotDeclared);
    // a string literal that spells U+0000 is lexically legal in estr only
    let mut b = B::new();
    b.line("estr 1 \"a\\u{0}b\"");
    assert_eq!(err(&b.bytes(), &lib()), E::FeatureNotDeclared);
    let mut b = B::new();
    b.line("quot");
    assert_eq!(err(&b.bytes(), &lib()), E::FeatureNotDeclared);
    // a mutual block (more than one itype) needs `mutual`
    let mut b = B::new();
    b.line("inductive [] 0 2");
    assert_eq!(err(&b.bytes(), &lib()), E::FeatureNotDeclared);
}

#[test]
fn records_the_kernel_cannot_do_yet_fail_closed() {
    // projections, and prim roles of any name, are Unsupported, never skipped
    let e = lib_err(&file(&["eproj 1 1 1 1"]));
    assert!(matches!(e, E::Unsupported(_)), "{e:?}");
    let e = lib_err(&file(&["prim nat 1"]));
    assert!(matches!(e, E::Unsupported(_)), "{e:?}");
    // but their lexical form is still checked first
    assert_eq!(lib_err(&file(&["prim Nat 1"])), E::BadLexicalForm);
    assert_eq!(lib_err(&file(&["eproj 1 1 1"])), E::BadLexicalForm);
    assert_eq!(lib_err(&file(&["enat 1 01"])), E::BadLexicalForm);
    assert_eq!(lib_err(&file(&["quot 1"])), E::BadLexicalForm);
    assert_eq!(lib_err(&file(&["enat 2 5"])), E::IndexNotDense);
}
