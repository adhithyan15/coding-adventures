//! PIF-003d: the printer, its independent parser, the round trips and the G7 name rules.

mod common;

use common::*;
use lean_kernel::{Binder, BinderInfo, Environment, Level, Name};
use pif_check::chain::{load_chain, LoadOptions, Loaded};
use pif_check::control::Role;
use pif_check::cte::{display_cte_from_store, Dterm, Lv, Node};
use pif_check::filemodel::{decode, DeclModel, HintModel, ModelError};
use pif_check::filetext::{parse_file, print_file};
use pif_check::goals::{check_goals, parse_goals_list};
use pif_check::names::{check_goal_names, NameRule};
use pif_check::ppparse::{parse_term, tokenize, PpError};
use pif_check::printcheck::{agrees_with_kernel, check_goal, check_reviewed_file, PrintCheckError};
use pif_check::printer::{print_level, print_term, PrintError};
use pif_check::resolver::Resolver;
use pif_check::{CheckError, Digest};

const WORKED: &str = include_str!("../../lean-kernel/tests/fixtures/worked-example.pif");

fn n(s: &str) -> Name {
    Name::from_dotted(s)
}

fn prelude_loaded() -> Loaded {
    let p = prelude();
    let m = mem(&[&p]);
    load_chain(
        &Resolver::new(&m),
        &[entry(Role::Prelude, &p)],
        &LoadOptions::default(),
    )
    .unwrap()
}

/// The worked example's goal as a display term (names from the file).
fn worked_goal() -> Dterm {
    let model = decode(WORKED).unwrap();
    let Some(DeclModel::Theorem { ty, .. }) = model
        .decls
        .iter()
        .find(|d| matches!(d, DeclModel::Theorem { .. }))
    else {
        panic!("no theorem")
    };
    ty.clone()
}

fn dt(text: &str) -> Dterm {
    Dterm::parse(text).unwrap()
}

// ---- goals --------------------------------------------------------------------------------

#[test]
fn the_worked_goal_prints_readably_and_round_trips() {
    let g = worked_goal();
    let text = print_term(&g).unwrap();
    assert_eq!(
        text,
        "forall (a : Bool), forall (b : Bool), Eq.{1} Bool (Pif.and #1 #0) (Pif.and #0 #1)"
    );
    assert_eq!(parse_term(&text).unwrap(), g);
    assert_eq!(check_goal(&g).unwrap(), text);
    // Deterministic.
    assert_eq!(print_term(&g).unwrap(), text);
}

/// A term that uses every form: binder infos, arrows, lets, sorts with every level shape.
fn every_form() -> Dterm {
    let mut env = Environment::new();
    let st = &mut env.store;
    let (u, v) = (Level::param(&n("u")), Level::param(&n("v")));
    let b = |s: &str, i| Binder {
        name: n(s),
        info: i,
    };
    let lv = Level::imax(&Level::offset(&u, 2), &Level::max(&Level::of_nat(3), &v));
    let s1 = st.sort(&lv);
    let prop = st.sort(&Level::zero());
    let c = st.constant(&n("Foo.bar"), &[u.clone(), Level::succ(&v), Level::zero()]);
    let v0 = st.bvar(0);
    let v1 = st.bvar(1);
    let app = st.app(c, v1);
    let app = st.app(app, v0);
    let lam_arg = st.lam(b("x", BinderInfo::Default), s1, v0);
    let app2 = st.app(app, lam_arg);
    let arrow = st.pi(b("_", BinderInfo::Default), prop, s1);
    let arrow_arrow = st.pi(b("_", BinderInfo::Default), arrow, prop);
    let dep = st.pi(b("_", BinderInfo::Default), prop, v0); // named `_` but used: a forall
    let imp = st.pi(b("a", BinderInfo::Implicit), s1, app2);
    let strict = st.pi(b("s", BinderInfo::StrictImplicit), arrow_arrow, imp);
    let inst = st.pi(b("i", BinderInfo::InstImplicit), strict, dep);
    let let_ = st.let_e(b("z", BinderInfo::Default), inst, c, v0);
    let lam = st.lam(b("f", BinderInfo::Implicit), let_, app);
    let sort_arg = st.app(c, s1);
    let sort_head = st.app(s1, v0);
    let top = st.app(lam, sort_arg);
    let top = st.app(top, sort_head);
    let top = st.app(top, arrow);
    let top = st.app(top, let_);
    Dterm::from_store(&env.store, top).unwrap()
}

#[test]
fn every_term_form_prints_and_parses_back() {
    let g = every_form();
    let text = print_term(&g).unwrap();
    let back = parse_term(&text).unwrap();
    assert_eq!(back, g, "{text}");
    assert_eq!(back.display().unwrap(), g.display().unwrap());
    // Spot checks of the syntax.
    assert!(text.contains("Sort imax(u+2, max(3, v))"), "{text}");
    assert!(text.contains("Foo.bar.{u, v+1, 0}"), "{text}");
    assert!(text.contains("{|s : "), "{text}");
    assert!(text.contains("[i : "), "{text}");
    assert!(text.contains("let z : "), "{text}");
    assert!(text.contains("forall (_ : Sort 0), #0"), "{text}"); // `_` but used: not an arrow
    assert!(text.contains("Sort 0 -> Sort"), "{text}");
    assert!(text.contains("(Sort 0 -> Sort"), "{text}"); // an arrow on the left of an arrow
}

#[test]
fn levels_print_as_numerals_and_offsets() {
    let t = dt("Ss(s(z))");
    assert_eq!(print_term(&t).unwrap(), "Sort 2");
    let Node::Sort(l) = t.nodes[t.root as usize] else {
        panic!()
    };
    assert_eq!(print_level(&t, l).unwrap(), "2");
    for (cte, text) in [
        ("Sz", "Sort 0"),
        ("Sp<\"u\">", "Sort u"),
        ("Ss(p<\"u\">)", "Sort u+1"),
        ("Sm(s(z),p<\"u\">)", "Sort max(1, u)"),
        ("Ss(i(p<\"u\">,z))", "Sort imax(u, 0)+1"),
    ] {
        let t = dt(cte);
        assert_eq!(print_term(&t).unwrap(), text);
        assert_eq!(parse_term(text).unwrap(), t, "{text}");
    }
}

#[test]
fn the_printer_refuses_what_it_cannot_print_unambiguously() {
    // A used binder shadowing a used binder: the text would mislead a reader.
    let shadow = dt("Pd(<\"a\">,C<\"T\">[],Pd(<\"a\">,C<\"T\">[],A(v0,v1)))");
    assert_eq!(print_term(&shadow), Err(PrintError::Shadowed("a".into())));
    // An inner binder that is unused but the outer one is used inside it: still misleading.
    let outer_used = dt("Pd(<\"a\">,C<\"T\">[],Pd(<\"a\">,C<\"T\">[],v1))");
    assert_eq!(
        print_term(&outer_used),
        Err(PrintError::Shadowed("a".into()))
    );
    // Neither binder is used: harmless, as in the prelude's `Prop -> Prop`.
    let unused = dt("Pd(<\"a\">,C<\"T\">[],Pd(<\"a\">,C<\"T\">[],C<\"T\">[]))");
    assert!(print_term(&unused).is_ok());
    // The outer binder is used only in the inner binder's TYPE, outside the inner scope.
    let in_type = dt("Pd(<\"a\">,C<\"T\">[],Pd(<\"a\">,v0,C<\"T\">[]))");
    assert!(print_term(&in_type).is_ok());
    // `_` may repeat freely, even when used.
    assert!(print_term(&dt(
        "Pd(<\"_\">,C<\"T\">[],Pd(<\"_\">,C<\"T\">[],A(v0,v1)))"
    ))
    .is_ok());
    // Siblings are not shadowing.
    assert!(print_term(&dt(
        "A(Pd(<\"a\">,C<\"T\">[],v0),Pd(<\"a\">,C<\"T\">[],v0))"
    ))
    .is_ok());
    // A let shadowing a binder.
    let let_shadow = dt("Pd(<\"a\">,C<\"T\">[],Z(<\"a\">,C<\"T\">[],v0,A(v0,v1)))");
    assert_eq!(
        print_term(&let_shadow),
        Err(PrintError::Shadowed("a".into()))
    );
    // Names with no printed spelling.
    assert_eq!(
        print_term(&dt("C<\"\\u{430}\">[]")),
        Err(PrintError::NameNotPrintable)
    );
    assert_eq!(
        print_term(&dt("C<\"A\",3>[]")),
        Err(PrintError::NameNotPrintable)
    );
    assert_eq!(print_term(&dt("C<>[]")), Err(PrintError::NameNotPrintable));
    assert_eq!(
        print_term(&dt("C<\"a.b\">[]")),
        Err(PrintError::NameNotPrintable)
    );
    assert_eq!(
        print_term(&dt("C<\"1a\">[]")),
        Err(PrintError::NameNotPrintable)
    );
    for w in ["forall", "fun", "let", "Sort"] {
        assert_eq!(
            print_term(&dt(&format!("C<\"{w}\">[]"))),
            Err(PrintError::ReservedWord(w.into()))
        );
    }
    // A keyword as one component of a longer name is fine.
    assert!(print_term(&dt("C<\"Sort\",\"x\">[]")).is_ok());
    // Literals and projections have no spelling yet.
    assert_eq!(
        print_term(&dt("N5")),
        Err(PrintError::Unprintable("natural literal"))
    );
    assert_eq!(
        print_term(&dt("T\"s\"")),
        Err(PrintError::Unprintable("string literal"))
    );
    assert_eq!(
        print_term(&dt("J(<\"S\">,0,v0)")),
        Err(PrintError::Unprintable("projection"))
    );
    // A universe parameter that is not printable.
    assert_eq!(
        print_term(&dt("Sp<\"\\u{430}\">")),
        Err(PrintError::NameNotPrintable)
    );
    // A malformed tree is an error, not a panic.
    let bad = Dterm {
        nodes: vec![Node::App(5, 6)],
        levels: vec![Lv::Succ(9)],
        root: 0,
    };
    assert_eq!(print_term(&bad), Err(PrintError::Malformed));
    let bad = Dterm {
        nodes: vec![Node::Sort(3)],
        levels: vec![],
        root: 0,
    };
    assert_eq!(print_term(&bad), Err(PrintError::Malformed));
    let bad = Dterm {
        nodes: vec![],
        levels: vec![],
        root: 0,
    };
    assert_eq!(print_term(&bad), Err(PrintError::Malformed));
}

#[test]
fn deep_terms_print_and_parse_without_recursion() {
    // 20,000 nested foralls with distinct names would be slow to name; use arrows (binder `_`).
    let mut cte = String::new();
    for _ in 0..20_000 {
        cte.push_str("Pd(<\"_\">,C<\"T\">[],");
    }
    cte.push_str("C<\"T\">[]");
    for _ in 0..20_000 {
        cte.push(')');
    }
    let t = dt(&cte);
    let text = print_term(&t).unwrap();
    assert_eq!(text.len(), 20_000 * "T -> ".len() + 1);
    assert_eq!(parse_term(&text).unwrap().display().unwrap(), cte);
    // And a deep application spine and deep parentheses.
    let nested = format!("{}#0{}", "(".repeat(20_000), ")".repeat(20_000));
    assert!(parse_term(&nested).is_ok());
}

#[test]
fn the_parser_is_strict_and_total() {
    for bad in [
        "",
        "#",
        "#01",
        "#0 #",
        "forall (a : T) T", // missing comma
        "forall a : T, T",  // no bracket
        "fun (a : T), a",   // comma instead of =>
        "fun (a : T) => ",
        "let a : T := v T", // missing ;
        "(a",
        "a)",
        "a ->",
        "Sort",
        "Sort max(1)",
        "Sort max(1, 2",
        "Sort 1+",
        "Sort u+x",
        "f.{",
        "f.{1",
        "f.{1 2}",
        "f.{}",
        "forall (forall : T), T", // keyword as a name
        "x y forall",
        "T -> forall",
        "\"str\"",
        "a\tb",
        "a\nb",
        "\u{e9}",
        "a $ b",
        "forall {|a : T}, a",
        "#4294967295",
        "#99999999999",
        "Sort 99999999999999999999",
        "Sort 2000000",
        "1",
    ] {
        assert!(parse_term(bad).is_err(), "{bad:?} must be refused");
    }
    // Tokenizer corner cases.
    assert_eq!(tokenize("sha256:ab").unwrap().len(), 3); // short of 64 hex digits: a name, a colon, a name
    assert_eq!(tokenize("a b").unwrap().len(), 2);
    assert!(matches!(
        tokenize("\"unterminated"),
        Err(PpError::Syntax(..))
    ));
    assert!(matches!(tokenize("\"\\q\""), Err(PpError::Syntax(..))));
    assert!(matches!(tokenize("\"\\u{41}\""), Err(PpError::Syntax(..))));
    assert!(matches!(tokenize("\"\\u{}\""), Err(PpError::Syntax(..))));
    assert!(matches!(tokenize("\"\u{e9}\""), Err(PpError::Syntax(..))));
    assert!(tokenize("\"a\\u{20}b\\u{e9}\\\"\\\\\"").is_ok());
    // A term must be the whole text.
    assert!(parse_term("a b )").is_err());
}

// ---- files --------------------------------------------------------------------------------

#[test]
fn the_prelude_round_trips_and_agrees_with_the_kernel() {
    let p = prelude();
    let text = std::str::from_utf8(&p).unwrap();
    let model = decode(text).unwrap();
    let printed = print_file(&model).unwrap();
    assert_eq!(parse_file(&printed).unwrap(), model);
    assert_eq!(
        print_file(&model).unwrap(),
        printed,
        "printing is deterministic"
    );
    // The printed form is what a person reads.
    assert!(printed.starts_with("pifpp 1\nprofile native \"pif-prelude\\u{20}0.1\"\nfeatures\n"));
    assert!(printed.contains("inductive 0 1\n  type Bool : Sort 1\n    ctor Bool.false : Bool\n    ctor Bool.true : Bool\n"), "{printed}");
    assert!(printed.contains("def Pif.not : forall (a : Bool), Bool := fun (b : Bool) => Bool.rec.{1} (fun (_ : Bool) => Bool) Bool.true Bool.false #0 @regular 1\n"), "{printed}");
    assert!(
        printed.contains("inductive.{u_1} 2 1\n  type Eq : forall {alpha : Sort u_1}"),
        "{printed}"
    );
    // The gate-side check, against the kernel's environment.
    let l = prelude_loaded();
    let again = check_reviewed_file(&p, &l.env, &l.files[0].decls).unwrap();
    assert_eq!(again, printed);
    // The model has the expected shape.
    assert!(model.decls.iter().any(|d| matches!(d, DeclModel::Def { name, hint: HintModel::Regular(_), .. } if *name == n("Pif.and"))));
    assert!(model.features.is_empty() && model.imports.is_empty());
}

#[test]
fn the_worked_examples_definitions_round_trip_and_its_proof_is_refused_for_shadowing() {
    // The worked example's file has nested `a` binders in the proof (the motive of the
    // case split sits inside `fun a`), both used: the printer refuses the whole file.
    let model = decode(WORKED).unwrap();
    assert_eq!(model.profile.0, "native");
    match print_file(&model) {
        Err(ModelError::PrintIn(who, PrintError::Shadowed(name))) => {
            assert_eq!((who.as_str(), name.as_str()), ("Pif.and_comm", "a"));
        }
        other => panic!("expected a shadowing refusal, got {other:?}"),
    }
    // Its definitions (Bool, Eq, Pif.and) and its goal are printable and round trip.
    let mut defs = model.clone();
    defs.decls
        .retain(|d| !matches!(d, DeclModel::Theorem { .. } | DeclModel::Goal { .. }));
    assert_eq!(defs.decls.len(), 3);
    let printed = print_file(&defs).unwrap();
    assert_eq!(parse_file(&printed).unwrap(), defs);
    // The goal record round-trips as a declaration too.
    let mut goal = model.clone();
    goal.decls.retain(|d| matches!(d, DeclModel::Goal { .. }));
    let printed = print_file(&goal).unwrap();
    assert!(printed.contains(&format!(
        "goal \"gates.and_comm\" Pif.and_comm sha256:{WORKED_DIGEST}\n"
    )));
    assert_eq!(parse_file(&printed).unwrap(), goal);
}

#[test]
fn a_shadowing_file_is_refused_by_the_gate() {
    // A model file whose proof shadows a used binder: the chain is refused with the reason.
    let p = prelude();
    let c = claim_over_prelude(
        &[Digest::of(&p).hex()],
        "gates.and_comm",
        WORKED_DIGEST,
        "Pif.and_comm",
    );
    let m = mem(&[&p, c.as_bytes()]);
    let chain = [entry(Role::Prelude, &p), entry(Role::Model, c.as_bytes())];
    let e = load_chain(&Resolver::new(&m), &chain, &LoadOptions::default())
        .err()
        .unwrap();
    let CheckError::Printer { subject, why } = e else {
        panic!("{e:?}")
    };
    assert_eq!(subject, "file 1 (model)");
    assert!(why.contains("Shadowed"), "{why}");
}

#[test]
fn the_decoded_file_must_agree_with_the_kernel() {
    let l = prelude_loaded();
    let p = prelude();
    let model = decode(std::str::from_utf8(&p).unwrap()).unwrap();
    let acc = &l.files[0].decls;
    assert_eq!(agrees_with_kernel(&model, &l.env, acc), Ok(()));
    // A tampered type.
    let mut bad = model.clone();
    for d in &mut bad.decls {
        if let DeclModel::Def { name, ty, .. } = d {
            if *name == n("Pif.and") {
                *ty = dt("C<\"Bool\">[]");
            }
        }
    }
    assert!(matches!(
        agrees_with_kernel(&bad, &l.env, acc),
        Err(PrintCheckError::KernelDisagrees(_))
    ));
    // A tampered value.
    let mut bad = model.clone();
    for d in &mut bad.decls {
        if let DeclModel::Def { name, value, .. } = d {
            if *name == n("Pif.and") {
                *value = dt("C<\"Bool\",\"true\">[]");
            }
        }
    }
    assert!(matches!(
        agrees_with_kernel(&bad, &l.env, acc),
        Err(PrintCheckError::KernelDisagrees(_))
    ));
    // A tampered hint, kind, parameter list, constructor list.
    let mut bad = model.clone();
    for d in &mut bad.decls {
        if let DeclModel::Def { hint, .. } = d {
            *hint = HintModel::Abbrev;
        }
    }
    assert!(agrees_with_kernel(&bad, &l.env, acc).is_err());
    let mut bad = model.clone();
    bad.decls.push(DeclModel::Axiom {
        name: n("Bool"),
        params: vec![],
        ty: dt("Sz"),
    });
    assert!(agrees_with_kernel(&bad, &l.env, acc).is_err());
    let mut bad = model.clone();
    for d in &mut bad.decls {
        if let DeclModel::Inductive { types, .. } = d {
            types[0].ctors.pop();
        }
    }
    assert!(agrees_with_kernel(&bad, &l.env, acc).is_err());
    let mut bad = model.clone();
    for d in &mut bad.decls {
        if let DeclModel::Inductive { num_params, .. } = d {
            *num_params += 1;
        }
    }
    assert!(agrees_with_kernel(&bad, &l.env, acc).is_err());
    let mut bad = model.clone();
    for d in &mut bad.decls {
        if let DeclModel::Def { params, .. } = d {
            params.push(n("u"));
        }
    }
    assert!(agrees_with_kernel(&bad, &l.env, acc).is_err());
    // A declaration the kernel never saw, and one the file omits.
    let mut bad = model.clone();
    bad.decls.push(DeclModel::Axiom {
        name: n("No.such"),
        params: vec![],
        ty: dt("Sz"),
    });
    assert!(agrees_with_kernel(&bad, &l.env, acc).is_err());
    let mut bad = model.clone();
    bad.decls.pop();
    assert!(agrees_with_kernel(&bad, &l.env, acc).is_err());
    // Not UTF-8 and not PIF at all.
    assert!(matches!(
        check_reviewed_file(&[0xff], &l.env, acc),
        Err(PrintCheckError::Model(_))
    ));
    assert!(matches!(
        check_reviewed_file(b"hello\n", &l.env, acc),
        Err(PrintCheckError::Model(_))
    ));
}

#[test]
fn printed_files_that_are_not_what_the_printer_writes_are_refused() {
    let model = decode(WORKED).unwrap();
    let mut defs = model.clone();
    defs.decls
        .retain(|d| !matches!(d, DeclModel::Theorem { .. } | DeclModel::Goal { .. }));
    let good = print_file(&defs).unwrap();
    assert!(parse_file(&good).is_ok());
    for bad in [
        good.trim_end().to_string(), // no final LF
        good.replacen("pifpp 1", "pifpp 2", 1),
        good.replacen("pifpp 1", "other 1", 1),
        good.replacen("profile native", "profile", 1),
        good.replacen("features", "features 1", 1),
        good.replacen("\ndef ", "\ndef\n", 1),
        good.replacen("inductive 0 1", "inductive 0 x", 1),
        good.replacen("  type Bool", "  typo Bool", 1),
        good.replacen("ctor Bool.false : Bool", "ctor Bool.false Bool", 1),
        good.replacen("@regular 1", "@regular", 1),
        good.replacen("@regular 1", "@regular 1 2", 1),
        good.replacen("@regular 1", "@fixed", 1),
        good.replacen(" := ", " = ", 1),
        format!("{good}wibble x\n"),
        format!("{good}axiom X : Bool )\n"),
        format!("{good}goal \"c\" X sha256:00\n"),
        format!("{good}goal \"c\" X\n"),
        format!("{good}import sha256:{}\n", Digest::of(b"x").hex()), // an import after a declaration
    ] {
        assert!(parse_file(&bad).is_err(), "{bad:?}");
    }
    // Imports and features print and parse.
    let mut m = defs.clone();
    m.imports.push(Digest::of(b"a").hex());
    m.features.push("lits".into());
    let printed = print_file(&m).unwrap();
    assert!(printed.contains("features lits\nimport sha256:"));
    assert_eq!(parse_file(&printed).unwrap(), m);
    let bad_import = printed.replacen("import sha256:", "import sha256:zz", 1);
    assert!(parse_file(&bad_import).is_err());
}

#[test]
fn the_decoder_refuses_what_it_cannot_decode() {
    let good = WORKED.to_string();
    assert!(decode(&good).is_ok());
    let changed = |from: &str, to: &str| {
        assert!(good.contains(from), "the fixture has no {from:?}");
        good.replacen(from, to, 1)
    };
    for bad in [
        good.trim_end().to_string(),
        changed("pif 1", "pif 2"),
        changed("logic 1", "logic 2"),
        changed("nstr 2 1 \"false\"", "nstr 3 1 \"false\""), // not dense
        changed("lsucc 1 0", "lsucc 2 0"),
        changed("esort 1 1", "esort 2 1"),
        changed("\nend 97", "\nend 97\nextra"),
        changed("profile ", "profil "),
        changed("features", "feature"),
        changed("nstr 1 0 \"Bool\"", "nstr 1 9 \"Bool\""),
        changed("nstr 1 0 \"Bool\"", "nstr 1 0 Bool"),
        changed("nstr 1 0 \"Bool\"", "nstr 1 0 \"Bo\\x\""),
        changed("nstr 1 0 \"Bool\"", "nstr 1 0 \"Bo\\u{zz}\""),
        changed("nstr 1 0 \"Bool\"", "nstr 1 0 \"Bool\" x"),
        changed("lparam 2 6", "lparam 2 600"),
        changed("esort 1 1", "wibble 1 1"),
        changed("esort 1 1", "esort 1 01"),
        changed("econst 2 1 []", "econst 2 1 ["),
        changed("epi 7 default 9 6 4", "epi 7 dflt 9 6 4"),
        changed("def 11 [] 27 25 @regular 1", "def 11 [] 27 25 @regular"),
        changed("def 11 [] 27 25 @regular 1", "def 11 [] 27 25 @wrong"),
        changed("itype 1 1 2", "itype 1 1 3"),
        changed("inductive [] 0 1", "inductive [] 0 2"),
        changed("ictor 2 2", "ictop 2 2"),
        changed("goal \"gates.and_comm\" 14", "goal \"gates.and_comm\" 99"),
        changed("sha256:bf1f", "sha256:BF1F"),
        changed("econst 10 4 [2]", "econst 10 4 [2 3]"),
    ] {
        assert!(decode(&bad).is_err(), "{}", &bad[..bad.len().min(120)]);
    }
    // Literals, projections, quotients and prims are named as unsupported.
    let lit = changed("esort 1 1", "enat 1 5");
    assert_eq!(
        decode(&lit).err(),
        Some(ModelError::Unsupported("literal or projection"))
    );
    assert_eq!(
        decode(&changed("esort 1 1", "quot")).err(),
        Some(ModelError::Unsupported("quot or prim"))
    );
    assert_eq!(
        decode(&changed("esort 1 1", "prim x 1")).err(),
        Some(ModelError::Unsupported("quot or prim"))
    );
    let w = decode(WORKED).unwrap();
    assert_eq!(w.decls.len(), 5);
}

// ---- G7 ------------------------------------------------------------------------------------

#[test]
fn g7_rejects_homoglyphs_binders_named_like_constants_and_non_ascii_parameters() {
    let l = prelude_loaded();
    let check = |cte: &str, binders: bool| check_goal_names(&dt(cte), &l.env, binders);
    let ok = "Pd(<\"x\">,C<\"Bool\">[],C<\"Bool\">[])";
    assert_eq!(check(ok, true), Ok(()));
    // A Cyrillic `a` (U+0430) next to a Latin `a`: two binders that look alike.
    let homoglyph = "Pd(<\"a\">,C<\"Bool\">[],Pd(<\"\\u{430}\">,C<\"Bool\">[],v0))";
    let v = check(homoglyph, true).unwrap_err();
    assert_eq!((v.rule, v.name.as_str()), (NameRule::NotAscii, "\\u{430}"));
    // ... in a constant, too (the constants-only mode catches it with no display CTE).
    let v = check("C<\"B\\u{430}ol\">[]", false).unwrap_err();
    assert_eq!(v.rule, NameRule::NotAscii);
    // A zero-width character inside a name.
    assert_eq!(
        check("C<\"E\\u{200b}q\">[]", false).unwrap_err().rule,
        NameRule::NotAscii
    );
    // A binder named like an existing constant.
    let v = check("Pd(<\"Eq\">,C<\"Bool\">[],C<\"Bool\">[])", true).unwrap_err();
    assert_eq!(
        (v.rule, v.name.as_str()),
        (NameRule::BinderIsConstant, "Eq")
    );
    // A binder named `Bool.false`: two components, or one component with a dot.
    let v = check("Pd(<\"Bool\",\"false\">,C<\"Bool\">[],C<\"Bool\">[])", true).unwrap_err();
    assert_eq!(v.rule, NameRule::BinderNotSingle);
    let v = check("Pd(<\"Bool.false\">,C<\"Bool\">[],C<\"Bool\">[])", true).unwrap_err();
    assert_eq!(v.rule, NameRule::NotAscii);
    // A non-ASCII universe parameter name.
    let v = check("Sp<\"\\u{3b1}\">", false).unwrap_err();
    assert_eq!(v.rule, NameRule::NotAscii);
    assert_eq!(check("Sp<\"u\">", false), Ok(()));
    // Duplicate binder names, even in separate branches (the literal reading of G7).
    let dup = "A(Pd(<\"n\">,C<\"Bool\">[],v0),Pd(<\"n\">,C<\"Bool\">[],v0))";
    assert_eq!(
        check(dup, true).unwrap_err().rule,
        NameRule::DuplicateBinder
    );
    // `_` is exempt from distinctness (an arrow is a binder named `_`).
    assert_eq!(
        check(
            "Pd(<\"_\">,C<\"Bool\">[],Pd(<\"_\">,C<\"Bool\">[],C<\"Bool\">[]))",
            true
        ),
        Ok(())
    );
    // Numeric components and empty names have no ASCII spelling.
    assert_eq!(
        check("C<\"A\",3>[]", false).unwrap_err().rule,
        NameRule::NumericOrEmpty
    );
    assert_eq!(
        check("C<>[]", false).unwrap_err().rule,
        NameRule::NumericOrEmpty
    );
    assert_eq!(
        check("Pd(<>,C<\"Bool\">[],C<\"Bool\">[])", true)
            .unwrap_err()
            .rule,
        NameRule::NumericOrEmpty
    );
    // Let and lambda binders follow the same rules.
    assert_eq!(
        check("Ld(<\"Eq\">,C<\"Bool\">[],v0)", true)
            .unwrap_err()
            .rule,
        NameRule::BinderIsConstant
    );
    assert_eq!(
        check("Z(<\"Eq\">,C<\"Bool\">[],v0,v0)", true)
            .unwrap_err()
            .rule,
        NameRule::BinderIsConstant
    );
    // Binder rules are skipped in constants-only mode (the store's names are arbitrary).
    assert_eq!(
        check("Pd(<\"Eq\">,C<\"Bool\">[],C<\"Bool\">[])", false),
        Ok(())
    );
    // Every rule has a distinct word in the report.
    let words: std::collections::BTreeSet<_> = [
        NameRule::NotAscii,
        NameRule::NumericOrEmpty,
        NameRule::BinderNotSingle,
        NameRule::DuplicateBinder,
        NameRule::BinderIsConstant,
    ]
    .iter()
    .map(|r| r.word())
    .collect();
    assert_eq!(words.len(), 5);
}

#[test]
fn g7_and_the_round_trip_are_enforced_on_the_goals_list() {
    let p = prelude();
    let c = claim_over_prelude(
        &[Digest::of(&p).hex()],
        "gates.and_comm",
        WORKED_DIGEST,
        "Pif.and_comm",
    );
    let m = mem(&[&p, c.as_bytes()]);
    let chain = [entry(Role::Prelude, &p), entry(Role::Claim, c.as_bytes())];
    let l = load_chain(&Resolver::new(&m), &chain, &LoadOptions::default()).unwrap();
    let ty = l.env.get(&n("Pif.and_comm")).unwrap().ty;
    let display = display_cte_from_store(&l.env.store, ty).unwrap();
    let run = |d: &str| {
        let list = parse_goals_list(&format!("gates.and_comm {d}\n")).unwrap();
        check_goals(&l, Some(&list), None)
    };
    assert!(run(&display).is_ok());
    // A Cyrillic binder name erases to the same binding CTE, but the signer would be misled.
    let evil = display.replace("<\"a\">", "<\"\\u{430}\">");
    let e = run(&evil).err().unwrap();
    assert_eq!(
        e,
        CheckError::GoalNames {
            claim: "gates.and_comm".into(),
            rule: "not-ascii",
            name: "\\u{430}".into()
        }
    );
    // A binder called `Eq`.
    let e = run(&display.replace("<\"a\">", "<\"Eq\">")).err().unwrap();
    assert_eq!(e.code(), "GoalNames");
    // Two binders with one name: a shadowing goal, refused by G7 before the printer.
    let e = run(&display.replace("<\"b\">", "<\"a\">")).err().unwrap();
    assert_eq!(
        e,
        CheckError::GoalNames {
            claim: "gates.and_comm".into(),
            rule: "duplicate-binder",
            name: "a".into()
        }
    );
    // Without a goals list the constants and parameters of the stored term are still checked.
    assert!(check_goals(&l, None, None).is_ok());
}

#[test]
fn a_goal_the_printer_cannot_print_is_refused_by_the_gate() {
    // The binding CTE of `Sort (max u 1)` is fine, but a binder named `fun` is a printed
    // keyword: G7 accepts the name, the printer refuses it.
    let l = prelude_loaded();
    let g = dt("Pd(<\"fun\">,C<\"Bool\">[],C<\"Bool\">[])");
    assert_eq!(check_goal_names(&g, &l.env, true), Ok(()));
    assert_eq!(
        check_goal(&g),
        Err(PrintCheckError::Print(PrintError::ReservedWord(
            "fun".into()
        )))
    );
    // A round trip failure is reported as such: here a tree the printer prints but whose
    // text does not parse back to it (a level numeral too large for the parser).
    let big = Dterm {
        nodes: vec![Node::Sort(1)],
        levels: vec![Lv::Zero, Lv::Param(n("u"))],
        root: 0,
    };
    assert!(check_goal(&big).is_ok());
}

// ---- the pif-pp command line ------------------------------------------------------------------

#[test]
fn the_pif_pp_command_line_prints_checks_and_refuses() {
    use pif_check::pp_cli::run;
    let s = |v: &[&str]| v.iter().map(|x| (*x).to_string()).collect::<Vec<_>>();
    let t = Tmp::new();
    let path = t.put("prelude.pif", &prelude());
    let out = run(&s(&["file", &path]));
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert!(out.stdout.starts_with("pifpp 1\n"));
    assert_eq!(run(&s(&["file", &path])), out, "deterministic");
    let out = run(&s(&["check", &path]));
    assert_eq!((out.code, out.stdout.as_str()), (0, "ok\n"));
    // A file with a shadowed proof is refused, with the reason on standard error.
    let worked = t.put("worked.pif", WORKED.as_bytes());
    let out = run(&s(&["file", &worked]));
    assert_eq!(out.code, 1);
    assert!(out.stderr.contains("Shadowed"), "{}", out.stderr);
    // A goal.
    let g = display_goal();
    let out = run(&s(&["goal", &g]));
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert_eq!(
        out.stdout,
        "forall (a : Bool), forall (b : Bool), Eq.{1} Bool (Pif.and #1 #0) (Pif.and #0 #1)\n"
    );
    assert_eq!(run(&s(&["goal", "not a cte"])).code, 1);
    assert_eq!(run(&s(&["goal", "C<\"\\u{430}\">[]"])).code, 1);
    // Unreadable and undecodable inputs, and usage errors.
    assert_eq!(run(&s(&["file", &t.path("missing.pif")])).code, 1);
    let junk = t.put("junk.pif", b"hello\n");
    assert_eq!(run(&s(&["file", &junk])).code, 1);
    assert_eq!(run(&s(&[])).code, 2);
    assert_eq!(run(&s(&["file"])).code, 2);
    assert_eq!(run(&s(&["frob", "x"])).code, 2);
    // The binary.
    let bin = env!("CARGO_BIN_EXE_pif-pp");
    let o = std::process::Command::new(bin)
        .args(["check", &path])
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(0));
    assert_eq!(String::from_utf8_lossy(&o.stdout), "ok\n");
    let o = std::process::Command::new(bin).output().unwrap();
    assert_eq!(o.status.code(), Some(2));
}

fn display_goal() -> String {
    worked_goal().display().unwrap()
}
