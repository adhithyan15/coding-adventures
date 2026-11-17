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

/// The contract-implied images: `<op>_img` for every operation but `init`.
fn contract_images(loaded: &Loaded) -> BTreeSet<String> {
    contract_ops(loaded)
        .into_iter()
        .filter(|op| *op != "init")
        .map(|op| format!("{op}_img"))
        .collect()
}

/// What the lint knows about the operations of one document, gathered once.
struct OpFacts<'a> {
    /// Functions treated as operations (see [`operation_names`]).
    names: BTreeSet<String>,
    /// The contract-implied `<op>_out` and `<op>_img` names.
    interface: BTreeSet<String>,
    /// The REAL images: contract-implied `<op>_img` names that the document declares with
    /// `op`.  An author's `model foo_img() -> Impl = ..` is not an image of anything, and
    /// neither is a model that reuses a contract name: only a bound operation is the
    /// implementation's behaviour (R3).
    images: BTreeSet<String>,
    /// Which parameters of each declared op have type `Impl`.
    sigs: BTreeMap<&'a str, &'a [usize]>,
}

impl<'a> OpFacts<'a> {
    fn new(loaded: &Loaded, a: &'a Analysis) -> Self {
        let mut interface = BTreeSet::new();
        for op in contract_ops(loaded) {
            if op != "init" {
                interface.insert(format!("{op}_out"));
                interface.insert(format!("{op}_img"));
            }
        }
        let declared: BTreeSet<&str> = a.op_decls.iter().map(String::as_str).collect();
        let images = contract_images(loaded)
            .into_iter()
            .filter(|n| declared.contains(n.as_str()))
            .collect();
        OpFacts {
            names: operation_names(loaded, a),
            interface,
            images,
            sigs: a
                .op_sigs
                .iter()
                .map(|s| (s.name.as_str(), s.impl_params.as_slice()))
                .collect(),
        }
    }

    /// Is `head` something a state may be: `rep(..)` or the real image of an operation?
    fn is_state_head(&self, head: &str) -> bool {
        head == "rep" || self.images.contains(head)
    }

    /// Why a call of an operation is wrong, or `None` when it is fine.  Every argument whose
    /// declared type is `Impl` is a state (an operation with no declaration: the first),
    /// and each must be `rep(..)` or a real image (whose own call is checked in turn, so the
    /// chain is validated down to a `rep(..)`).  An operation that takes a state and is
    /// called with none (`cfg_n_img()`) has nothing to be an image of.
    fn call_problem(&self, call: &CallFact) -> Option<String> {
        if !self.names.contains(&call.callee) {
            return None;
        }
        if call.arity == 0 {
            return self.interface.contains(&call.callee).then(|| {
                format!(
                    "calls `{}` with no arguments: an operation of the contract takes a state",
                    call.callee
                )
            });
        }
        let states: Vec<usize> = match self.sigs.get(call.callee.as_str()) {
            Some(idx) => idx.to_vec(),
            None => vec![0],
        };
        for i in states.into_iter().filter(|i| *i < call.arity) {
            let ok = call.arg_heads[i]
                .as_deref()
                .is_some_and(|h| self.is_state_head(h));
            if !ok {
                return Some(format!(
                    "applies the operation `{}` to a state that is not `rep(..)`: every operation in a representation claim is applied to a `rep(..)` state",
                    call.callee
                ));
            }
        }
        None
    }

    /// Is the call a fine application of an operation (it has a state, and each is a `rep(..)`
    /// chain)?
    fn is_good_call(&self, call: &CallFact) -> bool {
        call.arity > 0 && self.call_problem(call).is_none()
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
    // (3b) Coverage is approximated structurally: `rep` needs at least as many parameters as
    // `Impl` has fields, and every field outside the cone of `build` (a history register)
    // must depend on a parameter that no field inside the cone depends on, so the history
    // is free of the abstract state.  `Impl(a, bit_and(a, 7u8))` fails both.
    if let (BuildShape::Constructor { in_cone: rep_cone }, BuildShape::Constructor { in_cone }) =
        (&a.rep, &a.build)
    {
        if a.rep_params < rep_cone.len() {
            cd07(format!(
                "mode representation: `rep` has {} parameter(s) for {} field(s) of `Impl`: a `rep` with fewer parameters than fields couples a field to another and covers only a slice of the `inv_impl` states (CD07-R)",
                a.rep_params,
                rep_cone.len()
            ));
        }
        if in_cone.len() == a.rep_masks.len() {
            let cone_params = in_cone
                .iter()
                .zip(&a.rep_masks)
                .filter(|(c, _)| **c)
                .fold(0u64, |m, (_, p)| m | p);
            for (i, (c, p)) in in_cone.iter().zip(&a.rep_masks).enumerate() {
                if !*c && p & !cone_params == 0 {
                    cd07(format!(
                        "mode representation: field {i} of `Impl` (outside the cone of `build`) depends on no parameter of `rep` that the cone fields do not: the history is not independent of the abstract state, so the `_rep` claims cover only a slice of the `inv_impl` states (CD07-R)"
                    ));
                }
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
    let facts = OpFacts::new(loaded, a);
    let ops_of_contract = contract_ops(loaded);
    let alphas = alpha_names(a, &facts);
    if alphas.is_empty() {
        cd07("mode representation: the alpha operation cannot be found (no `op` applied to `build(..)` in the contract's claims besides the contract's own, and none named `alpha*`), so `rep_alpha` cannot be checked (fails closed)".into());
    }
    for c in a.claim_shapes.iter().filter(|c| is_rep_claim_name(&c.name)) {
        let mut say = |m: String| {
            cd07(format!(
                "mode representation: claim `{}` {m} (VOS02 2.7)",
                c.name
            ))
        };
        let is_init = c.name == "rep_init";
        // A refinement on a binder narrows what the claim ranges over (`mask == 0u8`), and
        // would let a claim cover a slice and still look quantified.
        if !c.refined.is_empty() {
            say(format!(
                "has a refinement on binder(s) {}: a representation claim ranges over every `rep(..)` state",
                c.refined.join(", ")
            ));
        }
        if c.mentions.contains("build") {
            say("mentions `build`: a representation claim ranges over `rep(..)` states, not over the image of `build`".into());
        }
        // Every operation call: a state argument that is `rep(..)` or a real image.
        let mut problems: BTreeSet<String> = BTreeSet::new();
        for call in &c.calls {
            if let Some(why) = facts.call_problem(call) {
                // (the long-standing wording for a wrong state keeps its prefix)
                problems.insert(if why.starts_with("applies") {
                    format!("applies the operation `{}` to a state that is not `rep(..)`: every operation in a representation claim is applied to a `rep(..)` state", call.callee)
                } else {
                    why
                });
            }
        }
        for why in problems {
            say(why);
        }
        // The arguments of `rep(..)`: distinct variables of this claim's quantifier.  A
        // constant (`rep(mask, 0u8)`) covers the history-zero slice only.  `rep_init` is the
        // exception: the start state is ONE state, `rep(0u8, 0u8)`.
        if !is_init {
            if c.rep_calls.is_empty() {
                say("never applies `rep`: it says nothing about the representation".into());
            }
            for args in &c.rep_calls {
                let mut seen: BTreeSet<&str> = BTreeSet::new();
                let all_vars = args.iter().all(|x| {
                    x.as_deref()
                        .is_some_and(|v| c.binders.iter().any(|b| b == v) && seen.insert(v))
                });
                if !all_vars {
                    say("applies `rep` to arguments that are not distinct variables bound by its quantifier: a constant or a repeated variable covers only a slice of the `inv_impl` states".into());
                }
            }
        }
        // What each required claim must be about.
        if is_init {
            if !c.calls.iter().any(|k| k.callee == "init") || !c.mentions.contains("rep") {
                say("must mention both `init` and `rep`".into());
            }
        } else if c.name == "rep_alpha" {
            let calls_alpha = c
                .calls
                .iter()
                .any(|k| alphas.contains(&k.callee) && facts.is_good_call(k));
            if !alphas.is_empty() && !calls_alpha {
                say(format!(
                    "does not apply the alpha operation ({}) to a `rep(..)` state",
                    alphas.iter().cloned().collect::<Vec<_>>().join(", ")
                ));
            }
        } else {
            let op = c
                .name
                .strip_suffix("_rep")
                .or_else(|| c.name.strip_suffix("_det"))
                .unwrap_or("");
            if op == "init" || !ops_of_contract.contains(&op) {
                say(format!(
                    "names `{op}`, which is not an operation of `{}` other than `init`",
                    loaded.contract_pkg
                ));
            } else {
                let mine = [format!("{op}_out"), format!("{op}_img"), op.to_string()];
                let calls_it = c
                    .calls
                    .iter()
                    .any(|k| mine.contains(&k.callee) && facts.is_good_call(k));
                if !calls_it {
                    say(format!("does not call the operation `{op}` (`{op}_out` or `{op}_img`) on a `rep(..)` state"));
                }
            }
        }
        // A determinism claim is a conjunction of equations `op(rep(..))... == model(..)`:
        // the left side roots in an operation on a `rep(..)` state, and the right side calls
        // NO operation anywhere (arguments included).  `not (a != b)`, `<=`/`>=` pairs and a
        // model wrapper `eq_pick(..)` are not equations and pin nothing the lint can see.
        if c.name.ends_with("_det") {
            match &c.equations {
                None => say("is not a conjunction (`and`) of `==` equations (`not (a != b)`, `<=`, a wrapper model, `or` and `if` are not accepted)".into()),
                Some(eqs) => {
                    let mut rhs: BTreeSet<&str> = BTreeSet::new();
                    for e in eqs {
                        let rooted = e
                            .left_root
                            .as_ref()
                            .is_some_and(|r| facts.names.contains(&r.callee) && facts.is_good_call(r));
                        if !rooted {
                            say("has an equation whose left side is not an operation applied to a `rep(..)` state (directly or through a field projection)".into());
                        }
                        for k in &e.right {
                            if facts.names.contains(&k.callee) {
                                rhs.insert(&k.callee);
                            }
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
    }
}

/// The alpha operation(s): the `op`s the contract's own claims apply to `build(..)` that
/// are not `<op>_out`, `<op>_img` or `init` (the contract does not name alpha in
/// `contract.vos`, but `safety_alpha_build` applies it to `build(m)`); failing that, an `op`
/// whose name starts `alpha`.
fn alpha_names(a: &Analysis, f: &OpFacts<'_>) -> BTreeSet<String> {
    let declared: BTreeSet<&str> = a.op_decls.iter().map(String::as_str).collect();
    let mut found: BTreeSet<String> = BTreeSet::new();
    for c in a
        .claim_shapes
        .iter()
        .filter(|c| !is_rep_claim_name(&c.name))
    {
        for k in &c.calls {
            if k.state_arg.as_deref() == Some("build")
                && declared.contains(k.callee.as_str())
                && !f.interface.contains(&k.callee)
                && k.callee != "init"
            {
                found.insert(k.callee.clone());
            }
        }
    }
    if found.is_empty() {
        found = a
            .op_decls
            .iter()
            .filter(|n| n.starts_with("alpha"))
            .cloned()
            .collect();
    }
    found
}
