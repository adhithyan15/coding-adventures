//! Security-review regressions for PIF-003b: the sign-off checks are reported, and goals lists
//! and sign-offs are processed in linear time with a cap on their size.

mod common;

use common::*;
use pif_check::chain::{load_chain, LoadOptions};
use pif_check::cli::run;
use pif_check::control::{Role, Signoff};
use pif_check::cte::display_cte_from_store;
use pif_check::goals::{aggregate, check_goals, parse_goals_list};
use pif_check::resolver::Resolver;
use pif_check::Digest;
use pif_writer::{Level, Writer};

fn raw(v: &[&str]) -> Vec<String> {
    v.iter().map(|x| (*x).to_string()).collect()
}

/// A claim file over the prelude with `n` theorems `Pif.t<i> : Eq Bool true true`, each bound to
/// claim `c.<i>`. All share one statement, so one goal digest.
fn many_goals(prelude_hex: &str, n: usize, digest: &str) -> String {
    let mut w = Writer::new("native", "many 0.1")
        .unwrap()
        .ascii_names_only();
    w.import(prelude_hex).unwrap();
    let (b, tt) = (w.name("Bool").unwrap(), w.name("Bool.true").unwrap());
    let (eq, refl) = (w.name("Eq").unwrap(), w.name("Eq.refl").unwrap());
    let one = w.level_succ(Level::ZERO);
    let (bc, ttc) = (w.constant(b, &[]), w.constant(tt, &[]));
    let (eq1, refl1) = (w.constant(eq, &[one]), w.constant(refl, &[one]));
    let ty = w.app_many(eq1, &[bc, ttc, ttc]);
    let pf = w.app_many(refl1, &[bc, ttc]);
    for i in 0..n {
        let t = w.name(&format!("Pif.t{i}")).unwrap();
        w.theorem(t, &[], ty, pf).unwrap();
        w.goal(&format!("c.{i}"), t, digest).unwrap();
    }
    w.to_text()
}

fn on_big_stack<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(f)
        .unwrap()
        .join()
        .unwrap()
}

#[test]
fn finding_6_many_goals_are_checked_in_linear_time_and_the_list_is_capped() {
    let start = std::time::Instant::now();
    on_big_stack(|| {
        let p = prelude();
        let ph = Digest::of(&p).hex();
        // The gate computes the digest; learn it from a one-goal file.
        let one = many_goals(&ph, 1, &Digest::of(b"x").hex());
        let m = mem(&[&p, one.as_bytes()]);
        let chain = [entry(Role::Prelude, &p), entry(Role::Claim, one.as_bytes())];
        let l = load_chain(&Resolver::new(&m), &chain, &LoadOptions::default()).unwrap();
        let ty = l
            .env
            .get(&lean_kernel::Name::from_dotted("Pif.t0"))
            .unwrap()
            .ty;
        let digest = pif_check::cte::goal_digest(&l.env.store, ty).unwrap().hex();
        let display = display_cte_from_store(&l.env.store, ty).unwrap();

        const N: usize = 20_000;
        let big = many_goals(&ph, N, &digest);
        let m = mem(&[&p, big.as_bytes()]);
        let chain = [entry(Role::Prelude, &p), entry(Role::Claim, big.as_bytes())];
        let l = load_chain(&Resolver::new(&m), &chain, &LoadOptions::default()).unwrap();
        let list_text: String = (0..N).map(|i| format!("c.{i} {display}\n")).collect();
        let list = parse_goals_list(&list_text).unwrap();
        let goals = check_goals(&l, Some(&list), None).unwrap();
        assert_eq!(goals.len(), N);
        // A sign-off that lists them all.
        let so = Signoff {
            base: vec![],
            base_environment: Some(l.base_environment),
            goals: goals
                .iter()
                .map(|g| (g.claim_id.clone(), g.digest))
                .collect(),
            goals_sha256: Some(aggregate(&goals, &l.base_environment)),
        };
        assert!(check_goals(&l, Some(&list), Some(&so)).is_ok());
    });
    assert!(
        start.elapsed() < std::time::Duration::from_secs(120),
        "took {:?}",
        start.elapsed()
    );

    // The cap: one line more than the limit is refused (and quickly).
    let start = std::time::Instant::now();
    let text: String = (0..100_001).map(|i| format!("a{i} v0\n")).collect();
    let e = parse_goals_list(&text).err().unwrap();
    assert_eq!(e.code(), "BadGoalsList");
    assert!(e.detail().contains("too many"), "{}", e.detail());
    assert!(start.elapsed() < std::time::Duration::from_secs(30));
    let text: String = (0..100_000).map(|i| format!("a{i} v0\n")).collect();
    assert_eq!(parse_goals_list(&text).unwrap().len(), 100_000);
}

#[test]
fn finding_2_the_report_says_which_signoff_parts_were_compared() {
    let t = Tmp::new();
    let p = prelude();
    let c = claim_over_prelude(
        &[Digest::of(&p).hex()],
        "gates.and_comm",
        WORKED_DIGEST,
        "Pif.and_comm",
    );
    t.store(&p);
    t.store(c.as_bytes());
    let chain = t.put(
        "chain.txt",
        format!(
            "pif-chain 1\nprelude sha256:{}\nclaim sha256:{}\n",
            Digest::of(&p).hex(),
            Digest::of(c.as_bytes()).hex()
        )
        .as_bytes(),
    );
    let store = t.0.to_string_lossy().into_owned();
    let base = Digest::env_start().fold(&Digest::of(&p));
    let agg =
        Digest::of(format!("gates.and_comm {WORKED_DIGEST} {}\n", base.hex()).as_bytes()).hex();
    // Without a sign-off the aggregate is computed but not verified.
    let out = run(&raw(&[
        "--unsafe-no-allow",
        "--store",
        &store,
        "--chain",
        &chain,
    ]));
    assert_eq!(out.code, 0, "{}", out.stdout);
    assert!(
        out.stdout.contains(&format!(
            "\"goals_sha256\":\"{agg}\",\"goals_sha256_verified\":false"
        )),
        "{}",
        out.stdout
    );
    // With a complete sign-off every part is checked, and the aggregate is verified.
    let so = t.put(
        "so.txt",
        format!(
            "pif-signoff 1\nbase prelude sha256:{}\nbase-environment {}\ngoal gates.and_comm {WORKED_DIGEST}\ngoals-sha256 {agg}\n",
            Digest::of(&p).hex(),
            base.hex()
        )
        .as_bytes(),
    );
    let out = run(&raw(&[
        "--store",
        &store,
        "--chain",
        &chain,
        "--signoff",
        &so,
    ]));
    assert_eq!(out.code, 0, "{}", out.stdout);
    assert!(out.stdout.contains("\"signoff\":{\"provided\":true,\"base_environment\":\"checked\",\"goals\":\"checked\",\"aggregate\":\"checked\"}"), "{}", out.stdout);
    assert!(out.stdout.contains("\"goals_sha256_verified\":true"));
    // A sign-off whose aggregate is wrong is refused.
    let bad = t.put(
        "bad.txt",
        format!(
            "pif-signoff 1\nbase prelude sha256:{}\nbase-environment {}\ngoal gates.and_comm {WORKED_DIGEST}\ngoals-sha256 {WORKED_DIGEST}\n",
            Digest::of(&p).hex(),
            base.hex()
        )
        .as_bytes(),
    );
    let out = run(&raw(&[
        "--store",
        &store,
        "--chain",
        &chain,
        "--signoff",
        &bad,
    ]));
    assert_eq!(out.code, 1);
    assert!(out.stdout.contains("AggregateMismatch"));
}
