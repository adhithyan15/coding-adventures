//! # Starlark Compiler -- Compiles Starlark ASTs to bytecode.
//!
//! This crate provides the opcode definitions and compilation pipeline for
//! Starlark. It defines all bytecode opcodes that the Starlark VM executes,
//! organized by category with the high nibble indicating the instruction group.
//!
//! ## The Starlark Compilation Pipeline
//!
//! ```text
//! Starlark source code
//!     | (starlark_lexer)
//! Token stream
//!     | (starlark_parser)
//! AST (ASTNode tree)
//!     | (THIS CRATE)
//! CodeObject (bytecode)
//!     | (starlark_vm)
//! Execution result
//! ```
//!
//! ## Opcode Organization
//!
//! Opcodes are grouped by category using the high nibble (first hex digit):
//!
//! - `0x0_` = Stack operations (push, pop, dup, load constants)
//! - `0x1_` = Variable operations (store/load by name or slot)
//! - `0x2_` = Arithmetic (add, sub, mul, div, bitwise)
//! - `0x3_` = Comparison and boolean (==, !=, <, >, in, not)
//! - `0x4_` = Control flow (jump, branch)
//! - `0x5_` = Functions (make, call, return)
//! - `0x6_` = Collections (build list, dict, tuple)
//! - `0x7_` = Subscript and attribute (indexing, slicing, dot access)
//! - `0x8_` = Iteration (get_iter, for_iter, unpack)
//! - `0x9_` = Module (load statement)
//! - `0xA_` = I/O (print)
//! - `0xF_` = VM control (halt)

use std::collections::HashMap;

// =========================================================================
// Starlark Opcodes
// =========================================================================

/// Starlark bytecode opcodes.
///
/// Each value is a single byte (0x00-0xFF). The high nibble groups opcodes
/// by category. Handlers for each opcode are registered with the GenericVM
/// by the Starlark VM plugin.
///
/// Stack effect notation:
/// - `-> value` = pushes one value
/// - `value ->` = pops one value
/// - `a b -> c` = pops two, pushes one
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Op {
    // ===== Stack Operations (0x0_) =====

    /// Push a constant from the pool. Operand: pool index. -> value
    LoadConst = 0x01,
    /// Discard top of stack. value ->
    Pop = 0x02,
    /// Duplicate top of stack. value -> value value
    Dup = 0x03,
    /// Push None. -> None
    LoadNone = 0x04,
    /// Push True. -> True
    LoadTrue = 0x05,
    /// Push False. -> False
    LoadFalse = 0x06,

    // ===== Variable Operations (0x1_) =====

    /// Pop and store in named variable. Operand: name index. value ->
    StoreName = 0x10,
    /// Push named variable's value. Operand: name index. -> value
    LoadName = 0x11,
    /// Pop and store in local slot. Operand: slot index. value ->
    StoreLocal = 0x12,
    /// Push local slot's value. Operand: slot index. -> value
    LoadLocal = 0x13,
    /// Pop and store in closure cell. Operand: cell index. value ->
    StoreClosure = 0x14,
    /// Push closure cell's value. Operand: cell index. -> value
    LoadClosure = 0x15,

    // ===== Arithmetic Operations (0x2_) =====

    /// Pop two values, push a + b. a b -> result
    Add = 0x20,
    /// Pop two values, push a - b. a b -> result
    Sub = 0x21,
    /// Pop two values, push a * b. a b -> result
    Mul = 0x22,
    /// Pop two values, push a / b (float division). a b -> result
    Div = 0x23,
    /// Pop two values, push a // b. a b -> result
    FloorDiv = 0x24,
    /// Pop two values, push a % b. a b -> result
    Mod = 0x25,
    /// Pop two values, push a ** b. a b -> result
    Power = 0x26,
    /// Pop one value, push -a. a -> -a
    Negate = 0x27,
    /// Pop two values, push a & b. a b -> result
    BitAnd = 0x28,
    /// Pop two values, push a | b. a b -> result
    BitOr = 0x29,
    /// Pop two values, push a ^ b. a b -> result
    BitXor = 0x2A,
    /// Pop one value, push ~a. a -> ~a
    BitNot = 0x2B,
    /// Pop two values, push a << b. a b -> result
    LShift = 0x2C,
    /// Pop two values, push a >> b. a b -> result
    RShift = 0x2D,

    // ===== Comparison Operations (0x3_) =====

    /// Pop two values, push a == b. a b -> bool
    CmpEq = 0x30,
    /// Pop two values, push a != b. a b -> bool
    CmpNe = 0x31,
    /// Pop two values, push a < b. a b -> bool
    CmpLt = 0x32,
    /// Pop two values, push a > b. a b -> bool
    CmpGt = 0x33,
    /// Pop two values, push a <= b. a b -> bool
    CmpLe = 0x34,
    /// Pop two values, push a >= b. a b -> bool
    CmpGe = 0x35,
    /// Pop two values, push a in b. a b -> bool
    CmpIn = 0x36,
    /// Pop two values, push a not in b. a b -> bool
    CmpNotIn = 0x37,

    // ===== Boolean Operations (0x38) =====

    /// Pop one value, push logical not. a -> !a
    Not = 0x38,

    // ===== Control Flow (0x4_) =====

    /// Unconditional jump. Operand: target index.
    Jump = 0x40,
    /// Pop value, jump if falsy. Operand: target. value ->
    JumpIfFalse = 0x41,
    /// Pop value, jump if truthy. Operand: target. value ->
    JumpIfTrue = 0x42,
    /// If top is falsy, jump (keep value); else pop. For `and` short-circuit.
    JumpIfFalseOrPop = 0x43,
    /// If top is truthy, jump (keep value); else pop. For `or` short-circuit.
    JumpIfTrueOrPop = 0x44,

    // ===== Function Operations (0x5_) =====

    /// Create a function object. Operand: flags.
    MakeFunction = 0x50,
    /// Call function with N positional args. Operand: arg count.
    CallFunction = 0x51,
    /// Call function with keyword args. Operand: total arg count.
    CallFunctionKw = 0x52,
    /// Return from function. value ->
    Return = 0x53,

    // ===== Collection Operations (0x6_) =====

    /// Create list from N stack items. Operand: count. items -> list
    BuildList = 0x60,
    /// Create dict from N key-value pairs. Operand: pair count.
    BuildDict = 0x61,
    /// Create tuple from N stack items. Operand: count. items -> tuple
    BuildTuple = 0x62,
    /// Append value to list (for comprehensions). list value -> list
    ListAppend = 0x63,
    /// Set dict entry (for comprehensions). dict key value -> dict
    DictSet = 0x64,

    // ===== Subscript & Attribute Operations (0x7_) =====

    /// obj[key]. obj key -> value
    LoadSubscript = 0x70,
    /// obj[key] = value. obj key value ->
    StoreSubscript = 0x71,
    /// obj.attr. Operand: attr name index. obj -> value
    LoadAttr = 0x72,
    /// obj.attr = value. Operand: attr name index. obj value ->
    StoreAttr = 0x73,
    /// obj[start:stop:step]. Operand: flags.
    LoadSlice = 0x74,

    // ===== Iteration Operations (0x8_) =====

    /// Get iterator from iterable. iterable -> iterator
    GetIter = 0x80,
    /// Get next from iterator, or jump to end. Operand: target.
    ForIter = 0x81,
    /// Unpack N items from sequence. Operand: count. seq -> items
    UnpackSequence = 0x82,

    // ===== Module Operations (0x9_) =====

    /// Load a module. Operand: module name index. -> module
    LoadModule = 0x90,
    /// Extract symbol from module. Operand: symbol name index.
    ImportFrom = 0x91,

    // ===== I/O Operations (0xA_) =====

    /// Pop and print value, capture in output. value ->
    Print = 0xA0,

    // ===== VM Control (0xF_) =====

    /// Stop execution.
    Halt = 0xFF,
}

impl Op {
    /// Convert a u8 to an Op, if valid.
    pub fn from_u8(value: u8) -> Option<Op> {
        match value {
            0x01 => Some(Op::LoadConst), 0x02 => Some(Op::Pop), 0x03 => Some(Op::Dup),
            0x04 => Some(Op::LoadNone), 0x05 => Some(Op::LoadTrue), 0x06 => Some(Op::LoadFalse),
            0x10 => Some(Op::StoreName), 0x11 => Some(Op::LoadName),
            0x12 => Some(Op::StoreLocal), 0x13 => Some(Op::LoadLocal),
            0x14 => Some(Op::StoreClosure), 0x15 => Some(Op::LoadClosure),
            0x20 => Some(Op::Add), 0x21 => Some(Op::Sub), 0x22 => Some(Op::Mul),
            0x23 => Some(Op::Div), 0x24 => Some(Op::FloorDiv), 0x25 => Some(Op::Mod),
            0x26 => Some(Op::Power), 0x27 => Some(Op::Negate),
            0x28 => Some(Op::BitAnd), 0x29 => Some(Op::BitOr), 0x2A => Some(Op::BitXor),
            0x2B => Some(Op::BitNot), 0x2C => Some(Op::LShift), 0x2D => Some(Op::RShift),
            0x30 => Some(Op::CmpEq), 0x31 => Some(Op::CmpNe), 0x32 => Some(Op::CmpLt),
            0x33 => Some(Op::CmpGt), 0x34 => Some(Op::CmpLe), 0x35 => Some(Op::CmpGe),
            0x36 => Some(Op::CmpIn), 0x37 => Some(Op::CmpNotIn), 0x38 => Some(Op::Not),
            0x40 => Some(Op::Jump), 0x41 => Some(Op::JumpIfFalse),
            0x42 => Some(Op::JumpIfTrue), 0x43 => Some(Op::JumpIfFalseOrPop),
            0x44 => Some(Op::JumpIfTrueOrPop),
            0x50 => Some(Op::MakeFunction), 0x51 => Some(Op::CallFunction),
            0x52 => Some(Op::CallFunctionKw), 0x53 => Some(Op::Return),
            0x60 => Some(Op::BuildList), 0x61 => Some(Op::BuildDict),
            0x62 => Some(Op::BuildTuple), 0x63 => Some(Op::ListAppend),
            0x64 => Some(Op::DictSet),
            0x70 => Some(Op::LoadSubscript), 0x71 => Some(Op::StoreSubscript),
            0x72 => Some(Op::LoadAttr), 0x73 => Some(Op::StoreAttr),
            0x74 => Some(Op::LoadSlice),
            0x80 => Some(Op::GetIter), 0x81 => Some(Op::ForIter),
            0x82 => Some(Op::UnpackSequence),
            0x90 => Some(Op::LoadModule), 0x91 => Some(Op::ImportFrom),
            0xA0 => Some(Op::Print),
            0xFF => Some(Op::Halt),
            _ => None,
        }
    }
}

// =========================================================================
// Operator-to-opcode mappings (used by the compiler)
// =========================================================================

/// Maps binary operator symbols to their bytecode opcodes.
pub fn binary_op_map() -> HashMap<&'static str, Op> {
    let mut m = HashMap::new();
    m.insert("+", Op::Add);
    m.insert("-", Op::Sub);
    m.insert("*", Op::Mul);
    m.insert("/", Op::Div);
    m.insert("//", Op::FloorDiv);
    m.insert("%", Op::Mod);
    m.insert("**", Op::Power);
    m.insert("&", Op::BitAnd);
    m.insert("|", Op::BitOr);
    m.insert("^", Op::BitXor);
    m.insert("<<", Op::LShift);
    m.insert(">>", Op::RShift);
    m
}

/// Maps comparison operator symbols to their bytecode opcodes.
pub fn compare_op_map() -> HashMap<&'static str, Op> {
    let mut m = HashMap::new();
    m.insert("==", Op::CmpEq);
    m.insert("!=", Op::CmpNe);
    m.insert("<", Op::CmpLt);
    m.insert(">", Op::CmpGt);
    m.insert("<=", Op::CmpLe);
    m.insert(">=", Op::CmpGe);
    m.insert("in", Op::CmpIn);
    m.insert("not in", Op::CmpNotIn);
    m
}

/// Maps augmented assignment operators to their underlying arithmetic opcodes.
pub fn augmented_assign_map() -> HashMap<&'static str, Op> {
    let mut m = HashMap::new();
    m.insert("+=", Op::Add);
    m.insert("-=", Op::Sub);
    m.insert("*=", Op::Mul);
    m.insert("/=", Op::Div);
    m.insert("//=", Op::FloorDiv);
    m.insert("%=", Op::Mod);
    m.insert("&=", Op::BitAnd);
    m.insert("|=", Op::BitOr);
    m.insert("^=", Op::BitXor);
    m.insert("<<=", Op::LShift);
    m.insert(">>=", Op::RShift);
    m.insert("**=", Op::Power);
    m
}

/// Maps unary operator symbols to their bytecode opcodes.
pub fn unary_op_map() -> HashMap<&'static str, Op> {
    let mut m = HashMap::new();
    m.insert("-", Op::Negate);
    m.insert("~", Op::BitNot);
    m
}

// =========================================================================
// Tests
// =========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_op_values() {
        assert_eq!(Op::LoadConst as u8, 0x01);
        assert_eq!(Op::Add as u8, 0x20);
        assert_eq!(Op::CmpEq as u8, 0x30);
        assert_eq!(Op::Jump as u8, 0x40);
        assert_eq!(Op::MakeFunction as u8, 0x50);
        assert_eq!(Op::BuildList as u8, 0x60);
        assert_eq!(Op::LoadSubscript as u8, 0x70);
        assert_eq!(Op::GetIter as u8, 0x80);
        assert_eq!(Op::LoadModule as u8, 0x90);
        assert_eq!(Op::Print as u8, 0xA0);
        assert_eq!(Op::Halt as u8, 0xFF);
    }

    #[test]
    fn test_op_from_u8() {
        assert_eq!(Op::from_u8(0x01), Some(Op::LoadConst));
        assert_eq!(Op::from_u8(0x20), Some(Op::Add));
        assert_eq!(Op::from_u8(0xFF), Some(Op::Halt));
        assert_eq!(Op::from_u8(0xEE), None); // Invalid opcode
    }

    #[test]
    fn test_binary_op_map() {
        let map = binary_op_map();
        assert_eq!(map["+"], Op::Add);
        assert_eq!(map["-"], Op::Sub);
        assert_eq!(map["*"], Op::Mul);
        assert_eq!(map["/"], Op::Div);
        assert_eq!(map["//"], Op::FloorDiv);
        assert_eq!(map["%"], Op::Mod);
        assert_eq!(map["**"], Op::Power);
        assert_eq!(map["&"], Op::BitAnd);
        assert_eq!(map["|"], Op::BitOr);
        assert_eq!(map["^"], Op::BitXor);
        assert_eq!(map["<<"], Op::LShift);
        assert_eq!(map[">>"], Op::RShift);
        assert_eq!(map.len(), 12);
    }

    #[test]
    fn test_compare_op_map() {
        let map = compare_op_map();
        assert_eq!(map["=="], Op::CmpEq);
        assert_eq!(map["!="], Op::CmpNe);
        assert_eq!(map["<"], Op::CmpLt);
        assert_eq!(map[">"], Op::CmpGt);
        assert_eq!(map["<="], Op::CmpLe);
        assert_eq!(map[">="], Op::CmpGe);
        assert_eq!(map["in"], Op::CmpIn);
        assert_eq!(map["not in"], Op::CmpNotIn);
        assert_eq!(map.len(), 8);
    }

    #[test]
    fn test_augmented_assign_map() {
        let map = augmented_assign_map();
        assert_eq!(map["+="], Op::Add);
        assert_eq!(map["-="], Op::Sub);
        assert_eq!(map["*="], Op::Mul);
        assert_eq!(map["/="], Op::Div);
        assert_eq!(map["**="], Op::Power);
        assert_eq!(map.len(), 12);
    }

    #[test]
    fn test_unary_op_map() {
        let map = unary_op_map();
        assert_eq!(map["-"], Op::Negate);
        assert_eq!(map["~"], Op::BitNot);
        assert_eq!(map.len(), 2);
    }

    #[test]
    fn test_op_category_grouping() {
        // Stack ops are in 0x0_ range
        assert!((Op::LoadConst as u8) < 0x10);
        assert!((Op::Pop as u8) < 0x10);

        // Variable ops are in 0x1_ range
        assert!((Op::StoreName as u8) >= 0x10 && (Op::StoreName as u8) < 0x20);
        assert!((Op::LoadName as u8) >= 0x10 && (Op::LoadName as u8) < 0x20);

        // Arithmetic ops are in 0x2_ range
        assert!((Op::Add as u8) >= 0x20 && (Op::Add as u8) < 0x30);
        assert!((Op::RShift as u8) >= 0x20 && (Op::RShift as u8) < 0x30);

        // Comparison ops are in 0x3_ range
        assert!((Op::CmpEq as u8) >= 0x30 && (Op::CmpEq as u8) < 0x40);

        // Control flow ops are in 0x4_ range
        assert!((Op::Jump as u8) >= 0x40 && (Op::Jump as u8) < 0x50);
    }

    #[test]
    fn test_all_opcodes_round_trip() {
        // Every defined opcode should round-trip through from_u8
        let ops = [
            Op::LoadConst, Op::Pop, Op::Dup, Op::LoadNone, Op::LoadTrue, Op::LoadFalse,
            Op::StoreName, Op::LoadName, Op::StoreLocal, Op::LoadLocal,
            Op::StoreClosure, Op::LoadClosure,
            Op::Add, Op::Sub, Op::Mul, Op::Div, Op::FloorDiv, Op::Mod, Op::Power, Op::Negate,
            Op::BitAnd, Op::BitOr, Op::BitXor, Op::BitNot, Op::LShift, Op::RShift,
            Op::CmpEq, Op::CmpNe, Op::CmpLt, Op::CmpGt, Op::CmpLe, Op::CmpGe,
            Op::CmpIn, Op::CmpNotIn, Op::Not,
            Op::Jump, Op::JumpIfFalse, Op::JumpIfTrue,
            Op::JumpIfFalseOrPop, Op::JumpIfTrueOrPop,
            Op::MakeFunction, Op::CallFunction, Op::CallFunctionKw, Op::Return,
            Op::BuildList, Op::BuildDict, Op::BuildTuple, Op::ListAppend, Op::DictSet,
            Op::LoadSubscript, Op::StoreSubscript, Op::LoadAttr, Op::StoreAttr, Op::LoadSlice,
            Op::GetIter, Op::ForIter, Op::UnpackSequence,
            Op::LoadModule, Op::ImportFrom,
            Op::Print,
            Op::Halt,
        ];
        for op in &ops {
            let value = *op as u8;
            let roundtrip = Op::from_u8(value).unwrap();
            assert_eq!(*op, roundtrip, "Failed round-trip for {:?} (0x{:02X})", op, value);
        }
    }
}
