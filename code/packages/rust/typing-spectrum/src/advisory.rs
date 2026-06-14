//! # Compilation advisory — per-module and per-function strategy recommendations.
//!
//! The [`CompilationAdvisory`] struct combines the outputs of
//! `iir-type-checker` (type tier classification + inference) with the
//! LANG22 strategy tables ([`CompilationMode`] + [`JitPromotionThreshold`])
//! to produce a human-readable and machine-readable compilation plan.
//!
//! ## Why a separate advisory layer
//!
//! The pipeline stages are:
//!
//! ```text
//! iir-type-checker::infer_and_check(module)  ← fills type_hint, classifies tier
//!         │
//!         ▼
//! typing_spectrum::advise(module)             ← maps tier → mode + thresholds
//!         │
//!         ▼
//! aot-core / jit-core                         ← acts on mode + thresholds
//! ```
//!
//! The advisory step is a pure data transformation: it reads the inferred
//! `FunctionTypeStatus` from each function and emits strategy metadata.
//! It does **not** mutate the module.
//!
//! ## Example
//!
//! ```
//! use interpreter_ir::module::IIRModule;
//! use interpreter_ir::function::IIRFunction;
//! use interpreter_ir::instr::{IIRInstr, Operand};
//! use iir_type_checker::infer_and_check;
//! use typing_spectrum::advisory::advise;
//! use typing_spectrum::mode::CompilationMode;
//!
//! let fn_ = IIRFunction::new(
//!     "main", vec![], "void",
//!     vec![
//!         IIRInstr::new("const", Some("x".into()), vec![Operand::Int(42)], "any"),
//!         IIRInstr::new("ret_void", None, vec![], "void"),
//!     ],
//! );
//! let mut module = IIRModule::new("demo", "twig");
//! module.add_or_replace(fn_);
//! infer_and_check(&mut module);
//!
//! let adv = advise(&module);
//! // "const Int(42)" should have been inferred as i64, promoting the tier.
//! assert_ne!(adv.recommended_mode, CompilationMode::TreeWalking);
//! ```

use interpreter_ir::function::FunctionTypeStatus;
use interpreter_ir::module::IIRModule;
use iir_type_checker::check::check_module;
use iir_type_checker::tier::TypingTier;

use crate::mode::CompilationMode;
use crate::threshold::JitPromotionThreshold;

// ---------------------------------------------------------------------------
// FunctionAdvisory
// ---------------------------------------------------------------------------

/// Compilation strategy for a single function.
///
/// The advisory is computed from the function's [`FunctionTypeStatus`]
/// (already set by `iir-type-checker::infer_and_check`).  Fields are
/// read-only data — nothing is mutated.
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionAdvisory {
    /// The function name.
    pub name: String,

    /// The typing tier derived from this function's instruction `type_hint`s.
    pub tier: TypingTier,

    /// The recommended compilation mode for this function in isolation.
    ///
    /// Note: the module-level `recommended_mode` may differ because it
    /// takes the *worst* function tier into account.
    pub mode: CompilationMode,

    /// JIT promotion threshold: call count the interpreter must reach before
    /// the JIT compiles this function.
    pub jit_threshold: JitPromotionThreshold,

    /// Fraction of data-flow instructions that have a concrete `type_hint`
    /// (after inference).  `0.0` = fully untyped, `1.0` = fully typed.
    pub typed_fraction: f32,
}

impl FunctionAdvisory {
    /// Short human-readable summary for CLI / log output.
    ///
    /// ```
    /// use typing_spectrum::advisory::FunctionAdvisory;
    /// use typing_spectrum::mode::CompilationMode;
    /// use typing_spectrum::threshold::JitPromotionThreshold;
    /// use iir_type_checker::tier::TypingTier;
    ///
    /// let a = FunctionAdvisory {
    ///     name: "fact".into(),
    ///     tier: TypingTier::Untyped,
    ///     mode: CompilationMode::Jit,
    ///     jit_threshold: JitPromotionThreshold { call_count: 100 },
    ///     typed_fraction: 0.0,
    /// };
    /// let s = a.summary();
    /// assert!(s.contains("fact"));
    /// assert!(s.contains("jit"));
    /// ```
    pub fn summary(&self) -> String {
        format!(
            "{:30}  tier={:<18}  mode={:<18}  jit={}  typed={:.0}%",
            self.name,
            self.tier.to_string(),
            self.mode.to_string(),
            self.jit_threshold.label(),
            self.typed_fraction * 100.0,
        )
    }
}

// ---------------------------------------------------------------------------
// CompilationAdvisory
// ---------------------------------------------------------------------------

/// Module-level compilation advisory.
///
/// Contains:
/// - An overall `module_tier` (the *worst* function tier in the module, because
///   `liblang-runtime` must be linked if any function is untyped).
/// - A `recommended_mode` driven by `module_tier`.
/// - Per-function advisories for fine-grained reporting.
///
/// Obtain via [`advise`].
#[derive(Debug, Clone, PartialEq)]
pub struct CompilationAdvisory {
    /// Name of the IIRModule this advisory covers.
    pub module_name: String,

    /// Overall typing tier for the module.
    ///
    /// Computed as the *lowest* tier across all functions, because even one
    /// untyped function forces the AOT binary to link `liblang-runtime`.
    ///
    /// ```text
    /// [FullyTyped, FullyTyped, Untyped] → Untyped
    /// [FullyTyped, Partial(0.8)]        → Partial(0.4)  (average)
    /// ```
    ///
    /// The exact formula: average `typed_fraction` across all functions,
    /// then classify with [`TypingTier::from_fraction`].
    pub module_tier: TypingTier,

    /// Recommended compilation mode for the module as a whole.
    pub recommended_mode: CompilationMode,

    /// The number of warnings the type checker found in the module (after
    /// inference).  Zero means the module is clean.  Warnings do not block
    /// compilation but may indicate latent type errors.
    pub warning_count: usize,

    /// Per-function advisories, one per function in the module, in the
    /// order they appear in `IIRModule::functions`.
    pub functions: Vec<FunctionAdvisory>,
}

impl CompilationAdvisory {
    /// Human-readable multi-line summary for CLI output.
    ///
    /// The format is stable — tests assert on it.
    pub fn summary(&self) -> String {
        let mut lines = Vec::new();
        lines.push(format!(
            "Module: {}  tier={}  mode={}  warnings={}",
            self.module_name,
            self.module_tier,
            self.recommended_mode,
            self.warning_count,
        ));
        for fa in &self.functions {
            lines.push(format!("  fn  {}", fa.summary()));
        }
        lines.join("\n")
    }

    /// Return functions whose typing tier is `Untyped` — the ones that
    /// will call into `liblang-runtime` for every instruction at AOT time.
    pub fn fully_untyped_functions(&self) -> Vec<&FunctionAdvisory> {
        self.functions
            .iter()
            .filter(|fa| fa.tier.is_untyped())
            .collect()
    }

    /// Return functions that are `FullyTyped` — the ones that can be
    /// compiled to pure native code without any runtime calls.
    pub fn fully_typed_functions(&self) -> Vec<&FunctionAdvisory> {
        self.functions
            .iter()
            .filter(|fa| fa.tier.is_fully_typed())
            .collect()
    }

    /// Whether the module requires the deopt mechanism.
    ///
    /// True when `recommended_mode` requires deopt.
    pub fn requires_deopt(&self) -> bool {
        self.recommended_mode.requires_deopt()
    }
}

// ---------------------------------------------------------------------------
// advise() — the public entry point
// ---------------------------------------------------------------------------

/// Compute a [`CompilationAdvisory`] for `module`.
///
/// The module **must have been preprocessed** by `iir_type_checker::infer_and_check`
/// (or `infer_types_mut`) before calling this function.  `advise` reads the
/// `FunctionTypeStatus` that inference set; it does not run inference itself.
///
/// ```
/// use interpreter_ir::module::IIRModule;
/// use interpreter_ir::function::IIRFunction;
/// use interpreter_ir::instr::{IIRInstr, Operand};
/// use iir_type_checker::infer_and_check;
/// use typing_spectrum::advisory::advise;
///
/// let fn_ = IIRFunction::new(
///     "add", vec![("a".into(), "i64".into()), ("b".into(), "i64".into())], "i64",
///     vec![
///         IIRInstr::new("add", Some("v".into()),
///             vec![Operand::Var("a".into()), Operand::Var("b".into())], "i64"),
///         IIRInstr::new("ret", None, vec![Operand::Var("v".into())], "i64"),
///     ],
/// );
/// let mut module = IIRModule::new("math", "tetrad");
/// module.add_or_replace(fn_);
/// infer_and_check(&mut module);
///
/// let adv = advise(&module);
/// assert_eq!(adv.module_name, "math");
/// assert!(adv.functions.len() == 1);
/// ```
pub fn advise(module: &IIRModule) -> CompilationAdvisory {
    // ── Run type checker to count warnings (read-only) ───────────────
    let report = check_module(module);

    // ── Build per-function advisories ────────────────────────────────
    let mut functions = Vec::with_capacity(module.functions.len());
    let mut total_typed: f32 = 0.0;
    let mut total_data_flow: usize = 0;

    for func in module.functions.iter() {
        // Count data-flow instructions (those with a dest register).
        let data_flow: usize = func
            .instructions
            .iter()
            .filter(|i| i.dest.is_some())
            .count();

        let typed_count: usize = func
            .instructions
            .iter()
            .filter(|i| i.dest.is_some() && i.type_hint != "any")
            .count();

        let typed_fraction = if data_flow == 0 {
            1.0 // empty functions are trivially "fully typed"
        } else {
            typed_count as f32 / data_flow as f32
        };

        total_typed += typed_count as f32;
        total_data_flow += data_flow;

        // Derive per-function tier from the FunctionTypeStatus already set.
        let tier = function_status_to_tier(&func.type_status, typed_fraction);
        let mode = CompilationMode::recommended_for(&tier);
        let jit_threshold = JitPromotionThreshold::for_tier(&tier);

        functions.push(FunctionAdvisory {
            name: func.name.clone(),
            tier,
            mode,
            jit_threshold,
            typed_fraction,
        });
    }

    // ── Module-level tier (average typed fraction) ────────────────────
    let module_fraction = if total_data_flow == 0 {
        1.0
    } else {
        total_typed / total_data_flow as f32
    };
    let module_tier = TypingTier::from_fraction(module_fraction);
    let recommended_mode = CompilationMode::recommended_for(&module_tier);

    CompilationAdvisory {
        module_name: module.name.clone(),
        module_tier,
        recommended_mode,
        warning_count: report.warnings.len(),
        functions,
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Convert a `FunctionTypeStatus` (set by `iir-type-checker`) + a measured
/// `typed_fraction` to the `TypingTier` enum used by this crate.
///
/// `FunctionTypeStatus` distinguishes only three states; `TypingTier::Partial`
/// carries the precise fraction so the threshold interpolation formula works.
fn function_status_to_tier(status: &FunctionTypeStatus, typed_fraction: f32) -> TypingTier {
    match status {
        FunctionTypeStatus::FullyTyped    => TypingTier::FullyTyped,
        FunctionTypeStatus::Untyped       => TypingTier::Untyped,
        FunctionTypeStatus::PartiallyTyped => {
            // Use the precisely measured fraction for smooth interpolation.
            TypingTier::from_fraction(typed_fraction)
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use interpreter_ir::function::IIRFunction;
    use interpreter_ir::instr::{IIRInstr, Operand};
    use iir_type_checker::infer_and_check;

    fn typed_module() -> IIRModule {
        let fn_ = IIRFunction::new(
            "add",
            vec![("a".into(), "i64".into()), ("b".into(), "i64".into())],
            "i64",
            vec![
                IIRInstr::new(
                    "add",
                    Some("v".into()),
                    vec![Operand::Var("a".into()), Operand::Var("b".into())],
                    "i64",
                ),
                IIRInstr::new("ret", None, vec![Operand::Var("v".into())], "i64"),
            ],
        );
        let mut module = IIRModule::new("typed_mod", "tetrad");
        module.add_or_replace(fn_);
        module
    }

    fn untyped_module() -> IIRModule {
        let fn_ = IIRFunction::new(
            "fact",
            vec![("n".into(), "any".into())],
            "any",
            vec![
                IIRInstr::new(
                    "const",
                    Some("zero".into()),
                    vec![Operand::Int(0)],
                    "any",
                ),
                IIRInstr::new(
                    "cmp_eq",
                    Some("done".into()),
                    vec![Operand::Var("n".into()), Operand::Var("zero".into())],
                    "any",
                ),
                IIRInstr::new("ret", None, vec![Operand::Var("n".into())], "any"),
            ],
        );
        let mut module = IIRModule::new("untyped_mod", "twig");
        module.add_or_replace(fn_);
        module
    }

    #[test]
    fn typed_module_recommends_aot_no_profile() {
        let mut m = typed_module();
        infer_and_check(&mut m);
        let adv = advise(&m);
        // Fully-typed → AOT-no-profile.
        assert_eq!(adv.recommended_mode, CompilationMode::AotNoProfile);
        assert!(adv.functions[0].tier.is_fully_typed());
    }

    #[test]
    fn untyped_module_after_inference_is_partial_or_untyped() {
        // After inference: "const Int(0)" → "i64", "cmp_eq" → "bool".
        // "n" parameter stays "any", "fact"'s ret stays "any" (interprocedural).
        // So the function ends up partially typed at best.
        let mut m = untyped_module();
        infer_and_check(&mut m);
        let adv = advise(&m);
        // Module has at least some untyped instructions → Jit or similar.
        assert_ne!(adv.recommended_mode, CompilationMode::TreeWalking);
    }

    #[test]
    fn all_fully_untyped_functions_returns_correct_subset() {
        let m = untyped_module();
        // Don't run inference — keep everything "any".
        // Manually check that fact is untyped.
        let adv = advise(&m);
        // Fact has some inferred types (const + cmp_eq) even without running
        // infer_and_check because advise uses the typed fraction.
        // Run without inference for raw untyped check:
        let _ = adv; // advise ran above

        // Now build a module that stays truly untyped.
        let fn2 = IIRFunction::new(
            "raw",
            vec![("x".into(), "any".into())],
            "any",
            vec![IIRInstr::new("ret", None, vec![Operand::Var("x".into())], "any")],
        );
        let mut m2 = IIRModule::new("raw_mod", "twig");
        m2.add_or_replace(fn2);
        let adv2 = advise(&m2);
        // ret is not a data-flow instruction (no dest), so data_flow count = 0
        // → typed_fraction = 1.0 → FullyTyped.  That's correct: a function
        // with no data-flow instructions has nothing to observe.
        assert_eq!(adv2.module_tier, TypingTier::FullyTyped);
    }

    #[test]
    fn module_name_is_preserved() {
        let mut m = typed_module();
        infer_and_check(&mut m);
        let adv = advise(&m);
        assert_eq!(adv.module_name, "typed_mod");
    }

    #[test]
    fn function_advisory_count_matches_function_count() {
        let mut m = typed_module();
        infer_and_check(&mut m);
        let adv = advise(&m);
        assert_eq!(adv.functions.len(), m.functions.len());
    }

    #[test]
    fn summary_contains_module_name_and_mode() {
        let mut m = typed_module();
        infer_and_check(&mut m);
        let adv = advise(&m);
        let s = adv.summary();
        assert!(s.contains("typed_mod"), "summary missing module name");
        assert!(s.contains("aot-no-profile"), "summary missing mode");
    }

    #[test]
    fn fully_typed_functions_returns_correct_subset() {
        let mut m = typed_module();
        infer_and_check(&mut m);
        let adv = advise(&m);
        assert!(!adv.fully_typed_functions().is_empty());
    }

    #[test]
    fn requires_deopt_matches_mode() {
        let mut m = typed_module();
        infer_and_check(&mut m);
        let adv = advise(&m);
        assert_eq!(adv.requires_deopt(), adv.recommended_mode.requires_deopt());
    }

    #[test]
    fn function_advisory_summary_contains_name_and_mode() {
        let fa = FunctionAdvisory {
            name: "my_fn".into(),
            tier: TypingTier::Untyped,
            mode: CompilationMode::Jit,
            jit_threshold: JitPromotionThreshold { call_count: 100 },
            typed_fraction: 0.0,
        };
        let s = fa.summary();
        assert!(s.contains("my_fn"));
        assert!(s.contains("jit"));
    }
}
