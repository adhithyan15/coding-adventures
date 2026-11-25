//! # The adopter census (VER04r 16.2, slice S3b)
//!
//! VER04r 16.2 lists five places where what the evidence generator enumerates is not what
//! section 5.1 says, and 16.3 item 11e asked S3 to **report a census** so that S10 could decide
//! them with real documents in hand: which adopters have a binder whose domain is a list of
//! byte strings, `nat_count`, a signed edge set or a 16-bit edge set.  S10c decided items 3, 4 and
//! 5 ([D26], [D21], [D25]: described, no longer refused), so only a list of byte strings is still a
//! hit.  This module is that report, and a little more: for **every claim** of a document it says whether the S3
//! generator can emit it at `ir.core` and, if not, *why* (the first refusal: a level, a
//! VER04r generation rule, a cost bound, an over-budget product, a 16.2 descriptor).
//!
//! ```text
//!   WarrantFile ──lower_partial──▶ per item: lowered or the first refusal       (level, W0701, ...)
//!        │                                  └─ cost check on what lowered         (limit)
//!        └─ DomainEnv + claim_domains ──▶ per claim: product, descriptors,
//!                                         VER04r 16.2 items touched              (independent of lowering)
//! ```
//!
//! The 16.2 scan is independent of lowering on purpose: a claim over a `List<Bytes>` is
//! the reserved `ir.list` and is refused at lowering (S11b; it was `ir.data` before), but S10 still
//! needs to know it also touches item 1/2.  Since S11b the census counts what **lowers**: the
//! `ir.data` adopters (arithmetic, logic-gates) are `ok` here (S11c-2 evaluates `ir.data`, VER04r 16.14 R37; arithmetic
//! still breaks the eight-binder bound).  Arithmetic is `emittable()` by census but `generate` refuses it until the
//! adopter splits its nine-binder claim (R75).  [`render`] prints the report as text; `tests/census.rs` prints it for the eleven
//! adopters that bind `gen_evidence` and pins the facts the README quotes.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use coding_adventures_warrant_grammar::domain::DomainEnv;
use coding_adventures_warrant_grammar::parser::{Declaration, WarrantFile};

use crate::cost::{suite_cost, ModelCosts};
use crate::domains::{claim_domains, kind_name, s162_items};
use crate::error::GenError;
use crate::lower::{lower_partial, ItemKind};
use crate::profile::Profile;

/// One claim of a document.
#[derive(Debug, Clone)]
pub struct ClaimCensus {
    /// The claim's name.
    pub name: String,
    /// `None` if the S3 generator can emit the claim at `ir.core`; else the first refusal
    /// (lowering, then the cost bounds).  The domain is checked separately.
    pub blocked: Option<GenError>,
    /// The product ordinals and the descriptor of each binder, or the resolver's refusal
    /// (`W0706` over budget, `W0707` an over-wide `Bytes<N>`).
    pub domain: Result<(u64, Vec<String>), GenError>,
    /// The VER04r 16.2 items the binders touch that are still undecided: `"1/2"` only (S10c).
    pub s162: Vec<&'static str>,
}

impl ClaimCensus {
    /// Can S3 emit this claim: lowered, within cost, resolvable and free of 16.2?
    pub fn emittable(&self) -> bool {
        self.blocked.is_none() && self.domain.is_ok() && self.s162.is_empty()
    }

    /// The reason key used in the histogram: the level name, else the refusal code, else
    /// `16.2` or `ok`.
    pub fn reason_key(&self) -> String {
        if let Some(b) = &self.blocked {
            return b.level.map_or_else(|| b.code.to_string(), str::to_string);
        }
        if let Err(e) = &self.domain {
            return e.code.to_string();
        }
        if self.s162.is_empty() {
            "ok".into()
        } else {
            "16.2".into()
        }
    }
}

/// The census of one document.
#[derive(Debug, Clone)]
pub struct DocCensus {
    /// The module name.
    pub module: String,
    /// `v0` or `v1`.
    pub document: &'static str,
    /// Items other than claims that are refused (document-level declarations, operations,
    /// models, examples), as `(kind, name, refusal)`.
    pub other_blocks: Vec<(ItemKind, String, GenError)>,
    /// A model chain over the cost bound (it blocks every claim that calls it).
    pub cost_error: Option<GenError>,
    /// Every claim, in declaration order.
    pub claims: Vec<ClaimCensus>,
}

impl DocCensus {
    /// Claims the S3 generator can emit.
    pub fn emittable_claims(&self) -> usize {
        self.claims.iter().filter(|c| c.emittable()).count()
    }

    /// Does the document, as a whole, enumerate (every item within `ir.core`)?  A document
    /// is emitted whole or not at all (VER04r 13: fail closed).
    pub fn emittable(&self) -> bool {
        self.other_blocks.is_empty()
            && self.cost_error.is_none()
            && self.claims.iter().all(ClaimCensus::emittable)
    }
}

/// Census one parsed document against `profile` (its `budget` and `max_ir_cost`).
///
/// # Errors
///
/// Only a malformed document (checker or type error): a document with refusals is an
/// `Ok` whose rows carry them.
pub fn census_document(file: &WarrantFile<'_>, profile: &Profile) -> Result<DocCensus, GenError> {
    let partial = lower_partial(file)?;
    let rows = partial.census();
    let models = partial.lowered_models();
    let costs = ModelCosts::compute(&models, profile.max_ir_cost);
    let (costs, cost_error) = match costs {
        Ok(c) => (Some(c), None),
        Err(e) => (None, Some(e)),
    };
    let suites = partial.lowered_suites();
    let env = DomainEnv::new(file);
    let mut claims = Vec::new();
    for d in &file.declarations {
        let Declaration::Claim(c) = d else { continue };
        let mut blocked = rows
            .iter()
            .find(|r| r.kind == ItemKind::Claim && r.name == c.name)
            .and_then(|r| r.blocked.clone());
        if blocked.is_none() {
            blocked = match (&costs, suites.iter().find(|s| s.claim == c.name)) {
                (Some(costs), Some(s)) => suite_cost(s, costs, profile.max_ir_cost).err(),
                _ => cost_error.clone().map(|e| e.at(c.name)),
            };
        }
        let resolved = claim_domains(file, &env, c.name, profile.budget);
        let (domain, s162) = match resolved {
            Ok((ds, product)) => {
                let mut items: Vec<&'static str> = Vec::new();
                for dom in &ds {
                    for i in s162_items(dom) {
                        if !items.contains(&i) {
                            items.push(i);
                        }
                    }
                }
                items.sort_unstable();
                (Ok((product, ds.iter().map(kind_name).collect())), items)
            }
            Err(e) => (Err(e), Vec::new()),
        };
        claims.push(ClaimCensus {
            name: c.name.to_string(),
            blocked,
            domain,
            s162,
        });
    }
    let other_blocks = rows
        .into_iter()
        .filter(|r| r.kind != ItemKind::Claim)
        .filter_map(|r| r.blocked.map(|b| (r.kind, r.name, b)))
        .collect();
    Ok(DocCensus {
        module: partial.module().to_string(),
        document: partial.document(),
        other_blocks,
        cost_error,
        claims,
    })
}

fn reason_text(e: &GenError) -> String {
    match e.level {
        Some(level) => format!("level [{level}]: {}", e.message),
        None => format!("{}: {}", e.code, e.message),
    }
}

/// The key a refusal is grouped under: its level, else its code.
fn block_key(e: &GenError) -> String {
    e.level.map_or_else(|| e.code.to_string(), str::to_string)
}

/// Render the report.  Per document: a header, the refused non-claim items grouped by kind
/// and reason, and (`verbose`) one line per claim with its status, product and descriptors.
/// Then totals: claims by first blocker and every VER04r 16.2 hit by name.
///
/// A document that is not even well formed (`Err`) is reported as such.
pub fn render(docs: &[(String, Result<DocCensus, GenError>)], verbose: bool) -> String {
    let mut out = String::new();
    let mut by_reason: BTreeMap<String, usize> = BTreeMap::new();
    let mut by_item: BTreeMap<&'static str, Vec<String>> = BTreeMap::new();
    let (mut claims, mut emittable, mut docs_ok) = (0usize, 0usize, 0usize);
    for (label, doc) in docs {
        let d = match doc {
            Err(e) => {
                let _ = writeln!(out, "{label}: not well formed ({})", reason_text(e));
                continue;
            }
            Ok(d) => d,
        };
        let _ = writeln!(
            out,
            "{label} ({}, {}): {} claims, {} emittable at ir.core{}",
            d.module,
            d.document,
            d.claims.len(),
            d.emittable_claims(),
            if d.emittable() {
                ", document emittable"
            } else {
                ""
            }
        );
        docs_ok += usize::from(d.emittable());
        let mut grouped: BTreeMap<(String, String), Vec<&str>> = BTreeMap::new();
        for (kind, name, b) in &d.other_blocks {
            grouped
                .entry((format!("{kind:?}"), block_key(b)))
                .or_default()
                .push(name);
        }
        for ((kind, reason), names) in &grouped {
            let shown: Vec<&str> = names.iter().take(3).copied().collect();
            let more = if names.len() > 3 { ", ..." } else { "" };
            let _ = writeln!(
                out,
                "  {kind} blocked by {reason}: {} ({}{more})",
                names.len(),
                shown.join(", ")
            );
        }
        if let Some(e) = &d.cost_error {
            let _ = writeln!(out, "  models: {}", reason_text(e));
        }
        let mut doc_reasons: BTreeMap<String, usize> = BTreeMap::new();
        for c in &d.claims {
            claims += 1;
            emittable += usize::from(c.emittable());
            *by_reason.entry(c.reason_key()).or_default() += 1;
            *doc_reasons.entry(c.reason_key()).or_default() += 1;
            for i in &c.s162 {
                by_item
                    .entry(i)
                    .or_default()
                    .push(format!("{label}/{}", c.name));
            }
            if verbose {
                let status = match (&c.blocked, &c.domain) {
                    (Some(b), _) => format!("blocked {}", reason_text(b)),
                    (None, Err(e)) => format!("blocked {}", reason_text(e)),
                    (None, Ok(_)) if !c.s162.is_empty() => "blocked 16.2".to_string(),
                    (None, Ok(_)) => "ok".to_string(),
                };
                let shape = match &c.domain {
                    Ok((p, kinds)) => format!("P={p} [{}]", kinds.join(" x ")),
                    Err(_) => "P=?".to_string(),
                };
                let s162 = if c.s162.is_empty() {
                    String::new()
                } else {
                    format!(" 16.2 item {}", c.s162.join(","))
                };
                let _ = writeln!(out, "    {}: {status}; {shape}{s162}", c.name);
            }
        }
        let hist: Vec<String> = doc_reasons
            .iter()
            .map(|(r, n)| format!("{r} {n}"))
            .collect();
        let _ = writeln!(out, "  claims by first blocker: {}", hist.join(", "));
    }
    let _ = writeln!(
        out,
        "TOTAL: {} documents ({docs_ok} emittable whole), {claims} claims, {emittable} emittable at ir.core",
        docs.len(),
    );
    for (reason, n) in &by_reason {
        let _ = writeln!(out, "  claims by first blocker: {reason}: {n}");
    }
    for (item, names) in &by_item {
        let _ = writeln!(
            out,
            "  VER04r 16.2 item {item}: {} claims: {}",
            names.len(),
            names.join(" ")
        );
    }
    if by_item.is_empty() {
        let _ = writeln!(out, "  VER04r 16.2: no claim touches an undecided item (only 1/2, a list of byte strings, is left)");
    }
    out
}
