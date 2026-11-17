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
//!        ▼                                                  CD07 R6 by kind (CD07-R by mode)
//! ```
//!
//! ## `carries` is computed, never read
//!
//! A tier is *carried* when every obligation of that tier appears in the document with
//! the contract's `claim_sha256`.  The `tier=` attribute of a marker is never consulted:
//! a claim moved between blocks, or a block relabelled, changes nothing about what is
//! carried (VOS02 section 2.3).

use std::collections::{BTreeMap, BTreeSet};

use vos_manifest::document::{document_name, parse_layout, TRAILER};
use vos_manifest::warrant::{
    analyze_document_with_budget, Analysis, AssumeKind, BuildShape, CallFact, ClaimFact,
    ClaimShape, WarrantFailure, TAINT_BUDGET,
};
use vos_manifest::Tier;

use crate::error::{Finding, NoVerdict};
use crate::fsio::esc;
use crate::inputs::{ImplKind, Loaded, Mode};

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
    /// For `mode = representation`: each representation claim (`rep_init`, `rep_alpha`,
    /// `<op>_rep`, `<op>_det`) with its `claim_sha256`, in document order, so a report can
    /// print it as `impl-own rep` and the lock (VOS-034) can pin it.  Empty otherwise.
    pub rep_claims: Vec<ClaimFact>,
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
    if loaded.mode == Mode::Representation && loaded.kind != ImplKind::Any {
        v.rep_claims = analysis
            .claims
            .iter()
            .filter(|c| is_rep_claim_name(&c.name))
            .cloned()
            .collect();
    }
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
    // `mode = representation` replaces the canonical and cone rules of production by CD07-R
    // (VOS02 R6, rule 2).  A reference crate keeps its own rule (no `bound` rows) below.
    let representation = loaded.mode == Mode::Representation;
    match loaded.kind {
        ImplKind::Production if representation => r6_representation(loaded, a, counts, &mut cd07),
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
            if representation {
                r6_representation(loaded, a, counts, &mut cd07);
            }
        }
    }
}

/// Is `name` one of the representation-mode claims of VOS02 2.7?  `rep_init`, `rep_alpha`,
/// and the `<op>_rep` and `<op>_det` of every operation.
fn is_rep_claim_name(name: &str) -> bool {
    name == "rep_init" || name == "rep_alpha" || name.ends_with("_rep") || name.ends_with("_det")
}

/// The operations of the contract, in `contract.vos` order, each once (the ceilings name
/// them: every operation has one).
fn contract_ops(loaded: &Loaded) -> Vec<&str> {
    let mut ops: Vec<&str> = Vec::new();
    for c in &loaded.contract.ceilings {
        if !ops.contains(&c.op.as_str()) {
            ops.push(&c.op);
        }
    }
    ops
}

/// The functions the lint treats as *operations*: everything declared with `op` in the
/// document (bound behaviour, R3) and the interface functions the contract implies,
/// `init` and the `<op>_out` and `<op>_img` of each contract operation (an author who
/// wrote one as a `model` with a body does not thereby make a claim about it op-free).
fn operation_names(loaded: &Loaded, a: &Analysis) -> BTreeSet<String> {
    let mut names: BTreeSet<String> = a.op_decls.iter().cloned().collect();
    names.insert("init".to_string());
    for op in contract_ops(loaded) {
        if op != "init" {
            names.insert(format!("{op}_out"));
            names.insert(format!("{op}_img"));
        }
    }
    names
}

/// The step state of a call is right when it is `rep(..)` itself or the `<op>_img` of
/// something (an image is a state; that inner call is a call of its own, checked in turn).
fn state_is_rep(call: &CallFact, ops: &BTreeSet<String>) -> bool {
    match call.state_arg.as_deref() {
        Some("rep") => true,
        Some(head) => head.ends_with("_img") && ops.contains(head),
        None => false,
    }
}

/// CD07-R: the structural lint of a representation-mode implementation (VOS02 R6, rule 2).
/// Every finding names what to change.
fn r6_representation(
    loaded: &Loaded,
    a: &Analysis,
    counts: &BTreeMap<Tier, (usize, usize)>,
    cd07: &mut impl FnMut(String),
) {
    let has = |t: Tier| counts.get(&t).is_some_and(|(h, _)| *h > 0);
    // (1) no canonical, no any, not named -any.
    if loaded.layout.impl_pkg.ends_with("-any") {
        cd07(
            "mode representation: a crate named `-any` is kind any, which has no mode (R6)".into(),
        );
    }
    if has(Tier::Canonical) {
        cd07("mode representation: the document carries a `canonical` obligation; a state that carries history is not canonical, so a representation crate carries none (VOS02 2.7)".into());
    }
    if has(Tier::Any) {
        cd07("mode representation: the document carries an `any` obligation; a representation crate carries no `any` block (R6, NC5)".into());
    }
    // (2) the off-cone set of build is non-empty.
    match &a.build {
        BuildShape::Missing => cd07("mode representation: the `build` model is missing, so the history register outside its cone cannot be found".into()),
        BuildShape::NotConstructor => cd07("mode representation: the `build` model is not an `Impl(..)` constructor application, so the cone cannot be decided structurally (fails closed)".into()),
        BuildShape::BudgetExhausted => cd07("mode representation: the `build` cone walk exhausted its step budget, so the cone cannot be decided (fails closed)".into()),
        BuildShape::WideModel => cd07("mode representation: the `build` cone walk met a model with more than 63 parameters, so the cone cannot be decided (fails closed)".into()),
        BuildShape::Constructor { in_cone } => {
            if !in_cone.iter().any(|c| !*c) {
                cd07("mode representation: every field of `Impl` is set from the abstract state by `build`, so there is no history register outside the cone; declare canonical (representation mode is for history only, VOS02 2.7)".into());
            }
        }
    }
    // (3) rep exists, is an Impl(..) application, and covers: every field in its cone.
    match &a.rep {
        BuildShape::Missing => cd07("mode representation: the model `rep` is missing (or has no parameters): the `_rep` claims have nothing to range over (VOS02 2.7)".into()),
        BuildShape::NotConstructor => cd07("mode representation: the model `rep` is not an `Impl(..)` constructor application, so the states it covers cannot be decided structurally (fails closed)".into()),
        BuildShape::BudgetExhausted => cd07("mode representation: the `rep` cone walk exhausted its step budget, so the cone cannot be decided (fails closed)".into()),
        BuildShape::WideModel => cd07("mode representation: the `rep` cone walk met a model with more than 63 parameters, so the cone cannot be decided (fails closed)".into()),
        BuildShape::Constructor { in_cone } => {
            let outside: Vec<String> = in_cone
                .iter()
                .enumerate()
                .filter(|(_, c)| !**c)
                .map(|(i, _)| i.to_string())
                .collect();
            if !outside.is_empty() {
                cd07(format!(
                    "mode representation: field(s) {} of `Impl` do not depend on any parameter of `rep`: a constant or an ignored argument, so the `_rep` claims do not range over every `inv_impl` state (CD07-R)",
                    outside.join(", ")
                ));
            }
        }
    }
    // (4) the claims.
    let shapes: BTreeMap<&str, &ClaimShape> = a
        .claim_shapes
        .iter()
        .map(|c| (c.name.as_str(), c))
        .collect();
    let mut required: Vec<String> = vec!["rep_init".into(), "rep_alpha".into()];
    required.extend(
        contract_ops(loaded)
            .into_iter()
            .filter(|op| *op != "init")
            .map(|op| format!("{op}_rep")),
    );
    for name in &required {
        if !shapes.contains_key(name.as_str()) {
            cd07(format!(
                "mode representation: no claim `{name}` (rep_init, rep_alpha and one `<op>_rep` per operation of `{}` are required, VOS02 2.7)",
                loaded.contract_pkg
            ));
        }
    }
    let dets: Vec<&&ClaimShape> = shapes
        .values()
        .filter(|c| c.name.ends_with("_det") && c.name.len() > 4)
        .collect();
    if dets.is_empty() {
        cd07("mode representation: no `<op>_det` claim: the determinism claim pins how the history evolves (the output and the full post-state of a step against an op-free model), and a representation crate without one has said nothing about its history register (VOS02 2.7)".into());
    }
    let ops = operation_names(loaded, a);
    for c in a.claim_shapes.iter().filter(|c| is_rep_claim_name(&c.name)) {
        if c.mentions.contains("build") {
            cd07(format!(
                "mode representation: claim `{}` mentions `build`: a representation claim ranges over `rep(..)` states, not over the image of `build` (VOS02 2.7)",
                c.name
            ));
        }
        let mut bad: BTreeSet<&str> = BTreeSet::new();
        for call in &c.calls {
            if call.arity > 0 && ops.contains(&call.callee) && !state_is_rep(call, &ops) {
                bad.insert(&call.callee);
            }
        }
        for callee in bad {
            cd07(format!(
                "mode representation: claim `{}` applies the operation `{callee}` to a state that is not `rep(..)`: every operation in a representation claim is applied to a `rep(..)` state (VOS02 2.7)",
                c.name
            ));
        }
        if c.name.ends_with("_det") {
            let mut rhs: BTreeSet<&str> = BTreeSet::new();
            for call in &c.calls {
                if call.in_eq_rhs && ops.contains(&call.callee) {
                    rhs.insert(&call.callee);
                }
            }
            for callee in rhs {
                cd07(format!(
                    "mode representation: determinism claim `{}` calls the operation `{callee}` on the right of `==`: the model it is compared with must be op-free, or it pins nothing (VOS02 2.7)",
                    c.name
                ));
            }
        }
    }
}
