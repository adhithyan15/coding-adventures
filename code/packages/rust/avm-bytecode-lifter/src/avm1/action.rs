//! AVM1 action structures.

#![deny(unsafe_code)]

use super::value::Avm1Value;

/// An individual parsed AVM1 action with source byte offset and byte length.
#[derive(Debug, Clone, PartialEq)]
pub struct Avm1Action {
    /// Byte offset within the bytecode stream where this action starts.
    pub offset: usize,
    /// Total byte length of this action including header and payload.
    pub length: usize,
    /// Decoded action payload.
    pub kind: Avm1ActionKind,
}

/// Decoded AVM1 action variants.
#[derive(Debug, Clone, PartialEq)]
pub enum Avm1ActionKind {
    End,
    NextFrame,
    PreviousFrame,
    Play,
    Stop,
    ToggleQuality,
    StopSounds,
    Add,
    Subtract,
    Multiply,
    Divide,
    Equals,
    Less,
    And,
    Or,
    Not,
    StringEquals,
    StringLength,
    StringExtract,
    Pop,
    ToInteger,
    GetVariable,
    SetVariable,
    StringAdd,
    Trace,
    CharToAscii,
    AsciiToChar,
    GetTime,
    CallFunction,
    Return,
    NewObject,
    Add2,
    Less2,
    Equals2,
    ToNumber,
    ToString,
    GetMember,
    SetMember,
    CallMethod,
    GotoFrame(u16),
    StoreRegister(u8),
    ConstantPool(Vec<String>),
    Push(Vec<Avm1Value>),
    BranchAlways(i16),
    BranchIfTrue(i16),
    DefineFunction {
        name: String,
        params: Vec<String>,
        body_len: usize,
        body: Vec<Avm1Action>,
    },
    DefineFunction2 {
        name: String,
        register_count: u8,
        flags: u16,
        params: Vec<(u8, String)>,
        body_len: usize,
        body: Vec<Avm1Action>,
    },
}
