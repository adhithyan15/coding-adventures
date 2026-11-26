//! The milestone of PIF-002: the VER32 section 4.6 worked example is read into a fresh
//! environment, and the kernel checks, from the file's bytes alone, that `Pif.and` is
//! commutative. Nothing in this test trusts the file: the theorem is accepted only because
//! the kernel type-checked a proof term the reader built from the records.

mod pif_common;

use lean_kernel::pif::{read, GateInputs, Kind};
use lean_kernel::{DeclKind, Environment, Name};
use pif_common::{err, ok};

const EXAMPLE: &[u8] = include_bytes!("fixtures/worked-example.pif");

fn n(s: &str) -> Name {
    Name::from_dotted(s)
}

#[test]
fn the_worked_example_is_accepted_with_the_strictest_rules() {
    // The default gate inputs are a claim file: ASCII names, no axioms, no imports.
    let r = ok(EXAMPLE, &GateInputs::default());
    let got: Vec<(Kind, String)> = r
        .decls
        .iter()
        .map(|d| (d.kind, d.name.to_string()))
        .collect();
    let want = [
        (Kind::Inductive, "Bool"),
        (Kind::Constructor, "Bool.false"),
        (Kind::Constructor, "Bool.true"),
        (Kind::Recursor, "Bool.rec"),
        (Kind::Inductive, "Eq"),
        (Kind::Constructor, "Eq.refl"),
        (Kind::Recursor, "Eq.rec"),
        (Kind::Definition, "Pif.and"),
        (Kind::Theorem, "Pif.and_comm"),
    ];
    let want: Vec<(Kind, String)> = want.iter().map(|(k, s)| (*k, s.to_string())).collect();
    assert_eq!(got, want);
    assert_eq!(
        r.profile,
        ("native".to_string(), "pif-case-split 0.1".to_string())
    );
}

#[test]
fn the_commutativity_theorem_is_in_the_environment_as_a_theorem() {
    let r = ok(EXAMPLE, &GateInputs::default());
    let d = r.env.get(&n("Pif.and_comm")).expect("theorem accepted");
    // The kernel's own record, not the reader's report: a THEOREM, with a proof value.
    assert!(matches!(d.kind, DeclKind::Theorem { .. }));
    assert!(d.value().is_some());
    // `Bool.rec` was derived by the kernel; the file never declared it.
    assert!(matches!(
        r.env.get(&n("Bool.rec")).unwrap().kind,
        DeclKind::Recursor(_)
    ));
    // And the goal binding was recorded against exactly that theorem.
    assert_eq!(r.goals.len(), 1);
    assert_eq!(r.goals[0].claim_id, "gates.and_comm");
    assert_eq!(r.goals[0].theorem, n("Pif.and_comm"));
    assert_eq!(
        r.goals[0].digest,
        "bf1fa0a0d828b019965f46efa44ff2627bf54a59558a1bc2959b52c161f74676"
    );
}

#[test]
fn the_report_type_handles_belong_to_the_returned_environment() {
    let r = ok(EXAMPLE, &GateInputs::default());
    let last = r.decls.last().unwrap();
    assert!(r.env.store.owns(last.ty));
    assert_eq!(r.env.get(&last.name).unwrap().ty, last.ty);
}

#[test]
fn the_theorem_is_checked_not_assumed_a_corrupted_proof_is_rejected_by_the_kernel() {
    // Swap the two constructors in the proof's final leaf by editing the text: still
    // well-formed PIF, but no longer a proof. The reader must hand it to the kernel and
    // the kernel must refuse; if the theorem were added unchecked this would pass.
    let text = String::from_utf8(EXAMPLE.to_vec()).unwrap();
    // Record 43 is `eapp 43 42 17` (Eq.refl Bool true-side); change which constructor it uses.
    let broken = text.replace("eapp 43 42 17", "eapp 43 42 18");
    assert_ne!(broken, text);
    let e = err(broken.as_bytes(), &GateInputs::default());
    assert!(matches!(e, lean_kernel::pif::PifError::Kernel(_)), "{e:?}");
}

#[test]
fn nothing_is_returned_on_error_so_a_half_read_environment_cannot_leak() {
    // `read` consumes the environment; on error there is nothing to keep using.
    let truncated = &EXAMPLE[..EXAMPLE.len() - 7]; // drops "end 97\n"
    assert!(read(Environment::new(), truncated, &GateInputs::default()).is_err());
}
