//! Every error variant has a distinct stable code and a one-line detail.

use lean_kernel::pif::PifError;
use pif_check::cte::CteError;
use pif_check::CheckError;

fn s(x: &str) -> String {
    x.to_string()
}

#[test]
fn every_variant_has_a_distinct_code_and_a_one_line_detail() {
    let errs = vec![
        CheckError::BadDigestSyntax(s("x")),
        CheckError::FileMissing(s("d")),
        CheckError::DigestMismatch {
            expected: s("a"),
            actual: s("b"),
        },
        CheckError::FileTooLarge(s("d")),
        CheckError::NotRegularFile(s("d")),
        CheckError::Io {
            digest: s("d"),
            kind: s("k"),
        },
        CheckError::BadControlFile {
            what: "chain",
            line: 1,
            why: "w",
        },
        CheckError::EmptyChain,
        CheckError::ChainTooLong,
        CheckError::PreludeNotFirst,
        CheckError::ClaimNotLast,
        CheckError::RoleOrder { index: 1 },
        CheckError::DuplicateRole { index: 1 },
        CheckError::NotAllowListed {
            index: 0,
            digest: s("d"),
        },
        CheckError::SignoffChainMismatch,
        CheckError::SignoffBaseEnvironmentMismatch {
            expected: s("a"),
            actual: s("b"),
        },
        CheckError::Read {
            index: 0,
            role: "prelude",
            error: PifError::ImportMismatch,
        },
        CheckError::Cte {
            claim: s("c"),
            error: CteError::TooLarge,
        },
        CheckError::BadGoalsList { line: 2, why: "w" },
        CheckError::DuplicateListedGoal(s("c")),
        CheckError::GoalInBaseFile { index: 1 },
        CheckError::GoalDigestMismatch {
            claim: s("c"),
            recorded: s("a"),
            computed: s("b"),
        },
        CheckError::DisplayBindingMismatch { claim: s("c") },
        CheckError::GoalNotInList(s("c")),
        CheckError::ListedGoalMissing(s("c")),
        CheckError::GoalNotSigned(s("c")),
        CheckError::SignedGoalMismatch {
            claim: s("c"),
            signed: s("a"),
            computed: s("b"),
        },
        CheckError::SignedGoalMissing(s("c")),
        CheckError::AggregateMismatch {
            expected: s("a"),
            actual: s("b"),
        },
        CheckError::GoalConstantRedeclared(s("n")),
    ];
    let codes: std::collections::BTreeSet<_> = errs.iter().map(CheckError::code).collect();
    // DuplicateListedGoal deliberately shares the reader's name `DuplicateGoal` but is its own variant.
    assert_eq!(codes.len(), errs.len());
    for e in &errs {
        let d = e.detail();
        assert!(!d.is_empty() && !d.contains('\n'), "{e:?}");
    }
}
