//! The per-claim record of the report (VER32 9.2 report fields, 8.3 certificate fields).
//!
//! For every claim the gate prints: the claim id, the theorem, the goal digest, the base and
//! full environment digests, the footprint with each axiom's class and bucket, and which
//! kernel produced the verdict (crate version and commit: finding 11 of the review record,
//! because an implementation bug withdraws evidence *by kernel version*, V3). Fields that
//! need later backlog items (the VER31 rungs) print as `"pending"`, never as a guess.
//!
//! # The kernel version and commit
//!
//! [`KERNEL_VERSION`] is the version of the `lean-kernel` crate this gate was built against.
//! A cargo dependency does not expose its version to the dependent at compile time, so it is
//! written here by hand and **a test compares it with the kernel's own manifest**: a bump of
//! one without the other fails the build. The commit comes from the build environment
//! variable `PIF_KERNEL_COMMIT` (set by the release build to `git rev-parse HEAD`); a build
//! without it reports `"unknown"` rather than inventing a value.

use crate::chain::Loaded;
use crate::cte::name_cte;
use crate::error::CheckError;
use crate::footprint::{classify, declared_in, AxClass, Classified, Footprinter};
use crate::goals::CheckedGoal;
use crate::json::Json;
use crate::registry::Registry;
use crate::report::dotted;

/// The `lean-kernel` crate version this gate was built against (checked by a test).
pub const KERNEL_VERSION: &str = "0.4.0";

/// The kernel commit, from the build environment, or `"unknown"`.
pub fn kernel_commit() -> &'static str {
    option_env!("PIF_KERNEL_COMMIT").unwrap_or("unknown")
}

/// Everything the report says about one claim.
#[derive(Clone, Debug)]
pub struct ClaimReport {
    pub goal: CheckedGoal,
    pub footprint: Vec<Classified>,
    /// `pif:sha256:<digest of the claim file>` (VER32 8.3 certificate id).
    pub certificate_id: String,
}

/// Build the claim reports: footprint and classification for every checked goal.
pub fn claim_reports(
    l: &Loaded,
    goals: &[CheckedGoal],
    registry: Option<&Registry>,
) -> Result<Vec<ClaimReport>, CheckError> {
    let decl_files = declared_in(l);
    let cert = l
        .files
        .last()
        .map(|f| format!("pif:sha256:{}", f.digest.hex()))
        .unwrap_or_default();
    // One memoising calculator for every claim: the shared prelude is walked once.
    let mut fp = Footprinter::new(l);
    goals
        .iter()
        .map(|g| {
            let axioms = fp.of(&g.theorem)?;
            Ok(ClaimReport {
                goal: g.clone(),
                footprint: classify(l, &axioms, &decl_files, registry),
                certificate_id: cert.clone(),
            })
        })
        .collect()
}

impl ClaimReport {
    /// The JSON of one claim.
    pub fn json(&self, l: &Loaded) -> Json {
        let c = self.footprint.len() as u64;
        let src = self.footprint.iter().any(|a| a.class == AxClass::Src);
        let axioms = self
            .footprint
            .iter()
            .map(|a| {
                Json::Obj(vec![
                    ("axiom", Json::s(dotted(&a.name))),
                    ("name_cte", Json::s(name_cte(&a.name))),
                    ("class", Json::s(a.class.word())),
                    ("bucket", Json::s(a.bucket.to_string())),
                    ("declared_in", Json::Num(a.file as u64)),
                    ("note", Json::s(a.note)),
                ])
            })
            .collect();
        let mut fields = vec![
            ("claim", Json::s(&self.goal.claim_id)),
            ("theorem", Json::s(dotted(&self.goal.theorem))),
            ("goal_digest", Json::s(self.goal.digest.hex())),
            ("goal_listed", Json::Bool(self.goal.listed)),
            ("base_environment", Json::s(l.base_environment.hex())),
            ("environment", Json::s(l.environment.hex())),
            ("footprint", Json::Arr(axioms)),
            (
                "buckets",
                Json::Obj(vec![
                    ("a", Json::Num(0)),
                    ("b", Json::Num(0)),
                    ("c", Json::Num(c)),
                ]),
            ),
        ];
        if src {
            fields.push((
                "consistency",
                Json::s("relative to Lean + these axioms; consistency with the kernel logic not established"),
            ));
        }
        fields.extend([
            ("certificate_id", Json::s(&self.certificate_id)),
            ("kernel_version", Json::s(KERNEL_VERSION)),
            ("kernel_commit", Json::s(kernel_commit())),
            ("rungs", Json::s("pending")),
        ]);
        Json::Obj(fields)
    }
}
