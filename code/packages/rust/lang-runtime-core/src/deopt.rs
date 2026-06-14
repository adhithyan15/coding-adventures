//! # Deopt protocol: frame descriptors and native locations.
//!
//! When a JIT- or AOT-emitted guard fails (or any other
//! deoptimisation trigger fires — see LANG20 §"Deopt protocol"),
//! the runtime needs to *materialise* the speculative state back
//! into an interpreter frame.  Two pieces of data make that
//! possible:
//!
//! 1. **The frame descriptor** ([`FrameDescriptor`]) — published
//!    by the JIT/AOT compiler at every deopt anchor.  Maps each
//!    live IIR variable to *where* it lives in the native frame
//!    (register / stack slot / inlined constant) and *how* it's
//!    encoded ([`crate::value::BoxedReprToken`]).
//! 2. **The materialise callback** — provided by each
//!    [`crate::LangBinding`] via `materialize_value`.  Reads the
//!    raw `u64` from the native location and produces the
//!    language's `Value` representation.
//!
//! `rt_deopt` (LANG20 §"C ABI extensions") walks the descriptor,
//! calls the binding for each entry, builds a fresh `VMFrame`,
//! and resumes the interpreter at `ir_index`.
//!
//! ## Inlined-call deopt
//!
//! When the JIT inlines a callee into the caller's native code,
//! one deopt point may need to materialise *multiple* interpreter
//! frames (one per inlined level).  [`InlinedDeoptDescriptor`]
//! holds an ordered list of `FrameDescriptor`s in caller→callee
//! order; the runtime materialises bottom-up before resuming.

use crate::value::BoxedReprToken;

// ---------------------------------------------------------------------------
// NativeLocation
// ---------------------------------------------------------------------------

/// Where in the native frame a single live value resides at a
/// deopt anchor.
///
/// Used inside [`RegisterEntry`] to describe each IIR variable's
/// physical location.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeLocation {
    /// Hardware register, indexed by the codegen's own
    /// register-allocation scheme.  The runtime is responsible for
    /// translating the index to an actual ABI register on the
    /// current platform (it's the codegen that chose the index, so
    /// the deopt path can mirror the choice).
    Register(u8),

    /// Byte offset from the frame pointer.  Negative offsets are
    /// below FP (typical for spill slots in System V x86_64);
    /// positive offsets are above (typical for incoming args).
    StackSlot(i32),

    /// Compile-time-known constant the codegen inlined; the deopt
    /// path uses this value directly without touching the native
    /// frame.  Common for trivially-constant-folded values.
    Constant(u64),
}

// ---------------------------------------------------------------------------
// RegisterEntry
// ---------------------------------------------------------------------------

/// One IIR variable's deopt metadata.
///
/// Carries enough information for the runtime to read the value
/// from the native frame, decode it via the binding's
/// `materialize_value`, and write the result back into the
/// interpreter's register file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegisterEntry {
    /// IIR variable name (matches an `IIRInstr::dest` somewhere
    /// earlier in the function).  The interpreter uses this name
    /// as the key in its register file.
    pub ir_name: String,

    /// Where the value lives in the native frame.
    pub location: NativeLocation,

    /// How the value is encoded — boxed ref vs. unboxed scalar
    /// vs. derived pointer.  See [`BoxedReprToken`] for the
    /// variant table.
    pub repr: BoxedReprToken,

    /// The type tag the JIT speculated at this anchor.  Optional
    /// hint the binding's `materialize_value` can validate against
    /// (defensive deopt: if the speculation was wrong the
    /// materialised value will mismatch the recorded tag).
    pub speculated_type_tag: Option<u32>,
}

// ---------------------------------------------------------------------------
// FrameDescriptor
// ---------------------------------------------------------------------------

/// Per-deopt-anchor metadata describing how to reconstruct an
/// interpreter frame from the JIT/AOT native frame.
///
/// Stored in a side table indexed by deopt anchor id; the JIT
/// publishes one descriptor per emitted guard.  AOT publishes a
/// section in the `.aot` file (LANG04 §"snapshot format" / LANG20
/// §"Deopt protocol").
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameDescriptor {
    /// IIR instruction index to resume the interpreter at.  For a
    /// failed type guard this is the instruction that owned the
    /// guard; for soft deopts it's the next safepoint after the
    /// triggering condition.
    pub ir_index: u32,

    /// One entry per IIR variable that was live at this point.
    /// Order is not significant — the interpreter resolves by
    /// `ir_name`.
    pub registers: Vec<RegisterEntry>,
}

impl FrameDescriptor {
    /// Construct a descriptor with no registers — useful as a
    /// placeholder before the JIT codegen fills in entries.
    pub fn new(ir_index: u32) -> Self {
        FrameDescriptor { ir_index, registers: Vec::new() }
    }

    /// Append a register entry; chains for fluent construction.
    pub fn with_register(mut self, entry: RegisterEntry) -> Self {
        self.registers.push(entry);
        self
    }
}

// ---------------------------------------------------------------------------
// InlinedDeoptDescriptor
// ---------------------------------------------------------------------------

/// Deopt metadata for an inlined-call site, containing one
/// [`FrameDescriptor`] per inlined level.
///
/// When a JIT-inlined callee deoptimises, the runtime must
/// materialise *every* inlined frame, not just the leaf — otherwise
/// the interpreter's call stack is missing the intermediate
/// frames.  Frames are stored caller→callee order so the runtime
/// can iterate in stack-build order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InlinedDeoptDescriptor {
    /// Inlined frames in caller→callee order.  Materialise this
    /// list as a stack: first frame becomes the deepest active
    /// frame after deopt; last frame becomes the shallowest
    /// (currently-executing) frame.
    pub frames: Vec<FrameDescriptor>,
}

impl InlinedDeoptDescriptor {
    /// Construct a descriptor for a non-inlined deopt point — a
    /// single frame.  Equivalent to wrapping a [`FrameDescriptor`]
    /// in a one-element list.
    pub fn single(frame: FrameDescriptor) -> Self {
        InlinedDeoptDescriptor { frames: vec![frame] }
    }

    /// True iff this descriptor describes inlined frames (more
    /// than one level).
    pub fn is_inlined(&self) -> bool {
        self.frames.len() > 1
    }
}

// ---------------------------------------------------------------------------
// DeoptAnchor
// ---------------------------------------------------------------------------

/// Compile-time-assigned id for a deopt anchor — the index used by
/// `rt_deopt(anchor_id, frame_ptr)` (LANG20 §"C ABI extensions")
/// to look up the frame descriptor.
///
/// The codegen assigns a fresh `DeoptAnchor` per emitted guard and
/// pairs it with a [`FrameDescriptor`] in the side table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(transparent)]
pub struct DeoptAnchor(pub u32);

impl DeoptAnchor {
    /// Return the underlying `u32` for ABI passing.
    pub const fn as_u32(self) -> u32 {
        self.0
    }
}

impl std::fmt::Display for DeoptAnchor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "anchor#{}", self.0)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str, loc: NativeLocation, repr: BoxedReprToken) -> RegisterEntry {
        RegisterEntry {
            ir_name: name.to_string(),
            location: loc,
            repr,
            speculated_type_tag: None,
        }
    }

    #[test]
    fn fresh_descriptor_has_no_registers() {
        let d = FrameDescriptor::new(7);
        assert_eq!(d.ir_index, 7);
        assert!(d.registers.is_empty());
    }

    #[test]
    fn with_register_chains() {
        let d = FrameDescriptor::new(3)
            .with_register(entry("a", NativeLocation::Register(0), BoxedReprToken::I64Unboxed))
            .with_register(entry("b", NativeLocation::StackSlot(-8), BoxedReprToken::BoxedRef));
        assert_eq!(d.registers.len(), 2);
        assert_eq!(d.registers[0].ir_name, "a");
        assert_eq!(d.registers[1].ir_name, "b");
    }

    #[test]
    fn native_location_register_carries_index() {
        let l = NativeLocation::Register(15);
        match l {
            NativeLocation::Register(n) => assert_eq!(n, 15),
            _ => panic!(),
        }
    }

    #[test]
    fn native_location_stack_slot_supports_negative_offset() {
        // System V x86_64 uses negative offsets below FP for
        // local variables; this must round-trip cleanly.
        let l = NativeLocation::StackSlot(-32);
        match l {
            NativeLocation::StackSlot(n) => assert_eq!(n, -32),
            _ => panic!(),
        }
    }

    #[test]
    fn native_location_constant_carries_u64() {
        let l = NativeLocation::Constant(0xCAFE_BABE_DEAD_BEEF);
        match l {
            NativeLocation::Constant(n) => assert_eq!(n, 0xCAFE_BABE_DEAD_BEEF),
            _ => panic!(),
        }
    }

    #[test]
    fn inlined_single_is_not_inlined() {
        let d = InlinedDeoptDescriptor::single(FrameDescriptor::new(0));
        assert_eq!(d.frames.len(), 1);
        assert!(!d.is_inlined());
    }

    #[test]
    fn inlined_with_multiple_frames_is_inlined() {
        let d = InlinedDeoptDescriptor {
            frames: vec![FrameDescriptor::new(0), FrameDescriptor::new(5)],
        };
        assert!(d.is_inlined());
    }

    #[test]
    fn deopt_anchor_display_includes_id() {
        assert_eq!(format!("{}", DeoptAnchor(42)), "anchor#42");
    }

    #[test]
    fn register_entry_round_trips_speculated_type_tag() {
        let mut e = entry("x", NativeLocation::Register(2), BoxedReprToken::I64Unboxed);
        e.speculated_type_tag = Some(7);
        assert_eq!(e.speculated_type_tag, Some(7));
    }

    #[test]
    fn register_entry_with_derived_ptr_repr() {
        let e = entry(
            "cdr",
            NativeLocation::Register(3),
            BoxedReprToken::DerivedPtr { base_register: 5, offset: 8 },
        );
        match e.repr {
            BoxedReprToken::DerivedPtr { base_register, offset } => {
                assert_eq!(base_register, 5);
                assert_eq!(offset, 8);
            }
            _ => panic!(),
        }
    }
}
