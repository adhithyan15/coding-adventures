//! # `LispyBinding` — concrete `LangBinding` for Lisp/Scheme/Twig/Clojure.
//!
//! First implementation of the [`LangBinding`] trait shipped by
//! LANG20 PR 1.  Twig consumes this binding directly (see PR 3);
//! Lisp / Scheme / Clojure frontends consume the same binding once
//! their lexer / parser / IIR-compiler shims land.
//!
//! ## What's wired
//!
//! | Trait method | Behaviour |
//! |--------------|-----------|
//! | `type_tag` | low 3 tag bits of [`LispyValue`] (or class id for heap) |
//! | `class_of` | one of [`LispyClass`] (Int / Nil / Bool / Symbol / Cons / Closure) |
//! | `is_truthy` | Scheme: only `#f` and `nil` are false |
//! | `equal` | bitwise-equal for immediates; recursive structural for cons cells |
//! | `identical` | bitwise-equal of the value words (`eq?` semantics) |
//! | `hash` | the value bits, suitable for HashMap |
//! | `trace_object` / `trace_value` | walks the cons / closure payloads |
//! | `apply_callable` | unwraps a closure handle and dispatches to `DispatchCx::call_iir_function` (PR 4 wires the actual call) |
//! | `send_message` / `load_property` / `store_property` | error: Lispy doesn't have method dispatch |
//! | `resolve_builtin` | name → fn pointer, see [`crate::builtins`] |
//! | `materialize_value` / `box_value` | trivial because every Lispy value is already a 64-bit word |
//!
//! ## Class IDs and IC entries
//!
//! [`LispyClass`] is the binding's per-language class identifier;
//! it has a `to_class_id` helper that converts to LANG20's
//! universal `u32` `ClassId` for IC keying.  The IC entry shape
//! ([`LispyICEntry`]) carries `(type_tag, target_ptr)` — Lispy
//! doesn't have hidden classes, so the tag is enough to drive
//! type-keyed dispatch.

use std::hash::{Hash, Hasher};

use lang_runtime_core::{
    BoxedReprToken, BuiltinFn, ClassId, DispatchCx, InlineCache, LangBinding, ObjectHeader,
    RuntimeError, SymbolId, ValueVisitor,
};

use crate::builtins;
use crate::heap::{self, CLASS_CLOSURE, CLASS_CONS};
use crate::value::LispyValue;

// ---------------------------------------------------------------------------
// LispyClass — per-language opaque class identity
// ---------------------------------------------------------------------------

/// Per-language class identifier returned by
/// [`LangBinding::class_of`].  Carries language-meaningful kind
/// information (richer than the `u32` `ClassId` used for IC
/// keying — `class_of` returns this; bindings convert to
/// `ClassId` via [`LispyClass::to_class_id`] before recording in
/// caches).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LispyClass {
    /// An immediate integer.
    Int,
    /// The `nil` singleton.
    Nil,
    /// `#t` or `#f`.
    Bool,
    /// An interned symbol.
    Symbol,
    /// A heap-allocated cons cell.
    Cons,
    /// A heap-allocated closure.
    Closure,
}

impl LispyClass {
    /// Convert to LANG20's universal [`ClassId`] (a `u32`) for IC
    /// keying.  The mapping is stable: Int=10, Nil=11, … so two
    /// `LispyClass::Int` calls always produce the same id.
    pub const fn to_class_id(self) -> ClassId {
        ClassId(match self {
            LispyClass::Int => 10,
            LispyClass::Nil => 11,
            LispyClass::Bool => 12,
            LispyClass::Symbol => 13,
            LispyClass::Cons => CLASS_CONS,
            LispyClass::Closure => CLASS_CLOSURE,
        })
    }
}

// ---------------------------------------------------------------------------
// LispyICEntry — IC entry shape for type-keyed dispatch
// ---------------------------------------------------------------------------

/// Lispy IC entry: keyed on the value's `type_tag`, stores a
/// resolved target pointer.
///
/// Lispy doesn't have method dispatch in the traditional OO sense —
/// the `send` opcode (LANG20 §"IIR additions") is a no-op for
/// pure Lispy frontends.  This IC entry shape exists so the trait
/// constraint `ICEntry: Copy + 'static` is satisfied; future
/// dispatch tactics (e.g. type-tag-keyed arithmetic specialisation)
/// can populate it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LispyICEntry {
    /// The type tag this entry was warmed for.
    pub tag: u32,
    /// Resolved target — opaque pointer the JIT/AOT codegen
    /// interprets per its own conventions.
    pub target: usize,
}

// ---------------------------------------------------------------------------
// LispyBinding — the unit struct that implements LangBinding
// ---------------------------------------------------------------------------

/// The binding for Lisp / Scheme / Twig / Clojure frontends.
///
/// Stateless — all per-process state (intern table, allocator)
/// lives in dedicated modules.  This struct is purely a type-level
/// hook to satisfy the `LangBinding` trait surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LispyBinding;

impl LangBinding for LispyBinding {
    // ── Associated types ─────────────────────────────────────────────

    type Value = LispyValue;
    type ClassRef = LispyClass;
    type ICEntry = LispyICEntry;

    const LANGUAGE_NAME: &'static str = "lispy";

    // ── Type & identity ──────────────────────────────────────────────

    fn type_tag(value: LispyValue) -> u32 {
        // For immediates, the low 3 tag bits *are* the type tag.
        // For heap values, the class_or_kind from the header gives
        // a richer tag (Cons vs Closure) than the bare `0b111`
        // would.  This makes `IIRInstr::observed_type` distinguish
        // cons from closure without re-dereferencing.
        if let Some(header_ptr) = value.as_heap_ptr::<ObjectHeader>() {
            // SAFETY: heap pointers from this crate's allocator are
            // always live (PR 2 leaks); the header read is sound.
            unsafe { (*header_ptr).class_or_kind }
        } else {
            value.tag() as u32
        }
    }

    fn class_of(value: LispyValue) -> Option<LispyClass> {
        if value.is_int() { return Some(LispyClass::Int); }
        if value.is_nil() { return Some(LispyClass::Nil); }
        if value.is_bool() { return Some(LispyClass::Bool); }
        if value.is_symbol() { return Some(LispyClass::Symbol); }
        if unsafe { heap::is_cons(value) } { return Some(LispyClass::Cons); }
        if unsafe { heap::is_closure(value) } { return Some(LispyClass::Closure); }
        None
    }

    fn is_truthy(value: LispyValue) -> bool {
        // Scheme rule: only #f and nil are false.
        value.is_truthy()
    }

    fn equal(a: LispyValue, b: LispyValue) -> bool {
        // Structural equality.  Immediates compare by bits; cons
        // cells recurse on car / cdr.  Closures compare by identity
        // (two closures are `equal?` iff `eq?`) — that matches the
        // Scheme convention for opaque values.
        if a.bits() == b.bits() {
            return true;
        }
        // SAFETY: same precondition as the rest of the binding —
        // values seen here come from the runtime's value space.
        if unsafe { heap::is_cons(a) && heap::is_cons(b) } {
            // SAFETY: is_cons returned true so the heap pointer is valid.
            unsafe {
                return Self::equal(heap::car(a).unwrap(), heap::car(b).unwrap())
                    && Self::equal(heap::cdr(a).unwrap(), heap::cdr(b).unwrap());
            }
        }
        false
    }

    fn identical(a: LispyValue, b: LispyValue) -> bool {
        // `eq?` — bitwise equality of the value words.  Sound for
        // LispyValue because it's exactly 8 bytes (asserted at
        // compile time in value.rs).
        a.bits() == b.bits()
    }

    fn hash(value: LispyValue) -> u64 {
        // Use the default hasher on the bits — gives reasonable
        // distribution for hash maps.  For cons cells this hashes
        // identity, not structure; structural hashing would require
        // recursive walk which can blow the stack on long lists.
        let mut h = std::collections::hash_map::DefaultHasher::new();
        value.bits().hash(&mut h);
        h.finish()
    }

    // ── Heap interaction ─────────────────────────────────────────────

    unsafe fn trace_object(obj_header: *const ObjectHeader, visitor: &mut dyn ValueVisitor) {
        // Dispatch on the header's class id.  PR 2 only has Cons +
        // Closure; future heap kinds (vector, hash, big-int) get a
        // new arm here.
        let class = unsafe { (*obj_header).class_or_kind };
        match class {
            CLASS_CONS => {
                let cell = obj_header as *const heap::ConsCell;
                let car_v = unsafe { (*cell).car };
                let cdr_v = unsafe { (*cell).cdr };
                visitor.visit_value(car_v.bits());
                visitor.visit_value(cdr_v.bits());
            }
            CLASS_CLOSURE => {
                let clos = obj_header as *const heap::Closure;
                // SAFETY: Closure is #[repr(C)] and live (caller invariant).
                for cap in unsafe { (*clos).captures.iter() } {
                    visitor.visit_value(cap.bits());
                }
            }
            _ => {
                // Unknown class — defensively do nothing rather
                // than panic.  In a future debug build a sanity
                // check could fire here.
            }
        }
    }

    fn trace_value(value: LispyValue, visitor: &mut dyn ValueVisitor) {
        // Tagged immediates carry no references; only heap values
        // need recursion.  The collector handles the recursion
        // itself by enqueuing the visited word and walking from
        // there — `trace_value` here just yields the heap pointer.
        if value.is_heap() {
            visitor.visit_value(value.bits());
        }
    }

    // ── Dispatch ─────────────────────────────────────────────────────

    fn apply_callable(
        callable: LispyValue,
        _args: &[LispyValue],
        _cx: &mut DispatchCx<'_, Self>,
    ) -> Result<LispyValue, RuntimeError> {
        // PR 2: validate the value is a closure but cannot actually
        // dispatch — DispatchCx has no methods until PR 4 wires
        // vm-core into it.  We return a clear error so callers
        // know this path isn't live yet rather than silently
        // succeeding.
        //
        // SAFETY: `callable` is a `LispyValue` constructed via the
        // safe constructors (it's a parameter of trait method —
        // the runtime only ever passes valid LispyValues here).
        if !unsafe { heap::is_closure(callable) } {
            return Err(RuntimeError::NotCallable);
        }
        Err(RuntimeError::Custom(
            "apply_callable: closure dispatch lands in PR 4 (vm-core wiring)".into(),
        ))
    }

    fn send_message(
        _receiver: LispyValue,
        selector: SymbolId,
        _args: &[LispyValue],
        _ic: &mut InlineCache<LispyICEntry>,
        _cx: &mut DispatchCx<'_, Self>,
    ) -> Result<LispyValue, RuntimeError> {
        // Lispy has no method dispatch — `send` is never emitted
        // by Lispy frontends.  We return NoSuchMethod for any call
        // to keep the error model consistent with Ruby/JS bindings
        // that *do* implement send.
        Err(RuntimeError::NoSuchMethod { selector })
    }

    fn load_property(
        _obj: LispyValue,
        key: SymbolId,
        _ic: &mut InlineCache<LispyICEntry>,
    ) -> Result<LispyValue, RuntimeError> {
        Err(RuntimeError::NoSuchProperty { key })
    }

    fn store_property(
        _obj: LispyValue,
        key: SymbolId,
        _val: LispyValue,
        _ic: &mut InlineCache<LispyICEntry>,
    ) -> Result<(), RuntimeError> {
        Err(RuntimeError::NoSuchProperty { key })
    }

    // ── Builtins ─────────────────────────────────────────────────────

    fn resolve_builtin(name: &str) -> Option<BuiltinFn<Self>> {
        match name {
            "+" => Some(builtins::add),
            "-" => Some(builtins::sub),
            "*" => Some(builtins::mul),
            "/" => Some(builtins::div),
            "=" => Some(builtins::eq),
            "<" => Some(builtins::lt),
            ">" => Some(builtins::gt),
            "cons" => Some(builtins::cons),
            "car" => Some(builtins::car),
            "cdr" => Some(builtins::cdr),
            "null?" => Some(builtins::null_p),
            "pair?" => Some(builtins::pair_p),
            "number?" => Some(builtins::number_p),
            "symbol?" => Some(builtins::symbol_p),
            "print" => Some(builtins::print),
            _ => None,
        }
    }

    // ── Inline-cache invalidation (Lispy: no class redefinition) ─────
    //
    // Lispy has no runtime class modification, so the trait's
    // default no-op `invalidate_ics` is correct.  Explicit override
    // omitted.

    // ── Deopt support ────────────────────────────────────────────────

    fn materialize_value(repr: BoxedReprToken, location_value: u64) -> LispyValue {
        match repr {
            // Boxed reference: the location holds the raw
            // LispyValue word verbatim.
            //
            // SAFETY: the deopt frame descriptor records this as
            // BoxedRef only when the JIT/AOT codegen put a
            // properly-tagged LispyValue here.  Codegen owns this
            // invariant; the runtime cannot validate it.
            BoxedReprToken::BoxedRef => unsafe { LispyValue::from_raw_bits(location_value) },
            // Unboxed i64: the location holds an unboxed signed
            // integer; re-tag as immediate.
            BoxedReprToken::I64Unboxed => LispyValue::int(location_value as i64),
            // Unboxed f64: Lispy doesn't have floats yet (PR 2);
            // truncate to int for now and document that this path
            // gets revisited when flonums land.
            BoxedReprToken::F64Unboxed => LispyValue::int(f64::from_bits(location_value) as i64),
            // Unboxed bool: low bit determines the singleton.
            BoxedReprToken::BoolUnboxed => LispyValue::bool(location_value & 1 != 0),
            // Derived pointer: the runtime has resolved
            // base + offset into `location_value`.  Clear the low
            // 3 bits before OR'ing the tag so non-aligned
            // derived offsets can't corrupt the address (Finding
            // 6 from the security review).
            BoxedReprToken::DerivedPtr { .. } => unsafe {
                let cleared = location_value & !crate::value::TAG_BITS;
                LispyValue::from_raw_bits(cleared | crate::value::TAG_HEAP)
            },
        }
    }

    fn box_value(value: LispyValue) -> (BoxedReprToken, u64) {
        // Lispy values are already 64-bit words — boxed-ref is
        // always the right choice.  Future arithmetic
        // specialisation (unboxed i64 in tight loops) will choose
        // I64Unboxed instead, but PR 2 doesn't speculate yet.
        (BoxedReprToken::BoxedRef, value.bits())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intern::intern;

    fn cx<'a>() -> DispatchCx<'a, LispyBinding> {
        DispatchCx::<LispyBinding>::new_for_test()
    }

    fn ic() -> InlineCache<LispyICEntry> {
        InlineCache::new()
    }

    // ── type_tag / class_of ─────────────────────────────────────────

    #[test]
    fn type_tag_for_immediates_is_low_3_bits() {
        assert_eq!(LispyBinding::type_tag(LispyValue::int(0)), 0b000);
        assert_eq!(LispyBinding::type_tag(LispyValue::NIL), 0b001);
        assert_eq!(LispyBinding::type_tag(LispyValue::FALSE), 0b011);
        assert_eq!(LispyBinding::type_tag(LispyValue::TRUE), 0b101);
        assert_eq!(LispyBinding::type_tag(LispyValue::symbol(SymbolId(7))), 0b010);
    }

    #[test]
    fn type_tag_for_heap_returns_class_id() {
        let cell = heap::alloc_cons(LispyValue::int(1), LispyValue::int(2));
        assert_eq!(LispyBinding::type_tag(cell), CLASS_CONS);
        let clos = heap::alloc_closure(SymbolId(3), vec![]);
        assert_eq!(LispyBinding::type_tag(clos), CLASS_CLOSURE);
    }

    #[test]
    fn class_of_returns_correct_kind() {
        assert_eq!(LispyBinding::class_of(LispyValue::int(0)), Some(LispyClass::Int));
        assert_eq!(LispyBinding::class_of(LispyValue::NIL), Some(LispyClass::Nil));
        assert_eq!(LispyBinding::class_of(LispyValue::TRUE), Some(LispyClass::Bool));
        assert_eq!(LispyBinding::class_of(LispyValue::symbol(SymbolId(0))), Some(LispyClass::Symbol));
        let cell = heap::alloc_cons(LispyValue::int(1), LispyValue::int(2));
        assert_eq!(LispyBinding::class_of(cell), Some(LispyClass::Cons));
        let clos = heap::alloc_closure(SymbolId(0), vec![]);
        assert_eq!(LispyBinding::class_of(clos), Some(LispyClass::Closure));
    }

    #[test]
    fn lispy_class_to_class_id_is_stable() {
        // Two calls return the same id (no random state).
        assert_eq!(LispyClass::Int.to_class_id(), LispyClass::Int.to_class_id());
        // Distinct kinds get distinct ids.
        let ids: std::collections::HashSet<_> = [
            LispyClass::Int, LispyClass::Nil, LispyClass::Bool,
            LispyClass::Symbol, LispyClass::Cons, LispyClass::Closure,
        ].iter().map(|c| c.to_class_id()).collect();
        assert_eq!(ids.len(), 6);
    }

    // ── is_truthy / equal / identical / hash ────────────────────────

    #[test]
    fn is_truthy_follows_scheme_semantics() {
        assert!(LispyBinding::is_truthy(LispyValue::int(0)));
        assert!(LispyBinding::is_truthy(LispyValue::TRUE));
        assert!(LispyBinding::is_truthy(LispyValue::symbol(SymbolId(1))));
        assert!(!LispyBinding::is_truthy(LispyValue::FALSE));
        assert!(!LispyBinding::is_truthy(LispyValue::NIL));
    }

    #[test]
    fn equal_compares_immediates_by_bits() {
        assert!(LispyBinding::equal(LispyValue::int(5), LispyValue::int(5)));
        assert!(!LispyBinding::equal(LispyValue::int(5), LispyValue::int(6)));
        assert!(LispyBinding::equal(LispyValue::NIL, LispyValue::NIL));
        assert!(!LispyBinding::equal(LispyValue::NIL, LispyValue::FALSE));
    }

    #[test]
    fn equal_is_structural_for_cons_cells() {
        let a = heap::alloc_cons(LispyValue::int(1), LispyValue::int(2));
        let b = heap::alloc_cons(LispyValue::int(1), LispyValue::int(2));
        assert!(LispyBinding::equal(a, b), "(1 . 2) equal? (1 . 2) — distinct allocations");
        assert!(!LispyBinding::identical(a, b), "different allocations are not eq?");
    }

    #[test]
    fn equal_recurses_through_proper_lists() {
        // Build (1 2) twice from distinct allocations and compare.
        let a = heap::alloc_cons(LispyValue::int(1),
                heap::alloc_cons(LispyValue::int(2), LispyValue::NIL));
        let b = heap::alloc_cons(LispyValue::int(1),
                heap::alloc_cons(LispyValue::int(2), LispyValue::NIL));
        assert!(LispyBinding::equal(a, b));
    }

    #[test]
    fn identical_is_eq_question_mark_semantics() {
        assert!(LispyBinding::identical(LispyValue::int(7), LispyValue::int(7)));
        assert!(LispyBinding::identical(LispyValue::NIL, LispyValue::NIL));
        let a = heap::alloc_cons(LispyValue::int(1), LispyValue::NIL);
        let b = heap::alloc_cons(LispyValue::int(1), LispyValue::NIL);
        assert!(!LispyBinding::identical(a, b), "different allocations differ in eq?");
    }

    #[test]
    fn hash_is_deterministic() {
        let a = LispyValue::int(99);
        assert_eq!(LispyBinding::hash(a), LispyBinding::hash(a));
        assert_ne!(LispyBinding::hash(LispyValue::int(99)), LispyBinding::hash(LispyValue::int(100)));
    }

    // ── trace_value / trace_object ──────────────────────────────────

    struct RecordingVisitor { seen: Vec<u64> }
    impl ValueVisitor for RecordingVisitor {
        fn visit_value(&mut self, raw: u64) { self.seen.push(raw); }
    }

    #[test]
    fn trace_value_skips_immediates() {
        let mut v = RecordingVisitor { seen: vec![] };
        LispyBinding::trace_value(LispyValue::int(7), &mut v);
        LispyBinding::trace_value(LispyValue::NIL, &mut v);
        LispyBinding::trace_value(LispyValue::symbol(SymbolId(1)), &mut v);
        assert!(v.seen.is_empty(), "immediates carry no references");
    }

    #[test]
    fn trace_value_yields_heap_pointer() {
        let cell = heap::alloc_cons(LispyValue::int(1), LispyValue::int(2));
        let mut v = RecordingVisitor { seen: vec![] };
        LispyBinding::trace_value(cell, &mut v);
        assert_eq!(v.seen, vec![cell.bits()]);
    }

    #[test]
    fn trace_object_walks_cons_cell() {
        let cell = heap::alloc_cons(LispyValue::int(7), LispyValue::int(8));
        let header_ptr: *const ObjectHeader = cell.as_heap_ptr().unwrap();
        let mut v = RecordingVisitor { seen: vec![] };
        // SAFETY: alloc_cons returned a live, properly-tagged heap pointer.
        unsafe { LispyBinding::trace_object(header_ptr, &mut v) };
        assert_eq!(v.seen.len(), 2, "cons has 2 references");
        assert_eq!(v.seen[0], LispyValue::int(7).bits());
        assert_eq!(v.seen[1], LispyValue::int(8).bits());
    }

    #[test]
    fn trace_object_walks_closure_captures() {
        let captures = vec![LispyValue::int(10), LispyValue::int(20), LispyValue::TRUE];
        let clos = heap::alloc_closure(SymbolId(1), captures.clone());
        let header_ptr: *const ObjectHeader = clos.as_heap_ptr().unwrap();
        let mut v = RecordingVisitor { seen: vec![] };
        unsafe { LispyBinding::trace_object(header_ptr, &mut v) };
        assert_eq!(v.seen.len(), captures.len());
        for (got, want) in v.seen.iter().zip(captures.iter()) {
            assert_eq!(*got, want.bits());
        }
    }

    // ── Dispatch ────────────────────────────────────────────────────

    #[test]
    fn apply_callable_rejects_non_closure() {
        let mut c = cx();
        let err = LispyBinding::apply_callable(LispyValue::int(0), &[], &mut c).unwrap_err();
        assert_eq!(err, RuntimeError::NotCallable);
    }

    #[test]
    fn apply_callable_on_closure_returns_pr2_placeholder_error() {
        let clos = heap::alloc_closure(SymbolId(0), vec![]);
        let mut c = cx();
        let err = LispyBinding::apply_callable(clos, &[], &mut c).unwrap_err();
        // PR 2 documents this is a placeholder until PR 4.
        assert!(matches!(err, RuntimeError::Custom(s) if s.contains("PR 4")));
    }

    #[test]
    fn send_message_returns_no_such_method() {
        let mut c = cx();
        let mut i = ic();
        let err = LispyBinding::send_message(LispyValue::NIL, intern("foo"), &[], &mut i, &mut c).unwrap_err();
        match err {
            RuntimeError::NoSuchMethod { selector } => assert_eq!(selector, intern("foo")),
            other => panic!("expected NoSuchMethod, got {other:?}"),
        }
    }

    #[test]
    fn load_store_property_return_no_such_property() {
        let mut i = ic();
        assert!(matches!(
            LispyBinding::load_property(LispyValue::NIL, intern("x"), &mut i),
            Err(RuntimeError::NoSuchProperty { .. })
        ));
        assert!(matches!(
            LispyBinding::store_property(LispyValue::NIL, intern("x"), LispyValue::int(0), &mut i),
            Err(RuntimeError::NoSuchProperty { .. })
        ));
    }

    // ── Builtin resolution ──────────────────────────────────────────

    #[test]
    fn resolve_builtin_finds_each_tw00_name() {
        for name in ["+", "-", "*", "/", "=", "<", ">",
                     "cons", "car", "cdr",
                     "null?", "pair?", "number?", "symbol?",
                     "print"] {
            assert!(
                LispyBinding::resolve_builtin(name).is_some(),
                "{name} should resolve",
            );
        }
    }

    #[test]
    fn resolve_builtin_returns_none_for_unknown() {
        assert!(LispyBinding::resolve_builtin("does_not_exist").is_none());
    }

    #[test]
    fn resolved_builtin_executes() {
        let plus = LispyBinding::resolve_builtin("+").unwrap();
        assert_eq!(plus(&[LispyValue::int(2), LispyValue::int(3)]).unwrap(), LispyValue::int(5));
    }

    // ── Deopt support ───────────────────────────────────────────────

    #[test]
    fn box_then_materialize_round_trips() {
        let cases = [
            LispyValue::int(42),
            LispyValue::int(-1),
            LispyValue::NIL,
            LispyValue::TRUE,
            LispyValue::FALSE,
            LispyValue::symbol(SymbolId(7)),
        ];
        for v in cases {
            let (repr, raw) = LispyBinding::box_value(v);
            assert_eq!(repr, BoxedReprToken::BoxedRef);
            assert_eq!(LispyBinding::materialize_value(repr, raw), v);
        }
    }

    #[test]
    fn materialize_unboxed_i64() {
        assert_eq!(
            LispyBinding::materialize_value(BoxedReprToken::I64Unboxed, 99u64),
            LispyValue::int(99),
        );
    }

    #[test]
    fn materialize_unboxed_bool() {
        assert_eq!(LispyBinding::materialize_value(BoxedReprToken::BoolUnboxed, 1), LispyValue::TRUE);
        assert_eq!(LispyBinding::materialize_value(BoxedReprToken::BoolUnboxed, 0), LispyValue::FALSE);
    }

    #[test]
    fn invalidate_ics_default_is_no_op() {
        use lang_runtime_core::ICInvalidator;
        struct Inv { called: bool }
        impl ICInvalidator for Inv {
            fn invalidate_ic(&mut self, _: lang_runtime_core::ICId) { self.called = true; }
            fn invalidate_class(&mut self, _: lang_runtime_core::ClassId) { self.called = true; }
        }
        let mut inv = Inv { called: false };
        LispyBinding.invalidate_ics(&mut inv);
        assert!(!inv.called, "Lispy has no class redefinition; should not invalidate");
    }

    // ── Trait-level integration ─────────────────────────────────────

    #[test]
    fn binding_value_is_8_bytes() {
        assert_eq!(std::mem::size_of::<<LispyBinding as LangBinding>::Value>(), 8);
    }

    #[test]
    fn language_name_is_lispy() {
        assert_eq!(LispyBinding::LANGUAGE_NAME, "lispy");
    }
}
