//! # Register VM
//!
//! A register-based virtual machine modelled on V8 Ignition — Google Chrome's
//! bytecode interpreter for JavaScript.
//!
//! ## What is a register-based VM?
//!
//! There are two dominant designs for bytecode interpreters:
//!
//! | Design       | How operands move               | Examples                |
//! |--------------|----------------------------------|-------------------------|
//! | Stack-based  | Push/pop an operand stack        | JVM, CPython, WebAssembly |
//! | Register-based | Named register slots + accumulator | Lua VM, V8 Ignition, Dalvik |
//!
//! In a register machine, instructions name their operands explicitly
//! (e.g. `ADD r2, r5` means "add register 2 to register 5").  V8 Ignition
//! takes this further with an **accumulator** model: most instructions read
//! one operand from the implicit *accumulator* register and write their result
//! back there, while the other operand comes from an explicitly numbered
//! general-purpose register.  This hybrid design reduces the number of
//! register-move instructions compared to a pure register machine, while
//! keeping instruction encoding simpler than a pure stack machine.
//!
//! ## Module structure
//!
//! ```text
//! register_vm
//! ├── opcodes   — numeric opcode constants + opcode_name()
//! ├── types     — VMValue, CodeObject, RegisterInstruction, CallFrame, …
//! ├── feedback  — type-feedback vectors (Mono/Poly/Mega transitions)
//! ├── scope     — lexical scope / closure Context chain
//! └── vm        — the main VM struct and execution loop
//! ```
//!
//! ## Quick start
//!
//! ```rust
//! use register_vm::{VM, CodeObject, RegisterInstruction, VMValue};
//! use register_vm::opcodes::{LDA_SMI, STAR, ADD, HALT};
//!
//! // 10 + 32 = 42
//! let code = CodeObject {
//!     name: "main".to_string(),
//!     instructions: vec![
//!         RegisterInstruction { opcode: LDA_SMI, operands: vec![10], feedback_slot: None },
//!         RegisterInstruction { opcode: STAR,    operands: vec![0],  feedback_slot: None },
//!         RegisterInstruction { opcode: LDA_SMI, operands: vec![32], feedback_slot: None },
//!         RegisterInstruction { opcode: ADD,     operands: vec![0],  feedback_slot: None },
//!         RegisterInstruction { opcode: HALT,    operands: vec![],   feedback_slot: None },
//!     ],
//!     constants: vec![],
//!     names: vec![],
//!     register_count: 1,
//!     feedback_slot_count: 0,
//!     parameter_count: 0,
//! };
//!
//! let mut vm = VM::new();
//! let result = vm.execute(&code);
//! assert_eq!(result.return_value, VMValue::Integer(42));
//! ```
//!
//! ## V8 Ignition concepts implemented here
//!
//! | Concept              | Where implemented       |
//! |----------------------|-------------------------|
//! | Accumulator register | `CallFrame::accumulator` |
//! | Register file        | `CallFrame::registers`   |
//! | Constant pool        | `CodeObject::constants`  |
//! | Name table           | `CodeObject::names`      |
//! | Feedback vector      | `CallFrame::feedback_vector` |
//! | Context chain        | `scope::Context`         |
//! | Hidden classes       | `types::VMObject::hidden_class_id` |
//! | Type feedback states | `feedback::FeedbackSlot` |

pub mod feedback;
pub mod opcodes;
pub mod scope;
pub mod types;
pub mod vm;

pub use opcodes::*;
pub use types::{
    CallFrame, CodeObject, RegisterInstruction, VMError, VMResult, VMValue, UNDEFINED,
};
pub use vm::VM;
