//! Error definitions, limits, and security bounds for `avm-bytecode-lifter`.

#![deny(unsafe_code)]

use std::fmt;

/// Maximum bytecode size accepted for lifting (16 MB).
pub const MAX_BYTECODE_SIZE: usize = 16 * 1024 * 1024;

/// Maximum abstract operand stack depth (CWE-400 defense).
pub const MAX_OPERAND_STACK: usize = 1024;

/// Maximum number of constants in a constant pool (CWE-400 defense).
pub const MAX_CONSTANTS: usize = 65_536;

/// Maximum number of instructions lifted in a single module (CWE-400 defense).
pub const MAX_INSTRUCTIONS: usize = 100_000;

/// Maximum function count per bytecode payload.
pub const MAX_FUNCTIONS: usize = 8_192;

/// Maximum VM step execution budget per frame/script (SWF00 §7.2 / CWE-400 defense).
pub const MAX_VM_STEPS: u64 = 2_000_000;

/// Comprehensive error type for AVM bytecode decoding, CFG verification,
/// stack simulation, and deterministic execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LifterError {
    /// Branch offset points outside bytecode or does not land on an instruction boundary (CWE-125 / CWE-787).
    InvalidBranchTarget,
    /// Stack depth underflows below zero during abstract simulation or execution (CWE-125 / CWE-787).
    StackUnderflow,
    /// Stack depth exceeds `MAX_OPERAND_STACK` (CWE-400 / CWE-770).
    StackOverflow,
    /// Bytecode stream ended unexpectedly while reading a tag or instruction.
    UnexpectedEof,
    /// Malformed bytecode or header structure.
    InvalidBytecode(String),
    /// Unsupported or invalid bytecode opcode.
    UnsupportedOpcode(u8),
    /// Constant pool item count exceeds `MAX_CONSTANTS`.
    ConstantPoolLimitExceeded,
    /// Function declaration count exceeds `MAX_FUNCTIONS`.
    FunctionLimitExceeded,
    /// Instruction count exceeds `MAX_INSTRUCTIONS`.
    InstructionLimitExceeded,
    /// Method body or ABC structure exceeded size or depth bounds.
    MethodBodyLimitExceeded,
    /// VM runtime execution error on vm-core.
    ExecutionError(String),
    /// Attempted modification of frozen native prototype (CWE-1321 defense).
    PrototypePollutionBlocked(String),
    /// Execution instruction budget exceeded `MAX_VM_STEPS` (CWE-400 defense).
    ScriptTimeout,
}

impl fmt::Display for LifterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidBranchTarget => write!(
                f,
                "invalid branch target: offset lands outside instruction boundary"
            ),
            Self::StackUnderflow => write!(f, "stack underflow: operand stack dropped below zero"),
            Self::StackOverflow => {
                write!(f, "stack overflow: operand stack exceeded maximum limit")
            }
            Self::UnexpectedEof => write!(f, "unexpected EOF while parsing bytecode"),
            Self::InvalidBytecode(msg) => write!(f, "invalid bytecode: {}", msg),
            Self::UnsupportedOpcode(op) => write!(f, "unsupported opcode: 0x{:02X}", op),
            Self::ConstantPoolLimitExceeded => write!(f, "constant pool limit exceeded"),
            Self::FunctionLimitExceeded => write!(f, "function count limit exceeded"),
            Self::InstructionLimitExceeded => write!(f, "instruction limit exceeded"),
            Self::MethodBodyLimitExceeded => write!(f, "method body limit exceeded"),
            Self::ExecutionError(msg) => write!(f, "execution error: {}", msg),
            Self::PrototypePollutionBlocked(msg) => {
                write!(f, "prototype pollution blocked: {}", msg)
            }
            Self::ScriptTimeout => write!(
                f,
                "script execution timeout: instruction step budget exhausted"
            ),
        }
    }
}

impl std::error::Error for LifterError {}
