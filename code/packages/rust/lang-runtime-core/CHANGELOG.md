# Changelog — lang-runtime-core

## [0.1.0] — 2026-04-29

### Added

- **PR 1 of [LANG20](../../../specs/LANG20-multilang-runtime.md) §"Migration path"**:
  trait skeleton + supporting types.  No live GC, no interpreter
  wiring, no real C ABI exports — those follow in PR 2+.
- The `LangBinding` trait — single contract every language frontend
  implements.  3 associated types (`Value`, `ClassRef`, `ICEntry`),
  1 const (`LANGUAGE_NAME`), 15 required methods (`type_tag`,
  `class_of`, `is_truthy`, `equal`, `identical`, `hash`,
  `trace_object`, `trace_value`, `finalize`, `apply_callable`,
  `send_message`, `load_property`, `store_property`,
  `resolve_builtin`, `materialize_value`, `box_value`).  Three
  methods carry correct defaults (`is_truthy=true`, `identical`=
  bitwise eq, `finalize`=no-op, `invalidate_ics`=no-op).
- `BuiltinFn<L>` — function-pointer signature for builtin handlers.
- `DispatchCx<'a, L>` — opaque skeleton handed to dispatch methods;
  PR 4 will add the real entry points (call_iir_function,
  intern_symbol, current_frame_pointer).
- `SymbolId(u32)` — interned-symbol handle; `EMPTY` reserved for
  the empty string.
- `BoxedReprToken` — discriminator for how a value is encoded at a
  deopt anchor (BoxedRef, I64Unboxed, F64Unboxed, BoolUnboxed,
  DerivedPtr).  Const-asserted to ≤ 8 bytes.
- `ObjectHeader` — uniform 16-byte heap-object preamble
  (`class_or_kind`, `gc_word: AtomicU32`, `size_bytes`, `flags`).
  Const-asserted at exactly 16 bytes — the LANG20 ABI commitment
  every language must agree on.
- `header_flags::*` — reserved low-8-bit flag namespace
  (`HAS_FINALIZER`, `IS_IMMORTAL`, `IS_OLD_GEN`, `SKIP_TRACE`).
  Upper 24 bits free for per-language use.
- `InlineCache<E>` — generic V8-style inline cache; per-language
  entry shape via `LangBinding::ICEntry`.  4-entry default
  (`MAX_PIC_ENTRIES = 4`) matches V8/SpiderMonkey convention.
  `ICState` lifecycle: Uninit → Monomorphic → Polymorphic →
  Megamorphic.
- `ICId(u32)` / `ClassId(u32)` — newtype wrappers around u32 for
  IC identification.  4-byte transparent newtypes (ABI-friendly).
- `ICInvalidator` trait — runtime-side callback for invalidating
  caches after class redefinition.
- `FrameDescriptor` + `RegisterEntry` + `NativeLocation` +
  `DeoptAnchor` + `InlinedDeoptDescriptor` — full deopt protocol
  type set.  Supports inlined-call deopt via stacked descriptors.
- `ValueVisitor` + `RootVisitor` — non-generic visitor traits for
  GC tracing and root scanning.  Object-safe so `&mut dyn` works.
- `RuntimeError` — cross-language error transport with five
  variants (NotCallable, NoSuchMethod, NoSuchProperty, TypeError,
  Custom); implements `std::error::Error`.
- `LangBinding::materialize_frame` — default-impl convenience that
  walks a `FrameDescriptor` and produces `(ir_name, Value)` pairs
  the runtime can hand to `VMFrame::assign`.
- 68 unit tests + 1 doc test exercising every type's invariants
  plus a complete `TestBinding` impl that proves the trait is
  implementable end-to-end from the doc alone.

### Trait-surface ABI commitments

These are baked into the type system via const assertions and tested:

- `LangBinding::Value` must be exactly 8 bytes (enforced at
  registration; the `TestBinding` test asserts).
- `ObjectHeader` is exactly 16 bytes (compile-time const assertion).
- `SymbolId` / `ICId` / `ClassId` are exactly 4 bytes
  (`#[repr(transparent)]` + tested).
- `MAX_PIC_ENTRIES == 4` (locked default; per-language tunings can
  read but should match for codegen consistency).

### Hardening from security review

- `LangBinding::is_truthy` and `LangBinding::identical` are
  **required** (no defaults).  Earlier drafts shipped a default
  `is_truthy` returning `true` (control-flow footgun in any
  binding that forgot to override) and a default `identical`
  using `transmute_copy::<Value, u64>` (undefined behaviour
  whenever `Value` isn't exactly 8 bytes — the trait can't enforce
  size on associated types, so this would fire silently in any
  language frontend with a non-64-bit value rep).  Both are now
  explicit obligations on the implementor.
- `SymbolId::EMPTY` (`SymbolId(0)`, the empty string) and
  `SymbolId::NONE` (`SymbolId(u32::MAX)`, the missing-symbol
  sentinel) are now **distinct** values.  Earlier drafts overloaded
  `SymbolId(0)` as both, which would cause `obj[""]` lookups to
  silently behave like missing-property errors in any binding that
  used the same id as its sentinel.
- `InlineCache` fields (`entries`, `state`, `hit_count`,
  `miss_count`) are now **private**, with read-only accessors.
  Public fields would let consumers desynchronise the state
  machine (e.g. `state = Monomorphic` with empty `entries`),
  which becomes a load-from-stale-pointer the moment JIT-emitted
  code reads via known offsets.  `#[repr(C)]` is added so the
  JIT can still bake offsets in via `offset_of!` at codegen time.
- Compile-time size assertions (`const _: () = assert!(...)`)
  added for `SymbolId`, `ICId`, `ClassId`, and `BoxedReprToken`.
  Earlier drafts had only runtime `#[test]` checks for these;
  promoting them to compile-time means an accidental enlargement
  of any of these types breaks the build immediately, not just
  when downstream tests happen to run.

### Known limitations (intentional, follow in later PRs)

- `DispatchCx` has no public methods yet — PR 4 wires it to vm-core.
- No GC/allocator implementation — LANG16 work covers it.
- No C ABI exports yet — PR 6+.
- No real binding implementations — PR 2 ships `LispyBinding`.
