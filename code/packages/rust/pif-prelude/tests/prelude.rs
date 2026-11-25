//! Tests of the PIF prelude: the committed file is the generator's output, the reader
//! accepts it as a reviewed file with no features and no imports, every declaration has its
//! expected kind, every inductive type matches the Lean oracle, and small theorems check
//! through it.

use coding_adventures_sha256::sha256_hex;
use lean_kernel::pif::{read, FileClass, GateInputs, Kind, PifError, ReadReport};
use lean_kernel::{BinderInfo, DeclKind, Environment, Expr, ExprKind, Level, LevelKind, Name};
use pif_writer::{BinderInfo as B, Hint, Level as L, Writer};

const COMMITTED: &str = include_str!("../../../../specs/data/pif/prelude.pif");
const ORACLE: &str = include_str!("../oracle/expected.sexp");

fn reviewed() -> GateInputs<'static> {
    GateInputs {
        class: FileClass::Reviewed,
        ..GateInputs::default()
    }
}

fn prelude() -> ReadReport {
    read(Environment::new(), COMMITTED.as_bytes(), &reviewed()).expect("the prelude is accepted")
}

fn n(s: &str) -> Name {
    Name::from_dotted(s)
}

#[test]
fn the_committed_file_is_exactly_what_the_generator_emits() {
    assert_eq!(
        COMMITTED,
        pif_prelude::generate(),
        "prelude.pif drifted: run `cargo run -q -p pif-prelude > code/specs/data/pif/prelude.pif`"
    );
    assert_eq!(pif_prelude::generate(), pif_prelude::generate());
}

#[test]
fn the_prelude_has_no_features_no_imports_and_no_axioms() {
    let lines: Vec<&str> = COMMITTED.lines().collect();
    assert_eq!(
        &lines[..4],
        [
            "pif 1",
            "logic 1",
            "profile native \"pif-prelude\\u{20}0.1\"",
            "features"
        ]
    );
    assert!(!COMMITTED.lines().any(|l| l.starts_with("import ")));
    let r = prelude();
    assert!(r.goals.is_empty());
    assert!(r
        .decls
        .iter()
        .all(|d| d.kind != Kind::Axiom && d.kind != Kind::Opaque));
}

#[test]
fn every_declaration_has_its_expected_kind() {
    use Kind::{Constructor as C, Definition as D, Inductive as I, Recursor as R};
    let mut want: Vec<(Kind, String)> = vec![];
    let blocks: [(&str, &[&str]); 13] = [
        ("Bool", &["false", "true"]),
        ("Nat", &["zero", "succ"]),
        ("Int", &["ofNat", "negSucc"]),
        ("Eq", &["refl"]),
        ("And", &["intro"]),
        ("Or", &["inl", "inr"]),
        ("False", &[]),
        ("True", &["intro"]),
        ("Exists", &["intro"]),
        ("Option", &["none", "some"]),
        ("List", &["nil", "cons"]),
        ("Prod", &["mk"]),
        ("Pif.Result", &["ok", "err"]),
    ];
    let add_block = |t: &str, cs: &[&str], out: &mut Vec<(Kind, String)>| {
        out.push((I, t.to_string()));
        out.extend(cs.iter().map(|c| (C, format!("{t}.{c}"))));
        out.push((R, format!("{t}.rec")));
    };
    for (t, cs) in &blocks[..12] {
        add_block(t, cs, &mut want);
    }
    for d in [
        "Pif.Not",
        "Pif.not",
        "Pif.and",
        "Pif.or",
        "Pif.xor",
        "Pif.Nat.add",
    ] {
        want.push((D, d.to_string()));
    }
    for (t, cs) in &blocks[12..] {
        add_block(t, cs, &mut want);
    }
    let got: Vec<(Kind, String)> = prelude()
        .decls
        .iter()
        .map(|d| (d.kind, d.name.to_string()))
        .collect();
    assert_eq!(got, want);
}

// ---- the Lean oracle -----------------------------------------------------------------------

fn level(l: Level, pm: &dyn Fn(&Name) -> String) -> String {
    match l.kind() {
        LevelKind::Zero => "z".into(),
        LevelKind::Succ(a) => format!("(succ {})", level(a.clone(), pm)),
        LevelKind::Max(a, b) => format!("(max {} {})", level(a.clone(), pm), level(b.clone(), pm)),
        LevelKind::IMax(a, b) => {
            format!("(imax {} {})", level(a.clone(), pm), level(b.clone(), pm))
        }
        LevelKind::Param(p) => format!("(p {})", pm(p)),
    }
}

/// The oracle's S-expression for a term, with binder names dropped (see `oracle/dump.lean`).
fn show(env: &Environment, e: Expr, pm: &dyn Fn(&Name) -> String) -> String {
    let info = |b: &BinderInfo| match b {
        BinderInfo::Default => "d",
        BinderInfo::Implicit => "i",
        BinderInfo::StrictImplicit => "s",
        BinderInfo::InstImplicit => "c",
    };
    match env.store.kind(e) {
        ExprKind::BVar(i) => format!("(v {i})"),
        ExprKind::Sort(l) => format!("(s {})", level(l.clone(), pm)),
        ExprKind::Const(c, ls) => {
            let ls: Vec<String> = ls.iter().map(|l| level(l.clone(), pm)).collect();
            format!("(c {c} ({}))", ls.join(" "))
        }
        ExprKind::App(f, a) => format!("(a {} {})", show(env, *f, pm), show(env, *a, pm)),
        ExprKind::Lam(b, t, x) => format!(
            "(l {} {} {})",
            info(&b.0.info),
            show(env, *t, pm),
            show(env, *x, pm)
        ),
        ExprKind::Pi(b, t, x) => format!(
            "(pi {} {} {})",
            info(&b.0.info),
            show(env, *t, pm),
            show(env, *x, pm)
        ),
        other => panic!("not in the prelude's fragment: {other:?}"),
    }
}

fn own(p: &Name) -> String {
    p.to_string()
}

fn lps(env: &Environment, name: &str) -> String {
    let d = env.get(&n(name)).unwrap();
    let v: Vec<String> = d.level_params.iter().map(|p| p.to_string()).collect();
    format!("({})", v.join(" "))
}

/// The line the oracle prints for inductive `t`, rendered from our environment.
fn ind_line(env: &Environment, t: &str) -> String {
    let d = env.get(&n(t)).unwrap();
    let DeclKind::Inductive(info) = &d.kind else {
        panic!("{t} is not an inductive")
    };
    let ctors: Vec<String> = info
        .ctors
        .iter()
        .map(|c| format!("({c} {})", show(env, env.get(c).unwrap().ty, &own)))
        .collect();
    let r = env.get(&info.rec).unwrap();
    let pos = |p: &Name| format!("u{}", r.level_params.iter().position(|q| q == p).unwrap());
    format!(
        "(ind {t} {} {} {} {} (ctors {}) (rec {} {}))",
        lps(env, t),
        info.num_params,
        info.num_indices,
        show(env, d.ty, &own),
        ctors.join(" "),
        r.level_params.len(),
        show(env, r.ty, &pos)
    )
}

/// The kernel derives recursors with every binder default, while Lean marks the parameters,
/// motive and indices implicit. Binder infos of a derived recursor are display metadata (the
/// kernel's equality ignores them), so the recursor part of a line is compared with them
/// erased; types and constructors keep theirs and are compared exactly.
fn default_rec_infos(line: &str) -> String {
    let (head, rec) = line.split_once("(rec ").unwrap();
    let rec = ["i", "s", "c"].iter().fold(rec.to_string(), |acc, i| {
        acc.replace(&format!("(pi {i} "), "(pi d ")
    });
    format!("{head}(rec {rec}")
}

#[test]
fn every_inductive_matches_leans_declaration_and_recursor() {
    let r = prelude();
    let mut seen = vec![];
    for line in ORACLE.lines() {
        let name = line.split(' ').nth(1).unwrap();
        if line.starts_with("(def ") {
            // `Not` is a definition in Lean; the prelude calls it Pif.Not (same type and value).
            let d = r.env.get(&n("Pif.Not")).unwrap();
            let DeclKind::Definition { value, .. } = &d.kind else {
                panic!()
            };
            let ours = format!(
                "(def Not {} {} {})",
                lps(&r.env, "Pif.Not"),
                show(&r.env, d.ty, &own),
                show(&r.env, *value, &own)
            );
            assert_eq!(ours, line, "Not");
        } else if r.env.contains(&n(name)) {
            assert_eq!(
                default_rec_infos(&ind_line(&r.env, name)),
                default_rec_infos(line),
                "{name} differs from Lean's declaration"
            );
            seen.push(name.to_string());
        } else {
            // deliberately absent (README): pinned separately for the prim roles of VER32 3.5
            assert!(
                ["Iff", "Nonempty"].contains(&name),
                "{name} is missing from the prelude"
            );
        }
    }
    assert_eq!(seen.len(), 12, "{seen:?}");
}

// ---- theorems through the prelude ----------------------------------------------------------

/// A claim-style file over the prelude: its own tables, importing the prelude by digest.
fn over_prelude(build: impl FnOnce(&mut Writer)) -> Result<ReadReport, PifError> {
    let digest = sha256_hex(COMMITTED.as_bytes());
    let mut w = Writer::new("native", "sanity").unwrap().ascii_names_only();
    w.import(&digest).unwrap();
    build(&mut w);
    let gate = GateInputs {
        expected_imports: vec![digest],
        class: FileClass::Library,
        ..GateInputs::default()
    };
    read(prelude().env, w.to_text().as_bytes(), &gate)
}

/// `theorem name : forall b : Bool, Eq Bool (lhs b) (rhs b)` proved by `Bool.rec` with
/// `Eq.refl` in both cases, where `lhs` and `rhs` are built from `b`.
fn bool_law(
    w: &mut Writer,
    name: &str,
    sides: impl Fn(&mut Writer, pif_writer::Expr) -> (pif_writer::Expr, pif_writer::Expr),
    refl_at: [&str; 2],
) {
    let (bool_n, anon) = (w.name("Bool").unwrap(), w.name("b").unwrap());
    let bool_t = w.constant(bool_n, &[]);
    let one = w.level_succ(L::ZERO);
    let (eq_n, refl_n) = (w.name("Eq").unwrap(), w.name("Eq.refl").unwrap());
    let (eq, refl) = (w.constant(eq_n, &[one]), w.constant(refl_n, &[one]));
    let v0 = w.var(0);
    let (l, r) = sides(w, v0);
    let eq_bool = w.app(eq, bool_t);
    let stmt = w.app_many(eq_bool, &[l, r]);
    let ty = w.pi(B::Default, anon, bool_t, stmt);
    let motive = w.lam(B::Default, anon, bool_t, stmt);
    let rec_n = w.name("Bool.rec").unwrap();
    let rec = w.constant(rec_n, &[L::ZERO]);
    let refl_bool = w.app(refl, bool_t);
    let cases: Vec<_> = refl_at
        .iter()
        .map(|c| {
            let cn = w.name(&format!("Bool.{c}")).unwrap();
            let cc = w.constant(cn, &[]);
            w.app(refl_bool, cc)
        })
        .collect();
    let body = w.app_many(rec, &[motive, cases[0], cases[1], v0]);
    let val = w.lam(B::Default, anon, bool_t, body);
    let t = w.name(name).unwrap();
    w.theorem(t, &[], ty, val).unwrap();
}

fn call(w: &mut Writer, f: &str, args: &[pif_writer::Expr]) -> pif_writer::Expr {
    let n = w.name(f).unwrap();
    let c = w.constant(n, &[]);
    w.app_many(c, args)
}

fn bool_const(w: &mut Writer, c: &str) -> pif_writer::Expr {
    call(w, &format!("Bool.{c}"), &[])
}

#[test]
fn double_negation_xor_self_and_or_laws_check_through_the_prelude() {
    let r = over_prelude(|w| {
        // not (not b) = b
        let sides = |w: &mut Writer, b| {
            let nb = call(w, "Pif.not", &[b]);
            (call(w, "Pif.not", &[nb]), b)
        };
        bool_law(w, "T.not_not", sides, ["false", "true"]);
        // xor b b = false
        let sides = |w: &mut Writer, b| (call(w, "Pif.xor", &[b, b]), bool_const(w, "false"));
        bool_law(w, "T.xor_self", sides, ["false", "false"]);
        // and b true = b ; or b false = b
        let sides = |w: &mut Writer, b| {
            let t = bool_const(w, "true");
            (call(w, "Pif.and", &[b, t]), b)
        };
        bool_law(w, "T.and_true", sides, ["false", "true"]);
        let sides = |w: &mut Writer, b| {
            let f = bool_const(w, "false");
            (call(w, "Pif.or", &[b, f]), b)
        };
        bool_law(w, "T.or_false", sides, ["false", "true"]);
    })
    .expect("four theorems check");
    assert_eq!(r.decls.len(), 4);
    assert!(r.decls.iter().all(|d| d.kind == Kind::Theorem));
}

#[test]
fn a_wrong_law_is_refused_by_the_kernel() {
    // `not false = false` is not true: the file is well formed, the kernel refuses it.
    let r = over_prelude(|w| {
        let sides = |w: &mut Writer, b| (call(w, "Pif.not", &[b]), b);
        bool_law(w, "T.bad", sides, ["false", "true"]);
    });
    assert!(matches!(r, Err(PifError::Kernel(_))), "{:?}", r.err());
}

#[test]
fn nat_addition_and_pif_not_compute_by_reflexivity() {
    let r = over_prelude(|w| {
        let nat_n = w.name("Nat").unwrap();
        let nat = w.constant(nat_n, &[]);
        let one_l = w.level_succ(L::ZERO);
        let eq_n = w.name("Eq").unwrap();
        let eq = w.constant(eq_n, &[one_l]);
        let refl_n = w.name("Eq.refl").unwrap();
        let refl = w.constant(refl_n, &[one_l]);
        let zero = call(w, "Nat.zero", &[]);
        let s1 = call(w, "Nat.succ", &[zero]);
        let s2 = call(w, "Nat.succ", &[s1]);
        // 1 + 1 = 2
        let sum = call(w, "Pif.Nat.add", &[s1, s1]);
        let ty = w.app_many(eq, &[nat, sum, s2]);
        let val = w.app_many(refl, &[nat, s2]);
        let t = w.name("T.one_plus_one").unwrap();
        w.theorem(t, &[], ty, val).unwrap();
        // Pif.Not False, proved by the identity on False (needs delta on Pif.Not)
        let fls = call(w, "False", &[]);
        let ty = call(w, "Pif.Not", &[fls]);
        let h = w.name("h").unwrap();
        let v0 = w.var(0);
        let val = w.lam(B::Default, h, fls, v0);
        let t = w.name("T.not_false").unwrap();
        w.theorem(t, &[], ty, val).unwrap();
        // Pif.Result.ok at universe 0 builds a `Result Nat Bool`, as a definition
        let z = L::ZERO;
        let (res_n, ok_n) = (
            w.name("Pif.Result").unwrap(),
            w.name("Pif.Result.ok").unwrap(),
        );
        let ge = call(w, "Bool", &[]);
        let res = w.constant(res_n, &[z, z]);
        let ty = w.app_many(res, &[nat, ge]);
        let ok = w.constant(ok_n, &[z, z]);
        let val = w.app_many(ok, &[nat, ge, zero]);
        let d = w.name("T.ok_zero").unwrap();
        w.definition(d, &[], ty, val, Hint::Abbrev).unwrap();
    })
    .expect("the sanity file checks");
    assert_eq!(r.decls.len(), 3);
}
