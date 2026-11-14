# VER04k — Warrant v1, Wave 4 (WX-D, WX-F, WX-G)

**Status:** Specification only. Nothing here is implemented. The precondition,
the WX-H/WX-E slice of [VER04j](VER04j-warrant-v1-extensions.md) (branch
`claude/warrant-wx-h-e`, #19121), has merged, so every adapter shape, fault
mechanism and observer this document extends is on `main`. The WX-D1 errata and
decisions of the S0 review (section 9.1, 2026-11-11) are applied to the text
below; WX-D1 lands in the slices of section 8.1. Each extension below is a
separately landable slice (section 8). None changes the meaning of a v0
document, and none changes the meaning of a valid v1 document written for the
wave-3 tool (section 1); the two wave-3 v1 refusals this wave adds (list
literals above 4,096 elements, S0 E8, and identifiers longer than 128 bytes,
E20) are recorded in 1.2.

**Amendment (2026-11-14), with [VER04m](VER04m-warrant-stateful-objects.md):** three small
changes travel with the first slice of stateful objects (WX-S1). Section 4.3 now says
explicitly that `inout List<T>` admits `T` an admissible **record** (the `register` of the
4004 sequential layer is `&mut [FlipFlopState]`); section 4.6 gains the host forms
`slice-of:struct:<Name>`, `vec-of:struct:<Name>` and `codec:<Record>`, and says that with
`error=<CASE>` an S2 or S4 slot may be served by a host that returns the remaining components
directly (a panic is `Err` with no post-state), which is the shape of `d_flip_flop`; section 4.12's
last bullet now points to VER04m. Why: without them the sequential layer, `pixel-container` and
the CPU step cannot be bound. No existing rule is weakened and no v0 or wave-3 meaning changes;
a tuple is still not an admissible `inout` or record-field type.

**Parent:** [VER04 §2–§8](VER04-warrant-language.md),
[VER04d](VER04d-warrant-type-system.md), [VER04e](VER04e-warrant-codegen.md),
[VER04h](VER04h-warrant-derived-evidence.md),
[VER04i](VER04i-polyglot-warrant-evidence.md) (corpus encodings),
[VER04j](VER04j-warrant-v1-extensions.md) (v1 header, WX-B/C/A/E/H; its section 7
holds the non-normative sketches this document replaces),
[VER10](VER10-bit-blasting.md) and [VER19](VER19-symbolic-execution.md)
(symbolic discharge).

**Section references.** "VER04j §n" means the text of VER04j on
`claude/warrant-wx-h-e`, including its section 14 (implementation deviations),
which is the newest statement of what the wave-3 tool actually does. Where this
document needs a corpus rule it states a requirement on VER04i, as VER04j does
(VER04j §0, §12a); it does not edit VER04i.

## 0. Why this exists

After wave 3, a v1 document can say "this operation equals this bit-level model"
for straight-line algorithms, pin known answers, and bound output sizes. Three
shapes of real leaf-library API still cannot be stated at all:

| What cannot be said | Real API (Rust leaf packages) | Why v1 (wave 3) fails | Removed by |
|---|---|---|---|
| "this loop over the input equals that loop" | `gf256` multiplication (8 conditional xtime steps); `wasm-leb128::encode_unsigned` (up to ten 7-bit groups); SHA-family message schedules (64 or 80 rounds) | models are non-recursive and there is no iteration, so a model must be unrolled by hand (VER04j §0 records the `xor1..xor8` workaround of exactly this kind) | **WX-D** |
| "feeding the input in any split gives the one-shot answer" | `coding_adventures_sha1::Digest::{new, update, sum1}`, the same for sha256/sha512/md5; `range-coder::BoolEncoder::{new, write_bit, finish}`; `BoolDecoder::{new, read_bit}` | an `op` binds one export called once; a stateful object needs a constructor, many method calls and an observer | **WX-F** |
| "after the call, the buffer passed in holds this" | `coding_adventures_zeroize::zeroize_bytes(&mut [u8])`; `sqlite-file::varint::write(i64, &mut Vec<u8>) -> usize`; `tls13-alpn` `encode(self, &mut [u8]) -> Result<usize, _>` | an `op` result is the export's return value; a `&mut` argument's new contents are invisible, and `zeroize_bytes` returns nothing | **WX-G** |

The three are designed together because they share machinery: WX-F's
split-point sweeps need WX-D's `take`/`drop`/`slice`; WX-G's most common
postconditions (`result == to_bytes(replicate(0u8, len(buf)))`, "the old
contents are a prefix of the new") need WX-D's sequence built-ins; and all three
need list values to flow through generated evidence bodies, which wave 3 still
refuses (VER04j §14 items 24 and 27: "list literals ... inside evidence bodies
remain W0701").

| Ext | Adds | Main packages unlocked | Size | Risk |
|---|---|---|---|---|
| WX-D | `take drop slice replicate range to_list to_bytes` (D1); `fold map count all any` over `List` and `Bytes` with first-order function references (D2); bounded symbolic unrolling (D3) | wasm-leb128 (general encode model), gf256 (`gmul` as a loop), SHA-family reference models, every WX-F/WX-G postcondition below | M (three slices) | medium-high |
| WX-F | binding-level sequence drivers: a constructor, a step method driven by a sequence parameter, a finish observer; obligations and sequence faults | sha1/sha256/sha512/md5 streaming digests, range-coder encoder and decoder, rans (with a context object, open question 7) | M | medium |
| WX-G | `inout` parameter mode, the post-state slot rule, `ensures` blocks with `result`, the `&mut` adapter, monitored postconditions | zeroize, sqlite-file varint writer, tls13-alpn/server-name encoders into caller buffers, hkdf `expand_fixed` | S-M | medium |

The order of presentation (D, F, G) is the dependency order, not the order in
VER04j's survey table.

## 1. Versioning and compatibility (applies to every extension)

### 1.1 Header

Nothing changes: a document is v1 exactly when its header is `warrant name v1`
(VER04j §1.1). There is no `v2` and no per-feature switch; open question 3 asks
whether v1 should be frozen before this wave lands.

### 1.2 Every new surface element and its treatment in v0

The rule of VER04j §1.2 holds: no new reserved words, no new tokens, no new
multi-character operators. A v0 document means exactly what it did.

| Element | Kind | In a v0 document | In a v1 document |
|---|---|---|---|
| `fold map count all any take drop slice replicate range to_list to_bytes` | v1 built-in function names (WX-D) | ordinary identifiers. A call of an undeclared one is **W0202** (unbound name), exactly as for `bit_xor` today (VER04j §14 item 9 and the checker's `V1_FUNCTIONS` gating); a v0 model *named* `count` keeps meaning its own definition | reserved, all twelve, from WX-D1 on (S0 E7): a v1 model, op, binder or constructor of that name is **W0201**/**W0203** as for every other v1 built-in. Until WX-D2 lands, a call (or any other use) of `fold`, `map`, `count`, `all` or `any` is **W0202** with the message "reserved built-in `<name>` is not yet implemented in this version" |
| `and`, `or`, `=>` in the ground evaluator | evaluation rule (S0 E2) | **eager**, unchanged: both operands are evaluated and charged. This is a recorded deviation from VER04 §4, kept so that v0 output stays byte-identical (2.6) | **short-circuit**, as VER04 §4 says and as the generated code already does |
| a list literal of more than 4,096 elements | checker cap (S0 E8) | accepted by the checker, unchanged (the ground evaluator still refuses more than 65,536 elements with W0703) | **W0602** at checking, anywhere in the document |
| an identifier longer than 128 bytes (`MAX_IDENTIFIER_BYTES`) | checker cap (E20) | accepted, unchanged: v0 names stay unbounded | **W0602** at checking, in any position (a declaration, a parameter, a field, a constructor, an error case, a binder, any segment of a reference) |
| a list literal around an operation call (`to_bytes([f(a)]) == b`) | operation discovery (S0 E13) | unchanged: discovery does not look inside list literals | discovery descends into list-literal elements (2.11) |
| a function reference `f` or `f(e, ...)` in the function position of a WX-D call | checker rule, no syntax | unreachable (the WX-D names are unbound) | W0712 if it is not a first-order model reference (section 2.3) |
| list literals and WX-D forms inside generated evidence bodies | generator capability | v0 generation unchanged (still W0701 for list literals) | translated (section 2.11) |
| `inout` before a parameter type (`buf: inout Bytes`) | contextual word (WX-G) | **W0101** (`V1ConstructInV0`); no valid v0 text has two NAMEs after `:` | parameter mode |
| `ensures NAME:` as a declaration head | reserved word `ensures` in a new position (WX-G) | **W0101**. `ensures` is already reserved (VER04 §2) and was only valid as the last row of an `input` block, so no valid v0 document contains it at declaration level; such a document used to be a W0001/W0102 syntax error and becomes W0101, which changes no valid document (the WX-H precedent, VER04j §14 item 28) | postcondition block |
| `result` inside an `ensures` row | contextual binder | unreachable | the operation's return value |
| `driver` lines and the `driver` option of an `op` line | binding schema (WX-F), no document syntax | a driver bound to an operation of a v0 document is **W0711** (the v0 generated harness is frozen byte-identical, VER04j §1.4) | allowed |
| `inout=` binding option | binding schema (WX-G) | unreachable (no `inout` parameter can exist) | required for every `inout` parameter |

Adding reserved names to v1 is a breaking change for any v1 document that used
one of them as a model name. The four in-repository v1 documents
(`warrant-grammar/tests/fixtures/{wx_fixed,wxa,wxh,wxv}.warrant`) use none of
them (checked when this specification was written; the WX-D1 slice re-checks
mechanically, section 2.15). This is the same trade VER04j made for `imin`/`imax`.
All twelve names are reserved in the first slice (WX-D1), not five of them later
in WX-D2, so that the break happens once (S0 E7). Reserving them now answers
only that much of open question 3: it does not freeze v1. The new list-literal
cap (S0 E8) is the same kind of break for a v1 document holding a literal of
4,097 to 65,536 elements, which the wave-3 checker accepted; none of the four
documents has a list literal at all. The identifier cap (E20) is the same kind
of break for a v1 document with a name longer than 128 bytes; the longest
identifier in the four documents is 53 bytes (in `wxv`).

### 1.3 What "means the same" is tested by

1. Every v0 test of VER04j §1.3 still passes byte-identically (`ct-compare`,
   `gf256`; `tests/v0_compat.rs`).
2. **Wave-3 v1 documents regenerate byte-identically.** The generated evidence of
   `wx_fixed`, `wxa`, `wxh` and `wxv` must not change. Every new prelude part
   (the meter, sequence adapters, driver loop, inout adapter, postcondition
   monitor) is gated like the `comp`/`obs` parts of VER04j §14 items 25 and 33:
   emitted only for a document that uses the feature.
3. The checker's `Debug` output for the two checked-in v0 specifications is
   unchanged (VER04j §14 item 26). No slice adds a field to
   `CheckedWarrantModule`: new checker data stays inside the private checker
   (the lesson recorded when WX-A broke this gate).
4. The ground evaluator's v0 behaviour is unchanged, including its eager
   `and`/`or`/`=>` (2.6) and the absence of a data counter (2.7). Every v1-only
   evaluator change (short-circuit, the data counter, the WX-D forms) is keyed on
   the document version, exactly as the `Int` overflow code is (VER04j §2.5).

### 1.4 Diagnostics

Existing codes keep their meaning and are reused wherever the condition is one
they already name. Three codes are new; each marks a *boundary rule* of one
extension, so that the rejection is a stable, separately testable class instead
of one more meaning of the heavily overloaded W0301. `W0711`–`W0713` were checked
to be unused in VER04, VER04d, VER04e, VER04h, VER04i, VER04j and in the code on
`claude/warrant-wx-h-e` (the highest code in use is W0710).

| ID | Phase | Trigger |
|---|---|---|
| **W0711** | generation (binding against the checked document) | A WX-F driver is inconsistent with its operation: driven parameter not `List<T>`/`Bytes`/`Nat`, an element path that does not exist, a host type that cannot carry the Warrant type at that path, a non-driven parameter that no constructor/step argument consumes, a result mode that does not produce the declared result type, a fallible step on an operation whose result is not `Result`, static parameters, proof obligations (secret labels, bound rows) without `proofs_not_live=driver`, or a driver on an operation of a v0 document |
| **W0712** | checking | The function argument of a WX-D form is not a first-order model reference: it names an operation, a built-in, a constructor or a binder; it is not a bare name or a name applied to bound arguments; the model has static parameters; arity or tail types do not match; or the model, or a model it reaches, contains a quantifier or calls an operation |
| **W0713** | checking | A WX-G rule: an `inout` parameter whose post-state has no admissible slot in the result type, an inadmissible `inout` type, more than 4 `inout` parameters, `inout` on a `fresh` operation, or an `ensures` row that calls an operation or contains a quantifier |

Reused codes (each row cites the section that uses it):

| ID | New uses |
|---|---|
| W0101 | `inout` and `ensures NAME:` in a v0 document (1.2) |
| W0201 | two `ensures` blocks for one operation; a v1 declaration named like a WX-D built-in |
| W0202 | a WX-D built-in called in a v0 document that does not define it; until WX-D2, any use of `fold`, `map`, `count`, `all` or `any` in a v1 document ("not yet implemented in this version", S0 E7) |
| W0203 | a cycle through a function reference (2.8); an operation parameter named `result` on an operation with an `ensures` block (4.3) |
| W0301 | ordinary type errors inside WX-D calls, `ensures` rows and `inout` signatures |
| W0401 | `ensures` naming an undeclared operation |
| W0501 | a malformed `driver` line or `inout=` option, or a binding-file cap exceeded (section 6.3) |
| W0502 | a driver operation whose `faults=` omits an applicable driver fault (3.6) |
| W0601 | a driver operation that no claim or example drives with two or more non-empty elements (3.5); an `ensures` row contradicted by the operation's own example (4.7) |
| W0602 | higher-order nesting deeper than 4 (2.8), a list literal longer than 4,096 elements in a v1 document, at checking (2.11, S0 E8), an identifier longer than 128 bytes in a v1 document, at checking (E20), more than 32 `ensures` rows (4.2), a symbolic unrolling that exceeds its caps (2.10) |
| W0701 | a fault that does not apply to the operation's shape (for the list result faults, 2.12); a lane that has no driver or inout adapter (target `not-live`) |
| W0703 | ground-evaluator exhaustion of any counter or cap of the v1 meter (2.7): work, depth, sequence elements, composite leaves or data, inside or outside a WX-D form |
| W0704 | an `inout` host form that cannot carry the parameter's Warrant type |
| W0706 | a split or hint domain over the exhaustive budget (2.9, 3.7); a split-domain binder whose count flows into conflicting domain rules (2.9, S0 E9) |

## 2. WX-D — bounded sequence combinators

### 2.1 Motivation

* **wasm-leb128.** `encode_unsigned(v: u64) -> Vec<u8>` emits 7-bit groups until
  the rest is zero. Without iteration the only honest model is ten nested
  `if`s. With `fold` over `range(10)` the model is the textbook loop (below).
* **gf256.** `gmul(a, b)` is eight conditional xtime-and-xor steps. VER04j §7.2
  named "`gmul` without unrolling (`fold` over `range(8)`)" as a target.
* **SHA-family reference models.** A compression function is a fold of a round
  function over `range(64)` (or 80) on an 8-word state tuple (WX-A), after a
  message schedule that is a fold over `range(48)` appending words.
* **Every WX-F and WX-G postcondition** that talks about "the first k bytes",
  "all zero", "the old contents followed by the new bytes".
* **Bounded quantification that evaluates.** `forall i: Nat | i < len(xs). p(xs, i)`
  is an unbounded quantifier whose exhaustive domain is `0..=64` (VER04h §3); the
  evidence generator cannot translate it inside a model body (W0701, VER04j §14
  item 24) and an `ensures` row may not contain it (4.2). `all(range(len(xs)), p(xs))`
  means the same when the refinement bounds the binder, and is evaluable
  everywhere (2.9).

Newly expressible, as the canonical example (an excerpt: the labels, input and
bound contracts every operation needs are omitted; a body that spans lines is
parenthesised, because a newline outside brackets ends a declaration, VER04 §2):

```text
warrant leb_excerpt v1
  op encode_unsigned(v: U64) -> Bytes fresh

  -- One 7-bit group per step. The state is (rest, out, done).
  model group(v: U64) -> U8 = wrap<U8>(to_int(bit_and(v, 127u64)))
  model leb_step(st: (U64, Bytes, Bool), i: Nat) -> (U64, Bytes, Bool) = (
    if st.2 then st
    else if shr(st.0, 7) == 0u64 then (0u64, concat(st.1, be_bytes(group(st.0))), true)
    else (shr(st.0, 7), concat(st.1, be_bytes(bit_or(group(st.0), 128u8))), false))
  model leb_encode(v: U64) -> Bytes = fold(range(10), (v, b"", false), leb_step).1

  claim encode_matches_model:
    forall v: U64. encode_unsigned(v) == leb_encode(v)
```

and the gf256 multiplication (`xtime` as in VER04j §3.1), which needs one bound argument (`b`) inside the
step, written as a *section* (2.3):

```text
  model gmul_step(b: U8, st: (U8, U8), i: Nat) -> (U8, U8) = (
    (if bit(b, i) then bit_xor(st.0, st.1) else st.0), xtime(st.1))
  model gmul(a: U8, b: U8) -> U8 = fold(range(8), (0u8, a), gmul_step(b)).0
```

### 2.2 Built-ins

`Seq` stands for `List<T>`, `Bytes` or `Bytes<N>`; `elem(List<T>) = T` and
`elem(Bytes) = elem(Bytes<N>) = U8`. A count argument has type `Int` (a `Nat`
widens, as everywhere in v0). Every form is total.

| Form | Arguments | Result | Total semantics |
|---|---|---|---|
| `fold(xs, init, f)` | `xs: Seq`; `f` a function reference whose last two parameters are `(A, elem)` and whose result is `A`; `init` checked with expected type `A` | `A` | `acc := init`; for each `x` of `xs` in index order, `acc := f(..., acc, x)`; the result is `acc` |
| `map(xs, f)` | `f` with last parameter `elem`, result `U` | `Bytes<N>` if `xs: Bytes<N>` and `U = U8`; `Bytes` if `xs: Bytes` and `U = U8`; otherwise `List<U>` | `f` applied to each element in index order |
| `count(xs, p)` | `p` with last parameter `elem`, result `Bool` | `Nat` | number of elements with `p(x)`; every element visited |
| `all(xs, p)` | as `count` | `Bool` | `true` iff every element satisfies `p`; stops at the first `false`; `all` of an empty sequence is `true` |
| `any(xs, p)` | as `count` | `Bool` | `true` iff some element satisfies `p`; stops at the first `true`; `any` of empty is `false` |
| `take(xs, n)` | `xs: Seq`, `n: Int` | `List<T>` for a list, `Bytes` for `Bytes`/`Bytes<N>` | the first `clamp(n)` elements, where `clamp(n) = max(0, min(n, len(xs)))` |
| `drop(xs, n)` | as `take` | as `take` | the elements from index `clamp(n)` on |
| `slice(xs, s, n)` | `xs: Seq`, `s, n: Int` | as `take` | `take(drop(xs, s), n)` |
| `replicate(x, n)` | `x: T`, `n: Int` | `List<T>` | `max(0, n)` copies of `x` |
| `range(n)` | `n: Int` | `List<Nat>` | `[0, 1, ..., n - 1]`; empty for `n <= 0` |
| `to_list(b)` | `b: Bytes` or `Bytes<N>` | `List<U8>` | the bytes as a list, in order |
| `to_bytes(xs)` | `xs: List<U8>` | `Bytes` | the list as bytes, in order |

**Negative counts mean zero.** `take(xs, -1) == []` and `range(-3) == []`. This is
a deliberate, total choice that differs from Python's negative slicing
(`xs[:-1]` drops the *last* element) and from JavaScript's `slice` (negative
counts from the end); section 2.13 makes lanes implement it explicitly. It also
closes, for the sequence built-ins, the gap of VER04j §14 item 5: `Nat + Nat` is
`Int` in the checker, so `drop(xs, len(xs) - 1)` would otherwise be a type error.
Open question 2 recorded the alternative; the owner accepted `Int` counts with
negatives as zero, as drafted (section 9.1, E-Q2).

**Typing in context.** Every count, start and length argument is checked with
expected type `Int`, and `replicate`'s element and `to_bytes`'s list are checked
with the expected type their position implies (2.4), so a literal such as `[]` or
`3` takes its type from the call. The checker threads that expected type into
the v1 built-in rule (`typecheck_v1_builtin` takes none today); this is the
"checked with expected type" of 2.4, not a new rule.

**Why no `Bytes<N>` result for `take`.** The length of `take(b, n)` depends on a
runtime `n`; `fixed<N>` recovers a static width with an explicit `Option`, as in
v0. `map` keeps `Bytes<N>` because a length-preserving map has a static length.

There is deliberately no `zip`, `filter`, `sort`, `reverse` or indexed fold:
`range(len(xs))` with `at` gives indices, `count` covers filtering for the
claims in view, and each further form is another evaluator, another lane IR tag
and another differential test (open question 13).

### 2.3 Function references (first-order sections)

The third argument of `fold` and the second of `map`, `count`, `all` and `any` is
a **function reference**, which is not an expression and has no type:

```text
fnref = NAME                                   -- a model of exactly the required arity
      | NAME "(" expression { "," expression } ")"   -- a section: k bound leading arguments
```

Let `model f(q1: Q1, ..., qm: Qm) -> R` be the referenced declaration and `r` the
arity the form requires (2 for `fold`, 1 otherwise). The reference is well formed
iff every rule below holds, and each failure is **W0712**:

1. `f` resolves to a declared `model` function. An operation, a built-in, a
   constructor, an error case, a binder or an alias is rejected. Operations are
   excluded on purpose: a WX-D form never calls the subject per element, so the
   number of operation calls an expression makes stays visible in its text
   (VER04i §4.4 counts them as `expected_calls`).
2. `f` has no static parameters.
3. `m = k + r`, where `k` is the number of bound arguments (`0 <= k <= 8`).
4. Each bound argument `ei` type checks against `Qi` with the v0 subtyping
   (`Nat <= Int`, `Bytes<N> <= Bytes`), exactly like an ordinary call argument.
5. The remaining parameter types equal the required types **exactly**
   (subtyping is not applied to element or accumulator positions, as VER04j §4.4
   does not lift it through composites): for `fold`, `Q(k+1) = R` and
   `Q(k+2) = elem(xs)`; for the others `Q(k+1) = elem(xs)`, and `R = Bool` for
   `count`/`all`/`any`.
6. `f`, and every model reachable from `f` through calls and function
   references, contains no quantifier and calls no operation. Model bodies
   cannot call operations anyway (VER04d §4.1); the quantifier rule keeps every
   per-element evaluation ground and finite.

A section is not a closure and creates no function value. It is
defunctionalised: the bound arguments are ordinary arguments that the form
passes to every call. There is no partial application anywhere else, and a bare
model name is still an error in any other expression position (as in v0).

**Why sections at all.** Without them a step cannot see anything but the
accumulator and the element, so `gmul_step` would have to carry `b` inside the
accumulator (`fold(range(8), (0u8, a, b), step).0`). That is expressible with
WX-A tuples but unreadable, and for `map`, which has no accumulator, it is
impossible without a `zip`. Open question 1 asks the owner to confirm sections
over the bare-name rule of the VER04j sketch.

### 2.4 Typing rules

Written as judgments over the checker environment `Γ`; `≤` is the v0 subtyping
and `=` is type equality.

```text
Γ ⊢ xs : S    S ∈ {List<T>, Bytes, Bytes<N>}    fnref f(e1..ek) : (A, elem(S)) → A    Γ ⊢ init ⇐ A
────────────────────────────────────────────────────────────────────────────────────────
Γ ⊢ fold(xs, init, f(e1..ek)) : A

Γ ⊢ xs : S    fnref f(e1..ek) : (elem(S)) → U
──────────────────────────────────────────────  (result: Bytes<N> / Bytes when S is one and U = U8, else List<U>)
Γ ⊢ map(xs, f(e1..ek)) : M(S, U)

Γ ⊢ xs : S    fnref p(e1..ek) : (elem(S)) → Bool
──────────────────────────────────────────────────
Γ ⊢ count(xs, p(..)) : Nat     Γ ⊢ all(xs, p(..)) : Bool     Γ ⊢ any(xs, p(..)) : Bool

Γ ⊢ xs : S    Γ ⊢ n : Int          Γ ⊢ xs : S    Γ ⊢ s : Int    Γ ⊢ n : Int
──────────────────────────────     ─────────────────────────────────────────
Γ ⊢ take(xs, n) : D(S)             Γ ⊢ slice(xs, s, n) : D(S)        (D(List<T>) = List<T>; D(Bytes<N>) = D(Bytes) = Bytes)

Γ ⊢ x : T    Γ ⊢ n : Int           Γ ⊢ n : Int                Γ ⊢ b : Bytes            Γ ⊢ xs : List<U8>
──────────────────────────         ─────────────────────      ───────────────────      ─────────────────────
Γ ⊢ replicate(x, n) : List<T>      Γ ⊢ range(n) : List<Nat>   Γ ⊢ to_list(b) : List<U8>  Γ ⊢ to_bytes(xs) : Bytes
```

`init ⇐ A` means "checked with expected type `A`", so a literal takes its
contextual type: in `fold(range(10), (v, b"", false), leb_step)` the literal `b""`
is a `Bytes`, not a `Bytes<0>`, and the tuple `(U64, Bytes, Bool)` equals the
accumulator type even though tuples are invariant. `replicate(x, n)` with an
empty-list literal for `x` needs an expected element type, as `[]` does in v0.

Equality between a `List<U8>` and a `Bytes` is W0301 (they are different types,
VER04 §3); convert with `to_list`/`to_bytes`.

### 2.5 Grammar

No production changes. Every form is a call (`reference = qualified_name [ "<"
widths ">" ] [ "(" args ")" ]`). The checker treats the function position of the
five higher-order forms specially: the argument expression there must be a bare
NAME or a NAME applied to arguments, and is resolved as a function reference
rather than evaluated. Any other expression in that position (an `if`, a
literal, a projection, a nested WX-D call) is W0712. This is a checker rule, not
a parser rule, so the token and grammar files are unchanged and no valid v0 or
wave-3 v1 text changes meaning.

### 2.6 Evaluation semantics

* **Order.** Arguments are evaluated left to right (VER04 §4). The bound
  arguments of a section are evaluated **once**, after `xs` and `init` and before
  the first element, and **even when `xs` is empty**, so their charges and any
  exhaustion they cause do not depend on the data.
* **Iteration, not recursion.** Every implementation must iterate. A fold makes
  one model call per element, entered and left in turn, so the evaluation depth
  it reaches (2.7) is fixed by the static nesting of the expressions and calls,
  never by the length of `xs`. An implementation that recurses on the tail adds
  at least one depth level per element, so it exceeds the 64-level limit within
  the first 64 elements and is non-conforming (the differential tests of 2.15
  include 1,000-element folds).
* **Short-circuit.** `all` and `any` stop at the deciding element. Elements after
  it are neither evaluated nor charged. `count`, `map` and `fold` visit every
  element.
* **Short-circuit of `and`, `or` and `=>` (S0 E2).** In a v1 document every
  evaluator evaluates the right operand only when the left one does not decide
  the result: `false and q`, `true or q` and `false => q` do not evaluate `q`, and
  `q` is neither charged nor able to exhaust or fail. This is what VER04 §4 says
  and what generated Rust (`&&`, `||`) already does; the ground evaluator on
  `main` evaluates both operands. The fix is **v1 only**. The v0 ground evaluator stays
  eager, a deviation from VER04 §4 recorded there, because changing it would
  change v0 charges and diagnostics and v0 output must stay byte-identical (1.3).
  Eager evaluation can only add work, exhaustion or an evaluation error to what
  short-circuit evaluation does; when both succeed they give the same Bool, so the
  v0 deviation never accepts what VER04 §4 would reject. The fix changes no
  wave-3 v1 output: an evaluation that succeeded eagerly gives the same value
  short-circuited, and one that failed was refused at generation.
* **Sequences are values.** `take`/`drop`/`slice` produce new values. An
  implementation may share storage internally, but charges as if it copied
  (2.7), so every implementation reaches exhaustion at the same point.
* **Values.** No new value constructor. `List` and `Bytes` are the existing
  `EvalValue`/`V` variants.

### 2.7 Cost accounting

This is the design VER04j §7.2 deferred. One meter, three implementations (the
ground evaluator, the generated model code, each lane's IR evaluator), identical
charges.

**The meter.** One evaluation owns one meter: an example's expected value; one
claim instance, that is one assignment of the binders, including its admission
predicate (below) and every result conversion and postcondition check it
performs; one table row; one synthesized-suite instance. The meter has three
counters and two per-value caps:

| Counter or cap | Limit (reference profile) | Source of the limit |
|---|---|---|
| work units | 1,000,000 | VER04 §2, VER04e §2.3 |
| evaluation depth: nested node evaluations, across model calls | 64 levels | VER04 §2 ("64 active model/expression frames"), as the ground evaluator implements it (`max_depth`, S0 E1) |
| owned data, cumulative | 64 MiB = 2^26 bytes (`DATA_BUDGET_BYTES`) | VER04 §2 ("64 MiB of owned intermediate/output data") |
| elements in any one sequence value (`List` or `Bytes`) | 65,536 (`MAX_SEQ_ELEMENTS`) | the ground evaluator's per-sequence check against `max_value_bytes` (list literals, `concat`); now also the generated code's |
| leaves of any one tuple, record or constructor value | 65,536 (`MAX_VALUE_LEAVES`) | the ground evaluator's `bounded()` check of `leaves()` (VER04j §14 item 18); now also the generated code's |

The new constants are constants of the meter, not fields of `CodegenLimits`,
whose shape stays as it is (section 8.1).

**Evaluation depth (S0 E1).** The depth counter is the ground evaluator's, not a
count of model frames. Every node evaluation (a literal, a reference, a call, an
operator, `if`, `match`, a tuple, a projection, a list literal, a parenthesised
group) enters one level and leaves it when the node's value is known, and a
model call's body is evaluated one level below the call node. So the depth
inside a model is the depth of the call plus the nesting within the body.
Entering level 65 is exhaustion. For the WX-D forms, the bound arguments of a
section are evaluated one level below the form node, each per-element model call
occupies one level below the form node, exactly where a call node would be, and
the callee's body is one level below that. VER04 §2 says "model/expression
frames"; nesting inside one body counts, so this is the reading that matches it
and the evaluator that already exists. Generated code keeps the same counter.

**Parenthesised groups (S0 E3).** A parenthesised expression is a node. It
charges 1 work unit and one depth level, as the ground evaluator does. The
generator drops the parentheses when it emits Rust, so it emits the charge and
the depth step for every group explicitly, under the meter. `((x))` costs 3
units and 3 levels in every implementation.

**Two caps, not one (S0 E6).** The element cap bounds every `List` and `Bytes`
value a node produces. It is checked before the allocation, from the length the
node knows in advance (the check-before-allocate order below), and it is the only
size cap on a sequence. The leaves cap bounds every tuple, record and
constructor value. It is checked where the ground evaluator checks it today: after
the components exist and before the composite is returned. The composite's own
allocation is one slot per component, so this check comes after no unbounded
allocation. The leaves cap is not applied to lists. A list of 65,536 scalars has
65,537 leaves, so applying it would quietly lower the element cap to 65,535; the
data counter bounds what a list's elements hold. Generated code checks both caps
at the same points.

*Owned data* is measured as **cumulative allocation, never refunded**, for
determinism: live-data accounting would depend on when each implementation frees
memory. Cumulative allocation over-approximates live data, so the cap remains a
sound bound. (The owner accepted this as drafted: section 9.1, E-Q4.)

**Data size.** The byte cost of a value, `dsize(v)`, makes VER04h §3's cost
model exact:

| Value | `dsize` |
|---|---|
| `Bool`, `Nat`, `Int`, a fixed-width integer, `None` | 0: its storage is the slot that holds it |
| `Bytes` or `Bytes<N>` of length `L` | `L` |
| a `List` or a tuple with components `c1 .. cn` | `Σ (VALUE_OVERHEAD + dsize(ci))`, `VALUE_OVERHEAD = 32` |
| a constructor `C` of type `T` (a record, or a variant constructor with or without fields), with fields `c1 .. cn` | `len(T) + len(C) + Σ (VALUE_OVERHEAD + dsize(ci))` (E20) |
| `Err(E.CASE)`, and the error value `E.CASE` | `len(E) + len(CASE)` (E20) |
| `Some(x)`, `Ok(x)` | `VALUE_OVERHEAD + dsize(x)` |

`len(T)`, `len(C)`, `len(E)` and `len(CASE)` are the byte lengths of the names
as written in the document (E20). The ground evaluator's constructor and error
values own those names, so every copy allocates them again; a name priced at 0
let `replicate(C, 65536)` with a 10,000-byte `C` allocate about 660 MB while the
meter saw 2 MiB. With the names counted, the charge is never below what a copy
allocates beyond its fixed-size slot. (The slot itself is larger than
`VALUE_OVERHEAD` in the ground evaluator, 96 bytes on a 64-bit host, so the real
footprint of a value is within a small constant factor of its `dsize`, about 3×
at worst for a list of scalars.) Every implementation charges the names,
generated code included, so the meter stays identical across them.

A `List<U8>` of `L` elements is `32 L` bytes, a `Bytes` of `L` bytes is `L`,
and a `List<Bytes>` counts its elements' payloads as well as their slots. The
first draft charged a list only for its slots, assuming "the element's own
payload was charged when that element value was produced". That assumption does
not hold for a form that copies its elements, and the ground evaluator's
`EvalValue` copies deeply. So it is withdrawn (S0 E5).

**Which nodes charge data (S0 E4).** In a v1 evaluation, every evaluated node
charges `dsize` of its value, except the *pass-through* nodes. A pass-through node
returns a value that already exists and charges no data. The pass-through nodes
are a binder reference (including a dotted projection path), a projection, `if`,
`match`, a group, a model call (its body's nodes charge), a call of `unwrap_or`,
and `fold` (its value is the last accumulator). A node whose value is a scalar
charges 0 by the table above, so the rule needs no list of scalar built-ins. The
nodes that charge something are therefore exactly those that build a sequence or
composite: list literals, tuples, records and constructors (with or without
fields, since a constructor stores its names, E20), error values `E.CASE`, `Some`,
`Ok`, `Bytes` literals, `concat`, `be_bytes`, `le_bytes`, `fixed<N>`, `at` (the
`Some` around the element), the `Option`-valued v1 built-ins (`try`,
`checked_*`, `div`), the seven WX-D1 forms and `map`.

This over-charges on purpose. A tuple is charged again for components that were
charged when they were built. `take`, `drop`, `slice` and `replicate` are charged
for full copies of their elements, which is what the ground evaluator allocates;
an implementation that shares storage charges the same ("charges as if it
copied", 2.6). Reading a value charges nothing. A binder reference or a
projection may copy in one implementation and share in another. A
charge on reads would make the meter depend on that choice, and on how often a
body mentions a name. The memory a copying implementation spends on reads is
therefore not bounded by the data counter. It is a pre-existing property of the
ground evaluator, not of this wave, and an **open hazard**: section 9.1's note
on E4 gives the measured cost and the intended fix (backlog WX-RD1).

**When data is charged.** A node that knows its result length in advance
reserves its data in step 2 of the order below. Every other node charges as soon
as its components exist and before it builds its value: for a tuple, list
literal or constructor the components are already values, so `dsize` of the
result is a sum over them, and for `concat` it is `dsize(a) + dsize(b)`.
Computing `dsize` is linear in the value. Each slot it visits adds at least 32
bytes to the charge, so the cost of the computation is bounded by the data
budget. An implementation may stop summing as soon as the sum exceeds the
remaining budget, since the outcome is the same exhaustion.

**Scope of the data counter.** The ground evaluator keeps the data counter for
every evaluation of a v1 document, with or without WX-D forms. It resets the
counter wherever it resets the work counter. A v0 evaluation has no data
counter, as before (1.3). In generated code the whole meter, the data counter
included, exists only under the `seq` gate (below).

**Admission inside the meter (S0 E14).** A claim instance's meter starts before
its binders' admission predicates (type refinements and the `|` filters of the
quantifier) are evaluated, and their charges count. There is one meter per case:
in an exhaustive suite, each enumerated assignment; in a property or fuzz suite,
each drawn candidate. A candidate that the predicate rejects is discarded with
its meter. Exhaustion while the predicate is evaluated is **not** a rejection:
the instance fails with `model-budget-exhausted`, as for exhaustion in the body.
Otherwise a predicate that exhausts would silently remove cases from the domain.

**Charges** (units), in addition to the existing per-node charges of VER04e §2.3
and VER04j §2.5/§3.5/§4.5:

| Form | Work | Data, reserved before allocation |
|---|---|---|
| any node, including a WX-D call node and a parenthesised group | 1 | `dsize` of its value, unless it is pass-through (above). The rows below spell that one charge out for the forms that reserve it; it is charged once, not once per row |
| list literal `[e1, ..., en]` | + `n` (the ground evaluator's existing charge) | `Σ (32 + dsize(ei))` |
| `concat(a, b)` | + `len(a) + len(b)` (existing) | `dsize(a) + dsize(b)` |
| `fold`, `map`, `count`, `all`, `any` | per **visited** element: 1 (iteration) + 1 (the model call, as for any call) + the callee body's own charges | `map`: the result container, `len(xs)` × 32 for a `List` result and `len(xs)` for a `Bytes` result, reserved before the first element (each element value is charged by the body nodes that build it) |
| section bound arguments | their own expression charges, once | their own |
| `take`, `drop`, `slice` | + `L`, the result length | `L` for a `Bytes` result; for a `List` result, `Σ (32 + dsize(e))` over the `L` elements selected (S0 E5) |
| `replicate(x, n)` | + `max(0, n)` | `max(0, n) × (32 + dsize(x))` (S0 E5) |
| `range(n)` | + `max(0, n)` | `max(0, n) × 32` |
| `to_list(b)` | + `len(b)` | `len(b) × 32` |
| `to_bytes(xs)` | + `len(xs)` | `len(xs)` |

**Check before allocation.** Every form that produces a sequence knows its result
length `L` before producing it (`len(xs)` for `map`, `clamp(n)` for `take`,
`max(0, n)` for `replicate`/`range`). The implementation must, in this order:
(1) check `L <= MAX_SEQ_ELEMENTS`; (2) compute the data reservation of the table
above from `L` and the source elements (a sum over the selected elements for
`take`/`drop`/`slice` of a list), with checked arithmetic, and charge it; (3)
charge the work; (4) only then allocate. A `replicate(0u8, 10^12)` therefore
fails at step 1 having allocated nothing, and a count read from hostile data
cannot make any implementation allocate first and fail later. `n` is an exact
`Int`, so the comparison in step 1 must be done on the exact value (no narrowing
of `n` to a host integer before the check).

**Exhaustion** is never a truncated value, a skipped element, a skipped case or a
proof:

| Implementation | Exhaustion is |
|---|---|
| ground evaluator (examples, tables, generation-time checks) | **W0703** (v1), as VER04j §2.5 already uses for `Int` overflow |
| generated model code (exhaustive, property, fuzz, table and synthesized suites) | the instance **fails** with the registered failure id its suite runner uses and the note `model-budget-exhausted` (the WX-H precedent for notes: VER04j §14 item 34). A document whose claim cannot be evaluated within budget does not get a passing suite |
| a lane's IR evaluator | the same as generated model code; in practice unreachable, because the corpus generator refuses to emit any case on which the Rust evaluation exhausted (W0703 at corpus generation, section 2.13) |

**What the budgets admit** (illustrative arithmetic, not normative):

| Workload | Approximate cost | Fits? |
|---|---|---|
| `count(b, is_zero)` over 65,536 bytes, `is_zero(x) = x == 0u8` | 5 per element (iteration, call, and the three body nodes `==`, `x`, `0u8`) ≈ 327,680 work; no data (scalars) | yes; the element cap, not work, is the limit |
| a fold over 65,536 elements with a body of `c` charged nodes | `65,536 × (2 + c)` work | yes for `c <= 13` |
| `gmul` via `fold(range(8), ...)` | ≈ 130 work per instance | yes (65,536 instances of a `U8 × U8` claim each get their own meter) |
| `leb_encode(v)` (2.1) | ≈ 10 × 25 ≈ 250 work | yes |
| one SHA-256 compression (schedule + 64 rounds) | ≈ 3,500 work per 64-byte block | ≈ 280 blocks ≈ 18 KiB of message; property inputs are at most 64 bytes (two blocks with padding) |
| `replicate(b, n)` for a 65,536-byte binder `b` | ≈ `n` work, `n × 65,568` bytes of data | data: fails at `n = 1,024` (`1,024 × 65,568 = 67,141,632 > 2^26`; `n = 1,023` reserves 67,076,064) |

**Why both a work meter and a data meter.** Work bounds steps, data bounds
memory, and neither implies the other. For byte strings, the per-element work
charge of `concat` (VER04e §2.3) already makes the quadratic accumulator

```text
model push(acc: Bytes, x: U8) -> Bytes = concat(acc, to_bytes([x]))
model copy(b: Bytes) -> Bytes = fold(b, b"", push)
```

quadratic in *work*. Step `k` charges about `k + 8` units (`k` for the
`concat`), so the work meter stops `copy` at about 1,400 elements
(`Σ k ≈ 10^6`), while its data (`k + 33` bytes at step `k`: 32 for `[x]`, 1 for
`to_bytes`, `k` for the `concat`) is still about 1 MB. The first draft of this
section said that the data meter stops `copy` at about 11,585 elements, "long
before the work meter would". That ignored `concat`'s existing work charge (S0
E4). The data meter stops forms whose memory grows faster than their work.
`replicate(b, n)` with a 65,536-byte `b` costs about `n` units but reserves
`n × 65,568` bytes, so it fails at `n = 1,024` having allocated nothing; without
the data meter it would allocate 4 GiB at `n = 65,536` well inside the work
budget. `take`/`drop`/`slice` of a list of large elements, list literals and
tuples of large values, and a fold whose accumulator is a list of large elements
behave the same way.

**Scope of the new meter in generated code.** Generated model code had no meter
in wave 3: without iteration, a model's cost was bounded by its static size. The
meter is emitted only for documents that use a WX-D form (a `seq` prelude part,
gated like `comp`), so wave-3 documents regenerate byte-identically (1.3). It
charges every node with the ground evaluator's charges, including the group
charges of S0 E3 and the short-circuit of S0 E2, and it keeps every counter and
both caps of the table above.

### 2.8 Termination by construction and the acyclic-model rule

Every WX-D form iterates over a sequence value that has already been evaluated
(or over `range(n)`/`replicate(_, n)` whose `n` has been evaluated and capped).
Its per-element functions are models, which are total and non-recursive. So
every evaluation is a finite tree, and the meter bounds its size.

The non-recursion rule of VER04 §2 ("reject cycles among model definitions") is
extended to function references: the model dependency graph gains an edge
`g → f` for every function reference to `f` inside the body of `g`. A cycle is
**W0203**, citing the path. Examples:

```text
model loop(acc: Int, x: U8) -> Int = fold(to_bytes([x]), acc, loop)        -- self edge: W0203
model a(acc: Int, x: U8) -> Int = fold(range(2), acc, b(x))
model b(x: U8, acc: Int, i: Nat) -> Int = a(acc, x)                        -- a → b → a: W0203
```

**Static higher-order nesting** is capped at **4** (`MAX_HO_NESTING`): the
longest chain `f0 → f1 → ... → fn` in which each `f(i+1)` is referenced as a
function argument inside `fi` or a model `fi` calls (the claim, example or
`ensures` row is `f0`) may have at most 4 function-reference edges. A deeper
chain is **W0602** (a checker resource limit, as for WX-A's pattern depth). The
work meter already bounds evaluation, so the cap exists for the other consumers:
a five-deep nest of 16-element folds is `16^5 ≈ 10^6` steps, which always
exhausts the meter, and symbolic unrolling (2.10) multiplies its unroll bounds
through the nest. The chain length is a longest-path computation on the acyclic
graph, linear in its size.

### 2.9 Quantifier interplay

1. **Meaning is unchanged.** WX-D forms are total functions. `forall xs:
   List<U8>. p(fold(xs, ...))` quantifies over all finite lists (VER04 §4); a
   sampling budget never changes it.
2. **No quantifier runs per element.** Rule 6 of 2.3 keeps quantifiers out of
   every per-element function, so a quantifier never appears inside an
   iteration, and the ground evaluator never meets a quantifier it would have to
   enumerate.
3. **Bounded index quantifiers.** `all(range(len(xs)), ok_at(xs))` and
   `forall i: Nat | i < len(xs). ok_at(xs, i)` have the same meaning. Only the
   first is evaluable inside a model and an `ensures` row, and only the first is
   checked on every length (the second's exhaustive domain stops at 64). Authors
   should prefer the WX-D form for index properties.
4. **Domains for binders that feed a WX-D form** (the VER04j §3.6 "flows
   directly" idea, applied to sequences):
   * *Sequence-aware lists (S0 E10).* A `List<T>` binder that flows directly
     into the sequence argument of a WX-D form keeps its exhaustive domain
     (VER04j §4.7: lengths 0..=3) but its property generator draws the length
     from `0..=3` with probability 1/2 and uniformly from `0..=64` otherwise. The
     manifest domain text says `seq-aware`. Only `List<T>` binders change: the
     `Bytes` generator already draws lengths uniformly from `0..=64`. The
     sequence-aware generator, shrinker (2.11) and fuzz decoder are emitted in
     the `seq` prelude part and apply only to sequence-aware binders; every other
     `List<T>` binder keeps the `comp` generator (lengths 0..=3) and the `comp`
     shrinker (the empty list, then each element removed, then element-wise).
     Changing the shrinker for every list would change the generated evidence of
     `wxa` and break 1.3.
   * *Split domain (S0 E9).* A `Nat` binder `k` that flows directly as the count
     of `take`/`drop`, or as the start or length of `slice`, whose sequence
     argument is a binder `m` of the same quantifier declared earlier, gets the
     **dependent** exhaustive domain `0..=len(m) + 1` (one past the end
     exercises the clamp) instead of `0..=64`. The exact case count of the pair
     `(m, k)` is `Σ over m ∈ D(m) of (len(m) + 2)`, and the budget is checked
     against the upper bound `|D(m)| × (maxlen(D(m)) + 2)`:

     | `m`'s type | `D(m)` | exact pairs | budget bound |
     |---|---|---|---|
     | `Bytes` | 347 strings: 1 of length 0, 256 of length 1, 16 of length 2, 64 of length 3, 2 of each length 4..=8 | **1,234** | 347 × 10 = **3,470** |
     | `Bytes<N>` | `s` strings, every one of length `N`, where `s` is the size of `D(Bytes<N>)` (VER04h §3) | `s × (N + 2)` | the same |
     | `List<T>` | lengths 0..=3: 1 empty, `d` of length 1, `d²` of length 2, `e³` of length 3, with `d = min(t, 16)`, `e = min(t, 4)` and `t` the size of `D(T)` (VER04j §4.7) | `2 + 3d + 4d² + 5e³` (1,394 for `List<U8>`) | `(1 + d + d² + e³) × 5` (1,685 for `List<U8>`) |

     For `D(Bytes)` the undependent product would be `347 × 65 = 22,555`. The
     bound enters the exhaustive budget product of the claim in place of `k`'s own
     domain size (a factor of `maxlen(D(m)) + 2`), with checked arithmetic and
     before enumeration; over budget is W0706, as for any domain.
     **Enumeration order** is the existing exhaustive order: binders in
     declaration order, the first outermost, with `k`'s range recomputed from the
     current `m` and taken in ascending order. So `m` is outer and `k` ascends:
     for `forall m: Bytes, k: Nat` the first tuple is `(b"", 0)` and the last is
     `(a5 × 8, 9)`. The property generator draws `k` as `0` (1/4), `len(m)` (1/4),
     uniform in `0..=len(m)` (1/4), `len(m) + 1` (1/8) and uniform in `0..=64`
     (1/8). The **manifest** domain text of `k` is `k: Nat split(m)(<pairs>)`,
     where `<pairs>` is the exact pair count (`k: Nat split(m)(1234)` for
     `m: Bytes`), not the 3,470 bound and not a factor of a product; the suite's
     case count records what actually ran. Two split binders over the same `m`
     (`i` and `j` in 3.7) each get their own entry, and their joint enumeration
     is `Σ (len(m) + 2)^2` pairs (4,824 for `D(Bytes)`, budget bound
     347 × 10 × 10 = 34,700). This is a domain *choice*, honestly labelled; it
     is not a claim that larger `k` were checked.
     **Conflicts are refused, never resolved by guessing.** When `k` flows
     directly into split positions over two different sequence binders, or into
     a split position and a shift count (the `NatCount` domain of VER04j §3.6),
     there is no single honest domain. The intersection would drop the clamp
     boundary of the longer sequence, and any other choice is a guess, so
     generation fails with **W0706** naming `k` and both uses. The author splits
     `k` into two binders. A `k` whose split position takes a sequence that is not
     an earlier binder of the same quantifier (an expression, or a binder declared
     after `k`) gets no split domain and keeps its type's ordinary domain; that is
     the existing rule, not a conflict.
5. **Symbolic discharge** of a claim with a fold over a binder of symbolic length
   is `bounded(K)` by unrolling and never `proved` (2.10).

### 2.10 Symbolic discharge (VER10, VER19)

Consumers that turn a model into a circuit are VER19 §5 (the "reference
specification circuit `R_spec`" of an implementation miter) and spec-level
consistency/reachability queries over models (VER04 §8). WX-D adds one rule for
both: **a WX-D form is encoded by unrolling it at concrete lengths.**

* **Shapes.** For an obligation, every sequence binder with a dynamic length
  gets each length `0..=K`, and every `Nat`/`Int` binder that reaches a count or
  an index gets each value `0..=K + 1`. A *shape* is one assignment of those
  lengths and values. Within a shape, `len`, `range`, counts and indices are
  concrete, and the elements are fresh inputs (`BitVec(8)` per byte,
  `BitVec(w)` per fixed-width element, one Bool per `Bool`).
* **Unrolling.** Within a shape a `fold` is `L` copies of the step circuit, a
  `map` is `L` independent circuits, `all`/`any`/`count` are a conjunction,
  disjunction or adder tree, `take`/`drop`/`slice` are rewiring, and
  `range`/`replicate` produce constants. Because `range(8)` yields concrete
  `Nat`s, `bit(b, i)` and `shl(a, i)` inside `gmul_step` become constant-amount
  operations (VER10 §4 "by a constant: rewiring"), not barrel shifters.
* **The encodable fragment** is what VER10 §5's BV tactic accepts: Bool,
  fixed-width integers up to 512 bits, the §4 operations, `ite`. Within a shape,
  every value must also have a *fixed layout*: tuples and records of encodable
  values, and sequences whose length is concrete. Two things fall outside and
  label the obligation `unknown`: a `Nat` or `Int` value that is not concrete
  within the shape (say `to_int(x)` of a symbolic `U32` used as a count), since
  BV mixed with LIA answers `Unknown` (VER10 §5); and a sequence whose length
  depends on symbolic data (an accumulator that `concat`s on only some paths, as
  `leb_encode`'s does). WX-D never invents a symbolic iteration count or a
  symbolic length.
* **Labels** (the VER04h §8 vocabulary): if every shape is refuted with a
  certificate accepted by `drat-check`, the label is `bounded(K)` with `K` the
  largest length enumerated. It is **never `proved`** when any sequence length
  or count varied with the input, because the domain of lengths is infinite and
  only `0..=K` was encoded. The one exception: if every sequence in the
  obligation has a static length (`range(<literal>)`, a `Bytes<N>` binder with a
  literal `N`, a list literal) and no count binder exists, the single shape *is*
  the whole domain and the label may be `proved`. `gmul` qualifies (`range(8)`,
  two `U8` binders, fixed-width accumulator). `leb_encode` does not: its
  iteration count is static but its `Bytes` accumulator's length depends on `v`,
  so it is `unknown` (its evidence stays the exhaustive, property and fuzz
  suites). A claim over `m: Bytes` is at best `bounded(K)`.
* **Caps** (all checked before encoding; exceeding any of them labels the
  obligation `unknown`, records the cap that was hit, and is W0602 for
  checked-spec acceptance; a smaller `K` is never substituted silently):

| Cap | Value |
|---|---|
| unroll length `K` | default 16 (`UNROLL_K_DEFAULT`), at most 64 (`UNROLL_K_MAX`), per obligation |
| shapes per obligation | 4,096 (`MAX_SHAPES`) |
| unrolled step instances in one shape (product through nested forms) | 4,096 (`MAX_UNROLL_PRODUCT`) |
| VER10 circuit limits | 250,000 AIG nodes, 2,000,000 gate constructions, 1,000,000 conflicts, 2^30 propagations, widths <= 512 |

  A partially completed shape set is `unknown`, not `bounded(K')` for the largest
  `K'` that happened to finish: the label states the `K` that was asked for and
  completed for every shape, or nothing.
* **Manifest.** Proof-manifest entries gain `unroll: {k, shapes, max_aig_nodes,
  max_gate_constructions}` (optional fields; existing entries unchanged).

### 2.11 Evidence-generation impact (VER04h)

* **Translation.** List literals, the twelve built-ins and function references
  are translated in model, claim, table `holds` and `ensures` bodies (lifting the
  W0701 of VER04j §14 item 24 for list literals in v1 documents). WX-D1
  translates list literals and the seven non-higher-order forms; WX-D2 adds the
  other five and function references. The generated model code iterates over
  `V::List`/`V::Bytes` with the meter of 2.7.
* **List-literal cap (S0 E8).** A list literal has at most 4,096 elements
  (`MAX_LIST_LITERAL`). The **checker** enforces the cap, in v1 documents only,
  for every list literal anywhere in the document (models, claims, examples,
  tables, refinements, `ensures` rows): more is **W0602**, reported at the
  literal's span before its elements are type checked. The first draft placed the
  cap at generation here and in the checker in 6.2; the checker is the single
  place. A v0 document is unchanged: the v0 checker has no cap. The ground
  evaluator keeps its own refusal of more than 65,536 elements (W0703) as defence
  in depth, for v0 documents and for any path that bypasses the checker.
* **Operation discovery (S0 E13).** In a v1 document, the search that finds the
  operation a claim, example or table calls descends into list-literal elements,
  treating each element like a call argument. So `to_bytes([f(a)]) == b` and
  `[f(a)] == [b]` both name `f`. Without this, they are refused with "claim calls
  no operation". The search keeps its order: the shallow search first (the v0
  behaviour), then arguments and list elements left to right, first match wins. A
  v0 document keeps today's search, which does not look inside list literals.
* **Domains, generators, shrinkers.** As 2.9 (sequence-aware lists, the split
  domain). Shrinkers: a sequence-aware list shrinks toward shorter lists (drop
  the last element, then halve) before element-wise shrinking. Every other list
  keeps the `comp` shrinker (2.9, S0 E10). A split-domain `k` shrinks within
  `0..=len(m)` after `m` has shrunk.
* **Fuzz decoding** of a sequence-aware list binder reads a length byte, taken
  modulo 65, then each element with its type's decoder. Other list binders keep
  the `comp` decoding.
* **Manifests.** Domain text gains `seq-aware` and `split(<binder>)(<pairs>)`
  (2.9); nothing else changes.

### 2.12 Sabotage implications

WX-D adds model vocabulary; faults attack subjects, not models. Two consequences:

1. **New result faults (S0 E11)** become meaningful because `List` results can
   now be checked element-wise. They are rewrites of the composite adapter's
   result, like WX-E's `drop-last-result-byte` (VER04j §14 item 46), and they
   apply to the operation they are bound to only.
   * **Where a fault applies is decided by type, at generation.** Take the
     operation's declared result type, remove at most one `Result` layer (keeping
     the `Ok` payload) and then at most one `Option` layer (keeping the `Some`
     payload). This is the *core* type the existing composite faults are checked
     against. A list fault applies iff the core type is `List<T>`. Any other
     binding of a list fault is **W0701**. That includes a `List` nested in a
     tuple, a record, a variant or another list's elements: out of scope for
     WX-D1, and W0701 rather than a guess at which list was meant.
   * **What each fault does is decided by value, at run time.**
     `drop-last-result-element` removes the last element of the core list.
     `reverse-result-list` reverses the core list. On an `Err` or `None` result
     either fault does nothing. Each fault also does nothing where it would not
     change the value: an empty list has no last element to lose, so
     `drop-last-result-element` leaves `[]` unchanged (unlike
     `drop-last-result-byte`, it does not invent an element, which would need a
     value of `T` the tool chose), and reversing a list of length 0 or 1 (or a
     palindrome) is the identity. "Length at least 2" in the first draft was this
     run-time fact, not a generation-time condition.
   * As for the driver faults (3.6), catching a list fault relies on the domain
     containing inputs where it changes the value. The runtime control, under
     which a fault that no suite catches fails its control, is the fail-closed
     backstop.
   * The fault variants and their prelude functions are emitted only for an
     operation whose binding lists them, as for the WX-E faults, so wave-3 output
     is unchanged (1.3).
2. **Evaluator mutants** are the WX-D analogue of VER04j §10.3's killed mutants:
   a fold that skips the last element, a fold that visits in reverse, `all`
   without short-circuit (detected by the charge comparison, not the value),
   `take` that does not clamp negative counts, `range(n)` that includes `n`, a
   recursive fold (detected by the 1,000-element case hitting the depth limit),
   and a meter that charges after allocating (detected by an allocation probe in
   the Rust evaluator). The S0 errata add four for WX-D1: an eager `and`/`or`/`=>`
   in a v1 evaluation, a group that charges nothing, `take`/`drop`/`slice`/
   `replicate` charging `L × 32` instead of the deep copy, and an admission
   predicate charged outside the instance meter. The first three are detected by
   the charge comparison; the last by an instance whose admission predicate
   exhausts the meter and must fail. Each must make the differential suite
   fail. The `fold` and `all` mutants belong to WX-D2.

### 2.13 Cross-language portability (VER04i)

Requirements on VER04i, which owns the corpus encoding:

1. **Model IR tags.** `["list", e...]`, `["fold", xs, init, F]`, `["map", xs, F]`,
   `["count", xs, F]`, `["all", xs, F]`, `["any", xs, F]`, `["take", xs, n]`,
   `["drop", xs, n]`, `["slice", xs, s, n]`, `["replicate", x, n]`,
   `["range", n]`, `["to_list", b]`, `["to_bytes", xs]`, where
   `F = ["fnref", "<model>", [bound...]]`. A lane evaluator rejects an unknown tag
   with a hard error (VER04i §2.5).
2. **The meter is part of the IR semantics.** Each lane evaluator implements the
   charges and caps of 2.7, including the depth rule, the group charge, `dsize`
   data charges and the short-circuit `and`/`or`/`=>` of the S0 errata (a lane
   IR that lowers `and` to an ordinary two-argument call would charge both
   operands and is non-conforming). Because the corpus generator evaluates every
   materialised and enumerated case in Rust and **refuses to emit a corpus in
   which any case exhausted** (W0703 at corpus generation), a lane never needs to
   agree on an exhaustion boundary to pass; the meter is its fail-closed
   safety net, and a lane whose meter is stricter than Rust's fails a case and is
   caught.
3. **Lane hazards.**

| Hazard | Lanes | Required handling |
|---|---|---|
| negative counts mean "from the end" natively | Python, Ruby, Perl (`[-1]`, `[:-1]`), JavaScript/TypeScript `slice` | clamp to zero *before* calling any native slicing |
| recursion depth | Haskell, OCaml, Elixir, Scheme-like lanes where `foldr` or naive recursion is idiomatic | a strict left fold (`foldl'`, `List.fold_left`, `Enum.reduce`); never `foldr` |
| `Bytes` is not a list type | every lane | keep the Warrant distinction: `map` over bytes with a `U8 -> U8` function yields the lane's byte type |
| huge `n` | every lane | compare the exact `n` with 65,536 before converting it to a host integer (a 2^70 `n` must not wrap to a small count in a 64-bit lane) |
| `range` conventions (`1..n` inclusive in Elixir/Ruby/Kotlin) | those lanes | build `[0, n)` explicitly |
| short-circuit of `all`/`any` | lanes whose library `all` is eager | iterate manually; the charge depends on it |

The canonical stream (VER04i §2.6) already encodes `List` as `30` and `Bytes` as
`20`; no new value encoding is needed.

### 2.14 Compatibility

The twelve names are v1 built-ins (1.2), all reserved from WX-D1 on. A v0
document is unaffected. A v1 document that does not use them generates
byte-identical evidence (1.3). The S0 errata add three v1-only checker and
evaluator changes that a wave-3 v1 document can observe, but never as a
different value: the list-literal cap (W0602) and the data counter (W0703) can
add a refusal, and short-circuit `and`/`or`/`=>` can remove one and lowers
charges (1.2).

### 2.15 Test plan (WX-D)

Each bullet names the slice that owns it (section 8.1); "WX-D2" and "WX-D3"
items wait for those slices.

* **Syntax** (Rust tests over the real parser, as VER04j §14 item 11 did; no
  corpus file): every form parses as a call; a section parses; the grammar
  files are unchanged.
* **Checker negatives, WX-D1** (one case each, pinning ID and span): a list
  literal of 4,096 elements accepted and one of 4,097 refused with W0602 at the
  literal's span in a v1 document, and the same 4,097-element literal accepted by
  the v0 checker (S0 E8); a call of each of `fold`, `map`, `count`, `all` and
  `any` in a v1 document is W0202 with "not yet implemented in this version", and
  a v1 model named `fold` is W0201 (S0 E7); `take(xs, true)` (W0301); comparing
  `to_list(b)` with `b` (W0301); a v1 model named `count` (W0201); `take(...)` in
  a v0 document (W0202) and a v0 model named `take` (accepted); a literal
  argument typed from its position (`replicate([], 3)` where a
  `List<List<U8>>` is expected). A mechanical scan confirms that none of the four
  wave-3 v1 documents declares one of the twelve names or holds a list literal.
* **Checker negatives, WX-D2:** an operation as the function argument (W0712); a
  built-in (`fold(xs, 0, len)`, W0712); a binder (W0712); an `if` in the function
  position (W0712); a section with too many and too few bound arguments (W0712); a
  step whose accumulator type is `Int` where the result is `Nat` (W0712, exact
  tail types); a model with static parameters (W0712); a predicate that reaches a
  quantifier through a helper model (W0712); self and mutual cycles through
  function references (W0203); nesting depth 5 (W0602); `fold` over a `Bool`
  (W0301); `fold(...)` in a v0 document (W0202) and a v0 model named `fold`
  (accepted). A mutant checker that drops rule 5 or rule 6 of 2.3 must fail a
  case (VER04 §9).
* **Evaluator differential, WX-D1 (S0 E12).** Two implementations, the ground
  evaluator and the generated model code, must agree on the **triple**
  `(value, work units, data bytes)`, and on which counter or cap exhausts. The
  native Rust iterator equivalents (`iter().take`, `skip`, `repeat`) are an
  oracle for the values. A small set of triples computed by hand from this text
  is pinned as the independent check until the Python reference evaluator lands
  with WX-D2: `((x))` costs 3 units, and `replicate(b, 1023)` and
  `replicate(b, 1024)` on a 65,536-byte binder sit on either side of the data
  budget (2.7). The cases:
  * every WX-D1 form over every list of length 0..=4 over a 3-element alphabet;
  * sequences of length 1,000 and 65,536, and 65,537 refused at step 1 of 2.7
    with nothing allocated (an allocation probe in the Rust evaluator);
  * counts `-2, -1, 0, len - 1, len, len + 1, 2^70`;
  * the quadratic accumulator `copy` of 2.7 on both sides of its *work* boundary.
    The boundary (about 1,400 elements) is found by bisection on the ground
    evaluator and pinned in the test; it changes only if a charge changes.
  * the data charge of every constructing node (`concat`, a list literal, a
    tuple, `be_bytes`, `at`, `Some`, each WX-D1 form, including the deep copy of
    `take` and `replicate` over a `List<Bytes>`), and no data charge for the
    pass-through nodes (S0 E4, E5);
  * depth 64 accepted and depth 65 refused, through nested groups and through
    model calls (S0 E1, E3);
  * the two caps: a `Bytes` and a `List` of 65,536 elements accepted and of
    65,537 refused, and a tuple of 65,536 leaves accepted and of 65,537 refused
    (S0 E6);
  * short-circuit in v1: `false and q`, `true or q` and `false => q`, where `q`
    would exhaust or fail, all succeed and do not charge `q`; the same
    expressions in a v0 document keep today's eager result and diagnostics,
    pinned (S0 E2);
  * admission: an exhaustive and a property instance whose refinement exhausts
    the meter fails with `model-budget-exhausted` and is not counted as
    discarded (S0 E14).

  The suites run with and without `CI=true` (the generated prelude reads `CI`).
  The mutants of 2.12 that belong to WX-D1 must be killed.
* **Evaluator differential, WX-D2.** The Python reference evaluator, written
  from this text, joins as the third implementation and covers the WX-D1 forms
  too. The cases add the higher-order forms over the same lists, sections with 0
  and 8 bound arguments, nesting depth 4 and 1,000-element folds. Native Rust
  iterator equivalents (`iter().fold`, `all`, `any`) are an oracle for the
  values. The remaining mutants of 2.12 must be killed.
* **Generator, WX-D1.** List literals and the seven forms translate in every body
  kind (S2a); the pinned refusals of list literals in v1 bodies flip to
  translations, and a v0 document still refuses them (W0701). Operation discovery:
  `to_bytes([f(a)]) == b` and `[f(a)] == [b]` name `f` in a v1 document, and the
  v0 search is unchanged (S0 E13). Gating: wave-3 fixtures regenerate
  byte-identically. An instance whose claim body exhausts fails with
  `model-budget-exhausted` and never passes (S2b). The two list result faults
  (S4) yield `not ok` under `WARRANT_EVIDENCE_FAULT` for a `List<T>` and a
  `Result<List<T>, E>` result; `drop-last-result-element` leaves `[]`, and
  `reverse-result-list` leaves a one-element list, unchanged; a list fault bound
  to `(List<U8>, Nat)` is W0701 (S0 E11).
* **Generator, split domain and sequence-aware lists (S3).** The split domain of
  `forall m: Bytes, k: Nat` has 1,234 pairs, its budget bound is 3,470, and the
  manifest text is `k: Nat split(m)(1234)`. Its first tuple is `(b"", 0)` and its
  last is `(a5 × 8, 9)`, both pinned. `List<U8>` has 1,394 pairs, and
  `Bytes<4>` has 16 × 6 = 96. Two split binders over one `m` enumerate 4,824
  joint pairs before filtering. The dependent budget check rejects an
  over-budget product before materialising it (W0706). A `k` used in split
  positions over two sequence binders, or in a split position and a shift
  count, is W0706 (S0 E9). Sequence-aware property lengths reach 64 for a
  `List<T>` binder. In the same document, a list binder that feeds no WX-D form
  keeps the `comp` generator and shrinker (S0 E10).
* **Symbolic (WX-D3).** The fold `gmul` against the gf256 export's lowered WIR
  at `proved` (static lengths, fixed layout); `leb_encode` labelled `unknown`
  (data-dependent accumulator length); a fixed-width fold over `m: Bytes` at
  `bounded(16)`, never `proved`; a symbolic count (`take(m, to_int(x))`)
  labelled `unknown`; each cap exceeded once, each giving `unknown` with the cap
  recorded.
* **Conversion pilots.** WX-D1: the `wxd` fixture document and its subject (S2a),
  regenerated and checked in like `wxa`/`wxh`/`wxv`. WX-D2: `wasm-leb128`, with
  the `encode_matches_model` claim of 2.1 over the full `U64` edge domain (377
  values), plus `decode_unsigned` round trips (WX-A), with the
  `off-by-one-consumed` control of VER04j §4.8 failing. Second WX-D2 pilot: the
  gf256 document's `gmul` rewritten as the fold of 2.1, every previous verdict
  kept.

## 3. WX-F — binding-level sequence drivers

### 3.1 Motivation

Streaming digests, entropy coders and incremental decoders are objects:

| Package | Constructor | Step | Finish | Shape |
|---|---|---|---|---|
| `coding_adventures_sha1` (also sha256, sha512, md5) | `Digest::new()` | `update(&mut self, &[u8])` | `sum1(&self) -> [u8; 20]` | absorb, observe |
| `coding_adventures_sha256` | `Sha256Hasher::new()` | `try_update(&mut self, &[u8]) -> Result<(), Sha256Error>` | `digest(&self) -> [u8; 32]` | fallible absorb |
| `range-coder` | `BoolEncoder::new()` | `write_bit(&mut self, bool, u8)` | `finish(self) -> Vec<u8>` | absorb, consuming finish |
| `range-coder` | `BoolDecoder::new(&[u8])` | `read_bit(&mut self, u8) -> bool` | none | emit one value per step |
| `rans` | `RansEncoder::new(&AnsTable)` | `put(&mut self, u8)` | `finish(self) -> Vec<u8>` | needs a context object (open question 7) |

The bug classes are the ones a one-shot test never sees: an `update` that
mishandles a chunk crossing the 64-byte block boundary, a final partial block
that is never absorbed, state that is re-created per call, a decoder that loses
its last symbol, and a finish that is destructive when the binding assumed it
was not (Java's `MessageDigest.digest()` resets the object; section 3.10).

The document stays pure. To Warrant, a driven operation is an ordinary `op` with
a sequence parameter; only the binding knows it is executed as a protocol.

```text
warrant sha1_excerpt v1
  -- excerpt: labels, input and bound contracts omitted
  op digest_chunks(chunks: List<Bytes>) -> Bytes<20>
  op sum1(data: Bytes) -> Bytes<20>

  example empty_stream_is_the_empty_digest:
    digest_chunks([]) == b"da39a3ee5e6b4b0d3255bfef95601890afd80709"
  claim one_shot_agrees:
    forall m: Bytes. digest_chunks([m]) == sum1(m)
  claim split_independent:
    forall a: Bytes, b: Bytes. digest_chunks([a, b]) == digest_chunks([concat(a, b)])
  claim any_two_way_split:
    forall m: Bytes, k: Nat. digest_chunks([take(m, k), drop(m, k)]) == sum1(m)
```

### 3.2 Language and schema additions (grammar: none)

No Warrant syntax. Two things change outside the grammar:

1. **List values in evidence bodies** (2.11): `digest_chunks([a, b])` needs list
   literals translated in claim bodies.
2. **The binding schema** gains a driver record per driven operation. The
   normative data model is given in the closed JSON form of VER04d §5 (and of the
   portable per-lane `warrant.binding.json` of VER04i §5); the Rust
   `evidence.bindings` line form below is its exact transcription for the file
   the generator reads today (`binding.rs`).

**JSON form** (one operation; field order is not significant; unknown fields are
errors):

```json
"digest_chunks": {
  "driver": {
    "state":  "coding_adventures_sha1::Digest",
    "new":    { "fn": "new", "args": [] },
    "drive":  "chunks",
    "step":   { "method": "update", "receiver": "mut",
                "args": [ { "from": "elem", "host": "bytes" } ],
                "returns": "unit" },
    "finish": { "method": "sum1", "receiver": "ref", "host": "bytes" },
    "result": "finish",
    "lengths": [55, 56, 63, 64, 65, 119, 120, 127, 128, 129]
  },
  "faults": ["drop-last-chunk", "reorder-chunks", "duplicate-last-chunk",
             "early-finish", "reset-per-step"]
}
```

| Field | Values | Meaning |
|---|---|---|
| `state` | Rust path relative to the crate root, at most 8 identifier segments | the host type whose associated function is `new` |
| `ctx` (optional) | `{ "fn": <path>, "args": [<arg>...], "returns": "value" \| "result" }` | a context value built once from parameters and passed by shared reference to `new` (open question 7) |
| `new` | `{ "fn": <identifier>, "args": [<arg>...] }` | called as `<state>::<fn>(args)`; an argument may be `{"from": "ctx"}` |
| `drive` | a parameter name | the parameter whose elements are fed one per step |
| `step` | `method`, `receiver` (`mut` = `&mut self`, `own` = `self -> Self`), `args`, `returns` (`unit`, `result` for a host `Result<(), E>`, a host type for an emitted value, or `result<ht>`) | called once per element |
| `finish` (optional) | `method`, `receiver` (`ref` = `&self`, `own` = `self`), `host` (the host type of its return) | called once after the last step |
| `result` | `finish`, `collect` (a `List` of the step return values), `both` (the tuple `(collected, finished)`) | what the operation returns |
| `errors` (optional) | `{ "path": <path>, "map": { "<Variant>": "<CASE>" } }` | the host error enum of a fallible `ctx`/`step`, as WX-A's `errors=` (VER04j §14 item 22) |
| `lengths` (optional) | strictly increasing integers in `0..=4096`, at most 32 | block-boundary hints for `Bytes` elements (3.7) |

An argument `<arg>` is `{ "from": <source>, "host": <ht> }` with
`<source>` one of `"elem"`, `"elem.<i>"` (a component of a tuple or record
element, nested at most 4 deep, `"elem.1.0"`), `"param.<name>"` (a non-driven
parameter, constant across steps) or `"ctx"`. `<ht>` is a WX-A host type
(`u8`..`u64`, `i8`..`i64`, `usize`, `bool`, `bytes`, `(ht, ...)`, `opt<ht>`,
`list<ht>`, a declared struct/enum name). `bytes` is passed as `&[u8]` and, for a
return, accepts any host type that is `AsRef<[u8]>` (`Vec<u8>`, `[u8; N]`).

**Line form** (Rust `evidence.bindings`): the `op` line carries the bare option
`driver` instead of `export=`, and one `driver` line carries the record:

```text
op digest_chunks driver faults=drop-last-chunk,reorder-chunks,duplicate-last-chunk,early-finish,reset-per-step
driver digest_chunks state=coding_adventures_sha1::Digest new=new() drive=chunks step=update(mut,elem:bytes) finish=sum1(ref)->bytes result=finish lengths=55,56,63,64,65,119,120,127,128,129

op encode_bits driver faults=drop-last-chunk,reorder-chunks,duplicate-last-chunk,early-finish,reset-per-step
driver encode_bits state=range_coder::BoolEncoder new=new() drive=bits step=write_bit(mut,elem.0:bool,elem.1:u8) finish=finish(own)->bytes result=finish

op digest_try driver faults=drop-last-chunk,reorder-chunks,duplicate-last-chunk,early-finish,reset-per-step
driver digest_try state=coding_adventures_sha256::Sha256Hasher new=new() drive=chunks step=try_update(mut,elem:bytes)->result finish=digest(ref)->bytes result=finish errors=coding_adventures_sha256::Sha256Error/LengthOverflow=TOO_LONG
```

`step=NAME(receiver,arg,...)[->returns]`, `finish=NAME(receiver)->ht`,
`new=NAME(arg,...)`, `ctx=PATH(arg,...)[->result]`, where an `arg` is
`source:ht` or `ctx`. Values are split with the existing `split_top` (commas
inside brackets do not split). (The exact variant names in the `errors=` example
are illustrative; the binding must name the crate's real variants.)

### 3.3 Typing rules (binding against the document)

The binding is validated against the checked document before any code is
generated. Schema errors are **W0501** (3.4, rules 1–3, 9, 10); inconsistency with
the operation is **W0711**:

1. The driven parameter `p` has type `List<T>`, `Bytes`/`Bytes<N>` (elements are
   `U8`) or `Nat` (a count: `p` steps, each with no element; `elem` sources are
   then W0711).
2. Every `elem.<path>` resolves through tuple components and record fields of
   `T` (by index), at most 4 deep, and every argument's host type can carry the
   Warrant type at that path under the WX-A host mapping (a `Nat` component
   bound to a host `usize`/`u32` declares its width; values the host cannot
   hold are *unreachable by type*, VER04j §4.6).
3. Every runtime parameter other than `p` is consumed by `new`, `ctx` or a step
   argument. An unconsumed parameter would make the operation's meaning
   independent of an argument that the document says it depends on.
4. Result mode: `finish` requires a `finish` record whose host type carries the
   operation's result type `R`; `collect` requires a step that returns a host
   type carrying `U` with `R = List<U>`; `both` requires both, with
   `R = (List<U>, F)`. If `ctx` or `step` is fallible (`returns: result...`),
   `R` must be `Result<R', E>`, the mode is checked against `R'`, and `errors`
   must map every listed host variant to a case of `E` (at most 64 entries).
5. The operation has no static parameters, and the document is v1.
6. `fresh` is allowed: a finish that returns an owned `Vec<u8>` or array cannot
   alias an input, so freshness holds by construction, as for WX-H's composite
   adapter (VER04j §14 item 35).

### 3.4 Validation rules (schema)

1. An `op` line has exactly one of `export=` and the bare `driver`; a `driver`
   line names an operation whose `op` line has `driver`, exactly once (W0501).
2. `state`, `ctx.fn` and `errors.path` are Rust paths (`is_rust_path`): at most 8
   segments, each a Rust identifier of at most 128 bytes. `new.fn`, `step.method`
   and `finish.method` are Rust identifiers (W0501). These strings are spliced
   into generated source, so anything else is refused before generation, never
   escaped.
3. `receiver`, `returns`, `result` and host types come from the closed
   vocabularies of 3.2; anything else (including a repeated key) is W0501.
4. `faults=` lists **every** driver fault of 3.6 that applies to the operation's
   shape (W0502 if one is missing: bindings cannot opt out of negative controls,
   VER04 §6) and no fault that does not apply (W0701).
5. A driven operation's contracts are honest about proofs: a `labels` row with
   `secret` or a `bound` row on a driven operation produces VER04h §7
   obligations that the pilot prover cannot lower (there is no single export),
   so the `op` line must carry `proofs_not_live=driver` (VER04h §9 item 18), or
   generation fails with W0711.
6. Export coverage (VER04 §6): every method a driver names counts as bound for
   the independent export analyzer; a method named by no op and no driver is
   still unbound.
7. `lengths` is strictly increasing, at most 32 values, each `<= 4096`, and only
   on a driver whose elements are `Bytes` or whose driven parameter is `Bytes`.

### 3.5 Evaluation semantics: how the generator drives it

The adapter replaces "call one export" with a loop. Generated shape (Rust):

```rust
fn adapt_digest_chunks(chunks: &V) -> Result<V, AdapterError> {
    let elems = seq_elems(chunks)?;                    // List or Bytes elements, index order
    if elems.len() > MAX_DRIVER_STEPS {                // 65,536: checked before `new`
        return Err(AdapterError::Note("driver-steps-exceeded"));
    }
    let plan = driver_fault_plan(elems.len());        // identity unless a driver fault is active
    let mut st = coding_adventures_sha1::Digest::new();
    for i in plan.order() {                            // each element converted fresh per step
        if plan.reset_before(i) { st = coding_adventures_sha1::Digest::new(); }
        st.update(&bytes_of(&elems[i])?);
    }
    Ok(V::Bytes(st.sum1().as_ref().to_vec()))
}
```

* Each element is converted to a fresh host value per step, so the subject never
  holds a reference into a value the oracle still uses.
* A panic in `ctx`, `new`, a step or `finish` is a subject panic (`panic`), unless
  the operation declares panic normalisation (`error=`), exactly as for an
  export. A fallible step's `Err` stops the drive: no further steps, no finish,
  and the operation returns the mapped `Err(E.CASE)`.
* If the operation has a WX-H `heap` or `output` row, the whole drive (`ctx`
  through `finish`) runs in **one** allocation window of the capped observer
  (VER04j §6.3), so per-step allocation is charged to the operation.
* A drive that exceeds `MAX_DRIVER_STEPS` fails its instance with the note
  `driver-steps-exceeded` (a defect of the document's inputs, never truncated).
  Generated domains never come near it; it guards examples and table rows.

**Obligations.** Besides the claims, the generator registers these L0 tests for
every driven operation (ids `warrant/<module>/driver/<op>/<obligation>`):

| Obligation | What runs | Fails when |
|---|---|---|
| `shape` | a compile-time item pinning every signature: `const _: fn(&mut Digest, &[u8]) = Digest::update;` (and `new`, `finish`, `ctx`) | a method is missing or its signature changed; the compile error names the generated item. The export analyzer's listing of each method as public is recorded when that analyzer is live, `not-live` otherwise |
| `empty` | the drive over the empty sequence | an unexpected panic |
| `empty-element` | for a `List<Bytes>`/`List<List<_>>` driven parameter: every list of length 1..=3 over `{b"", b"00"}` with the empty element at every position | an unexpected panic |
| `deterministic` | 16 fixed-seed sequences, each driven twice from fresh state | the two results differ (hidden global state) |
| `finish-observational` | only for `finish.receiver = ref`: after the steps, `finish` is called twice | the two observations differ: the finish mutated the state although the binding declared `&self` (the **double-finish** check) |
| `steps-cap` | a sequence of `MAX_DRIVER_STEPS + 1` elements | the adapter called `new` or any step before refusing |

Every obligation has its own control, so none can pass vacuously:
`finish-observational` is re-run with one extra step (the first non-empty element
of the element domain) between the two finishes and must then report a
difference for at least one of the 16 sequences; `deterministic` is re-run with
the second drive given a different sequence and must report a difference;
`steps-cap` is re-run with the cap raised by one and must observe the call.

**Why double-finish is an obligation, not a fault.** A fault must fail some
suite against a *correct* subject. Calling a correct `&self` finish twice changes
nothing, so a "double-finish" fault would be uncatchable and its control would
fail spuriously; for an `own` finish it cannot even be expressed (the state is
moved). What double-finish really guards is the binding's claim that `finish`
observes without mutating, which is a property of the subject-plus-binding pair,
so it is checked as an obligation with its own control. Open question 5.

**Vacuity check (W0601).** A driven operation that no claim or example ever calls
with a sequence of at least two non-empty elements cannot catch
`reorder-chunks`, `early-finish` or `reset-per-step`; generation fails with
W0601 ("driver not exercised"), before the runtime controls would.

### 3.6 Sabotage implications

Driver faults are rewrites of the *drive plan*, injected in the adapter (the crate
is never modified), selected per operation like every scoped fault:

| Fault | Plan rewrite (`n` elements) | Applies when | Bug modelled |
|---|---|---|---|
| `drop-last-chunk` | drive elements `0..n-1` | always | final partial block never absorbed |
| `reorder-chunks` | drive in reverse order | the driven sequence can have 2 or more elements | out-of-order processing |
| `duplicate-last-chunk` | drive the last element twice | always | retried or double-fed final chunk |
| `early-finish` | drive only element 0 | 2 or more elements possible | stops after the first chunk |
| `reset-per-step` | re-create the state (from `new`; `ctx` is kept) before every step | 2 or more elements possible, and `result` is `finish` or `both` | "new hasher inside the loop" |
| `drop-last-output` | remove the last collected value | `result` is `collect` or `both` | decoder loses its final symbol |

On an input where the rewrite is the identity (`drop-last-chunk` and
`duplicate-last-chunk` on an empty sequence, `reorder-chunks` on one element) the
fault does nothing, so catching it relies on the domain containing inputs where
it does something, which the next paragraph shows.

**Catchability with the canonical claims.** `D(List<Bytes>)` (VER04j §4.7) holds
every list of length 0..=2 over the first 16 members of `D(Bytes)` (`b""`, `b"00"`
… `b"0e"`) and length-3 lists over the first four. With `one_shot_agrees` and
`split_independent` of 3.1: `drop-last-chunk` turns `[b"00"]` into `[]`, whose
digest differs from `sum1(b"00")`; `duplicate-last-chunk` turns `[b"00"]` into
the digest of `b"0000"`; `reorder-chunks` and `early-finish` change
`[b"00", b"01"]` relative to `[b"0001"]`; `reset-per-step` makes
`[b"00", b"01"]` hash as `b"01"`. Each therefore fails the exhaustive suite by
construction (assuming SHA-1 has no collision among these short inputs, which is
checked by the run, not assumed by the evidence). For other subjects the
runtime control is the proof: a fault no suite catches fails its control.

### 3.7 Composition with WX-D: split-point sweeps

`take`, `drop` and `slice` turn "every way of splitting the input" into an
ordinary quantified claim, and the split domain (2.9) enumerates every split
point of every member of `D(Bytes)` exactly:

```text
  claim any_two_way_split:
    forall m: Bytes, k: Nat. digest_chunks([take(m, k), drop(m, k)]) == sum1(m)
  claim any_three_way_split:
    forall m: Bytes, i: Nat, j: Nat | i <= j. (
      digest_chunks([take(m, i), slice(m, i, j - i), drop(m, j)]) == sum1(m))

  -- fixed-size chunking: every chunk size 1..=64
  model chunk_at(m: Bytes, c: Int, i: Nat) -> Bytes = slice(m, i * c, c)
  model chunks_of(m: Bytes, c: Int) -> List<Bytes> = (
    map(range(unwrap_or(div(len(m) + c - 1, c), 0)), chunk_at(m, c)))
  claim any_chunk_size:
    forall m: Bytes, c: Nat | c >= 1. digest_chunks(chunks_of(m, c)) == sum1(m)
```

* `any_two_way_split` is 1,234 cases (2.9); `any_three_way_split` has both `i`
  and `j` in `split(m)`, `Σ (len(m) + 2)^2` = 4,824 tuples before the `i <= j`
  filter.
* **Block-boundary hints.** `D(Bytes)` stops at length 8 and the property
  generator at 64; SHA-1's interesting lengths are 55/56 (padding spills into a
  second block) and 63/64/65 (a chunk crosses the block). A driver's `lengths`
  hint adds, for every `Bytes` binder of a claim that calls the driven operation,
  the fills `00×L` and `a5×L` for each hinted `L` to the binder's exhaustive
  domain (recorded as `bytes_d1+lengths(55,56,...)`), and makes the property
  generator draw a hinted length with probability 1/4. The domain bytes are
  counted against the 64 MiB domain budget with checked arithmetic (W0706 when
  over). With the hints of 3.2, `any_two_way_split` covers every split point of
  messages that straddle one and two blocks.
* The relation bias of VER04h §4 gains a *re-chunking* relation for a driven
  `List<Bytes>` binder pair: the second list is the concatenation of the first,
  re-split at a random point, so property suites hit equal-content different-split
  pairs from the first case.

### 3.8 Evidence-generation impact

Driven operations are ordinary operations to every suite: examples, exhaustive,
property, fuzz, tables, WX-H output-bound suites and WX-G postconditions all call
them through the driver adapter. Domains are those of the driven parameter's type
(VER04j §4.7) plus the hints of 3.7; fuzz decodes a `List<Bytes>` as a count byte
(modulo 9) followed by length-prefixed elements. Manifests gain, per driven
operation, `driver: {obligations: [...], steps_max_observed}` and the obligation
entries of 3.5 with `kind: "driver"`; v0 and non-driver manifests are unchanged.

### 3.9 Cross-language portability (VER04i)

A driven operation is an ordinary entry in `corpus.json.ops` with an added
`"driven": "<param>"`; the lane runner implements the drive loop once (about 40
lines in `warrant-corpus`), and the per-lane `warrant.binding.json` carries the
JSON record of 3.2 with lane-native names. Driver faults join the fault registry
with `"applies": "sequence"`. Lane notes:

| Lane | Natural form | Hazard the obligations catch |
|---|---|---|
| Python | `hashlib`-style object: `update`, `digest()` | none special (`digest()` is observational: `receiver: ref`) |
| Java | `MessageDigest.update` / `digest()` | `digest()` **resets** the object: it is `receiver: own`; a binding that says `ref` fails `finish-observational` |
| Go | `hash.Hash`: `Write`, `Sum(nil)` | `Write` returns `(n, err)`; `err` maps through `errors`; `Sum` appends to its argument (pass `nil`) |
| C | `*_init`, `*_update`, `*_final(ctx, out)` | `final` pads in place: `receiver: own`; the state lives in a lane-allocated struct |
| Haskell, OCaml | pure `step :: S -> X -> S`, `finish :: S -> R` | `receiver: own` everywhere; the loop is a strict left fold |
| TypeScript | class with `update`/`digest` | `Uint8Array` views must be copied per step (aliasing) |

A lane that cannot express a driver for an operation marks every target that
calls it `not-live` (VER04j §8 item 4); it never substitutes the one-shot export.

### 3.10 Compatibility

Binding-only. A document is unchanged; an operation without a `driver` binds and
generates exactly as before. Drivers are refused on v0 documents (W0711), so the
frozen v0 harness never needs the driver prelude.

### 3.11 Test plan (WX-F)

* **Schema negatives (W0501):** `op` with both `export=` and `driver`; with
  neither; a `driver` line for an operation without the flag; two `driver` lines;
  a path segment with `;`, `{` or `..`; a 9-segment path; a 129-byte identifier;
  receiver `mutable`; result `stream`; `lengths` unsorted, 33 values, or a value
  of 4,097; a repeated key; an unknown key.
* **Consistency negatives (W0711):** driven parameter of type `U8`; `elem.2` on a
  pair; `elem` on a `Nat` driver; a `bool` host for a `Bytes` element; an
  unconsumed second parameter; `result=collect` on a `Bytes<20>` result; a
  fallible step on a non-`Result` operation; an `errors` map naming a case the
  operation's error type lacks; a driver on a v0 document; a secret label without
  `proofs_not_live=driver`.
* **Fault set:** a `faults=` missing `reorder-chunks` (W0502); `drop-last-output`
  on a `finish`-mode driver (W0701); a driver no claim exercises with two
  non-empty elements (W0601).
* **Obligations and their controls:** a subject with a hidden global counter fails
  `deterministic`; a subject whose `&self` finish pads in place (via interior
  mutability) fails `finish-observational`; a binding with a misspelled method
  fails `shape` at compile time; every obligation's control fails as specified in
  3.5.
* **Every fault:** each of the six yields `not ok` under `WARRANT_EVIDENCE_FAULT`
  against the sha1 and range-coder pilots (where applicable).
* **Conversion pilot:** `sha1` (`Digest::new/update/sum1`) with the claims of 3.1
  and 3.7 and the hints of 3.2; second, the `range-coder` round trip
  `decode_bits(encode_bits(bs), map(bs, prob_of)) == map(bs, bit_of)` (a
  `finish`-mode encoder and a `collect`-mode decoder, `new=new(param.data:bytes)`,
  with a refinement keeping `len(data) >= 2` because `BoolDecoder::new` panics on
  shorter input); third, sha256's fallible `try_update`.

## 4. WX-G — in-out postconditions

### 4.1 Motivation

| API | Host signature | What must be stated |
|---|---|---|
| `coding_adventures_zeroize::zeroize_bytes` | `fn(&mut [u8])` | every byte of the buffer is zero afterwards and its length is unchanged |
| `zeroize` for `[u8; N]` / integers | `Zeroize::zeroize(&mut self)` | the value is zero afterwards |
| `sqlite-file::varint::write` | `fn(i64, &mut Vec<u8>) -> usize` | the old contents are a prefix of the new, the appended bytes are the minimal varint, and the return value is their count |
| `tls13-alpn` `encode` | `fn(self, &mut [u8]) -> Result<usize, AlpnError>` | on success, the first `n` bytes hold the encoding; on error, nothing is promised |
| `hkdf::hkdf_expand_fixed` | `fn(&HkdfPrk, &[u8], &mut [u8]) -> Result<(), HkdfError>` | the output buffer holds `len(output)` bytes of OKM |

Bug classes: the mutation lands on a temporary copy (zeroizing a copy of a key,
CWE-226 at value level), an off-by-one loop bound leaves the last byte unwritten
(CWE-193), an append that overwrites existing data, a cursor that advances by
the wrong amount. VER04j §9 deferred exactly this ("needs `ensures post(a)` over
an in/out parameter"), naming zeroize.

**Design in one sentence.** The document stays functional: an operation with an
`inout` parameter *returns the post-state* as (part of) its result; parameter
names always denote the *pre-state*; the binding adapter is what turns a `&mut`
call into that pure function; and an `ensures` block states, per call, a relation
between the pre-state (the parameters) and the post-state (`result`). There is no
`old()`, no mutation and no state in the language.

### 4.2 Language additions

```text
warrant inout_excerpt v1
  -- excerpt: contracts omitted; varint_model is a WX-D model not shown, and the
  -- alpn signature is simplified from the crate's `encode(self, &mut [u8])`
  op zeroize(buf: inout Bytes) -> Bytes
  model is_zero(x: U8) -> Bool = x == 0u8
  ensures zeroize:
    len(result) == len(buf)
    all(result, is_zero)

  op write_varint(value: I64, out: inout Bytes) -> (Nat, Bytes)
  ensures write_varint:
    take(result.1, len(out)) == out
    result.0 == len(result.1) - len(out)
    drop(result.1, len(out)) == varint_model(value)

  op encode_alpn(names: List<Bytes>, out: inout Bytes) -> Result<(Nat, Bytes), AlpnError>
  ensures encode_alpn:
    match result with Ok(r) -> len(r.1) == len(out) and r.0 <= len(out) | Err(e) -> true
```

1. **`inout` mode.** A parameter may be written `name: inout T`. Its value at the
   call is the pre-state; the operation's result carries its post-state.
2. **The post-state slot.** For an operation with `inout` parameters
   `p1..pk` (in parameter order) of types `P1..Pk` and declared result `R`, the
   post-states are found by the first rule that matches:

| Rule | `R` | Post-states | Host returns |
|---|---|---|---|
| S1 | `k = 1` and `R = P1` | the whole result | nothing (`()`) |
| S2 | `k = 1` and `R = Result<P1, E>` | the `Ok` payload; none on `Err` | `Result<(), E>` |
| S3 | a tuple `(C1, ..., Cn)`, `n >= 2`, `n >= k`, whose last `k` components equal `P1..Pk` | those components | the remaining `h = n - k` components: `()` if `h = 0`, `C1` if `h = 1`, the tuple `(C1..Ch)` otherwise |
| S4 | `Result<(C1, ..., Cn), E>` with the same tail condition | those components of the `Ok` payload | `Result<_, E>` of the remaining components as in S3 |

   Equality here is exact type equality. No match is **W0713**. Rule order makes
   the slot unique: S1 is tried before S3, so a single `inout` parameter whose own
   type is a tuple equal to `R` is "the whole result". A post-state is never
   inside an `Option`, a `List` or a user variant (W0713): the slot must be
   reachable by projection alone, so that the adapter knows statically where to
   put the buffer it reads back.
   * S3 with the post-state *outside* a `Result`, as in
     `op read(data: Bytes, pos: inout Nat) -> (Result<U8, E>, Nat)`, observes the
     post-state on errors too; S4 only on success. The author chooses by the
     result type.
3. **The `ensures` block.** `ensures op:` followed by one to 32 Bool rows (more
   is W0602). In a row, the operation's parameters are bound to the argument
   values (for an `inout` parameter, the pre-state) and the contextual name
   `result` to the operation's return value. A row may call models and built-ins,
   including WX-D; it may **not** call any operation and may **not** contain a
   quantifier (both W0713). An `ensures` block may be attached to any operation,
   with or without `inout` parameters: "output relates to input" is useful for
   pure operations too, and the `inout` mode is what makes it reach in-place APIs.
4. **Meaning.** The specification of `op` includes, for every argument tuple
   `a` admitted by the parameters' refinements, `r1 and ... and rn` with the
   parameters bound to `a` and `result` bound to `op(a)`. The rows add evidence
   obligations (4.7); they never change what a call returns.

### 4.3 Typing rules

* `inout` types are admissible when they can be passed and read back by a host
  mutable reference: `Bool`, the fixed-width integers, `Nat`, `Bytes`, `Bytes<N>`
  with a literal `N`, `List<T>` with `T` one of those **or an admissible record**
  (amended 2026-11-14: a list of records, for `&mut [FlipFlopState]`), and records
  whose fields are all admissible. A tuple, as an `inout` type or as a record
  field, remains inadmissible. Anything else is W0713 (an `Option` cannot be "written into";
  a variant would need a constructor-changing write).
* At most 4 `inout` parameters per operation (W0713). An `inout` parameter is a
  runtime parameter (never static). A refinement on it constrains the
  pre-state.
* An operation with `inout` parameters cannot be `fresh` (W0713): a post-state is
  by design the caller's own buffer, which is the opposite of a fresh result.
* In an `ensures` row, `Γ` contains each parameter at its declared type and
  `result : R`; each row must have type `Bool` (W0301). An operation parameter
  named `result` on an operation that has an `ensures` block is W0203 (the
  contextual binder would shadow it); without an `ensures` block such a
  parameter stays legal.
* `ensures` names a declared operation (W0401); one block per operation (W0201).
* `labels` rows of an `inout` parameter classify both its pre-state and its
  post-state; an `input op.param` block constrains the pre-state, as for any
  parameter. A WX-H `output` row bounds the result as written in VER04j §6.2 (for
  S1, the post-state's size).

### 4.4 Grammar

```text
parameter    = NAME ":" [ "inout" ] type [ "|" expression ] ;      -- "inout": v1 only
ensures_decl = "ensures" NAME ":" NEWLINE INDENT ensures_row { ensures_row } DEDENT ;
ensures_row  = expression NEWLINE ;
declaration += ensures_decl ;                                      -- v1 only
```

`inout` is contextual: it is a mode only when the NAME `inout` is immediately
followed by the start of a type (a NAME or `(`). `x: inout` alone (a type alias
named `inout`) keeps meaning the alias. No new token; `ensures` and `result` are
recognised by position. In a v0 document both forms are W0101 (1.2).

### 4.5 Evaluation semantics

An operation is still a pure function from arguments to its declared result; a
call in a claim returns the whole result, post-states included. The ground
evaluator never calls operations, so it needs nothing new for calls. `ensures`
rows are evaluated like WX-E `holds` rows (VER04j §5.2): with the parameters and
`result` bound as ground values, under the meter of the evaluation that made the
call (2.7). A row that cannot be evaluated (a model overflow, exhaustion) fails
the check with the note `postcondition-unevaluable`, a defect of the document; it
never counts as satisfied.

### 4.6 Bindings: the `&mut` adapter

Each `inout` parameter declares its host form:

| Host form | Host argument | Warrant types | Read-back |
|---|---|---|---|
| `slice` | `&mut [u8]` | `Bytes`, `Bytes<N>` | the same slice; its length cannot change, and the adapter checks it equals the pre-state length (a lane where it can change fails `corpus-malformed`) |
| `vec` | `&mut Vec<u8>`, or `&mut Vec<ht>` for `List<T>` | `Bytes`, `List<T>` | the vector after the call, any length |
| `array` | `&mut [u8; N]` | `Bytes<N>` | the array |
| `int:<ht>` | `&mut u8` … `&mut i64`, `&mut usize` | fixed-width, `Nat` | the integer (a `Nat` pre-state the host cannot hold is unreachable by type) |
| `bool` | `&mut bool` | `Bool` | the Bool |
| `struct:<Name>` | `&mut Name` for a declared WX-A `struct` line | records | field-wise observation |
| `returned` | none: the host returns the post-state as its result (functional lanes, 4.9) | any admissible | the host result |
| `slice-of:struct:<Name>` | `&mut [Name]` for a declared `struct` line (VER04m E4) | `List<R>`, `R` a record | each element field-wise; the length cannot change and the adapter checks it equals the pre-state length |
| `vec-of:struct:<Name>` | `&mut Vec<Name>` | `List<R>`, `R` a record | each element field-wise, any length |
| `codec:<Record>` | a host object built and observed through a `codec` line (VER04m E11) | the record | the observation, from the same object that was called |

Line form: `op zeroize export=zeroize_bytes inout=buf:slice faults=stale-post,skip-last-write`.
JSON form: `"inout": { "buf": "slice" }`. For an operation with an `inout`
parameter, `export=` may be a Rust path of at most 8 segments relative to the
crate root (`varint::write`, `Zeroize::zeroize`), called in fully qualified
form (`coding_adventures_zeroize::Zeroize::zeroize(&mut host)`), which reaches
inherent and trait methods without imports; the arguments are passed in the
operation's parameter order. The `ret=` and `errors=` options describe the
*host* return of the slot table (4.2), never the post-states. (Amended 2026-11-14.) With
`error=<CASE>`, an S2 or S4 slot may be served by a host export that does **not** return a
`Result` but the remaining components directly: a panic is normalised to `Err(E.CASE)` and carries no
post-state, and an ordinary return is `Ok` of the components with the read-back post-state
(`d_flip_flop(data, clock, &mut state) -> (u8, u8)` against the result type
`Result<((U8, U8), FlipFlopState), GateError>`). A missing host form,
or one that cannot carry the parameter's type, is W0704; a malformed option is
W0501.

Generated shape (Rust):

```rust
fn adapt_write_varint(value: &V, out: &V) -> Result<V, AdapterError> {
    let pre_out = bytes_of(out)?;                  // the oracle's value, never handed to the subject
    let mut host: Vec<u8> = pre_out.to_vec();      // a fresh buffer the subject may mutate
    let n = sqlite_file::varint::write(i64_of(value)?, &mut host);
    Ok(V::Tuple(vec![nat_of_usize(n), V::Bytes(host)]))   // S3: (host return, post-state)
}
```

Two adapter rules carry the soundness of the design:

1. **Pre-state isolation.** The subject receives a fresh copy. The value the
   claim and the `ensures` rows use as the pre-state is the adapter's own,
   untouched value, so a subject cannot corrupt the oracle's view of the input.
2. **Read-back from the same buffer.** The post-state is read from the very
   buffer passed to the subject after the call returns, never from a copy taken
   before it. The `stale-post` fault (4.8) is exactly the violation of this rule,
   so the rule is checked by a control, not trusted.

### 4.7 Evidence-generation impact

* **Synthesized suites.** For every operation with an `ensures` block, the
  generator registers `exhaustive`, `property` and `fuzz` suites with ids
  `warrant/<module>/<kind>/<op>/post` (claim field `<op>/post`), over the
  operation's parameters as binders with their refinements: every domain, budget,
  demotion and cap of VER04h and VER04j applies, as for WX-H's `output-bound`
  suites (VER04j §14 item 32). `inout` parameters take the domains of their types.
* **Monitoring.** Every call of the operation that any generated test makes
  (examples, claim instances, table rows, other synthesized suites, a driver's
  enclosing operation) is followed by evaluation of its `ensures` rows on the
  call's arguments and result, when the arguments satisfy the operation's
  parameter refinements (a call outside the admitted domain is governed by the
  input contract, not the postcondition). A violation fails the instance with
  the note `postcondition-violated: <op> row <i>` and its suite's registered
  failure id. Rows cannot call operations (4.2), so monitoring never recurses,
  and the rows charge the enclosing evaluation's meter, so monitoring cannot
  multiply the budget.
* **Example self-consistency (W0601).** For an example of the form
  `op(<ground args>) == <ground expected>`, the generator evaluates the rows with
  `result` bound to the expected value. If a row is false, the document
  contradicts itself (VER04 §8: "examples consistent with claims") and generation
  fails with W0601 before any subject runs.
* **Manifests.** Synthesized entries as for WX-H, plus `monitored_calls` and
  `postcondition_rows` per operation.

### 4.8 Sabotage implications

Faults are rewrites of the adapter's read-back, scoped to the operation:

| Fault | Rewrite | Applies when | Bug modelled |
|---|---|---|---|
| `stale-post` | the subject is called on a copy and the post-state is the pre-state | every `inout` operation | mutation applied to a temporary copy (the zeroize-a-copy bug) |
| `skip-last-write` | the last element of the post-state is replaced by the pre-state's element at the same index | `Bytes`/`Bytes<N>`/`List` `inout`, pre-state length at least 1 | off-by-one loop bound (CWE-193) |
| `drop-last-push` | if the post-state is longer than the pre-state, its last element is removed | `vec` host form | a lost append |
| `clobber-prefix` | if the pre-state is non-empty, the first element of the post-state has its low bit flipped (`U8`) or is replaced by the element domain's first value | `vec` host form | an append that overwrote existing data |
| `cursor-off-by-one` | an integer post-state gets `+1` when even and `-1` when odd, wrapping in its type | `int:` host form | a cursor advanced by the wrong amount |

Each is caught only if the `ensures` rows or the claims distinguish the faulty
post-state; the runtime control enforces it (a fault no suite catches fails its
control). `stale-post` in particular fails against any honest postcondition of a
mutating operation: for `zeroize`, any non-zero pre-state in the domain (`b"01"`)
violates `all(result, is_zero)`. A postcondition so weak that the identity
satisfies it is therefore caught by the control, which is the point.

### 4.9 Cross-language portability (VER04i)

The post-state is part of the result, so the corpus needs no new value
encoding: a result is encoded as its declared type. `corpus.json.ops.<op>` gains
`"inout": ["<param>", ...]` and `"post_slot": "S1" | "S2" | "S3" | "S4"` so each
lane adapter knows where to put the buffer it reads back. Lane notes:

| Lane | `&mut` analogue | Hazard |
|---|---|---|
| C | `uint8_t *buf, size_t len` (and `size_t *len` for growable) | the adapter must pass a buffer distinct from every other argument; aliasing is legal C |
| Java, Kotlin, C# | `byte[]` mutated in place | same aliasing rule |
| Python | `bytearray` or `memoryview` | `bytes` is immutable: a binding passing `bytes` cannot observe writes |
| TypeScript | `Uint8Array` | a `subarray` view aliases its parent; copy first |
| Go | `[]byte` (fixed length) or `*[]byte`; `append` may reallocate | a function returning the new slice is host form `returned` |
| Haskell, Elixir, OCaml (immutable `bytes`) | none | host form `returned`: the export returns the post-state; the Warrant meaning is the same pure function |

### 4.10 Compatibility

Both forms are new syntax, W0101 in v0. In v1, an operation without `inout` and
without `ensures` means and generates exactly what it did. The contextual words
take nothing from existing documents: `inout` is a mode only when followed by a
type, and `result` is bound only inside `ensures` rows.

### 4.11 Test plan (WX-G)

* **Syntax:** `inout` before every admissible type; `x: inout` as an alias type;
  `ensures` with 1 and 32 rows; v0 rejections (W0101) for both forms with the
  identical text accepted under `v1`.
* **Checker negatives:** each slot rule's near misses (a `Result<P, E>` with
  `k = 2`, a tuple whose tail is in the wrong order, the post-state inside an
  `Option`) are W0713; `inout Option<U8>`, five `inout` parameters, `inout` on a
  `fresh` operation (W0713); a row calling an operation, a row with `forall`
  (W0713); a non-Bool row (W0301); 33 rows (W0602); `ensures` of an undeclared
  operation (W0401); two blocks (W0201); a parameter named `result` (W0203).
  The slot resolver is differential-tested against a brute-force enumerator over
  all result types of depth 2 built from three base types.
* **Adapter controls:** pre-state isolation (a subject that mutates its input
  slice cannot change the pre-state the rows see); read-back (`stale-post` is
  caught on every pilot); a `slice` host form cannot change length.
* **Monitoring:** a postcondition violated only on a call nested inside a claim's
  larger expression is reported with that claim's instance; an example whose
  expected value contradicts a row is W0601 at generation; an unevaluable row
  fails with `postcondition-unevaluable`.
* **Every fault** of 4.8 yields `not ok` under `WARRANT_EVIDENCE_FAULT`.
* **Conversion pilot:** `zeroize`: `zeroize_bytes` (`slice`), the `[u8; 32]` and
  `u64` trait impls (`array`, `int:u64`), with `stale-post` and `skip-last-write`
  caught. Second: `sqlite-file::varint::write` (`vec`, S3) with `drop-last-push`
  and `clobber-prefix` caught and a WX-D varint model in the third row.

### 4.12 What WX-G is not

* **Not an erasure proof.** The harness reads the buffer back, which makes the
  zeroing stores observable, so the compiler cannot remove them *in the test*.
  The real CWE-14 risk is a zeroize whose result is never read (on drop), where
  dead-store elimination applies. Value-level evidence cannot see that; the
  optimised-IR check and VER20 own it, and the zeroize report keeps CWE-14 as an
  exemption until they exist.
* **Not a capacity scrub.** `Vec<u8>::zeroize` also wipes the spare capacity; no
  Warrant value contains spare capacity, so that part is unobservable here.
* **Not stateful objects.** `&mut self` receivers whose state is not a Warrant
  value (bitset, heap) need a state codec (open question 11); WX-F drives
  objects but does not expose their state. (Amended 2026-11-14: the `codec` line,
  the explicit-state classification and the dispatch driver for protocols are
  specified in [VER04m](VER04m-warrant-stateful-objects.md); heaps and hash maps
  without a canonical observer stay protocols there.)

## 5. Cross-cutting evidence and portability rules

1. **One semantics, three evaluators.** The ground evaluator, the generated model
   code and each lane's IR evaluator implement WX-D and `ensures` rows
   separately. They must agree on values *and* charges (2.15); VER04j §8 item 1's
   differential rule extends to the meter.
2. **Manifest additions** (all optional; existing manifests unchanged):
   `seq-aware` and `split(...)` domain text, `unroll` on proof entries, `driver`
   per driven operation with `kind: "driver"` obligation entries,
   `monitored_calls` and `postcondition_rows`.
3. **New failure notes** (all carried by registered TF01 failure ids, as VER04j
   §14 item 34 does): `model-budget-exhausted`, `driver-steps-exceeded`,
   `postcondition-violated`, `postcondition-unevaluable`. Obligation failures
   (`driver-nondeterministic`, `finish-not-observational`) use the same
   mechanism.
4. **Not-live honesty.** A lane without the meter, a driver loop or an inout
   adapter marks every dependent target `not-live`; it never runs a weaker
   substitute (VER04j §8 item 4).

## 6. Security considerations

### 6.1 Threat model

The Warrant document, the bindings file, table fixtures and the subject are all
untrusted inputs to the tools. Documents and bindings are reviewed, but the tools
must be safe against a hostile one (a malicious pull request, a fuzzed
document). The subject is hostile at run time: it may panic, allocate without
bound, mutate what it is given or keep hidden state. The earlier WX-A review
found two denial-of-service issues in exactly this class (type expansion and
exhaustiveness work); WX-E's review found unbounded loader reads. The rule
followed here is the one those reviews forced: **every quantity an input
controls has an explicit cap, checked with checked arithmetic, before the
allocation or recursion it would drive.**

### 6.2 Resource caps

| Cap | Value | Where enforced | Checked before | On exceed |
|---|---|---|---|---|
| work per evaluation | 1,000,000 units | every evaluator | each charge | W0703 / `model-budget-exhausted` |
| evaluation depth (nested node evaluations, across model calls) | 64 levels | every evaluator | each node entry | same |
| elements per sequence value | 65,536 | every evaluator | allocation (2.7 step 1) | same |
| leaves per tuple, record or constructor value | 65,536 | every evaluator | returning the composite (2.7) | same |
| owned data per evaluation, cumulative (`dsize`, stored constructor and error names included, E20) | 64 MiB | every evaluator, for v1 documents | allocation (2.7 step 2) | same |
| list literal | 4,096 elements | checker, v1 documents | checking | W0602 |
| identifier, in any position | 128 bytes | checker, v1 documents | every other check (pass 0) | W0602 |
| section bound arguments | 8 | checker | — | W0712 |
| higher-order nesting | 4 | checker | — | W0602 |
| model graph cycle detection | linear in edges | checker (charged to `max_work`) | — | W0602 |
| unroll `K`, shapes, unroll product | 16 (max 64), 4,096, 4,096 | symbolic consumers | encoding | `unknown` + W0602 |
| VER10 circuit | 250,000 nodes, 2,000,000 gates, 10^6 conflicts, 2^30 propagations | VER10 | each operation | `Unknown` |
| split / hint domains | the exhaustive budget `B` and 64 MiB domain bytes | generator and generated prelude (`within_budget`) | enumeration | W0706 |
| driver steps per call | 65,536 | adapter | `new` | `driver-steps-exceeded` |
| driver arguments per call, element path depth | 8, 4 | binding parser | generation | W0501 |
| `lengths` hints | 32 values, each <= 4,096 | binding parser | generation | W0501 |
| `errors` map | 64 entries | binding parser | generation | W0501 |
| path segments | 8, each <= 128 bytes | binding parser | splicing into source | W0501 |
| bindings file, line | 1 MiB, 4,096 bytes | binding reader | reading (bounded read) | W0501 |
| `inout` parameters, `ensures` rows | 4, 32 | checker | — | W0713, W0602 |
| `deterministic` obligation sequences | 16, each within the driven type's domain | generator | — | fixed |

Every product or sum of sizes (split-domain counts, hint fills × binders, shape
counts, unroll products) is computed in `u64` with checked operations; overflow
counts as over budget, never as zero (the VER04h §3 lesson: eight `U8` binders
wrapped to zero cases).

### 6.3 Malformed bindings

* The reader is a bounded line parser: at most 1 MiB is read (no
  read-to-end of an unbounded file), lines longer than 4,096 bytes are refused,
  and the same symlink and regular-file rules as the WX-E loader apply
  (VER04j §14 items 41 and 52). Backlog WD-014 recorded that `gen_evidence` file
  I/O is uncapped; this specification makes the cap normative for the bindings
  file.
* Every string that reaches generated source (paths, method names, host types)
  is validated against a closed grammar (Rust identifiers and `::` paths, the
  host-type grammar) **before** generation, and refused, never escaped, when it
  fails. A path segment such as `new(); std::process::exit(0); //` cannot pass.
* Unknown keys, repeated keys, unknown vocabulary words and dangling references
  (a `driver` line for no operation, a host type with no `struct` line) are
  errors. Nothing is defaulted silently except the documented defaults (a step
  returns `unit` unless `->` says otherwise).
* The complete binding is validated against the checked document (3.3, 4.6)
  before the first byte of generated code is produced, so generation never emits
  a partial harness.

### 6.4 Fail-closed behaviour

| Situation | Outcome |
|---|---|
| budget exhaustion anywhere | incomplete or failed, never a truncated value, a skipped case, a pass or a proof |
| a symbolic cap hit | `unknown`, never `bounded(K')` for a smaller `K'` |
| an `ensures` row that cannot be evaluated | the instance fails (`postcondition-unevaluable`) |
| a driver step limit | the instance fails before the subject is called |
| an unknown fault name, driver key or host form | generation fails |
| a lane missing the meter, driver loop or inout adapter | the dependent targets are `not-live` |
| a faulted or filtered run | writes no manifest (unchanged, VER04h §6) |
| a driven operation with secret labels or bounds | the obligations are `not-live` and must be acknowledged by `proofs_not_live=driver` in the reviewed bindings file |

### 6.5 Subject-side hazards

* **Oracle corruption.** Pre-state isolation (4.6) and fresh per-step element
  conversion (3.5) keep every value the oracle uses out of the subject's reach.
* **Allocation.** A driven or in-place operation with a WX-H `heap`/`output` row
  runs inside the capped allocation observer for the whole protocol; one without
  such a row is no worse than any operation today (the observer is not a
  sandbox, VER04j §14 item 50).
* **Non-termination** of a subject call is outside value evidence, as for every
  operation (VER04 §8: termination needs an admitted observer or a proof); the
  driver adds at most `MAX_DRIVER_STEPS` calls per instance, so it cannot turn a
  terminating subject into a non-terminating harness.

## 7. Test plan summary

Per-extension plans are in 2.15, 3.11 and 4.11. Cross-cutting:

1. **Compatibility:** v0 evidence and wave-3 v1 evidence byte-identical; the
   checker's v0 output unchanged; the new built-in names rejected in v1 and
   unbound in v0.
2. **Security falsification:** one test per row of 6.2 showing the cap fires
   before allocation (an allocation probe in the Rust evaluator and adapter), one
   per bullet of 6.3 (including an injection attempt in each spliced field), and
   one per row of 6.4.
3. **Independence:** the Python reference evaluator for WX-D and `ensures` rows is
   written from this text, not from the Rust code (closing, for this wave, the
   gap VER04j §14 item 12 recorded for wave 1). It lands with WX-D2, as section 8
   always placed it, and then covers the WX-D1 forms too. Until then the
   hand-computed triples of 2.15 are WX-D1's independent check (S0 E12).

## 8. Recommended implementation order

Precondition: the WX-H/WX-E branch (`claude/warrant-wx-h-e`) has merged.

| Step | Slice | Contents | Size (engineer-days) | Depends on |
|---|---|---|---|---|
| 1 | **WX-D1** sequence core | the twelve names reserved; `take drop slice replicate range to_list to_bytes`; list literals in evidence bodies; the meter in generated code (gated); sequence-aware lists; the split domain; the two result faults; in the slices S0–S4 of 8.1 | 4–6 | — |
| 2 | **WX-F** drivers | schema (JSON + line form), validation, driver adapter, obligations and their controls, six faults, hints, sha1 pilot | 5–7 | D1 (list literals, split sweeps) |
| 3 | **WX-G** in-out postconditions | `inout`, slot resolution, `ensures`, monitoring, inout adapters, five faults, zeroize pilot | 5–7 | D1 (`replicate`, `take`, `drop`) |
| 4 | **WX-D2** higher-order forms | `fold map count all any`, sections, W0712, acyclicity through references, nesting cap, Python reference evaluator, wasm-leb128 pilot | 6–9 | D1 |
| 5 | **WX-D3** symbolic unrolling | shapes, unrolling, labels and caps for VER19 §5's spec circuit; `gmul` `proved`, fixed-width folds over `Bytes` `bounded(16)` | 4–6 | D2, a VER19 spec-circuit path |

Total about 24–35 engineer-days, plus per-lane work under VER04i (the IR tags, the
drive loop and the inout adapter, each written once per lane). Steps 2 and 3 are
independent and can run in parallel after step 1.

**Deviation from VER04j §11** ("WX-F, then WX-D"): D is split, and its cheap,
non-higher-order part goes first because both F and G need it; the expensive
higher-order part follows F and G so that the two unlocks with real pilots
(streaming digests, zeroize) are not held behind the slice with the most design
risk.

### 8.1 WX-D1 slices (S0 review)

WX-D1 lands in five slices, all in `rust/warrant-grammar` except S0. Each is one
pull request; each keeps v0 output and wave-3 v1 output byte-identical (1.3).

| Slice | Contents | Depends on |
|---|---|---|
| **S0** | This errata: the decisions of section 9.1 applied to the text, and the VER04 §4 deviation note. `code/specs` only | — |
| **S1** checker and ground evaluator | Reserve the twelve names (W0201/W0203); W0202 "not yet implemented in this version" for `fold map count all any`. The typing of 2.4 for the seven forms, with an expected type threaded into `typecheck_v1_builtin`. The v1 list-literal cap (W0602). The seven forms in the ground evaluator with check-before-allocate (element cap, then data reservation, then work, then allocation) on exact `i128` counts. The data counter with `dsize` and the pass-through rule. Short-circuit `and`/`or`/`=>` in v1. Operation discovery into list literals in v1. Generation still refuses the forms with W0701. No field is added to `CheckedWarrantModule` (the `v0_compat` `Debug` hash); `CodegenLimits` and `EvalValue` keep their shape, and the new limits are constants | S0 |
| **S2a** generator | Translate list literals and the seven forms. The gated `seq` prelude part with both caps and the data reservation. The `wxd` fixture, subject and regeneration. Flip the pinned W0701 refusals of list literals in v1 bodies and the pinned "lists are not searched" discovery assertion (`tests/wx_generator.rs`) | S1 |
| **S2b** meter parity | The full meter in generated code: work, depth, data, groups, short-circuit, admission inside the meter. The `model-budget-exhausted` note. The `(value, work, data)` differential of 2.15, run with and without `CI=true` | S2a |
| **S3** domains | Sequence-aware lists and the split domain (2.9), gated to `seq`, with the conflict refusal (W0706) and the manifest text | S2a |
| **S4** result faults | `drop-last-result-element` and `reverse-result-list` (2.12). They rewrite the adapter's result and need no WX-D form, so they are independent of S1–S3 | S0 |

Order: S0, then S1, then S2a; S2b and S3 after S2a, in either order or in
parallel; S4 at any time after S0.

## 9. Open questions for the owner

1. **Sections.** Function arguments may bind leading arguments (`gmul_step(b)`),
   defunctionalised, not closures (2.3). The VER04j sketch allowed only bare
   names, which forces context through the accumulator and makes `map` with
   context impossible. Confirm sections.
2. **Count arguments.** `take`/`drop`/`slice`/`replicate`/`range` take `Int` and
   treat negatives as zero (2.2). The alternative is `Nat` counts plus a typing
   change `Nat + Nat : Nat` (which would also close VER04j §14 item 5 for shift
   counts) or a `to_nat` on `Int`. Choose. **Resolved (owner, 2026-11-11):
   accepted as drafted**, `Int` counts with negatives as zero (9.1, E-Q2).
3. **v1 stability.** This wave reserves twelve more names in v1. Freeze v1 after
   this wave (any later name needs a `v2` header), or keep v1 as an evolving
   profile with a mechanical scan of in-repository v1 documents? **Partly
   decided (owner, 2026-11-11):** all twelve names are reserved in WX-D1, so this
   wave reserves names once rather than in two steps (9.1, E7). Whether v1 is
   frozen after this wave is still open; the reservation does not freeze it.
4. **Data accounting.** Cumulative allocation (deterministic, over-approximate)
   rather than peak live data (2.7). Confirm. **Resolved (owner, 2026-11-11):
   accepted as drafted**, cumulative per evaluation with the 64 MiB budget (9.1,
   E-Q4).
5. **Double-finish** is an obligation with its own control, not a fault (3.5).
   Confirm this reading of the VER04j sketch.
6. **Mandatory driver faults.** Every applicable driver fault must be listed
   (W0502 otherwise). The alternative is a generator that adds them silently;
   the listed form keeps the bindings file the reviewable record.
7. **Context objects.** `ctx` (a value built once and lent to `new`) is specified
   for `rans` (`RansEncoder::new(&AnsTable)`). Keep it in WX-F, or defer `rans`?
8. **Driven operations and proofs.** Driven operations' constant-time and
   steps-bound obligations are `not-live` (3.4 rule 5). Should VER04h §7 grow a
   driver-aware lowering (inline `new`, the step loop for a fixed length, and
   `finish`), or is `not-live` acceptable for the digests?
9. **Monitoring cost.** `ensures` rows are checked on every call in every suite
   (4.7), charged to the enclosing evaluation. The cheaper alternative checks
   them only in the synthesized `<op>/post` suites. Confirm monitoring.
10. **Operations in `ensures` rows.** Forbidden (purity, no monitoring recursion).
    A useful relaxation would allow calls to operations that have no `ensures`
    block (`decode(drop(result.1, len(out))) == Some(value)`). Allow?
11. **State codecs.** `&mut self` objects with non-Warrant state (bitset, heap)
    need a build/observe pair of public functions to become `inout` values; the
    heap's internal order is not canonical, so its observer must be chosen with
    care. Defer to a stateful profile, or add `struct`-like codec lines now?
12. **Symbolic defaults.** `UNROLL_K_DEFAULT = 16`, and `proved` allowed for
    obligations whose every sequence length is static (2.10). Confirm both.
13. **More combinators.** `zip`, `filter`, `reverse`, an indexed fold. None is
    needed by the pilots; each costs an IR tag and a differential suite in every
    lane. Add any now?

### 9.1 Errata and decisions (S0, 2026-11-11)

The S0 review checked the WX-D1 part of this document against the code on
`main` (`rust/warrant-grammar`: `codegen.rs`, `checker.rs`, `evidence.rs`, the
evidence prelude and their tests). It found fourteen places where the text was
ambiguous, contradicted itself, or contradicted the code it builds on. Each is
settled below and in the section the last column names. The owner decided the
two open questions and items E2 and E7. The coordinator settled the rest under
one rule: **when in doubt, over-charge and fail closed.** Each of those choices
is open to override by the owner; an override is a new erratum, not a silent
edit.

| ID | Issue | Decision | Decided by | Sections changed |
|---|---|---|---|---|
| E-Q2 | Count arguments (open question 2) | `Int` counts; a negative count means zero. Accepted as drafted | owner-decided | 2.2, 9 |
| E-Q4 | Data accounting (open question 4) | Cumulative allocation per evaluation, never refunded, with the 64 MiB budget. Accepted as drafted | owner-decided | 2.7, 9 |
| E1 | "64 active model frames" against the evaluator, which counts expression nesting (`enter_scope` on every node) | The limit is 64 levels of nested node evaluation across model calls, exactly the ground evaluator's `max_depth` rule. Groups count; a model body is one level below its call; a WX-D per-element call is one level below the form and its body one below that. Generated code mirrors it | coordinator-decided (conservative, override welcome) | 2.6, 2.7, 2.12, 2.15, 6.2 |
| E2 | The ground evaluator evaluates `and`/`or`/`=>` eagerly (`codegen.rs`, the `Expression::Binary` arm), while VER04 §4 says short-circuit and generated Rust short-circuits | Short-circuit in every v1 evaluation. The v0 ground evaluator stays eager, a recorded deviation from VER04 §4, so v0 output stays byte-identical. Eager evaluation never accepts what short-circuit rejects | owner-decided | 1.2, 1.3, 2.6, 2.12, 2.13, 2.14, 2.15, 8.1; VER04 §4, §10 |
| E3 | Parenthesised groups: the evaluator charges 1 per `Grouped` node, while the generator's `emit` drops the parentheses | A group is a node in every implementation: 1 work unit and one depth level. The generator emits the charge under the meter | coordinator-decided (conservative, override welcome) | 2.7, 2.12, 2.15 |
| E4 | Data-meter coverage: the charge table covered only WX-D forms, and the quadratic-accumulator boundary of 11,585 assumed `concat` charges data but little work | Every evaluated node charges `dsize` of its value except the pass-through nodes (references, projections, `if`, `match`, groups, model calls, `unwrap_or`, `fold`). That covers `concat`, list literals, tuples, records, `Some`/`Ok`, `be_bytes`/`le_bytes`, `at` and the WX-D forms. The ground evaluator keeps the data counter for every v1 evaluation; v0 has none. `concat` already charges work per element, so the work meter, not data, stops the accumulator, at about 1,400 elements; the data example is now `replicate`, which fails at `n = 1,024`. Reads charge no data (note below) | coordinator-decided (conservative, override welcome) | 1.3, 2.7, 2.14, 2.15, 6.2 |
| E5 | `replicate`/`take`/`drop`/`slice` of non-scalar elements were charged `L` × unit, while `EvalValue` deep-clones them | Charge the deep copy: `Σ (32 + dsize(e))` over the selected elements, `max(0, n) × (32 + dsize(x))` for `replicate`. The draft's assumption that the payload was charged when the element was built is withdrawn | coordinator-decided (conservative, override welcome) | 2.7, 2.12, 2.15 |
| E6 | Two caps: `MAX_SEQ_ELEMENTS` per sequence against the existing per-composite leaves cap (`bounded()`/`leaves()`, VER04j §14 item 18) | Both stand. 65,536 elements per `List`/`Bytes`, checked before allocation. 65,536 leaves per tuple, record or constructor, checked where the evaluator checks it today. The leaves cap is not applied to lists, since it would lower the element cap to 65,535. Generated code mirrors both | coordinator-decided (conservative, override welcome) | 2.7, 2.15, 6.2 |
| E7 | Reserved names: reserve twelve now, or the five WX-D2 names later | All twelve reserved in WX-D1. Until WX-D2, any use of `fold`, `map`, `count`, `all` or `any` in a v1 document is W0202 "reserved built-in `<name>` is not yet implemented in this version". Tied to open question 3 without freezing v1 | owner-decided (the reservation); coordinator-decided (W0202 as the code: the name is reserved but has no meaning in this tool version; override welcome) | 1.2, 1.4, 2.15, 8, 8.1, 9 |
| E8 | List-literal cap at generation (2.11) or in the checker (6.2). Today only the evaluator refuses, above 65,536 (W0703) | The checker, in v1 documents only, for every literal anywhere: W0602 above 4,096, before the elements are typed. v0 is unchanged. The evaluator's 65,536 refusal stays as defence in depth | coordinator-decided (conservative, override welcome) | Status, 1.2, 1.4, 2.11, 2.14, 2.15, 6.2 |
| E9 | Split-domain gaps: `k` over two sequence binders; `k` also a shift count (`NatCount`); `D(m)` for `List<T>` and `Bytes<N>`; enumeration order; the number the manifest shows | Exact pair counts and budget bounds per type (1,234 and 3,470 for `Bytes`). Order: `m` outer, `k` ascending. The manifest shows the exact pair count, `split(m)(1234)`. A `k` that two domain rules claim is refused with W0706. The intersection was rejected, because it drops the clamp boundary of the longer sequence | coordinator-decided (conservative, override welcome) | 1.4, 2.9, 2.11, 2.15 |
| E10 | Sequence-aware lists: the new shrinker order differs from the `comp` list shrinker, and changing all lists would alter `wxa`'s generated evidence | Only `List<T>` binders that feed a WX-D form, and only in the `seq` part. Every other list keeps the `comp` generator, shrinker and fuzz decoding. `Bytes` is unchanged, since it already draws lengths 0..=64 | coordinator-decided (conservative, override welcome) | 2.9, 2.11, 2.15 |
| E11 | Result-fault edges: an empty list under `drop-last-result-element`; "length at least 2"; lists nested in tuples | Applicability is decided by the core result type at generation; anything else, including a list nested in a tuple, record, variant or list, is W0701. At run time a fault is the identity where it cannot change the value (`[]`, length below 2, a palindrome), and no element is invented. The runtime control is the backstop | coordinator-decided (conservative, override welcome) | 1.4, 2.12, 2.15 |
| E12 | The Python reference evaluator: in the WX-D differential (2.15, 7 item 3) or in WX-D2 (8) | WX-D2, where section 8 placed it; it then covers the WX-D1 forms too. WX-D1's differential is the ground evaluator against generated code, with the native-iterator value oracle and hand-computed pinned triples | coordinator-decided (conservative, override welcome) | 2.15, 7 |
| E13 | Operation discovery skips list literals (`find_operation_*`), so `to_bytes([f(a)]) == b` reports "claim calls no operation" | In v1 the argument search descends into list-literal elements like call arguments, after the unchanged shallow search, first match wins. v0 is unchanged | coordinator-decided (conservative, override welcome) | 1.2, 2.11, 2.15, 8.1 |
| E14 | Are admission and refinement predicates inside the instance meter? | Yes. One meter per case; a rejected candidate is discarded with its meter. Exhaustion during admission fails the instance with `model-budget-exhausted`; it is never treated as a rejection | coordinator-decided (conservative, override welcome) | 2.7, 2.12, 2.15 |
| E15 | Does an unsuffixed integer literal take its type from a sequence context (`to_bytes([0, 1])`)? | No. Unsuffixed integer literals stay `Int` (VER04 §2: only `Nat` is contextual), so `to_bytes([0, 1])` is W0301. Write `to_bytes([0u8, 1u8])`, or bind the elements as `U8`. `to_bytes([])` type-checks. Contextual `U8` literals would be a language change and are not part of WX-D | coordinator-decided (S1, override welcome) | 2.4 |
| E16 | How do expected types flow into the forms? | `replicate(x, n)` has type `List<type(x)>`. The expected element type is passed to `x` as a hint (so `replicate([], n)` types), and the result is never widened to the expected type. This matches v0 list literals. For `take`, `drop` and `slice`, an expected `List<T>` is passed into the sequence operand; an expected `Bytes` is not | coordinator-decided (S1, override welcome) | 2.4 |
| E17 | How is a reserved-but-unimplemented name reported, and what does a WX-D call node cost? | A D2 name used in v1 is W0202 with the exact text "reserved built-in `<name>` is not yet implemented in this version" (a distinct checker kind, so D2 can remove it cleanly). A WX-D call node costs its ordinary node charge of 1 work unit; the form's own work charges are added to that | coordinator-decided (S1, override welcome) | 2.7, 2.15 |
| E18 | Which element cap applies under a stricter profile, and when is data charged during construction? | The evaluator's element cap is the smaller of 65,536 and the profile's `max_value_bytes`, so a stricter profile is never loosened. List literals, tuples and constructors charge data after their components exist and before work; tuples and constructors then also go through the leaves check. `concat` charges before building. A bytes literal charges `len/2` before decoding. After a refused reservation the counter keeps the overshooting total; the instance has already failed | coordinator-decided (S1, override welcome) | 2.7, 6.2 |
| E19 | What does D1 test from §2.15, and what moves later? | Two items move. The quadratic-accumulator work-boundary bisection needs `fold`, so it moves to D2. Admission-in-meter (E14) needs nothing in the ground evaluator, which never evaluates admission predicates, so it moves to S2b with the generated meter | coordinator-decided (S1) | 2.15, 8.1 |
| E20 | The S1 security review: `dsize` priced a nullary constructor and `Err(E.CASE)` at 0, but constructor and error values own their names, so `replicate(<10,000-byte constructor>, 65536)` allocated about 660 MB (peaking at 1.3 GB) with 2 MiB charged; list literals and `concat` did the same, in v0 and v1. Identifiers had no length cap | `dsize` counts the bytes of the stored names: `len(T) + len(C)` for a constructor of type `T`, nullary ones included, and `len(E) + len(CASE)` for an error value, so the charge is never below what a copy allocates. Every implementation charges them, generated code included. In v1 documents the checker also caps every identifier at 128 bytes (`MAX_IDENTIFIER_BYTES`, the binding parser's path-segment cap), as W0602 with the message "an identifier of N bytes is longer than the 128 bytes a v1 document allows", in a pass before every other check, so an over-long reference is refused before resolution. The cap is defence in depth: the name charge is what bounds the copies. v0 is unchanged (no data counter, no cap) | coordinator-decided (S1 security review, override welcome) | Status, 1.2, 1.4, 2.7, 6.2 |
| E21 | The S1 security review: the checker resolved a dotted name by its first segment alone (a binder, a built-in such as `Some`, `None` or `len`, a model or an operation) and typed it as that, while the ground evaluator turns any two-segment name it does not recognise into the error value `Err((seg0, seg1))` and ignores the arguments. So v1 `g.x(a)`, `g` a model, was typed `U8` and evaluated to `Err(("g", "x"))`; `Some.x(a)`, `None.x` and `len.x(a)` likewise; v0 `a.foo` was typed as the binder `a`. An example `select_bytes(..) == m.FORGED(..)` checked, and `compile_warrant_to_tfv` emitted the error id `FORGED`, which no declaration has. The evidence generator already refused such names, so the lanes disagreed | A reference of more than one segment is exactly one of two things: `E.CASE`, with `E` a declared error and `CASE` one of its cases (v0 and v1), or a v1 projection `x.f.g` from a binder `x` in scope. Anything else is W0202 `UnboundName` of the whole dotted name, in v0 and in v1, refused before any first-segment lookup. This is a **v0 deviation**: such v0 documents used to check, but evaluated to a forged error value, never to what their type said. No in-repository document uses one (`git ls-files '*.warrant'`: the only dotted references are `E.CASE` and the v1 projection `b.tag` in `wxa`; `labels` and `input` lines name operation parameters, not references), and every v0 output, the v0 compatibility pin and the regenerated harnesses are byte-identical | coordinator-decided (S1 security review, override welcome) | 9.1; VER04 §10 |

**Note on E4: reads (a bounded-copy guard, with a residual accounting gap).**
The portable data meter does not charge reads, because a charge per read would
make that meter depend on whether an implementation copies or shares, and on
how often a body names a value. The Rust ground evaluator does deep-clone
`EvalValue` on several read paths, however, and those copies are not small. The
S1 security review measured them:

* a list literal of 48 references to one binder whose value was charged about
  16 MB peaked at **2.4 GB** before the literal's own charge failed with W0703;
* a model call with 48 such arguments peaked at **2.4 GB** and **succeeded**,
  with `data_used` at only 16 MB, because a call is a pass-through node and its
  arguments are reads;
* each such clone costs about 33 ms of CPU for about 3 work units, so the work
  meter does not bound the time either;
* the worst case, many references to a value near the 64 MiB budget, inside a
  bounded nesting of calls, is hundreds of GB.

The S1 measurements show that `dsize` does not bound the host copy work. The
64 MiB v1 data meter still caps `dsize` for each produced value, not the
aggregate memory of an evaluation. A separate read-copy guard is selected for
the Rust ground evaluator while preserving the public `EvalValue`,
`CodegenError`, `CodegenErrorKind` and `CodegenLimits` shapes:

* `MAX_READ_COPY_CHARGE_BYTES` is 64 MiB per top-level evaluation. The counter
  is cumulative across nested model calls and expression evaluation, and
  `reset_work` resets it with the existing per-evaluation counters.
* Before any evaluator-owned deep copy, compute its copy-footprint charge using
  checked/saturating arithmetic and refuse before allocating any part of that
  copy if the cumulative charge would exceed the limit. For a copied value
  tree, charge 96 bytes per `EvalValue` node plus the exact payload length of
  every `Bytes` vector and every owned constructor/error name string. A copied
  list or tuple includes the charge for every child node; a copied constructor,
  `Option`, or `Result` includes all nested nodes and names. Multi-input copies
  such as list/byte `concat` reserve the combined charge before copying either
  operand. The walk stops as soon as the charge is over budget and rejects
  values deeper than `TFV_MAX_DEPTH` with W0703 `RecursionDepthExceeded` before
  copying.
* The 96-byte node charge is a fixed, conservative host-accounting assumption
  for the current Rust `EvalValue` layout (96 bytes on the supported 64-bit
  hosts), not the portable value charge. The implementation must fail to
  compile if `size_of::<EvalValue>()` grows beyond 96 until this contract and
  its tests are deliberately updated. The charge is an evaluator copy
  footprint, not a measurement of allocator requests, process RSS, or total
  memory. It excludes the input environment, parser/AST storage, allocator
  metadata/capacity rounding and non-copying value construction.
* The guard applies to v0 and v1 so the pre-WX-D memory hazard is bounded too.
  In v0, a workload whose read-copy charge stays within the limit keeps its
  existing output; an over-limit read returns W0703 with the existing
  `CodegenErrorKind::EvaluationError("read-copy budget exceeded")` shape. This
  does not change `data_used`, whose 32-byte portable `VALUE_OVERHEAD` remains
  separate. The diagnostic mapping preserves the public error enum shape and
  does not imply `work_used` or `data_used` reached its limit.

The guard must cover binder/model argument reads, projection heads and fields,
`at`, `fixed`, `concat`, both `unwrap_or` copy branches, `take`/`drop`/`slice`,
`replicate`, and pattern bindings including copied error names. Match-arm
evaluation must not deep-clone the whole environment; use a borrowed binding
overlay or charge the complete environment copy before creating it. Every
remaining evaluator-owned `Clone`, `to_vec` or equivalent deep-copy path must
have one preflight before allocation. The independent allocator probe constructs
its large binder before measurement and checks both the 48-reference list
literal and 48-argument model-call shapes: with a 16 MiB `Bytes` value, three
copies fit and copy four is refused because `4 * (16 MiB + 96) > 64 MiB`.
Positive single-copy controls and the v0 corpus regeneration check remain
required.

This guard reduces the measured deep-copy amplification but is **not a total
memory cap**. The portable data meter still charges 32 bytes per value slot
while `EvalValue` is 96 bytes on supported 64-bit hosts, and this first slice
does not account for allocator overhead or every non-copying allocation. The
slot-cost undercharge and total-memory claim remain open under backlog
**WX-RD1** (P1); do not close the row or claim bounded process RSS from this
guard alone.

**Slices.** The S0 review also split WX-D1 into the slices of 8.1. Each
coordinator decision above is listed in the S0 pull request for owner override.

## 10. Out of scope, and why

| Item | Why |
|---|---|
| closures, lambdas, function-typed values | sections cover the pilots without making functions values; a value-level function type breaks the first-order evaluator and every lane IR |
| general recursion, recursive types | still excluded (VER04j §9); WX-D gives bounded iteration over finite values, which is what the leaf libraries need |
| stateful models (a model of a digest object with transitions) | WX-F keeps the document pure; a stateful model language needs transition relations and invariants (VER04j §9) |
| exposing a driven object's intermediate state | needs state codecs (open question 11) |
| erasure and constant-time properties of in-place writes | VER20 and the optimised-IR checks (4.12) |
| a `v2` header | open question 3 |

## 11. Specification change record

- Amendment with VER04m (2026-11-14): section 4.3 admits `inout List<R>` for an admissible
  record `R` (tuples stay inadmissible); section 4.6 adds the host forms `slice-of:struct:`,
  `vec-of:struct:` and `codec:`, and lets `error=<CASE>` serve an S2 or S4 slot from a host that
  returns the remaining components directly; section 4.12 points to VER04m. Reason: the 4004
  sequential layer (`register`, `shift_register`), `pixel-container` and the CPU `step` cannot
  be bound without them. No new diagnostic, no other rule changes.

- S1 security review (2026-11-12), recorded in 9.1 as E20: `dsize` counts the
  stored names of constructor and error values, and v1 identifiers are capped at
  128 bytes (W0602); the note on E4 records the measured read-copy hazard and
  selects a separate API-preserving host copy-footprint guard, while leaving
  the portable slot-cost undercharge and total-memory claim open under WX-RD1.
  The S1 implementation also enforces the existing §1.2 rule that a v1
  parameter or quantifier binder may not spell a reserved built-in (W0203); no
  text changed for that.

- WX-RD1 first bounded repair (2026-11-12), recorded in 9.1: a separate
  cumulative 64 MiB copy-footprint budget rejects deep copies before allocation
  in v0 and v1, with W0703 and the existing `EvaluationError` shape. The guard
  uses a fixed 96-byte host slot charge and payload lengths; it does not replace
  the portable 32-byte `VALUE_OVERHEAD` or claim a total-memory/RSS bound.

- Initial VER04k: normative WX-D (in three slices), WX-F and WX-G, replacing the
  non-normative sketches of VER04j §7. Departures from those sketches, each
  justified in the section cited: function arguments may be sections (2.3);
  count arguments are `Int` with negatives as zero (2.2); `double-finish` is an
  obligation, not a fault, and three further driver faults are added (3.5, 3.6);
  WX-D is split so its non-higher-order part precedes WX-F (8). New diagnostics
  W0711–W0713. No v0 text and no wave-3 v1 meaning changes; VER04j is not edited.
- S0 errata and decisions (2026-11-11), recorded in 9.1: open questions 2 and 4
  resolved as drafted, and question 3 partly decided (all twelve names reserved
  in WX-D1, v1 not frozen); the short-circuit fix for v1 with the v0 deviation
  recorded in VER04 §4; the evaluation-depth reading of the 64-level limit; group
  charges; `dsize` data charges for every constructing node and for deep copies;
  the two per-value caps; the list-literal cap moved to the v1 checker; the
  split-domain rules and conflict refusal; sequence-aware lists gated to `seq`;
  the list result-fault edges; the Python reference evaluator placed in WX-D2;
  operation discovery into list literals; admission inside the instance meter;
  and the WX-D1 slices of 8.1. Corrected: the quadratic-accumulator example of
  2.7 (the work meter, not the data meter, stops it, at about 1,400 elements
  rather than 11,585) and the `count` cost of 2.7 (5, not 3, units per element).
