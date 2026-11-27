//! PIF-003d: the remaining corners of the printer, the decoder and the goal checks, driven by a
//! "kitchen sink" model file that uses every declaration kind and term form the reader accepts.

mod common;

use common::*;
use lean_kernel::Name;
use pif_check::chain::{load_chain, LoadOptions, Loaded};
use pif_check::claims::claim_reports;
use pif_check::control::Role;
use pif_check::cte::{display_cte_from_store, goal_digest, Dterm};
use pif_check::filemodel::{decode, DeclModel, FileModel, HintModel, IndModel, ModelError};
use pif_check::filetext::{parse_file, print_file};
use pif_check::footprint::footprint;
use pif_check::goals::{check_goals, constants_of, parse_goals_list};
use pif_check::printcheck::{agrees_with_kernel, check_reviewed_file, PrintCheckError};
use pif_check::printer::{print_term, PrintError};
use pif_check::report::dotted;
use pif_check::resolver::Resolver;
use pif_check::{CheckError, Digest};
use pif_writer::{BinderInfo::Default as D, Hint, Level, Writer};

fn n(s: &str) -> Name {
    Name::from_dotted(s)
}

/// A model file over the prelude using: abbrev/opaque hints, an `opaque` declaration, universe
/// parameters with `imax`, a `let`, an axiom, and a profile label with quotes and a backslash.
fn sink(prelude_hex: &str) -> String {
    let mut w = Writer::new("native", "sink \"q\" \\ 0.1")
        .unwrap()
        .ascii_names_only();
    w.import(prelude_hex).unwrap();
    let (b, tt, ff) = (
        w.name("Bool").unwrap(),
        w.name("Bool.true").unwrap(),
        w.name("Bool.false").unwrap(),
    );
    let (bc, ttc, ffc) = (w.constant(b, &[]), w.constant(tt, &[]), w.constant(ff, &[]));
    let k1 = w.name("Pif.k1").unwrap();
    w.definition(k1, &[], bc, ttc, Hint::Abbrev).unwrap();
    let k2 = w.name("Pif.k2").unwrap();
    w.definition(k2, &[], bc, ffc, Hint::Opaque).unwrap();
    let k1c = w.constant(k1, &[]);
    let k3 = w.name("Pif.k3").unwrap();
    w.opaque(k3, &[], bc, k1c).unwrap();
    // def Pif.idu.{u} : forall (A : Sort u) (x : A), A := fun A x => x
    let (u, a, x, anon) = (
        w.name("u").unwrap(),
        w.name("A").unwrap(),
        w.name("x").unwrap(),
        w.name("_").unwrap(),
    );
    let lu = w.level_param(u);
    let sort_u = w.sort(lu);
    let (v0, v1) = (w.var(0), w.var(1));
    let inner_ty = w.pi(D, anon, v0, v1);
    let ty = w.pi(D, a, sort_u, inner_ty);
    let inner_val = w.lam(D, x, v0, v0);
    let val = w.lam(D, a, sort_u, inner_val);
    let idu = w.name("Pif.idu").unwrap();
    w.definition(idu, &[u], ty, val, Hint::Regular(3)).unwrap();
    // axiom Pif.ax5.{u} : Sort (imax u 1)
    let one = w.level_succ(Level::ZERO);
    let im = w.level_imax(lu, one);
    let sort_im = w.sort(im);
    let ax5 = w.name("Pif.ax5").unwrap();
    w.axiom(ax5, &[u], sort_im).unwrap();
    // def Pif.k6 : Bool := let y : Bool := Bool.true; y
    let y = w.name("y").unwrap();
    let body = w.let_(y, bc, ttc, v0);
    let k6 = w.name("Pif.k6").unwrap();
    w.definition(k6, &[], bc, body, Hint::Regular(1)).unwrap();
    w.to_text()
}

struct Sink {
    prelude: Vec<u8>,
    model: String,
}

fn sink_world() -> Sink {
    let prelude = prelude();
    let model = sink(&Digest::of(&prelude).hex());
    Sink { prelude, model }
}

fn load(s: &Sink, extra: &[(Role, &[u8])]) -> Result<Loaded, CheckError> {
    let mut files: Vec<&[u8]> = vec![&s.prelude, s.model.as_bytes()];
    let mut chain = vec![
        entry(Role::Prelude, &s.prelude),
        entry(Role::Model, s.model.as_bytes()),
    ];
    for (r, b) in extra {
        files.push(b);
        chain.push(entry(*r, b));
    }
    let m = mem(&files);
    load_chain(&Resolver::new(&m), &chain, &LoadOptions::default())
}

#[test]
fn the_kitchen_sink_file_round_trips_and_agrees_with_the_kernel() {
    let s = sink_world();
    let l = load(&s, &[]).unwrap();
    let model = decode(&s.model).unwrap();
    assert_eq!(model.profile.1, "sink \"q\" \\ 0.1");
    let printed = print_file(&model).unwrap();
    assert!(
        printed.contains("def Pif.k1 : Bool := Bool.true @abbrev\n"),
        "{printed}"
    );
    assert!(
        printed.contains("def Pif.k2 : Bool := Bool.false @opaque\n"),
        "{printed}"
    );
    assert!(
        printed.contains("opaque Pif.k3 : Bool := Pif.k1\n"),
        "{printed}"
    );
    assert!(
        printed.contains("def Pif.idu.{u} : forall (A : Sort u), #0 -> #1 := "),
        "{printed}"
    );
    assert!(
        printed.contains("axiom Pif.ax5.{u} : Sort imax(u, 1)\n"),
        "{printed}"
    );
    assert!(
        printed.contains("let y : Bool := Bool.true; #0"),
        "{printed}"
    );
    assert!(
        printed.contains("profile native \"sink\\u{20}\\\"q\\\"\\u{20}\\\\\\u{20}0.1\"\n"),
        "{printed}"
    );
    assert_eq!(parse_file(&printed).unwrap(), model);
    let accepted = &l.files[1].decls;
    assert_eq!(agrees_with_kernel(&model, &l.env, accepted), Ok(()));
    assert_eq!(
        check_reviewed_file(s.model.as_bytes(), &l.env, accepted).unwrap(),
        printed
    );
}

#[test]
fn footprints_and_goal_constants_see_through_let_terms() {
    let s = sink_world();
    let l = load(&s, &[]).unwrap();
    assert!(footprint(&l, &n("Pif.k6")).unwrap().is_empty());
    assert_eq!(footprint(&l, &n("Pif.ax5")).unwrap(), [n("Pif.ax5")]);
    let k6 = l.env.get(&n("Pif.k6")).unwrap();
    let value = k6.value().unwrap();
    assert!(constants_of(&l.env.store, value).contains(&n("Bool.true")));
    // A claim whose goal contains a `let`: the gate, the footprint and the printer all handle it.
    let (ph, mh) = (
        Digest::of(&s.prelude).hex(),
        Digest::of(s.model.as_bytes()).hex(),
    );
    let claim = |hint: &str| {
        let mut w = Writer::new("native", "claim 0.1")
            .unwrap()
            .ascii_names_only();
        w.import(&ph).unwrap();
        w.import(&mh).unwrap();
        let (b, tt) = (w.name("Bool").unwrap(), w.name("Bool.true").unwrap());
        let (eq, refl, y) = (
            w.name("Eq").unwrap(),
            w.name("Eq.refl").unwrap(),
            w.name("y").unwrap(),
        );
        let one = w.level_succ(Level::ZERO);
        let (bc, ttc) = (w.constant(b, &[]), w.constant(tt, &[]));
        let v0 = w.var(0);
        let lettm = w.let_(y, bc, ttc, v0);
        let (eq1, refl1) = (w.constant(eq, &[one]), w.constant(refl, &[one]));
        let ty = w.app_many(eq1, &[bc, lettm, ttc]);
        let pf = w.app_many(refl1, &[bc, ttc]);
        let t = w.name("Pif.tlet").unwrap();
        w.theorem(t, &[], ty, pf).unwrap();
        w.goal("c.let", t, hint).unwrap();
        w.to_text()
    };
    let first = claim(&Digest::of(b"x").hex());
    let l1 = load(&s, &[(Role::Claim, first.as_bytes())]).unwrap();
    let ty = l1.env.get(&n("Pif.tlet")).unwrap().ty;
    let hint = goal_digest(&l1.env.store, ty).unwrap().hex();
    let second = claim(&hint);
    let l2 = load(&s, &[(Role::Claim, second.as_bytes())]).unwrap();
    let ty = l2.env.get(&n("Pif.tlet")).unwrap().ty;
    let list = parse_goals_list(&format!(
        "c.let {}\n",
        display_cte_from_store(&l2.env.store, ty).unwrap()
    ))
    .unwrap();
    let goals = check_goals(&l2, Some(&list), None).unwrap();
    let reports = claim_reports(&l2, &goals, None).unwrap();
    assert!(reports[0].footprint.is_empty());
    // The printed goal shows the let.
    let text = print_term(&list[0].term).unwrap();
    assert!(text.contains("let y : Bool := Bool.true; #0"), "{text}");
}

#[test]
fn decode_handles_features_imports_and_numeric_names_and_the_printer_refuses_the_latter() {
    let mut w = Writer::new("native", "feat").unwrap().ascii_names_only();
    w.feature("lits").unwrap();
    w.import(&Digest::of(b"x").hex()).unwrap();
    let t = w.name("T").unwrap();
    let num = w.name_num(t, 7).unwrap();
    let sort = w.sort(Level::ZERO);
    w.axiom(t, &[], sort).unwrap();
    let txt = w.to_text();
    let model = decode(&txt).unwrap();
    assert_eq!(model.features, ["lits"]);
    assert_eq!(model.imports, [Digest::of(b"x").hex()]);
    // Features and imports print and come back.
    assert_eq!(parse_file(&print_file(&model).unwrap()).unwrap(), model);
    // A declaration named with a numeric component has no printed spelling.
    let mut with_num = model.clone();
    let ty = Dterm::parse("Sz").unwrap();
    let numbered = Name::num(&n("T"), 7);
    let _ = num;
    with_num.decls.push(DeclModel::Axiom {
        name: numbered,
        params: vec![],
        ty,
    });
    assert!(matches!(
        print_file(&with_num),
        Err(ModelError::PrintIn(_, PrintError::NameNotPrintable))
    ));
    // A numeric name inside the file decodes to a numeric component.
    let w2 = {
        let mut w = Writer::new("native", "numfile").unwrap();
        let t = w.name("T").unwrap();
        let t7 = w.name_num(t, 7).unwrap();
        let sort = w.sort(Level::ZERO);
        w.axiom(t7, &[], sort).unwrap();
        w.to_text()
    };
    let m2 = decode(&w2).unwrap();
    let DeclModel::Axiom { name, .. } = &m2.decls[0] else {
        panic!()
    };
    assert_eq!(*name, Name::num(&n("T"), 7));
    assert_eq!(dotted(name), "T.7");
}

#[test]
fn print_errors_name_the_declaration_in_every_kind() {
    let bad = n("fun"); // a keyword of the printed syntax
    let ty = Dterm::parse("Sz").unwrap();
    let decls = [
        DeclModel::Axiom {
            name: bad.clone(),
            params: vec![],
            ty: ty.clone(),
        },
        DeclModel::Def {
            name: bad.clone(),
            params: vec![],
            ty: ty.clone(),
            value: ty.clone(),
            hint: HintModel::Abbrev,
        },
        DeclModel::Theorem {
            name: bad.clone(),
            params: vec![],
            ty: ty.clone(),
            value: ty.clone(),
        },
        DeclModel::Opaque {
            name: bad.clone(),
            params: vec![],
            ty: ty.clone(),
            value: ty.clone(),
        },
        DeclModel::Inductive {
            params: vec![],
            num_params: 0,
            types: vec![IndModel {
                name: bad.clone(),
                ty: ty.clone(),
                ctors: vec![],
            }],
        },
        DeclModel::Goal {
            claim: "c".into(),
            theorem: bad.clone(),
            digest: Digest::of(b"x").hex(),
        },
    ];
    for d in decls {
        let m = FileModel {
            profile: ("native".into(), "x".into()),
            features: vec![],
            imports: vec![],
            decls: vec![d],
        };
        assert!(
            matches!(
                print_file(&m),
                Err(ModelError::PrintIn(_, PrintError::ReservedWord(_)))
            ),
            "{m:?}"
        );
    }
    // Universe parameters with an unprintable name.
    let m = FileModel {
        profile: ("native".into(), "x".into()),
        features: vec![],
        imports: vec![],
        decls: vec![DeclModel::Axiom {
            name: n("A"),
            params: vec![n("\u{3b1}")],
            ty: ty.clone(),
        }],
    };
    assert!(matches!(
        print_file(&m),
        Err(ModelError::PrintIn(_, PrintError::NameNotPrintable))
    ));
    let m = FileModel {
        profile: ("native".into(), "x".into()),
        features: vec![],
        imports: vec![],
        decls: vec![DeclModel::Inductive {
            params: vec![n("\u{3b1}")],
            num_params: 0,
            types: vec![],
        }],
    };
    assert!(print_file(&m).is_err());
    // A model with no declarations prints and parses.
    let empty = FileModel {
        profile: ("native".into(), "e".into()),
        features: vec![],
        imports: vec![],
        decls: vec![],
    };
    assert_eq!(parse_file(&print_file(&empty).unwrap()).unwrap(), empty);
}

#[test]
fn more_malformed_printed_files_are_refused() {
    let s = sink_world();
    let good = print_file(&decode(&s.model).unwrap()).unwrap();
    assert!(parse_file(&good).is_ok());
    let changed = |from: &str, to: &str| {
        assert!(good.contains(from), "no {from:?} in\n{good}");
        good.replacen(from, to, 1)
    };
    for bad in [
        changed("profile native", "profile native extra"),
        changed("\nfeatures", "\n42 x\nfeatures"),
        format!("{good}42 x\n"),
        changed("@regular 3", "@regular x"),
        changed("@regular 3", "@regular 99999999999"),
        changed("opaque Pif.k3 : Bool := Pif.k1", "opaque Pif.k3 : Bool"),
        changed("axiom Pif.ax5", "axiom Pif.ax5 extra"),
        format!("{good}inductive 0 1 2\n"),
        format!("{good}inductive 0 5000\n"),
        format!("{good}inductive 0 1\n  type T : Sort 1 )\n"),
        format!("{good}inductive 0 1\n  type T : Sort 1\n    ctor T.c : T )\n"),
        format!("{good}inductive 0 1\n  type T : Sort 1\n    ctor T.c T\n"),
        format!(
            "{good}goal \"c\" X sha256:{} extra\n",
            Digest::of(b"x").hex()
        ),
        format!("{good}goal c X sha256:{}\n", Digest::of(b"x").hex()),
        changed("profile native \"", "profile native x\""),
        changed("pifpp 1", "pifpp 1 2"),
        changed("import sha256:", "import sha256:aa sha256:"),
        changed("import sha256:", "import 5 sha256:"),
    ] {
        assert!(parse_file(&bad).is_err(), "{bad:?}");
    }
    // Strings with escapes in the profile label parse back exactly.
    assert!(parse_file(&good).unwrap().profile.1.contains('"'));
}

#[test]
fn decoder_errors_on_dangling_indices_and_missing_trailers() {
    let fixture = include_str!("../../lean-kernel/tests/fixtures/worked-example.pif");
    let changed = |from: &str, to: &str| {
        assert!(fixture.contains(from), "no {from:?}");
        fixture.replacen(from, to, 1)
    };
    // An expression that mentions an undefined level, name, or expression.
    assert!(decode(&changed("esort 3 2", "esort 3 99")).is_err());
    assert!(decode(&changed("econst 10 4 [2]", "econst 10 99 [2]")).is_err());
    assert!(decode(&changed("eapp 11 10 6", "eapp 11 10 99")).is_err());
    assert!(decode(&changed("lsucc 1 0", "lsucc 1 7")).is_err());
    assert!(decode(&changed("lparam 2 6", "lparam 2 600")).is_err());
    // A file with no `end` line.
    let no_end = fixture.trim_end().rsplit_once('\n').unwrap().0.to_string() + "\n";
    assert_eq!(
        decode(&no_end).err(),
        Some(ModelError::Decode("no end line", 1))
    );
    // A number that is not a u64 in an nnum, and a bad features line.
    let mut w = Writer::new("native", "x").unwrap();
    let t = w.name("T").unwrap();
    let t7 = w.name_num(t, 7).unwrap();
    let sort = w.sort(Level::ZERO);
    w.axiom(t7, &[], sort).unwrap();
    let txt = w.to_text();
    assert!(decode(&txt.replacen("nnum 2 1 7", "nnum 2 1 99999999999999999999", 1)).is_err());
    assert!(decode(&txt.replacen("features", "featuresx", 1)).is_err());
    assert!(
        decode(&txt.replacen("features\n", "features lits\n", 1).replacen(
            "features lits",
            "features",
            1
        ))
        .is_ok()
    );
}

#[test]
fn printcheck_catches_the_wrong_kind_and_the_wrong_inductive() {
    let s = sink_world();
    let l = load(&s, &[]).unwrap();
    let acc = &l.files[1].decls;
    let model = decode(&s.model).unwrap();
    assert_eq!(agrees_with_kernel(&model, &l.env, acc), Ok(()));
    // A definition presented as a theorem (the kernel has it as a definition).
    let mut bad = model.clone();
    for d in &mut bad.decls {
        if let DeclModel::Def {
            name,
            params,
            ty,
            value,
            ..
        } = d.clone()
        {
            if name == n("Pif.k1") {
                *d = DeclModel::Theorem {
                    name,
                    params,
                    ty,
                    value,
                };
            }
        }
    }
    assert!(
        matches!(agrees_with_kernel(&bad, &l.env, acc), Err(PrintCheckError::KernelDisagrees(m)) if m == "declaration kind")
    );
    // An opaque value that differs.
    let mut bad = model.clone();
    for d in &mut bad.decls {
        if let DeclModel::Opaque { value, .. } = d {
            *value = Dterm::parse("C<\"Bool\",\"false\">[]").unwrap();
        }
    }
    assert!(agrees_with_kernel(&bad, &l.env, acc).is_err());
    // An "inductive" that is really a definition in the kernel.
    let mut bad = model.clone();
    bad.decls.push(DeclModel::Inductive {
        params: vec![],
        num_params: 0,
        types: vec![IndModel {
            name: n("Pif.k1"),
            ty: Dterm::parse("C<\"Bool\">[]").unwrap(),
            ctors: vec![],
        }],
    });
    assert!(agrees_with_kernel(&bad, &l.env, acc).is_err());
}

#[test]
fn a_library_files_names_may_be_non_ascii_but_a_goal_may_not_mention_them() {
    // The Library role gets the lenient reader class (it is not offered to the printer); the
    // same file in a reviewed position is refused (R-NAM-3); and a goal that mentions such a
    // name is refused by G7, even with no goals list.
    let s = sink_world();
    let (ph, mh) = (
        Digest::of(&s.prelude).hex(),
        Digest::of(s.model.as_bytes()).hex(),
    );
    let lib = {
        let mut w = Writer::new("native", "lib").unwrap();
        w.import(&ph).unwrap();
        w.import(&mh).unwrap();
        let (b, tt) = (w.name("Bool").unwrap(), w.name("Bool.true").unwrap());
        let (bc, ttc) = (w.constant(b, &[]), w.constant(tt, &[]));
        let c = w.name("B\u{430}d").unwrap(); // a Cyrillic a inside the name
        w.definition(c, &[], bc, ttc, Hint::Regular(1)).unwrap();
        w.to_text()
    };
    let m = mem(&[&s.prelude, s.model.as_bytes(), lib.as_bytes()]);
    let chain = [
        entry(Role::Prelude, &s.prelude),
        entry(Role::Model, s.model.as_bytes()),
        entry(Role::Library, lib.as_bytes()),
    ];
    let l = load_chain(&Resolver::new(&m), &chain, &LoadOptions::default()).unwrap();
    assert!(l.env.contains(&Name::from_dotted("B\u{430}d")));
    let reviewed = [chain[0], chain[1], entry(Role::Axioms, lib.as_bytes())];
    let e = load_chain(&Resolver::new(&m), &reviewed, &LoadOptions::default())
        .err()
        .unwrap();
    assert_eq!(e.code(), "ReadRejected");
    // A goal term that mentions the constant, checked from the store (no display CTE).
    let mut env = lean_kernel::Environment::new();
    let c = env.store.constant(&Name::from_dotted("B\u{430}d"), &[]);
    let t = Dterm::from_store(&env.store, c).unwrap();
    let v = pif_check::names::check_goal_names(&t, &env, false).unwrap_err();
    assert_eq!(v.rule, pif_check::names::NameRule::NotAscii);
}

#[test]
fn a_goal_that_g7_accepts_but_the_printer_refuses_is_refused_by_the_gate() {
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
    // Binder names `fun` and `x`: ASCII, single, distinct, not constants (G7 is satisfied), but
    // `fun` is a keyword of the printed syntax, so the printer refuses.
    let evil = display
        .replace("<\"a\">", "<\"fun\">")
        .replace("<\"b\">", "<\"x\">");
    let list = parse_goals_list(&format!("gates.and_comm {evil}\n")).unwrap();
    let e = check_goals(&l, Some(&list), None).err().unwrap();
    let CheckError::Printer { subject, why } = e else {
        panic!("{e:?}")
    };
    assert_eq!(subject, "claim gates.and_comm");
    assert!(why.contains("ReservedWord"), "{why}");
}

#[test]
fn deep_shadow_and_wide_indices() {
    // A variable index of 64 or more: the loose-variable mask is conservative, never wrong.
    let t = Dterm::parse("Pd(<\"a\">,C<\"T\">[],A(v70,v0))").unwrap();
    assert!(print_term(&t).unwrap().contains("#70"));
    // Shadowing across more than 64 scopes still refuses a used outer binder.
    let mut cte = String::from("Pd(<\"a\">,C<\"T\">[],");
    for k in 0..70 {
        cte.push_str(&format!("Pd(<\"b{k}\">,C<\"T\">[],"));
    }
    cte.push_str("Pd(<\"a\">,C<\"T\">[],v71)");
    for _ in 0..71 {
        cte.push(')');
    }
    let t = Dterm::parse(&cte).unwrap();
    assert_eq!(print_term(&t), Err(PrintError::Shadowed("a".into())));
}
