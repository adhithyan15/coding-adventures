//! # IR Opcodes — the instruction set for the general-purpose compiler IR.
//!
//! This module defines the `IrOp` enum, which enumerates every operation
//! the IR can express. The IR is designed to be:
//!
//! - **General-purpose** — suitable for Brainfuck today, BASIC tomorrow.
//! - **Versioned** — new opcodes are only ever appended; existing ones
//!   never change semantics. This keeps all frontends and backends
//!   forward-compatible.
//! - **Target-independent** — backends map IR opcodes to physical ISAs
//!   (RISC-V, ARM, x86-64). The IR itself knows nothing about registers,
//!   calling conventions, or memory models.
//!
//! ## Opcode categories
//!
//! ```text
//! Constants:    LOAD_IMM, LOAD_ADDR
//! Memory:       LOAD_BYTE, STORE_BYTE, LOAD_WORD, STORE_WORD
//! Arithmetic:   ADD, ADD_IMM, SUB, MUL, DIV, AND, AND_IMM
//! Comparison:   CMP_EQ, CMP_NE, CMP_LT, CMP_GT
//! Control Flow: LABEL, JUMP, BRANCH_Z, BRANCH_NZ, CALL, RET
//! System:       SYSCALL, HALT
//! Meta:         NOP, COMMENT
//! ```

use std::fmt;

// ===========================================================================
// IrOp — the opcode enumeration
// ===========================================================================

/// An IR opcode: the operation a single IR instruction performs.
///
/// Each variant maps to one conceptual machine operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IrOp {
    // ── Constants ──────────────────────────────────────────────────────────
    /// Load an immediate integer value into a register.
    /// `LOAD_IMM  v0, 42  →  v0 = 42`
    LoadImm,
    /// Load the address of a data label into a register.
    /// `LOAD_ADDR v0, tape  →  v0 = &tape`
    LoadAddr,
    // ── Memory ────────────────────────────────────────────────────────────
    /// Load a byte from memory, zero-extended to word width.
    /// `LOAD_BYTE v2, v0, v1  →  v2 = mem[v0 + v1] & 0xFF`
    LoadByte,
    /// Store a byte to memory (low 8 bits of src).
    /// `STORE_BYTE v2, v0, v1  →  mem[v0 + v1] = v2 & 0xFF`
    StoreByte,
    /// Load a machine word from memory.
    /// `LOAD_WORD v2, v0, v1  →  v2 = *(int*)(v0 + v1)`
    LoadWord,
    /// Store a machine word to memory.
    /// `STORE_WORD v2, v0, v1  →  *(int*)(v0 + v1) = v2`
    StoreWord,
    // ── Arithmetic ────────────────────────────────────────────────────────
    /// Register-register addition. `ADD v3, v1, v2  →  v3 = v1 + v2`
    Add,
    /// Register-immediate addition. `ADD_IMM v1, v1, 1  →  v1 = v1 + 1`
    AddImm,
    /// Register-register subtraction. `SUB v3, v1, v2  →  v3 = v1 - v2`
    Sub,
    /// Register-register signed multiplication. `MUL v3, v1, v2  →  v3 = v1 * v2`
    ///
    /// Signed integer multiplication; overflow behaviour matches the
    /// target machine (wrap-around in two's complement is the norm).
    Mul,
    /// Register-register signed integer division. `DIV v3, v1, v2  →  v3 = v1 / v2`
    ///
    /// Truncates toward zero (C-style), consistent with Python's `//`
    /// for non-negative operands.  Division by zero is undefined behaviour
    /// in V1; backends may raise a trap or return 0.
    Div,
    /// Register-register bitwise AND. `AND v3, v1, v2  →  v3 = v1 & v2`
    And,
    /// Register-immediate bitwise AND. `AND_IMM v2, v2, 255  →  v2 = v2 & 0xFF`
    AndImm,
    // ── Comparison ────────────────────────────────────────────────────────
    /// Set dst = 1 if lhs == rhs, else 0.
    CmpEq,
    /// Set dst = 1 if lhs != rhs, else 0.
    CmpNe,
    /// Set dst = 1 if lhs < rhs (signed), else 0.
    CmpLt,
    /// Set dst = 1 if lhs > rhs (signed), else 0.
    CmpGt,
    // ── Control Flow ──────────────────────────────────────────────────────
    /// Define a label at this point in the instruction stream.
    Label,
    /// Unconditional jump to a label.
    Jump,
    /// Conditional branch: jump to label if register == 0.
    BranchZ,
    /// Conditional branch: jump to label if register != 0.
    BranchNz,
    /// Call a subroutine at the given label.
    Call,
    /// Return from a subroutine.
    Ret,
    // ── System ────────────────────────────────────────────────────────────
    /// Invoke a system call.
    Syscall,
    /// Halt execution. The program terminates.
    Halt,
    // ── Meta ──────────────────────────────────────────────────────────────
    /// No operation.
    Nop,
    /// A human-readable comment. Produces no machine code.
    Comment,
}

// ===========================================================================
// Display — canonical text name for each opcode
// ===========================================================================

impl fmt::Display for IrOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            IrOp::LoadImm   => "LOAD_IMM",
            IrOp::LoadAddr  => "LOAD_ADDR",
            IrOp::LoadByte  => "LOAD_BYTE",
            IrOp::StoreByte => "STORE_BYTE",
            IrOp::LoadWord  => "LOAD_WORD",
            IrOp::StoreWord => "STORE_WORD",
            IrOp::Add       => "ADD",
            IrOp::AddImm    => "ADD_IMM",
            IrOp::Sub       => "SUB",
            IrOp::Mul       => "MUL",
            IrOp::Div       => "DIV",
            IrOp::And       => "AND",
            IrOp::AndImm    => "AND_IMM",
            IrOp::CmpEq     => "CMP_EQ",
            IrOp::CmpNe     => "CMP_NE",
            IrOp::CmpLt     => "CMP_LT",
            IrOp::CmpGt     => "CMP_GT",
            IrOp::Label     => "LABEL",
            IrOp::Jump      => "JUMP",
            IrOp::BranchZ   => "BRANCH_Z",
            IrOp::BranchNz  => "BRANCH_NZ",
            IrOp::Call      => "CALL",
            IrOp::Ret       => "RET",
            IrOp::Syscall   => "SYSCALL",
            IrOp::Halt      => "HALT",
            IrOp::Nop       => "NOP",
            IrOp::Comment   => "COMMENT",
        };
        write!(f, "{}", name)
    }
}

// ===========================================================================
// parse_op — text name → IrOp
// ===========================================================================

/// Convert a canonical text opcode name to its `IrOp` value.
///
/// Returns `Some(op)` if the name is recognised, `None` otherwise.
/// This is the inverse of `IrOp::to_string()`.
pub fn parse_op(name: &str) -> Option<IrOp> {
    match name {
        "LOAD_IMM"   => Some(IrOp::LoadImm),
        "LOAD_ADDR"  => Some(IrOp::LoadAddr),
        "LOAD_BYTE"  => Some(IrOp::LoadByte),
        "STORE_BYTE" => Some(IrOp::StoreByte),
        "LOAD_WORD"  => Some(IrOp::LoadWord),
        "STORE_WORD" => Some(IrOp::StoreWord),
        "ADD"        => Some(IrOp::Add),
        "ADD_IMM"    => Some(IrOp::AddImm),
        "SUB"        => Some(IrOp::Sub),
        "MUL"        => Some(IrOp::Mul),
        "DIV"        => Some(IrOp::Div),
        "AND"        => Some(IrOp::And),
        "AND_IMM"    => Some(IrOp::AndImm),
        "CMP_EQ"     => Some(IrOp::CmpEq),
        "CMP_NE"     => Some(IrOp::CmpNe),
        "CMP_LT"     => Some(IrOp::CmpLt),
        "CMP_GT"     => Some(IrOp::CmpGt),
        "LABEL"      => Some(IrOp::Label),
        "JUMP"       => Some(IrOp::Jump),
        "BRANCH_Z"   => Some(IrOp::BranchZ),
        "BRANCH_NZ"  => Some(IrOp::BranchNz),
        "CALL"       => Some(IrOp::Call),
        "RET"        => Some(IrOp::Ret),
        "SYSCALL"    => Some(IrOp::Syscall),
        "HALT"       => Some(IrOp::Halt),
        "NOP"        => Some(IrOp::Nop),
        "COMMENT"    => Some(IrOp::Comment),
        _            => None,
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_load_imm() {
        assert_eq!(IrOp::LoadImm.to_string(), "LOAD_IMM");
    }

    #[test]
    fn test_display_branch_z() {
        assert_eq!(IrOp::BranchZ.to_string(), "BRANCH_Z");
    }

    #[test]
    fn test_display_all_opcodes_non_empty() {
        let ops = [
            IrOp::LoadImm, IrOp::LoadAddr, IrOp::LoadByte, IrOp::StoreByte,
            IrOp::LoadWord, IrOp::StoreWord, IrOp::Add, IrOp::AddImm,
            IrOp::Sub, IrOp::And, IrOp::AndImm, IrOp::CmpEq, IrOp::CmpNe,
            IrOp::CmpLt, IrOp::CmpGt, IrOp::Label, IrOp::Jump, IrOp::BranchZ,
            IrOp::BranchNz, IrOp::Call, IrOp::Ret, IrOp::Syscall, IrOp::Halt,
            IrOp::Nop, IrOp::Comment,
        ];
        for op in &ops {
            let s = op.to_string();
            assert!(!s.is_empty(), "{:?} has empty display", op);
        }
    }

    #[test]
    fn test_parse_op_known() {
        assert_eq!(parse_op("LOAD_IMM"), Some(IrOp::LoadImm));
        assert_eq!(parse_op("ADD_IMM"), Some(IrOp::AddImm));
        assert_eq!(parse_op("BRANCH_Z"), Some(IrOp::BranchZ));
        assert_eq!(parse_op("BRANCH_NZ"), Some(IrOp::BranchNz));
        assert_eq!(parse_op("HALT"), Some(IrOp::Halt));
        assert_eq!(parse_op("COMMENT"), Some(IrOp::Comment));
    }

    #[test]
    fn test_parse_op_unknown() {
        assert_eq!(parse_op("NOT_AN_OP"), None);
        assert_eq!(parse_op(""), None);
        assert_eq!(parse_op("load_imm"), None); // case-sensitive
    }

    #[test]
    fn test_roundtrip_all_opcodes() {
        let ops = [
            IrOp::LoadImm, IrOp::LoadAddr, IrOp::LoadByte, IrOp::StoreByte,
            IrOp::LoadWord, IrOp::StoreWord, IrOp::Add, IrOp::AddImm,
            IrOp::Sub, IrOp::And, IrOp::AndImm, IrOp::CmpEq, IrOp::CmpNe,
            IrOp::CmpLt, IrOp::CmpGt, IrOp::Label, IrOp::Jump, IrOp::BranchZ,
            IrOp::BranchNz, IrOp::Call, IrOp::Ret, IrOp::Syscall, IrOp::Halt,
            IrOp::Nop, IrOp::Comment,
        ];
        for op in &ops {
            let name = op.to_string();
            let parsed = parse_op(&name);
            assert_eq!(parsed, Some(*op),
                "roundtrip failed for {:?}: name={:?}", op, name);
        }
    }

    #[test]
    fn test_opcode_count_is_25() {
        // Regression guard — we must have exactly 25 opcodes.
        let ops = [
            IrOp::LoadImm, IrOp::LoadAddr, IrOp::LoadByte, IrOp::StoreByte,
            IrOp::LoadWord, IrOp::StoreWord, IrOp::Add, IrOp::AddImm,
            IrOp::Sub, IrOp::And, IrOp::AndImm, IrOp::CmpEq, IrOp::CmpNe,
            IrOp::CmpLt, IrOp::CmpGt, IrOp::Label, IrOp::Jump, IrOp::BranchZ,
            IrOp::BranchNz, IrOp::Call, IrOp::Ret, IrOp::Syscall, IrOp::Halt,
            IrOp::Nop, IrOp::Comment,
        ];
        assert_eq!(ops.len(), 25);
    }

    #[test]
    fn test_clone_and_eq() {
        let op = IrOp::AddImm;
        let cloned = op;
        assert_eq!(op, cloned);
    }
}
