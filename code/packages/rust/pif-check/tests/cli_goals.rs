//! PIF-003b through the command line: the goals list and the aggregate in the report.

mod common;

use common::*;
use lean_kernel::pif::{read, GateInputs};
use lean_kernel::Name;
use pif_check::cli::run;
use pif_check::Digest;

fn s(v: &[&str]) -> Vec<String> {
    v.iter().map(|x| (*x).to_string()).collect()
}

#[test]
fn the_goals_list_and_the_aggregate_appear_in_the_report() {
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
    let fixture = include_str!("../../lean-kernel/tests/fixtures/worked-example.pif");
    let r = read(
        lean_kernel::Environment::new(),
        fixture.as_bytes(),
        &GateInputs::default(),
    )
    .unwrap();
    let ty = r
        .decls
        .iter()
        .find(|d| d.name == Name::from_dotted("Pif.and_comm"))
        .unwrap()
        .ty;
    let display = pif_check::cte::display_cte_from_store(&r.env.store, ty).unwrap();
    let goals = t.put(
        "goals.txt",
        format!("gates.and_comm {display}\n").as_bytes(),
    );
    let store = t.0.to_string_lossy().into_owned();
    let out = run(&s(&[
        "--unsafe-no-allow",
        "--store",
        &store,
        "--chain",
        &chain,
        "--goals",
        &goals,
    ]));
    assert_eq!(out.code, 0, "{}", out.stdout);
    assert!(out
        .stdout
        .contains(&format!("\"goal_digest\":\"{WORKED_DIGEST}\"")));
    assert!(out.stdout.contains("\"goals_list\":\"checked\""));
    assert!(out.stdout.contains("\"goal_listed\":true"));
    let base = Digest::env_start().fold(&Digest::of(&p));
    let line = format!("gates.and_comm {WORKED_DIGEST} {}\n", base.hex());
    assert!(out.stdout.contains(&format!(
        "\"goals_sha256\":\"{}\"",
        Digest::of(line.as_bytes()).hex()
    )));
    // A goals list for a different statement is refused.
    let bad = t.put(
        "bad.txt",
        format!("gates.and_comm {}\n", display.replace("\"Eq\"", "\"Ex\"")).as_bytes(),
    );
    let out = run(&s(&[
        "--unsafe-no-allow",
        "--store",
        &store,
        "--chain",
        &chain,
        "--goals",
        &bad,
    ]));
    assert_eq!(out.code, 1);
    assert!(out.stdout.contains("\"error\":\"DisplayBindingMismatch\""));
    // A malformed goals list is a refusal, not a panic.
    let junk = t.put("junk.txt", b"nonsense");
    let out = run(&s(&[
        "--unsafe-no-allow",
        "--store",
        &store,
        "--chain",
        &chain,
        "--goals",
        &junk,
    ]));
    assert!(out.stdout.contains("\"error\":\"BadGoalsList\""));
}
