//! PIF-003b: the canonical term encoding, goal digests, the goals list and the aggregate.

mod common;

use common::*;
use lean_kernel::pif::{read, GateInputs};
use lean_kernel::{Binder, BinderInfo, Environment, Level, Name};
use pif_check::chain::{load_chain, LoadOptions, Loaded};
use pif_check::control::Role;
use pif_check::cte::{
    binding_cte, display_cte_from_store, goal_digest, name_cte, quote, CteError, Dterm,
    MAX_CTE_BYTES,
};
use pif_check::goals::{
    aggregate, check_goals, constants_of, list_constants, parse_goals_list, CheckedGoal,
};
use pif_check::resolver::Resolver;
use pif_check::{CheckError, Digest};

const WORKED: &str = include_str!("../../lean-kernel/tests/fixtures/worked-example.pif");

fn n(s: &str) -> Name {
    Name::from_dotted(s)
}

/// The worked example read alone (it is a one-file illustration); returns the environment and
/// the theorem's type.
fn worked() -> (Environment, lean_kernel::Expr) {
    let r = read(
        Environment::new(),
        WORKED.as_bytes(),
        &GateInputs::default(),
    )
    .unwrap();
    let ty = r
        .decls
        .iter()
        .find(|d| d.name == n("Pif.and_comm"))
        .unwrap()
        .ty;
    (r.env, ty)
}

#[test]
fn the_worked_examples_goal_digest_is_the_value_in_the_spec() {
    let (env, ty) = worked();
    let cte = binding_cte(&env.store, ty).unwrap();
    // VER32 3.6 quotes the beginning of this encoding.
    assert!(
        cte.starts_with("P(C<\"Bool\">[],P(C<\"Bool\">[],A(A(A(C<\"Eq\">[s(z)],C<\"Bool\">[]),"),
        "{cte}"
    );
    // VER32 4.6: the `goal` record's digest.
    assert_eq!(goal_digest(&env.store, ty).unwrap().hex(), WORKED_DIGEST);
    // The whole encoding, pinned (no spaces, no names, levels syntactic).
    assert_eq!(
        cte,
        "P(C<\"Bool\">[],P(C<\"Bool\">[],A(A(A(C<\"Eq\">[s(z)],C<\"Bool\">[]),\
         A(A(C<\"Pif\",\"and\">[],v1),v0)),A(A(C<\"Pif\",\"and\">[],v0),v1))))"
    );
}

#[test]
fn the_display_form_keeps_names_and_info_and_erases_to_the_binding_form() {
    let (env, ty) = worked();
    let display = display_cte_from_store(&env.store, ty).unwrap();
    assert!(
        display.starts_with("Pd(<\"a\">,C<\"Bool\">[],Pd(<\"b\">,C<\"Bool\">[],"),
        "{display}"
    );
    let t = Dterm::parse(&display).unwrap();
    assert_eq!(t.display().unwrap(), display);
    assert_eq!(t.binding().unwrap(), binding_cte(&env.store, ty).unwrap());
    assert_eq!(t, Dterm::from_store(&env.store, ty).unwrap());
}

/// Build `Pi (x : Sort u) (y : Sort v), Sort (max u v)` in a store with the given binder names.
fn pi_term(names: (&str, &str), lv: (&str, &str)) -> (Environment, lean_kernel::Expr) {
    let mut env = Environment::new();
    let st = &mut env.store;
    let (u, v) = (Level::param(&n(lv.0)), Level::param(&n(lv.1)));
    let (su, sv, m) = (st.sort(&u), st.sort(&v), st.sort(&Level::max(&u, &v)));
    let b = |s: &str| Binder {
        name: n(s),
        info: BinderInfo::Default,
    };
    let inner = st.pi(b(names.1), sv, m);
    let outer = st.pi(b(names.0), su, inner);
    (env, outer)
}

#[test]
fn renaming_binders_leaves_the_binding_digest_unchanged() {
    let (e1, t1) = pi_term(("x", "y"), ("u", "v"));
    let (e2, t2) = pi_term(("alpha", "beta"), ("u", "v"));
    assert_eq!(
        goal_digest(&e1.store, t1).unwrap(),
        goal_digest(&e2.store, t2).unwrap()
    );
    // ...but the display forms differ, as they must.
    assert_ne!(
        display_cte_from_store(&e1.store, t1).unwrap(),
        display_cte_from_store(&e2.store, t2).unwrap()
    );
}

#[test]
fn changing_a_level_or_a_constant_changes_the_digest_and_levels_are_syntactic() {
    let (e1, t1) = pi_term(("x", "y"), ("u", "v"));
    let (e2, t2) = pi_term(("x", "y"), ("u", "w")); // a different level parameter
    assert_ne!(
        goal_digest(&e1.store, t1).unwrap(),
        goal_digest(&e2.store, t2).unwrap()
    );
    // max u v and max v u are different terms and different digests (no levels_equiv).
    let mut env = Environment::new();
    let st = &mut env.store;
    let (u, v) = (Level::param(&n("u")), Level::param(&n("v")));
    let (a, b) = (st.sort(&Level::max(&u, &v)), st.sort(&Level::max(&v, &u)));
    assert_ne!(
        goal_digest(&env.store, a).unwrap(),
        goal_digest(&env.store, b).unwrap()
    );
    // A different constant, same shape.
    let (c1, c2) = (
        env.store.constant(&n("A"), &[]),
        env.store.constant(&n("B"), &[]),
    );
    assert_ne!(
        goal_digest(&env.store, c1).unwrap(),
        goal_digest(&env.store, c2).unwrap()
    );
    // And a changed constant in the worked example changes its digest.
    let (wenv, wty) = worked();
    let cte = binding_cte(&wenv.store, wty)
        .unwrap()
        .replace("\"Eq\"", "\"Ex\"");
    assert_ne!(Digest::of(cte.as_bytes()).hex(), WORKED_DIGEST);
}

#[test]
fn every_term_form_round_trips_through_the_display_encoding() {
    let mut env = Environment::new();
    let st = &mut env.store;
    let u = Level::param(&n("u"));
    let lv = Level::imax(&Level::succ(&u), &Level::max(&Level::zero(), &u));
    let s = st.sort(&lv);
    let c = st.constant(&Name::num(&n("A.B"), 7), &[u.clone(), lv.clone()]);
    let b = |s: &str, i| Binder {
        name: n(s),
        info: i,
    };
    let app = st.app(c, s);
    let lam = st.lam(b("f", BinderInfo::Implicit), s, app);
    let pi = st.pi(b("g", BinderInfo::StrictImplicit), lam, c);
    let v0 = st.bvar(0);
    let let_ = st.let_e(b("z", BinderInfo::Default), pi, c, v0);
    let proj = st.proj(&n("P.S"), 3, let_);
    let lit = st.str_lit("a \"q\" \\ \u{e9}");
    let top = st.app(proj, lit);
    let inst = st.pi(b("i", BinderInfo::InstImplicit), top, v0);
    let t = Dterm::from_store(&env.store, inst).unwrap();
    let text = t.display().unwrap();
    assert!(text.is_ascii() && !text.contains(' '));
    let back = Dterm::parse(&text).unwrap();
    assert_eq!(back, t);
    assert_eq!(back.display().unwrap(), text);
    // erase(display) = binding computed straight from the store.
    assert_eq!(
        back.binding().unwrap(),
        binding_cte(&env.store, inst).unwrap()
    );
}

#[test]
fn strings_and_names_have_one_spelling() {
    assert_eq!(quote("a b"), "\"a\\u{20}b\"");
    assert_eq!(quote("\"\\"), "\"\\\"\\\\\"");
    assert_eq!(quote("\u{430}"), "\"\\u{430}\"");
    assert_eq!(name_cte(&Name::anonymous()), "<>");
    assert_eq!(name_cte(&Name::num(&n("A"), 3)), "<\"A\",3>");
    assert_ne!(
        name_cte(&Name::num(&n("A"), 3)),
        name_cte(&Name::str(&n("A"), "3"))
    );
}

#[test]
fn the_parser_is_strict_and_total() {
    let good = "Pd(<\"a\">,C<\"Bool\">[],v0)";
    assert!(Dterm::parse(good).is_ok());
    for bad in [
        "",
        "v",
        "v01",
        "v0 ",
        " v0",
        "v0v0",
        "A(v0,v1",
        "A(v0 ,v1)",
        "Px(<>,v0,v0)",
        "L(v0,v0)",
        "C<\"a\">",
        "C<\"a\">[s(z]",
        "C<\"a\">[z,]",
        "S",
        "Sq",
        "Sp",
        "Sp<\"u\"",
        "T\"\\u{41}\"",
        "T\"\\u{0041}\"",
        "T\"\\u{D}\"",
        "T\"\\u{d800}\"",
        "T\"\\u{110000}\"",
        "T\"\\x\"",
        "T\"a",
        "T\"\u{e9}\"",
        "J(<>,70000,v0)",
        "J(<>,1,v0",
        "N",
        "N01",
        "C<1,\"a\",>[]",
        "C<99999999999999999999>[]",
        "Z(<>,v0,v0)",
        "v99999999999",
        "v2147483647",
        "Q",
    ] {
        assert!(Dterm::parse(bad).is_err(), "{bad:?} must be refused");
    }
    // An escaped space and an escaped non-ASCII letter are the canonical spellings.
    assert!(Dterm::parse("T\"a\\u{20}b\\u{e9}\"").is_ok());
    // A zero escape is allowed in a literal string (R-LEX-3 allows it only in estr).
    assert!(Dterm::parse("T\"\\u{0}\"").is_ok());
}

#[test]
fn the_size_cap_is_enforced_when_building_emitting_and_parsing() {
    // 40 doublings of `f x` share their children in the store: 2^40 nodes as a tree.
    let mut env = Environment::new();
    let st = &mut env.store;
    let mut t = st.constant(&n("c"), &[]);
    for _ in 0..40 {
        t = st.app(t, t);
    }
    assert_eq!(binding_cte(&env.store, t), Err(CteError::TooLarge));
    assert_eq!(goal_digest(&env.store, t), Err(CteError::TooLarge));
    // A just-too-long text is refused without being scanned.
    let long = format!("T\"{}\"", "a".repeat(MAX_CTE_BYTES));
    assert_eq!(Dterm::parse(&long), Err(CteError::TooLarge));
    // A deep chain of applications is walked without the call stack (depth 30,000).
    let mut chain = String::new();
    for _ in 0..30_000 {
        chain.push_str("A(");
    }
    chain.push_str("v0");
    for _ in 0..30_000 {
        chain.push_str(",v0)");
    }
    let t = Dterm::parse(&chain).unwrap();
    assert_eq!(t.display().unwrap(), chain);
    // The same shape, built in the store (depth 30,000, no sharing) and unfolded.
    let mut env = Environment::new();
    let st = &mut env.store;
    let (mut e, leaf) = (st.bvar(0), st.bvar(0));
    for _ in 0..30_000 {
        e = st.app(e, leaf);
    }
    assert_eq!(binding_cte(&env.store, e).unwrap().len(), 30_000 * 6 + 2);
}

#[test]
fn a_free_variable_is_refused() {
    let mut env = Environment::new();
    let id = env.store.fresh_fvar_id();
    let f = env.store.fvar(id);
    assert_eq!(binding_cte(&env.store, f), Err(CteError::FreeVariable));
}

#[test]
fn literals_have_an_encoding_too() {
    let mut env = Environment::new();
    let a = env.store.nat_lit(lean_kernel_bignum(12345));
    assert_eq!(binding_cte(&env.store, a).unwrap(), "N12345");
    assert_eq!(Dterm::parse("N12345").unwrap().display().unwrap(), "N12345");
}

fn lean_kernel_bignum(v: u64) -> bignum_core::BigInteger {
    bignum_core::BigInteger::from(v)
}

// ---- the goals list, the sign-off and the aggregate --------------------------------------

fn loaded_with_claim(claim_id: &str, hint: &str) -> (Loaded, Vec<u8>, String) {
    let p = prelude();
    let c = claim_over_prelude(&[Digest::of(&p).hex()], claim_id, hint, "Pif.and_comm");
    let m = mem(&[&p, c.as_bytes()]);
    let chain = [entry(Role::Prelude, &p), entry(Role::Claim, c.as_bytes())];
    let l = load_chain(&Resolver::new(&m), &chain, &LoadOptions::default()).unwrap();
    (l, p, c)
}

fn worked_display(l: &Loaded) -> String {
    let ty = l.env.get(&n("Pif.and_comm")).unwrap().ty;
    display_cte_from_store(&l.env.store, ty).unwrap()
}

#[test]
fn goals_are_recomputed_and_the_record_digest_is_only_a_hint() {
    let (l, _, _) = loaded_with_claim("gates.and_comm", WORKED_DIGEST);
    let goals = check_goals(&l, None, None).unwrap();
    assert_eq!(goals.len(), 1);
    assert_eq!(goals[0].digest.hex(), WORKED_DIGEST);
    assert!(!goals[0].listed);
    // A wrong hint is an error, and the computed digest is the right one.
    let wrong = Digest::of(b"wrong").hex();
    let (l, _, _) = loaded_with_claim("gates.and_comm", &wrong);
    assert_eq!(
        check_goals(&l, None, None).err().unwrap(),
        CheckError::GoalDigestMismatch {
            claim: "gates.and_comm".into(),
            recorded: wrong,
            computed: WORKED_DIGEST.into()
        }
    );
}

#[test]
fn the_goals_list_is_parsed_and_compared_with_the_claim_files_goals() {
    let (l, _, _) = loaded_with_claim("gates.and_comm", WORKED_DIGEST);
    let line = format!("gates.and_comm {}\n", worked_display(&l));
    let list = parse_goals_list(&line).unwrap();
    assert_eq!(list.len(), 1);
    assert!(list_constants(&list).contains(&n("Pif.and")));
    let goals = check_goals(&l, Some(&list), None).unwrap();
    assert!(goals[0].listed);
    // The display CTE may use any binder names: only erase(display) matters.
    let renamed = worked_display(&l)
        .replace("<\"a\">", "<\"p\">")
        .replace("<\"b\">", "<\"q\">");
    let list = parse_goals_list(&format!("gates.and_comm {renamed}\n")).unwrap();
    assert!(check_goals(&l, Some(&list), None).is_ok());
    // A display CTE that erases to a different term is refused.
    let other = worked_display(&l).replace("\"Eq\"", "\"Ex\"");
    let list = parse_goals_list(&format!("gates.and_comm {other}\n")).unwrap();
    assert_eq!(
        check_goals(&l, Some(&list), None).err().unwrap(),
        CheckError::DisplayBindingMismatch {
            claim: "gates.and_comm".into()
        }
    );
    // Level spelling is part of the goal.
    let other = worked_display(&l).replace("s(z)", "m(s(z),z)");
    let list = parse_goals_list(&format!("gates.and_comm {other}\n")).unwrap();
    assert_eq!(
        check_goals(&l, Some(&list), None).err().unwrap().code(),
        "DisplayBindingMismatch"
    );
    // The claim is missing from the list, and the list has a claim the file does not.
    let list = parse_goals_list(&format!("other.claim {}\n", worked_display(&l))).unwrap();
    assert_eq!(
        check_goals(&l, Some(&list), None).err().unwrap(),
        CheckError::GoalNotInList("gates.and_comm".into())
    );
    let two = format!("gates.and_comm {0}\nother.claim {0}\n", worked_display(&l));
    let list = parse_goals_list(&two).unwrap();
    assert_eq!(
        check_goals(&l, Some(&list), None).err().unwrap(),
        CheckError::ListedGoalMissing("other.claim".into())
    );
}

#[test]
fn the_goals_list_format_is_strict() {
    let d = "v0";
    assert_eq!(parse_goals_list("").unwrap(), vec![]);
    assert!(parse_goals_list(&format!("a {d}\nb {d}\n")).is_ok());
    for bad in [
        format!("a {d}"),            // no final LF
        format!("a  {d}\n"),         // doubled space
        format!("a {d} x\n"),        // a third field
        "a\n".to_string(),           // no CTE
        format!("a {d}\n\nb {d}\n"), // blank line
        format!("bad id {d}\n"),     // space in the id
        format!("a/b {d}\n"),        // bad id charset
        format!("{} {d}\n", "x".repeat(129)),
        format!(" a {d}\n"),
        format!("a {d}\r\n"),
        "a T\"\u{e9}\"\n".to_string(),
        "a v0v0\n".to_string(),
    ] {
        assert!(parse_goals_list(&bad).is_err(), "{bad:?}");
    }
    assert_eq!(
        parse_goals_list(&format!("a {d}\na {d}\n")).err().unwrap(),
        CheckError::DuplicateListedGoal("a".into())
    );
    assert_eq!(parse_goals_list("a Q\n").err().unwrap().code(), "CteFailed");
}

#[test]
fn a_goal_in_a_base_file_is_refused() {
    // A "model" file that carries a goal record: models hold definitions only.
    let p = prelude();
    // A small printable file: theorem `Pif.t0 : Eq Bool true true := Eq.refl Bool true`.
    let mut w = pif_writer::Writer::new("native", "model 0.1")
        .unwrap()
        .ascii_names_only();
    w.import(&Digest::of(&p).hex()).unwrap();
    let (b, tt) = (w.name("Bool").unwrap(), w.name("Bool.true").unwrap());
    let (eq, refl) = (w.name("Eq").unwrap(), w.name("Eq.refl").unwrap());
    let one = w.level_succ(pif_writer::Level::ZERO);
    let (bc, ttc) = (w.constant(b, &[]), w.constant(tt, &[]));
    let (eq1, refl1) = (w.constant(eq, &[one]), w.constant(refl, &[one]));
    let ty = w.app_many(eq1, &[bc, ttc, ttc]);
    let pf = w.app_many(refl1, &[bc, ttc]);
    let t0 = w.name("Pif.t0").unwrap();
    w.theorem(t0, &[], ty, pf).unwrap();
    w.goal("c.t0", t0, &Digest::of(b"x").hex()).unwrap();
    let c = w.to_text();
    let m = mem(&[&p, c.as_bytes()]);
    let chain = [entry(Role::Prelude, &p), entry(Role::Model, c.as_bytes())];
    let l = load_chain(&Resolver::new(&m), &chain, &LoadOptions::default()).unwrap();
    assert_eq!(
        check_goals(&l, None, None).err().unwrap(),
        CheckError::GoalInBaseFile { index: 1 }
    );
}

#[test]
fn the_aggregate_is_the_hash_of_the_sorted_lines() {
    let base = Digest::of(b"base");
    let g = |id: &str, d: &[u8]| CheckedGoal {
        claim_id: id.into(),
        theorem: n("T"),
        digest: Digest::of(d),
        listed: false,
    };
    let goals = [g("b.claim", b"2"), g("a.claim", b"1")];
    let expect = format!(
        "a.claim {} {}\nb.claim {} {}\n",
        Digest::of(b"1").hex(),
        base.hex(),
        Digest::of(b"2").hex(),
        base.hex()
    );
    assert_eq!(aggregate(&goals, &base), Digest::of(expect.as_bytes()));
    // Order of the input does not matter; the base environment does.
    assert_eq!(
        aggregate(&[goals[1].clone(), goals[0].clone()], &base),
        aggregate(&goals, &base)
    );
    assert_ne!(
        aggregate(&goals, &Digest::of(b"other")),
        aggregate(&goals, &base)
    );
    // No claims: the hash of nothing.
    assert_eq!(aggregate(&[], &base), Digest::of(b""));
}

#[test]
fn a_signoff_is_checked_against_the_computed_goals_in_both_directions() {
    use pif_check::control::Signoff;
    let (l, _, _) = loaded_with_claim("gates.and_comm", WORKED_DIGEST);
    let goals = check_goals(&l, None, None).unwrap();
    let agg = aggregate(&goals, &l.base_environment);
    // Built field by field: a *parsed* sign-off must be complete, but these cases isolate one
    // comparison each.
    let so = |body: String| {
        let mut s = Signoff::default();
        for line in body.lines() {
            match line.split(' ').collect::<Vec<_>>().as_slice() {
                ["goal", id, d] => s.goals.push(((*id).to_string(), Digest::parse(d).unwrap())),
                ["goals-sha256", d] => s.goals_sha256 = Digest::parse(d),
                _ => panic!("{line}"),
            }
        }
        s
    };
    let ok = so(format!(
        "goal gates.and_comm {WORKED_DIGEST}\ngoals-sha256 {}\n",
        agg.hex()
    ));
    assert!(check_goals(&l, None, Some(&ok)).is_ok());
    let wrong = Digest::of(b"x").hex();
    assert_eq!(
        check_goals(
            &l,
            None,
            Some(&so(format!("goal gates.and_comm {wrong}\n")))
        )
        .err()
        .unwrap(),
        CheckError::SignedGoalMismatch {
            claim: "gates.and_comm".into(),
            signed: wrong.clone(),
            computed: WORKED_DIGEST.into()
        }
    );
    assert_eq!(
        check_goals(&l, None, Some(&so(format!("goal other {wrong}\n"))))
            .err()
            .unwrap(),
        CheckError::GoalNotSigned("gates.and_comm".into())
    );
    assert_eq!(
        check_goals(
            &l,
            None,
            Some(&so(format!(
                "goal gates.and_comm {WORKED_DIGEST}\ngoal extra {wrong}\n"
            )))
        )
        .err()
        .unwrap(),
        CheckError::SignedGoalMissing("extra".into())
    );
    assert_eq!(
        check_goals(&l, None, Some(&so(format!("goals-sha256 {wrong}\n"))))
            .err()
            .unwrap(),
        CheckError::AggregateMismatch {
            expected: wrong,
            actual: agg.hex()
        }
    );
}

#[test]
fn a_claim_file_may_not_redeclare_a_constant_of_a_goal() {
    // The claim declares `Pif.mine` and its own goal mentions it. With the goals list the
    // reader refuses before any record is accepted; without it the gate refuses afterwards.
    use pif_writer::Writer;
    let p = prelude();
    let mut w = Writer::new("native", "claim 0.1")
        .unwrap()
        .ascii_names_only();
    w.import(&Digest::of(&p).hex()).unwrap();
    let bool_n = w.name("Bool").unwrap();
    let mine = w.name("Pif.mine").unwrap();
    let ty = w.constant(bool_n, &[]);
    let tt = w.name("Bool.true").unwrap();
    let val = w.constant(tt, &[]);
    w.definition(mine, &[], ty, val, pif_writer::Hint::Regular(1))
        .unwrap();
    // theorem Pif.t : Eq Bool mine mine := Eq.refl Bool mine
    let (eq_n, refl) = (w.name("Eq").unwrap(), w.name("Eq.refl").unwrap());
    let one = w.level_succ(pif_writer::Level::ZERO);
    let mine_c = w.constant(mine, &[]);
    let eq1 = w.constant(eq_n, &[one]);
    let e = w.app_many(eq1, &[ty, mine_c, mine_c]);
    let r1 = w.constant(refl, &[one]);
    let pf = w.app_many(r1, &[ty, mine_c]);
    let thm = w.name("Pif.t").unwrap();
    w.theorem(thm, &[], e, pf).unwrap();
    // The gate computes the digest itself; the hint here is irrelevant to this test.
    w.goal("c.t", thm, &Digest::of(b"hint").hex()).unwrap();
    let c = w.to_text();
    let m = mem(&[&p, c.as_bytes()]);
    let chain = [entry(Role::Prelude, &p), entry(Role::Claim, c.as_bytes())];
    // Without a goals list the reader cannot know the goal's constants; the gate's own check does.
    let l = load_chain(&Resolver::new(&m), &chain, &LoadOptions::default()).unwrap();
    let ty = l.env.get(&n("Pif.t")).unwrap().ty;
    assert!(constants_of(&l.env.store, ty).contains(&n("Pif.mine")));
    let hint = goal_digest(&l.env.store, ty).unwrap();
    let e = check_goals(&l, None, None).err().unwrap();
    // (the recorded hint is wrong, which is reported first; fix the hint and retry)
    assert_eq!(e.code(), "GoalDigestMismatch");
    let c2 = c.replace(&Digest::of(b"hint").hex(), &hint.hex());
    let m2 = mem(&[&p, c2.as_bytes()]);
    let chain2 = [entry(Role::Prelude, &p), entry(Role::Claim, c2.as_bytes())];
    let l2 = load_chain(&Resolver::new(&m2), &chain2, &LoadOptions::default()).unwrap();
    assert_eq!(
        check_goals(&l2, None, None).err().unwrap(),
        CheckError::GoalConstantRedeclared("Pif.mine".into())
    );
    // With the goals list, the reader itself refuses the file (R-GOL-4).
    let list = parse_goals_list(&format!(
        "c.t {}\n",
        display_cte_from_store(&l2.env.store, l2.env.get(&n("Pif.t")).unwrap().ty).unwrap()
    ))
    .unwrap();
    let opts = LoadOptions {
        goal_constants: list_constants(&list),
        ..LoadOptions::default()
    };
    let e = load_chain(&Resolver::new(&m2), &chain2, &opts)
        .err()
        .unwrap();
    assert_eq!(
        e,
        CheckError::Read {
            index: 1,
            role: "claim",
            error: lean_kernel::pif::PifError::GoalConstantRedeclared
        }
    );
}
