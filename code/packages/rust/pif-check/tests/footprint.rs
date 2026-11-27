//! PIF-003c: footprints, axiom classification, the registry and the per-claim report.

mod common;

use common::*;
use lean_kernel::pif::PifError;
use lean_kernel::{Environment, Name};
use pif_check::chain::{load_chain, LoadOptions, Loaded};
use pif_check::claims::{claim_reports, KERNEL_VERSION};
use pif_check::control::Role;
use pif_check::cte::{goal_digest, name_cte};
use pif_check::footprint::{footprint, AxClass};
use pif_check::goals::check_goals;
use pif_check::registry::{shape, Entry, Registry, Shape};
use pif_check::resolver::Resolver;
use pif_check::{CheckError, Digest};
use pif_writer::{Hint, Level, Writer};

fn n(s: &str) -> Name {
    Name::from_dotted(s)
}

/// The digest of the binding CTE of the constant `False` (the type of the test axioms).
fn false_digest() -> Digest {
    let mut env = Environment::new();
    let c = env.store.constant(&n("False"), &[]);
    goal_digest(&env.store, c).unwrap()
}

/// The axioms file: `Warrant.Assume.t.ax : False`, `Src.demo.s : False`, `propext : False`
/// and `Pif.A : Bool`, over the prelude.
fn axioms_file(prelude_hex: &str) -> String {
    let mut w = Writer::new("native", "axioms 0.1")
        .unwrap()
        .ascii_names_only();
    w.import(prelude_hex).unwrap();
    let (f, b) = (w.name("False").unwrap(), w.name("Bool").unwrap());
    let (fc, bc) = (w.constant(f, &[]), w.constant(b, &[]));
    for name in ["Warrant.Assume.t.ax", "Src.demo.s", "propext"] {
        let nm = w.name(name).unwrap();
        w.axiom(nm, &[], fc).unwrap();
    }
    let a = w.name("Pif.A").unwrap();
    w.axiom(a, &[], bc).unwrap();
    // def Pif.tt : Bool := Pif.A   (a definition hiding an axiom, in a BASE file: a claim file
    // may not declare a constant that occurs in a goal, so thm4's statement needs it here)
    let ac = w.constant(a, &[]);
    let tt = w.name("Pif.tt").unwrap();
    w.definition(tt, &[], bc, ac, Hint::Regular(1)).unwrap();
    w.to_text()
}

/// The claim file. Each theorem is bound to a claim id with the hint from `hints` (by index),
/// or a dummy when `hints` is empty.
fn claim_file(imports: &[String], hints: &[String], extra_axiom: Option<&str>) -> String {
    let mut w = Writer::new("native", "claim 0.1")
        .unwrap()
        .ascii_names_only();
    for i in imports {
        w.import(i).unwrap();
    }
    let (f, b) = (w.name("False").unwrap(), w.name("Bool").unwrap());
    let (fc, bc) = (w.constant(f, &[]), w.constant(b, &[]));
    let tt = w.name("Bool.true").unwrap();
    let ttc = w.constant(tt, &[]);
    let (eq, refl) = (w.name("Eq").unwrap(), w.name("Eq.refl").unwrap());
    let one = w.level_succ(Level::ZERO);
    let (eq1, refl1) = (w.constant(eq, &[one]), w.constant(refl, &[one]));
    let (ax, src, pext) = (
        w.name("Warrant.Assume.t.ax").unwrap(),
        w.name("Src.demo.s").unwrap(),
        w.name("propext").unwrap(),
    );
    let (axc, srcc, pextc) = (
        w.constant(ax, &[]),
        w.constant(src, &[]),
        w.constant(pext, &[]),
    );
    if let Some(name) = extra_axiom {
        let q = w.name(name).unwrap();
        w.axiom(q, &[], fc).unwrap();
    }
    // def Pif.d : False := Warrant.Assume.t.ax          (a definition hiding an axiom)
    let d = w.name("Pif.d").unwrap();
    w.definition(d, &[], fc, axc, Hint::Regular(1)).unwrap();
    let dc = w.constant(d, &[]);
    let tt_n = w.name("Pif.tt").unwrap();
    let ttd = w.constant(tt_n, &[]);
    // theorem Pif.thm0 : Eq Bool true true := Eq.refl Bool true      (no axiom)
    let eq_tt = w.app_many(eq1, &[bc, ttc, ttc]);
    let pf0 = w.app_many(refl1, &[bc, ttc]);
    // theorem Pif.thm4 : Eq Bool Pif.tt Pif.tt := Eq.refl Bool Pif.tt   (axiom only in the TYPE)
    let eq_d = w.app_many(eq1, &[bc, ttd, ttd]);
    let pf4 = w.app_many(refl1, &[bc, ttd]);
    let thms = [
        ("Pif.thm0", "c.zero", eq_tt, pf0),
        ("Pif.thm1", "c.one", fc, dc),
        ("Pif.thm2", "c.two", fc, srcc),
        ("Pif.thm3", "c.three", fc, pextc),
        ("Pif.thm4", "c.four", eq_d, pf4),
    ];
    for (i, (name, claim, ty, pf)) in thms.iter().enumerate() {
        let t = w.name(name).unwrap();
        w.theorem(t, &[], *ty, *pf).unwrap();
        let hint = hints
            .get(i)
            .cloned()
            .unwrap_or_else(|| Digest::of(b"dummy").hex());
        w.goal(claim, t, &hint).unwrap();
    }
    w.to_text()
}

struct World {
    prelude: Vec<u8>,
    axioms: String,
    claim: String,
}

impl World {
    fn chain(&self, with_claim: bool) -> Vec<pif_check::control::ChainEntry> {
        let mut c = vec![
            entry(Role::Prelude, &self.prelude),
            entry(Role::Axioms, self.axioms.as_bytes()),
        ];
        if with_claim {
            c.push(entry(Role::Claim, self.claim.as_bytes()));
        }
        c
    }
    fn load(&self, opts: &LoadOptions) -> Result<Loaded, CheckError> {
        let m = mem(&[&self.prelude, self.axioms.as_bytes(), self.claim.as_bytes()]);
        load_chain(&Resolver::new(&m), &self.chain(true), opts)
    }
}

fn world() -> World {
    let prelude = prelude();
    let ph = Digest::of(&prelude).hex();
    let axioms = axioms_file(&ph);
    let imports = [ph, Digest::of(axioms.as_bytes()).hex()];
    // Two passes: the gate computes each goal digest; the record only has to agree.
    let first = claim_file(&imports, &[], None);
    let m = mem(&[&prelude, axioms.as_bytes(), first.as_bytes()]);
    let w0 = World {
        prelude: prelude.clone(),
        axioms: axioms.clone(),
        claim: first,
    };
    let l = load_chain(&Resolver::new(&m), &w0.chain(true), &LoadOptions::default()).unwrap();
    let hints: Vec<String> = l.files[2]
        .goals
        .iter()
        .map(|g| {
            goal_digest(&l.env.store, l.env.get(&g.theorem).unwrap().ty)
                .unwrap()
                .hex()
        })
        .collect();
    World {
        prelude,
        axioms,
        claim: claim_file(&imports, &hints, None),
    }
}

fn prefix_of_axioms(w: &World) -> Digest {
    Digest::env_start().fold(&Digest::of(&w.prelude))
}

fn registry(w: &World) -> Registry {
    let mut r = Registry::default();
    for name in ["Warrant.Assume.t.ax", "Src.demo.s"] {
        r.add(Entry {
            name: n(name),
            params: vec![],
            type_digest: false_digest(),
            prefix: prefix_of_axioms(w),
        });
    }
    r
}

#[test]
fn a_theorem_using_an_axiom_shows_it_and_one_without_shows_none() {
    let w = world();
    let l = w.load(&LoadOptions::default()).unwrap();
    let goals = check_goals(&l, None, None).unwrap();
    let reports = claim_reports(&l, &goals, None).unwrap();
    let fp = |claim: &str| -> Vec<String> {
        reports
            .iter()
            .find(|r| r.goal.claim_id == claim)
            .unwrap()
            .footprint
            .iter()
            .map(|a| pif_check::report::dotted(&a.name))
            .collect()
    };
    // No axiom anywhere under thm0 (it walks Bool, Eq, their recursors and constructors).
    assert!(fp("c.zero").is_empty());
    // Through a definition's value.
    assert_eq!(fp("c.one"), ["Warrant.Assume.t.ax"]);
    assert_eq!(fp("c.two"), ["Src.demo.s"]);
    assert_eq!(fp("c.three"), ["propext"]);
    // An axiom that appears only in the theorem's TYPE (via a definition) is still found.
    assert_eq!(fp("c.four"), ["Pif.A"]);
    // Without a registry everything is an unclassified (c) assumption: fail closed.
    for r in &reports {
        assert!(r
            .footprint
            .iter()
            .all(|a| a.class == AxClass::Unclassified && a.bucket == 'c'));
    }
    // The certificate id is the claim file's digest.
    assert_eq!(
        reports[0].certificate_id,
        format!("pif:sha256:{}", Digest::of(w.claim.as_bytes()).hex())
    );
}

#[test]
fn registered_statements_are_classified_and_everything_else_stays_unclassified() {
    let w = world();
    let l = w.load(&LoadOptions::default()).unwrap();
    let goals = check_goals(&l, None, None).unwrap();
    let reg = registry(&w);
    let reports = claim_reports(&l, &goals, Some(&reg)).unwrap();
    let class = |claim: &str| {
        reports
            .iter()
            .find(|r| r.goal.claim_id == claim)
            .unwrap()
            .footprint[0]
            .class
    };
    assert_eq!(class("c.one"), AxClass::WarrantAssume);
    assert_eq!(class("c.two"), AxClass::Src);
    // `propext` is a logic-axiom NAME: without a validated axiom.* role it is unclassified (A1).
    assert_eq!(class("c.three"), AxClass::Unclassified);
    assert_eq!(class("c.four"), AxClass::Unclassified);
    let json = reports
        .iter()
        .find(|r| r.goal.claim_id == "c.two")
        .unwrap()
        .json(&l)
        .render();
    assert!(json.contains("\"class\":\"src\""));
    assert!(json.contains("consistency with the kernel logic not established"));
    assert!(json.contains("\"rungs\":\"pending\""));
    assert!(json.contains(&format!("\"kernel_version\":\"{KERNEL_VERSION}\"")));
    assert!(json.contains("\"kernel_commit\":"));
    assert!(json.contains("\"buckets\":{\"a\":0,\"b\":0,\"c\":1}"));
    assert!(json.contains(&format!(
        "\"base_environment\":\"{}\"",
        l.base_environment.hex()
    )));
    assert!(json.contains(&format!("\"environment\":\"{}\"", l.environment.hex())));
    // A registry entry under a different environment prefix does not match (A2).
    let mut other = Registry::default();
    other.add(Entry {
        name: n("Warrant.Assume.t.ax"),
        params: vec![],
        type_digest: false_digest(),
        prefix: Digest::of(b"elsewhere"),
    });
    let reports = claim_reports(&l, &goals, Some(&other)).unwrap();
    assert_eq!(
        reports
            .iter()
            .find(|r| r.goal.claim_id == "c.one")
            .unwrap()
            .footprint[0]
            .class,
        AxClass::Unclassified
    );
    // A different statement digest does not match either.
    let mut wrong = Registry::default();
    wrong.add(Entry {
        name: n("Src.demo.s"),
        params: vec![],
        type_digest: Digest::of(b"x"),
        prefix: prefix_of_axioms(&w),
    });
    let reports = claim_reports(&l, &goals, Some(&wrong)).unwrap();
    assert_eq!(
        reports
            .iter()
            .find(|r| r.goal.claim_id == "c.two")
            .unwrap()
            .footprint[0]
            .class,
        AxClass::Unclassified
    );
}

#[test]
fn a_constants_footprint_includes_itself_and_unknown_constants_are_errors() {
    let w = world();
    let l = w.load(&LoadOptions::default()).unwrap();
    // G2: an axiom's footprint is itself (it has no value to walk).
    assert_eq!(
        footprint(&l, &n("Warrant.Assume.t.ax")).unwrap(),
        [n("Warrant.Assume.t.ax")]
    );
    // A definition built from an axiom.
    assert_eq!(footprint(&l, &n("Pif.tt")).unwrap(), [n("Pif.A")]);
    // The prelude's own functions use no axiom.
    assert!(footprint(&l, &n("Pif.and")).unwrap().is_empty());
    assert!(footprint(&l, &n("Nat.rec")).unwrap().is_empty());
    assert_eq!(
        footprint(&l, &n("No.such")).err().unwrap(),
        CheckError::UnknownConstant("No.such".into())
    );
    // The result is sorted by canonical name.
    let all = [n("Warrant.Assume.t.ax"), n("Src.demo.s")];
    let mut sorted = all.to_vec();
    sorted.sort_by_key(name_cte);
    assert_eq!(sorted[0], n("Src.demo.s"));
}

#[test]
fn the_registry_file_format_parses_and_refuses_the_wrong_things() {
    let (t, p) = (Digest::of(b"t").hex(), Digest::of(b"p").hex());
    let ok = format!(
        "pif-registry 1\naxiom <\"Warrant\",\"Assume\",\"a\"> - {t} sha256:{p}\n\
         axiom <\"Src\",\"hol\",\"select\"> <\"u\">,<\"v\"> {t} {p}\n\
         axiom <\"Src\",\"hol\",\"select\"> - {t} {p}\n"
    );
    let r = Registry::parse(&ok).unwrap();
    assert_eq!(r.len(), 3);
    assert!(!r.is_empty());
    assert!(r.matches(
        &n("Warrant.Assume.a"),
        &[],
        &Digest::parse(&t).unwrap(),
        &Digest::parse(&p).unwrap()
    ));
    assert!(r.matches(
        &n("Src.hol.select"),
        &[n("u"), n("v")],
        &Digest::parse(&t).unwrap(),
        &Digest::parse(&p).unwrap()
    ));
    assert!(!r.matches(
        &n("Src.hol.select"),
        &[n("v"), n("u")],
        &Digest::parse(&t).unwrap(),
        &Digest::parse(&p).unwrap()
    ));
    assert!(Registry::parse("pif-registry 1\n").unwrap().is_empty());
    for bad in [
        format!("pif-registry 1\naxiom <\"Foo\",\"bar\",\"baz\"> - {t} {p}\n"), // not a registrable name
        format!("pif-registry 1\naxiom <\"Warrant\",\"Assume\"> - {t} {p}\n"),     // no id
        format!("pif-registry 1\naxiom <\"Src\",\"a\",\"b\"> - {t}\n"),            // missing field
        format!("pif-registry 1\naxiom Src.a.b - {t} {p}\n"),                      // not a name CTE
        format!("pif-registry 1\naxiom <\"Src\",\"a\",\"b\"> x {t} {p}\n"),        // bad params
        format!("pif-registry 1\naxiom <\"Src\",\"a\",\"b\"> - nothex {p}\n"),
        format!("pif-registry 1\naxiom <\"Src\",\"a\",\"b\"> - {t} {p}\naxiom <\"Src\",\"a\",\"b\"> - {t} {p}\n"),
        format!("pif-registry 1\nfoo <\"Src\",\"a\",\"b\"> - {t} {p}\n"),
        "pif-registry 2\n".to_string(),
    ] {
        assert_eq!(Registry::parse(&bad).err().map(|e| e.code()), Some("BadControlFile"), "{bad}");
    }
    assert_eq!(shape(&n("Warrant.Assume.x")), Some(Shape::WarrantAssume));
    assert_eq!(shape(&n("Src.rocq.x")), Some(Shape::Src));
    assert_eq!(shape(&n("Src.rocq")), None);
    assert_eq!(shape(&n("propext")), None);
}

#[test]
fn a_claim_files_own_axioms_must_match_the_registry_exactly() {
    let w = world();
    let prelude_hex = Digest::of(&w.prelude).hex();
    let ax_hex = Digest::of(w.axioms.as_bytes()).hex();
    let imports = [prelude_hex, ax_hex];
    let base = Digest::env_start()
        .fold(&Digest::of(&w.prelude))
        .fold(&Digest::of(w.axioms.as_bytes()));
    let claim = claim_file(&imports, &[], Some("Warrant.Assume.q"));
    let m = mem(&[&w.prelude, w.axioms.as_bytes(), claim.as_bytes()]);
    let chain = [
        entry(Role::Prelude, &w.prelude),
        entry(Role::Axioms, w.axioms.as_bytes()),
        entry(Role::Claim, claim.as_bytes()),
    ];
    let load = |reg: Option<&Registry>| {
        load_chain(
            &Resolver::new(&m),
            &chain,
            &LoadOptions {
                registry: reg,
                ..LoadOptions::default()
            },
        )
    };
    let refused = CheckError::Read {
        index: 2,
        role: "claim",
        error: PifError::AxiomNotAllowed,
    };
    // No registry: every axiom of a claim file is refused.
    assert_eq!(load(None).err().unwrap(), refused);
    let entry_for = |params: Vec<Name>, ty: Digest, prefix: Digest| {
        let mut r = Registry::default();
        r.add(Entry {
            name: n("Warrant.Assume.q"),
            params,
            type_digest: ty,
            prefix,
        });
        r
    };
    // Registered exactly: accepted (and the claim's goals still need their own checks).
    assert!(load(Some(&entry_for(vec![], false_digest(), base))).is_ok());
    // A different statement, different universe parameters or a different prefix: refused.
    assert_eq!(
        load(Some(&entry_for(vec![], Digest::of(b"x"), base)))
            .err()
            .unwrap(),
        refused
    );
    assert_eq!(
        load(Some(&entry_for(vec![n("u")], false_digest(), base)))
            .err()
            .unwrap(),
        refused
    );
    assert_eq!(
        load(Some(&entry_for(vec![], false_digest(), Digest::of(b"y"))))
            .err()
            .unwrap(),
        refused
    );
}

#[test]
fn the_command_line_prints_footprints_with_a_registry() {
    let w = world();
    let t = Tmp::new();
    for f in [
        w.prelude.as_slice(),
        w.axioms.as_bytes(),
        w.claim.as_bytes(),
    ] {
        t.store(f);
    }
    let chain = t.put(
        "chain.txt",
        format!(
            "pif-chain 1\nprelude {}\naxioms {}\nclaim {}\n",
            Digest::of(&w.prelude).hex(),
            Digest::of(w.axioms.as_bytes()).hex(),
            Digest::of(w.claim.as_bytes()).hex()
        )
        .as_bytes(),
    );
    let (ty, prefix) = (false_digest().hex(), prefix_of_axioms(&w).hex());
    let reg = t.put(
        "registry.txt",
        format!(
            "pif-registry 1\naxiom <\"Warrant\",\"Assume\",\"t\",\"ax\"> - {ty} {prefix}\n\
             axiom <\"Src\",\"demo\",\"s\"> - {ty} {prefix}\n"
        )
        .as_bytes(),
    );
    let store = t.0.to_string_lossy().into_owned();
    let args = |extra: &[&str]| -> Vec<String> {
        ["--unsafe-no-allow", "--store", &store, "--chain", &chain]
            .iter()
            .chain(extra)
            .map(|s| (*s).to_string())
            .collect()
    };
    let out = pif_check::cli::run(&args(&["--registry", &reg]));
    assert_eq!(out.code, 0, "{}", out.stdout);
    assert!(out.stdout.contains("\"registry\":\"checked\""));
    assert!(out.stdout.contains("\"axiom\":\"Warrant.Assume.t.ax\",\"name_cte\":\"<\\\"Warrant\\\",\\\"Assume\\\",\\\"t\\\",\\\"ax\\\">\",\"class\":\"warrant-assume\""));
    assert!(out.stdout.contains("\"class\":\"src\""));
    assert!(out.stdout.contains("\"class\":\"unclassified\""));
    assert!(out.stdout.contains("\"footprint\":[]"));
    // Without the registry the same chain loads and every axiom is unclassified.
    let out = pif_check::cli::run(&args(&[]));
    assert_eq!(out.code, 0, "{}", out.stdout);
    assert!(out.stdout.contains("\"registry\":\"not-provided\""));
    assert!(!out.stdout.contains("warrant-assume"));
    // A malformed registry is a refusal.
    let bad = t.put("bad.txt", b"pif-registry 1\nnonsense\n");
    let out = pif_check::cli::run(&args(&["--registry", &bad]));
    assert_eq!(out.code, 1);
    assert!(out.stdout.contains("\"error\":\"BadControlFile\""));
}

#[test]
fn the_kernel_version_constant_matches_the_kernels_manifest() {
    let manifest = include_str!("../../lean-kernel/Cargo.toml");
    let line = manifest
        .lines()
        .find(|l| l.starts_with("version = "))
        .unwrap();
    assert_eq!(line, format!("version = \"{KERNEL_VERSION}\""));
}
