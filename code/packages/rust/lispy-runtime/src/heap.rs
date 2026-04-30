//! # Heap object types: `ConsCell` and `Closure`.
//!
//! Both types are `#[repr(C)]` and start with the LANG20 16-byte
//! [`ObjectHeader`] (per LANG20 §"Cross-language value
//! representation").  This is the **non-negotiable** ABI commitment
//! that lets one GC trace every language's heap without
//! per-language plumbing.
//!
//! ## Why only Cons + Closure for now?
//!
//! `LispyValue` carries integers, booleans, nil, and symbols as
//! immediates (no allocation).  That leaves cons cells and closures
//! as the two structures Lispy needs to allocate.  Future heap
//! objects (vectors, strings, hash tables, big-integers) get added
//! in subsequent PRs as the language surface grows.
//!
//! ## Layout
//!
//! ```text
//! ConsCell (32 bytes):
//!   ┌──────────────────────────────────┐
//!   │  ObjectHeader (16 bytes)         │
//!   ├──────────────────────────────────┤
//!   │  car: LispyValue (8 bytes)       │
//!   ├──────────────────────────────────┤
//!   │  cdr: LispyValue (8 bytes)       │
//!   └──────────────────────────────────┘
//!
//! Closure (variable; ≥ 24 + 24-byte Vec):
//!   ┌──────────────────────────────────┐
//!   │  ObjectHeader (16 bytes)         │
//!   ├──────────────────────────────────┤
//!   │  fn_name: SymbolId (4 bytes)     │
//!   │  _pad: u32 (4 bytes for align)   │
//!   ├──────────────────────────────────┤
//!   │  captures: Vec<LispyValue> (24B) │
//!   └──────────────────────────────────┘
//! ```
//!
//! ## Allocator
//!
//! PR 2 ships `Box::leak`-based allocation.  This is intentionally
//! a "non-collecting GC" — every allocation lives forever.  When
//! LANG16 lands the real `gc-core` allocator, [`alloc_cons`] /
//! [`alloc_closure`] swap to the bump-pointer + mark-sweep path
//! without their callers noticing — the function signatures are
//! the runtime contract, not the storage strategy.
//!
//! ## Class IDs
//!
//! Each kind has a fixed [`u32`] class id stored in the header's
//! `class_or_kind`.  The collector dispatches trace via
//! `binding_for(header.class_or_kind).trace_object(...)` (LANG16),
//! so these ids are language-private — only `LispyBinding` ever
//! reads them.

use lang_runtime_core::{ObjectHeader, SymbolId};

use crate::value::LispyValue;

// ---------------------------------------------------------------------------
// Class ids (per-language, registered with the collector at startup)
// ---------------------------------------------------------------------------

/// Class id for [`ConsCell`].
pub const CLASS_CONS: u32 = 1;

/// Class id for [`Closure`].
pub const CLASS_CLOSURE: u32 = 2;

// ---------------------------------------------------------------------------
// ConsCell
// ---------------------------------------------------------------------------

/// A Lisp cons cell — the fundamental list-building primitive.
///
/// `car` is the first element; `cdr` is the rest.  In a proper
/// list, `cdr` chains through more cons cells until reaching `nil`;
/// in a dotted pair, `cdr` is any non-cons value.
///
/// The 16-byte LANG20 header makes this struct 32 bytes total
/// (header + 2× 8-byte values), which is naturally 8-byte aligned —
/// satisfying the [`LispyValue::from_heap`](crate::value::LispyValue::from_heap)
/// invariant.
#[repr(C)]
pub struct ConsCell {
    /// LANG20 uniform 16-byte header.  `class_or_kind == CLASS_CONS`.
    pub header: ObjectHeader,
    /// First element of the pair.
    pub car: LispyValue,
    /// Rest of the pair.  In a proper list this chains through more
    /// `ConsCell`s and terminates with `nil`.
    pub cdr: LispyValue,
}

// Manual Debug impl that skips the header — `ObjectHeader` doesn't
// derive Debug (it carries an `AtomicU32` field whose Debug isn't
// generally meaningful), and the bookkeeping isn't useful in test
// output anyway.  We surface the language-meaningful fields.
impl std::fmt::Debug for ConsCell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConsCell")
            .field("car", &self.car)
            .field("cdr", &self.cdr)
            .finish()
    }
}

// 32 bytes is the natural size: 16 + 8 + 8.  Asserted at compile
// time so any future field addition that would change layout
// breaks the build immediately.
const _: () = assert!(std::mem::size_of::<ConsCell>() == 32);
const _: () = assert!(std::mem::align_of::<ConsCell>() == 8);

impl ConsCell {
    /// Construct a fresh cons cell with the given `car` and `cdr`.
    /// Caller is responsible for boxing + leaking — use
    /// [`alloc_cons`] for the full allocation path.
    pub fn new(car: LispyValue, cdr: LispyValue) -> ConsCell {
        ConsCell {
            header: ObjectHeader::new(CLASS_CONS, std::mem::size_of::<ConsCell>() as u32),
            car,
            cdr,
        }
    }
}

// ---------------------------------------------------------------------------
// Closure
// ---------------------------------------------------------------------------

/// A Lispy closure — captured-environment + reference to a
/// top-level IIRFunction.
///
/// `fn_name` identifies the underlying function (the gensym'd
/// `__lambda_N` for anonymous lambdas, or the user name for
/// `(define (f ...) ...)`); `captures` holds the values that
/// were in scope at the closure's construction site.
///
/// When the closure is applied, the runtime prepends `captures` to
/// the user-supplied arguments and calls the underlying function —
/// per the apply-closure semantics in TW00.
#[repr(C)]
pub struct Closure {
    /// LANG20 uniform 16-byte header.  `class_or_kind == CLASS_CLOSURE`.
    pub header: ObjectHeader,
    /// Interned name of the underlying IIRFunction (or builtin name,
    /// when [`Self::flags`] bit 0 is set).
    pub fn_name: SymbolId,
    /// Bitfield.  Bit 0 (`CLOSURE_FLAG_BUILTIN`) marks the closure as
    /// wrapping a builtin rather than a user IIRFunction.  Higher
    /// bits reserved (planned: arity hint).
    pub flags: u32,
    /// Captured values, prepended to user args at apply time.
    /// Always empty for builtin closures.
    pub captures: Vec<LispyValue>,
}

/// [`Closure::flags`] bit 0: this closure wraps a builtin
/// (`make_builtin_closure`) rather than a user IIRFunction
/// (`make_closure`).
///
/// Distinguishes the two at apply time:
///
/// - User-fn closure → look up `fn_name` in the IIRModule's
///   functions table, dispatch with `captures ++ args`.
/// - Builtin closure → look up `fn_name` via
///   [`crate::LispyBinding::resolve_builtin`], dispatch with `args`
///   (captures must be empty).
pub const CLOSURE_FLAG_BUILTIN: u32 = 0b0001;

impl std::fmt::Debug for Closure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Closure")
            .field("fn_name", &self.fn_name)
            .field("captures", &self.captures)
            .finish()
    }
}

const _: () = assert!(std::mem::align_of::<Closure>() == 8);

impl Closure {
    /// Construct a fresh closure.  Caller is responsible for boxing +
    /// leaking — use [`alloc_closure`] for the full allocation path.
    pub fn new(fn_name: SymbolId, captures: Vec<LispyValue>) -> Closure {
        // The Vec accounts for itself; size_bytes records the size
        // of the Closure struct (the Vec heap is tracked separately
        // by the GC's secondary-allocation mechanism).
        Closure {
            header: ObjectHeader::new(CLASS_CLOSURE, std::mem::size_of::<Closure>() as u32),
            fn_name,
            flags: 0,
            captures,
        }
    }

    /// Construct a builtin-wrapping closure.  No captures by
    /// construction; sets the [`CLOSURE_FLAG_BUILTIN`] flag so apply
    /// time can dispatch through `resolve_builtin` instead of the
    /// user-fn lookup path.
    pub fn new_builtin(fn_name: SymbolId) -> Closure {
        Closure {
            header: ObjectHeader::new(CLASS_CLOSURE, std::mem::size_of::<Closure>() as u32),
            fn_name,
            flags: CLOSURE_FLAG_BUILTIN,
            captures: Vec::new(),
        }
    }

    /// Number of captured values.
    pub fn capture_count(&self) -> usize {
        self.captures.len()
    }

    /// Returns `true` if this closure wraps a builtin
    /// (created via `make_builtin_closure`).
    pub fn is_builtin(&self) -> bool {
        self.flags & CLOSURE_FLAG_BUILTIN != 0
    }
}

// ---------------------------------------------------------------------------
// Allocator (PR 2: Box::leak; PR 4+: real GC)
// ---------------------------------------------------------------------------

/// Allocate a [`ConsCell`] and return a tagged [`LispyValue`].
///
/// **PR 2 implementation:** `Box::leak`.  Every cons cell allocated
/// here lives forever — there is no collector yet.  Tests that need
/// to verify allocation behaviour without leaking real memory should
/// build cells via [`ConsCell::new`] and inspect them in-place
/// instead of going through this function.
///
/// **Future:** when LANG16's `gc-core` lands, this function calls
/// `gc_core::alloc(CLASS_CONS, size_of::<ConsCell>() as u32)` and
/// initialises the returned slot.  The signature stays the same;
/// only the body changes.
pub fn alloc_cons(car: LispyValue, cdr: LispyValue) -> LispyValue {
    let cell = Box::new(ConsCell::new(car, cdr));
    let ptr = Box::leak(cell) as *const ConsCell;
    // SAFETY: Box::leak'd ConsCell is 8-aligned (compile-time
    // const_assert) and lives forever (intentional PR 2 leak).
    unsafe { LispyValue::from_heap(ptr) }
}

/// Allocate a [`Closure`] and return a tagged [`LispyValue`].
///
/// Same PR 2 vs. future-PR contract as [`alloc_cons`].
pub fn alloc_closure(fn_name: SymbolId, captures: Vec<LispyValue>) -> LispyValue {
    let clos = Box::new(Closure::new(fn_name, captures));
    let ptr = Box::leak(clos) as *const Closure;
    // SAFETY: Box::leak'd Closure is 8-aligned (const_assert) and lives forever.
    unsafe { LispyValue::from_heap(ptr) }
}

/// Allocate a builtin-wrapping [`Closure`] (no captures, marked
/// with [`CLOSURE_FLAG_BUILTIN`]) and return a tagged [`LispyValue`].
///
/// Used by `make_builtin_closure` so that bare builtin references
/// like `(+) ` or `(cons)` can be passed as values into higher-order
/// positions.  At apply time the dispatcher detects the flag and
/// routes through `LispyBinding::resolve_builtin`.
pub fn alloc_builtin_closure(fn_name: SymbolId) -> LispyValue {
    let clos = Box::new(Closure::new_builtin(fn_name));
    let ptr = Box::leak(clos) as *const Closure;
    // SAFETY: Box::leak'd Closure is 8-aligned (const_assert) and lives forever.
    unsafe { LispyValue::from_heap(ptr) }
}

// ---------------------------------------------------------------------------
// Heap-value accessors
// ---------------------------------------------------------------------------

/// Read the `car` of a heap value, returning `None` if the value
/// isn't a cons cell.
///
/// # Safety
///
/// `value`'s heap-tag bits (`value.is_heap()`) must mean it
/// genuinely points at a live `ObjectHeader` produced by this
/// crate's allocator.  The function dereferences the heap pointer
/// to read the class id; an attacker-fabricated `LispyValue` with
/// the heap tag pattern but a bogus address would read garbage
/// memory.
///
/// In PR 2 (`Box::leak`'d allocations) every heap value produced
/// by this crate satisfies the live-pointer invariant
/// permanently.  Once a real GC lands, callers must hold the
/// value inside the GC-tracked root set.  Callers who construct
/// `LispyValue`s only via the safe constructors (`int`, `bool`,
/// `symbol`, `from_heap` of an `alloc_*` result) are safe.
pub unsafe fn car(value: LispyValue) -> Option<LispyValue> {
    let header_ptr: *const ObjectHeader = value.as_heap_ptr()?;
    // SAFETY: caller's invariant — see function-level safety note.
    unsafe {
        if (*header_ptr).class_or_kind != CLASS_CONS {
            return None;
        }
        let cell = header_ptr as *const ConsCell;
        Some((*cell).car)
    }
}

/// Read the `cdr` of a heap value.
///
/// # Safety
///
/// Same contract as [`car`].
pub unsafe fn cdr(value: LispyValue) -> Option<LispyValue> {
    let header_ptr: *const ObjectHeader = value.as_heap_ptr()?;
    unsafe {
        if (*header_ptr).class_or_kind != CLASS_CONS {
            return None;
        }
        let cell = header_ptr as *const ConsCell;
        Some((*cell).cdr)
    }
}

/// `true` iff this heap value is a cons cell.
///
/// # Safety
///
/// Same contract as [`car`].
pub unsafe fn is_cons(value: LispyValue) -> bool {
    if let Some(header_ptr) = value.as_heap_ptr::<ObjectHeader>() {
        unsafe { (*header_ptr).class_or_kind == CLASS_CONS }
    } else {
        false
    }
}

/// `true` iff this heap value is a closure.
///
/// # Safety
///
/// Same contract as [`car`].
pub unsafe fn is_closure(value: LispyValue) -> bool {
    if let Some(header_ptr) = value.as_heap_ptr::<ObjectHeader>() {
        unsafe { (*header_ptr).class_or_kind == CLASS_CLOSURE }
    } else {
        false
    }
}

/// Borrow a heap value as a [`Closure`].
///
/// # Safety
///
/// Same contract as [`car`].
pub unsafe fn as_closure(value: LispyValue) -> Option<&'static Closure> {
    let header_ptr: *const ObjectHeader = value.as_heap_ptr()?;
    unsafe {
        if (*header_ptr).class_or_kind != CLASS_CLOSURE {
            return None;
        }
        let clos = header_ptr as *const Closure;
        Some(&*clos)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cons_cell_is_32_bytes() {
        assert_eq!(std::mem::size_of::<ConsCell>(), 32);
    }

    #[test]
    fn cons_cell_starts_with_header() {
        let c = ConsCell::new(LispyValue::int(1), LispyValue::int(2));
        // Field offset of `header` is 0 — verified by reading the
        // first u32 and confirming it's the class id.
        let ptr = &c as *const ConsCell as *const u32;
        let class_id = unsafe { *ptr };
        assert_eq!(class_id, CLASS_CONS);
        assert_eq!(c.car, LispyValue::int(1));
        assert_eq!(c.cdr, LispyValue::int(2));
    }

    #[test]
    fn alloc_cons_returns_heap_tagged_value() {
        let v = alloc_cons(LispyValue::int(1), LispyValue::int(2));
        assert!(v.is_heap());
        // SAFETY: v came from alloc_cons in this test — live, valid.
        unsafe {
            assert!(is_cons(v));
            assert_eq!(car(v), Some(LispyValue::int(1)));
            assert_eq!(cdr(v), Some(LispyValue::int(2)));
        }
    }

    #[test]
    fn cons_chain_renders_proper_list_structure() {
        // Build (1 2 3) = (cons 1 (cons 2 (cons 3 nil)))
        let three = alloc_cons(LispyValue::int(3), LispyValue::NIL);
        let two = alloc_cons(LispyValue::int(2), three);
        let list = alloc_cons(LispyValue::int(1), two);

        // SAFETY: every value in this chain came from alloc_cons.
        unsafe {
            assert_eq!(car(list), Some(LispyValue::int(1)));
            let after_first = cdr(list).unwrap();
            assert_eq!(car(after_first), Some(LispyValue::int(2)));
            let after_second = cdr(after_first).unwrap();
            assert_eq!(car(after_second), Some(LispyValue::int(3)));
            assert_eq!(cdr(after_second), Some(LispyValue::NIL));
        }
    }

    #[test]
    fn car_cdr_return_none_for_non_cons() {
        // Immediates aren't cons; the heap-tag check fails first.
        // SAFETY: passing immediates is always sound (no deref).
        unsafe {
            assert_eq!(car(LispyValue::int(42)), None);
            assert_eq!(cdr(LispyValue::int(42)), None);
            assert_eq!(car(LispyValue::NIL), None);
            assert_eq!(car(LispyValue::TRUE), None);
            assert_eq!(car(LispyValue::symbol(SymbolId(1))), None);
        }
    }

    #[test]
    fn closure_is_8_aligned() {
        assert_eq!(std::mem::align_of::<Closure>(), 8);
    }

    #[test]
    fn closure_records_fn_and_captures() {
        let c = Closure::new(SymbolId(7), vec![LispyValue::int(1), LispyValue::int(2)]);
        assert_eq!(c.fn_name, SymbolId(7));
        assert_eq!(c.capture_count(), 2);
        assert_eq!(c.captures[0], LispyValue::int(1));
    }

    #[test]
    fn alloc_closure_returns_heap_tagged_value() {
        let v = alloc_closure(SymbolId(3), vec![LispyValue::int(99)]);
        assert!(v.is_heap());
        // SAFETY: v came from alloc_closure.
        unsafe {
            assert!(is_closure(v));
            let clos = as_closure(v).expect("closure round-trip");
            assert_eq!(clos.fn_name, SymbolId(3));
            assert_eq!(clos.captures, vec![LispyValue::int(99)]);
        }
    }

    #[test]
    fn cons_and_closure_have_distinct_class_ids() {
        // Critical for the `is_*` predicates and the GC's trace
        // dispatch.  Catches accidental id collisions.
        assert_ne!(CLASS_CONS, CLASS_CLOSURE);
    }

    #[test]
    fn is_cons_and_is_closure_are_mutually_exclusive() {
        let cons_v = alloc_cons(LispyValue::NIL, LispyValue::NIL);
        let clos_v = alloc_closure(SymbolId(0), vec![]);
        // SAFETY: both values came from this crate's allocators.
        unsafe {
            assert!(is_cons(cons_v));
            assert!(!is_closure(cons_v));
            assert!(!is_cons(clos_v));
            assert!(is_closure(clos_v));
        }
    }
}
