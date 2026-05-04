//! # interpreter-ir — InterpreterIR (IIR), the bytecode for the LANG pipeline.
//!
//! InterpreterIR is the "middle tier" of the LANG pipeline:
//!
//! ```text
//! Language Frontend (lexer → parser → type-checker → compiler)
//!         │
//!         ▼  IIRModule   (this crate)
//!   ┌─────────────┐
//!   │  vm-core    │  ← interprets IIR, profiles types
//!   │  jit-core   │  ← detects hot functions, compiles to native
//!   └─────────────┘
//!         │
//!         ▼  bytes (WASM / JVM / Intel 4004 / …)
//!   Backend runtimes
//! ```
//!
//! ## What this crate provides
//!
//! - [`instr::Operand`] — an instruction operand: variable name, int/float/bool literal
//! - [`instr::IIRInstr`] — a single instruction with optional runtime profiling state
//! - [`slot_state::SlotKind`] + [`slot_state::SlotState`] — V8 Ignition-style
//!   feedback state machine (UNINIT → MONO → POLY → MEGA)
//! - [`function::FunctionTypeStatus`] — compilation tier (FullyTyped / PartiallyTyped / Untyped)
//! - [`function::IIRFunction`] — a named function with parameters, return type, and body
//! - [`module::IIRModule`] — top-level container for all functions in a program
//! - [`opcodes`] — predicate functions for opcode category membership, type helpers
//! - [`serialise`] — compact binary serialisation / deserialisation
//! - [`source_loc::SourceLoc`] — one source position, indexed in lockstep with
//!   [`function::IIRFunction::instructions`]; substrate for the dev-tools stack
//!   (LSP, debugger, coverage, AOT DWARF/PDB)
//!
//! ## Design principles
//!
//! **Language agnosticism.** IIR has no concept of "Tetrad" or "BASIC" — it is
//! a generic IR suitable for any register-based language.  Type strings are
//! plain `String`s so each frontend can define its own type ontology without
//! modifying this crate.
//!
//! **Dynamic-type friendly.** Instructions carry an optional `type_hint`; frontends
//! that don't have static types emit `"any"`.  The profiler fills in observed types
//! at runtime; the JIT uses the observations to specialise.
//!
//! **Zero overhead for typed programs.** The profiler only records observations for
//! instructions whose `type_hint == "any"`.  A fully-typed Tetrad program pays
//! zero profiling cost.
//!
//! ## Example: building an IIRModule
//!
//! ```
//! use interpreter_ir::module::IIRModule;
//! use interpreter_ir::function::IIRFunction;
//! use interpreter_ir::instr::{IIRInstr, Operand};
//! use interpreter_ir::serialise::{serialise, deserialise};
//!
//! // Build a simple "add(a, b) -> u8" function.
//! let fn_ = IIRFunction::new(
//!     "add",
//!     vec![("a".into(), "u8".into()), ("b".into(), "u8".into())],
//!     "u8",
//!     vec![
//!         IIRInstr::new("add", Some("v0".into()),
//!             vec![Operand::Var("a".into()), Operand::Var("b".into())], "u8"),
//!         IIRInstr::new("ret", None,
//!             vec![Operand::Var("v0".into())], "u8"),
//!     ],
//! );
//!
//! let mut module = IIRModule::new("hello.bas", "basic");
//! module.entry_point = Some("add".into());
//! module.add_or_replace(fn_);
//!
//! // Validate — should be clean.
//! assert!(module.validate().is_empty());
//!
//! // Round-trip through binary serialisation.
//! let bytes = serialise(&module);
//! let recovered = deserialise(&bytes).unwrap();
//! assert_eq!(recovered.get_function("add").unwrap().params.len(), 2);
//! ```

pub mod function;
pub mod instr;
pub mod module;
pub mod opcodes;
pub mod serialise;
pub mod slot_state;
pub mod source_loc;

// Re-export the most commonly used types at the crate root for ergonomic use.
pub use function::{FunctionTypeStatus, IIRFunction};
pub use instr::{IIRInstr, Operand};
pub use module::IIRModule;
pub use slot_state::{SlotKind, SlotState, MAX_POLYMORPHIC_OBSERVATIONS};
pub use source_loc::SourceLoc;
