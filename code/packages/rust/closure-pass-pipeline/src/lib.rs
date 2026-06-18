//! Pass trait + scheduler for the Closure Compiler clone.
//!
//! Per [CLOC06](../../../specs/CLOC06-pass-interface-contract.md), every
//! `closure-pass-*` crate implements the [`Pass`] trait defined here.
//! [`PassPipeline`] is the scheduler that runs them in dependency
//! order. This crate is the **harness** all optimization passes plug
//! into.
//!
//! # What v1 supports
//!
//! - The full [`Pass`] trait surface: `name`, `depends_on`,
//!   `invalidates`, `iteration_policy`, `cost`, `run`.
//! - Topo-sort scheduling by `depends_on`. Cycles produce a clear
//!   [`PassError`].
//! - `OneShot` and `FixedPoint` iteration. The scheduler runs the
//!   whole topo order in repeated *sweeps* and keeps sweeping while any
//!   `FixedPoint` pass reports a change, up to [`MAX_SWEEPS`]. This is
//!   what lets transforms cascade: `inline` turns `double(7)` into
//!   `7 * 2`, and the next sweep's `constant-fold` folds it to `14`.
//! - Per-pass [`PassStats`] in the final [`PipelineOutput`] so callers
//!   can see what ran.
//! - Cost / budget gating: deferred to a follow-up. v1 ignores
//!   `cost()` entirely.
//!
//! # The dependency contract
//!
//! `Pass::depends_on()` returns the names of passes that must run
//! before this one. The scheduler resolves these as a DAG and runs
//! passes in topological order. If two passes have no ordering
//! constraint between them, the order they were added to the
//! pipeline wins (stable sort).
//!
//! `Pass::invalidates()` is currently informational only — v1 does
//! not re-run passes on invalidation. The coarse-grained invalidation
//! story from CLOC06 §"Open question 1" lands when we actually have
//! mutating passes.

use std::collections::HashMap;

use coding_adventures_closure_typechecker::Diagnostic;
use coding_adventures_correlation_vector::{CVLog, Contribution};
use coding_adventures_javascript_ast::Program;
use coding_adventures_type_sidecar::Sidecar;

// ============================================================================
// Pass trait surface
// ============================================================================

/// How often a pass should run within a single pipeline invocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IterationPolicy {
    /// Run exactly once. The default.
    OneShot,
    /// Participate in the pipeline's fixed-point loop: the scheduler
    /// re-runs the whole pass order in sweeps and keeps sweeping while
    /// *any* `FixedPoint` pass reports [`PassOutput::changed`] — so a
    /// transform one pass exposes (e.g. `inline` turning `double(7)`
    /// into `7 * 2`) is picked up by an earlier pass on the next sweep
    /// (`constant-fold` folding it to `14`). Bounded by
    /// [`PassPipeline`]'s sweep cap as a backstop against a
    /// non-convergent pass.
    FixedPoint,
}

/// What the scheduler hands a pass when it calls [`Pass::run`].
///
/// v1 keeps this minimal: program + sidecar + CV log. CLOC06 specs
/// `options: &PassOptions` and `prior: &PassResults` which we'll add
/// once they're actually used. Passes today access only what they
/// strictly need.
pub struct PassContext<'a> {
    /// The current AST. Read-only for the pass; mutations land in
    /// `PassOutput::program`.
    pub program: &'a Program,
    /// The merged type sidecar.
    pub sidecar: &'a Sidecar,
    /// The shared correlation-vector log. Passes append contributions
    /// per CLOC03 §"Pass contribution conventions."
    pub cv: &'a mut CVLog,
}

/// What a pass returns from [`Pass::run`].
#[derive(Debug, Clone)]
pub struct PassOutput {
    /// The new AST. v1 passes that don't mutate the tree clone the
    /// input.
    pub program: Program,
    /// Contributions the pass would like to record in the CV log.
    /// The scheduler appends these for the pass; passes also append
    /// directly to `ctx.cv` during their walk if they need richer
    /// per-node bookkeeping.
    pub contributions: Vec<Contribution>,
    /// Whether the pass changed anything. Only meaningful for
    /// [`IterationPolicy::FixedPoint`].
    pub changed: bool,
    /// Diagnostics the pass surfaced.
    pub diagnostics: Vec<Diagnostic>,
    /// Per-pass stats — collected into [`PipelineOutput::stats`].
    pub stats: PassStats,
}

/// Per-pass metrics returned from [`PassOutput`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PassStats {
    /// How many AST nodes the pass examined. v1 leaves this to the
    /// pass to count.
    pub nodes_touched: u32,
}

/// Catastrophic pass failure. Recoverable issues should go into
/// [`PassOutput::diagnostics`]; this is for invariant violations and
/// malformed inputs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PassError {
    /// Which pass failed.
    pub pass_name: String,
    /// Human-readable summary.
    pub message: String,
}

impl std::fmt::Display for PassError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "pass {:?} failed: {}", self.pass_name, self.message)
    }
}

impl std::error::Error for PassError {}

/// The contract every optimization pass implements.
///
/// Per CLOC06 §"The `Pass` trait" — object-safe so passes can be held
/// as `Box<dyn Pass>` in the pipeline.
pub trait Pass {
    /// Stable canonical name, e.g. `"dce"`, `"constant-fold"`,
    /// `"rename"`. Used as the `Contribution.source` per CLOC03 and as
    /// the key in the scheduler's dependency graph. Must be unique
    /// across all passes registered in a single pipeline.
    fn name(&self) -> &'static str;

    /// Other passes whose effects this one relies on. The scheduler
    /// runs all `depends_on` passes before this one.
    fn depends_on(&self) -> &[&'static str] {
        &[]
    }

    /// Other passes whose results this one might invalidate.
    /// Informational in v1; CLOC06 Open Question 1 settles the
    /// re-run semantics in a follow-up.
    fn invalidates(&self) -> &[&'static str] {
        &[]
    }

    /// One-shot vs fixed-point. v1 always executes once regardless.
    fn iteration_policy(&self) -> IterationPolicy {
        IterationPolicy::OneShot
    }

    /// Coarse cost estimate, in arbitrary "pass-units." v1 ignores
    /// the cost budget entirely.
    fn cost(&self) -> u32 {
        1
    }

    /// Run the pass.
    fn run(&self, ctx: PassContext<'_>) -> Result<PassOutput, PassError>;
}

// ============================================================================
// Pipeline scheduler
// ============================================================================

/// The full output of one [`PassPipeline::run`] call.
#[derive(Debug, Clone)]
pub struct PipelineOutput {
    /// The final AST after every pass ran.
    pub program: Program,
    /// Diagnostics accumulated from every pass.
    pub diagnostics: Vec<Diagnostic>,
    /// Per-pass stats, keyed by pass name. Order matches execution.
    pub stats: HashMap<String, PassStats>,
    /// The execution order the scheduler actually used. Useful for
    /// debugging dependency-graph issues without re-running the
    /// scheduler.
    pub execution_order: Vec<String>,
}

/// Maximum number of full pipeline sweeps [`PassPipeline::run`] will
/// perform before giving up on convergence.
///
/// A real optimization fixed point is reached in a handful of sweeps —
/// each `FixedPoint` change strictly simplifies the program (folds
/// shrink expressions, inlines remove calls, DCE/treeshake delete
/// nodes), so the iteration count is bounded by expression/chain depth,
/// not program size. This cap is therefore deliberately generous: it
/// exists only as a backstop against a *buggy* pass that reports
/// `changed = true` without making progress (e.g. two passes that undo
/// each other's work). Hitting it emits a `pipeline.fixed-point-cap-reached`
/// note.
pub const MAX_SWEEPS: usize = 100;

/// The pass scheduler.
///
/// Build one with [`PassPipeline::new`], register passes with
/// [`PassPipeline::add`], then call [`PassPipeline::run`] to execute
/// the whole graph to a fixed point.
pub struct PassPipeline {
    passes: Vec<Box<dyn Pass>>,
}

impl Default for PassPipeline {
    fn default() -> Self {
        Self::new()
    }
}

impl PassPipeline {
    /// Construct an empty pipeline.
    pub fn new() -> Self {
        Self { passes: Vec::new() }
    }

    /// Register a pass. Order of registration becomes the
    /// tie-breaker when topo-sort has multiple valid orderings.
    pub fn add(&mut self, pass: Box<dyn Pass>) -> &mut Self {
        self.passes.push(pass);
        self
    }

    /// Number of registered passes.
    pub fn len(&self) -> usize {
        self.passes.len()
    }

    /// Is the pipeline empty?
    pub fn is_empty(&self) -> bool {
        self.passes.is_empty()
    }

    /// Run the pipeline to a fixed point. See module docs for behavior.
    ///
    /// The scheduler topo-sorts the passes once, then runs that order in
    /// repeated **sweeps**. After each sweep it asks: did any
    /// `FixedPoint` pass report a change? If so — and we're under
    /// [`MAX_SWEEPS`] — it runs another sweep, so a transform one pass
    /// exposes is picked up by an earlier pass next time around
    /// (`inline`'s `7 * 2` → `constant-fold`'s `14`). When a sweep makes
    /// no `FixedPoint` change, the program has converged and we stop.
    ///
    /// `OneShot` passes re-run each sweep too, but their `changed` flag
    /// does **not** drive the loop — they are expected to be idempotent
    /// at the fixed point (running them again is a no-op), so re-running
    /// them just lets them observe the final program (e.g. `rename`
    /// shortens names on the fully-folded output) without risking a
    /// spin. The [`MAX_SWEEPS`] cap is the backstop against a buggy pass
    /// that reports `changed = true` forever (e.g. two passes that undo
    /// each other); hitting it surfaces a `pipeline.fixed-point-cap-reached`
    /// note rather than silently under- or over-optimizing.
    pub fn run(
        &self,
        program: Program,
        sidecar: &Sidecar,
        cv: &mut CVLog,
    ) -> Result<PipelineOutput, PassError> {
        let order = self.topo_sort()?;

        // Build a lookup so we can fetch passes by name during execution.
        let by_name: HashMap<&'static str, &dyn Pass> =
            self.passes.iter().map(|p| (p.name(), p.as_ref())).collect();

        let mut current = program;
        // Diagnostics + stats describe the FINAL (converged) sweep — an
        // earlier sweep's diagnostics referred to a transient
        // intermediate program, so we keep only the last sweep's.
        let mut diagnostics = Vec::new();
        let mut stats = HashMap::new();
        let mut converged = false;

        for _sweep in 0..MAX_SWEEPS {
            let mut sweep_diagnostics = Vec::new();
            let mut sweep_stats = HashMap::new();
            let mut fixed_point_changed = false;

            for name in &order {
                let pass = *by_name
                    .get(name.as_str())
                    .expect("topo-sort returns only registered passes");

                let ctx = PassContext {
                    program: &current,
                    sidecar,
                    cv,
                };
                let output = pass.run(ctx)?;

                // Append CV contributions the pass returned to the log.
                // The pass's own name should already match its
                // contribution.source per the CLOC06 review checklist.
                // We tag-append against the program root's CV since
                // that's the durable handle. Contributions accumulate
                // across sweeps — each is a real transformation in the
                // provenance record.
                //
                // CLOC09 made Program.cv optional. When tracing is
                // disabled (cv == None), there's no CV id to attach
                // contributions to — passes shouldn't be emitting them
                // in that mode anyway, but we skip silently here for
                // safety.
                if let Some(ref prog_cv) = current.cv {
                    for c in &output.contributions {
                        let _ = cv.contribute(prog_cv, &c.source, &c.tag, c.meta.clone());
                    }
                }

                // Only a FixedPoint pass's change drives another sweep.
                // A OneShot pass that reports a change does not spin the
                // loop (it is expected to converge in one application).
                if output.changed && pass.iteration_policy() == IterationPolicy::FixedPoint {
                    fixed_point_changed = true;
                }

                sweep_diagnostics.extend(output.diagnostics);
                sweep_stats.insert(pass.name().to_string(), output.stats);
                current = output.program;
            }

            // Keep this sweep's diagnostics + stats as the running final.
            diagnostics = sweep_diagnostics;
            stats = sweep_stats;

            if !fixed_point_changed {
                converged = true;
                break;
            }
        }

        if !converged {
            // We ran out of sweeps with a FixedPoint pass still asking
            // for more. Surface it rather than pretending we converged.
            // Diagnostic.cv is still plain String (closure-typechecker
            // hasn't migrated to Option<CvId>); for untraced programs we
            // emit an empty cv to keep the diagnostic shape stable.
            diagnostics.push(Diagnostic {
                cv: current.cv.clone().unwrap_or_default(),
                severity: coding_adventures_closure_typechecker::Severity::Note,
                group: coding_adventures_closure_typechecker::DiagnosticGroup::new(
                    "pipeline.fixed-point-cap-reached",
                ),
                message: format!(
                    "pipeline stopped after {MAX_SWEEPS} sweeps without reaching a fixed \
                     point; a FixedPoint pass is still reporting changes (possible \
                     non-convergent pass)."
                ),
            });
        }

        Ok(PipelineOutput {
            program: current,
            diagnostics,
            stats,
            execution_order: order,
        })
    }

    /// Topologically sort the registered passes by `depends_on`.
    ///
    /// Returns the execution order as a `Vec<String>` of pass names.
    /// Cycles produce a [`PassError`] whose `pass_name` is the first
    /// pass detected in the cycle and whose message names the cycle's
    /// neighbours.
    fn topo_sort(&self) -> Result<Vec<String>, PassError> {
        // Build the dependency graph: for each pass, list its
        // declared `depends_on` targets that are actually registered.
        // Unknown deps are dropped (with no warning in v1 — CLOC06
        // doesn't yet require us to surface them).
        let names: Vec<&'static str> = self.passes.iter().map(|p| p.name()).collect();
        let name_set: std::collections::HashSet<&'static str> = names.iter().copied().collect();

        // Detect duplicate pass names — they violate the CLOC06
        // contract that names are unique.
        if name_set.len() != names.len() {
            return Err(PassError {
                pass_name: "<duplicate>".to_string(),
                message: "two or more registered passes share the same name()".into(),
            });
        }

        // adj[pass] = passes that depend on `pass`.
        let mut adj: HashMap<&'static str, Vec<&'static str>> = HashMap::new();
        let mut in_degree: HashMap<&'static str, usize> = HashMap::new();
        for &n in &names {
            adj.entry(n).or_default();
            in_degree.entry(n).or_insert(0);
        }
        for p in &self.passes {
            for &dep in p.depends_on() {
                if !name_set.contains(dep) {
                    // CLOC06 doesn't pin behavior for missing deps;
                    // v1 silently skips them.
                    continue;
                }
                adj.entry(dep).or_default().push(p.name());
                *in_degree.entry(p.name()).or_insert(0) += 1;
            }
        }

        // Process passes in their registration order so stable
        // tie-breaking holds.
        let mut ready: Vec<&'static str> = names
            .iter()
            .copied()
            .filter(|n| in_degree.get(n).copied().unwrap_or(0) == 0)
            .collect();
        let mut order: Vec<String> = Vec::with_capacity(self.passes.len());

        while let Some(n) = ready.first().copied() {
            ready.remove(0);
            order.push(n.to_string());
            // Process dependents in registration order (Vec preserves
            // insertion order).
            let dependents = adj.remove(n).unwrap_or_default();
            for dep in dependents {
                let d = in_degree.entry(dep).or_insert(0);
                *d = d.saturating_sub(1);
                if *d == 0 && !order.iter().any(|s| s == dep) && !ready.contains(&dep) {
                    ready.push(dep);
                }
            }
        }

        if order.len() != names.len() {
            let remaining: Vec<&'static str> = names
                .iter()
                .copied()
                .filter(|n| !order.iter().any(|o| o == n))
                .collect();
            return Err(PassError {
                pass_name: remaining.first().map(|s| s.to_string()).unwrap_or_default(),
                message: format!(
                    "dependency cycle detected among passes: {}",
                    remaining
                        .iter()
                        .map(|s| format!("{s:?}"))
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            });
        }

        Ok(order)
    }
}

// ============================================================================
// PassRegistry — runtime pass discovery layer (CLOC10.A)
// ============================================================================
//
// Per [CLOC10 §5](../../../specs/CLOC10-pass-plugin-api.md#5-passregistry-runtime-discovery),
// the `PassRegistry` sits on top of `PassPipeline` and lets callers
// instantiate a pipeline by *naming* passes rather than by holding
// onto concrete `Box<dyn Pass>` values. This is what makes the
// plugin API practical:
//
// - A host like `closurec` can populate a registry once at startup
//   (canonical passes + any third-party plugins the host wants to
//   expose).
// - User input — a `--enable=<name>` flag, a `--passes <config>`
//   file, an interactive REPL — names passes by their canonical
//   string name. The registry resolves names into a pipeline.
// - Third-party plugin crates ship a public `register(&mut registry)`
//   function. The host calls it; the registry grows; no recompile
//   of `closurec` itself is needed for any pass that's already
//   linked in.
//
// Why a *factory* rather than a stored `Box<dyn Pass>`? Two reasons:
//
// 1. A pipeline owns its passes. If we stored `Box<dyn Pass>` in
//    the registry, building a pipeline would have to *move* them
//    out, which means a one-shot registry. Factories let one
//    registry hand out as many pipeline instances as the caller
//    wants.
// 2. Some passes have per-instance state that *must* be fresh per
//    pipeline run. A factory closure naturally constructs a fresh
//    pass each time.
//
// Note that we deliberately do NOT pre-populate `PassRegistry::new`
// with the eight canonical passes. Doing that would require
// `closure-pass-pipeline` to depend on every `closure-pass-*`
// crate, which would create a circular dep (each pass already
// depends on pipeline for the `Pass` trait). The convention is:
// the *host* (typically `closurec`) imports each pass crate and
// calls `registry.register(...)` for each canonical pass at
// startup. CLOC10.C will wire this up in the CLI.

use std::collections::HashMap as StdHashMap;

/// A type-erased pass factory — closure that produces a fresh
/// pass instance on demand.
type PassFactory = Box<dyn Fn() -> Box<dyn Pass> + Send + Sync>;

/// Errors that can occur during registry operations.
///
/// Per CLOC10 §5, all registry failures are reported via this enum
/// rather than panics — the caller is in a position to recover
/// (e.g. by falling back to a smaller pipeline, or by reporting a
/// friendly CLI error).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryError {
    /// A name was passed to `build_pipeline` that wasn't registered.
    UnknownPass(String),
    /// A name passed to `register` was already taken. Names must be
    /// unique per registry per CLOC10 §3.
    DuplicateName(String),
}

impl std::fmt::Display for RegistryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RegistryError::UnknownPass(name) => {
                write!(f, "no pass registered under the name {:?}", name)
            }
            RegistryError::DuplicateName(name) => {
                write!(
                    f,
                    "a pass with name {:?} is already registered; \
                     names must be unique within a registry",
                    name
                )
            }
        }
    }
}

impl std::error::Error for RegistryError {}

/// Runtime registry of named pass factories.
///
/// Lets a host (typically `closurec`) build pipelines by naming
/// passes instead of holding onto concrete `Box<dyn Pass>` values.
/// See [CLOC10 §5] for the design rationale.
///
/// # Example
///
/// ```rust,ignore
/// // In the host (e.g. `closurec` startup):
/// let mut registry = PassRegistry::new();
/// registry.register("constant-fold",
///                   || Box::new(ConstantFoldPass::new()))?;
/// registry.register("dce", || Box::new(DcePass::new()))?;
/// registry.register("acme/foo",
///                   || Box::new(AcmePass::new()))?;
///
/// // Later, from user input:
/// let pipeline = registry.build_pipeline(&[
///     "constant-fold", "acme/foo", "dce",
/// ])?;
/// pipeline.run(program, &sidecar, &mut cv)?;
/// ```
///
/// [CLOC10 §5]: ../../../specs/CLOC10-pass-plugin-api.md
pub struct PassRegistry {
    factories: StdHashMap<String, PassFactory>,
}

impl Default for PassRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for PassRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Don't try to print the factories (they're closures, no
        // Debug impl). Just list the names.
        f.debug_struct("PassRegistry")
            .field("names", &self.registered_names())
            .finish()
    }
}

impl PassRegistry {
    /// Create an empty registry. Canonical pass registration is the
    /// *host's* responsibility — see the module-level comment for
    /// why this isn't auto-populated.
    pub fn new() -> Self {
        Self {
            factories: StdHashMap::new(),
        }
    }

    /// Register a pass factory under `name`. The factory will be
    /// called once per `build_pipeline` invocation that mentions
    /// the name — fresh pass instance every time, no shared state
    /// between pipelines.
    ///
    /// Returns `Err(RegistryError::DuplicateName)` if `name` is
    /// already registered. Use this strictness deliberately — silent
    /// shadowing of pass names would be a debugging nightmare.
    pub fn register<F>(&mut self, name: &str, factory: F) -> Result<(), RegistryError>
    where
        F: Fn() -> Box<dyn Pass> + Send + Sync + 'static,
    {
        if self.factories.contains_key(name) {
            return Err(RegistryError::DuplicateName(name.to_string()));
        }
        self.factories.insert(name.to_string(), Box::new(factory));
        Ok(())
    }

    /// True iff `name` has been registered.
    pub fn contains(&self, name: &str) -> bool {
        self.factories.contains_key(name)
    }

    /// All registered names, sorted alphabetically. Used by
    /// `closurec --list-passes` per CLOC10 §6.
    pub fn registered_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.factories.keys().cloned().collect();
        names.sort();
        names
    }

    /// Build a fresh pipeline containing the passes named in
    /// `names`, in that order. The order matters as a tie-breaker
    /// for the topo-sort (see [`PassPipeline::add`]).
    ///
    /// Returns `Err(RegistryError::UnknownPass)` on the first
    /// unrecognized name. Stops at the first error rather than
    /// collecting all unknowns — keep the API tight; callers that
    /// want validation-up-front can call `contains` in a loop first.
    pub fn build_pipeline(&self, names: &[&str]) -> Result<PassPipeline, RegistryError> {
        let mut pipeline = PassPipeline::new();
        for name in names {
            let factory = self
                .factories
                .get(*name)
                .ok_or_else(|| RegistryError::UnknownPass((*name).to_string()))?;
            pipeline.add(factory());
        }
        Ok(pipeline)
    }

    /// Number of registered passes.
    pub fn len(&self) -> usize {
        self.factories.len()
    }

    /// Is the registry empty?
    pub fn is_empty(&self) -> bool {
        self.factories.is_empty()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use coding_adventures_javascript_ast::SourceType;
    use coding_adventures_javascript_tokens::EsVersion;

    /// A do-nothing pass that just records which name it has.
    struct NoOpPass {
        name_: &'static str,
        deps: &'static [&'static str],
        policy: IterationPolicy,
    }

    impl NoOpPass {
        fn new(name: &'static str) -> Self {
            Self {
                name_: name,
                deps: &[],
                policy: IterationPolicy::OneShot,
            }
        }
        fn with_deps(mut self, deps: &'static [&'static str]) -> Self {
            self.deps = deps;
            self
        }
        fn with_policy(mut self, policy: IterationPolicy) -> Self {
            self.policy = policy;
            self
        }
    }

    impl Pass for NoOpPass {
        fn name(&self) -> &'static str {
            self.name_
        }
        fn depends_on(&self) -> &[&'static str] {
            self.deps
        }
        fn iteration_policy(&self) -> IterationPolicy {
            self.policy
        }
        fn run(&self, ctx: PassContext<'_>) -> Result<PassOutput, PassError> {
            Ok(PassOutput {
                program: ctx.program.clone(),
                contributions: Vec::new(),
                changed: false,
                diagnostics: Vec::new(),
                stats: PassStats { nodes_touched: 1 },
            })
        }
    }

    fn program() -> Program {
        Program::new("prog.1".to_string(), EsVersion::Es2025, SourceType::Module)
    }

    #[test]
    fn empty_pipeline_returns_program_unchanged() {
        let pipeline = PassPipeline::new();
        assert!(pipeline.is_empty());
        let mut cv = CVLog::new(true);
        let out = pipeline
            .run(program(), &Sidecar::new(), &mut cv)
            .expect("empty pipeline runs cleanly");
        assert_eq!(out.program.cv.as_deref(), Some("prog.1"));
        assert!(out.execution_order.is_empty());
        assert!(out.stats.is_empty());
    }

    #[test]
    fn single_pass_runs() {
        let mut pipeline = PassPipeline::new();
        pipeline.add(Box::new(NoOpPass::new("alpha")));
        let mut cv = CVLog::new(true);
        let out = pipeline
            .run(program(), &Sidecar::new(), &mut cv)
            .expect("pipeline runs cleanly");
        assert_eq!(out.execution_order, vec!["alpha"]);
        assert!(out.stats.contains_key("alpha"));
        assert_eq!(out.stats["alpha"].nodes_touched, 1);
    }

    #[test]
    fn dependent_passes_get_ordered_after_their_deps() {
        let mut pipeline = PassPipeline::new();
        // Register beta first to verify that depends_on actually
        // forces reordering — without it, registration order would
        // give us [beta, alpha].
        pipeline.add(Box::new(NoOpPass::new("beta").with_deps(&["alpha"])));
        pipeline.add(Box::new(NoOpPass::new("alpha")));

        let mut cv = CVLog::new(true);
        let out = pipeline
            .run(program(), &Sidecar::new(), &mut cv)
            .expect("pipeline runs cleanly");
        assert_eq!(
            out.execution_order,
            vec!["alpha".to_string(), "beta".to_string()]
        );
    }

    #[test]
    fn independent_passes_keep_registration_order() {
        // alpha and beta have no relationship → registration order
        // wins as the tie-breaker.
        let mut pipeline = PassPipeline::new();
        pipeline.add(Box::new(NoOpPass::new("alpha")));
        pipeline.add(Box::new(NoOpPass::new("beta")));

        let mut cv = CVLog::new(true);
        let out = pipeline.run(program(), &Sidecar::new(), &mut cv).unwrap();
        assert_eq!(
            out.execution_order,
            vec!["alpha".to_string(), "beta".to_string()]
        );
    }

    #[test]
    fn dependency_cycle_errors_cleanly() {
        let mut pipeline = PassPipeline::new();
        pipeline.add(Box::new(NoOpPass::new("alpha").with_deps(&["beta"])));
        pipeline.add(Box::new(NoOpPass::new("beta").with_deps(&["alpha"])));

        let mut cv = CVLog::new(true);
        let err = pipeline
            .run(program(), &Sidecar::new(), &mut cv)
            .expect_err("cycle should error");
        assert!(err.message.contains("cycle"));
        // pass_name is set to one of the cycle members.
        assert!(err.pass_name == "alpha" || err.pass_name == "beta");
        // Display has a useful summary.
        let printed = format!("{err}");
        assert!(printed.contains("cycle"));
    }

    #[test]
    fn duplicate_pass_names_error() {
        let mut pipeline = PassPipeline::new();
        pipeline.add(Box::new(NoOpPass::new("dupe")));
        pipeline.add(Box::new(NoOpPass::new("dupe")));

        let mut cv = CVLog::new(true);
        let err = pipeline
            .run(program(), &Sidecar::new(), &mut cv)
            .expect_err("duplicates should error");
        assert!(err.message.to_lowercase().contains("name"));
    }

    #[test]
    fn fixed_point_pass_that_never_changes_converges_in_one_sweep() {
        // A FixedPoint pass that reports `changed = false` converges
        // immediately — no "not-yet-iterated" note (that limitation is
        // gone), and no "cap-reached" note (we converged well inside the
        // cap).
        let mut pipeline = PassPipeline::new();
        pipeline.add(Box::new(
            NoOpPass::new("fp").with_policy(IterationPolicy::FixedPoint),
        ));

        let mut cv = CVLog::new(true);
        let out = pipeline.run(program(), &Sidecar::new(), &mut cv).unwrap();
        assert_eq!(out.execution_order, vec!["fp".to_string()]);
        assert_eq!(out.stats["fp"].nodes_touched, 1);
        assert!(
            !out.diagnostics
                .iter()
                .any(|d| d.group.0 == "pipeline.fixed-point-not-yet-iterated"),
            "the not-yet-iterated limitation is gone; got {:?}",
            out.diagnostics
        );
        assert!(
            !out.diagnostics
                .iter()
                .any(|d| d.group.0 == "pipeline.fixed-point-cap-reached"),
            "a non-changing pass must not hit the sweep cap; got {:?}",
            out.diagnostics
        );
    }

    /// A FixedPoint pass that reports `changed = true` for its first
    /// `n` runs and `false` thereafter — models a pass whose work
    /// cascades across a bounded number of sweeps. Uses interior
    /// mutability because `Pass::run` takes `&self`.
    struct CountingPass {
        name_: &'static str,
        remaining: std::sync::atomic::AtomicUsize,
        runs: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    }

    impl CountingPass {
        fn new(name: &'static str, changes: usize) -> Self {
            Self {
                name_: name,
                remaining: std::sync::atomic::AtomicUsize::new(changes),
                runs: std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            }
        }
    }

    impl Pass for CountingPass {
        fn name(&self) -> &'static str {
            self.name_
        }
        fn iteration_policy(&self) -> IterationPolicy {
            IterationPolicy::FixedPoint
        }
        fn run(&self, ctx: PassContext<'_>) -> Result<PassOutput, PassError> {
            use std::sync::atomic::Ordering;
            self.runs.fetch_add(1, Ordering::SeqCst);
            // Decrement the change budget; report `changed` while it lasts.
            let changed = self
                .remaining
                .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |r| {
                    if r > 0 {
                        Some(r - 1)
                    } else {
                        None
                    }
                })
                .is_ok();
            Ok(PassOutput {
                program: ctx.program.clone(),
                contributions: Vec::new(),
                changed,
                diagnostics: Vec::new(),
                stats: PassStats { nodes_touched: 1 },
            })
        }
    }

    #[test]
    fn fixed_point_iterates_until_no_pass_reports_change() {
        // A pass that reports `changed` three times should be run four
        // times total: three changing sweeps plus one confirming sweep
        // that reports no change and ends the loop.
        let pass = CountingPass::new("counter", 3);
        let runs = pass.runs.clone();

        let mut pipeline = PassPipeline::new();
        pipeline.add(Box::new(pass));

        let mut cv = CVLog::new(true);
        let out = pipeline.run(program(), &Sidecar::new(), &mut cv).unwrap();

        assert_eq!(
            runs.load(std::sync::atomic::Ordering::SeqCst),
            4,
            "3 changing sweeps + 1 confirming sweep"
        );
        // Converged inside the cap → no cap-reached note.
        assert!(!out
            .diagnostics
            .iter()
            .any(|d| d.group.0 == "pipeline.fixed-point-cap-reached"));
        // execution_order is the topo order (distinct passes), not the
        // per-sweep run log.
        assert_eq!(out.execution_order, vec!["counter".to_string()]);
    }

    #[test]
    fn one_shot_change_does_not_drive_the_loop() {
        // A OneShot pass that always reports `changed = true` must NOT
        // spin the loop — only FixedPoint changes do. So the pipeline
        // converges in a single sweep despite the perpetual `changed`.
        struct AlwaysChangesOneShot;
        impl Pass for AlwaysChangesOneShot {
            fn name(&self) -> &'static str {
                "oneshot"
            }
            fn iteration_policy(&self) -> IterationPolicy {
                IterationPolicy::OneShot
            }
            fn run(&self, ctx: PassContext<'_>) -> Result<PassOutput, PassError> {
                Ok(PassOutput {
                    program: ctx.program.clone(),
                    contributions: Vec::new(),
                    changed: true, // would spin the loop if OneShot drove it
                    diagnostics: Vec::new(),
                    stats: PassStats { nodes_touched: 1 },
                })
            }
        }

        let mut pipeline = PassPipeline::new();
        pipeline.add(Box::new(AlwaysChangesOneShot));

        let mut cv = CVLog::new(true);
        let out = pipeline.run(program(), &Sidecar::new(), &mut cv).unwrap();
        // Did not hit the cap → converged in one sweep.
        assert!(!out
            .diagnostics
            .iter()
            .any(|d| d.group.0 == "pipeline.fixed-point-cap-reached"));
    }

    #[test]
    fn non_convergent_pass_hits_the_cap_and_notes_it() {
        // A FixedPoint pass that ALWAYS reports `changed` (more changes
        // than the cap) must stop after MAX_SWEEPS and surface the
        // cap-reached note rather than looping forever.
        let pass = CountingPass::new("runaway", MAX_SWEEPS + 50);
        let runs = pass.runs.clone();

        let mut pipeline = PassPipeline::new();
        pipeline.add(Box::new(pass));

        let mut cv = CVLog::new(true);
        let out = pipeline.run(program(), &Sidecar::new(), &mut cv).unwrap();

        assert_eq!(
            runs.load(std::sync::atomic::Ordering::SeqCst),
            MAX_SWEEPS,
            "the cap bounds the number of sweeps"
        );
        assert!(
            out.diagnostics
                .iter()
                .any(|d| d.group.0 == "pipeline.fixed-point-cap-reached"),
            "hitting the cap must surface a note; got {:?}",
            out.diagnostics
        );
    }

    #[test]
    fn missing_dependency_is_silently_dropped_v1() {
        // CLOC06 doesn't pin behavior here for v1; we silently drop
        // unknown deps so a pass referring to a not-yet-implemented
        // upstream still gets scheduled.
        let mut pipeline = PassPipeline::new();
        pipeline.add(Box::new(NoOpPass::new("solo").with_deps(&["ghost"])));

        let mut cv = CVLog::new(true);
        let out = pipeline.run(program(), &Sidecar::new(), &mut cv).unwrap();
        assert_eq!(out.execution_order, vec!["solo".to_string()]);
    }

    #[test]
    fn diamond_dependency_resolves_correctly() {
        // a → b, a → c, b → d, c → d. Order must be [a, b|c, c|b, d].
        let mut pipeline = PassPipeline::new();
        pipeline.add(Box::new(NoOpPass::new("a")));
        pipeline.add(Box::new(NoOpPass::new("b").with_deps(&["a"])));
        pipeline.add(Box::new(NoOpPass::new("c").with_deps(&["a"])));
        pipeline.add(Box::new(NoOpPass::new("d").with_deps(&["b", "c"])));

        let mut cv = CVLog::new(true);
        let out = pipeline.run(program(), &Sidecar::new(), &mut cv).unwrap();
        assert_eq!(out.execution_order.first().map(|s| s.as_str()), Some("a"));
        assert_eq!(out.execution_order.last().map(|s| s.as_str()), Some("d"));
        // b and c each follow a and precede d.
        let pos = |n: &str| out.execution_order.iter().position(|s| s == n).unwrap();
        assert!(pos("a") < pos("b"));
        assert!(pos("a") < pos("c"));
        assert!(pos("b") < pos("d"));
        assert!(pos("c") < pos("d"));
    }

    #[test]
    fn default_pipeline_is_empty() {
        let p: PassPipeline = Default::default();
        assert!(p.is_empty());
        assert_eq!(p.len(), 0);
    }

    #[test]
    fn pass_error_implements_display_and_error() {
        let e = PassError {
            pass_name: "foo".into(),
            message: "boom".into(),
        };
        let s = format!("{e}");
        assert!(s.contains("foo"));
        assert!(s.contains("boom"));
        let _: &dyn std::error::Error = &e;
    }

    // ------------------------------------------------------------------------
    // PassRegistry tests (CLOC10.A)
    // ------------------------------------------------------------------------

    #[test]
    fn registry_starts_empty() {
        let r = PassRegistry::new();
        assert!(r.is_empty());
        assert_eq!(r.len(), 0);
        assert!(r.registered_names().is_empty());
        // Default impl matches new().
        let d: PassRegistry = Default::default();
        assert!(d.is_empty());
    }

    #[test]
    fn registry_register_and_contains() {
        let mut r = PassRegistry::new();
        r.register("alpha", || Box::new(NoOpPass::new("alpha")))
            .expect("first register succeeds");
        assert!(r.contains("alpha"));
        assert!(!r.contains("beta"));
        assert_eq!(r.len(), 1);
    }

    #[test]
    fn registry_duplicate_name_errors() {
        let mut r = PassRegistry::new();
        r.register("dupe", || Box::new(NoOpPass::new("dupe")))
            .expect("first register succeeds");
        let err = r
            .register("dupe", || Box::new(NoOpPass::new("dupe")))
            .expect_err("second register should fail");
        assert_eq!(err, RegistryError::DuplicateName("dupe".to_string()));
        // Display includes the name and a hint about uniqueness.
        let printed = format!("{err}");
        assert!(printed.contains("dupe"));
        assert!(printed.contains("already registered"));
        // The original registration is still intact.
        assert_eq!(r.len(), 1);
        assert!(r.contains("dupe"));
    }

    #[test]
    fn registry_registered_names_are_sorted() {
        let mut r = PassRegistry::new();
        // Register in deliberately scrambled order.
        for name in &["zeta", "alpha", "mu", "beta"] {
            let n = *name;
            r.register(n, move || Box::new(NoOpPass::new(n))).unwrap();
        }
        assert_eq!(
            r.registered_names(),
            vec![
                "alpha".to_string(),
                "beta".to_string(),
                "mu".to_string(),
                "zeta".to_string()
            ],
        );
    }

    #[test]
    fn registry_build_pipeline_preserves_input_order() {
        let mut r = PassRegistry::new();
        for name in &["a", "b", "c"] {
            let n = *name;
            r.register(n, move || Box::new(NoOpPass::new(n))).unwrap();
        }
        // Independent passes with no deps → input order is the
        // tie-breaker, which proves build_pipeline preserved it.
        let pipeline = r.build_pipeline(&["c", "a", "b"]).expect("ok");
        let mut cv = CVLog::new(true);
        let out = pipeline.run(program(), &Sidecar::new(), &mut cv).unwrap();
        assert_eq!(
            out.execution_order,
            vec!["c".to_string(), "a".to_string(), "b".to_string()],
        );
    }

    #[test]
    fn registry_build_pipeline_unknown_name_errors() {
        let mut r = PassRegistry::new();
        r.register("known", || Box::new(NoOpPass::new("known")))
            .unwrap();
        let err = match r.build_pipeline(&["known", "ghost"]) {
            Ok(_) => panic!("unknown name should error"),
            Err(e) => e,
        };
        assert_eq!(err, RegistryError::UnknownPass("ghost".to_string()));
        // Display includes the missing name.
        let printed = format!("{err}");
        assert!(printed.contains("ghost"));
        assert!(printed.contains("no pass registered"));
    }

    #[test]
    fn registry_factories_produce_fresh_instances() {
        // Each build_pipeline call should re-invoke the factory.
        // We verify by sharing an Arc<AtomicUsize> counter that
        // increments on each construction.
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        let mut r = PassRegistry::new();
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = Arc::clone(&counter);
        r.register("counter", move || {
            counter_clone.fetch_add(1, Ordering::SeqCst);
            Box::new(NoOpPass::new("counter"))
        })
        .unwrap();

        let _p1 = r.build_pipeline(&["counter"]).unwrap();
        let _p2 = r.build_pipeline(&["counter"]).unwrap();
        let _p3 = r.build_pipeline(&["counter", "counter"]).ok(); // would error
                                                                  // Two successful single-pass builds → factory called twice;
                                                                  // the third build fails (PassPipeline rejects duplicate names
                                                                  // at run time, but build_pipeline itself does invoke the
                                                                  // factory once per name before adding). We assert at least 2.
        assert!(counter.load(Ordering::SeqCst) >= 2);
    }

    #[test]
    fn registry_build_pipeline_empty_input_yields_empty_pipeline() {
        let r = PassRegistry::new();
        let pipeline = r.build_pipeline(&[]).expect("empty build ok");
        assert!(pipeline.is_empty());
    }

    #[test]
    fn registry_debug_lists_names() {
        let mut r = PassRegistry::new();
        r.register("alpha", || Box::new(NoOpPass::new("alpha")))
            .unwrap();
        let printed = format!("{r:?}");
        assert!(printed.contains("PassRegistry"));
        assert!(printed.contains("alpha"));
    }

    #[test]
    fn registry_error_implements_std_error() {
        let e: &dyn std::error::Error = &RegistryError::UnknownPass("x".into());
        // Just exercising the trait — if it compiles, we're good.
        let _ = e.to_string();
    }
}
