//! Goal records (R-GOL-1 to 3) and the claim-file rules (R-AX-1, R-GOL-4): a claim file is
//! the proof, so it may not assume its goal, may not redefine what the goal mentions,
//! and may not point a goal at anything but a theorem it proved itself.

mod pif_common;

use lean_kernel::pif::{read, FileClass, GateInputs, PifError as E};
use lean_kernel::{Environment, ExprKind, Name};
use pif_common::{err, lib, ok, B};
use std::collections::HashSet;

fn n(s: &str) -> Name {
    Name::from_dotted(s)
}

/// A library file with the proposition `A`, a proof `a : A` and a theorem `t : A`, as a
/// previous link of the chain; returns the environment and its bytes' digest placeholder.
fn base_env() -> Environment {
    let mut b = B::new();
    let (_, ac) = b.prop_axiom("A");
    let an = b.name("a");
    b.axiom(an, &[], ac);
    let a_val = b.cnst(an, &[]);
    let t = b.name("t");
    b.theorem(t, &[], ac, a_val);
    read(Environment::new(), &b.bytes(), &lib())
        .expect("base")
        .env
}

const D: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

/// A claim file on top of the base: it names `A` and `a` again (indices are file-local).
fn claim_with(body: impl FnOnce(&mut B, usize, usize)) -> B {
    let mut b = B::new();
    b.header.push(format!("import sha256:{D}"));
    let an = b.name("A");
    let ac = b.cnst(an, &[]);
    let a = b.name("a");
    let a_val = b.cnst(a, &[]);
    body(&mut b, ac, a_val);
    b
}

fn claim_gate() -> GateInputs<'static> {
    GateInputs {
        expected_imports: vec![D.to_string()],
        ..GateInputs::default()
    }
}

fn claim_err(b: &B, g: &GateInputs) -> E {
    match read(base_env(), &b.bytes(), g) {
        Ok(_) => panic!("expected a rejection\n{}", b.text()),
        Err(e) => e,
    }
}

// ---- goals -------------------------------------------------------------------------------

#[test]
fn a_goal_on_a_theorem_declared_in_this_file_is_recorded() {
    let b = claim_with(|b, ac, a_val| {
        let t = b.name("proof");
        b.theorem(t, &[], ac, a_val);
        b.line(&format!("goal \"gates.and_comm\" {t} sha256:{D}"));
        b.line(&format!("goal \"second-claim_1\" {t} sha256:{D}"));
    });
    let r = read(base_env(), &b.bytes(), &claim_gate()).expect("accepted");
    let ids: Vec<&str> = r.goals.iter().map(|g| g.claim_id.as_str()).collect();
    assert_eq!(ids, ["gates.and_comm", "second-claim_1"]);
    assert_eq!(r.goals[0].theorem, n("proof"));
    assert_eq!(r.goals[0].digest, D);
}

#[test]
fn r_gol_1_a_goal_on_an_axiom_is_rejected() {
    let b = claim_with(|b, ac, _| {
        // not allowed in a claim file as an axiom, so make the target a library-style one
        let t = b.name("T");
        b.axiom(t, &[], ac);
        b.line(&format!("goal \"c\" {t} sha256:{D}"));
    });
    // in a library file the axiom is allowed, so the goal rule is what refuses it
    let g = GateInputs {
        class: FileClass::Library,
        ..claim_gate()
    };
    assert_eq!(claim_err(&b, &g), E::GoalTargetNotTheorem);
    // under claim rules the axiom itself is refused first (R-AX-1)
    assert_eq!(claim_err(&b, &claim_gate()), E::AxiomNotAllowed);
}

#[test]
fn r_gol_1_a_goal_on_a_definition_an_opaque_or_a_type_is_rejected() {
    for what in ["def", "opaque", "inductive"] {
        let b = claim_with(|b, ac, a_val| {
            let t = b.name("target");
            match what {
                "def" => b.def(t, &[], ac, a_val, "@abbrev"),
                "opaque" => b.opaque(t, &[], ac, a_val),
                _ => {
                    let mk = b.nstr(t, "mk");
                    let one = b.lsucc(0);
                    let ty = b.sort(one);
                    let tc = b.cnst(t, &[]);
                    b.line("inductive [] 0 1");
                    b.line(&format!("itype {t} {ty} 1"));
                    b.line(&format!("ictor {mk} {tc}"));
                }
            }
            b.line(&format!("goal \"c\" {t} sha256:{D}"));
        });
        assert_eq!(
            claim_err(&b, &claim_gate()),
            E::GoalTargetNotTheorem,
            "{what}"
        );
    }
}

#[test]
fn r_gol_1_a_goal_on_a_name_that_is_not_declared_or_not_yet_declared() {
    let b = claim_with(|b, _, _| {
        let ghost = b.name("Ghost");
        b.line(&format!("goal \"c\" {ghost} sha256:{D}"));
    });
    assert_eq!(claim_err(&b, &claim_gate()), E::GoalTargetNotTheorem);
    // the theorem comes after its goal: the goal could not have been checked against it
    let b = claim_with(|b, ac, a_val| {
        let t = b.name("proof");
        b.line(&format!("goal \"c\" {t} sha256:{D}"));
        b.theorem(t, &[], ac, a_val);
    });
    assert_eq!(claim_err(&b, &claim_gate()), E::GoalTargetNotTheorem);
}

#[test]
fn r_gol_1_a_goal_on_an_imported_theorem_or_axiom_is_rejected() {
    // `t` is a theorem and `a` an axiom of the base file, not of this one
    for imported in ["t", "a", "A"] {
        let b = claim_with(|b, _, _| {
            let t = b.name(imported);
            b.line(&format!("goal \"c\" {t} sha256:{D}"));
        });
        assert_eq!(
            claim_err(&b, &claim_gate()),
            E::GoalTargetImported,
            "{imported}"
        );
    }
}

#[test]
fn r_gol_2_claim_ids_are_unique_in_the_environment() {
    // twice in one file
    let b = claim_with(|b, ac, a_val| {
        let t = b.name("proof");
        b.theorem(t, &[], ac, a_val);
        b.line(&format!("goal \"c\" {t} sha256:{D}"));
        b.line(&format!("goal \"c\" {t} sha256:{D}"));
    });
    assert_eq!(claim_err(&b, &claim_gate()), E::DuplicateGoal);
    // already used by an earlier file of the environment
    let b = claim_with(|b, ac, a_val| {
        let t = b.name("proof");
        b.theorem(t, &[], ac, a_val);
        b.line(&format!("goal \"used\" {t} sha256:{D}"));
    });
    let g = GateInputs {
        prior_claim_ids: HashSet::from(["used".to_string()]),
        ..claim_gate()
    };
    assert_eq!(claim_err(&b, &g), E::DuplicateGoal);
}

#[test]
fn r_gol_3_claim_id_charset_and_length() {
    let bad = [
        "",
        "a\\u{20}b",
        "a\\u{a}b",
        "a/b",
        "a:b",
        "caf\\u{e9}",
        "a\\u{a0}b",
    ];
    let long = "a".repeat(129);
    for id in bad.iter().copied().chain([long.as_str()]) {
        let b = claim_with(|b, ac, a_val| {
            let t = b.name("proof");
            b.theorem(t, &[], ac, a_val);
            b.line(&format!("goal \"{id}\" {t} sha256:{D}"));
        });
        let e = claim_err(&b, &claim_gate());
        assert_eq!(e, E::BadClaimId, "{id:?}");
    }
    let longest = "A_z.9-".repeat(21) + "ab"; // 128 characters
    assert_eq!(longest.len(), 128);
    let b = claim_with(|b, ac, a_val| {
        let t = b.name("proof");
        b.theorem(t, &[], ac, a_val);
        b.line(&format!("goal \"{longest}\" {t} sha256:{D}"));
    });
    read(base_env(), &b.bytes(), &claim_gate()).expect("128 characters is allowed");
}

#[test]
fn a_goal_digest_must_be_lexically_a_sha256() {
    for digest in [
        "sha256:abc",
        "sha256:ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789",
        "md5:0123",
        "sha256:",
    ] {
        let b = claim_with(|b, ac, a_val| {
            let t = b.name("proof");
            b.theorem(t, &[], ac, a_val);
            b.line(&format!("goal \"c\" {t} {digest}"));
        });
        assert_eq!(claim_err(&b, &claim_gate()), E::BadLexicalForm, "{digest}");
    }
}

// ---- R-AX-1 ------------------------------------------------------------------------------

#[test]
fn r_ax_1_a_claim_file_may_not_declare_propext_as_false() {
    // `axiom propext : False`, the attack of VER32 1.4 A1: the name is a standard axiom's,
    // the statement is not, and a claim file may declare no such axiom at all
    let mut base = B::new();
    let (_, fc) = base.prop_axiom("False");
    let _ = fc;
    let env = read(Environment::new(), &base.bytes(), &lib()).unwrap().env;
    let mut b = B::new();
    b.header.push(format!("import sha256:{D}"));
    let f = b.name("False");
    let fc = b.cnst(f, &[]);
    let p = b.name("propext");
    b.axiom(p, &[], fc);
    match read(env, &b.bytes(), &claim_gate()) {
        Err(e) => assert_eq!(e, E::AxiomNotAllowed),
        Ok(_) => panic!("axiom propext : False was accepted in a claim file"),
    }
}

#[test]
fn r_ax_1_only_registered_assumptions_pass_and_only_when_the_registry_agrees() {
    let body = |name: &str| {
        let mut b = B::new();
        let prop = b.sort(0);
        let a = b.name(name);
        b.axiom(a, &[], prop);
        b
    };
    // the registry sees the exact declaration: name, universe parameters and type
    let wants_prop = |nm: &Name, ps: &[Name], ty, st: &lean_kernel::ExprStore| {
        *nm == n("Warrant.Assume.ok")
            && ps.is_empty()
            && matches!(st.kind(ty), ExprKind::Sort(l) if l.is_zero())
    };
    let g = GateInputs {
        axiom_registry: Some(&wants_prop),
        ..GateInputs::default()
    };
    ok(&body("Warrant.Assume.ok").bytes(), &g);
    // a registered-looking name the registry does not know
    assert_eq!(
        err(&body("Warrant.Assume.other").bytes(), &g),
        E::AxiomNotAllowed
    );
    // names of the wrong shape never reach the registry, even if it would say yes
    let yes = |_: &Name, _: &[Name], _, _: &lean_kernel::ExprStore| true;
    let g = GateInputs {
        axiom_registry: Some(&yes),
        ..GateInputs::default()
    };
    for bad in [
        "propext",
        "Classical.choice",
        "Quot.sound",
        "Warrant.Assume",
        "Warrant.Other.x",
        "Src.x",
        "Src",
        "warrant.Assume.x",
        "Foo.Warrant.Assume.x",
    ] {
        assert_eq!(err(&body(bad).bytes(), &g), E::AxiomNotAllowed, "{bad}");
    }
    // a Src.<profile>.<name> axiom is eligible
    ok(&body("Src.rocq.classic").bytes(), &g);
    // and with no registry at all, even a well-shaped assumption is refused
    assert_eq!(
        err(&body("Warrant.Assume.ok").bytes(), &GateInputs::default()),
        E::AxiomNotAllowed
    );
}

#[test]
fn r_ax_1_the_registry_sees_the_declared_type_so_a_wrong_statement_is_refused() {
    // a registered name declared with a different statement: Prop -> Prop instead of Prop
    let mut b = B::new();
    let prop = b.sort(0);
    let a = b.name("Warrant.Assume.ok");
    let x = b.name("x");
    let pi = b.pi(x, prop, prop);
    b.axiom(a, &[], pi);
    let wants_prop = |_: &Name, _: &[Name], ty, st: &lean_kernel::ExprStore| matches!(st.kind(ty), ExprKind::Sort(l) if l.is_zero());
    let g = GateInputs {
        axiom_registry: Some(&wants_prop),
        ..GateInputs::default()
    };
    assert_eq!(err(&b.bytes(), &g), E::AxiomNotAllowed);
}

#[test]
fn other_file_classes_may_declare_axioms() {
    // a prelude or model file (Reviewed) and a converted library (Library) declare axioms;
    // the gate classifies them in the footprint instead
    let mut b = B::new();
    b.prop_axiom("Foo");
    for class in [FileClass::Library, FileClass::Reviewed] {
        let g = GateInputs {
            class,
            ..GateInputs::default()
        };
        assert_eq!(ok(&b.bytes(), &g).decls.len(), 1);
    }
    assert_eq!(err(&b.bytes(), &GateInputs::default()), E::AxiomNotAllowed);
}

// ---- R-GOL-4 -----------------------------------------------------------------------------

#[test]
fn r_gol_4_a_claim_file_may_not_declare_a_constant_the_goals_mention() {
    let goal_consts = || HashSet::from([n("Pif.and"), n("Mine"), n("Mine.mk"), n("Mine.rec")]);
    let gate = |class| GateInputs {
        class,
        goal_constants: goal_consts(),
        ..GateInputs::default()
    };
    // a theorem or definition with a goal constant's name
    for name in ["Pif.and", "Mine"] {
        let mut b = B::new();
        let d = b.name(name);
        let one = b.lsucc(0);
        let ty = b.sort(one);
        let val = b.sort(0);
        b.def(d, &[], ty, val, "@abbrev");
        assert_eq!(
            err(&b.bytes(), &gate(FileClass::Claim)),
            E::GoalConstantRedeclared,
            "{name}"
        );
    }
    // an inductive type, a constructor or a derived recursor named like one
    let block = |ty: &str, ctor: &str| {
        let mut b = B::new();
        let t = b.name(ty);
        let mk = b.name(ctor);
        let one = b.lsucc(0);
        let s = b.sort(one);
        let tc = b.cnst(t, &[]);
        b.line("inductive [] 0 1");
        b.line(&format!("itype {t} {s} 1"));
        b.line(&format!("ictor {mk} {tc}"));
        b
    };
    assert_eq!(
        err(&block("Mine", "Mine.go").bytes(), &gate(FileClass::Claim)),
        E::GoalConstantRedeclared
    );
    assert_eq!(
        err(&block("Other", "Mine.mk").bytes(), &gate(FileClass::Claim)),
        E::GoalConstantRedeclared
    );
    // a recursor the kernel would derive with a goal constant's name
    let mut goal_rec = goal_consts();
    goal_rec.insert(n("Plain.rec"));
    let g = GateInputs {
        goal_constants: goal_rec,
        ..GateInputs::default()
    };
    assert_eq!(
        err(&block("Plain", "Plain.mk").bytes(), &g),
        E::GoalConstantRedeclared
    );
    // the same files are fine in the model file's class, where those constants are defined
    ok(
        &block("Mine", "Mine.go").bytes(),
        &gate(FileClass::Reviewed),
    );
    // and an unrelated name is fine in a claim file
    ok(&block("Fresh", "Fresh.mk").bytes(), &gate(FileClass::Claim));
}
