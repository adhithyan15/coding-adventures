//! # `vm-runtime` — LANG15: Linkable, Embeddable Runtime Library
//!
//! `vm-runtime` is the **linkable, embeddable form of `vm-core`** (LANG02).
//! It exists so that AOT-compiled binaries (LANG04) can fall back to
//! interpretation for the parts of a program that could not be fully
//! specialised, without depending on a host Python interpreter.
//!
//! ## Architecture
//!
//! ```text
//! AOT binary (.aot)
//!   │
//!   ├── Native code section       ← compiled IIRFunctions (fully typed)
//!   ├── vm_iir_table section      ← IIRFunctions that fell back to interpreter
//!   ├── Relocation table          ← patches native code with runtime indices
//!   └── String pool               ← symbol names for debugging
//!        │
//!        ▼
//!     vm-runtime
//!   ┌──────────────────────────────────────┐
//!   │ InProcessVMRuntime  ← dev / tests    │
//!   │   vm_execute(fn_idx, args)           │
//!   │   vm_call_builtin(bi_idx, args)      │
//!   │   vm_resume_at(fn, ip, regs)         │
//!   │                                      │
//!   │ IIRTableWriter / IIRTableReader      │
//!   │   IIRT binary format (magic, index)  │
//!   │                                      │
//!   │ RuntimeLevel  (0–3)                  │
//!   │   required_level(module)             │
//!   │                                      │
//!   │ VmResult / VmResultTag               │
//!   │   C-ABI tagged result type           │
//!   │                                      │
//!   │ RelocationKind / RelocationEntry     │
//!   │   AOT → linker relocation protocol   │
//!   └──────────────────────────────────────┘
//! ```
//!
//! ## Runtime levels
//!
//! | Level | Name | What's linked | Typical program |
//! |-------|------|--------------|-----------------|
//! | 0 | `None` | Nothing (pure native) | Fully-typed Tetrad on 4004 |
//! | 1 | `Minimal` | Dispatch loop + arith/control | Mostly-typed program |
//! | 2 | `Standard` | Level 1 + builtins + I/O | Programs using `print` |
//! | 3 | `Full` | Level 2 + profiler + GC hooks | Hybrid AOT+JIT / GC |
//!
//! Use [`level::required_level`] to compute the minimum level needed.
//!
//! ## Quick start
//!
//! ```
//! use interpreter_ir::module::IIRModule;
//! use interpreter_ir::function::IIRFunction;
//! use interpreter_ir::instr::{IIRInstr, Operand};
//! use vm_runtime::iir_table::{IIRTableWriter, IIRTableReader};
//! use vm_runtime::level::{RuntimeLevel, required_level};
//! use vm_runtime::result::VmResult;
//!
//! // 1. Determine runtime level needed.
//! let fn_ = IIRFunction::new("main", vec![], "void",
//!     vec![IIRInstr::new("ret_void", None, vec![], "void")]);
//! let mut module = IIRModule::new("demo", "tetrad");
//! module.functions.push(fn_.clone());
//! assert_eq!(required_level(&module), RuntimeLevel::None); // no "any" instrs
//!
//! // 2. Serialise an IIR table for unspecialised functions.
//! let mut writer = IIRTableWriter::new();
//! writer.add_function(fn_);
//! let blob = writer.serialise();
//! assert_eq!(&blob[0..4], b"IIRT");
//!
//! // 3. Read it back.
//! let reader = IIRTableReader::new(blob).unwrap();
//! assert_eq!(reader.lookup("main"), Some(0));
//! ```

pub mod iir_table;
pub mod inprocess;
pub mod level;
pub mod reloc;
pub mod result;

// Convenient re-exports.
pub use iir_table::{IIRTableReader, IIRTableWriter};
pub use inprocess::InProcessVMRuntime;
pub use level::{required_level, RuntimeLevel};
pub use reloc::{RelocationEntry, RelocationKind};
pub use result::{VmResult, VmResultTag};

#[cfg(test)]
mod tests {
    use super::*;
    use interpreter_ir::function::IIRFunction;
    use interpreter_ir::instr::{IIRInstr, Operand};
    use interpreter_ir::module::IIRModule;

    fn simple_fn(name: &str) -> IIRFunction {
        IIRFunction::new(name, vec![], "void",
            vec![IIRInstr::new("ret_void", None, vec![], "void")])
    }

    fn make_module(instrs: Vec<IIRInstr>) -> IIRModule {
        let fn_ = IIRFunction::new("main", vec![], "void", instrs);
        let mut m = IIRModule::new("test", "tetrad");
        m.functions.push(fn_);
        m
    }

    // ── Integration: level + table + inprocess ────────────────────────────

    #[test]
    fn fully_typed_module_level_is_none() {
        let module = make_module(vec![
            IIRInstr::new("const", Some("x".into()), vec![Operand::Int(1)], "u8"),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ]);
        assert_eq!(required_level(&module), RuntimeLevel::None);
    }

    #[test]
    fn iir_table_round_trip() {
        let fn_ = simple_fn("helper");
        let mut writer = IIRTableWriter::new();
        writer.add_function(fn_);
        let blob = writer.serialise();
        let reader = IIRTableReader::new(blob).unwrap();
        assert_eq!(reader.function_count(), 1);
        assert_eq!(reader.lookup("helper"), Some(0));
    }

    #[test]
    fn inprocess_runtime_lookup() {
        let mut m = IIRModule::new("t", "tetrad");
        m.functions.push(simple_fn("alpha"));
        m.functions.push(simple_fn("beta"));
        let rt = InProcessVMRuntime::from_module(m);
        assert_eq!(rt.lookup_function("alpha"), Some(0));
        assert_eq!(rt.lookup_function("beta"), Some(1));
        assert_eq!(rt.lookup_function("gamma"), None);
    }

    #[test]
    fn vm_result_trap_vs_void() {
        let void = VmResult::void();
        let trap = VmResult::trap(5);
        assert!(void.is_void());
        assert!(!void.is_trap());
        assert!(trap.is_trap());
        assert!(!trap.is_void());
    }

    #[test]
    fn relocation_entry_serialise_length() {
        let e = RelocationEntry {
            site_offset: 0xFF,
            symbol: "sym".into(),
            kind: RelocationKind::BuiltinIndex,
            addend: 3,
        };
        assert_eq!(e.serialise().len(), 16);
    }
}
