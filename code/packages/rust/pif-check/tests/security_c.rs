//! Security-review regression for PIF-003c: footprints are memoised across claims, so thousands
//! of claims over one prelude do not re-walk it once per claim.

mod common;

use common::*;
use lean_kernel::Name;
use pif_check::chain::{load_chain, LoadOptions};
use pif_check::claims::claim_reports;
use pif_check::control::Role;
use pif_check::cte::goal_digest;
use pif_check::footprint::{footprint, Footprinter};
use pif_check::goals::check_goals;
use pif_check::resolver::Resolver;
use pif_check::Digest;
use pif_writer::{Hint, Level, Writer};

fn n(s: &str) -> Name {
    Name::from_dotted(s)
}

fn axioms_file(ph: &str) -> String {
    let mut w = Writer::new("native", "axioms").unwrap().ascii_names_only();
    w.import(ph).unwrap();
    let (f, b) = (w.name("False").unwrap(), w.name("Bool").unwrap());
    let (fc, bc) = (w.constant(f, &[]), w.constant(b, &[]));
    let a = w.name("Pif.A").unwrap();
    w.axiom(a, &[], fc).unwrap();
    // A definition that uses the axiom, so footprints must follow definitions.
    let ac = w.constant(a, &[]);
    let d = w.name("Pif.D").unwrap();
    w.definition(d, &[], fc, ac, Hint::Regular(1)).unwrap();
    let _ = (bc, Level::ZERO);
    w.to_text()
}

fn claims(imports: &[String], n_claims: usize, hint: &str) -> String {
    let mut w = Writer::new("native", "claims").unwrap().ascii_names_only();
    for i in imports {
        w.import(i).unwrap();
    }
    let f = w.name("False").unwrap();
    let fc = w.constant(f, &[]);
    let d = w.name("Pif.D").unwrap();
    let dc = w.constant(d, &[]);
    for i in 0..n_claims {
        let t = w.name(&format!("Pif.t{i}")).unwrap();
        w.theorem(t, &[], fc, dc).unwrap();
        w.goal(&format!("c.{i}"), t, hint).unwrap();
    }
    w.to_text()
}

#[test]
fn finding_6_footprints_are_memoised_across_claims() {
    let start = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| {
            let p = prelude();
            let ph = Digest::of(&p).hex();
            let ax = axioms_file(&ph);
            let imports = [ph, Digest::of(ax.as_bytes()).hex()];
            let probe = claims(&imports, 1, &Digest::of(b"x").hex());
            let load = |c: &str| {
                let m = mem(&[&p, ax.as_bytes(), c.as_bytes()]);
                let chain = [
                    entry(Role::Prelude, &p),
                    entry(Role::Axioms, ax.as_bytes()),
                    entry(Role::Claim, c.as_bytes()),
                ];
                load_chain(&Resolver::new(&m), &chain, &LoadOptions::default()).unwrap()
            };
            let l = load(&probe);
            let ty = l.env.get(&n("Pif.t0")).unwrap().ty;
            let hint = goal_digest(&l.env.store, ty).unwrap().hex();
            const N: usize = 10_000;
            let l = load(&claims(&imports, N, &hint));
            let goals = check_goals(&l, None, None).unwrap();
            let reports = claim_reports(&l, &goals, None).unwrap();
            assert_eq!(reports.len(), N);
            assert!(reports
                .iter()
                .all(|r| r.footprint.len() == 1 && r.footprint[0].name == n("Pif.A")));
            // The memoised walker agrees with the one-shot function, including across queries.
            let mut f = Footprinter::new(&l);
            for name in [
                "Pif.D", "Pif.A", "Pif.t17", "Bool", "Bool.rec", "Eq.refl", "Pif.and",
            ] {
                let direct = footprint(&l, &n(name)).unwrap();
                assert_eq!(f.of(&n(name)).unwrap(), direct, "{name}");
                assert_eq!(f.of(&n(name)).unwrap(), direct, "{name} (memo hit)");
            }
            assert_eq!(f.of(&n("No.such")).err().unwrap().code(), "UnknownConstant");
        })
        .unwrap()
        .join()
        .unwrap();
    assert!(
        start.elapsed() < std::time::Duration::from_secs(120),
        "took {:?}",
        start.elapsed()
    );
}
