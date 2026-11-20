//! Compiled, public-export controls for the opt-in WX-FAULT2 adapter fault.

use coding_adventures_test_bootstrap::run::Options;
use wx_fault2_harness::{Fault, V};
use std::process::Command;

// The generated evidence module deliberately includes generic helpers and
// claim bodies that a single fixture may not exercise; keep generated-code
// lints scoped here, as with the other Warrant fixture harnesses.
#[allow(dead_code, unused_variables, unused_parens, unused_mut, clippy::all)]
mod wx_fault2_harness {
    pub mod wx_fault2_subject {
        #[derive(Clone, Debug, PartialEq, Eq)]
        pub struct Nested {
            pub enabled: bool,
            pub code: u8,
        }

        #[derive(Clone, Debug, PartialEq, Eq)]
        pub struct Status {
            pub ready: bool,
            pub visible: bool,
            pub nested: Nested,
            pub code: u8,
        }

        #[derive(Clone, Debug, PartialEq, Eq)]
        pub enum StatusError {
            Rejected,
        }

        pub fn direct(ready: bool, visible: bool, nested: Nested, code: u8) -> Status {
            Status { ready, visible, nested, code }
        }

        pub fn wrapped(
            accepted: bool,
            ready: bool,
            visible: bool,
            nested: Nested,
            code: u8,
        ) -> Result<Status, StatusError> {
            if accepted {
                Ok(direct(ready, visible, nested, code))
            } else {
                Err(StatusError::Rejected)
            }
        }

        pub fn optional(
            present: bool,
            ready: bool,
            visible: bool,
            nested: Nested,
            code: u8,
        ) -> Option<Status> {
            present.then(|| direct(ready, visible, nested, code))
        }

        pub fn plain(ready: bool, visible: bool, nested: Nested, code: u8) -> Status {
            direct(ready, visible, nested, code)
        }

        pub fn numeric(ready: bool, visible: bool, nested: Nested, code: u8) -> Status {
            direct(ready, visible, nested, code)
        }

        pub fn both(ready: bool, visible: bool, nested: Nested, code: u8) -> Status {
            direct(ready, visible, nested, code)
        }
    }

    include!("generated/wxfault2_evidence.rs");

    pub fn bool_flip(value: V) -> V {
        fault_bool_field_flip(value)
    }

    pub fn numeric_flip(value: V) -> V {
        fault_field_flip(value)
    }

    pub fn all_ops() -> &'static [&'static str] {
        HARNESS_OPS
    }

    pub fn bool_ops() -> &'static [&'static str] {
        HARNESS_BOOL_FIELD_FLIP_OPS
    }

    pub fn attack_check(
        attack: Option<&str>,
        fault: Option<&str>,
    ) -> Result<(), String> {
        check_attack(attack, fault, HARNESS_OPS)
    }

    pub fn bool_attack_check(
        attack: Option<&str>,
        fault: Option<&str>,
    ) -> Result<(), String> {
        check_bool_field_attack(attack, fault, HARNESS_BOOL_FIELD_FLIP_OPS)
    }

    pub fn raw_wrapped(accepted: bool, fault: Fault) -> V {
        let nested = V::Con("Nested", 0, "Nested", vec![V::Bool(false), V::Fixed(F::U8, 7)]);
        op_wrapped(&[], &[V::Bool(accepted), V::Bool(false), V::Bool(true), nested, V::Fixed(F::U8, 9)], fault)
    }

}

#[test]
fn boolean_flip_changes_only_first_direct_bool_and_preserves_wrappers() {
    let status = |ready: bool| {
        V::Con(
            "Status",
            0,
            "Status",
            vec![
                V::Bool(ready),
                V::Bool(true),
                V::Con("Nested", 0, "Nested", vec![V::Bool(false), V::Fixed(wx_fault2_harness::F::U8, 7)]),
                V::Fixed(wx_fault2_harness::F::U8, 9),
            ],
        )
    };
    let flipped = |ready: bool| {
        V::Con(
            "Status",
            0,
            "Status",
            vec![
                V::Bool(!ready),
                V::Bool(true),
                V::Con("Nested", 0, "Nested", vec![V::Bool(false), V::Fixed(wx_fault2_harness::F::U8, 7)]),
                V::Fixed(wx_fault2_harness::F::U8, 9),
            ],
        )
    };
    for input in [false, true] {
        assert_eq!(wx_fault2_harness::bool_flip(status(input)), flipped(input));
        assert_eq!(
            wx_fault2_harness::bool_flip(V::Ok(Box::new(status(input)))),
            V::Ok(Box::new(flipped(input)))
        );
        assert_eq!(
            wx_fault2_harness::bool_flip(V::Opt(Some(Box::new(status(input))))),
            V::Opt(Some(Box::new(flipped(input))))
        );
    }
    assert_eq!(wx_fault2_harness::bool_flip(V::Err("REJECTED".into())), V::Err("REJECTED".into()));
    assert_eq!(wx_fault2_harness::bool_flip(V::Opt(None)), V::Opt(None));

    let numeric = status(false);
    let expected_numeric = V::Con(
        "Status",
        0,
        "Status",
        vec![
            V::Bool(false),
            V::Bool(true),
            V::Con("Nested", 0, "Nested", vec![V::Bool(false), V::Fixed(wx_fault2_harness::F::U8, 7)]),
            V::Fixed(wx_fault2_harness::F::U8, 8),
        ],
    );
    assert_eq!(wx_fault2_harness::numeric_flip(numeric), expected_numeric);
}

#[test]
fn aimed_fault_must_select_a_bound_operation_and_generated_controls_are_nonvacuous() {
    assert_ne!(
        wx_fault2_harness::raw_wrapped(true, Fault::None),
        wx_fault2_harness::raw_wrapped(true, Fault::BoolFieldFlip),
        "the generated Result adapter should actually toggle its Ok payload"
    );
    assert_eq!(
        wx_fault2_harness::all_ops(),
        &["both", "direct", "numeric", "optional", "plain", "wrapped"]
    );
    assert_eq!(
        wx_fault2_harness::bool_ops(),
        &["both", "direct", "optional", "wrapped"]
    );
    assert!(wx_fault2_harness::attack_check(Some("direct"), Some("bool-field-flip")).is_ok());
    assert!(wx_fault2_harness::bool_attack_check(Some("direct"), Some("bool-field-flip")).is_ok());
    assert!(wx_fault2_harness::attack_check(Some("missing"), Some("bool-field-flip")).is_err());
    assert!(wx_fault2_harness::attack_check(Some("direct"), None).is_err());
    let err = wx_fault2_harness::bool_attack_check(Some("plain"), Some("bool-field-flip"))
        .unwrap_err();
    assert!(err.contains("does not bind fault"), "{err}");

    let (summary, outcomes) = wx_fault2_harness::build_suite(Fault::None)
        .run_recorded(&Options::default(), &mut Vec::new())
        .expect("recorded run");
    assert_eq!(summary.failed, 0, "failed outcomes: {:#?}", outcomes.iter().filter(|outcome| !outcome.passed()).collect::<Vec<_>>());
    let caught: Vec<_> = outcomes
        .iter()
        .filter(|outcome| outcome.id.contains("/negctl/") && outcome.id.ends_with("/bool-field-flip"))
        .collect();
    assert_eq!(caught.len(), 12, "each opted-in operation needs exhaustive, property, and fuzz controls");
    assert!(caught.iter().all(|outcome| outcome.passed()));
    assert!(outcomes.iter().any(|outcome| outcome.id.contains("numeric/field-flip")));
    assert!(outcomes.iter().any(|outcome| outcome.id.contains("both/field-flip")));
    assert!(outcomes.iter().any(|outcome| outcome.id.contains("direct_status_matches")));
    assert!(outcomes.iter().any(|outcome| outcome.id.contains("wrapped_status_matches")));
    assert!(outcomes.iter().any(|outcome| outcome.id.contains("optional_status_matches")));
}

#[test]
fn injected_fault_is_detected_by_the_arithmetic_claims_not_setup_failure() {
    let suite = wx_fault2_harness::build_suite(Fault::BoolFieldFlip);
    let (summary, outcomes) = suite
        .run_recorded(&Options::default(), &mut Vec::new())
        .expect("recorded run");
    let failed_claims: Vec<_> = outcomes
        .iter()
        .filter(|outcome| !outcome.passed() && !outcome.id.contains("/negctl/"))
        .collect();
    assert!(summary.failed > 0 && !failed_claims.is_empty());
    assert!(failed_claims.iter().any(|outcome| outcome.id.contains("direct_status_matches")));
}

#[test]
fn aimed_existing_but_non_opted_operation_exits_two_in_a_fresh_process() {
    if std::env::var_os("WX_FAULT2_RUN_MAIN_CHILD").is_some() {
        let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let code = wx_fault2_harness::run_main(&src);
        std::process::exit(code);
    }

    let output = Command::new(std::env::current_exe().expect("test executable"))
        .args([
            "--exact",
            "aimed_existing_but_non_opted_operation_exits_two_in_a_fresh_process",
            "--nocapture",
        ])
        .env("WX_FAULT2_RUN_MAIN_CHILD", "1")
        .env("WARRANT_EVIDENCE_FAULT", "bool-field-flip")
        .env("WARRANT_EVIDENCE_ATTACK", "plain")
        .output()
        .expect("spawn isolated generated-harness check");
    assert_eq!(output.status.code(), Some(2), "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("does not bind fault `bool-field-flip`"),
        "stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
}
