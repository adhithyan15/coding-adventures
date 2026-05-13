//! # iir-to-jvm-class-file — IIR → JVM class file backend.
//!
//! Lowers an [`interpreter_ir::IIRModule`] to a [`jvm_class_file::JvmClassFile`]
//! without going through the deprecated `compiler-ir` layer.
//!
//! ## Pipeline
//!
//! ```text
//! IIRModule
//!   → validate_for_jvm()      — pre-flight check, returns Vec<String>
//!   → lower_iir_to_jvm()      — two-pass lowering, returns JvmClassFile
//! ```
//!
//! ## Why IIR → JVM directly?
//!
//! The existing `ir-to-jvm-class-file` crate lowers `compiler_ir::IrProgram` —
//! a flat, single-function IR with no type information.  `IIRModule` is richer:
//! it has multiple functions, named variables, static type hints, and a full
//! comparison operator set that maps cleanly to JVM's conditional branch
//! instructions.  This crate exploits that richness without retrofitting it
//! through a deprecated intermediate.
//!
//! ## Quick start
//!
//! ```rust
//! use interpreter_ir::{IIRModule, IIRFunction, IIRInstr, Operand};
//! use iir_to_jvm_class_file::{validate_for_jvm, lower_iir_to_jvm, IIRJvmConfig};
//!
//! let fn_ = IIRFunction::new(
//!     "main",
//!     vec![],
//!     "void",
//!     vec![IIRInstr::new("ret_void", None, vec![], "void")],
//! );
//! let module = IIRModule {
//!     name: "demo".into(),
//!     functions: vec![fn_],
//!     entry_point: Some("main".into()),
//!     language: "test".into(),
//!     exports: vec![],
//!     imports: vec![],
//! };
//!
//! let errors = validate_for_jvm(&module);
//! assert!(errors.is_empty());
//!
//! let config = IIRJvmConfig::new("Demo");
//! let class_file = lower_iir_to_jvm(&module, &config).unwrap();
//! assert_eq!(class_file.methods.len(), 1);
//! assert_eq!(class_file.this_class_name, "Demo");
//! ```
//!
//! ## Supported opcodes
//!
//! | IIR op | JVM emission |
//! |--------|-------------|
//! | `const` (Int/Bool) | `iconst_N` / `bipush` / `sipush` |
//! | `const` (Float/Double) | `fconst_N` / `dconst_N` / `ldc` |
//! | `add` | `iadd` / `ladd` / `fadd` / `dadd` |
//! | `sub` | `isub` / `lsub` / `fsub` / `dsub` |
//! | `mul` | `imul` / `lmul` / `fmul` / `dmul` |
//! | `div` | `idiv` / `ldiv` / `fdiv` / `ddiv` |
//! | `mod` | `irem` / `lrem` |
//! | `neg` | `ineg` / `lneg` / `fneg` / `dneg` |
//! | `and` | `iand` / `land` |
//! | `or` | `ior` / `lor` |
//! | `xor` | `ixor` / `lxor` |
//! | `not` | `iconst_m1; ixor` (bitwise NOT) |
//! | `shl` | `ishl` / `lshl` |
//! | `shr` | `ishr` / `lshr` |
//! | `cmp_eq` | `if_icmpne +7; iconst_1; goto +4; iconst_0` |
//! | `cmp_ne` | `if_icmpeq +7; iconst_1; goto +4; iconst_0` |
//! | `cmp_lt` | `if_icmpge +7; iconst_1; goto +4; iconst_0` |
//! | `cmp_le` | `if_icmpgt +7; iconst_1; goto +4; iconst_0` |
//! | `cmp_gt` | `if_icmple +7; iconst_1; goto +4; iconst_0` |
//! | `cmp_ge` | `if_icmplt +7; iconst_1; goto +4; iconst_0` |
//! | `label` | backpatch target — no bytes |
//! | `jmp` | `goto <offset>` + fixup |
//! | `jmp_if_true` | `iload cond; ifne <offset>` + fixup |
//! | `jmp_if_false` | `iload cond; ifeq <offset>` + fixup |
//! | `ret` | `iload/lload/fload/dload; ireturn/lreturn/…` |
//! | `ret_void` | `return` |
//! | `call` | `iload args…; invokestatic CP#; istore dest` |
//! | `load_reg` | `iload/lload/… src; istore/lstore/… dest` |
//! | `store_reg` | same as `load_reg` |
//! | `type_assert` | nop (erased) |
//!
//! ## Unsupported (validation rejects)
//!
//! `call_builtin`, `io_in`, `io_out`, `cast`, `load_mem`, `store_mem`, `alloc`,
//! `box`, `unbox`, `field_load`, `field_store`, `is_null`, `safepoint`, and any
//! instruction with `type_hint` of `"any"`, `"polymorphic"`, `"str"`, or
//! `"ref<…>"`.
//!
//! Float type hints (`f32`, `f64`) and float constant operands **are supported**
//! — unlike the BEAM backend.
//!
//! ## Module structure
//!
//! | Module | Responsibility |
//! |--------|----------------|
//! | [`validate`] | Pre-flight checks on `IIRModule` |
//! | [`lower`] | Two-pass IIR → JVM bytecode lowering; error types; config |
//! | [`codegen`] | `IIRJvmCodeGenerator` — thin adapter (`name` / `validate` / `generate`) |

pub mod codegen;
pub mod lower;
pub mod validate;

// ── Public re-exports ─────────────────────────────────────────────────────────

pub use validate::validate_for_jvm;
pub use lower::{IIRJvmConfig, IIRJvmError, lower_iir_to_jvm, serialize_jvm_class_file};
pub use codegen::IIRJvmCodeGenerator;

// Re-export JvmClassFile so callers do not need a separate jvm-class-file
// dependency just to use the output type.
pub use jvm_class_file::JvmClassFile;
