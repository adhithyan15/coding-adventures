# LANG20 — Multi-Language Runtime Architecture: V8-Class Tiering for Every Language

## Overview

LANG20 is the **cross-language overlay** on the LANG pipeline.  It pins down
the contract that lets *any* language frontend (Lisp, Ruby, JavaScript,
Smalltalk, Perl, Tetrad, Twig, …) plug into the same interpreter, JIT, and
AOT compiler — and get **V8-class tiered execution** plus **GraalVM-style
ahead-of-time native binaries** without re-implementing either tier.

The vision the user has stated:

> Bring something like V8 and its JIT compiler to every language that
> implements on top of the LANG VM with very little effort, and combine
> that with GraalVM-style "everything AOT".

That combination is **horizontal × vertical**:

| Axis | V8 | GraalVM | LANG goal |
|------|----|---------|-----------|
| Languages supported | 1 (JS) | many (via Truffle) | **many** |
| Tiers per language | interp + 2 JITs | interp + JIT (partial-eval) | **interp + JIT + AOT** |
| AOT story | none | Native Image (mostly Java/JS) | **all languages** |

LANG20 specifies the architectural pieces that make this combination work:

1. **The `LangBinding` trait** — the contract every language frontend
   implements to plug into the runtime.
2. **The cross-language `Value` representation strategy** — how Lispy
   tagged-int, JS NaN-boxing, Smalltalk class-pointer-header, and Ruby
   `RObject` value reps coexist on one heap and one ABI.
3. **The feedback-slot taxonomy** — per-IIR-instruction profile data that
   the interpreter writes, the JIT consumes for speculation, and the AOT
   compiler consumes for profile-guided optimisation.
4. **The deopt protocol** — how JIT/AOT-emitted native code yields control
   back to the interpreter at IIR-level granularity when speculation
   fails.
5. **The inline-cache (IC) machinery** — V8 hidden classes generalised
   into a per-language IC strategy (Lisp type-tag IC, Smalltalk PIC,
   Ruby class+version IC, JS hidden classes).
6. **The crate structure** — what lives in
   `lang-runtime-core` (truly generic) vs.
   `<lang>-runtime` (per-language) vs.
   the language frontend.

LANG20 does **not** redefine what LANG15 (vm-runtime C ABI) or LANG16
(gc-core: heap, alloc opcodes, root scanning, stack maps, write barriers)
already pin down.  Those specs are language-agnostic; LANG20 fills in the
language-specific seams that were left as forward references in those
documents ("the language frontend's root scanner", "the registered
collector", "the kind id table").

This spec is **architecture, not implementation**.  Concrete crate work
follows in sub-specs (LANG21+) and per-language runtime specs.

---

## Why this spec is needed now

The existing LANG00–LANG19 specs were drafted around **statically-typed
Tetrad** as the prototype language.  IIR carries `type_hint`s like `u8`,
`u32`, `bool`, the JIT specialises from observed primitive types, AOT
works for fully-typed programs.  This is a great bring-up target — but it
leaves three holes for the multi-language vision:

1. **No explicit per-language plug-in surface.**  Today a frontend like
   `twig-ir-compiler` reaches directly into `interpreter-ir`, emits
   `call_builtin "make_closure"` strings, and *implicitly* requires
   `vm-core` to be configured with a matching builtin registry.  There
   is no trait that says "to plug into the LANG runtime, implement these
   methods".  A second language frontend rediscovers the same
   integration points by trial and error.
2. **The dynamic-language story is unspecified.**  Lisp, Ruby, JS,
   Smalltalk, Perl don't have static `u8`/`u32`/`bool` types — they
   carry runtime type tags on every value.  The existing IIR
   `type_hint = "any"` is the catch-all, but the observed-type
   machinery is built around a small set of primitive types and the
   `polymorphic` sentinel.  A real dynamic language needs a richer
   feedback model (call-site target identity, hidden-class shape,
   method-version stamping) and a deopt protocol that materialises
   boxed values back into interpreter state.
3. **Inline caches are absent.**  V8's per-call-site type/shape feedback
   is what turns a generic property load into a single MOV.  Without
   IC machinery, "V8-class JIT for every language" is a slogan, not an
   architecture.

LANG20 fills all three holes in one spec because the trait surface, the
feedback taxonomy, and the IC machinery are deeply interrelated — pinning
one without the others produces a contract that doesn't actually compose.

---

## Relationship to existing specs

LANG20 sits **alongside** the existing LANG specs and adds the
cross-language overlay:

| Spec | What it pins down | LANG20's relationship |
|------|-------------------|-----------------------|
| LANG00 | Generic pipeline overview | LANG20 extends with "and supports many languages" |
| LANG01 | InterpreterIR (IIR) format | LANG20 adds: feedback-slot taxonomy, deopt anchors, may-IC flag |
| LANG02 | vm-core interpreter | LANG20 adds: must call `LangBinding` for all polymorphic dispatches |
| LANG03 | jit-core tiered JIT | LANG20 adds: speculation reads feedback slots; deopt protocol pinned |
| LANG04 | aot-core AOT pipeline | LANG20 adds: PGO-mode reads recorded feedback; closed-world via LangBinding |
| LANG05 | Backend protocol | LANG20 adds: backends emit stack maps + deopt anchors per protocol |
| LANG15 | vm-runtime C ABI | LANG20 extends C ABI for per-language entries + IC machinery |
| LANG16 | gc-core (heap, alloc, GC) | LANG20 specifies how `LangBinding` registers kinds + trace functions |
| LANG11 | jit-profiling-insights | Same feedback slots LANG20 defines feed into LANG11's reports |

A new LANG20-conformant frontend therefore must:

- Emit IIR per LANG01 (with feedback slots filled in per LANG20 §"Feedback slot taxonomy").
- Implement `LangBinding` per LANG20 §"The LangBinding trait".
- Register the binding with `vm-core` / `jit-core` / `aot-core` via the entry points LANG20 §"C ABI extensions" defines.
- Reuse the heap/GC machinery via LANG16 (no per-language GC).
- Reuse the linkable runtime via LANG15 (no per-language ABI fork).

---

## Architecture

```text
┌──────────────────────────────────────────────────────────────────────────────┐
│                         Language frontends (per-language)                    │
│                                                                              │
│  twig    lisp    scheme    clojure    ruby    js    smalltalk    perl   …    │
│   │        │       │         │         │      │       │           │          │
│   └────────┴───────┴─────────┘         │      │       │           │          │
│                │                       │      │       │           │          │
│            (lispy AST)             (ruby AST)(JS AST)(STalk AST)(Perl AST)   │
│                │                       │      │       │           │          │
│  twig-ir-     ...                  ruby-ir   js-ir   ...        ...          │
│  compiler                          compiler  compiler                        │
│         \____________________________|______|_______|____________/           │
│                                      │                                       │
│                                      ▼                                       │
│                               ┌─────────────┐                                │
│                               │  IIRModule  │ ◄─ universal bytecode (LANG01) │
│                               └──────┬──────┘                                │
│                                      │                                       │
│       ┌──────────────────────────────┼──────────────────────────────┐        │
│       ▼                              ▼                              ▼        │
│  ┌──────────┐                  ┌──────────┐                  ┌──────────┐    │
│  │ vm-core  │                  │ jit-core │                  │ aot-core │    │
│  │ (LANG02) │ ◄── feedback ──► │ (LANG03) │ ◄── profiles ──► │ (LANG04) │    │
│  │ interp   │     slots         │  JIT     │                 │   AOT    │    │
│  └────┬─────┘                  └────┬─────┘                  └────┬─────┘    │
│       │                             │                             │          │
│       └─────────────────────────────┼─────────────────────────────┘          │
│                                     ▼                                        │
│                          ┌────────────────────┐                              │
│                          │  LangBinding<L>    │ ◄── per-language trait impl  │
│                          │  trait (LANG20)    │                              │
│                          └─────────┬──────────┘                              │
│                                    │                                         │
│      ┌─────────────┬───────────────┼───────────────┬──────────────┐          │
│      ▼             ▼               ▼               ▼              ▼          │
│  ┌────────┐  ┌──────────┐  ┌────────────┐  ┌──────────────┐ ┌─────────┐      │
│  │ lispy- │  │ ruby-    │  │ js-runtime │  │ smalltalk-   │ │ perl-   │      │
│  │runtime │  │ runtime  │  │            │  │ runtime      │ │ runtime │      │
│  └───┬────┘  └────┬─────┘  └─────┬──────┘  └──────┬───────┘ └────┬────┘      │
│      │            │              │                │              │           │
│      └────────────┴──────────────┼────────────────┴──────────────┘           │
│                                  ▼                                           │
│                     ┌────────────────────────────┐                           │
│                     │   lang-runtime-core        │ ◄── truly generic         │
│                     │   ─ GC (LANG16)            │                           │
│                     │   ─ allocator              │                           │
│                     │   ─ safepoints             │                           │
│                     │   ─ write barriers         │                           │
│                     │   ─ stack maps             │                           │
│                     │   ─ root-scanning          │                           │
│                     │   ─ symbol intern          │                           │
│                     │   ─ inline cache infra     │                           │
│                     │   ─ deopt protocol         │                           │
│                     │   ─ C ABI (LANG15)         │                           │
│                     └────────────────────────────┘                           │
└──────────────────────────────────────────────────────────────────────────────┘
```

Two rules govern the layering:

1. **Mechanism vs policy.**  `lang-runtime-core` and the IIR opcodes
   express *mechanism* (allocate, write-barrier, dispatch-indirect,
   safepoint).  Policy (what `cons` does, how a Ruby method is looked
   up, what `Object.keys` means) lives in `<lang>-runtime` and is
   reached via `LangBinding`.
2. **The feedback slot is the universal unit of optimisation.**  Every
   IIR instruction that does runtime polymorphic work has a slot.  The
   interpreter writes; the JIT reads and speculates; the AOT consumes
   recorded profiles or, absent a profile, treats the slot as
   uninitialised.  Tiers communicate through slots, not through tier-
   specific side channels.

> The diagram above shows the **LANG-runtime path** only — it is not the
> only path from IIR to execution.  The next section ("Compilation
> paths") makes the orthogonality with host-runtime backends (JVM, CLR,
> BEAM, WASM) explicit.

---

## Compilation paths: LANG runtime vs. host runtimes

LANG20 specifies the architecture for the **LANG-runtime path** —
where IIR is executed by `vm-core` / `jit-core` / `aot-core` against
a `LangBinding` and the heap/GC machinery in `lang-runtime-core` (per
LANG16).  This is the V8 × GraalVM target.

But IIR is **also** a portable interchange format that lowers to
*host* runtimes — JVM, CLR, BEAM, WebAssembly — through dedicated
backends that bypass the LANG runtime entirely.  The existing crates
`twig-jvm-compiler`, `twig-clr-compiler`, `twig-beam-compiler`,
`twig-jit-wasm`, and the generic `ir-to-jvm-class-file`,
`ir-to-cil-bytecode`, `ir-to-beam`, `ir-to-wasm-compiler` already do
this for Twig today.  LANG20 explicitly preserves this divergence —
both paths must remain first-class.

### Two paths from one IIR

```text
       Twig / Lisp / Ruby / JS / Smalltalk / Perl source
                              │
                              ▼
                          typed AST
                              │
                              ▼
                         IIRModule  ◄─ universal interchange
                              │
       ┌──────────────────────┴────────────────────────┐
       │                                               │
       ▼                                               ▼
┌──────────────────────────┐                ┌────────────────────────────┐
│   LANG-runtime path      │                │    Host-runtime path       │
│   (LANG20)               │                │    (existing per-target)   │
│                          │                │                            │
│   vm-core / jit-core /   │                │  ir-to-jvm-class-file      │
│   aot-core               │                │    → real `java`           │
│                          │                │  ir-to-cil-bytecode        │
│   + lang-runtime-core    │                │    → real `dotnet`         │
│     (GC, IC, deopt,      │                │  ir-to-beam                │
│      stack maps, ABI)    │                │    → real `erl`            │
│                          │                │  ir-to-wasm-compiler       │
│   + LangBinding          │                │    → real WASM runtime     │
│                          │                │                            │
│   Heap, GC, JIT, AOT     │                │  Host JVM/CLR/BEAM/WASM    │
│   are OUR code           │                │  handles GC, dispatch,     │
│                          │                │  class loading, JIT, etc.  │
└──────────────────────────┘                └────────────────────────────┘
```

### Which IIR features each path consumes

| IIR feature | LANG-runtime path | Host-runtime path |
|-------------|:-----------------:|:-----------------:|
| LANG01 base opcodes (`const`, `add`, `cmp_*`, `call`, `call_builtin`, `jmp_*`, `ret`, `label`, `load_mem`, `store_mem`, …) | ✓ | ✓ |
| LANG01 type hints (`u8`, `u32`, `bool`, `any`, …) | ✓ (drives JIT specialisation) | ✓ (drives backend type lowering) |
| LANG01 `observed_slot` (feedback) | ✓ (drives JIT) | ⌀ (host runtime profiles itself) |
| LANG16 alloc opcodes (`alloc`, `box`, `unbox`, `field_load`, `field_store`, `is_null`, `safepoint`) | ✓ (calls `lang-runtime-core` GC) | ✓ (lowers to host `new`, `getfield`, `putfield`, …) |
| LANG16 `ref<T>` type | ✓ (heap handle in our GC) | ✓ (lowers to host reference type) |
| LANG20 `send` / `load_property` / `store_property` opcodes | ✓ (calls `LangBinding`) | ✓ (lowers to `invokevirtual` / `callvirt` / `apply` / `call_indirect`) |
| LANG20 `ic_slot` field | ✓ (drives IC machinery) | ⌀ (host runtime has its own ICs) |
| LANG20 frame descriptors / deopt anchors | ✓ (`rt_deopt`) | ⌀ (host runtime handles its own deopt) |
| LANG20 feedback-slot taxonomy (CallSiteSlot, MethodSiteSlot, …) | ✓ (JIT + AOT-PGO consume) | ⌀ (host JIT profiles itself) |

The host-runtime path **ignores** the LANG-runtime-only fields
(feedback slots, IC slots, deopt anchors, frame descriptors).  These
are populated when a frontend wants the LANG-runtime path's tier-up
benefits and cost nothing to ignore — every host backend just walks
the IIR's primary opcode/operand structure and never touches the
profiling side-tables.

### Why both paths matter

- **The LANG-runtime path** delivers the V8 × GraalVM combination:
  tiered execution, closed-world AOT, every language gets all tiers
  from one implementation.  Best when there is no good host runtime
  for the target environment — embedded, constrained ISAs like Intel
  4004 / Intel 8080 / RV32, custom hardware, bare metal.
- **The host-runtime path** lets a Twig program ship as a `.jar` that
  runs on production JVMs (HotSpot's GC, C2 JIT, profiler, the entire
  observability ecosystem), or as a `.beam` that runs on Erlang's
  BEAM (preemptive scheduling, distributed messaging, hot code
  reload), or as a `.dll` on .NET (CoreCLR's tiered compilation,
  NuGet ecosystem), or as `.wasm` (browser, edge runtimes,
  capability-secure sandboxes).  Best when an existing runtime
  offers something we don't want to rebuild.

A frontend **does not have to choose**.  Twig already targets both:

| Twig spec | Path | Status |
|-----------|------|--------|
| TW00 | LANG-runtime (vm-core today; jit/aot tomorrow) | Lexer/parser/IR-compiler shipped (PR #1741) |
| TW02 | Host JVM (`twig-jvm-compiler` → real `java`) | Shipped |
| TW03 | Host CLR / BEAM / WASM cross-backend roadmap | In progress |
| TW04 (future) | Host WASM with custom GC | Planned per TW03 |

The same IIR feeds every backend.  Adding a Ruby frontend automatically
gives it both paths: the LANG-runtime path (via `RubyBinding`) *and* the
JVM path (via `ir-to-jvm-class-file` lowering ruby's IIR to JRuby-style
bytecode), without writing path-specific frontend code.

### Constraint LANG20 imposes on its own additions

To keep both paths first-class, LANG20 imposes one rule on every
opcode, field, or convention it adds:

> **LANG20-specific IIR additions must lower cleanly to host runtimes
> via existing backend mechanisms (`invokevirtual`, `callvirt`,
> `apply` on BEAM, `call_indirect` on WASM) without requiring the
> host backend to understand IC, deopt, or profile-feedback
> machinery.**

Concretely the three new opcodes (§"IIR additions") lower as:

| LANG20 opcode | JVM (`ir-to-jvm-class-file`) | CLR (`ir-to-cil-bytecode`) | BEAM (`ir-to-beam`) | WASM (`ir-to-wasm-compiler`) |
|---------------|------------------------------|----------------------------|---------------------|------------------------------|
| `send recv sel args…` | `invokevirtual` (selector → method ref) | `callvirt` (selector → MethodRef) | apply (selector → atom) | `call_indirect` (selector → table index) |
| `load_property obj key` | `getfield` or `invokevirtual getXxx` | `ldfld` or `callvirt get_Xxx` | `element/2` or map-get | struct.get (WASM-GC) or memory-load + offset table |
| `store_property obj key val` | `putfield` or `invokevirtual setXxx` | `stfld` or `callvirt set_Xxx` | map-put / record update | struct.set or memory-store |

`apply_callable` (the `call_indirect` opcode's runtime semantics)
lowers to JVM `invokeinterface`, CLR `calli`, BEAM apply,
WASM `call_indirect`.  Frame descriptors / feedback slots / IC slots
are simply not emitted into the host artefact.

If a future LANG20 extension genuinely cannot lower to host runtimes,
it goes behind an explicit `IIRModule.requires_lang_runtime: bool`
flag that host backends refuse.  But the rule should hold by
construction — every LANG20 addition is an *optimisation
opportunity* the LANG-runtime path takes and that host runtimes
ignore (or implement themselves through their own mechanisms).

### Where each path's spec lives

| Concern | LANG-runtime path spec | Host-runtime path spec |
|---------|------------------------|------------------------|
| Interpreter | LANG02 (vm-core) | n/a — host runtime |
| JIT | LANG03 (jit-core) | n/a — host JIT |
| AOT | LANG04 (aot-core) | per-target (JVM02, CLR01, BEAM01, …) |
| GC | LANG16 (gc-core) | host GC (JVM, CLR, BEAM, WASM-GC) |
| IC + deopt | LANG20 (this spec) | n/a — host handles |
| Calling convention | LANG15 (vm-runtime C ABI), LANG20 §"C ABI extensions" | per-target host ABI (JVM stack, CLR stack, BEAM registers, WASM linear stack) |

LANG20 does **not** spec the host-runtime path; per-target specs (TW02
for JVM, CLR01 for CLR, BEAM01 for BEAM, the future TW04 for WASM)
already do.  This document only commits to *not breaking* the
host-runtime path with anything it adds.

---

## Part 1: The `LangBinding` trait

A language frontend pairs an IIR-emitting compiler with a
`LangBinding<L>` impl.  The binding is a Rust trait (and a matching
C-side function table for AOT/JIT consumption) whose surface is
deliberately small but comprehensive.

### Required associated types

```rust
pub trait LangBinding: 'static + Sync + Send {
    /// The ABI-stable value representation.
    ///
    /// Must be `Copy` and the size of a machine word (`u64` on
    /// 64-bit targets).  See §"Cross-language value representation"
    /// for the encoding contract.
    type Value: Copy + 'static;

    /// Per-language opaque object class identifier.  Recorded in
    /// every heap object header; consumed by the GC's trace dispatch.
    type ClassRef: Copy + Eq + Hash + 'static;

    /// Per-language inline-cache entry.  See §"Inline cache machinery".
    type ICEntry: Copy + 'static;

    /// Stable language identifier, used in profile files, debug
    /// dumps, and IIRModule.language.  Lower-snake-case.
    const LANGUAGE_NAME: &'static str;
}
```

### Required methods (15)

The trait surface is sized so that:

- **No method is optional in the way that breaks correctness.**
  Every binding must implement every method.
- **Many methods have a default impl** that is correct for
  Lispy/dynamic languages.  Languages with simpler models override
  for performance.

```rust
pub trait LangBinding: /* (see above) */ {
    // ─── Type & identity ─────────────────────────────────────────────

    /// Return the per-language type tag for `value`.
    ///
    /// Used by the profiler to record observed types in feedback
    /// slots and by `==` style operations.  Must be cheap (single
    /// instruction or a table lookup).
    fn type_tag(value: Self::Value) -> u32;

    /// Return the class of `value`, or `None` if `value` is an
    /// immediate (tagged int, nil, bool — anything without a heap
    /// header).  Used by IC keying and by reflection.
    fn class_of(value: Self::Value) -> Option<Self::ClassRef>;

    /// Truthiness for `if` / `jmp_if_*`.
    ///
    /// The IIR's `jmp_if_false` opcode delegates here.  Languages
    /// vary: Scheme treats only `#f` as false; Python treats `0`,
    /// empty containers, `None` as false; Ruby treats `nil` and
    /// `false` as false.  Default impl: `true` (override required
    /// for correctness in any real language).
    fn is_truthy(value: Self::Value) -> bool {
        let _ = value;
        true
    }

    /// Structural equality (`equal?` in Scheme; `==` in Ruby; `===`
    /// in JS).  Used by IIR `cmp_eq`.
    fn equal(a: Self::Value, b: Self::Value) -> bool;

    /// Identity equality (`eq?` in Scheme; `equal?` in Ruby;
    /// `Object.is` in JS).  Default: bitwise equality of value words.
    fn identical(a: Self::Value, b: Self::Value) -> bool {
        let a_bits: u64 = unsafe { std::mem::transmute_copy(&a) };
        let b_bits: u64 = unsafe { std::mem::transmute_copy(&b) };
        a_bits == b_bits
    }

    /// Hash for keying (used by IC keying, hash maps).
    fn hash(value: Self::Value) -> u64;

    // ─── Heap interaction (delegates to gc-core / LANG16) ────────────

    /// Walk all `Value`s reachable from this object, calling
    /// `visitor.visit_value(v)` on each.  Called by the collector
    /// during tracing.
    ///
    /// `obj_header` points at a heap object whose class matches a
    /// kind this binding registered.  The binding decodes the
    /// payload layout and visits every reference field.
    fn trace_object(obj_header: ObjectHeader, visitor: &mut dyn ValueVisitor);

    /// Walk references reachable from a `Value` directly (for tagged
    /// immediates this is a no-op; for heap-backed values it
    /// dereferences and calls `trace_object`).  Invoked by the GC
    /// when scanning roots.
    fn trace_value(value: Self::Value, visitor: &mut dyn ValueVisitor);

    /// Optional: object finalizer.  Called at most once when the
    /// object becomes unreachable.  Default: no-op.
    fn finalize(_obj_header: ObjectHeader) {}

    // ─── Dispatch (the polymorphic seam) ─────────────────────────────

    /// Apply a callable value to argument values.  Used by IIR
    /// `call_indirect`.  This is where `apply_closure` semantics
    /// live (lookup the closure, prepend captured env, call inner
    /// IIRFunction).
    ///
    /// Returns either the result value or a runtime error (which the
    /// interpreter raises as a language-level exception per the
    /// binding's own exception model).
    fn apply_callable(
        callable: Self::Value,
        args: &[Self::Value],
        cx: &mut DispatchCx<'_, Self>,
    ) -> Result<Self::Value, RuntimeError>;

    /// Look up a method on a receiver and invoke it.  Used by IIR
    /// `send` / `invoke` opcodes (added in LANG20).  `selector` is
    /// an interned symbol id.
    ///
    /// Languages without method dispatch (pure Lispy languages)
    /// can panic-default this; Ruby/Smalltalk/JS implement it.
    fn send_message(
        receiver: Self::Value,
        selector: SymbolId,
        args: &[Self::Value],
        ic: &mut InlineCache<Self::ICEntry>,
        cx: &mut DispatchCx<'_, Self>,
    ) -> Result<Self::Value, RuntimeError>;

    /// Read an object property by symbol.  Used by IIR `load_property`
    /// (added in LANG20).
    fn load_property(
        obj: Self::Value,
        key: SymbolId,
        ic: &mut InlineCache<Self::ICEntry>,
    ) -> Result<Self::Value, RuntimeError>;

    /// Write an object property by symbol.  Used by IIR
    /// `store_property` (added in LANG20).
    fn store_property(
        obj: Self::Value,
        key: SymbolId,
        val: Self::Value,
        ic: &mut InlineCache<Self::ICEntry>,
    ) -> Result<(), RuntimeError>;

    // ─── Builtins ────────────────────────────────────────────────────

    /// Resolve a builtin by name.  Returns a stable function pointer
    /// the JIT/AOT can emit a direct call to.  Called once per name
    /// at link time (LANG10 linker / LANG15 §"relocation contract").
    fn resolve_builtin(name: &str) -> Option<BuiltinFn<Self>>;

    // ─── Deopt support ───────────────────────────────────────────────

    /// Materialize a `Value` from its specialized native
    /// representation, given the deopt frame descriptor's record
    /// of how it was stored (e.g. unboxed `i64` in register R5).
    /// Called on deopt to reconstruct the interpreter frame.  See
    /// §"Deopt protocol" for the descriptor format.
    fn materialize_value(
        repr: BoxedReprToken,
        location_value: u64,
    ) -> Self::Value;

    /// Inverse: produce the specialised native representation of a
    /// `Value` so a re-entered JIT frame can place it in registers.
    /// Most languages return a token that says "boxed reference;
    /// store the pointer".
    fn box_value(value: Self::Value) -> (BoxedReprToken, u64);
}
```

### Why these specific 15 methods

Each method maps to a *runtime mechanism* the IIR must dispatch through.
Adding more would creep semantics into the trait; removing any forces
language-specific code into `lang-runtime-core` where it doesn't belong.

| Method | Mechanism it serves | IIR opcodes that call it |
|--------|---------------------|--------------------------|
| `type_tag` | profiler | every typed instruction's feedback record |
| `class_of` | IC keying | `send_message`, `load/store_property` |
| `is_truthy` | branching | `jmp_if_false`, `jmp_if_true` |
| `equal` | structural eq | `cmp_eq` |
| `identical` | identity eq | `cmp_is` (added in LANG20) |
| `hash` | keying | hash-map builtins |
| `trace_object` | GC tracing | invoked by collector, not IIR |
| `trace_value` | GC root scan | invoked by collector |
| `finalize` | GC finalization | invoked by collector |
| `apply_callable` | indirect call | `call_indirect`, `apply` |
| `send_message` | method dispatch | `send` (added in LANG20) |
| `load_property` | dynamic load | `load_property` (added in LANG20) |
| `store_property` | dynamic store | `store_property` (added in LANG20) |
| `resolve_builtin` | linker | resolved at LANG10 link time |
| `materialize_value` | deopt | invoked by `rt_deopt` |
| `box_value` | re-enter JIT | invoked when calling JIT-compiled code |

### Bindings can pair up

Two bindings can share state (a Lisp + Scheme tandem can share a symbol
table; a Ruby + JS interop layer can share an exception model).  The
trait is `Sync + Send` and bindings hold their state in static globals
or via a context handle the runtime threads through dispatch calls.

### Example: `LispyBinding`

`lispy-runtime` provides a single binding consumed by Twig, Lisp,
Scheme, and Clojure frontends:

```rust
pub struct LispyBinding;

impl LangBinding for LispyBinding {
    type Value = LispyValue;            // tagged i64
    type ClassRef = LispyClass;         // enum of {Cons, Symbol, Closure, ...}
    type ICEntry = LispyICEntry;        // (type_tag, handler_ptr)
    const LANGUAGE_NAME: &'static str = "lispy";

    fn type_tag(v: LispyValue) -> u32 { v.tag() as u32 }
    fn class_of(v: LispyValue) -> Option<LispyClass> { v.class() }
    fn is_truthy(v: LispyValue) -> bool { !v.is_false() && !v.is_nil() }
    fn equal(a: LispyValue, b: LispyValue) -> bool { lispy_equal(a, b) }
    fn hash(v: LispyValue) -> u64 { lispy_hash(v) }

    fn trace_object(h: ObjectHeader, vis: &mut dyn ValueVisitor) {
        match h.class::<LispyClass>() {
            LispyClass::Cons => {
                let cell = h.cast::<ConsCell>();
                vis.visit_value(cell.car);
                vis.visit_value(cell.cdr);
            }
            LispyClass::Closure => {
                let clos = h.cast::<Closure>();
                for cap in clos.captures() {
                    vis.visit_value(*cap);
                }
            }
            LispyClass::Symbol => { /* interned: no internal refs */ }
        }
    }

    fn trace_value(v: LispyValue, vis: &mut dyn ValueVisitor) {
        if let Some(h) = v.as_object_header() {
            Self::trace_object(h, vis);
        }
    }

    fn apply_callable(callable: LispyValue, args: &[LispyValue], cx: &mut _) -> _ {
        let clos = callable.as_closure().ok_or(RuntimeError::NotCallable)?;
        let combined: Vec<LispyValue> = clos.captures().iter().chain(args).copied().collect();
        cx.call_iir_function(clos.fn_name(), &combined)
    }

    // send_message / load_property / store_property panic — Lispy
    // doesn't dispatch by symbol on receivers.

    fn resolve_builtin(name: &str) -> Option<BuiltinFn<Self>> {
        match name {
            "+"     => Some(builtin_add),
            "cons"  => Some(builtin_cons),
            "car"   => Some(builtin_car),
            "cdr"   => Some(builtin_cdr),
            // …
            _ => None,
        }
    }

    fn materialize_value(repr: BoxedReprToken, raw: u64) -> LispyValue {
        match repr {
            BoxedReprToken::I64Unboxed => LispyValue::int(raw as i64),
            BoxedReprToken::F64Unboxed => LispyValue::flonum(f64::from_bits(raw)),
            BoxedReprToken::BoxedRef   => LispyValue::from_handle(raw),
        }
    }

    fn box_value(v: LispyValue) -> (BoxedReprToken, u64) {
        if let Some(n) = v.as_int() { (BoxedReprToken::I64Unboxed, n as u64) }
        else                         { (BoxedReprToken::BoxedRef, v.bits()) }
    }
}
```

A Lisp / Scheme / Clojure / Twig frontend therefore writes **zero**
runtime code — they all import `lispy-runtime`'s `LispyBinding` and
focus on AST → IIR.

---

## Part 2: Cross-language value representation

### The ABI contract

Every `Value` is a **single 64-bit word** when it crosses tier boundaries.
This is not a Rust requirement — it's an ABI requirement so JIT-emitted
machine code can pass values through registers and so AOT-compiled code
linked into another binary can interop.

`LangBinding::Value` therefore must be `Copy + 'static` and **must have
size 8** on 64-bit targets.  The runtime asserts this at `LangBinding`
registration time:

```rust
pub fn register_binding<L: LangBinding>() {
    const _: () = assert!(std::mem::size_of::<L::Value>() == 8);
    // …
}
```

### Per-language encodings (all valid)

| Language family | Encoding | Why |
|------|----------|-----|
| Lispy (Lisp/Scheme/Twig/Clojure) | Tagged i64: low 3 bits = tag, high 61 = payload | TW00 spec; tagged immediates for int/bool/nil; heap handles for cons/sym/closure |
| JavaScript | NaN-boxing: f64 with NaN-payload type discriminator | V8 / SpiderMonkey convention; numbers are unboxed f64 |
| Smalltalk | Pointer with low-bit SmallInteger tag | Squeak convention; class pointer in object header |
| Ruby | `VALUE` = pointer or tagged immediate (Fixnum/Symbol/Flonum/special) | MRI convention; matches CRuby ABI |
| Perl | `SV*` pointer; immediates rare | CPerl convention; refcounting handled by runtime |
| Tetrad (statically typed) | Plain machine word, type known at compile time | LANG02 baseline |

### Heap object header (uniform across languages)

Every heap object — cons cell, JS object, Ruby `RObject`, Smalltalk
`Object` — carries the **same 16-byte header** before the language-
specific payload:

```text
┌────────────────────────┬────────────┬──────────────┬────────────┐
│  class_or_kind: u32    │ gc_word: u32│ size_bytes: u32 │ flags: u32 │
└────────────────────────┴────────────┴──────────────┴────────────┘
                              ↓
                    language-specific payload
```

- `class_or_kind`: opaque per-language id (registered with the
  collector via `LangBinding::ClassRef`).  The GC uses this to dispatch
  trace through `LangBinding::trace_object`.
- `gc_word`: collector's bookkeeping (mark bit, age, forwarding ptr).
- `size_bytes`: object size; needed for sweep, copy GC, finalization.
- `flags`: 32 bits; lower 8 reserved by `lang-runtime-core`
  (has_finalizer, is_immortal, …); upper 24 free for language use.

This header is a **non-negotiable ABI commitment**.  Cross-language
heap sharing (a Ruby program embedding a JS regex object, say) only
works because every language's GC agrees on the header layout.

A binding that needs less header overhead (Tetrad, which has no GC)
opts out of the heap entirely — its `Value` is a primitive type and
its `trace_object` is unreachable.

### What `gc-core` sees

`gc-core` (LANG16) operates entirely on headers + the binding's
`trace_object`.  It never decodes payload.  The same mark-and-sweep,
copying, or generational collector therefore works for every language
without per-language plumbing.

---

## Part 3: Feedback-slot taxonomy

The IIR (LANG01) already carries `observed_slot: SlotState` on every
instruction.  LANG20 enumerates **what each slot stores per opcode**
so every tier reads the same shape.

### Slot kinds

| Slot kind | Stored data | Opcodes that own it |
|-----------|-------------|---------------------|
| `CallSiteSlot` | observed callee identity (function name or closure shape) | `call`, `call_indirect`, `call_builtin` |
| `MethodSiteSlot` | observed (receiver class, method version, target ptr) tuples | `send` (added in LANG20) |
| `LoadSiteSlot` | observed (receiver class, property offset) tuples | `load_property`, `load_field` |
| `StoreSiteSlot` | observed (receiver class, property offset, value type) | `store_property`, `field_store` |
| `ArithSiteSlot` | observed operand types (left, right) | `add`, `sub`, `mul`, `div`, `mod`, `cmp_*` |
| `BranchSiteSlot` | taken/not-taken counts; observed truthiness pattern | `jmp_if_false`, `jmp_if_true` |
| `AllocSiteSlot` | observed allocation count, age-out hint | `alloc`, `box`, `make_closure` |
| `TypeSiteSlot` | observed type tag of a single value | `is_*` predicates, `type_of`-like ops |

Each slot kind is a tagged enum variant of `FeedbackPayload`, carried
inside `SlotState` (LANG01).  The state machine
(UNINIT → MONO → POLY → MEGA) is unchanged from LANG01; LANG20 only
specifies what `payload` field each kind populates.

### Slot lifecycle

```text
┌─────────────────┐  first observation  ┌──────────────┐
│   UNINIT        │ ───────────────────►│  MONOMORPHIC │
└─────────────────┘                     └──────┬───────┘
                                               │ second distinct observation
                                               ▼
                                       ┌──────────────┐  Kth+1 observation
                                       │ POLYMORPHIC  │ ──────────────────►┐
                                       │  (≤K shapes) │                    │
                                       └──────────────┘                    ▼
                                                                  ┌──────────────┐
                                                                  │ MEGAMORPHIC  │
                                                                  │  (terminal)  │
                                                                  └──────────────┘
```

`K` is per-slot-kind (LANG20-recommended defaults: K=4 for `MethodSiteSlot`
and `LoadSiteSlot` matching V8's PIC width; K=2 for `ArithSiteSlot`;
K=1 — i.e. binary mono/mega — for `BranchSiteSlot`).

### How each tier uses slots

| Tier | When | What it does |
|------|------|--------------|
| Interpreter (vm-core) | Every instruction execution where the slot's owning op is dispatched | Calls `LangBinding::record_feedback(slot, observation)` to update; cost is one branch + one store in the common (already-mono) case |
| JIT (jit-core) | When promoting a hot function | Reads each slot to decide speculation: MONO → emit guard + specialised + deopt anchor; POLY → emit dispatch table; MEGA → emit generic; UNINIT → emit generic and mark for re-promotion |
| AOT-PGO (aot-core) | When compiling with a recorded profile | Treats each slot as if observed in the profile (replay) |
| AOT-cold (aot-core) | When compiling without a profile | Treats every slot as UNINIT (conservative) |

### Recording from the interpreter

`vm-core`'s dispatch handlers call:

```rust
fn record_feedback<L: LangBinding>(
    slot: &mut SlotState,
    observation: FeedbackPayload<L>,
) {
    slot.advance(observation);
}
```

The handler that owns a slot constructs its observation:

- `handle_call_indirect` records the callable's identity (its
  `ClassRef`) so the JIT can devirtualise.
- `handle_send` records `(receiver_class, method_version, resolved_target)`
  so the JIT can emit a class-keyed cache.
- `handle_arith_*` records `(left_type_tag, right_type_tag)` so the
  JIT can emit unboxed arithmetic with type guards.

Recording is skipped when the instruction's `type_hint` is concrete
(LANG01 invariant): typed code pays no profiling cost.

### Profile artefact format (for AOT-PGO)

A recorded profile is a flat binary file alongside the IIR module:

```text
profile_v1:
  header (16 bytes):
    magic       0x50 0x52 0x4f 0x46  ("PROF")
    version     0x01 0x00
    flags       u16
    entry_count u32
    string_off  u32
  entries (entry_count × 24 bytes):
    fn_name_off  u32     // name in string pool
    iir_index    u32     // instruction index in fn
    slot_kind    u8      // FeedbackPayload variant
    payload      u11×    // variant-specific
  string pool (variable)
```

The format is intentionally architecture-independent so a profile
collected on x86_64 can drive AOT for ARM or RISC-V.

---

## Part 4: Deopt protocol

### When deopt fires

| Trigger | Cause |
|---------|-------|
| Type guard failure | Value didn't match speculation (e.g. `add_u8` saw a string) |
| Method-version mismatch | Class was reopened; cached method ptr is stale |
| Hidden-class mismatch | Object's shape changed since the IC was warmed |
| Soft deopt | Debugger / GC / collector requests it |
| Hard deopt | Unrecoverable speculation (dead branch, bogus assumption) |

All five take the same path: the JIT/AOT-emitted code calls
`rt_deopt(anchor_id, native_frame_ptr)`.

### Frame descriptor

When the JIT/AOT compiler emits a guard, it publishes a **frame
descriptor** at the deopt anchor's IIR index:

```rust
pub struct FrameDescriptor {
    /// IIR instruction index to resume the interpreter at.
    pub ir_index: u32,

    /// One entry per IIR variable that was live at this point.
    pub registers: Vec<RegisterEntry>,
}

pub struct RegisterEntry {
    /// IIR variable name (matches `IIRInstr::dest` somewhere).
    pub ir_name: String,

    /// Where the value lives in the native frame.
    pub location: NativeLocation,

    /// How the value is encoded (boxed, unboxed primitive, etc.).
    pub repr: BoxedReprToken,

    /// Type tag the JIT speculated; useful for the binding's
    /// materialise_value to validate.
    pub type_tag: u32,
}

pub enum NativeLocation {
    /// Value lives in this hardware register.
    Register(u8),
    /// Value lives at this offset from the frame pointer.
    StackSlot(i32),
    /// Value is a constant the JIT inlined.
    Constant(u64),
}

pub enum BoxedReprToken {
    /// Value is a fully boxed `LangBinding::Value` (just store the bits).
    BoxedRef,
    /// Value is an unboxed signed integer.
    I64Unboxed,
    /// Value is an unboxed double.
    F64Unboxed,
    /// Value is an unboxed bool (low bit).
    BoolUnboxed,
    /// Value is a derived pointer (e.g. interior cons-cdr ptr); rebuild
    /// from base via the offset.
    DerivedPtr { base_register: u8, offset: i32 },
}
```

Frame descriptors are stored in a side table indexed by deopt anchor
id.  The JIT publishes descriptors when it emits the corresponding
guard; the AOT compiler emits a `FrameDescriptor` section in the
`.aot` file (LANG04 §"snapshot format").

### Materialisation

`rt_deopt` walks the descriptor:

```rust
extern "C" fn rt_deopt(anchor_id: u32, native_frame: *mut u8) -> ! {
    let desc = lookup_frame_descriptor(anchor_id);
    let mut frame = VMFrame::new(/* …function metadata… */);
    for entry in &desc.registers {
        let raw = read_native_location(native_frame, entry.location);
        let value = LangBinding::materialize_value(entry.repr, raw);
        frame.assign(&entry.ir_name, value);
    }
    frame.ip = desc.ir_index;
    install_interpreter_frame_and_resume(frame);
}
```

The interpreter resumes at `ir_index` with `frame.assign` populating
register names from the materialised values.  The JIT's specialised
frame is dropped — its registers are now garbage as far as the
runtime is concerned.

### Inlined-call deopt

When the JIT inlines callee frames into the caller's native code, a
single deopt may need to materialise **multiple** interpreter frames.
The descriptor format supports this by storing a `Vec<FrameDescriptor>`
per anchor — one per inlined frame, in caller→callee order.  `rt_deopt`
materialises them bottom-up and pushes them onto the interpreter's
frame stack before resuming.

### Cross-tier transitions

| From | To | Mechanism |
|------|----|-----------|
| Interp → JIT | Promotion threshold crossed | jit-core compiles, replaces fn pointer; next call dispatches to native |
| Interp → AOT | Always (AOT binary) | AOT entry point; vm-runtime present for fallback only |
| JIT → Interp | Deopt | `rt_deopt` (this section) |
| AOT → Interp | Deopt | `rt_deopt` (same protocol) |
| Interp → AOT | Re-entry to AOT after deopt | `rt_re_enter_specialised(fn_index, frame)` calls `LangBinding::box_value` for each register |
| Interp → JIT | Re-entry to JIT after deopt | Same path; `rt_re_enter_specialised` |

### Why this protocol works for any language

The descriptor stores `BoxedReprToken` and the binding owns
`materialize_value`.  A Lispy binding decodes
`I64Unboxed` → `LispyValue::int(raw)`.  A JS binding decodes
`F64Unboxed` → `JsValue::number(f64::from_bits(raw))`.  The runtime
doesn't need to know how either encoding works — it just calls the
binding.

---

## Part 5: Inline cache machinery

V8's secret weapon is the inline cache: a per-call-site / per-load-site
small cache that records observed shapes and emits direct loads/calls
when the shape matches.  Without ICs, every dynamic dispatch hits a
hash-table lookup.  With ICs, a hot polymorphic site is a few CMP +
JMP instructions.

LANG20 generalises ICs across languages.

### Generic IC types

```rust
pub struct InlineCache<E: Copy> {
    pub entries: [Option<E>; MAX_PIC_ENTRIES],   // typically 4
    pub state: ICState,
    pub hit_count: u32,
    pub miss_count: u32,
}

#[derive(Copy, Clone)]
pub enum ICState {
    Uninit,
    Monomorphic,
    Polymorphic,
    Megamorphic,
}

pub trait LangBinding {
    type ICEntry: Copy + 'static;
    /* … */
}
```

The IC instance lives in a side table next to the IIRFunction (one
per IC-bearing instruction).  The JIT emits inline checks against
its entries; on miss, it falls back to a runtime helper that updates
the cache via the binding.

### Per-language IC entry shapes

| Language | `ICEntry` | Hot path emitted by JIT |
|----------|-----------|--------------------------|
| Lispy | `(type_tag: u32, handler: fn ptr)` | `cmp r_value.tag, ENTRY.tag; je ENTRY.handler` |
| JS (V8-style) | `(hidden_class_id: u32, offset_or_method: u32)` | `cmp r_obj.hc_id, ENTRY.hc_id; je load+offset` |
| Smalltalk PIC | `(receiver_class: u32, method_addr: usize)` | `cmp r_recv.class, ENTRY.class; je ENTRY.method_addr` |
| Ruby | `(receiver_class: u32, method_version: u16, target: usize)` | `cmp [class, version], [ENTRY.class, ENTRY.ver]; je ENTRY.target` |
| Perl | `(package_id: u32, sub_addr: usize)` | `cmp r_pkg, ENTRY.pkg; je ENTRY.sub` |

The fact that all five fit `≤ 16 bytes` and all five emit
`compare-and-jump` sequences is why the same generic IC infrastructure
serves everyone.

### IC state transitions

- **Uninit** → first call: install entry, advance to **Monomorphic**.
- **Mono** → second call:
  - same shape: hit; increment counter.
  - different shape: install second entry, advance to **Polymorphic**.
- **Poly** → call:
  - cached shape: hit.
  - new shape, ≤ MAX_PIC_ENTRIES: install entry.
  - new shape, > MAX_PIC_ENTRIES: advance to **Megamorphic**.
- **Mega** → fall back to `LangBinding::send_message` /
  `load_property` etc. for every call; no caching.

### IC invalidation

Class redefinition (Ruby reopens, JS prototype changes, Smalltalk
`become:`, Perl `*foo = sub { … }`) makes cached entries stale.  The
binding owns the invalidation rule:

```rust
pub trait LangBinding {
    /// Called by the runtime after the binding has performed a class
    /// or method change that invalidates ICs.  The binding's own
    /// state-tracking determines which IC ids to invalidate.
    fn invalidate_ics(&self, invalidator: &mut dyn ICInvalidator);
}

pub trait ICInvalidator {
    fn invalidate_ic(&mut self, ic_id: ICId);
    fn invalidate_class(&mut self, class: ClassId);
}
```

`invalidate_class` is the bulk hammer (every IC keyed on this class is
reset to Uninit); `invalidate_ic` is targeted (one specific IC).
Invalidation walks the IC side-table and, for the JIT, marks affected
compiled functions for re-compilation on next entry.

---

## Part 6: C ABI extensions for multi-language

LANG15 specifies the vm-runtime C ABI for AOT/JIT to call into.
LANG20 extends it for the multi-language overlay.  The new entry
points are all language-agnostic; per-language operations are exposed
by each `<lang>-runtime`'s own `extern "C"` symbols.

### New entry points in `lang-runtime-core`

```c
/* ── Inline caches ──────────────────────────────────────────────── */

/* Look up a cached entry; returns NULL on miss.  ic_id is the
 * compile-time-assigned IC slot; receiver_class is from
 * LangBinding::class_of(receiver).  */
void* rt_ic_lookup(uint32_t ic_id, uint32_t receiver_class);

/* Install or update an entry after a runtime miss.  The entry is
 * binding-defined; the runtime stores it opaquely and returns it
 * on the next matching lookup.  */
void rt_ic_update(uint32_t ic_id, uint32_t receiver_class, const void* entry, uint32_t entry_size);

/* Invalidate every IC keyed on this class.  Called by the binding
 * when a class is reopened or a method redefined.  */
void rt_ic_invalidate_class(uint32_t class_id);

/* ── Deopt ──────────────────────────────────────────────────────── */

/* Yield to the interpreter at a published deopt anchor.  Does not
 * return.  native_frame must point to the JIT/AOT frame's local
 * area so the runtime can read register/spill slots per the
 * frame descriptor.  */
__attribute__((noreturn))
void rt_deopt(uint32_t anchor_id, void* native_frame);

/* Re-enter a JIT/AOT-compiled function from interpreter.  Boxes the
 * interpreter frame's register values per LangBinding::box_value
 * and tail-calls the specialised entry.  */
uint64_t rt_re_enter_specialised(
    uint32_t fn_index,
    const uint64_t* register_values,
    const uint8_t* register_repr_tokens,   // per-register BoxedReprToken
    uint32_t register_count
);

/* ── Symbol intern (used for property/method keys) ─────────────── */

uint32_t rt_intern_symbol(const uint8_t* bytes, size_t len);
const uint8_t* rt_symbol_bytes(uint32_t sym, size_t* out_len);

/* ── Per-language dispatch trampolines ─────────────────────────── */

/* Resolve a builtin by name through the active LangBinding.
 * Cached internally so each name is resolved once.  */
void* rt_resolve_lang_builtin(const uint8_t* name, size_t len);

/* Generic indirect call through LangBinding::apply_callable.
 * Used when the JIT sees an UNINIT or MEGA call site.  */
uint64_t rt_call_indirect(
    uint64_t callable,
    const uint64_t* args,
    uint32_t argc
);

/* Generic message send through LangBinding::send_message.
 * Used by JIT for MEGA send sites and as the slow path for ICs.  */
uint64_t rt_send_message(
    uint64_t receiver,
    uint32_t selector,
    const uint64_t* args,
    uint32_t argc,
    uint32_t ic_id            // for cache update on cold miss
);
```

### Per-language symbols

Each `<lang>-runtime` crate provides its own `extern "C"` surface for
operations the JIT/AOT compiler emits direct calls to:

```c
/* lispy-runtime exports */
uint64_t lispy_cons(uint64_t car, uint64_t cdr);
uint64_t lispy_car(uint64_t pair);
uint64_t lispy_cdr(uint64_t pair);
uint64_t lispy_make_symbol(const uint8_t* bytes, size_t len);
uint64_t lispy_make_closure(uint32_t fn_index, const uint64_t* captures, uint32_t n);
uint64_t lispy_apply_closure(uint64_t closure, const uint64_t* args, uint32_t n);

/* ruby-runtime exports */
uint64_t ruby_string_concat(uint64_t a, uint64_t b);
uint64_t ruby_array_new(const uint64_t* elems, uint32_t n);
uint64_t ruby_send(uint64_t recv, uint32_t selector, const uint64_t* args, uint32_t n, uint32_t ic_id);

/* js-runtime exports */
uint64_t js_get_property(uint64_t obj, uint32_t key, uint32_t ic_id);
uint64_t js_set_property(uint64_t obj, uint32_t key, uint64_t val, uint32_t ic_id);
uint64_t js_to_number(uint64_t v);
```

The naming convention (`<lang>_<verb>_<noun>`) is enforced by a lint
in `lang-runtime-core` so JIT/AOT codegen can resolve symbols
predictably.

### Calling convention

Standard System V x86_64 / AAPCS64 / RV64 ELF psABI for the platform.
Values passed and returned as `uint64_t`-sized machine words.  No
exceptions cross the ABI; runtime errors are signalled by setting a
thread-local error flag the caller checks (or, for fatal errors,
calling `rt_panic`).

### Ownership

Values passed across the ABI are **shared, not transferred**.  The
caller and callee both reference the underlying heap object; the GC
keeps the object alive as long as any frame references it.  No
explicit `incref` / `decref` (we are not refcounting at the ABI
level — per LANG16, the GC handles liveness).

---

## Part 7: Crate structure

```text
code/packages/rust/
├── lang-runtime-core/      # Generic substrate (LANG20 §"Architecture")
│   src/
│     gc.rs                  # Mark-sweep / copying / generational
│     allocator.rs           # Bump pointer, large-object area
│     safepoint.rs           # Suspend/resume mutators
│     write_barrier.rs       # Generational / incremental hooks
│     stack_map.rs           # Per-PC root tables
│     intern.rs              # Symbol table
│     ic.rs                  # InlineCache, ICEntry trait
│     deopt.rs               # Frame descriptor + materialise
│     binding.rs             # LangBinding trait
│     abi.rs                 # extern "C" entry points (LANG15 + LANG20)
│
├── lispy-runtime/          # Lisp/Scheme/Twig/Clojure value model
│   src/
│     value.rs               # tagged-i64 LispyValue
│     binding.rs             # impl LangBinding for LispyBinding
│     builtins/
│       arith.rs             # +, -, *, /, =, <, >
│       cons.rs              # cons, car, cdr, null?, pair?
│       symbol.rs            # make_symbol, symbol?
│       closure.rs           # make_closure, apply_closure
│     abi.rs                 # extern "C" lispy_cons / lispy_car / …
│
├── ruby-runtime/           # Ruby value model + builtins
├── js-runtime/             # JS value model + builtins
├── smalltalk-runtime/      # Smalltalk value model + builtins
├── perl-runtime/           # Perl value model + builtins
│
├── twig-frontend/          # Renamed from twig-ir-compiler; pure compile
│   src/
│     ast/                   # Typed AST (existing)
│     ir.rs                  # AST → IIR (existing)
│
├── twig-vm/                # Wires twig-frontend + lispy-runtime + vm-core
│
├── ruby-frontend/          # Future
├── js-frontend/            # Future
├── ...
│
└── (existing crates)
    interpreter-ir/         # IIR (LANG01); LANG20 adds slot kinds, send/load_property opcodes
    vm-core/                # Interpreter (LANG02); LANG20 adds LangBinding generic
    jit-core/               # JIT (LANG03); LANG20 adds IC + deopt protocol
    aot-core/               # AOT (LANG04); LANG20 adds PGO mode + frame-desc emission
    gc-core/                # GC (LANG16); LANG20 wires LangBinding for trace dispatch
    vm-runtime/             # Linkable runtime (LANG15); LANG20 adds new ABI entries
```

### Why split `<lang>-runtime` from `<lang>-frontend`

The runtime is what the JIT/AOT compiled code links against.  The
frontend is compile-time-only.  Splitting keeps the runtime small
(it must be linkable into AOT binaries that won't include any
front-end Rust code at all).

### Why share `lispy-runtime` across multiple languages

Lisp, Scheme, Clojure, and Twig all use:

- Tagged-int values
- Cons-cell pairs (heap-allocated)
- Interned symbols
- Closures with captured-env layout

Their differences (named lambdas vs. anonymous, `let` vs. `let*`,
syntax extensions) are *frontend* concerns — none of them propagate
to the value model.  One runtime crate, four frontends.

### Future polyglot binding

Several `LangBinding`s can coexist in one process.  Cross-language
calls use `rt_call_indirect` with a callable from one binding's
universe; the binding's `apply_callable` decides whether to dispatch
locally or proxy.  A Twig program embedding a Ruby regex object holds
it as `ruby_regex_handle` (a heap pointer) tagged in the Lispy value
space.

---

## Part 8: IIR additions

LANG20 requires three new opcodes and one new instruction-level field
on top of LANG01 + LANG16:

### New opcodes

| Mnemonic | Operands | Description | Slot kind |
|----------|----------|-------------|-----------|
| `send` | (receiver, selector_idx, args…) | Method dispatch | `MethodSiteSlot` |
| `load_property` | (obj, key_idx) | Read a property by symbol | `LoadSiteSlot` |
| `store_property` | (obj, key_idx, value) | Write a property by symbol | `StoreSiteSlot` |

`selector_idx` and `key_idx` are indices into the function's
**symbol-id constant pool** (a per-IIRFunction array of interned
symbol ids resolved at link time).  This avoids embedding strings in
the IIR.

### New `IIRInstr` field: `ic_slot`

```rust
pub struct IIRInstr {
    /* … existing fields … */

    /// If this instruction owns an inline cache, the IC's id.
    /// `None` for instructions without ICs (arithmetic, control flow).
    pub ic_slot: Option<u32>,
}
```

The compiler frontend assigns IC slot ids per function (sequential 0,
1, 2 …).  The runtime allocates IC storage at function-load time
based on the highest assigned id.

### IIR text format extension

For debug dumps:

```
v0 = send(recv, "to_s") : any        [ic=3 mono(class=String, target=String#to_s)]
```

The `[ic=…]` annotation is rendered when the IC is non-Uninit and
elided otherwise.

### Host-backend lowering for the new opcodes

The three new opcodes must remain lowerable by every host-runtime
backend (per §"Compilation paths"), without those backends needing to
understand IC or feedback machinery.  This is the lowering contract:

| Opcode | JVM | CLR | BEAM | WASM |
|--------|-----|-----|------|------|
| `send` | `invokevirtual` (selector → MethodRef in constant pool) | `callvirt` (selector → MethodRef metadata) | `apply/3` (atom-encoded selector + arity) | `call_indirect` (selector → table index resolved at link time) |
| `load_property` | `getfield` for known offset; `invokevirtual getXxx` for accessor | `ldfld` for known offset; `callvirt get_Xxx` for property | map-get / record `element/2` | `struct.get` (WASM-GC) or memory-load + offset table |
| `store_property` | `putfield` for known offset; `invokevirtual setXxx` | `stfld` for known offset; `callvirt set_Xxx` | map-put / record update | `struct.set` or memory-store |

`ic_slot` and `observed_slot` (LANG01 + LANG20 fields) are **dropped
on the floor** by host backends — the host runtime has its own type
profiling and inline cache machinery (HotSpot's profile counters,
CoreCLR's tiered JIT feedback, BEAM's inline-cache-free apply, V8's
own ICs when running WASM-GC).

---

## Part 9: Tier interaction matrix

How each tier exercises the LangBinding and runtime entry points:

| Operation | Interpreter (vm-core) | JIT (jit-core) | AOT (aot-core) |
|-----------|-----------------------|----------------|-----------------|
| Arithmetic on `any` | Calls binding builtin handler; records type_tag in `ArithSiteSlot` | Reads slot: mono → guard + unboxed op; mega → call rt_call_arith_generic | Same as JIT (with profile) or always-generic (without) |
| `send` | Calls `binding.send_message(recv, sel, args, ic, cx)` | Reads `MethodSiteSlot`: mono → emit class-id check + direct call to target; poly → emit dispatch table; mega → call `rt_send_message` | Same |
| `load_property` | Calls `binding.load_property(obj, key, ic)` | Reads `LoadSiteSlot`: mono → emit hidden-class check + offset load; poly → switch on class; mega → call `rt_load_property` | Same |
| `call_indirect` | Calls `binding.apply_callable(callable, args, cx)` | Reads `CallSiteSlot`: mono → guard callable identity + direct branch; poly → switch; mega → call `rt_call_indirect` | Same |
| `alloc` | `gc_core::alloc(size, kind)`; tells binding via `class_for(kind)` | Inlines bump-pointer alloc fast path; emits stack-map entry; falls back to `rt_alloc` on slow path | Same |
| `safepoint` | Calls `gc.maybe_collect()` if pending | Emits `rt_safepoint()` call; pollable via global flag | Same |
| Deopt | N/A (interpreter is the deopt target) | Emitted at every guard; calls `rt_deopt(anchor, frame_ptr)` | Same |

---

## Part 10: Migration path

The current state (post-PR #1741) has Twig + Brainfuck running
through `vm-core` with no LangBinding abstraction.  Migrating to the
LANG20 architecture happens in numbered PRs:

| PR | Scope | Unblocks |
|----|-------|----------|
| 1 | Define `LangBinding` trait skeleton in `lang-runtime-core`; no implementations yet | Type-checking the design |
| 2 | Implement `LispyBinding` in new `lispy-runtime` crate | Twig migration |
| 3 | Refactor `twig-ir-compiler` → `twig-frontend` (pure compile) + new `twig-vm` (runtime wiring) | E2E execution test for Twig |
| 4 | Wire `vm-core` to call LangBinding for `call_indirect`, `cmp_eq`, `is_truthy` | Twig E2E green |
| 5 | Add `send` / `load_property` / `store_property` IIR opcodes + interpreter handlers | Ruby/JS frontend prep |
| 6 | Implement IC machinery in `lang-runtime-core` | JIT specialization for dynamic dispatch |
| 7 | Implement second binding (recommend `RubyBinding`, smallest gap from Lispy) | Validates trait design holds |
| 8 | Implement deopt protocol in `jit-core` per LANG20 §"Deopt protocol" | Tier-up for dynamic languages |
| 9 | Implement frame-descriptor emission in AOT codegen | AOT for dynamic languages |
| 10 | PGO mode in AOT-core (consume profile artefact) | V8-style AOT-with-feedback |

PRs 1–4 are required before anything Lispy can run end-to-end.
PRs 5–7 unlock dynamic-language frontends.  PRs 8–10 deliver the
"V8 × GraalVM" promise.

Each PR includes acceptance tests — most importantly, the Twig
factorial program must keep passing through every PR.

---

## Part 11: Risk register

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| LangBinding trait churn | High | Touches every language | Implement two languages (Lispy + Ruby) before locking; trait changes thereafter require an RFC |
| Inline cache infrastructure complexity | Medium | Wide blast radius | Start with V8-style (4-entry PIC, hidden-class keying); generalise once Smalltalk PIC proves the contract works |
| Stack map generation depends on backend | Medium | AOT may lag | LLVM has `gc.statepoint`; for hand-rolled backends use shadow stacks initially (slower but simpler) |
| Closed-world AOT for arbitrary languages | High | AOT story incomplete | Land AOT-with-profile first (PGO drives reachability); pure closed-world AOT is a research follow-up |
| Cross-language heap interop bugs | Low (one language at a time) | Catastrophic when multi-lang lands | Defer cross-binding heap until needed; document the boundary explicitly |
| Generational GC requires write-barrier discipline | Medium | Subtle bugs | LANG16 already mandates barrier opcodes; LANG20 layers IC invalidation on top |
| Polyglot binding interaction | Low until we ship 2+ languages | Hard to reason about | Out of scope for v1; revisit after second language lands |

---

## Part 12: Open questions

1. **Single shared heap vs per-language heap.**  Today every
   `vm_instance_t` owns one heap (LANG16).  When two languages
   coexist in one process (e.g. Twig host invoking Ruby script),
   do they share or have separate heaps?  Trade-off: shared = simpler
   GC, potential cross-lang refs; separate = simpler invariants per
   language, requires explicit handles to cross.  **Recommendation:**
   one heap, multiple bindings, GC traces via the binding registered
   for the object's `class_or_kind`.
2. **Generational GC adoption.**  Worth it for Lispy alone?  Probably
   not.  Worth it for Ruby/JS where allocation rates are high?
   Definitely.  **Recommendation:** start mark-and-sweep
   (uncomplicated; LANG16 default), add generational once a JS or
   Ruby workload exists to drive design.
3. **MMTk vs in-house GC.**  MMTk is a production-grade modular GC
   used by JikesRVM, .NET prototypes, V8 experiments.  Adopting it
   would save years of GC work.  Trade-off: large external dep,
   binding to its API, less educational value.  **Recommendation:**
   defer; in-house mark-and-sweep is sufficient to ship 5 language
   frontends; switch to MMTk if/when a real workload demands a
   production collector.
4. **Threading model.**  Single-threaded today.  GIL?  Per-thread
   heap?  Actor-style?  Each language has different conventions.
   **Recommendation:** out of scope for LANG20; future LANG-CONCURRENCY
   spec when we have a use case.
5. **Where do exceptions live.**  Some languages have first-class
   exceptions (Ruby, JS, Python); some don't (most Lisps use
   call/cc or condition systems).  Trait-level `RuntimeError` is a
   placeholder; real exception interop needs a separate spec.
6. **Reflection / introspection.**  `class_of` gives you a
   `ClassRef`; what can you ask it?  LANG20 deliberately leaves this
   to per-language extensions of `LangBinding`.

---

## Acceptance criteria

This spec is "done" — locked enough to start implementation — when:

1. **`LangBinding` trait is specified at method-by-method
   granularity** (this doc, §"The LangBinding trait", §"Why these
   specific 15 methods").
2. **Cross-language value representation is specified** (§"Cross-
   language value representation"): every binding's `Value` is a
   single 64-bit word; every heap object has the LANG20 header.
3. **Feedback slot taxonomy is enumerated** with the data each kind
   stores and the per-tier consumption rules (§"Feedback-slot
   taxonomy", §"Tier interaction matrix").
4. **Deopt protocol is specified to the byte** (§"Deopt protocol")
   including frame descriptor format, materialisation, and inlined-
   call deopt.
5. **Inline cache machinery is specified generically** with a
   per-language entry shape table (§"Inline cache machinery").
6. **C ABI extensions are listed** with calling conventions (§"C ABI
   extensions for multi-language").
7. **Crate structure is agreed** (§"Crate structure").
8. **Migration path is sequenced** (§"Migration path") so PRs can be
   estimated and reviewed independently.

This document satisfies all eight.

---

## Out of scope (named for clarity)

- **Concurrency model** — single-threaded execution assumed throughout
  LANG20.  Future spec.
- **Exception interop across languages** — `RuntimeError` is a
  placeholder.  Real cross-language exceptions need a separate spec.
- **Reflection & metaprogramming** — `class_of` returns an opaque
  `ClassRef`; introspection is per-language.
- **Concrete GC algorithm choice** — covered by LANG16; LANG20 only
  specifies how `LangBinding` plugs into whatever GC is chosen.
- **Hot-swapping of LangBindings** — bindings are registered at VM
  init and live forever.  No reload story.
- **Persistent profile artefact format details** — sketch given in
  §"Profile artefact format"; full schema in a future LANG21 spec
  if/when AOT-PGO ships.
- **Specific JIT codegen** — that's per-backend (LANG05).  LANG20
  specifies the *contracts* the codegen must satisfy (frame
  descriptors, IC layout) but not the codegen itself.
- **Host-runtime backends** (JVM, CLR, BEAM, WASM lowerings) —
  covered by per-target specs (TW02, CLR01, BEAM01, …) and their
  associated `ir-to-<host>` crates.  LANG20 only commits to *not
  breaking* them; see §"Compilation paths".

---

## Relationship to other specs (cross-reference table)

| Spec | What it covers | LANG20 dependency |
|------|----------------|-------------------|
| LANG00 | Pipeline overview | LANG20 extends to multi-language |
| LANG01 | InterpreterIR | LANG20 adds `send`/`load_property`/`store_property` opcodes + `ic_slot` field |
| LANG02 | vm-core interpreter | LANG20 requires interpreter is parameterised over `LangBinding<L>` |
| LANG03 | jit-core | LANG20 specifies feedback-slot consumption + deopt protocol |
| LANG04 | aot-core | LANG20 specifies PGO mode + frame-descriptor emission |
| LANG05 | Backend protocol | LANG20 requires backends emit stack maps + deopt anchors |
| LANG10 | Code packager / linker | LANG20 requires linker resolves symbol-id pool entries |
| LANG11 | jit-profiling-insights | LANG20 feedback slots are the data source |
| LANG13 | debug-sidecar | Frame descriptors live in a sidecar section |
| LANG15 | vm-runtime C ABI | LANG20 extends with IC + deopt + per-language entries |
| LANG16 | gc-core | LANG20 wires `LangBinding::trace_*` to the collector's dispatch |
| LANG17 | vm-core metrics | LANG20 records IC hit/miss + deopt counts |
| LANG19 | codegen-core | LANG20 frame-descriptor format is what codegen emits |

---

## Appendix A: Why "V8 × GraalVM" hasn't been done before

Worth naming explicitly so the design choices have context.

V8 ships world-class tiered execution but only for JavaScript.  Adding
a second language means rewriting the bytecode, the IC strategy, the
hidden classes, and the JIT specialiser.

GraalVM Truffle ships partial-evaluation-driven JIT for many languages
(TruffleRuby, GraalJS, GraalPython, TruffleSqueak), but its AOT story
(Native Image) is mostly Java + JS.  AOT'ing TruffleRuby is possible
but unmaintained.

HHVM ships interp + JIT + AOT for one language (Hack/PHP) on one
bytecode.  Closest in spirit to what LANG20 describes — but
single-language.

CoreCLR (with Crossgen2 + RyuJIT) does interp / AOT (R2R) / JIT for
one runtime — but the runtime is C#-shaped and other languages on it
(F#, IronPython) compromise the shape.

What LANG20 describes — **horizontal × vertical** — has not shipped
in any production system the author is aware of.  It is achievable
because the existing LANG architecture has already paid the cost of
defining a generic IIR (LANG01) and a generic JIT/AOT pipeline
(LANG03/04); LANG20 adds the multi-language trait surface that lets
the pipeline serve the V8/GraalVM combined goal.

---

## Appendix B: Glossary

- **LangBinding** — Rust trait + matching C function table that a
  language frontend implements to plug into the runtime.
- **Feedback slot** — Per-IIR-instruction profile data; UNINIT →
  MONO → POLY → MEGA state machine.
- **Inline cache (IC)** — Per-call-site / per-load-site cache of
  observed shapes → resolved targets.  V8 originated; generalised
  here.
- **Deopt anchor** — IIR instruction index the JIT/AOT can yield
  back to.  Frame descriptor at the anchor describes how to
  reconstruct interpreter state.
- **Frame descriptor** — Per-deopt-anchor metadata mapping native
  registers/spill slots → IIR variable names + boxed/unboxed
  representation tokens.
- **Class** — Per-language opaque object identity (LangBinding's
  `ClassRef`); used for IC keying and method-version tracking.
- **Selector** — Interned symbol id for a method/property name.
- **Boxed repr token** — Discriminator for how a value is encoded
  in a native frame at a deopt anchor (boxed ref vs unboxed i64
  vs unboxed f64 vs unboxed bool vs derived ptr).
- **Stack map** — Per-PC table mapping native register/spill slots
  → "is this a pointer?" so the GC can find roots in JIT/AOT
  frames.
- **PGO (profile-guided optimisation)** — AOT compilation that
  consumes a recorded feedback profile to drive speculation
  decisions.
