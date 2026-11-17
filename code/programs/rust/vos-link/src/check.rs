//! # Judging a document: CD01 to CD07
//!
//! [`verify`] takes the bytes of a document (the one `gen-doc` is about to write, or the
//! one on disk) and the loaded inputs, and runs the checks of VOS02 section 10.1, D-GD8.
//! Nothing here reads a file; it is the pure library plus the contract package's text.
//!
//! The order matters only where a later check needs an earlier result:
//!
//! ```text
//!   CD01 marker skeleton ──► CD02 digests ──► CD03 copies equal the contract's parts
//!        │ (stops here when the skeleton is unreadable)
//!        ▼
//!   CD04 warrant-grammar parse + type check + trailer ──► CD05 same name, different claim
//!        │ (stops here when Warrant refuses)                CD06 carries, from parsed claims
//!        ▼                                                  CD07 R6 by kind
//! ```
//!
//! ## `carries` is computed, never read
//!
//! A tier is *carried* when every obligation of that tier appears in the document with
//! the contract's `claim_sha256`.  The `tier=` attribute of a marker is never consulted:
//! a claim moved between blocks, or a block relabelled, changes nothing about what is
//! carried (VOS02 section 2.3).

use std::collections::BTreeMap;

use vos_manifest::document::{document_name, parse_layout, TRAILER};
use vos_manifest::warrant::{
    analyze_document_with_budget, Analysis, AssumeKind, BuildShape, ClaimFact, WarrantFailure,
    TAINT_BUDGET,
};
use vos_manifest::Tier;

use crate::error::{Finding, NoVerdict};
use crate::fsio::esc;
use crate::inputs::{ImplKind, Loaded};

/// The stack the Warrant tools run on.
///
/// `warrant-grammar`'s checker recurses over the expression tree, and a long chain of
/// `and`s overflows a 2 MiB stack at a few hundred terms (backlog VOS-D057).  The Warrant
/// token budget (65,536) bounds the depth; 256 MiB is far above what that allows, and
/// costs address space, not memory, until it is used.
const WARRANT_STACK_BYTES: usize = 256 * 1024 * 1024;

/// What verification found.
#[derive(Debug, Default)]
pub struct Verified {
    /// The findings, in the order of the checks.
    pub findings: Vec<Finding>,
    /// The tiers the document carries, computed from its parsed claims.
    pub carries: Vec<Tier>,
}

/// Run the Warrant tools on a big stack.
fn analyze_big_stack(
    text: String,
    budget: u64,
) -> Result<Result<Analysis, WarrantFailure>, NoVerdict> {
    let handle = std::thread::Builder::new()
        .name("vos-link-warrant".to_string())
        .stack_size(WARRANT_STACK_BYTES)
        .spawn(move || analyze_document_with_budget(&text, budget))
        .map_err(|e| NoVerdict::new(format!("cannot start the Warrant analysis thread: {e}")))?;
    handle
        .join()
        .map_err(|_| NoVerdict::new("the Warrant analysis thread panicked"))
}

/// The line at which two texts first differ (1-based), for the drift message.
pub fn first_difference_line(a: &str, b: &str) -> usize {
    let mut la = a.split('\n');
    let mut lb = b.split('\n');
    let mut n = 1;
    loop {
        match (la.next(), lb.next()) {
            (Some(x), Some(y)) if x == y => n += 1,
            _ => return n,
        }
    }
}

/// Verify a document against the loaded inputs.  `CD08` (regeneration) is the caller's:
/// it needs the on-disk bytes and the freshly assembled ones.
pub fn verify(loaded: &Loaded, doc: &[u8]) -> Result<Verified, NoVerdict> {
    verify_with_budget(loaded, doc, TAINT_BUDGET)
}

/// [`verify`] with an explicit step budget for the cone walk (tests use a tiny one to
/// exercise the give-up path, which a real document cannot reach).
pub fn verify_with_budget(loaded: &Loaded, doc: &[u8], budget: u64) -> Result<Verified, NoVerdict> {
    let mut v = Verified::default();

    // --- CD01: the marker skeleton ---------------------------------------------------
    let layout = match parse_layout(doc) {
        Ok(l) => l,
        Err(e) if e.is_cap() => return Err(NoVerdict::new(format!("document: {e}"))),
        Err(e) => {
            v.findings.push(Finding::new(
                "CD01",
                format!("{}: {e}", loaded.layout.doc_display()),
            ));
            return Ok(v);
        }
    };
    // `doc` is ASCII (the layout admitted it), so this cannot fail.
    let text = String::from_utf8_lossy(doc).into_owned();
    let impl_name = document_name(&loaded.layout.impl_pkg).unwrap_or_default();
    let contract_name = document_name(&loaded.contract_pkg).unwrap_or_default();
    if layout.header != format!("warrant {impl_name} v1") {
        v.findings.push(Finding::new(
            "CD01",
            format!(
                "line 1: the header is {}, expected `warrant {impl_name} v1`",
                esc(&layout.header)
            ),
        ));
    }
    if layout.contract_package != contract_name {
        v.findings.push(Finding::new(
            "CD01",
            format!(
                "the generated marker names contract package {}, expected `{contract_name}`",
                esc(&layout.contract_package)
            ),
        ));
    }

    // --- CD02 / CD03: digests, and copies equal the contract's parts -------------------
    let mut blocks = vec![(
        "contract block".to_string(),
        &layout.contract,
        loaded.contract_part.text(),
    )];
    for (tier, block) in &layout.obligations {
        match loaded.tier_parts.iter().find(|(t, _)| t == tier) {
            Some((_, part)) => blocks.push((
                format!("obligations block `{}`", tier.name()),
                block,
                part.text(),
            )),
            None => v.findings.push(Finding::new(
                "CD03",
                format!(
                    "line {}: the obligations block `{}` is not a tier of contract `{}`",
                    block.begin_line,
                    tier.name(),
                    loaded.contract_pkg
                ),
            )),
        }
    }
    for (label, block, expect) in &blocks {
        let computed = block.computed_sha256();
        if block.marker_sha256.as_deref() != Some(computed.as_str()) {
            v.findings.push(Finding::new(
                "CD02",
                format!(
                    "line {}: the {label} records sha256={} but its bytes hash to {computed}",
                    block.begin_line,
                    block.marker_sha256.as_deref().unwrap_or("(none)")
                ),
            ));
        }
        if block.body != *expect {
            let at = first_difference_line(&block.body, expect);
            v.findings.push(Finding::new(
                "CD03",
                format!(
                    "line {}: the {label} differs from the contract package's text (first difference at line {at} of the block)",
                    block.begin_line
                ),
            ));
        }
    }

    // --- CD04: warrant-grammar ----------------------------------------------------------
    let analysis = match analyze_big_stack(text, budget)? {
        Ok(a) => a,
        Err(e) => {
            v.findings.push(Finding::new(
                "CD04",
                format!("warrant-grammar refused the document: {e}"),
            ));
            return Ok(v);
        }
    };
    let trailer_ok = analysis.assumptions.len() == 2
        && analysis.assumptions[0].kind == AssumeKind::Platform
        && analysis.assumptions[1].kind == AssumeKind::Environment
        && analysis
            .assumptions
            .iter()
            .all(|a| a.is_none && a.start >= layout.trailer_offset);
    if !trailer_ok {
        v.findings.push(Finding::new(
            "CD04",
            format!(
                "the document must have exactly one `assume platform: none` and one `assume environment: none`, after the last marker (an implementation assumes nothing, R7); found {} assume row(s)",
                analysis.assumptions.len()
            ),
        ));
    } else if layout.trailer != TRAILER {
        v.findings.push(Finding::new(
            "CD04",
            "the trailer after the last marker is not the generated one",
        ));
    }

    // --- CD05: the same name with a different claim ------------------------------------
    let mut contract_claims: BTreeMap<&str, (&str, Tier)> = BTreeMap::new();
    for (tier, claims) in &loaded.tier_claims {
        for c in claims {
            contract_claims.insert(&c.name, (&c.sha256, *tier));
        }
    }
    for c in &analysis.claims {
        if let Some((sha, tier)) = contract_claims.get(c.name.as_str()) {
            if c.sha256 != *sha {
                v.findings.push(Finding::new(
                    "CD05",
                    format!(
                        "claim `{}` has the name of the {} obligation `{}` but a different claim_sha256 (a weakened or changed claim)",
                        c.name,
                        tier.name(),
                        c.name
                    ),
                ));
            }
        }
    }

    // --- CD06: carries, from the parsed claims -----------------------------------------
    let present = |c: &ClaimFact| {
        analysis
            .claims
            .iter()
            .any(|d| d.name == c.name && d.sha256 == c.sha256)
    };
    let mut counts: BTreeMap<Tier, (usize, usize)> = BTreeMap::new();
    for (tier, claims) in &loaded.tier_claims {
        let have = claims.iter().filter(|c| present(c)).count();
        counts.insert(*tier, (have, claims.len()));
    }
    for (tier, (have, total)) in &counts {
        if *have == *total && *total > 0 {
            v.carries.push(*tier);
        } else if *have > 0 {
            v.findings.push(Finding::new(
                "CD06",
                format!(
                    "tier `{}` is only partly carried: {have} of {total} obligations appear with the contract's claim_sha256",
                    tier.name()
                ),
            ));
        }
    }
    if v.carries != loaded.obligations {
        let names = |ts: &[Tier]| ts.iter().map(|t| t.name()).collect::<Vec<_>>().join(",");
        v.findings.push(Finding::new(
            "CD06",
            format!(
                "the document carries [{}] but vos.toml obligations is [{}]",
                names(&v.carries),
                names(&loaded.obligations)
            ),
        ));
    }

    // --- CD07: R6, as far as the document goes ------------------------------------------
    r6(loaded, &analysis, &counts, &mut v.findings);
    Ok(v)
}

/// VOS02 R6 and section 10.1, D-GD8: the rules a document alone decides.
fn r6(
    loaded: &Loaded,
    a: &Analysis,
    counts: &BTreeMap<Tier, (usize, usize)>,
    out: &mut Vec<Finding>,
) {
    let named_any = loaded.layout.impl_pkg.ends_with("-any");
    let has = |t: Tier| counts.get(&t).is_some_and(|(h, _)| *h > 0);
    let full = |t: Tier| counts.get(&t).is_some_and(|(h, n)| *h == *n && *n > 0);
    let mut cd07 = |m: String| out.push(Finding::new("CD07", m));
    let cone_failure = |what: &str| format!("the `build` model {what}");
    match loaded.kind {
        ImplKind::Production => {
            if named_any {
                cd07("a crate named `-any` declares kind = \"production\" (an `-any` crate is kind any, R6)".into());
            }
            if !loaded.contract.has_tier(Tier::Canonical) {
                cd07(
                    "kind production needs a `canonical` tier, which the contract does not define"
                        .into(),
                );
            } else if !full(Tier::Canonical) {
                cd07(
                    "kind production: the document does not carry the `canonical` obligations (R6)"
                        .into(),
                );
            }
            if has(Tier::Any) {
                cd07("kind production: the document carries an `any` obligation; a production crate carries no `any` block (R6, NC5)".into());
            }
            match &a.build {
                BuildShape::Missing => cd07(cone_failure("is missing, so the cone of `Impl` cannot be checked")),
                BuildShape::NotConstructor => cd07(cone_failure(
                    "is not an `Impl(..)` constructor application, so the cone cannot be decided structurally (fails closed)",
                )),
                BuildShape::BudgetExhausted => cd07(cone_failure(
                    "exhausted the taint-walk step budget, so the cone cannot be decided (fails closed)",
                )),
                BuildShape::WideModel => cd07(cone_failure(
                    "calls a model with more than 63 parameters, which the cone walk cannot tell apart, so the cone cannot be decided (fails closed)",
                )),
                BuildShape::Constructor { in_cone } => {
                    let outside: Vec<String> = in_cone
                        .iter()
                        .enumerate()
                        .filter(|(_, c)| !**c)
                        .map(|(i, _)| i.to_string())
                        .collect();
                    if !outside.is_empty() {
                        cd07(format!(
                            "kind production: field(s) {} of `Impl` are not set from the abstract state by `build`: an oracle or history register outside the cone (R6 rule 2)",
                            outside.join(", ")
                        ));
                    }
                }
            }
        }
        ImplKind::Any => {
            if !named_any {
                cd07("kind any: the crate name does not end in `-any` (R6 lint)".into());
            }
            if !loaded.contract.has_tier(Tier::Any) {
                cd07("kind any needs an `any` tier, which the contract does not define".into());
            } else if !full(Tier::Any) {
                cd07("kind any: the document does not carry the `any` obligations (R6)".into());
            }
            if has(Tier::Canonical) {
                cd07("kind any: the document carries a `canonical` claim; an `any` crate may not (R6, NC4b)".into());
            }
            match &a.build {
                BuildShape::Missing => cd07(cone_failure("is missing, so the oracle field cannot be found")),
                BuildShape::NotConstructor => cd07(cone_failure(
                    "is not an `Impl(..)` constructor application, so the oracle field cannot be found (fails closed)",
                )),
                BuildShape::BudgetExhausted => cd07(cone_failure(
                    "exhausted the taint-walk step budget, so the oracle field cannot be found (fails closed)",
                )),
                BuildShape::WideModel => cd07(cone_failure(
                    "calls a model with more than 63 parameters, so the oracle field cannot be found (fails closed)",
                )),
                BuildShape::Constructor { in_cone } => {
                    if !in_cone.iter().any(|c| !*c) {
                        cd07("kind any: every field of `Impl` is set from the abstract state; there is no oracle field outside the cone (R6 rule 1)".into());
                    }
                }
            }
        }
        ImplKind::Reference => {
            if named_any {
                cd07("a crate named `-any` declares kind = \"reference\"".into());
            }
            if a.bound_declarations > 0 {
                cd07(format!(
                    "kind reference: the document has {} `bound` declaration(s); a reference model has none (R6)",
                    a.bound_declarations
                ));
            }
        }
    }
}
