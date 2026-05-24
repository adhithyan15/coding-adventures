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
//! - `OneShot` iteration. `FixedPoint` is accepted but executes once
//!   in v1 (with a diagnostic note) — fixed-point looping lands once
//!   we have a pass that actually mutates the AST.
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
    /// Re-run until [`PassOutput::changed`] is `false` (or the
    /// per-pipeline fixed-point cap is hit). v1 executes a
    /// FixedPoint pass once and notes the limitation; the full loop
    /// arrives with the first mutating pass.
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

/// The pass scheduler.
///
/// Build one with [`PassPipeline::new`], register passes with
/// [`PassPipeline::add`], then call [`PassPipeline::run`] to execute
/// the whole graph.
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

    /// Run the pipeline. See module docs for behavior.
    pub fn run(
        &self,
        program: Program,
        sidecar: &Sidecar,
        cv: &mut CVLog,
    ) -> Result<PipelineOutput, PassError> {
        let order = self.topo_sort()?;
        let mut current = program;
        let mut diagnostics = Vec::new();
        let mut stats = HashMap::new();

        // Build a lookup so we can fetch passes by name during execution.
        let by_name: HashMap<&'static str, &dyn Pass> = self
            .passes
            .iter()
            .map(|p| (p.name(), p.as_ref()))
            .collect();

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
            // We tag-append against the new program root's CV since
            // that's the durable handle.
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

            // If this pass requested FixedPoint, v1 notes the
            // limitation as a diagnostic and runs only once.
            if pass.iteration_policy() == IterationPolicy::FixedPoint {
                // Diagnostic.cv is still plain String (closure-typechecker
                // hasn't migrated to Option<CvId> yet — tracked as a
                // Phase 1.x follow-up). For untraced programs we emit
                // an empty cv to keep the diagnostic shape stable —
                // tooling that filters on diagnostic.cv just sees "".
                diagnostics.push(Diagnostic {
                    cv: current.cv.clone().unwrap_or_default(),
                    severity: coding_adventures_closure_typechecker::Severity::Note,
                    group: coding_adventures_closure_typechecker::DiagnosticGroup::new(
                        "pipeline.fixed-point-not-yet-iterated",
                    ),
                    message: format!(
                        "pass {:?} requested FixedPoint iteration but v1 only runs it once; \
                         multi-iteration support arrives with the first mutating pass.",
                        pass.name()
                    ),
                });
            }

            // Roll diagnostics + stats into the pipeline accumulators.
            diagnostics.extend(output.diagnostics);
            stats.insert(pass.name().to_string(), output.stats);
            current = output.program;
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
                pass_name: remaining
                    .first()
                    .map(|s| s.to_string())
                    .unwrap_or_default(),
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
        assert_eq!(out.execution_order, vec!["alpha".to_string(), "beta".to_string()]);
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
        assert_eq!(out.execution_order, vec!["alpha".to_string(), "beta".to_string()]);
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
    fn fixed_point_runs_once_with_diagnostic_note() {
        let mut pipeline = PassPipeline::new();
        pipeline.add(Box::new(
            NoOpPass::new("fp").with_policy(IterationPolicy::FixedPoint),
        ));

        let mut cv = CVLog::new(true);
        let out = pipeline.run(program(), &Sidecar::new(), &mut cv).unwrap();
        assert_eq!(out.execution_order, vec!["fp".to_string()]);
        // The diagnostic about FixedPoint not being iterated yet
        // should be present.
        assert!(
            out.diagnostics
                .iter()
                .any(|d| d.group.0 == "pipeline.fixed-point-not-yet-iterated"),
            "expected a FixedPoint note diagnostic; got {:?}",
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
}
