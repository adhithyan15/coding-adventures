//! Unit tests of the writer: spelling, sharing, determinism, refusals, and the records the
//! kernel does not implement yet (the reader must refuse those as `Unsupported`, never as a
//! lexical error, which would mean the writer's spelling is wrong).

use lean_kernel::pif::{read, FileClass, GateInputs, PifError};
use lean_kernel::Environment;
use pif_writer::{BinderInfo, Hint, IndType, Level, Name, WriteError, Writer, MAX_VAR_INDEX};

fn w() -> Writer {
    Writer::new("native", "t").unwrap()
}

fn lib() -> GateInputs<'static> {
    GateInputs {
        class: FileClass::Library,
        allowed_features: ["lits", "mutual", "nested", "quotient"]
            .map(String::from)
            .to_vec(),
        ..GateInputs::default()
    }
}

fn read_lib(text: &str) -> Result<lean_kernel::pif::ReadReport, PifError> {
    read(Environment::new(), text.as_bytes(), &lib())
}

#[test]
fn an_empty_file_is_the_header_and_end_zero() {
    let t = w().to_text();
    assert_eq!(t, "pif 1\nlogic 1\nprofile native \"t\"\nfeatures\nend 0\n");
    assert!(read_lib(&t).is_ok());
}

#[test]
fn strings_use_only_the_canonical_escapes() {
    let mut x = Writer::new("native", "a b\"c\\d\u{e9}\u{1f600}~!").unwrap();
    x.name_str(Name::ANON, "a'b").unwrap();
    let t = x.to_text();
    assert!(t.contains("profile native \"a\\u{20}b\\\"c\\\\d\\u{e9}\\u{1f600}~!\"\n"));
    assert!(read_lib(&t).is_ok());
}

#[test]
fn non_ascii_names_are_allowed_in_library_files_and_refused_in_reviewed_ones() {
    let mut x = w();
    let n = x.name("\u{3b1}").unwrap();
    assert_eq!(n.index(), 1);
    assert!(x.to_text().contains("nstr 1 0 \"\\u{3b1}\""));
    assert!(read_lib(&x.to_text()).is_ok());
    let mut y = w().ascii_names_only();
    assert!(matches!(
        y.name("\u{3b1}"),
        Err(WriteError::BadNameComponent(_))
    ));
    assert!(matches!(y.name("1x"), Err(WriteError::BadNameComponent(_))));
    assert!(y.name("it's_ok").is_ok());
}

#[test]
fn bad_name_components_are_refused() {
    let mut x = w();
    for bad in [
        "",
        "a\0b",
        "a\u{7f}",
        "a\u{200b}",
        "a\u{feff}",
        "a\u{e000}",
        "a\u{f0000}",
    ] {
        assert!(x.name_str(Name::ANON, bad).is_err(), "{bad:?}");
    }
    assert!(x.name("a..b").is_err());
    assert!(x.name("").is_err());
    assert_eq!(x.record_count(), 0);
}

#[test]
fn numeric_and_string_components_are_different_names() {
    let mut x = w();
    let p = x.name("P").unwrap();
    let s = x.name_str(p, "3").unwrap();
    let n = x.name_num(p, 3).unwrap();
    assert_ne!(s, n);
    assert_eq!(x.name_num(p, 3).unwrap(), n);
    let t = x.to_text();
    assert!(t.contains("nstr 2 1 \"3\"\nnnum 3 1 3\n"));
    assert!(read_lib(&t).is_ok());
}

#[test]
fn every_distinct_node_is_emitted_once() {
    let mut x = w();
    let a = x.name("A").unwrap();
    assert_eq!(x.name("A").unwrap(), a);
    let l = x.level_succ(Level::ZERO);
    assert_eq!(x.level_succ(Level::ZERO), l);
    let m = x.level_max(l, Level::ZERO);
    assert_eq!(x.level_max(l, Level::ZERO), m);
    let i = x.level_imax(m, l);
    assert_eq!(x.level_imax(m, l), i);
    let s = x.sort(i);
    assert_eq!(x.sort(i), s);
    let c = x.constant(a, &[l, m]);
    assert_eq!(x.constant(a, &[l, m]), c);
    assert_ne!(x.constant(a, &[m, l]), c);
    let v = x.var(0);
    let id = x.app(c, v);
    assert_eq!(x.app(c, v), id);
    // binder info and binder name are part of the record, so they are part of the identity
    let n = x.name("x").unwrap();
    let d = x.lam(BinderInfo::Default, n, s, v);
    assert_ne!(x.lam(BinderInfo::Implicit, n, s, v), d);
    assert_eq!(x.lam(BinderInfo::Default, n, s, v), d);
    let before = x.record_count();
    x.pi(BinderInfo::Strict, n, s, v);
    x.pi(BinderInfo::Strict, n, s, v);
    x.pi(BinderInfo::Inst, n, s, v);
    assert_eq!(x.record_count(), before + 2);
    assert!(read_lib(&x.to_text()).is_ok());
}

#[test]
fn the_same_calls_give_identical_bytes() {
    let go = || {
        let mut x = w();
        let a = x.name("A.b.c").unwrap();
        let l = x.level_param(a);
        let s = x.sort(l);
        x.axiom(a, &[a], s).unwrap();
        x.to_text()
    };
    assert_eq!(go(), go());
}

#[test]
fn bound_variable_indices_are_bounded() {
    let mut x = w();
    assert!(x.try_var(MAX_VAR_INDEX - 1).is_ok());
    assert_eq!(
        x.try_var(MAX_VAR_INDEX),
        Err(WriteError::BVarTooLarge(MAX_VAR_INDEX))
    );
    assert!(read_lib(&x.to_text()).is_ok());
}

#[test]
#[should_panic(expected = "too large")]
fn var_panics_above_the_bound() {
    w().var(u32::MAX);
}

#[test]
fn declaration_heads_are_checked() {
    let mut x = w();
    let a = x.name("A").unwrap();
    let t = x.sort(Level::ZERO);
    assert_eq!(x.axiom(Name::ANON, &[], t), Err(WriteError::AnonymousName));
    assert_eq!(x.axiom(a, &[a, a], t), Err(WriteError::DuplicateParam));
    assert_eq!(x.inductive(&[], 0, &[]), Err(WriteError::EmptyBlock));
    let bad = IndType {
        name: Name::ANON,
        ty: t,
        ctors: vec![],
    };
    assert_eq!(x.inductive(&[], 0, &[bad]), Err(WriteError::AnonymousName));
    let bad = IndType {
        name: a,
        ty: t,
        ctors: vec![(Name::ANON, t)],
    };
    assert_eq!(x.inductive(&[], 0, &[bad]), Err(WriteError::AnonymousName));
}

#[test]
fn every_declaration_kind_is_read_back_with_its_kind() {
    use lean_kernel::pif::Kind::*;
    let mut x = w();
    let prop = x.sort(Level::ZERO);
    let names: Vec<Name> = ["P", "p", "D", "O", "T"]
        .iter()
        .map(|n| x.name(n).unwrap())
        .collect();
    let (pn, pp, dn, on, tn) = (names[0], names[1], names[2], names[3], names[4]);
    x.axiom(pn, &[], prop).unwrap();
    let pc = x.constant(pn, &[]);
    x.axiom(pp, &[], pc).unwrap();
    let ppc = x.constant(pp, &[]);
    x.definition(dn, &[], prop, pc, Hint::Abbrev).unwrap();
    x.opaque(on, &[], pc, ppc).unwrap();
    x.theorem(tn, &[], pc, ppc).unwrap();
    let r = read_lib(&x.to_text()).unwrap();
    let kinds: Vec<_> = r.decls.iter().map(|d| d.kind).collect();
    assert_eq!(kinds, [Axiom, Axiom, Definition, Opaque, Theorem]);
}

#[test]
fn definition_hints_are_spelled_as_the_grammar_says() {
    let mut x = w();
    let prop = x.sort(Level::ZERO);
    let cases = [
        (Hint::Abbrev, "@abbrev"),
        (Hint::Opaque, "@opaque"),
        (Hint::Regular(0), "@regular 0"),
    ];
    for (i, (h, want)) in cases.into_iter().enumerate() {
        let n = x.name(&format!("D{i}")).unwrap();
        x.definition(n, &[], prop, prop, h).unwrap();
        assert!(x
            .to_text()
            .contains(&format!("def {} [] 1 1 {want}\n", i + 1)));
    }
}

#[test]
fn inductive_blocks_and_universe_parameters_read_back() {
    let mut x = w();
    let u = x.name("u").unwrap();
    let lu = x.level_param(u);
    let t = x.name("T").unwrap();
    let mk = x.name("T.mk").unwrap();
    let ty = x.sort(lu);
    let tc = x.constant(t, &[lu]);
    let blk = IndType {
        name: t,
        ty,
        ctors: vec![(mk, tc)],
    };
    x.inductive(&[u], 0, &[blk]).unwrap();
    let r = read_lib(&x.to_text()).unwrap();
    assert_eq!(r.decls.len(), 3); // T, T.mk and the derived T.rec
}

#[test]
fn imports_features_and_their_validation() {
    let mut x = w();
    let d = "0".repeat(64);
    x.import(&d).unwrap();
    assert!(matches!(x.import("ABC"), Err(WriteError::BadDigest(_))));
    assert!(matches!(
        x.import(&"A".repeat(64)),
        Err(WriteError::BadDigest(_))
    ));
    x.feature("nested").unwrap();
    x.feature("lits").unwrap();
    x.feature("lits").unwrap();
    assert!(matches!(x.feature("Lits"), Err(WriteError::BadIdent(_))));
    assert!(matches!(x.feature("inst"), Err(WriteError::BadIdent(_))));
    // features are distinct and ascending whatever order they were declared in
    let t = x.to_text();
    assert!(t.contains(&format!("features lits nested\nimport sha256:{d}\n")));
    assert!(matches!(
        Writer::new("Native", "x"),
        Err(WriteError::BadIdent(_))
    ));
    assert_eq!(
        Writer::new("native", "a\0").err(),
        Some(WriteError::BadLabel)
    );
}

#[test]
fn records_the_kernel_does_not_implement_yet_are_spelled_right() {
    // The reader refuses each of these for want of a kernel slice (`Unsupported` or
    // `FeatureNotDeclared`), never for its form: that proves the spelling is accepted.
    let mut x = w();
    let n = x.name("n").unwrap();
    let nat = x.nat("12345678901234567890123").unwrap();
    assert!(matches!(x.nat("007"), Err(WriteError::BadNumeral(_))));
    assert!(matches!(x.nat(""), Err(WriteError::BadNumeral(_))));
    assert!(matches!(x.nat("-1"), Err(WriteError::BadNumeral(_))));
    let zero = x.nat("0").unwrap();
    let s = x.string("a b\0\"");
    let ty = x.sort(Level::ZERO);
    let l = x.let_(n, ty, nat, zero);
    let p = x.proj(n, 1, s);
    let t = x.to_text();
    for want in [
        "features lits\n",
        "enat 1 12345678901234567890123\n",
        "estr 3 \"a\\u{20}b\\u{0}\\\"\"\n",
        "elet 5 1 4 1 2\n",
        "eproj 6 1 1 3\n",
    ] {
        assert!(t.contains(want), "missing {want:?} in\n{t}");
    }
    let _ = (l, p);
    assert!(matches!(
        read_lib(&t),
        Err(PifError::Unsupported(_) | PifError::FeatureNotDeclared)
    ));

    let mut q = w();
    q.quot();
    let c = q.name("C").unwrap();
    q.prim("nat.add", c).unwrap();
    assert!(matches!(q.prim("Nat", c), Err(WriteError::BadIdent(_))));
    let t = q.to_text();
    assert!(t.contains("features quotient\n"));
    assert!(t.contains("quot\nnstr 1 0 \"C\"\nprim nat.add 1\n"));
    assert!(
        matches!(read_lib(&t), Err(PifError::Unsupported(_))),
        "{:?}",
        read_lib(&t).err()
    );
}

#[test]
fn a_mutual_block_declares_the_mutual_feature() {
    let mut x = w();
    let ty = x.sort(Level::ZERO);
    let (a, b) = (x.name("A").unwrap(), x.name("B").unwrap());
    let blk = |n| IndType {
        name: n,
        ty,
        ctors: vec![],
    };
    x.inductive(&[], 0, &[blk(a), blk(b)]).unwrap();
    let t = x.to_text();
    assert!(t.contains("features mutual\n"));
    assert!(t.contains("inductive [] 0 2\nitype 1 1 0\nitype 2 1 0\n"));
    assert!(
        matches!(read_lib(&t), Err(PifError::Unsupported(_))),
        "{:?}",
        read_lib(&t).err()
    );
}

#[test]
fn goals_are_spelled_and_validated() {
    let mut x = w();
    let t = x.name("T").unwrap();
    let d = "a".repeat(64);
    x.goal("a.b-c_9", t, &d).unwrap();
    assert!(x
        .to_text()
        .contains(&format!("goal \"a.b-c_9\" 1 sha256:{d}\n")));
    for bad in ["", "a b", "\u{e9}", &"x".repeat(129)] {
        assert!(matches!(x.goal(bad, t, &d), Err(WriteError::BadClaimId(_))));
    }
    assert!(matches!(
        x.goal("ok", t, "xyz"),
        Err(WriteError::BadDigest(_))
    ));
}

#[test]
fn names_beyond_the_readers_caps_are_refused_and_the_caps_are_accepted() {
    use pif_writer::{MAX_NAME_BYTES, MAX_NAME_COMPONENTS};
    let mut x = w();
    // exactly 32 components is accepted; 33 is not, and nothing half-built is left behind
    let ok = vec!["a"; MAX_NAME_COMPONENTS as usize].join(".");
    let n = x.name(&ok).unwrap();
    let before = x.record_count();
    assert_eq!(x.name(&format!("{ok}.a")), Err(WriteError::NameTooLong));
    assert_eq!(x.name_str(n, "a"), Err(WriteError::NameTooLong));
    assert_eq!(x.name_num(n, 1), Err(WriteError::NameTooLong));
    assert_eq!(x.record_count(), before);
    assert!(read_lib(&x.to_text()).is_ok());
    // exactly 4096 bytes is accepted; one more is not
    let mut y = w();
    let full = "b".repeat(MAX_NAME_BYTES);
    let n = y.name_str(Name::ANON, &full).unwrap();
    assert!(read_lib(&y.to_text()).is_ok());
    assert_eq!(
        y.name_str(Name::ANON, &"b".repeat(MAX_NAME_BYTES + 1)),
        Err(WriteError::NameTooLong)
    );
    assert_eq!(y.name_str(n, "c"), Err(WriteError::NameTooLong));
    assert_eq!(y.name(&format!("{full}.c")), Err(WriteError::NameTooLong));
    // a numeric component counts as 8 bytes
    let mut z = w();
    let p = z
        .name_str(Name::ANON, &"c".repeat(MAX_NAME_BYTES - 8))
        .unwrap();
    let q = z.name_num(p, 7).unwrap();
    assert!(read_lib(&z.to_text()).is_ok());
    assert_eq!(z.name_num(q, 8), Err(WriteError::NameTooLong));
}
