//! # RISC-V RV32I Simulator with M-mode privileged extensions
//!
//! Implements all 37 RV32I instructions plus M-mode CSR access, trap handling,
//! and mret. Built on the cpu-simulator crate for RegisterFile and Memory.

pub mod opcodes;
pub mod csr;
pub mod decode;
pub mod execute;
pub mod encoding;
pub mod simulator;
pub mod core_adapter;

pub use csr::CSRFile;
pub use core_adapter::RiscVISADecoder;
pub use simulator::{
    ExecutionResult, HostEvent, RiscVSimulator, HOST_ECALL_ARGUMENT2_HIGH_REGISTER,
    HOST_ECALL_ARGUMENT2_LOW_REGISTER, HOST_ECALL_ARGUMENT_HIGH_REGISTER,
    HOST_ECALL_ARGUMENT_LOW_REGISTER, HOST_ECALL_EXIT, HOST_ECALL_F64_ADD,
    HOST_ECALL_F64_CMP, HOST_ECALL_F64_DIV, HOST_ECALL_F64_MUL,
    HOST_ECALL_F64_SUB, HOST_ECALL_F64_TO_I64_TRUNC, HOST_ECALL_I64_TO_F64,
    HOST_ECALL_READ_BYTE, HOST_ECALL_SERVICE_REGISTER, HOST_ECALL_WRITE_BYTE,
    HOST_ECALL_WRITE_I64,
};
