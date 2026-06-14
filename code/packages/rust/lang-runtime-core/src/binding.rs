//! # The `LangBinding` trait — every language's plug-in surface.
//!
//! `LangBinding` is the **single trait every language frontend
//! implements** to plug into the LANG pipeline.  It is the entire
//! language-specific seam between:
//!
//! - the generic IIR (LANG01),
//! - the generic interpreter / JIT / AOT (LANG02 / LANG03 /
//!   LANG04),
//! - the generic GC and IC machinery (LANG16 / LANG20),
//!
//! and:
//!
//! - the language's own value representation,
//! - the language's own dispatch semantics (method lookup,
//!   property access, callable invocation),
//! - the language's own builtins.
//!
//! Per LANG20 §"The LangBinding trait", the trait surface is **15
//! required methods plus 3 associated types and 1 const** —
//! deliberately small, deliberately comprehensive.  Each method
//! corresponds 1:1 with a runtime mechanism the IIR dispatches
//! through; adding more would creep semantics into the trait,
//! removing any would force language-specific code into
//! `lang-runtime-core` where it doesn't belong.
//!
//! ## What this PR ships
//!
//! PR 1 (this one) ships the **trait skeleton only** — every
//! method's signature is final, but no real implementations exist
//! yet.  PR 2 adds `LispyBinding` (the first concrete
//! implementation) so the trait gets exercised against a real
//! language; PR 3+ wires `vm-core` to dispatch through the trait.
//!
//! See LANG20 §"Migration path" for the full sequence.
//!
//! ## Why object-safe-ish?
//!
//! The trait is intentionally **not** object-safe (it has
//! associated types and `Self`-typed parameters) — we accept the
//! restriction because:
//!
//! - The interpreter / JIT / AOT can be parameterised over `<L:
//!   LangBinding>` at compile time.  No dynamic dispatch needed
//!   for the hot path.
//! - The C ABI surface (LANG20 §"C ABI extensions") goes through
//!   `extern "C"` function pointers indexed by language id, not
//!   through trait objects.  Cross-language calls hop through C.

use std::hash::Hash;

use crate::deopt::FrameDescriptor;
use crate::error::RuntimeError;
use crate::ic::{ICInvalidator, InlineCache};
use crate::object::ObjectHeader;
use crate::value::{BoxedReprToken, SymbolId};
use crate::visitor::ValueVisitor;

// ---------------------------------------------------------------------------
// BuiltinFn
// ---------------------------------------------------------------------------

/// Function pointer signature for a builtin handler.
///
/// Returned by [`LangBinding::resolve_builtin`].  The runtime caches
/// the resolved pointer and emits direct calls to it from JIT/AOT
/// code; the interpreter calls through the pointer on each
/// invocation.
///
/// # Why a fn-pointer instead of a closure?
///
/// `extern "Rust" fn` is a real machine-code address — JIT/AOT
/// codegen can emit a direct call instruction without the
/// indirection of a vtable or closure environment.  Closures
/// would force every builtin call through a fat pointer.
pub type BuiltinFn<L> = fn(&[<L as LangBinding>::Value]) -> Result<<L as LangBinding>::Value, RuntimeError>;

// ---------------------------------------------------------------------------
// DispatchCx
// ---------------------------------------------------------------------------

/// Context handed to dispatch methods (`apply_callable`,
/// `send_message`) so the binding can re-enter the runtime.
///
/// PR 1 ships an opaque skeleton: the struct exists so trait
/// signatures lock in, but it has no public methods yet.  PR 4
/// (vm-core wiring) adds:
///
/// - `call_iir_function(&mut self, fn_name: &str, args: &[L::Value])
///   -> Result<L::Value, RuntimeError>` — invoke an IIRFunction by
///   name through the active VM tier.
/// - `intern_symbol(&mut self, name: &str) -> SymbolId` — get or
///   allocate an interned symbol id.
/// - `current_frame_pointer()` — for stack walking during deopt.
///
/// The `'a` lifetime ties the context to the dispatch call site —
/// a binding may not stash a `DispatchCx` between invocations.
///
/// # Why generic over `L`?
///
/// So the context's eventual `call_iir_function` can return
/// `L::Value` without requiring the binding to convert types at
/// every callback.
pub struct DispatchCx<'a, L: LangBinding> {
    // PhantomData ties the lifetime + binding to this struct
    // without requiring real fields yet.  Real fields land in
    // PR 4 when the context becomes useful.
    _phantom: core::marker::PhantomData<&'a mut L>,
}

impl<'a, L: LangBinding> DispatchCx<'a, L> {
    /// Construct an opaque, empty context.  Used by tests today;
    /// the runtime will own real construction once vm-core is
    /// wired in.
    #[doc(hidden)]
    pub fn new_for_test() -> Self {
        DispatchCx { _phantom: core::marker::PhantomData }
    }
}

impl<L: LangBinding> std::fmt::Debug for DispatchCx<'_, L> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DispatchCx").finish_non_exhaustive()
    }
}

// ---------------------------------------------------------------------------
// LangBinding
// ---------------------------------------------------------------------------

/// The contract every language frontend implements to plug into
/// the LANG pipeline.
///
/// See module-level docs for the rationale; see LANG20 §"The
/// LangBinding trait" for the design discussion; see LANG20
/// §"Why these specific 15 methods" for the per-method
/// justification.
///
/// # ABI invariants
///
/// - [`Value`] must be `Copy + 'static` and **8 bytes**.  This is
///   enforced at registration time via a const assertion (LANG20
///   §"The ABI contract").  A [`Value`] crosses tier boundaries
///   as a single `u64` machine word.
/// - [`ClassRef`] must be `Copy + Eq + Hash + 'static` so it can
///   be a key in IC tables and class-version maps.
/// - [`ICEntry`] must be `Copy + 'static`; size should be ≤ 16
///   bytes for cache-line friendliness.
///
/// # Default impls
///
/// Only two methods carry defaults — both safe to leave in place
/// when the language doesn't need them:
///
/// - [`finalize`](Self::finalize) defaults to a no-op (most
///   languages don't have finalisation).
/// - [`invalidate_ics`](Self::invalidate_ics) defaults to a no-op
///   (languages without runtime class modification don't need
///   to invalidate caches).
///
/// **Every other method is required**, including ones with
/// "obvious" defaults like `is_truthy`.  Earlier drafts had a
/// default `is_truthy` returning `true` and a default `identical`
/// using `transmute_copy::<Value, u64>` — both were footguns.  The
/// `is_truthy` default would make `if false { … }` take the
/// then-branch in any binding that forgot to override; the
/// `identical` default is undefined behaviour when `Value` isn't
/// exactly 8 bytes (which the trait can't enforce — associated
/// types don't carry size bounds).  Both methods being required
/// catches omissions at compile time.
///
/// [`Value`]: LangBinding::Value
/// [`ClassRef`]: LangBinding::ClassRef
/// [`ICEntry`]: LangBinding::ICEntry
pub trait LangBinding: Sized + 'static + Sync + Send {
    // ─── Associated types ────────────────────────────────────────────

    /// The ABI-stable value representation.  Must be `Copy` and
    /// the size of a machine word (8 bytes on 64-bit targets).
    /// See LANG20 §"Cross-language value representation" for the
    /// per-language encoding choices.
    type Value: Copy + 'static;

    /// Per-language opaque class identifier.  Carried in heap
    /// object headers via [`ObjectHeader::class_or_kind`] (cast
    /// from / to `u32` at the boundary); used for IC keying and
    /// reflection.
    type ClassRef: Copy + Eq + Hash + 'static;

    /// Per-language inline-cache entry shape.  Stored in
    /// [`InlineCache<Self::ICEntry>`]; emitted by the JIT as a
    /// compare-and-jump fast path.  See LANG20 §"Inline cache
    /// machinery" for per-language layouts.
    type ICEntry: Copy + 'static;

    // ─── Stable language identifier ──────────────────────────────────

    /// Stable language identifier, used in profile artefact files,
    /// debug dumps, and the IIRModule's `language` field.
    /// Lower-snake-case, ASCII only.
    const LANGUAGE_NAME: &'static str;

    // ─── Type & identity ─────────────────────────────────────────────

    /// Return the per-language type tag for `value`.
    ///
    /// Used by the profiler to record observed types in feedback
    /// slots and by `==`-style operations.  Must be cheap (single
    /// instruction or table lookup).
    fn type_tag(value: Self::Value) -> u32;

    /// Return the class of `value`, or `None` if `value` is an
    /// immediate (tagged int, nil, bool — anything without a heap
    /// header).  Used by IC keying and reflection.
    fn class_of(value: Self::Value) -> Option<Self::ClassRef>;

    /// Truthiness for `if` / `jmp_if_*` opcodes.
    ///
    /// Languages vary: Scheme treats only `#f` as false; Python
    /// treats `0`, empty containers, and `None` as false; Ruby
    /// treats `nil` and `false` as false.
    ///
    /// **Required** — no default.  A trait-level default of
    /// `true` would silently turn `if false { … }` into the
    /// then-branch in any binding that forgot to override, an
    /// excellent recipe for control-flow bugs.  Forcing the impl
    /// catches the omission at compile time.
    fn is_truthy(value: Self::Value) -> bool;

    /// Structural equality (`equal?` in Scheme; `==` in Ruby;
    /// `===` in JS).  Used by the `cmp_eq` IIR opcode.
    fn equal(a: Self::Value, b: Self::Value) -> bool;

    /// Identity equality (`eq?` in Scheme; `equal?` in Ruby;
    /// `Object.is` in JS).
    ///
    /// **Required** — no default.  Earlier drafts shipped a
    /// default that used `transmute_copy::<Value, u64>`, but
    /// `transmute_copy` is **undefined behaviour** when the
    /// source type isn't exactly the destination type's size,
    /// and the trait can't enforce a compile-time size on
    /// `Value` (associated types don't carry size bounds).
    /// Bindings whose `Value` happens to be 8 bytes can
    /// implement this in one line as a `u64` transmute (see the
    /// `TestBinding` in this crate's tests for an example);
    /// bindings with smaller or padded values implement it
    /// however suits their tagging scheme.
    fn identical(a: Self::Value, b: Self::Value) -> bool;

    /// Hash for keying.  Used by IC keying and hash-map-builtin
    /// implementations.
    fn hash(value: Self::Value) -> u64;

    // ─── Heap interaction (delegates to gc-core / LANG16) ────────────

    /// Walk every reference reachable from this object's payload,
    /// calling [`ValueVisitor::visit_value`] on each.  Called by
    /// the collector during tracing.
    ///
    /// `obj_header` points at a heap object whose `class_or_kind`
    /// matches a kind this binding registered.  The binding
    /// decodes the payload layout and visits every reference
    /// field.
    ///
    /// # Safety
    ///
    /// `obj_header` must be a valid pointer to an
    /// [`ObjectHeader`] followed by a payload whose layout
    /// matches the registered class.  The runtime upholds this;
    /// implementations should not validate.
    ///
    /// The signature uses `*const ObjectHeader` to make crossing
    /// the FFI / GC boundary cheap; impls cast to their payload
    /// type internally.
    unsafe fn trace_object(obj_header: *const ObjectHeader, visitor: &mut dyn ValueVisitor);

    /// Walk references reachable from a `Value` directly.  For
    /// tagged immediates this is a no-op; for heap-backed values
    /// it dereferences and calls `trace_object`.  Invoked by the
    /// GC when scanning roots.
    fn trace_value(value: Self::Value, visitor: &mut dyn ValueVisitor);

    /// Optional object finalizer.  Called at most once when the
    /// object becomes unreachable, before the collector frees the
    /// allocation.
    ///
    /// Default: no-op.  Languages with finalization (Ruby's
    /// `ObjectSpace.define_finalizer`, JS's `FinalizationRegistry`)
    /// override; languages without (Lispy) leave the default.
    ///
    /// # Safety
    ///
    /// Same invariants as [`trace_object`](Self::trace_object).
    unsafe fn finalize(_obj_header: *mut ObjectHeader) {}

    // ─── Dispatch (the polymorphic seam) ─────────────────────────────

    /// Apply a callable value to argument values.  Backs the IIR
    /// `call_indirect` opcode.  This is where `apply_closure`
    /// semantics live (look up the closure handle, prepend
    /// captured env, call the inner IIRFunction).
    ///
    /// Returns the call result, or [`RuntimeError`] for runtime
    /// failures (e.g. value isn't callable, arity mismatch in a
    /// strict-arity language).
    fn apply_callable(
        callable: Self::Value,
        args: &[Self::Value],
        cx: &mut DispatchCx<'_, Self>,
    ) -> Result<Self::Value, RuntimeError>;

    /// Look up a method on a receiver and invoke it.  Backs the
    /// IIR `send` opcode (LANG20 §"IIR additions").  `selector`
    /// is an interned symbol id from the IIRFunction's constant
    /// pool.
    ///
    /// `ic` is the per-call-site inline cache; the binding warms
    /// it on the slow path so subsequent calls hit the JIT-emitted
    /// fast path.
    ///
    /// Languages without method dispatch (pure Lispy languages
    /// without object protocols) implement this by returning
    /// [`RuntimeError::NoSuchMethod`] for every call — the IR
    /// frontend won't emit `send` opcodes anyway.
    fn send_message(
        receiver: Self::Value,
        selector: SymbolId,
        args: &[Self::Value],
        ic: &mut InlineCache<Self::ICEntry>,
        cx: &mut DispatchCx<'_, Self>,
    ) -> Result<Self::Value, RuntimeError>;

    /// Read an object property by symbol.  Backs the IIR
    /// `load_property` opcode (LANG20 §"IIR additions").
    ///
    /// `ic` is the per-load-site inline cache (same lifecycle as
    /// `send_message`'s).
    fn load_property(
        obj: Self::Value,
        key: SymbolId,
        ic: &mut InlineCache<Self::ICEntry>,
    ) -> Result<Self::Value, RuntimeError>;

    /// Write an object property by symbol.  Backs the IIR
    /// `store_property` opcode (LANG20 §"IIR additions").
    fn store_property(
        obj: Self::Value,
        key: SymbolId,
        val: Self::Value,
        ic: &mut InlineCache<Self::ICEntry>,
    ) -> Result<(), RuntimeError>;

    // ─── Builtins ────────────────────────────────────────────────────

    /// Resolve a builtin by name to a stable function pointer.
    ///
    /// Called once per name at link time (LANG10 / LANG15
    /// §"relocation contract") so the JIT/AOT can emit direct
    /// calls.  Returns `None` if the binding doesn't recognise
    /// the name.
    ///
    /// The returned [`BuiltinFn`] has a single signature:
    /// `fn(&[Value]) -> Result<Value, RuntimeError>`.  Builtins
    /// that need a context (e.g. for symbol interning) must close
    /// over a static or thread-local — we deliberately don't
    /// thread `DispatchCx` through `BuiltinFn` because most
    /// builtins are pure.
    fn resolve_builtin(name: &str) -> Option<BuiltinFn<Self>>;

    // ─── Inline-cache invalidation ──────────────────────────────────

    /// Tell the runtime to invalidate caches affected by recent
    /// language-level state changes (class redefinition, method
    /// override, JS prototype mutation, Ruby reopen, Smalltalk
    /// `become:`).
    ///
    /// The binding's own state-tracking determines which caches
    /// to invalidate; it calls `invalidator.invalidate_class(cls)`
    /// (bulk hammer) or `invalidator.invalidate_ic(ic)` (targeted)
    /// for each.
    ///
    /// Default: no-op.  Languages without runtime class
    /// modification (Tetrad, Twig today) leave the default;
    /// dynamic languages (Ruby, JS, Smalltalk) override.
    fn invalidate_ics(&self, invalidator: &mut dyn ICInvalidator) {
        let _ = invalidator;
    }

    // ─── Deopt support ───────────────────────────────────────────────

    /// Materialise a `Value` from its specialised native
    /// representation.
    ///
    /// Called by `rt_deopt` (LANG20 §"C ABI extensions") for each
    /// register entry in the deopt frame descriptor.  The
    /// descriptor records the raw `u64` from the native location
    /// and the [`BoxedReprToken`] describing how to interpret it.
    ///
    /// # Examples per repr
    ///
    /// | `repr` | `location_value` | Materialise to |
    /// |--------|------------------|----------------|
    /// | [`BoxedReprToken::BoxedRef`] | the raw `Value` word | take as-is (transmute) |
    /// | [`BoxedReprToken::I64Unboxed`] | a signed i64 (as u64 bits) | wrap as the language's integer |
    /// | [`BoxedReprToken::F64Unboxed`] | `f64::to_bits()` | wrap as the language's float |
    /// | [`BoxedReprToken::BoolUnboxed`] | low bit | wrap as the language's bool |
    /// | [`BoxedReprToken::DerivedPtr { … }`] | base+offset already resolved by the runtime | re-box as a heap reference |
    fn materialize_value(repr: BoxedReprToken, location_value: u64) -> Self::Value;

    /// Inverse of [`materialize_value`](Self::materialize_value):
    /// produce the specialised native representation of a `Value`
    /// so a re-entered JIT/AOT frame can place it in registers.
    ///
    /// Returns the `BoxedReprToken` describing how the value will
    /// be encoded plus the raw `u64` to put in the native
    /// location.  `rt_re_enter_specialised` (LANG20 §"C ABI
    /// extensions") writes this pair into the appropriate native
    /// register / stack slot before tail-calling the JIT entry
    /// point.
    ///
    /// Most languages return `(BoxedRef, value_bits)` for boxed
    /// values and `(I64Unboxed, n as u64)` for unboxed integers
    /// where the codegen has chosen unboxed scalar reps.
    fn box_value(value: Self::Value) -> (BoxedReprToken, u64);

    // ─── Frame-descriptor materialisation helper ────────────────────

    /// Convenience wrapper: walk a [`FrameDescriptor`]'s register
    /// list and materialise every entry, returning a vector of
    /// `(ir_name, Value)` pairs the runtime can hand to
    /// `VMFrame::assign`.
    ///
    /// PR 1 ships this as a default impl that the runtime will
    /// use later; bindings can override for performance (e.g. to
    /// avoid a heap allocation per deopt).
    ///
    /// `read_native`: callback that reads the native location and
    /// returns the raw `u64` — supplied by the runtime because it
    /// owns the native frame.
    fn materialize_frame(
        descriptor: &FrameDescriptor,
        mut read_native: impl FnMut(crate::deopt::NativeLocation) -> u64,
    ) -> Vec<(String, Self::Value)>
    where
        Self: Sized,
    {
        descriptor
            .registers
            .iter()
            .map(|entry| {
                let raw = match entry.repr {
                    BoxedReprToken::DerivedPtr { base_register, offset } => {
                        let base = read_native(crate::deopt::NativeLocation::Register(base_register));
                        // Offset is signed bytes; arithmetic on raw u64 here.
                        base.wrapping_add(offset as i64 as u64)
                    }
                    _ => read_native(entry.location),
                };
                let value = Self::materialize_value(entry.repr, raw);
                (entry.ir_name.clone(), value)
            })
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Tests — implement LangBinding for a tiny test type to prove the trait
// is implementable end-to-end.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// A toy `Value` for the test binding: tagged i64 where the
    /// low 3 bits discriminate int / bool / nil / heap-handle.
    /// Matches the Lispy convention from LANG20 (§"Cross-language
    /// value representation") — close enough to validate the trait
    /// without pulling in the real lispy-runtime crate.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    struct TestValue(u64);

    const TAG_INT: u64 = 0b000;
    const TAG_NIL: u64 = 0b001;
    const TAG_FALSE: u64 = 0b011;
    const TAG_TRUE: u64 = 0b101;

    impl TestValue {
        fn int(n: i64) -> Self { TestValue((n as u64) << 3 | TAG_INT) }
        fn nil() -> Self { TestValue(TAG_NIL) }
        fn b(v: bool) -> Self { TestValue(if v { TAG_TRUE } else { TAG_FALSE }) }
        fn tag(self) -> u64 { self.0 & 0b111 }
    }

    /// A toy `ClassRef` enum.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    enum TestClass { Int, Nil, Bool, Cons }

    /// A toy IC entry — keyed on the integer tag.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct TestICEntry { tag: u32, target: usize }

    /// The actual test binding.
    struct TestBinding;

    impl LangBinding for TestBinding {
        type Value = TestValue;
        type ClassRef = TestClass;
        type ICEntry = TestICEntry;
        const LANGUAGE_NAME: &'static str = "test";

        fn type_tag(v: TestValue) -> u32 { v.tag() as u32 }

        fn class_of(v: TestValue) -> Option<TestClass> {
            match v.tag() {
                TAG_INT => Some(TestClass::Int),
                TAG_NIL => Some(TestClass::Nil),
                TAG_FALSE | TAG_TRUE => Some(TestClass::Bool),
                _ => None,
            }
        }

        fn is_truthy(v: TestValue) -> bool {
            // Scheme rule: only #f and nil are false.
            !matches!(v.tag(), TAG_FALSE | TAG_NIL)
        }

        fn equal(a: TestValue, b: TestValue) -> bool { a == b }

        fn identical(a: TestValue, b: TestValue) -> bool {
            // TestValue wraps a single u64; identity is bitwise
            // equality of that word.  Real bindings whose `Value`
            // is also a single 64-bit POD can use this same
            // pattern.
            a.0 == b.0
        }

        fn hash(v: TestValue) -> u64 { v.0 }

        unsafe fn trace_object(_h: *const ObjectHeader, _vis: &mut dyn ValueVisitor) {
            // Test binding has no heap objects (everything is
            // immediate), so trace is a no-op.
        }

        fn trace_value(_v: TestValue, _vis: &mut dyn ValueVisitor) {
            // Same reason — immediates only.
        }

        fn apply_callable(
            _callable: TestValue,
            _args: &[TestValue],
            _cx: &mut DispatchCx<'_, Self>,
        ) -> Result<TestValue, RuntimeError> {
            Err(RuntimeError::NotCallable)
        }

        fn send_message(
            _receiver: TestValue,
            _selector: SymbolId,
            _args: &[TestValue],
            _ic: &mut InlineCache<TestICEntry>,
            _cx: &mut DispatchCx<'_, Self>,
        ) -> Result<TestValue, RuntimeError> {
            Err(RuntimeError::NoSuchMethod { selector: SymbolId::NONE })
        }

        fn load_property(
            _obj: TestValue,
            _key: SymbolId,
            _ic: &mut InlineCache<TestICEntry>,
        ) -> Result<TestValue, RuntimeError> {
            Err(RuntimeError::NoSuchProperty { key: SymbolId::NONE })
        }

        fn store_property(
            _obj: TestValue,
            _key: SymbolId,
            _val: TestValue,
            _ic: &mut InlineCache<TestICEntry>,
        ) -> Result<(), RuntimeError> {
            Err(RuntimeError::NoSuchProperty { key: SymbolId::NONE })
        }

        fn resolve_builtin(name: &str) -> Option<BuiltinFn<Self>> {
            match name {
                "identity" => Some(test_identity),
                _ => None,
            }
        }

        fn materialize_value(repr: BoxedReprToken, raw: u64) -> TestValue {
            match repr {
                BoxedReprToken::BoxedRef => TestValue(raw),
                BoxedReprToken::I64Unboxed => TestValue::int(raw as i64),
                BoxedReprToken::BoolUnboxed => TestValue::b(raw & 1 != 0),
                BoxedReprToken::F64Unboxed => {
                    // Test binding has no float type; fold into an int for the test.
                    TestValue::int(f64::from_bits(raw) as i64)
                }
                BoxedReprToken::DerivedPtr { .. } => TestValue::nil(),
            }
        }

        fn box_value(value: TestValue) -> (BoxedReprToken, u64) {
            (BoxedReprToken::BoxedRef, value.0)
        }
    }

    fn test_identity(args: &[TestValue]) -> Result<TestValue, RuntimeError> {
        Ok(args.first().copied().unwrap_or(TestValue::nil()))
    }

    // ── Trait-surface tests ───────────────────────────────────────────

    #[test]
    fn type_tag_distinguishes_immediates() {
        assert_eq!(TestBinding::type_tag(TestValue::int(42)), TAG_INT as u32);
        assert_eq!(TestBinding::type_tag(TestValue::nil()), TAG_NIL as u32);
        assert_eq!(TestBinding::type_tag(TestValue::b(true)), TAG_TRUE as u32);
        assert_eq!(TestBinding::type_tag(TestValue::b(false)), TAG_FALSE as u32);
    }

    #[test]
    fn class_of_returns_some_for_immediates() {
        assert_eq!(TestBinding::class_of(TestValue::int(0)), Some(TestClass::Int));
        assert_eq!(TestBinding::class_of(TestValue::nil()), Some(TestClass::Nil));
    }

    #[test]
    fn is_truthy_follows_scheme_semantics() {
        assert!(TestBinding::is_truthy(TestValue::int(0))); // 0 is true in Scheme
        assert!(TestBinding::is_truthy(TestValue::int(1)));
        assert!(TestBinding::is_truthy(TestValue::b(true)));
        assert!(!TestBinding::is_truthy(TestValue::b(false)));
        assert!(!TestBinding::is_truthy(TestValue::nil()));
    }

    #[test]
    fn equal_is_value_equality() {
        assert!(TestBinding::equal(TestValue::int(7), TestValue::int(7)));
        assert!(!TestBinding::equal(TestValue::int(7), TestValue::int(8)));
        assert!(TestBinding::equal(TestValue::nil(), TestValue::nil()));
    }

    #[test]
    fn identical_default_uses_bitwise_equality() {
        assert!(TestBinding::identical(TestValue::int(5), TestValue::int(5)));
        assert!(!TestBinding::identical(TestValue::int(5), TestValue::int(6)));
    }

    #[test]
    fn hash_is_deterministic() {
        assert_eq!(TestBinding::hash(TestValue::int(99)), TestBinding::hash(TestValue::int(99)));
    }

    #[test]
    fn dispatch_methods_return_errors_for_unsupported_ops() {
        let mut cx = DispatchCx::<TestBinding>::new_for_test();
        let mut ic = InlineCache::new();
        let v = TestValue::int(0);

        assert!(matches!(
            TestBinding::apply_callable(v, &[], &mut cx),
            Err(RuntimeError::NotCallable)
        ));
        assert!(matches!(
            TestBinding::send_message(v, SymbolId(1), &[], &mut ic, &mut cx),
            Err(RuntimeError::NoSuchMethod { .. })
        ));
        assert!(matches!(
            TestBinding::load_property(v, SymbolId(1), &mut ic),
            Err(RuntimeError::NoSuchProperty { .. })
        ));
        assert!(matches!(
            TestBinding::store_property(v, SymbolId(1), v, &mut ic),
            Err(RuntimeError::NoSuchProperty { .. })
        ));
    }

    #[test]
    fn resolve_builtin_returns_known_handler() {
        let f = TestBinding::resolve_builtin("identity").expect("identity exists");
        let v = TestValue::int(7);
        assert_eq!(f(&[v]).unwrap(), v);
    }

    #[test]
    fn resolve_builtin_returns_none_for_unknown() {
        assert!(TestBinding::resolve_builtin("does_not_exist").is_none());
    }

    #[test]
    fn materialize_value_round_trips_boxed_ref() {
        let v = TestValue::int(42);
        let (_, bits) = TestBinding::box_value(v);
        let rt = TestBinding::materialize_value(BoxedReprToken::BoxedRef, bits);
        assert_eq!(rt, v);
    }

    #[test]
    fn materialize_value_handles_unboxed_i64() {
        let v = TestBinding::materialize_value(BoxedReprToken::I64Unboxed, 42u64);
        assert_eq!(v, TestValue::int(42));
    }

    #[test]
    fn materialize_value_handles_unboxed_bool() {
        assert_eq!(
            TestBinding::materialize_value(BoxedReprToken::BoolUnboxed, 1),
            TestValue::b(true)
        );
        assert_eq!(
            TestBinding::materialize_value(BoxedReprToken::BoolUnboxed, 0),
            TestValue::b(false)
        );
    }

    #[test]
    fn materialize_frame_walks_descriptor() {
        use crate::deopt::{FrameDescriptor, NativeLocation, RegisterEntry};

        let descriptor = FrameDescriptor::new(3)
            .with_register(RegisterEntry {
                ir_name: "a".into(),
                location: NativeLocation::Register(0),
                repr: BoxedReprToken::I64Unboxed,
                speculated_type_tag: None,
            })
            .with_register(RegisterEntry {
                ir_name: "b".into(),
                location: NativeLocation::Register(1),
                repr: BoxedReprToken::BoolUnboxed,
                speculated_type_tag: None,
            });

        // Stub native-read: register 0 holds 99, register 1 holds 1 (true).
        let read = |loc: NativeLocation| match loc {
            NativeLocation::Register(0) => 99,
            NativeLocation::Register(1) => 1,
            _ => 0,
        };

        let frame = TestBinding::materialize_frame(&descriptor, read);
        assert_eq!(frame.len(), 2);
        assert_eq!(frame[0].0, "a");
        assert_eq!(frame[0].1, TestValue::int(99));
        assert_eq!(frame[1].0, "b");
        assert_eq!(frame[1].1, TestValue::b(true));
    }

    #[test]
    fn invalidate_ics_default_is_no_op() {
        struct Inv { calls: u32 }
        impl ICInvalidator for Inv {
            fn invalidate_ic(&mut self, _: crate::ic::ICId) { self.calls += 1; }
            fn invalidate_class(&mut self, _: crate::ic::ClassId) { self.calls += 1; }
        }
        let mut inv = Inv { calls: 0 };
        TestBinding.invalidate_ics(&mut inv);
        assert_eq!(inv.calls, 0, "default impl must not invoke the invalidator");
    }

    #[test]
    fn dispatch_cx_is_constructible_for_tests() {
        // Compile-only — proves the test ctor exists and `Self: Sized` works.
        let _cx = DispatchCx::<TestBinding>::new_for_test();
    }

    #[test]
    fn language_name_is_set() {
        assert_eq!(TestBinding::LANGUAGE_NAME, "test");
    }

    #[test]
    fn value_size_assertion_holds() {
        // The trait's docs commit to Value being 8 bytes.  This
        // test catches accidental enlargement.
        assert_eq!(std::mem::size_of::<<TestBinding as LangBinding>::Value>(), 8);
    }
}
