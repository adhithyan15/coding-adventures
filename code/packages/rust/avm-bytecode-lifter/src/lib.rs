//! # AVM Bytecode Lifter (SWF04)
//!
//! Binary bytecode lifters translating compiled Flash bytecode into
//! `InterpreterIR` (`LANG01`) executable deterministically on `vm-core` (`LANG02`).
//!
//! Provides:
//! - **AVM1 Action Lifter**: Decodes `DoAction` / `DoInitAction` tag action byte streams.
//! - **AVM2 ABC Lifter**: Decodes `DoABC` tag constant pools and method bodies.
//! - **Mandatory CFG & Stack Verification**: Rejects invalid branch targets (`LifterError::InvalidBranchTarget`),
//!   operand stack underflow (`LifterError::StackUnderflow`), and stack overflow (`LifterError::StackOverflow`).
//! - **CWE-1321 Defense**: Isolated prototype environments with immutable native built-in prototypes.
//! - **Deterministic Execution**: Enforces `MAX_VM_STEPS <= 2_000_000` steps per execution.

#![deny(unsafe_code)]

pub mod avm1;
pub mod avm2;
pub mod capabilities;
pub mod error;
pub mod runtime;

pub use avm1::{Avm1Action, Avm1ActionKind, Avm1Lifter, Avm1Parser, Avm1Value, Avm1Verifier};
pub use avm2::{
    AbcConstantPool, AbcDecodedInstr, AbcFile, AbcMethod, AbcMethodBody, AbcMultiname,
    AbcNamespace, AbcParser, AbcScript, Avm2Lifter, Avm2Verifier,
};
pub use capabilities::PureCapabilities;
pub use error::{
    LifterError, MAX_BYTECODE_SIZE, MAX_CONSTANTS, MAX_FUNCTIONS, MAX_INSTRUCTIONS,
    MAX_OPERAND_STACK, MAX_VM_STEPS,
};
pub use runtime::{AvmEnvironment, AvmRunner};

/// Convenience alias matching historical SWF error naming conventions.
pub type SwfError = LifterError;

/// Lift an AVM1 bytecode slice into an `interpreter_ir::module::IIRModule`.
pub fn lift_avm1(
    bytecode: &[u8],
    module_name: &str,
) -> Result<interpreter_ir::module::IIRModule, LifterError> {
    let mut parser = Avm1Parser::a1p_new(bytecode)?;
    let actions = parser.a1p_parse_all()?;
    let mut lifter = Avm1Lifter::a1l_new(module_name);
    lifter.a1l_lift_module(&actions, bytecode.len())
}

/// Lift an AVM2 ABC bytecode slice into an `interpreter_ir::module::IIRModule`.
pub fn lift_avm2(
    bytecode: &[u8],
    module_name: &str,
) -> Result<interpreter_ir::module::IIRModule, LifterError> {
    let mut parser = AbcParser::a2p_new(bytecode)?;
    let abc_file = parser.a2p_parse()?;
    let mut lifter = Avm2Lifter::a2l_new(module_name);
    lifter.a2l_lift_file(&abc_file)
}

/// Lift and execute AVM1 bytecode, returning captured trace output.
pub fn run_avm1(bytecode: &[u8]) -> Result<String, LifterError> {
    let module = lift_avm1(bytecode, "avm1_script")?;
    let mut runner = AvmRunner::runner_new();
    let _ = runner.runner_execute(&module)?;
    Ok(runner.runner_output())
}

/// Lift and execute AVM2 ABC bytecode, returning captured trace output.
pub fn run_avm2(bytecode: &[u8]) -> Result<String, LifterError> {
    let module = lift_avm2(bytecode, "avm2_script")?;
    let mut runner = AvmRunner::runner_new();
    let _ = runner.runner_execute(&module)?;
    Ok(runner.runner_output())
}

/// Validates that stack depth does not exceed security limit (CWE-400).
#[inline]
pub fn validate_stack_depth(depth: usize) -> Result<bool, LifterError> {
    if depth <= MAX_OPERAND_STACK {
        Ok(true)
    } else {
        Err(LifterError::StackOverflow)
    }
}

/// Validates that instruction count does not exceed security limit (CWE-400).
#[inline]
pub fn validate_instruction_count(n: usize) -> Result<bool, LifterError> {
    if n <= MAX_INSTRUCTIONS {
        Ok(true)
    } else {
        Err(LifterError::InstructionLimitExceeded)
    }
}

/// Validates that constant pool item count does not exceed security limit (CWE-400).
#[inline]
pub fn validate_constant_count(n: usize) -> Result<bool, LifterError> {
    if n <= MAX_CONSTANTS {
        Ok(true)
    } else {
        Err(LifterError::ConstantPoolLimitExceeded)
    }
}
