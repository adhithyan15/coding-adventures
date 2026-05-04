# LANG22 — typing spectrum, AOT-no-profile, AOT-with-PGO, JIT
## (and how the three share one profile format)

## Overview

LANG20 specified the **multi-language runtime** — `LangBinding`, the
universal `IIRModule`, the cross-language value representation, the
deopt protocol, the IC entry shape.  But LANG20 deliberately stopped
short of saying *how compilation actually happens* once a language is
plugged in.  This spec fills that gap.

The thing the spec must accommodate is that the LANG VM is not "a JIT"
or "an AOT compiler" or "a tree-walking interpreter" — it has to be
**all three, simultaneously, for any language plugged in via
`LangBinding`**, and they have to *share a profile format* so a
program profiled by the JIT can be AOT-compiled by `aot-core` on a
later run, and a program AOT-compiled with profiles can be re-profiled
in the field to detect deviation.

LANG22 also introduces the **optional-typing spectrum**, which is the
single concept that ties everything together.  The same compilation
pipeline serves languages from "fully typed at compile time" (Tetrad,
Algol) to "partially typed" (TypeScript, Sorbet-Ruby, Hack) to
"untyped" (Twig, Lispy, MRI Ruby).  Each tier of typing changes which
compilation mode is most effective; nothing else about the architecture
changes.

---

## Why this spec is needed now

LANG04 (`aot-core`) was written assuming statically-typed input —
"the source language is fully (or sufficiently) typed".  That covers
Tetrad and Algol but not Twig, Ruby, or JavaScript.

LANG11 (`jit-profiling-insights`) and LANG12 (`vm-type-suggestions`)
were written assuming the JIT tier already exists and is producing
observations.  That's not yet true; LANG20 PR 8 wires the profiler in.

LANG20 (`multilang-runtime`) specified the trait surface and the deopt
protocol but listed AOT/JIT/PGO as future work.

What's missing is the **unified compilation story**:

1. A program written in any language plugged in via `LangBinding` must
   be compilable by `aot-core` *whether or not* a profile exists.
2. Profile data produced by the JIT must be consumable by `aot-core`
   on a later build (V8-style PGO).
3. Profile data must also be *expandable*: the same record that says
   "instr 42 was monomorphic-int after 100ms" must also be able to
   tell a developer "annotate parameter `n: int` and you skip the
   100ms warmup".
4. The optional-typing spectrum must be honoured: types are *optional*,
   not required, but the compiler must use them whenever they're
   provided.

LANG22 specifies all four.

---

## Relationship to existing specs

| Spec | Covered | Replaced/Extended by LANG22 |
|------|---------|------------------------------|
| LANG01 | InterpreterIR + `type_hint` field per instr | Field semantics extended: `type_hint` may now come from frontend annotation, frontend inference, or PGO-loaded profile. |
| LANG02 | Interpreter (vm-core) | Interpreter is the **fallback path**.  AOT-no-profile compiles whole programs but still calls into the interpreter for opcodes whose type isn't known.  Specified here. |
| LANG03 | jit-core | LANG22 specifies the profile artefact JIT *writes*.  Codegen mechanics stay in LANG03. |
| LANG04 | aot-core (typed languages) | Extended to handle untyped + partially-typed languages by linking the runtime library statically.  See §"AOT-no-profile pipeline". |
| LANG11 | JIT profiling insights | LANG22 specifies the *artefact format* the insights are built on top of. |
| LANG12 | Type suggestions for developers | LANG22 specifies the *data* the suggestions are derived from (per-instr observations + time-to-monomorphize). |
| LANG15 | vm-runtime C ABI | Extended with the LANG20 LangBinding entry points (`lang_call_builtin`, `lang_send_message`, etc.) so AOT codegen can emit calls.  See §"Runtime library shape". |
| LANG20 | LangBinding trait, value rep, deopt protocol | Foundation; LANG22 sits directly on top. |

---

## Architecture

### The five compilation modes

LANG22 organises the world into **five compilation modes**.  Every
language frontend, every program, every function picks one (or
several, for hybrid programs):

```
┌──────────────────────────────────────────────────────────────────┐
│  Mode 1: Tree-walking interpretation                             │
│    Input:  IIRModule                                             │
│    Output: live LispyValue (or per-language Value)               │
│    Speed:  baseline                                              │
│    Use:    development, REPL, cold paths after AOT               │
│    Status: PR 4 just shipped this for Twig.                      │
└──────────────────────────────────────────────────────────────────┘

┌──────────────────────────────────────────────────────────────────┐
│  Mode 2: AOT-no-profile                                          │
│    Input:  IIRModule                                             │
│    Output: native binary + statically-linked runtime library     │
│    Speed:  ~3–10× interpreter for dynamic languages              │
│            ~50–100× interpreter for statically-typed languages   │
│    Use:    cold-start sensitive programs (CLIs, embedded)        │
│            programs without warmup budget                        │
│            cross-compilation to non-host targets                 │
│    Status: PR 11 in the LANG20 migration path.                   │
└──────────────────────────────────────────────────────────────────┘

┌──────────────────────────────────────────────────────────────────┐
│  Mode 3: AOT-with-PGO                                            │
│    Input:  IIRModule + profile artefact (from a prior run)       │
│    Output: native binary + runtime library                       │
│    Speed:  ~30–50× interpreter for dynamic languages with        │
│            stable types (V8-class for static workloads)          │
│    Use:    production deployment after profile collection        │
│            per-version artefacts shipped with the binary         │
│    Status: PR 12 in the LANG20 migration path.                   │
└──────────────────────────────────────────────────────────────────┘

┌──────────────────────────────────────────────────────────────────┐
│  Mode 4: JIT (tier-up from interpreter)                          │
│    Input:  IIRModule + live observations from interpreter        │
│    Output: native code patched into the function table at run    │
│    Speed:  ~30–50× interpreter once warm                         │
│    Use:    long-running programs, servers, REPLs                 │
│    Status: PRs 7–10 in the LANG20 migration path.                │
└──────────────────────────────────────────────────────────────────┘

┌──────────────────────────────────────────────────────────────────┐
│  Mode 5: JIT-then-write-profile-then-AOT-with-PGO                │
│    Compound mode — run JIT in dev, ship the profile artefact     │
│    alongside the source, AOT-compile with PGO at release time.   │
│    GraalVM Native Image's killer feature.                        │
│    Status: enabled by Modes 3 + 4 + the shared profile format    │
│            specified in §"Profile artefact format" below.        │
└──────────────────────────────────────────────────────────────────┘
```

The "share one profile format" claim is what makes Mode 5 work.  The
JIT writes the same artefact `aot-core` reads.  See §"Profile artefact
format" below.

### Why no-profile AOT *is* a different mode from with-profile AOT

The temptation is to say "AOT-no-profile is just AOT-with-PGO with an
empty profile."  That's wrong, because:

| | AOT-no-profile | AOT-with-PGO |
|---|---|---|
| Speculation? | **No** | Yes |
| Type guards in emitted code? | **No** | Yes |
| Deopt mechanism required? | **No** | Yes |
| Frame-descriptor emission? | **No** | Yes |
| `BoxedReprToken` materialisation? | **No** | Yes |
| Codegen complexity | Lower | Higher |
| Codegen *correctness* boundary | "all behaviour LANG-runtime can produce" | "all behaviour LANG-runtime can produce on the profiled hot path; deopt for everything else" |

AOT-no-profile is the **conservative** AOT.  It doesn't speculate.
For every IIR instruction whose type isn't known statically, it
emits a call into the runtime library — `lang_call_builtin(...)` or
`lang_apply_callable(...)` — which uses `LangBinding` to dispatch
generically just like the interpreter would.  The win comes from
removing interpreter overhead (PC arithmetic, opcode decode, register
hash-lookup), not from type specialisation.

AOT-with-PGO *speculates*.  It reads the profile, sees that instr 42
was monomorphic-int 99.9% of the time, and emits specialised native
code that assumes int — with a guard that falls back to the
interpreter on mismatch.  That requires the deopt machinery.

This separation is **load-bearing** for the engineering plan: it lets
us ship AOT-no-profile in a single PR (PR 11a) without first
landing the deopt mechanism.  Programs gain a 3–10× speedup
immediately; the 30–50× from PGO comes later.

---

## The optional-typing spectrum

> "Types are optional.  If types are provided, produce optimized code
> from the start.  If types are provided for a subset, use that to
> produce optimized code for that subset.  If nothing is provided,
> attempt a conservative inference run."

The single concept that organises the rest of this spec is the
spectrum below.  A language frontend, or even a single function,
sits somewhere on it:

```
    ┌──────────────────────────────────────────────────────────┐
    │  TYPING SPECTRUM                                         │
    │                                                          │
    │   FULLY TYPED                          UNTYPED           │
    │       │                                    │             │
    │       │                                    │             │
    │       ▼                                    ▼             │
    │  ┌────────┐  ┌────────────┐  ┌──────────────────────┐    │
    │  │ Tier A │  │   Tier B   │  │       Tier C         │    │
    │  │ static │  │  partial   │  │ untyped + inference  │    │
    │  └────────┘  └────────────┘  └──────────────────────┘    │
    │                                                          │
    │  Examples:    Examples:        Examples:                 │
    │   Tetrad      TypeScript       Twig                      │
    │   Algol       Sorbet-Ruby      Lispy                     │
    │   Rust        Hack             MRI Ruby                  │
    │   future:     Mypy-Python      Python                    │
    │   gradual                      JavaScript                │
    │   "C+"                         Smalltalk                 │
    └──────────────────────────────────────────────────────────┘
```

### Tier A — fully typed

Every IIR instruction's `type_hint` is a concrete type
(`u8` / `i64` / `bool` / specific class).  The frontend has done all
the work; the compiler just lowers.

**AOT-no-profile** for Tier A produces code that is essentially
indistinguishable from C or Rust output — direct register-allocated
machine code, no runtime dispatch, no boxing.  ~50–100× the
interpreter.

**AOT-with-PGO** has nothing extra to learn (everything is already
known statically).  The profile data is still useful for *layout*
hints (hot/cold path partitioning, branch prediction hints), but
typing is already pinned.

**JIT** is rarely needed for Tier A — there's no observed-but-unknown
type to learn.  It can still help with inlining decisions, but the
gains are modest.

### Tier B — partial typing

Some instructions have concrete `type_hint`, others have `"any"`.
The most common case in practice: parameter types declared, local
variables inferred, generic-method bodies untyped.  This is
TypeScript with `any` escapes, Sorbet-Ruby, mypy-Python with
`Any`-typed deps, and gradual-typed Tetrad as it adopts dynamic
features.

**AOT-no-profile** for Tier B produces a *mosaic* binary: typed
instructions get specialised native code, `"any"`-typed instructions
get runtime-library calls.  The boundary between the two is the
seam where boxing/unboxing happens — well-understood territory, just
needs codegen support for both halves.

**AOT-with-PGO** sees all the `"any"` instructions and learns their
runtime types from the profile.  Specialisation expands inward from
the typed boundary.

**JIT** is highly effective for Tier B — most performance cliffs in
real-world dynamic-language programs are in untyped hot loops; the
JIT is good at exactly that.

### Tier C — untyped + inference

Every IIR instruction is `"any"`.  This is the default for Twig,
Lispy, JS, Ruby, Python before any user annotations land.

The compiler runs a **trivial inference pass** (specified below) to
recover what it can without help: literal types, comparison results,
boolean operators, etc.  In practice this picks up 20–40% of
instructions in a typical program — enough that AOT-no-profile sees
real wins even on completely untyped input.

**AOT-no-profile** for Tier C is essentially "ahead-of-time bytecode
dispatch".  Most opcodes turn into runtime-library calls.  The win
is removing interpreter overhead — measurable but small.  The win
*compounds* once the user starts adding annotations: each annotation
flips an instruction from runtime-call to native-code, and propagates
through inference.

**AOT-with-PGO** is where Tier C programs catch up to Tier A.  A
profiled untyped program can compile to within 2× of a fully-typed
equivalent.

**JIT** is the natural home of Tier C — the JIT does the inference
the user didn't, automatically.

### Why the same compiler works for all three tiers

The IIR's `type_hint` field is a single uniform interface.  The
*source* of the type can be:

1. User annotation in the source language (Tier A or B).
2. Frontend inference at compile time (Tier B promoted to better
   coverage; Tier C promoted to partial).
3. Profile artefact loaded by `aot-core` (Tier C promoted via PGO).
4. Live observation by the JIT (Tier C promoted by the running
   process).

The compiler doesn't care which.  It just reads the `type_hint`.
This is why the spec spends most of its weight on the **profile
artefact format** — that's where source-of-truth #3 and #4 live.

---

## AOT-no-profile pipeline

Below is the canonical pipeline for the no-profile AOT path.  This
is the **first AOT mode to land** because it has no dependencies on
the JIT or deopt mechanism — it can ship as soon as the language
frontend produces an `IIRModule` and the runtime library exposes a
stable C ABI.

```
┌───────────────────────────────────────────────────────────────────┐
│                                                                   │
│    Source code (.twig / .scm / .rb / ...)                         │
│        │                                                          │
│        ▼  language frontend (twig-ir-compiler / ruby-ir-... / ...)│
│                                                                   │
│    IIRModule (with whatever type_hint fields the frontend filled) │
│        │                                                          │
│        ▼  ir-optimizer: trivial inference pass                    │
│                                                                   │
│    IIRModule (with type_hint fields filled where inferable)       │
│        │                                                          │
│        ▼  aot-core: lower per-instruction                         │
│           - typed instr  → native machine code                    │
│           - untyped instr → call into liblang-runtime.a           │
│                                                                   │
│    CompilerIR → CodegenIR → object file (.o)                      │
│        │                                                          │
│        ▼  linker (system or built-in linker)                      │
│                                                                   │
│    Native binary (statically linked against liblang-runtime.a)    │
│        │                                                          │
│        ▼  user runs                                               │
│                                                                   │
│    Program executes; calls into runtime library for any "any"     │
│    instruction; uses native code for everything else.             │
│                                                                   │
└───────────────────────────────────────────────────────────────────┘
```

### What the binary looks like at runtime

```
┌────────────────────────────────────────────────────────┐
│  AOT-compiled program "fact"                           │
│                                                        │
│  fact_native:                                          │
│      ; if (n == 0) goto base                           │
│      cmp rdi, 0                                        │
│      je base                                           │
│                                                        │
│      ; mul_int(n, fact(n - 1))                         │
│      sub rdi, 1                                        │
│      call fact_native                                  │
│      ; recover saved n                                 │
│      mov rsi, [rbp - 8]                                │
│      ; CALL — typed multiplication, native             │
│      imul rax, rsi                                     │
│      ret                                               │
│                                                        │
│  base:                                                 │
│      mov rax, 1                                        │
│      ret                                               │
│                                                        │
│  ;; this is what fact() looks like for Tier C with     │
│  ;; trivial inference: int known, mul native-emitted   │
│                                                        │
│  apply_some_dynamic_method:                            │
│      ; for instructions whose type is unknown, we       │
│      ; emit calls into the runtime library:            │
│      mov rdi, <receiver>                               │
│      mov rsi, <selector>                               │
│      lea rdx, [rsp + 8]   ; argv                       │
│      mov rcx, <argc>                                   │
│      call lang_send_message                            │
│      ; rax now holds the boxed result                  │
│                                                        │
└────────────────────────────────────────────────────────┘
            │ statically linked against
            ▼
┌────────────────────────────────────────────────────────┐
│  liblang-runtime.a   (per-language flavour pre-built)  │
│                                                        │
│  - vm-core              (interpreter loop, called      │
│                          from generic dispatchers)     │
│  - lang-runtime-core    (LangBinding dispatch)         │
│  - lispy-runtime        (LispyBinding implementation)  │
│  - gc-core              (allocator + mark-and-sweep)   │
│  - intern table         (per-language symbol table)    │
│  - global table         (top-level value defines)      │
│                                                        │
│  Exported C symbols:                                   │
│    lang_call_builtin                                   │
│    lang_apply_callable                                 │
│    lang_send_message                                   │
│    lang_load_property                                  │
│    lang_store_property                                 │
│    gc_alloc                                            │
│    gc_safepoint                                        │
│    interp_dispatch_one    (fallback for any opcode)    │
│                                                        │
└────────────────────────────────────────────────────────┘
```

### Per-instruction lowering policy (no-profile)

| IIR opcode | Tier A/B (typed) | Tier C (`"any"`) |
|------------|-----------------|------------------|
| `const Int(n)` | `mov rax, n`              | `mov rax, tag_int(n)` (still trivial; literal types are inferable) |
| `const Bool(b)` | `mov rax, b ? 1 : 0`     | `mov rax, tag_bool(b)` (trivial) |
| `add a b`  | `add rax, rdx`              | `call lang_call_builtin "+", a, b` |
| `cmp_eq a b` | `cmp rax, rdx; sete al`   | `call lang_call_builtin "=", a, b` |
| `is_truthy v` | `test rax, rax`          | `call lang_is_truthy v` (binding hook) |
| `call_builtin name, args...` | always native call to resolved builtin fn | `call lang_call_builtin name, argv, argc` |
| `call name, args...` | native call into the AOT-compiled callee | native call (callee compiled in same module) |
| `send recv, sel, args...` | (Tier A doesn't use `send`) | `call lang_send_message recv, sel, argv, argc` |
| `load_property obj, key` | typed field offset access | `call lang_load_property obj, key` |
| `store_property obj, key, val` | typed field offset write | `call lang_store_property obj, key, val` |
| `jmp_if_false cond, label` | typed: `test rax,rax; je label`   | `call lang_is_truthy v; test rax,rax; je label` |
| `ret v` | `mov rax, v; ret` | same |
| `alloc class, fields...` | typed-layout `gc_alloc` then field stores | `call gc_alloc(class_id, size)` then `call_builtin` for field initialisation |

### Why not just inline `lang_call_builtin` per call site?

Because the builtin dispatch table is not flat — `LispyBinding::resolve_builtin`
matches names through a string switch, then returns a fn pointer.  At
AOT time we *can* resolve names at compile time (the builtin set is
fixed for a given binding) and emit direct calls to the resolved fn.
That's a worthwhile codegen optimisation but it's an optimisation,
not a correctness requirement — and it's exactly the kind of work
that gets duplicated by AOT-with-PGO and JIT.  Make it shared.

### Cross-compilation

`aot-core` already supports cross-compilation by virtue of
`codegen-core` emitting target-independent CompilerIR before the
backend lowers to a specific ISA.  AOT-no-profile inherits this
unchanged.  The runtime library must be available for the target
(`liblang-runtime-aarch64-darwin.a`, `liblang-runtime-x86_64-linux.a`,
etc.); building those is a one-time per-target operation.

---

## AOT-with-PGO pipeline

Conceptually identical to AOT-no-profile, with two differences:

1. **Input includes a profile artefact** alongside the IIRModule.
   The profile is consumed by a pass that promotes `type_hint` fields
   from `"any"` to monomorphic-observed types where the profile is
   confident.
2. **Codegen for promoted instructions emits speculation** — a guard
   that checks the runtime type matches the speculation, with a
   fallthrough to specialised code and a branch to a deopt anchor on
   mismatch.

The deopt anchor reconstructs the interpreter frame using
`BoxedReprToken` and the frame-descriptor metadata embedded in the
binary.  This is the LANG20 §"Deopt protocol" mechanism, applied at
AOT-compile time rather than at JIT-compile time — the codegen path
is the same.

```
   IIRModule + profile.ldp
        │
        ▼  ir-optimizer: profile-driven type promotion
   IIRModule with promoted type_hints
        │
        ▼  aot-core: lower per-instruction (now with speculation)
   CompilerIR with deopt anchors + frame descriptors
        │
        ▼  codegen-core: emit native + speculation guards
   object file (.o)  with .deopt section
        │
        ▼  linker
   Native binary with embedded deopt metadata
```

The `.deopt` section is a custom binary section the AOT-compiled binary
carries; on a guard failure, the runtime library reads it to
reconstruct the interpreter frame at the deopt anchor's instruction
index, then resumes interpretation.

PGO promotion has a configurable confidence threshold (default: 99%
of observations matched the speculated type).  Below threshold, the
instruction stays `"any"` and gets the no-profile treatment.

---

## JIT compilation pipeline

The JIT shares everything with AOT-with-PGO except the trigger and
the artefact lifetime:

| | JIT | AOT-with-PGO |
|---|---|---|
| Trigger | call count crosses promotion threshold | build-time, before any user runs |
| Profile source | live observations from this process's interpreter | persisted artefact from a prior run |
| Output | machine code in this process's memory | object file on disk |
| Lifetime | this process | persistent binary |
| Codegen backend | same as AOT (codegen-core) | same |
| Deopt mechanism | same as AOT-with-PGO | same |

The JIT additionally **writes** profile observations as it tiers up,
so the same artefact can be saved to disk at process shutdown for
the next AOT-PGO build.  See §"Profile artefact format" below.

### JIT promotion thresholds (per typing tier)

| Tier | `FullyTyped` | `PartiallyTyped` | `Untyped` |
|------|-------------|-------------------|-----------|
| Threshold (calls) | 0 (compile before first call) | 10 | 100 |

These match LANG01's existing constants and are not changed by
LANG22.

---

## Profile artefact format

The profile artefact is a **binary file** on disk (`.ldp` —
"language-runtime profile") with a strict, versioned layout.  It is
the **single shared format** the JIT writes, the AOT-PGO consumer
reads, and the developer-tooling expander parses.

### Goals

1. **Compact** — small enough to ship alongside the source as a
   build artefact (target: <1% of source size for typical programs).
2. **Versioned** — schema version in the header so future fields
   don't break old AOT compilers.
3. **Stable across processes** — does not embed pointers, addresses,
   or any process-local state.  Fully self-contained.
4. **Expandable** — every record carries enough information for the
   developer-tooling expander (LANG12-style) to produce educational
   reports.  The expander does not need extra source-language
   knowledge to do this; the profile carries it.
5. **Append-only at runtime** — the JIT can write incrementally
   without rewriting; merge happens on close or on explicit flush.

### Layout (binary, little-endian, version 1)

```
┌──────────────────────────────────────────────────────────────┐
│  Header (32 bytes)                                           │
│    magic = "LDP\0" (4 bytes)                                 │
│    version_major = 1 (u16)                                   │
│    version_minor = 0 (u16)                                   │
│    language = "twig" (16 bytes, NUL-padded)                  │
│    flags (u32) — bit 0 = closed-world, bit 1 = JIT-source    │
│    record_count (u32)                                        │
└──────────────────────────────────────────────────────────────┘
┌──────────────────────────────────────────────────────────────┐
│  String table                                                │
│    str_count (u32)                                           │
│    for each string:                                          │
│      length (u16) + bytes (UTF-8, NUL-terminated for safety) │
│  All later records reference strings by index into this      │
│  table — function names, type names, source paths, etc.      │
│  Compactness: identical strings share one entry.             │
└──────────────────────────────────────────────────────────────┘
┌──────────────────────────────────────────────────────────────┐
│  Module records (one per IIRModule that was profiled)        │
│    module_name (str_idx)                                     │
│    function_count (u32)                                      │
│    for each function:                                        │
│      function_name (str_idx)                                 │
│      param_count (u8)                                        │
│      for each param: declared_type (str_idx)                 │
│      call_count (u64)                                        │
│      total_self_time_ns (u64)                                │
│      type_status_at_record (u8) — FullyTyped/Partial/Untyped │
│      promotion_state (u8) — Interp / JITted / Deopted        │
│      instr_count (u32)                                       │
│      for each instr:                                         │
│        instr_index (u32)                                     │
│        opcode (str_idx)                                      │
│        observation_count (u32)                               │
│        observed_kind (u8) — Uninit/Mono/Poly/Mega            │
│        observation_count_at_promotion (u32) -- 0 if never    │
│        time_to_first_observation_ns (u64)                    │
│        time_to_promotion_ns (u64) -- 0 if never              │
│        types_seen (variable)                                 │
│          for each unique type:                               │
│            type_name (str_idx)                               │
│            type_count (u32)                                  │
│        ic_entries (variable, polymorphic only)               │
│          for each entry: see LANG20 IC entry shape           │
└──────────────────────────────────────────────────────────────┘
```

### Why these specific fields

- **call_count + total_self_time_ns** — drives PGO's "is this
  function hot enough to specialise" decision and the developer-
  tooling expander's "this function ate 40% of your runtime" line.
- **observation_count + types_seen** — the core PGO input.  Reads:
  "instr 42 saw 99,800 ints, 200 nils → speculate int".
- **time_to_first_observation_ns + time_to_promotion_ns** — the
  *educational* input.  Reads: "you spent 100ms warming up before
  the JIT figured out parameter `n` is always int.  Annotate
  `n: int` to skip this."
- **observation_count_at_promotion** — for the LANG12 type-suggestion
  expander to compute "how confident was the JIT" at the moment of
  promotion.
- **type_status_at_record** — distinguishes "this function had
  static types and the profile is just confirming them" from "this
  function was untyped and the JIT learned it".  Drives different
  expander messages.
- **promotion_state** — was this function actually JITted, or was it
  observed but never crossed the threshold?  AOT-PGO only specialises
  the JITted ones (closer to the runtime decision); the cold ones
  stay no-profile.

### Compactness math

For a 1000-function Twig program with ~10 instructions per function:

```
Header:        32 bytes
String table:  ~50KB  (function names, type names; many shared)
Module records:
  Per function: ~80 bytes static + 10 instrs × ~40 bytes = 480 bytes
  1000 functions = ~480 KB
Total:          ~530 KB
```

A typical Twig program of that size compiles to roughly 5MB of source
code; 530KB is 10%.  Acceptable but not great.  A future v2 of the
schema can add varint encoding (most counts are small) and column-
oriented storage (all instr_index values together, all observation
counts together) which typically gets these formats to 1–2%.

### Expandable: same artefact, two consumers

- **AOT-core**: reads only the type observations, ignores
  time-to-promotion, ignores per-frame timing.
- **lang-perf-suggestions** (LANG12-style tool, future PR): reads
  *everything* and produces a human report.

Same binary.  Different lenses.  This is what the user explicitly
asked for: "compact for AOT consumption, expandable for developer
education."

### Sample expanded report (developer-facing)

```
fact (twig:1234)
  Called 1,000,000 times, 4.7s total runtime.
  Tier: Untyped → JITted after 200,000 calls (122ms cold-start delay).

  Parameter `n`:
    Always observed: int (1,000,000 / 1,000,000 = 100%).
    SUGGESTION: declare `(define (fact (n int)) ...)` to:
      ─ skip 122ms warmup on next run
      ─ ship a smaller binary (deopt metadata for `n` not needed)
      ─ allow tier-A AOT (no PGO) to specialise `n` from build time

  Instruction 7: `mul`
    Always observed: int × int → int.
    Already specialised by JIT.  No action needed unless you want
    AOT-no-profile to specialise this from build time — same path as
    the parameter suggestion above.

  Instruction 12: `if (= n 0)`
    Always taken: false (999,999 / 1,000,000).
    SUGGESTION: hot-path the false branch in your code structure if
    Twig develops branch hint annotations.
```

---

## Trivial type inference pass

The pass that runs on every AOT-no-profile build, regardless of
typing tier.  Targets the cheap wins; nothing fancy.

### What it infers

| IIR pattern | Inferred type_hint |
|-------------|--------------------|
| `const Int(n)` | `i64` |
| `const Bool(b)` | `bool` |
| `cmp_eq` / `cmp_lt` / `cmp_gt` | `bool` |
| `is_truthy` | `bool` |
| `add` / `sub` / `mul` / `div` where both srcs are `i64` | `i64` |
| `add` / `sub` / `mul` / `div` where both srcs are `f64` | `f64` |
| `call_builtin "make_nil"` | `nil` |
| `call_builtin "+"` / `"-"` / `"*"` / `"/"` where args inferable as int | `i64` |
| `call_builtin "<"` / `"="` / `">"` | `bool` |
| `call_builtin "make_closure"` / `"make_builtin_closure"` | `closure` |
| `call_builtin "cons"` | per-language pair type |
| `call_builtin "make_symbol"` | `symbol` |
| `_move src` | type of `src` |
| `ret v` | type of `v` |

### What it does NOT do

- **Interprocedural inference** — calling `fact(5)` doesn't infer
  `fact`'s return type from the body; that requires either user
  annotation or PGO.
- **Branch-sensitive inference** — `(if (= x 0) "zero" x)` doesn't
  infer that the else branch is `int`; the inference works on the
  IIR's SSA form but doesn't track guards.
- **Recursive inference** — `(define (loop n) (loop (+ n 1)))` doesn't
  conclude that `n` is `int` even though the call site uses `+ n 1`;
  fixed-point inference is out of scope.

### Why so conservative

1. The pass runs on every AOT-no-profile build.  It must be fast —
   target: <1ms per 1000 instructions.  Anything beyond local
   propagation crosses that budget.
2. Interprocedural / fixed-point inference is what user annotations
   and PGO are for.  Doing it twice (compiler-side + profile-side)
   loses simplicity for marginal gain.
3. Wrong inference is much worse than missing inference.  Conservative
   makes correctness obvious.

### Pass output

The pass *writes back* into the IIRModule's `type_hint` fields.  It
does not produce a separate annotation graph.  Downstream codegen
reads `type_hint` exactly as if the user had written the annotation
themselves — which is the whole point: **inference is just
"computer-written annotations"**.

---

## Type ascription syntax (per language)

LANG22 doesn't mandate a syntax; each frontend chooses what fits its
host language.  But the *target* — the IIR `type_hint` field — is
uniform.

| Language | Syntax | Becomes IIR `type_hint` |
|----------|--------|-------------------------|
| Twig | `(define (square (x int)) (* x x))` | `i64` |
| Twig | `(define (f (x int) (y int) -> int) ...)` | params `i64`; `ret` `i64` |
| Lispy/Scheme | `(define (square (: x <integer>)) ...)` (R6RS-ish) | `i64` |
| Ruby | `sig { params(x: Integer).returns(Integer) }` (Sorbet) | `i64` |
| Ruby | `# typed: true` (file-level) | enables stricter inference |
| TypeScript | `function square(x: number): number { ... }` | `f64` (TS numbers are double) |
| Hack | `function square(int $x): int { ... }` | `i64` |
| Mypy-Python | `def square(x: int) -> int: ...` | `i64` |
| Future "C+" | `int square(int x) { ... }` | `i64` (Tier A) |

The frontend's job is to lower its language-specific annotation syntax
into the IIR `type_hint` strings.  The downstream pipeline doesn't
know or care which language fed it.

### Mapping table convention

For consistency across frontends, the canonical type-name strings are:

```
i8 i16 i32 i64
u8 u16 u32 u64
f32 f64
bool
nil
symbol
str
any
<binding>::<class-name>     (e.g. lispy::cons, ruby::String)
```

Frontends not on the canonical list (e.g. Smalltalk classes) prefix
with their binding name to avoid collision.

---

## Runtime library shape

`liblang-runtime.a` is the linkable runtime library.  AOT-no-profile
binaries link against it; AOT-with-PGO binaries link against it; even
the `vm-core` interpreter is built from it.  The same artefact serves
every mode.

### Crate composition

```
liblang-runtime/
├─ Cargo.toml           [lib]
│   crate-type = ["staticlib"]
│   (also "cdylib" for embed-in-other-language scenarios)
└─ src/
    ├─ lib.rs           Re-exports + sets up the C ABI shim
    ├─ c_abi.rs         #[no_mangle] extern "C" entry points
    └─ allocator.rs     gc-core's #[global_allocator] hookup
```

Internally `liblang-runtime` is a thin compositor over:

- `lang-runtime-core` (LangBinding trait)
- `lispy-runtime` (LispyBinding) — or whichever binding the build
  selected
- `vm-core` (interpreter loop)
- `gc-core` (allocator + collector)

Build-time feature flags select which binding(s) to include.  A
single-language program ships only the binding it needs:

```toml
# In a Twig program's Cargo.toml or build script:
[dependencies]
liblang-runtime = { features = ["lispy"] }
```

A polyglot program (e.g. embeds Twig + Ruby) ships both:

```toml
[dependencies]
liblang-runtime = { features = ["lispy", "ruby"] }
```

### Stable C ABI surface

```c
/* lang-runtime.h — auto-generated by cbindgen */

#include <stdint.h>
#include <stddef.h>

typedef uint64_t LangValue;
typedef uint32_t LangSymbolId;
typedef uint32_t LangClassId;

/* ── Builtin dispatch ──────────────────────────────────────────── */

/* Resolve and call a builtin by name.  The name must be valid for
 * the language whose binding is statically linked.  argc may be 0.
 * Returns the boxed result; sets `*out_err` if the builtin raised.
 */
LangValue lang_call_builtin(
    const char *name,
    const LangValue *argv,
    size_t argc,
    LangError *out_err
);

/* Apply a callable.  Used for closures and higher-order calls.
 * Implemented via LangBinding::apply_callable.
 */
LangValue lang_apply_callable(
    LangValue callable,
    const LangValue *argv,
    size_t argc,
    LangError *out_err
);

/* ── Method dispatch (Tier-aware) ──────────────────────────────── */

LangValue lang_send_message(
    LangValue receiver,
    LangSymbolId selector,
    const LangValue *argv,
    size_t argc,
    void *ic_slot,        /* per-call-site IC; NULL for non-cached */
    LangError *out_err
);

LangValue lang_load_property(
    LangValue object,
    LangSymbolId key,
    void *ic_slot,
    LangError *out_err
);

void lang_store_property(
    LangValue object,
    LangSymbolId key,
    LangValue value,
    void *ic_slot,
    LangError *out_err
);

/* ── Truthiness, equality, hash ────────────────────────────────── */

uint8_t lang_is_truthy(LangValue v);
uint8_t lang_equal(LangValue a, LangValue b);
uint8_t lang_identical(LangValue a, LangValue b);
uint64_t lang_hash(LangValue v);

/* ── GC ────────────────────────────────────────────────────────── */

LangValue gc_alloc(LangClassId class_id, size_t payload_size);
void gc_safepoint(void);  /* AOT codegen emits these at loop backedges */

/* ── Deopt entry (AOT-with-PGO + JIT) ──────────────────────────── */

void lang_deopt(
    const uint8_t *frame_descriptor,
    const uint64_t *register_state,
    size_t register_count
) __attribute__((noreturn));

/* ── Interpreter fallback ──────────────────────────────────────── */

/* Runs the IIR for a function from the supplied module entry by name.
 * Used by AOT-no-profile binaries when they hit an unsupported opcode
 * (e.g. `send` if compiled before PR 6) — falls back to interpretation.
 */
LangValue interp_run_function(
    const uint8_t *module_blob,
    size_t module_blob_size,
    const char *function_name,
    const LangValue *argv,
    size_t argc,
    LangError *out_err
);
```

### Linking model

Three supported configurations:

1. **Static**: `liblang-runtime.a` linked into the AOT binary.
   Default for CLI tools; ~600KB-1MB add to binary size.
2. **Dynamic**: `liblang-runtime.so` / `.dylib` shared object.
   Useful for distros packaging multiple AOT-compiled programs.
3. **Embedded**: liblang-runtime statically linked into a host
   program (e.g. a Rust app embedding Twig as a scripting language).
   Same surface; the embedding context provides the IIRModule.

---

## Educational tooling — `lang-perf-suggestions`

This is the developer-facing UI on the profile artefact.  Specified
to ensure the artefact carries the right data; the tool itself is
specced separately (extends LANG12).

### Inputs

- One or more `.ldp` profile artefacts (collected from JIT runs or
  AOT-with-PGO builds).
- Optionally, the source IIRModule for navigation.

### Outputs

- **Human-readable text report** (LANG12-style).
- **JSON for IDE integration** (LSP, "show me the type hints I
  should add").
- **Diff mode** — compare two profiles to detect regression
  ("this function used to monomorphise; now it's polymorphic; you
  may have introduced a type inconsistency").

### What the developer sees (specifically)

The user's exact ask was:

> "We should also allow for the profiling information to be output
> from the JIT compiler in a compact way that AOT PGO can understand
> but should also be expandable in a way where we can teach engineers
> that hey if you added types to this code, your program will boot
> up 50% faster because it took me like 100ms to figure out you
> were always passing integers."

The artefact carries `time_to_promotion_ns` per instruction and
`time_to_first_observation_ns` per function.  The expander produces
exactly that report:

```
$ lang-perf-suggestions --profile myapp.ldp

myapp/twig/handlers.twig
========================

Function: handle_request   (called 14,231 times)
  Tier: Untyped → JIT-promoted at call 100 (took 122ms).

  PARAMETER SUGGESTIONS — would skip 122ms warmup on cold start:
    parameter `req`:
      always observed: ruby::Hash (14,231 / 14,231 = 100%)
      ANNOTATE: (define (handle_request (req hash)) ...)
        save 122ms on every cold start (deployment, container restart)

  PARAMETER SUGGESTIONS (lower confidence):
    parameter `auth_token`:
      observed: str (12,488), nil (1,743) — 87.7% str
      ANNOTATE: (define (handle_request (auth_token (or str nil))) ...)
        if your language supports union types — saves ~95ms cold start

  HOT INSTRUCTIONS (no annotation will help; already optimal):
    instr 7  (load_property req "user_id"):    inline cache hit 99.8%
    instr 14 (call_builtin "+" timestamps):    type-stable, monomorphic

myapp/twig/util.twig
====================

Function: parse_int
  Tier: Untyped — JIT WAS NEVER PROMOTED (called 1,432 times,
        threshold = 100; promotion deferred because polymorphic).
  Type observations:
    parameter `s`:
      observed: str (1,200), int (200), nil (32) — POLYMORPHIC
      Cannot be specialised under PGO without union-type support.
      OPTIONS:
        - Refactor callers to always pass str
        - Wait for union-type AOT support (LANG23)
```

### `lang-perf-suggestions` as CI gate (optional)

```bash
$ lang-perf-suggestions --profile myapp.ldp \
    --warn-cold-start-above 50ms \
    --warn-poly-hot-functions 10
```

Fails CI if any hot function takes >50ms to JIT-promote, or if
>10 hot functions are polymorphic.  Surfaces type-stability
regressions automatically.

---

## Crate structure

New or extended crates introduced by LANG22:

```
code/packages/rust/
├─ liblang-runtime/        NEW
│   The static-library compositor.  Builds to .a + .so.
├─ aot-no-profile/         NEW (PR 11a in the LANG20 migration path)
│   Takes IIRModule → object file.  No deopt machinery.
│   Trivial inference pass lives here.
├─ aot-with-pgo/           NEW (PR 11b)
│   Extends aot-no-profile with profile consumption + speculation.
│   Reuses codegen-core for emitter.
├─ jit-core/               EXISTS (LANG03); LANG22 adds:
│   - profile.rs : writes .ldp on shutdown / on flush
│   - shared codegen path with aot-with-pgo (see codegen-core)
├─ ldp-format/             NEW
│   The .ldp serialisation/deserialisation crate.  Pure data.
│   Both jit-core and aot-with-pgo depend on it.
├─ lang-perf-suggestions/  NEW (extends LANG12)
│   Reads .ldp, emits human/JSON reports.  Pure-data CLI tool.
└─ codegen-core/           EXISTS (LANG19); LANG22 adds:
    - lower_typed_instr() : the typed-AOT path
    - lower_runtime_call(): the untyped-AOT path
    - emit_speculation_guard() : for AOT-PGO + JIT
    - emit_deopt_anchor()      : for AOT-PGO + JIT
```

Sharing notes:

- `aot-no-profile` does **not** depend on `ldp-format` — that's the
  whole point.  It can ship before profile collection works.
- `aot-with-pgo` and `jit-core` both depend on `ldp-format` and on
  `codegen-core`'s speculation/deopt emitters.  They share the
  artefact, the codegen, and the deopt mechanism.
- `lang-perf-suggestions` reads `ldp-format` directly without going
  through any compiler — it's a pure tooling crate.

---

## Migration path

LANG22 is unblocked by LANG20 PRs 5–8 (closures + send/load/store +
ICs + profiler).  Once those are in place, LANG22 ships in this
sequence:

| PR | Scope | Unblocks |
|----|-------|----------|
| 11a | `aot-no-profile` MVP for Twig: lower whole IIRModule to native, all dispatch through `liblang-runtime`.  Emit `liblang-runtime.a` build.  No inference yet — every untyped instr → runtime call. | First AOT binary for an LANG-runtime program; ~3–5× over interpreter. |
| 11b | Trivial inference pass in `ir-optimizer`; `aot-no-profile` consumes it. | ~5–10× over interpreter for untyped programs (typed instrs go native). |
| 11c | Type-ascription syntax in twig-parser + twig-ir-compiler.  `(define (f (x int)) ...)` form lowered to `type_hint`. | User-typed Twig functions get full Tier A treatment. |
| 11d | `ldp-format` crate.  Defines the binary layout + read/write APIs.  No producers or consumers yet — pure data. | Unblocks 11e and 11f to land in parallel. |
| 11e | `aot-with-pgo`: `aot-no-profile` extended to read `.ldp`, promote `type_hint`, emit speculation + deopt anchors. | First AOT-PGO binary; ~30–50× over interpreter for type-stable workloads. |
| 11f | jit-core writes `.ldp` on shutdown.  Uses same `ldp-format` crate. | JIT runs become reusable as PGO input for next AOT build. |
| 11g | `lang-perf-suggestions` v1: reads `.ldp`, emits human + JSON reports. | LANG12-style developer suggestions on real profile data. |
| 11h | `lang-perf-suggestions` v2 (CI mode): cold-start regression detection, polymorphism budgets. | Type-stability regressions caught at PR review time. |

PRs 11a–11d are **independent of the JIT** and can land before
LANG20 PRs 7–8 finish.  PRs 11e–11h depend on the profiler being
live (LANG20 PR 8).

### Acceptance test for 11a

A standalone binary named `fact-aot`:

```sh
$ ./fact-aot 5
120
$ time ./fact-aot 20
2432902008176640000
real    0m0.001s     # vs ~0.05s for the interpreter
```

The binary statically links `liblang-runtime.a`, contains the
AOT-compiled `fact` function as native machine code, and runs without
any external runtime dependency.  No JIT, no profile.

### Acceptance test for 11e

```sh
# First run: collect profile via JIT.
$ ./fact 1000000
<run with JIT>
$ ls fact.ldp
fact.ldp

# Second run: AOT-PGO compile using the profile.
$ aot-pgo --profile fact.ldp fact.iir -o fact-pgo
$ time ./fact-pgo 1000000
real    0m0.012s  # ~30x faster than no-PGO build
```

---

## Risk register

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| `liblang-runtime` ABI churn | Medium (early days) | Re-link required for every consumer | Versioned symbol names; break ABI only on major version; cbindgen-generated header in CI |
| Profile artefact format churn | Medium | Old `.ldp` files reject by new compiler | Version field in header; skip-unknown-fields semantics; tooling to migrate forward |
| Trivial inference pass false positive | Low (we explicitly stay conservative) | Wrong codegen | Verifier pass: re-check inferred types via interpreter on a synthetic input set, fail build if mismatch |
| Cross-language profile mixing (Twig + Ruby in same `.ldp`) | Medium when polyglot ships | Wrong PGO decisions | Header includes `language` field; AOT-with-PGO refuses cross-language match unless explicit override |
| Stack-map emission burden for AOT | Medium | AOT codegen complexity | Defer GC integration to LANG22-PR 11g; AOT-no-profile uses conservative GC (no stack maps required) for v1 |
| Closed-world AOT impossibility for dynamic langs | High | Pure no-runtime AOT impossible | LANG22 explicitly does NOT pursue closed-world; runtime library is always linked.  Educates users on this trade-off |
| Profile sensitivity to input distribution | Medium | Profile collected on dev workload mismatches prod | `lang-perf-suggestions` diff mode catches this; production profile collection (PR future) is the real fix |
| Inlining heuristics differ between AOT and JIT | Low | Surprising performance | Share heuristics via `codegen-core`; document the shared list |

---

## Open questions

1. **Fat profiles vs differential profiles.**
   A `.ldp` from a 2-week production run could be 100MB+.  Do we
   ship full or a "summary"?  **Recommendation:** start with full;
   add `--summarise` flag once size becomes a problem.

2. **Negative profiles ("never observed type X here").**
   Should the compiler use absence-of-observation as evidence
   (e.g. "instr 42 never saw nil → no nil check needed")?
   **Recommendation:** no.  Absence-of-evidence is not evidence-of-
   absence; codegen must still handle the path soundly.  Profile
   only drives speculation, never correctness.

3. **Cross-process profile aggregation.**
   When 100 production processes each write a `.ldp`, how do we
   merge?  **Recommendation:** simple sum — observations and call
   counts add, time-to-promotion takes the median.  Merging tool
   in `ldp-format`; not needed for v1.

4. **AOT for languages with macros / runtime code generation.**
   Lispy / Scheme `eval` defeats AOT.  **Recommendation:** AOT what
   you can; fall through to interpreter for `eval`'d code; document
   the boundary.  This is the LISP-Common-Compiled standard play.

5. **Cross-binary inlining.**
   Two AOT-compiled libraries with hot calls between them — should
   `aot-with-pgo` inline across the binary boundary?  Trade-off:
   binary size vs cross-binary speedup.  **Recommendation:** out of
   scope for v1; revisit when polyglot AOT ships.

6. **Separate `.ldp` per compilation unit vs one per program.**
   Per-unit is more flexible (recompile only the changed unit) but
   requires dependency tracking.  **Recommendation:** one per
   program for v1; revisit if build times become the bottleneck.

7. **Profile-guided GC tuning.**
   Allocation hotspots from profile could drive nursery sizing,
   generation choice, etc.  **Recommendation:** out of scope for
   v1; coordinate with LANG16 GC spec when generational lands.

8. **Educational tooling: how proactive?**
   Should `lang-perf-suggestions` be invoked automatically by the
   AOT compiler (printing suggestions on every build) or opt-in?
   **Recommendation:** opt-in via flag; spammy-by-default tools get
   ignored.

---

## Acceptance criteria

This spec is "done" — ready for implementation — when:

1. **The five compilation modes are enumerated and contrasted**
   (§"The five compilation modes").
2. **The optional-typing spectrum is defined** with concrete
   examples per tier (§"The optional-typing spectrum").
3. **AOT-no-profile pipeline is specified end to end** including
   per-instruction lowering policy (§"AOT-no-profile pipeline").
4. **AOT-with-PGO pipeline is specified** including its delta from
   no-profile (§"AOT-with-PGO pipeline").
5. **JIT pipeline is specified** including its shared mechanics
   with AOT-with-PGO (§"JIT compilation pipeline").
6. **Profile artefact format (`.ldp`) has a versioned binary
   layout** with field-by-field rationale (§"Profile artefact
   format").
7. **Trivial inference pass is specified** with the exact patterns
   it handles (§"Trivial type inference pass").
8. **Type-ascription syntax per language is mapped** (§"Type
   ascription syntax").
9. **`liblang-runtime` shape is specified** with the C ABI surface
   (§"Runtime library shape").
10. **Educational tooling shape is sketched** including the human
    report format (§"Educational tooling").
11. **Crate structure is agreed** (§"Crate structure").
12. **Migration path is sequenced into independent PRs**
    (§"Migration path").

This document satisfies all twelve.

---

## Out of scope (named for clarity)

- **Closed-world AOT for dynamic languages.**  The runtime library
  is always linked.  Pure no-runtime AOT for dynamic languages is a
  research follow-up.
- **Whole-program inlining.**  Not specified.  AOT compiles per-
  function with linker-time inlining left to LLVM/lld.
- **Cross-binary PGO.**  One binary at a time; cross-binary is
  future work.
- **Concurrency-aware profiles.**  Single-threaded profile collection
  assumed.  Threading goes with LANG-CONCURRENCY (future spec).
- **Runtime profile collection in production.**  Field PGO requires
  privacy / security review; spec'd separately when needed.
- **Reflective deopt** (deopt because user code introspected the
  current frame).  LANG20 §"Reflection" lists this; LANG22 doesn't
  add to that surface.
- **Custom backend ISAs** (RISC-V, Itanium, etc.).  `codegen-core`
  is target-independent; LANG22 specifies x86-64 + ARM64 only for
  its acceptance tests.
