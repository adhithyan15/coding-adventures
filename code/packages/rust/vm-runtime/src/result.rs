//! `VmResult` and `VmResultTag` — the C-ABI result type for vm-runtime.
//!
//! Every vm-runtime entry point (particularly `vm_execute` and
//! `vm_call_builtin`) returns a `VmResult` — a tagged union that can carry
//! any value the interpreter might produce.
//!
//! The design mirrors the JVM's operand stack (where `long` and `double`
//! occupy two slots but every other type fits in one) with the addition of
//! a `Trap` discriminant for unrecoverable errors.
//!
//! # Memory layout
//!
//! ```text
//! VmResult (16 bytes on 64-bit hosts):
//!   [0..1]  tag     : VmResultTag  (u8, padded)
//!   [2..7]  padding : [u8; 7]
//!   [8..15] payload : u64          (covers all non-trap variants)
//! ```
//!
//! The payload encoding by tag:
//!
//! | Tag | Payload interpretation |
//! |-----|------------------------|
//! | `Void` | payload ignored |
//! | `U8`…`U64` | zero-extended unsigned integer |
//! | `Bool` | 0 = false, 1 = true |
//! | `Str` | index into the vm-runtime string intern pool |
//! | `Ref` | opaque heap pointer (for LANG16 GC) |
//! | `Trap` | lower 16 bits = trap code |
//!
//! The Rust `VmResult` struct mirrors this layout in safe code, storing the
//! payload as a `u64` regardless of the logical type.  Constructors
//! (`VmResult::from_*`) and extractors (`VmResult::as_*`) do the encoding /
//! decoding.

use vm_core::value::Value;

/// Discriminant tag for a `VmResult` value.
///
/// Variants are ordered so that numeric comparison works as a rough "how
/// wide is this integer" test for the integer variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum VmResultTag {
    /// No return value (void function).
    Void = 0,
    /// 8-bit unsigned integer.
    U8 = 1,
    /// 16-bit unsigned integer.
    U16 = 2,
    /// 32-bit unsigned integer.
    U32 = 3,
    /// 64-bit unsigned integer.
    U64 = 4,
    /// Boolean.
    Bool = 5,
    /// String (interned; payload is the intern-pool index).
    Str = 6,
    /// Opaque heap reference (LANG16 GC pointer).
    Ref = 7,
    /// Execution trapped (unrecoverable error).
    Trap = 0xFF,
}

impl VmResultTag {
    /// Convert a raw `u8` discriminant to a `VmResultTag`.
    ///
    /// Returns `None` for unrecognised values.
    ///
    /// ```
    /// use vm_runtime::result::VmResultTag;
    /// assert_eq!(VmResultTag::from_u8(5), Some(VmResultTag::Bool));
    /// assert_eq!(VmResultTag::from_u8(0xAB), None);
    /// ```
    pub fn from_u8(raw: u8) -> Option<Self> {
        match raw {
            0 => Some(VmResultTag::Void),
            1 => Some(VmResultTag::U8),
            2 => Some(VmResultTag::U16),
            3 => Some(VmResultTag::U32),
            4 => Some(VmResultTag::U64),
            5 => Some(VmResultTag::Bool),
            6 => Some(VmResultTag::Str),
            7 => Some(VmResultTag::Ref),
            0xFF => Some(VmResultTag::Trap),
            _ => None,
        }
    }
}

impl std::fmt::Display for VmResultTag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VmResultTag::Void => write!(f, "void"),
            VmResultTag::U8 => write!(f, "u8"),
            VmResultTag::U16 => write!(f, "u16"),
            VmResultTag::U32 => write!(f, "u32"),
            VmResultTag::U64 => write!(f, "u64"),
            VmResultTag::Bool => write!(f, "bool"),
            VmResultTag::Str => write!(f, "str"),
            VmResultTag::Ref => write!(f, "ref"),
            VmResultTag::Trap => write!(f, "trap"),
        }
    }
}

// ---------------------------------------------------------------------------
// VmResult
// ---------------------------------------------------------------------------

/// A tagged result value from a vm-runtime entry point.
///
/// # Construction
///
/// ```
/// use vm_runtime::result::{VmResult, VmResultTag};
///
/// let r = VmResult::void();
/// assert_eq!(r.tag, VmResultTag::Void);
///
/// let n = VmResult::from_u8(42);
/// assert_eq!(n.tag, VmResultTag::U8);
/// assert_eq!(n.as_u64(), Some(42));
///
/// let t = VmResult::trap(1);
/// assert!(t.is_trap());
/// assert_eq!(t.trap_code(), Some(1));
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct VmResult {
    /// Discriminant: what kind of value does `payload` hold?
    pub tag: VmResultTag,
    /// The value, encoded as a `u64` (see module-level layout table).
    pub payload: u64,
}

impl VmResult {
    // ── Constructors ────────────────────────────────────────────────────

    /// A void result (no value).
    pub fn void() -> Self {
        VmResult { tag: VmResultTag::Void, payload: 0 }
    }

    /// An 8-bit unsigned integer result.
    pub fn from_u8(v: u8) -> Self {
        VmResult { tag: VmResultTag::U8, payload: v as u64 }
    }

    /// A 64-bit unsigned integer result.
    pub fn from_u64(v: u64) -> Self {
        VmResult { tag: VmResultTag::U64, payload: v }
    }

    /// A signed 64-bit integer result (encoded as bit-cast to u64).
    pub fn from_i64(v: i64) -> Self {
        VmResult { tag: VmResultTag::U64, payload: v as u64 }
    }

    /// A boolean result.
    pub fn from_bool(b: bool) -> Self {
        VmResult { tag: VmResultTag::Bool, payload: if b { 1 } else { 0 } }
    }

    /// An opaque heap reference result (LANG16).
    pub fn from_ref(ptr: u64) -> Self {
        VmResult { tag: VmResultTag::Ref, payload: ptr }
    }

    /// An execution-trap result.
    ///
    /// `code` should be a caller-defined 16-bit trap code.
    pub fn trap(code: u16) -> Self {
        VmResult { tag: VmResultTag::Trap, payload: code as u64 }
    }

    // ── Conversion from vm-core Value ────────────────────────────────────

    /// Convert a [`vm_core::value::Value`] into a `VmResult`.
    ///
    /// ```
    /// use vm_core::value::Value;
    /// use vm_runtime::result::{VmResult, VmResultTag};
    ///
    /// let r = VmResult::from_value(Value::Int(99));
    /// assert_eq!(r.tag, VmResultTag::U64);
    /// assert_eq!(r.payload, 99);
    /// ```
    pub fn from_value(v: Value) -> Self {
        match v {
            Value::Int(n) => VmResult::from_i64(n),
            Value::Float(f) => VmResult { tag: VmResultTag::U64, payload: f.to_bits() },
            Value::Bool(b) => VmResult::from_bool(b),
            Value::Str(_) => VmResult { tag: VmResultTag::Str, payload: 0 }, // intern idx TBD
            Value::Null => VmResult::void(),
        }
    }

    // ── Predicates ───────────────────────────────────────────────────────

    /// Return `true` if this result represents an execution trap.
    pub fn is_trap(&self) -> bool {
        self.tag == VmResultTag::Trap
    }

    /// Return `true` if this result is a void value.
    pub fn is_void(&self) -> bool {
        self.tag == VmResultTag::Void
    }

    // ── Extractors ───────────────────────────────────────────────────────

    /// Extract the payload as a `u64` (valid for all non-trap variants).
    ///
    /// Returns `None` for `Trap` results.
    pub fn as_u64(&self) -> Option<u64> {
        if self.is_trap() { None } else { Some(self.payload) }
    }

    /// Extract as a signed `i64` (valid when tag is `U64`).
    pub fn as_i64(&self) -> Option<i64> {
        if self.tag == VmResultTag::U64 {
            Some(self.payload as i64)
        } else {
            None
        }
    }

    /// Extract as `bool` (valid when tag is `Bool`).
    pub fn as_bool(&self) -> Option<bool> {
        if self.tag == VmResultTag::Bool {
            Some(self.payload != 0)
        } else {
            None
        }
    }

    /// Extract the trap code (valid when tag is `Trap`).
    pub fn trap_code(&self) -> Option<u16> {
        if self.is_trap() {
            Some(self.payload as u16)
        } else {
            None
        }
    }
}

impl std::fmt::Display for VmResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_trap() {
            write!(f, "trap({})", self.payload)
        } else {
            write!(f, "{}({})", self.tag, self.payload)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn void_result() {
        let r = VmResult::void();
        assert_eq!(r.tag, VmResultTag::Void);
        assert!(r.is_void());
        assert!(!r.is_trap());
        assert_eq!(r.as_u64(), Some(0));
    }

    #[test]
    fn u8_result() {
        let r = VmResult::from_u8(255);
        assert_eq!(r.tag, VmResultTag::U8);
        assert_eq!(r.as_u64(), Some(255));
    }

    #[test]
    fn bool_true() {
        let r = VmResult::from_bool(true);
        assert_eq!(r.tag, VmResultTag::Bool);
        assert_eq!(r.as_bool(), Some(true));
    }

    #[test]
    fn bool_false() {
        let r = VmResult::from_bool(false);
        assert_eq!(r.as_bool(), Some(false));
    }

    #[test]
    fn trap_result() {
        let r = VmResult::trap(42);
        assert!(r.is_trap());
        assert_eq!(r.trap_code(), Some(42));
        assert_eq!(r.as_u64(), None);
    }

    #[test]
    fn i64_round_trip() {
        let r = VmResult::from_i64(-1);
        assert_eq!(r.as_i64(), Some(-1));
    }

    #[test]
    fn from_value_int() {
        let r = VmResult::from_value(Value::Int(7));
        assert_eq!(r.payload, 7);
    }

    #[test]
    fn from_value_bool() {
        let r = VmResult::from_value(Value::Bool(true));
        assert_eq!(r.tag, VmResultTag::Bool);
        assert_eq!(r.as_bool(), Some(true));
    }

    #[test]
    fn from_value_null_is_void() {
        let r = VmResult::from_value(Value::Null);
        assert!(r.is_void());
    }

    #[test]
    fn tag_from_u8_roundtrip() {
        for b in [0u8, 1, 2, 3, 4, 5, 6, 7, 0xFF] {
            assert!(VmResultTag::from_u8(b).is_some(), "failed for {}", b);
        }
        assert_eq!(VmResultTag::from_u8(0xAB), None);
    }

    #[test]
    fn display_result() {
        let r = VmResult::from_u8(3);
        let s = r.to_string();
        assert!(s.contains("u8"));
        assert!(s.contains("3"));
    }

    #[test]
    fn display_trap() {
        let r = VmResult::trap(99);
        assert!(r.to_string().contains("trap"));
    }
}
