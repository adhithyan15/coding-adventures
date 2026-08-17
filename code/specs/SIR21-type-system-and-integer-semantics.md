# SIR21 — Optional Type Propagation & Integer Semantics

## Status

Proposed. Foundational, cross-cutting: extends the `SirType` carrier defined in
[SIR10 §"Types — a carrier, not a verifier"](SIR10-narrow-waist-semantic-ir.md)
and is referenced by every frontend and backend. Ships behind a version bump of
the SIR node surface; the default remains fully dynamic, so existing modules are
unaffected until a frontend opts in. Implementation is a multi-PR cascade (see
[§Milestones](#milestones)).

This spec is written to be **red-lined**. It proposes concrete enum shapes and
rules; treat every table as a starting point.

---

## Motivation: types are the keystone

SIR is a narrow waist: *N* source languages × *M* targets collapse to *N + M*
implementations because everything passes through one IR. Today that IR is
**dynamic** — a `SirType` is either supplied or `None`, and the small v0 enum
(`Any | Int | Bool | Nil | Symbol | Str | Pair | Closure | Fn`) treats `Int` as
"64-bit signed" and mostly rides `Any`/`None`. That is enough to transpile Ruby
→ {Python, JS, Go, Rust} where every value is boxed and every integer is
arbitrary-precision-ish.

It is **not** enough for the two things we now want:

1. **New targets where the machine integer matters** — C and C++ have no bignum;
   an `int32_t` wraps, a `size_t` is unsigned, signed overflow is undefined.
2. **New sources that already carry types** — C, typed Python, TypeScript, Java,
   C# — and the inverse direction (C → Ruby, C → JS).

Both hinge on the *same* missing information, and carrying it makes both sound at
once:

- **C → Ruby (faithfulness).** In C, `uint32_t x = 0xFFFFFFFF; x + 1` is `0`
  (wraps mod 2³²). In Ruby, `0xFFFFFFFF + 1` is `0x100000000` (arbitrary
  precision). If SIR records only "add," the Ruby backend silently changes the
  answer. If SIR records `add : u32{wrap}`, the Ruby backend emits
  `(a + b) & 0xFFFFFFFF` and preserves C's behaviour. **The type is the
  semantics.**

- **Ruby → C (efficiency + correctness).** Ruby integers are arbitrary
  precision; C has none. Without type information a faithful C backend must box
  *every* integer in a bignum runtime. With SIR carrying "this value is provably
  `i64`," the C backend emits a native `int64_t`. **The type is permission to
  specialise.**

- **Integer limits/min/max fall out for free** — they are a *function* of the
  integer type, not separate data (see [§Integer semantics](#integer-semantics)).

The design rule that makes this tractable is the one SIR10 already states for its
escape hatch, generalised: **when an operation's meaning depends on
source-language semantics, split it into unambiguous typed operations in the
core — do not overload one op and interpret it in the runtime.** A single
`divide` that "does the Ruby thing" is a latent bug the moment a second frontend
appears. `div_floor` and `div_trunc` are two honest ops.

---

## Non-goals (what SIR21 does **not** change)

SIR21 keeps SIR10's foundational stance:

- **SIR is a carrier, not a verifier.** SIR21 does **not** add type *inference*
  or type *checking*. A frontend supplies a `SirType` or supplies `Dynamic`; SIR
  round-trips that decision faithfully. A backend *reads* the carried type to
  choose a lowering. Nobody proves the types are internally consistent — that is
  the frontend's job (a C frontend gets types from the C type system; a Ruby
  frontend supplies `Dynamic`).
- **Dynamic stays first-class.** `Dynamic` (renamed from `Any`) is the top type
  and the default. A wholly-dynamic module is exactly today's behaviour: boxed
  values, runtime dispatch. Optional typing means *optional* — untyped programs
  never regress.
- **No new control flow, no new escape mechanism.** The [SIR10 Intrinsic escape
  hatch](SIR10-narrow-waist-semantic-ir.md) and its six hard rules stand
  unchanged; SIR21 only clarifies how types ride on intrinsics and proposes one
  additive `fallback` field ([§Escape hatch](#escape-hatch-integration)).

---

## The extended type lattice

`SirType` grows from a flat enum into a small algebra. `Dynamic` is the top;
every other type is a more-specific classification a frontend *may* supply.

```text
SirType =
    | Dynamic                              // top — unknown/any (was `Any`); the default
    | Nil
    | Bool
    | Int   { spec: IntSpec }              // parameterised — see below
    | Float { width: FloatWidth }          // F32 | F64
    | Str   { encoding: StrEncoding }      // Utf8 | Bytes  (default Utf8)
    | Symbol
    | Seq   { elem: Box<SirType>,          // homogeneous sequence / array
              len:  Option<u64> }          // Some(n) ⇒ fixed-length (C arrays)
    | Map   { key: Box<SirType>,
              val: Box<SirType> }
    | Pair                                 // cons cell (unchanged)
    | Closure                              // any closure handle (unchanged)
    | Fn    { params: Vec<SirType>,
              ret:    Box<SirType> }       // (unchanged)
    | Ptr   { pointee: Box<SirType>,       // C/C++ pointer or reference
              nullable: bool }
    | Struct{ name: String,                // nominal record (C struct, class field bag)
              fields: Vec<(String, SirType)> }
    | Optional { inner: Box<SirType> }     // nullable value; T-or-nil
```

Notes:
- **Backwards compatibility.** `Any → Dynamic`, and the old flat `Int` becomes
  `Int { spec: IntSpec::arbitrary() }` so existing serialised modules map
  deterministically onto the new shape. *(Implemented in T1a — see
  [semantic-ir CHANGELOG 0.16.0]. This resolves the earlier tentative
  `I64_WRAPPING_OR_ARBITRARY` name toward `Arbitrary`: the historical dynamic
  pipeline never masked integers — Ruby → Python both grow — so the
  behaviour-preserving default is arbitrary precision, and a frontend that means
  a fixed machine width must say so explicitly.)*
- **`Seq.len`** distinguishes a growable list (`None`) from a fixed C array
  (`Some(n)`). Backends that lack fixed arrays lower both to their list type.
- **`Ptr`/`Struct`** exist primarily for *source* fidelity (a C frontend needs
  them). Dynamic targets (Ruby/JS) lower `Ptr` to a reference and `Struct` to a
  record/object; targets without them declare so in their manifest and reject.
- The lattice is deliberately **shallow**. Generics, traits, sum types, and
  refinement/range types (`Int in 0..=100`) are **out of scope for v1** and
  noted in [§Deferred](#deferred).

### Feature-manifest additions

New `Feature` flags gate the new surface so a backend fast-rejects what it can't
express (mirrors SIR10 §Feature manifest):

```text
Feature +=
    | SizedIntegers      // any Int with a concrete width (not I64/Arbitrary)
    | Unsigned           // any unsigned Int
    | WrappingArithmetic // any op whose overflow mode is Wrap/Saturate/Checked
    | FixedArrays        // Seq { len: Some(_) }
    | Pointers           // Ptr
    | Structs            // Struct
    | Bignum             // Int { width: Arbitrary }
```

A Ruby module uses `Bignum` (its ints are arbitrary precision). A C module uses
`SizedIntegers`, likely `Unsigned`, `WrappingArithmetic`, `Pointers`, `Structs`.
A backend lists which it accepts; the O(1) manifest check happens before any body
traversal.

---

## Integer semantics

The integer is where "the type is the semantics" bites hardest, so it gets a
dedicated descriptor.

```text
IntSpec { width: IntWidth, signed: bool, overflow: Overflow }

IntWidth  = W8 | W16 | W32 | W64 | W128 | Arbitrary
Overflow  = Wrap        // modular 2^n            (C unsigned, Ruby's masking target)
          | Trap        // panic/raise on overflow (Swift, Rust debug)
          | Saturate    // clamp to min/max        (DSP, Rust saturating_*)
          | Checked     // produce Optional/None    (Rust checked_*)
          | Undefined   // UB — backend MAY choose; MUST record its choice
          | Arbitrary   // no overflow — grows      (only valid with width=Arbitrary)
```

### Min / max / limits are derived, not stored

For a concrete `(width, signed)`, the bounds are a pure function:

| width | signed | min | max | modulus (wrap) |
|-------|--------|-----|-----|----------------|
| W8    | false  | 0 | 255 | 2⁸ |
| W8    | true   | −128 | 127 | 2⁸ |
| W32   | false  | 0 | 4 294 967 295 | 2³² |
| W32   | true   | −2 147 483 648 | 2 147 483 647 | 2³² |
| W64   | true   | −2⁶³ | 2⁶³−1 | 2⁶⁴ |
| Arbitrary | true | −∞ | +∞ | — |

So SIR does **not** store min/max on the type; it stores `(width, signed)` and
every consumer computes the bound. Programs that *reflect* on limits (Ruby's
`Integer` has none; C's `INT_MAX`, Rust's `i32::MAX`) read them through a small
**constant intrinsic namespace** that const-folds:

```text
int.max(W32, signed=true)  ⟶  2147483647
int.min(U8)                ⟶  0
int.width(i)               ⟶  32
```

These are pure, total, and target-independent — a backend const-folds them at
emit time or calls a trivial runtime helper.

### `Arbitrary` is the bridge to dynamic languages

`Int { width: Arbitrary, overflow: Arbitrary }` is the Ruby/Python integer: it
never overflows, it grows. This is the *only* integer a purely-dynamic frontend
emits, and it is what a `Bignum`-declaring backend must support (natively in
Ruby/Python/JS-BigInt; via a bignum runtime in C/C++/Go/Rust when the value can't
be proven to fit a machine width).

---

## Type-directed operation selection

This is the semantic-neutrality principle made mechanical. An arithmetic or
comparison node resolves to a concrete operation **from the static types of its
operands**, and only falls back to runtime dispatch when an operand is `Dynamic`.

| operand types | `+` lowers to | overflow behaviour |
|---------------|---------------|--------------------|
| `Int i32{wrap}`, `Int i32{wrap}` | native 32-bit add, masked to 2³² | wraps |
| `Int i64{trap}`, `Int i64{trap}` | checked add; raise on overflow | traps |
| `Int Arbitrary`, `Int Arbitrary` | bignum add | grows |
| `Float f64`, anything numeric | float add (promote) | — |
| `Str`, `Str` | string concat | — |
| `Dynamic`, anything | **runtime `_sir_plus` dispatch** | as today |

The frontend does **not** pre-bake the target's opcode; it supplies typed
operands (or `Dynamic`), and each backend's emitter maps `(op, operand-types)` to
its realisation. Where two source languages disagree on an *untyped* op (Ruby
`/` floor vs Python `/` true-division), the **frontend disambiguates by emitting
different ops** — `div_floor` vs `div_true` — so the choice is explicit in the IR
and no runtime guesses. (Ruby's `7/2 == 3`; Python's `7/2 == 3.5`, `7//2 == 3`.)

> **Compatibility:** when both operands are `Dynamic`, behaviour is *identical to
> today* — the existing `_sir_plus`/`_sir_divide` runtime helpers, with their
> current (Ruby-flavoured) semantics. SIR21 only *adds* the typed fast paths;
> it removes nothing.

---

## Backend lowering & the faithfulness contract

Each backend declares, per target, how it realises each `IntSpec`. The
**faithfulness contract**: *the observable result of a typed op must equal the
result the type's own semantics prescribe, regardless of whether the target has a
native type of that width.*

| target | `i32{wrap}` | `u64{wrap}` | `Arbitrary` |
|--------|-------------|-------------|-------------|
| C / C++ | native `int32_t` (record UB→wrap choice) | native `uint64_t` | bignum runtime |
| Rust | `i32` with `wrapping_*` | `u64` `wrapping_*` | bignum runtime |
| Go | `int32` | `uint64` | `math/big` or inlined bignum |
| Java | `int` (already wraps) | **no native u64** → `long` + unsigned ops | `BigInteger` |
| C# | `int` | `ulong` | `System.Numerics.BigInteger` |
| Python | native `int` + `& mask` to emulate width | `& mask` | native `int` |
| Ruby | native Integer + `& mask` | `& mask` | native Integer |
| JavaScript | Number for ≤2⁵³, **`BigInt` + `BigInt.asUintN/asIntN`** beyond | `BigInt.asUintN(64,…)` | `BigInt` |

The hard cases the harness must pin:
- **JS numbers are f64** — any integer beyond 2⁵³ silently loses precision.
  A `u64`/`i64` typed op on JS therefore **must** emit `BigInt`, and `int32`
  wrap must use `BigInt.asIntN(32,…)` (or `| 0` for the signed-32 case). This is
  the single most error-prone target and gets the most conformance cases.
- **Java has no unsigned types** — `u32`/`u64` lower to `int`/`long` with the
  `Integer.divideUnsigned`/`Long.compareUnsigned` family.
- **Dynamic targets emulate width by masking** — Ruby/Python have no fixed-width
  int, so a `u8{wrap}` add is `(a + b) & 0xFF`. This is exactly the C→Ruby
  faithfulness case.

A target that cannot honour a spec (e.g. a backend with no bignum) **rejects via
the manifest** (`Bignum` not in its accept-list) — it never silently truncates.
Silent truncation is the failure mode this whole spec exists to prevent.

---

## Worked examples (the proof targets)

These are literal conformance programs; §Provability turns them into tests.

### E1 — unsigned wraparound survives the round trip

```
// source-neutral SIR (as a C frontend would emit)
let x: u32{wrap} = 0xFFFFFFFF
puts(x + 1)          // op: add on u32{wrap}
```

Expected stdout on **every** target: `0`.
- C/C++/Rust/Go: native unsigned add → 0.
- Java/C#: `int`/`uint` → 0.
- Ruby/Python: `(0xFFFFFFFF + 1) & 0xFFFFFFFF` → 0.
- JS: `BigInt.asUintN(32, 0xFFFFFFFFn + 1n)` → `0n` → prints `0`.

Before SIR21 the Ruby/Python/JS arms print `4294967296` — *wrong*. This program
is the regression oracle.

### E2 — arbitrary precision survives Ruby → C

```
let n: Arbitrary = 1000000000000        // 10^12, exceeds i32, fits i64
puts(n * n)                              // 10^24, exceeds i64
```

Expected stdout everywhere: `1000000000000000000000000`.
- Ruby/Python/JS(BigInt): native.
- C/C++/Go/Rust/Java/C#: bignum runtime (declared via `Bignum` feature).

A backend that lacks bignum **must reject** this module, not print a truncated
`2003764205206896640`.

### E3 — division semantics are explicit, not guessed

```
puts(div_floor(7, 2))    // 3   — Ruby's `/`
puts(div_trunc(-7, 2))   // -3  — C's `/`
puts(div_true(7, 2))     // 3.5 — Python's `/`
```

Each op is distinct in the IR; every backend emits the matching primitive. No
`divide` that "means Ruby on Tuesdays."

There is a fourth name, `udiv_trunc` — `div_trunc`'s unsigned twin, needed
because every backend stores values as tagged `i64`/`f64`: a `u64` ≥ 2^63
misreads as negative unless routed to a distinct code path. This is not a new
rounding mode — `div_trunc`/`udiv_trunc` round identically (toward zero); the
split is purely a backend *representation* concern, the same reason SIR
already has separate `tdiv`/`utdiv`/`tmod`/`utmod` names for C's own
truncating division/modulo. **T3b-2 folds `tdiv`/`utdiv` into
`div_trunc`/`udiv_trunc`** — one canonical truncating-division name family,
not two synonyms for the same rounding mode (per this section's own "no
`divide` that means Ruby on Tuesdays" rule, which applies just as much to a
second name for the *same* semantics as it does to one name with two
semantics). `tmod`/`utmod` (modulo) are **not** touched by T3b-2 — they stay
under their existing names, a deliberate, visible asymmetry (division
renamed, modulo not) left as a forward pointer to a future, not-yet-numbered
milestone rather than silently expanded scope here.

**Which ops are new code vs. a rename** (non-obvious enough to state
explicitly): every backend already has a division helper that floors
integers and true-divides floats — matching `div_floor` exactly — so on
six of the seven backends (C, Go, JavaScript, Python, Ruby, Rust)
`div_floor` is a *rename* of existing, already-correct logic, not new
behavior. TypeScript is the one exception: its division helper always
truncates, even on float operands, diverging from every sibling backend's
Ruby-floor-faithfulness — a real, previously undetected bug (invisible
because the TypeScript backend isn't in `sir-conformance`'s test matrix),
fixed as part of wiring `div_floor` there for the first time. `div_trunc`/
`udiv_trunc` are likewise a rename on C and Ruby (from `tdiv`/`utdiv`) but
genuinely new code on the other five. `div_true` — always coerce to float,
never branch on operand tag — is genuinely new on every backend; it must
raise the same typed `ZeroDivisionError` on a zero divisor every sibling op
already raises, not let a bare host `/` produce IEEE `Infinity`/`NaN`.

Twig (`twig-to-semantic-ir`, an internal Lisp-S-expression IR-testing DSL,
unrelated to PHP's Twig templates) does **not** migrate to the `div_*`
family and keeps emitting bare `"/"` permanently: its own `/` is variadic
(any number of operands folded left-to-right) where every `div_*` op is
strictly binary, and its numeric tower carries no static int/float
distinction to pick among the three rounding modes with. This is a
considered design conclusion, not deferred work — consequently, bare `"/"`
is never fully dead code on any backend and stays as Twig's permanent,
documented route (every backend keeps `"/"` pointed at the same
implementation as the renamed `div_floor`).

No `semantic-ir` core-IR or validator change is required to add
`div_floor`/`div_trunc`/`udiv_trunc`/`div_true` as new `BuiltinCall` names —
`validator.rs`'s entire `Expr::BuiltinCall` validation only special-cases
`"cons"|"car"|"cdr"|"pair?"` for `Feature::Pairs`; every other name,
including these four, passes through unconstrained (arity and existence are
each backend's own concern, exactly as `tdiv`/`utdiv` already are today).

---

## Escape hatch integration

The [SIR10 Intrinsic](SIR10-narrow-waist-semantic-ir.md) already carries a typed
`return_type` and typed `args` and is whitelist-gated per backend. SIR21 changes
**nothing** about its six hard rules and adds only:

1. **Types on the boundary now use the SIR21 lattice** — an intrinsic can declare
   `return_type: Int { u32{wrap} }`, so even opaque operations stay
   type-transparent at the seam.
2. **Proposed additive `fallback: Option<Box<Expr>>`** — a generic SIR subtree a
   backend that does *not* whitelist the intrinsic may lower **instead of
   rejecting**. This turns a hard reject into graceful degradation *only where a
   portable equivalent exists* (e.g. `simd.add_u32x4` with a scalar-loop
   fallback). If `fallback` is `None`, behaviour is exactly today's:
   non-whitelisted ⇒ compile error. This keeps total coverage: a program lowers
   through core, or a whitelisted intrinsic, or a declared fallback, or it is
   rejected — never silently mis-emitted.

**Integers stay in the core, not the hatch.** Per SIR10's rule of thumb — "first
ask whether the feature should be added to the IR's core instead" — width,
signedness, and overflow are *core* type/op information, never intrinsics. The
hatch remains for the genuinely unportable (inline asm, SIMD, a target-specific
syscall).

---

## Provability & verification — *how we prove this works*

This section is the point of the spec. A type system that carries integer
semantics is only worth anything if we can **demonstrate**, by running code, that
every backend honours it. The strategy is a **cross-backend conformance harness**
built on the `compile_and_run_*` pattern already used across the SIR backends
(hand-build or lower a module, emit, invoke the *real* toolchain, assert stdout).

### P1 — the conformance matrix (differential testing)

A single corpus of **conformance programs** (E1–E3 above and their siblings) each
paired with **one expected output string** — the *reference semantics*, computed
independently of any backend. The harness runs the Cartesian product:

```
for prog in corpus:
    ref = reference_oracle(prog)                 # the correct answer, computed once
    for backend in {py, js, ts, go, rust, java, csharp, c, cpp}:
        out = compile_and_run(emit(backend, prog))   # real toolchain
        assert out == ref, (prog, backend, out, ref)
```

A program *passes* only when **every** backend reproduces the reference byte-for-
byte. Disagreement is a faithfulness bug, localised to `(program, backend)`. This
is exactly the differential-testing structure that would have caught the
`case_eq` gap (a `when` program printing correctly on Python but panicking on
Go/Rust/JS) the day it landed — see [lessons.md]. **The conformance matrix is the
systemic guard against the "works on some backends" failure class.**

### P2 — the reference oracle

The expected output is not hand-typed guesswork; it is computed by a small,
audited **reference model** of the SIR semantics (a pure function from typed SIR
+ inputs to observable output), living beside the harness. For integers it is a
few lines: wrap = `result mod 2^width` re-centred for signedness; trap = raise if
out of range; arbitrary = Python-int math. The oracle is itself unit-tested
against known constants (`INT32_MAX + 1 == INT32_MIN`, `0u32 - 1 == 4294967295`).
Every backend is measured against the oracle, never against another backend
(avoids two-wrongs-agree).

### P3 — property-based / fuzz coverage of integer ops

For each `IntSpec` a target claims to support, generate random operand pairs and
random ops, compute the oracle result, and assert the emitted program matches.
This finds the edge the fixed corpus misses (e.g. `i32{wrap}` `MIN / -1`, shift
by ≥ width, `u64` just above 2⁵³ on JS). Seeded and shrinking so failures are
reproducible and minimal.

### P4 — bidirectional round-trip proofs

The two directions that motivated the spec become explicit tests:
- **Ruby → C → run** must equal **Ruby → (reference) → run** for programs whose
  values provably fit a machine width (the specialisation path) *and* for
  programs that don't (the bignum path).
- **C → Ruby → run** must equal **C → (gcc) → run**, including wraparound — the
  `uint32_t` overflow program prints the same thing compiled by `gcc` and
  transpiled to Ruby.

These use the real `gcc`/`javac`/`dotnet`/`node`/`go`/`rustc` toolchains
(verified available locally: Java 21, .NET 9, clang 21) and skip gracefully when
a toolchain is absent (the existing pattern).

### P5 — the coverage gate (no silent gaps)

The harness enforces a **completeness invariant**: for every `(SirType variant ×
op × backend)` pair that a backend's manifest claims to accept, there exists at
least one passing conformance case; anything accepted-but-untested fails CI as a
*coverage* error, and anything a backend can't do must be *explicitly rejected*
(manifest) rather than absent. This is the direct, structural fix for the
`case_eq` class of bug: a builtin/type the frontend can emit but a backend never
implemented can no longer hide — it is either proven or explicitly refused.

### P6 — what "proven" means for a PR

A backend PR in this cascade is **not mergeable** until:
1. its `compile_and_run_*` conformance tests pass on the real toolchain,
2. the coverage gate shows no accepted-but-untested `(type, op)` pair, and
3. the differential run against the reference oracle is green for every corpus
   program the backend claims to support.

---

## Migration & compatibility

- **Phase 0 (mechanical).** Rename `Any → Dynamic`; widen `Int` to
  `Int{IntSpec}` with the v0 `Int` mapping to a documented default. No behaviour
  change; every existing module still lowers identically (all operands are
  `Dynamic` or the default int). Bump the SIR version.
- **Phase 1.** Land the conformance harness + reference oracle **against the
  current backends** using only `Arbitrary`/`Dynamic` — this codifies today's
  behaviour and gives us the regression net *before* adding sized integers.
- **Phase 2+.** Introduce sized integers and type-directed op selection one
  backend at a time (per the repo's one-PR-per-backend convention), each gated by
  P6. A frontend opts into emitting sized types only when a target proves it.

Nothing is released; per project policy we break the `SirType` shape freely
rather than shim it.

---

## Milestones

One PR per row; backend rows fan out in parallel after the core rows land.

| # | Scope | Content |
|---|-------|---------|
| T0 | `code/specs/` | this spec |
| T1a ✅ | `semantic-ir` | Phase-0 core: `Any→Dynamic` rename + `Int→Int{IntSpec}` (`IntSpec`/`IntWidth`/`Overflow`, derived bounds). Behaviour-preserving; serialisation byte-identical (surface keyword stays `any`, default int prints `int`); no version bump yet. *(Done — CHANGELOG 0.16.0.)* |
| T1b ✅ | `semantic-ir` | Additive source-fidelity types `Ptr`/`Struct`/`Optional` + seven new `Feature` flags (`SizedIntegers`/`Unsigned`/`WrappingArithmetic`/`FixedArrays`/`Pointers`/`Structs`/`Bignum`). Bumps `CURRENT_SIR_VERSION` `0→1` (first new text tokens). *(Done — CHANGELOG 0.17.0.)* |
| T1c | `semantic-ir` | Reshape existing containers to the SIR21 lattice: `Seq(elem)` → `Seq { elem, len: Option<u64> }` (fixed C arrays) and `Map(val)` → `Map { key, val }`. Deferred out of T1b because these *change* existing variant shapes (ripple to `Seq`/`Map` consumers) rather than adding new ones. |
| T2a ✅ | `sir-conformance` | Reference oracle (P2): a pure, toolchain-free `oracle` module — `eval(op, lhs, rhs, spec)` / `reduce(exact, spec)` computing the observable integer `Outcome` from `IntSpec` semantics (wrap/saturate/trap/checked/UB/arbitrary), unit-tested against the canonical constants. The independent authority backends are measured against. *(Done — CHANGELOG 0.6.0.)* |
| T2b ✅ | `sir-conformance` | Differential runner (P1): `tests/arithmetic.rs` derives each integer case's expected output from the oracle (never hand-typed) and asserts every backend matches byte-for-byte (9×4 = 36 runs, Phase-1 net). Added `run_source`/`lower_source`. Surfaced the confirmed bignum frontier (10²⁴ diverges on JS/Go/Rust). *(Done — CHANGELOG 0.7.0.)* |
| T2c ✅ | `sir-conformance` | Coverage gate (P5), arithmetic slice: `IntOp::ALL` enumerates the emittable ops; two gate tests assert every op has ≥1 case and every accepted `(op, backend)` cell is proven — accepted-but-untested fails CI. *(Done — CHANGELOG 0.8.0.)* |
| T2d | `sir-conformance` | Extend the coverage gate from the `(op × backend)` arithmetic grid to the full `SirType`/feature surface of the golden corpus (e.g. `case_eq`, strings), keyed on each backend's accept-manifest. |
| T3a ✅ | `semantic-ir` | Integer-reflection const-intrinsics: `int_const` module — `IntConst {Max,Min,Width}` with canonical names and `eval(spec) -> Option<i128>` that const-folds from the `IntSpec` bounds (`None` for arbitrary). Pure, additive, behaviour-preserving. *(Done — CHANGELOG 0.18.0.)* |
| T3b-1 ✅ | `sir-conformance` | Division **reference semantics**: `oracle::DivOp { Floor, Trunc }` with `eval` (floor→−∞, trunc→0), `Outcome::DivByZero`, `MIN/-1`→`BeyondOracle`; canonical names `div_floor`/`div_trunc`. `div_true` (float) deferred to a float oracle. Pure/additive, out of `IntOp::ALL` (coverage gate untouched). *(Done — CHANGELOG 0.9.0.)* |
| T3b-2 (in progress) | frontends + backends | Wire the split into the pipeline per §E3's own decisions: `div_floor`/`div_trunc`/`udiv_trunc`/`div_true` IR ops, `div_trunc`/`udiv_trunc` absorbing `tdiv`/`utdiv`. Additive backend rollout (all 7) first, then frontend migration (Ruby → `div_floor`; Python/JavaScript/MATLAB/Scilab/IDL → `div_true`; C → `div_true`+`div_trunc`/`udiv_trunc`; Twig stays on bare `"/"` permanently, see §E3), then a cleanup slice deleting the now-dead `tdiv`/`utdiv`. Division conformance cases judged by the T3b-1 oracle, plus a dedicated TypeScript execution test proving its floor-vs-truncate bug fix (see §E3). |
| T3c-1 ✅ | `semantic-ir` | Type-directed op-selection **rule**: `op_select::resolve_numeric(lhs, rhs) -> NumericLowering` (`Int(spec)` / `Float` / `RuntimeDispatch`) — the pure decision each backend consults to specialise a numeric `+`/`-`/`*` from operand types, or dispatch when `Dynamic`/mismatched. No inference, no behaviour change. *(Done — CHANGELOG 0.19.0.)* |
| T3c-2 ✅ | `semantic-ir` | Fold string-concat (`+` on `Str`) and comparison operators into the resolver: `resolve_binary(op, lhs, rhs) -> BinaryLowering` (`IntArith`/`FloatArith`/`StrConcat`/`TypedCompare`/`RuntimeDispatch`). Pure/additive; `/` deliberately excluded (division split, §E3). *(Done — CHANGELOG 0.20.0.)* |
| T3c-3-prereq ✅ | `semantic-ir` | Discovered while starting T3c-3: `resolve_binary`'s `Option<&SirType>` operand types have nowhere to come from at a `BuiltinCall`'s call site — `sir_type` lives on *declaration* sites (`Param`/`Capture`/`LetBinding`/`LetStarBinding`) only, not on `Expr::VarRef` or any literal. `type_env::TypeEnv` is the missing name→type lookup: seed from a function's params/captures, `observe_stmt` as a caller walks a block in lexical order, `expr_type` resolves a `VarRef` or returns `None`. Pure/additive, 9 unit tests, not yet consulted by any backend. *(Done — CHANGELOG 0.24.0.)* |
| T3c-3 (python) ✅ | `semantic-ir-to-python` | Have the emitter build a `TypeEnv` while walking a function body (threaded through the whole `emit.rs` call graph, including the block-as-expression walrus path and the lifted-nested-`def` scope boundary) and consult `resolve_binary` at every binary-`BuiltinCall` site before the existing runtime-dispatch fallback. Discovered mid-implementation: no shipped frontend's typed output can reach this backend today — `c-to-semantic-ir` is the only frontend populating `sir_type`, but this backend doesn't declare `Feature::Conversions` (only `semantic-ir-to-c`/`semantic-ir-to-ruby` do), so the validator rejects any `c-to-semantic-ir`-sourced module before it reaches emission; the wiring is behaviour-preserving and unit-tested (6 new tests) but has no accompanying real-frontend conformance case for that reason. *(Done — CHANGELOG 0.13.0.)* |
| T3c-3 (remaining: ts, js, go, rust, c, ruby) | backends | Same wiring, one backend at a time, mirroring the OOP arc's per-backend-PR discipline and the python row above — threading a type environment through a live emitter's full call graph is real, backend-specific work, not a one-line change. |
| T4–T8 | one per existing backend (py, ts, js, go, rust) | sized-integer lowering per the faithfulness table; add conformance cases; pass P6. JS/BigInt gets extra scrutiny. |
| T9 | `semantic-ir` Intrinsic | additive `fallback` field + validator + graceful-degradation path. |
| T10+ | new targets | Java, C#, C++, C backends (each its own spec + cascade) consume this type system from day one. |
| Tn | new sources | C-frontend (`c-to-semantic-ir`) populates sized types + `Ptr`/`Struct`; typed-Python and TS frontends emit types they already have. |

---

## What this spec is not

- Not a type *checker* or *inferencer* — SIR still carries, frontends decide.
- Not generics, traits, sum types, or refinement/range types (deferred).
- Not a memory model — `Ptr` records pointer *shape* for source fidelity, not
  aliasing/ownership semantics; lifting C's manual memory to a GC'd target drops
  frees and is covered by the eventual C-frontend spec.
- Not a change to SIR10's escape-hatch rules — only the additive `fallback` and
  the use of the richer type lattice on the intrinsic boundary.

## Deferred

- Refinement / range types (`Int in 0..=100`) — would let a backend prove a
  bignum value fits a machine width and specialise; powerful, but needs a
  narrowing analysis that borders on inference. Revisit after v1.
- Generics / parametric `Fn` and container polymorphism beyond one `elem`/`key`/
  `val` level.
- `i128`/`u128` on targets without native 128-bit (would need the bignum path).
- Decimal / rational towers (Ruby `Rational`, `BigDecimal`).
