//! AVM1 pushed literal and dynamic values.

#![deny(unsafe_code)]

/// Representation of values pushed by `ActionPush`.
#[derive(Debug, Clone, PartialEq)]
pub enum Avm1Value {
    /// String literal (type 0).
    String(String),
    /// Single-precision 32-bit float (type 1).
    Float(f32),
    /// Null literal (type 2).
    Null,
    /// Undefined literal (type 3).
    Undefined,
    /// Local register index (type 4).
    Register(u8),
    /// Boolean flag (type 5).
    Boolean(bool),
    /// Double-precision 64-bit float (type 6).
    Double(f64),
    /// 32-bit signed integer (type 7).
    Integer(i32),
    /// 8-bit index into constant pool (type 8).
    Constant8(u8),
    /// 16-bit index into constant pool (type 9).
    Constant16(u16),
}
