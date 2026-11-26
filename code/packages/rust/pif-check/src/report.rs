//! The machine-readable report.
//!
//! Success and failure share one shape of line so a consumer reads one channel:
//!
//! ```text
//!   {"ok":true,"base_environment":"<hex>","environment":"<hex>","allow_list":"checked",
//!    "files":[{"index":0,"role":"prelude","digest":"<hex>","declarations":N,"goals":0}],
//!    "claims":[{"claim":"<id>","theorem":"<name>","recorded_goal_digest":"<hex>"}]}
//!   {"ok":false,"error":"<code>","detail":"..."}
//! ```
//!
//! This is the report of the resolver and the chain. Later parts of PIF-003 (goal digests,
//! footprints, buckets) extend the `claims` entries; fields that need later backlog items
//! print as `"pending"`, never as a guess.

use crate::chain::Loaded;
use crate::error::CheckError;
use crate::json::Json;
use lean_kernel::Name;
use lean_kernel::NamePart;

/// A name as dotted text for humans. Components are joined with `.`; a numeric component
/// prints as its digits. (The *canonical* spelling, used for digests, is the name CTE.)
pub fn dotted(n: &Name) -> String {
    let parts: Vec<String> = n
        .parts()
        .iter()
        .map(|p| match p {
            NamePart::Str(s) => s.clone(),
            NamePart::Num(k) => k.to_string(),
        })
        .collect();
    parts.join(".")
}

/// The report of a fully loaded chain.
/// What was supplied, for the report.
pub struct Provided {
    pub allow_list: bool,
    pub signoff: bool,
    /// `"signed"`, `"allow-listed"` or `"unvetted"`.
    pub base_trust: &'static str,
}

pub fn success_report(l: &Loaded, p: &Provided) -> Json {
    let files = l
        .files
        .iter()
        .enumerate()
        .map(|(i, f)| {
            Json::Obj(vec![
                ("index", Json::Num(i as u64)),
                ("role", Json::s(f.role.word())),
                ("digest", Json::s(f.digest.hex())),
                ("declarations", Json::Num(f.decls.len() as u64)),
                ("goals", Json::Num(f.goals.len() as u64)),
            ])
        })
        .collect();
    let claims = l
        .files
        .iter()
        .flat_map(|f| f.goals.iter())
        .map(|g| {
            Json::Obj(vec![
                ("claim", Json::s(&g.claim_id)),
                ("theorem", Json::s(dotted(&g.theorem))),
                ("recorded_goal_digest", Json::s(&g.digest)),
            ])
        })
        .collect();
    Json::Obj(vec![
        ("ok", Json::Bool(true)),
        ("base_environment", Json::s(l.base_environment.hex())),
        ("environment", Json::s(l.environment.hex())),
        ("base_trust", Json::s(p.base_trust)),
        (
            "allow_list",
            Json::s(if p.allow_list {
                "checked"
            } else {
                "not-provided"
            }),
        ),
        // The sign-off's goal digests and aggregate are compared from PIF-003b on; until then
        // the report says so rather than claiming a check.
        (
            "signoff",
            Json::Obj(vec![
                ("provided", Json::Bool(p.signoff)),
                (
                    "base_environment",
                    Json::s(if p.signoff { "checked" } else { "absent" }),
                ),
                (
                    "goals",
                    Json::s(if p.signoff { "pending" } else { "absent" }),
                ),
                (
                    "aggregate",
                    Json::s(if p.signoff { "pending" } else { "absent" }),
                ),
            ]),
        ),
        ("files", Json::Arr(files)),
        ("claims", Json::Arr(claims)),
    ])
}

/// The report of a refusal.
pub fn error_report(e: &CheckError) -> Json {
    Json::Obj(vec![
        ("ok", Json::Bool(false)),
        ("error", Json::s(e.code())),
        ("detail", Json::s(e.detail())),
    ])
}
