//! # `typing-spectrum` — LANG22 compilation strategy for the optional-typing spectrum.
//!
//! LANG22 §"The optional-typing spectrum" classifies every LANG program
//! somewhere on a continuum from "fully typed" (Tetrad, Algol) to "untyped"
//! (Twig, MRI Ruby).  The same compilation pipeline serves all three tiers;
//! what changes is *when and how* the pipeline specialises.
//!
//! This crate provides the **strategy layer** that bridges the type-tier
//! information produced by [`iir-type-checker`] and the compilation decisions
//! made by `aot-core`, `aot-with-pgo`, and `jit-core`:
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────┐
//! │  Language frontend                                                  │
//! │    produces IIRModule with type_hints (or "any" for dynamic langs)  │
//! └──────────────────────────────┬──────────────────────────────────────┘
//!                                │
//!                                ▼
//! ┌─────────────────────────────────────────────────────────────────────┐
//! │  iir-type-checker                                                   │
//! │    infer_and_check(&mut module)                                     │
//! │    ├─ infer_types_mut  : fills type_hint for inferable "any" instrs │
//! │    └─ check_module     : validates the enriched annotations         │
//! └──────────────────────────────┬──────────────────────────────────────┘
//!                                │
//!                                ▼
//! ┌─────────────────────────────────────────────────────────────────────┐
//! │  typing-spectrum  (THIS CRATE)                                      │
//! │    advisory::advise(&module)                                        │
//! │    ├─ module_tier        : TypingTier for the whole program         │
//! │    ├─ recommended_mode   : CompilationMode (tree-walk/AOT/JIT)      │
//! │    ├─ jit_threshold      : call count before JIT promotion          │
//! │    └─ per-function       : FunctionAdvisory for each function       │
//! └──────────────────────────────┬──────────────────────────────────────┘
//!                                │
//!                     ┌──────────┴──────────┐
//!                     ▼                     ▼
//!              aot-no-profile           jit-core
//!              aot-with-pgo             (uses threshold + tier)
//! ```
//!
//! ## The five compilation modes
//!
//! | Mode | Description | Requires deopt? | Requires `.ldp` input? |
//! |------|-------------|-----------------|------------------------|
//! | 1: [`TreeWalking`](mode::CompilationMode::TreeWalking) | Interpret `IIRModule` directly | No | No |
//! | 2: [`AotNoProfile`](mode::CompilationMode::AotNoProfile) | Typed → native; untyped → runtime calls | **No** | No |
//! | 3: [`AotWithPgo`](mode::CompilationMode::AotWithPgo) | Speculation + deopt anchors from `.ldp` | Yes | Yes |
//! | 4: [`Jit`](mode::CompilationMode::Jit) | Tier-up from interpreter on call-count | Yes | No |
//! | 5: [`JitThenAotWithPgo`](mode::CompilationMode::JitThenAotWithPgo) | JIT → save `.ldp` → AOT-PGO at release | Yes | Yes (at release) |
//!
//! ## The three typing tiers (LANG22 §"The optional-typing spectrum")
//!
//! | LANG22 label | `TypingTier` | Examples |
//! |---|---|---|
//! | Tier A — fully typed | `FullyTyped` | Tetrad, Algol, Rust, C+ |
//! | Tier B — partially typed | `Partial(fraction)` | TypeScript, Sorbet-Ruby, Hack |
//! | Tier C — untyped | `Untyped` | Twig, MRI Ruby, Python (pre-mypy) |
//!
//! ## Quick start
//!
//! ```
//! use interpreter_ir::module::IIRModule;
//! use interpreter_ir::function::IIRFunction;
//! use interpreter_ir::instr::{IIRInstr, Operand};
//! use iir_type_checker::infer_and_check;
//! use typing_spectrum::advisory::advise;
//! use typing_spectrum::mode::CompilationMode;
//!
//! // Build a fully-typed "add" function.
//! let fn_ = IIRFunction::new(
//!     "add",
//!     vec![("a".into(), "i64".into()), ("b".into(), "i64".into())],
//!     "i64",
//!     vec![
//!         IIRInstr::new("add", Some("v".into()),
//!             vec![Operand::Var("a".into()), Operand::Var("b".into())], "i64"),
//!         IIRInstr::new("ret", None, vec![Operand::Var("v".into())], "i64"),
//!     ],
//! );
//! let mut module = IIRModule::new("math", "tetrad");
//! module.add_or_replace(fn_);
//!
//! // Step 1: infer + check types.
//! infer_and_check(&mut module);
//!
//! // Step 2: get compilation advisory.
//! let adv = advise(&module);
//!
//! // Fully-typed → AOT-no-profile (no JIT warmup required).
//! assert_eq!(adv.recommended_mode, CompilationMode::AotNoProfile);
//! assert!(!adv.requires_deopt());
//! assert_eq!(adv.functions[0].jit_threshold.call_count, 0);
//! ```

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

pub mod advisory;
pub mod canonical;
pub mod mode;
pub mod threshold;

// Re-export the most commonly used items at the crate root.
pub use advisory::{advise, CompilationAdvisory, FunctionAdvisory};
pub use canonical::{map_frontend_type, ALL_CANONICAL_TYPES, TYPE_ANY, TYPE_BOOL, TYPE_I64};
pub use mode::CompilationMode;
pub use threshold::JitPromotionThreshold;
