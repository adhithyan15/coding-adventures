# VER04k — Warrant v1, Wave 4 (WX-D, WX-F, WX-G)

**Status:** Specification only. Nothing here is implemented. Implementation
starts after the WX-H/WX-E slice of [VER04j](VER04j-warrant-v1-extensions.md)
merges (branch `claude/warrant-wx-h-e`), because every adapter shape, fault
mechanism and observer this document extends lives on that branch. Each
extension below is a separately landable slice (section 8). None changes the
meaning of a v0 document, and none changes the meaning of a valid v1 document
written for the wave-3 tool (section 1).

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
| `fold map count all any take drop slice replicate range to_list to_bytes` | v1 built-in function names (WX-D) | ordinary identifiers. A call of an undeclared one is **W0202** (unbound name), exactly as for `bit_xor` today (VER04j §14 item 9 and the checker's `V1_FUNCTIONS` gating); a v0 model *named* `count` keeps meaning its own definition | reserved: a v1 model, op, binder or constructor of that name is **W0201**/**W0203** as for every other v1 built-in |
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

### 1.3 What "means the same" is tested by

1. Every v0 test of VER04j §1.3 still passes byte-identically (`ct-compare`,
   `gf256`; `tests/v0_compat.rs`).
2. **Wave-3 v1 documents regenerate byte-identically.** The generated evidence of
   `wx_fixed`, `wxa`, `wxh` and `wxv` must not change. Every new prelude part
   (the meter, sequence adapters, driver loop, inout adapter, postcondition
   monitor) is gated like the `comp`/`obs` parts of VER04j §14 items 25 and 33:
   emitted only for a document that uses the feature.
3. The checker's `Debug` output for the two checked-in v0 specifications is
   unchanged (VER04j §14 item 26).

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
| W0202 | a WX-D built-in called in a v0 document that does not define it |
| W0203 | a cycle through a function reference (2.8); an operation parameter named `result` on an operation with an `ensures` block (4.3) |
| W0301 | ordinary type errors inside WX-D calls, `ensures` rows and `inout` signatures |
| W0401 | `ensures` naming an undeclared operation |
| W0501 | a malformed `driver` line or `inout=` option, or a binding-file cap exceeded (section 6.3) |
| W0502 | a driver operation whose `faults=` omits an applicable driver fault (3.6) |
| W0601 | a driver operation that no claim or example drives with two or more non-empty elements (3.5); an `ensures` row contradicted by the operation's own example (4.7) |
| W0602 | higher-order nesting deeper than 4 (2.8), a list literal longer than 4,096 elements (2.11), more than 32 `ensures` rows (4.2), a symbolic unrolling that exceeds its caps (2.10) |
| W0701 | a fault that does not apply to the operation's shape; a lane that has no driver or inout adapter (target `not-live`) |
| W0703 | ground-evaluator exhaustion inside a WX-D form (2.7) |
| W0704 | an `inout` host form that cannot carry the parameter's Warrant type |
| W0706 | a split or hint domain over the exhaustive budget (2.9, 3.7) |

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
Open question 2 records the alternative.

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
* **Iteration, not recursion.** Every implementation must iterate. A fold is one
  model frame per element call, pushed and popped in turn; the frame depth is the
  static nesting of calls, never the length of `xs`. An implementation that
  recurses on the tail would exhaust the 64-frame limit at the 65th element and
  is non-conforming (the differential tests of 2.15 include 1,000-element folds).
* **Short-circuit.** `all` and `any` stop at the deciding element. Elements after
  it are neither evaluated nor charged. `count`, `map` and `fold` visit every
  element.
* **Sequences are values.** `take`/`drop`/`slice` produce new values. An
  implementation may share storage internally, but charges as if it copied
  (2.7), so every implementation reaches exhaustion at the same point.
* **Values.** No new value constructor. `List` and `Bytes` are the existing
  `EvalValue`/`V` variants.

### 2.7 Cost accounting

This is the design VER04j §7.2 deferred. One meter, three implementations (the
ground evaluator, the generated model code, each lane's IR evaluator), identical
charges.

**The meter.** One evaluation (an example's expected value, one claim instance
including every result conversion and postcondition check it performs, one table
row, one synthesized-suite instance) owns one meter with four counters:

| Counter | Limit (reference profile) | Source of the limit |
|---|---|---|
| work units | 1,000,000 | VER04 §2, VER04e §2.3 |
| active model frames | 64 | VER04 §2 |
| elements in any one sequence value | 65,536 (`MAX_SEQ_ELEMENTS`) | the ground evaluator's `max_value_bytes` (VER04j §14 item 18); now also the generated code's |
| owned data, cumulative | 64 MiB (`DATA_BUDGET_BYTES`) | VER04 §2 ("64 MiB of owned intermediate/output data") |

*Owned data* is measured as **cumulative allocation, never refunded**, for
determinism: live-data accounting would depend on when each implementation frees
memory. Cumulative allocation over-approximates live data, so the cap remains a
sound bound. Byte costs are those of VER04h §3: 1 byte per `Bytes` element and
`VALUE_OVERHEAD = 32` bytes per list element container (the element's own
payload was charged when that element value was produced).

**Charges** (units), in addition to the existing per-node charges of VER04e §2.3
and VER04j §2.5/§3.5/§4.5:

| Form | Work | Data, reserved before allocation |
|---|---|---|
| any WX-D call node | 1 | — |
| `fold`, `map`, `count`, `all`, `any` | per **visited** element: 1 (iteration) + 1 (the model call, as for any call) + the callee body's own charges | `map`: the result container, `len(xs)` × unit, reserved before the first element |
| section bound arguments | their own expression charges, once | — |
| `take`, `drop`, `slice` | + `L`, the result length | `L` × unit |
| `replicate(x, n)`, `range(n)` | + `max(0, n)` | `max(0, n)` × unit |
| `to_list`, `to_bytes` | + `len` | `len` × unit |

**Check before allocation.** Every form that produces a sequence knows its result
length `L` before producing it (`len(xs)` for `map`, `clamp(n)` for `take`,
`max(0, n)` for `replicate`/`range`). The implementation must, in this order:
(1) check `L <= MAX_SEQ_ELEMENTS`; (2) charge the data reservation; (3) charge
the work; (4) only then allocate. A `replicate(0u8, 10^12)` therefore fails at
step 1 having allocated nothing, and a count read from hostile data cannot make
any implementation allocate first and fail later. `n` is an exact `Int`, so the
comparison in step 1 must be done on the exact value (no narrowing of `n` to a
host integer before the check).

**Exhaustion** is never a truncated value, a skipped element, a skipped case or a
proof:

| Implementation | Exhaustion is |
|---|---|
| ground evaluator (examples, tables, generation-time checks) | **W0703** (v1), as VER04j §2.5 already uses for `Int` overflow |
| generated model code (exhaustive, property, fuzz, table and synthesized suites) | the instance **fails** with the registered failure id its suite runner uses and the note `model-budget-exhausted` (the WX-H precedent for notes: VER04j §14 item 34). A document whose claim cannot be evaluated within budget does not get a passing suite |
| a lane's IR evaluator | the same as generated model code; in practice unreachable, because the corpus generator refuses to emit any case on which the Rust evaluation exhausted (W0703 at corpus generation, section 2.13) |

**What the budgets admit** (illustrative arithmetic, not normative):

| Workload | Approximate cost | Fits 1,000,000 units? |
|---|---|---|
| `count(b, is_zero)` over 65,536 bytes, `is_zero(x) = x == 0u8` | 3 per element + 1 ≈ 196,609 | yes; the element cap, not work, is the limit |
| a fold over 65,536 elements with a body of `c` charged nodes | `65,536 × (2 + c)` | yes for `c <= 13` |
| `gmul` via `fold(range(8), ...)` | ≈ 130 per instance | yes (65,536 instances of a `U8 × U8` claim each get their own meter) |
| `leb_encode(v)` (2.1) | ≈ 10 × 25 ≈ 250 | yes |
| one SHA-256 compression (schedule + 64 rounds) | ≈ 3,500 per 64-byte block | ≈ 280 blocks ≈ 18 KiB of message; property inputs are at most 64 bytes (two blocks with padding) |

**Why both a work meter and a data meter.** The quadratic accumulator

```text
model push(acc: Bytes, x: U8) -> Bytes = concat(acc, to_bytes([x]))
model copy(b: Bytes) -> Bytes = fold(b, b"", push)
```

costs about 6 work units per element but allocates `1 + 2 + ... + n` bytes.
The data meter stops it at `n(n+1)/2 > 2^26`, that is at about **11,585**
elements, long before the work meter would (≈ 70,000 units). Without the data
meter, the same document would allocate 2 GiB at `n = 65,536`.

**Scope of the new meter in generated code.** Generated model code had no meter
in wave 3: without iteration, a model's cost was bounded by its static size. The
meter (charging every node, with the ground evaluator's charges) is emitted only
for documents that use a WX-D form (a `seq` prelude part, gated like `comp`), so
wave-3 documents regenerate byte-identically (1.3).

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
   * *Sequence-aware lists.* A `List<T>` binder that flows directly into the
     sequence argument of a WX-D form keeps its exhaustive domain (VER04j §4.7:
     lengths 0..=3) but its property generator draws the length from `0..=3`
     with probability 1/2 and uniformly from `0..=64` otherwise. The manifest
     domain text says `seq-aware`.
   * *Split domain.* A `Nat` binder `k` that flows directly as the count of
     `take`/`drop`, or as the start or length of `slice`, whose sequence argument
     is a binder `m` declared earlier, gets the **dependent** exhaustive domain
     `0..=len(m) + 1` (one past the end exercises the clamp) instead of `0..=64`.
     The case count is `Σ over m ∈ D(m) of (len(m) + 2)`; for `D(Bytes)` (347
     strings: 1 of length 0, 256 of length 1, 16 of length 2, 64 of length 3 and
     2 of each length 4..=8) that is **1,234** cases, against `347 × 65 = 22,555`
     for the undependent product. The budget is checked, with checked arithmetic
     and before enumeration, against the upper bound `|D(m)| × (maxlen + 2)`.
     The property generator draws `k` as `0` (1/4), `len(m)` (1/4), uniform in
     `0..=len(m)` (1/4), `len(m) + 1` (1/8) and uniform in `0..=64` (1/8). The
     manifest domain text says `k: split(m)`. This is a domain *choice*, honestly
     labelled; it is not a claim that larger `k` were checked.
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
  W0701 of VER04j §14 item 24 for list literals in v1 documents). A list literal
  has at most 4,096 elements (`MAX_LIST_LITERAL`; W0602 above it). The generated
  model code iterates over `V::List`/`V::Bytes` with the meter of 2.7.
* **Domains, generators, shrinkers.** As 2.9 (sequence-aware lists, the split
  domain). Shrinkers: a list shrinks toward shorter lists (drop the last element,
  then halve) before element-wise shrinking; a split-domain `k` shrinks within
  `0..=len(m)` after `m` has shrunk.
* **Fuzz decoding** of a sequence-aware list binder reads a length byte, taken
  modulo 65, then each element with its type's decoder.
* **Manifests.** Domain text gains `seq-aware` and `split(<binder>)`; nothing
  else changes.

### 2.12 Sabotage implications

WX-D adds model vocabulary; faults attack subjects, not models. Two consequences:

1. **New result faults** become meaningful because `List` results can now be
   checked element-wise: `drop-last-result-element` (the last element of a
   `List<T>` result, inside an `Ok` or `Some`, is removed) and
   `reverse-result-list` (the elements of a `List<T>` result are reversed;
   applies to results of length at least 2). They are rewrites of the composite
   adapter, like WX-E's `drop-last-result-byte` (VER04j §14 item 46), checked
   against the result type at generation (W0701).
2. **Evaluator mutants** are the WX-D analogue of VER04j §10.3's killed mutants:
   a fold that skips the last element, a fold that visits in reverse, `all`
   without short-circuit (detected by the charge comparison, not the value),
   `take` that does not clamp negative counts, `range(n)` that includes `n`, a
   recursive fold (detected by the 1,000-element case hitting the frame limit),
   and a meter that charges after allocating (detected by an allocation probe in
   the Rust evaluator). Each must make the differential suite fail.

### 2.13 Cross-language portability (VER04i)

Requirements on VER04i, which owns the corpus encoding:

1. **Model IR tags.** `["list", e...]`, `["fold", xs, init, F]`, `["map", xs, F]`,
   `["count", xs, F]`, `["all", xs, F]`, `["any", xs, F]`, `["take", xs, n]`,
   `["drop", xs, n]`, `["slice", xs, s, n]`, `["replicate", x, n]`,
   `["range", n]`, `["to_list", b]`, `["to_bytes", xs]`, where
   `F = ["fnref", "<model>", [bound...]]`. A lane evaluator rejects an unknown tag
   with a hard error (VER04i §2.5).
2. **The meter is part of the IR semantics.** Each lane evaluator implements the
   charges and caps of 2.7. Because the corpus generator evaluates every
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

The twelve names are v1 built-ins (1.2). A v0 document is unaffected. A v1
document that does not use them generates byte-identical evidence (1.3).

### 2.15 Test plan (WX-D)

* **Syntax** (Rust tests over the real parser, as VER04j §14 item 11 did; no
  corpus file): every form parses as a call; a section parses; the grammar
  files are unchanged.
* **Checker negatives** (one case each, pinning ID and span): an operation as the
  function argument (W0712); a built-in (`fold(xs, 0, len)`, W0712); a binder
  (W0712); an `if` in the function position (W0712); a section with too many and
  too few bound arguments (W0712); a step whose accumulator type is `Int` where
  the result is `Nat` (W0712, exact tail types); a model with static parameters
  (W0712); a predicate that reaches a quantifier through a helper model (W0712);
  self and mutual cycles through function references (W0203); nesting depth 5
  (W0602); `take(xs, true)` and `fold` over a `Bool` (W0301); comparing
  `to_list(b)` with `b` (W0301); a v1 model named `count` (W0201); `fold(...)` in
  a v0 document (W0202) and a v0 model named `fold` (accepted). A mutant checker
  that drops rule 5 or rule 6 of 2.3 must fail a case (VER04 §9).
* **Evaluator differential.** Three implementations (ground evaluator,
  generated model code, an independent Python reference written from this text)
  must agree on the **triple** `(value, work units, data bytes)` for: every form
  over every list of length 0..=4 over a 3-element alphabet; sequences of length
  1,000 and 65,536; counts `-2, -1, 0, len - 1, len, len + 1, 2^70`; sections with
  0 and 8 bound arguments; nesting depth 4; and the quadratic accumulator at the
  exhaustion boundary (both sides of 11,585). Native Rust iterator equivalents
  (`iter().fold`, `take`, `skip`, `all`, `any`) are a third oracle for the
  values. The mutants of 2.12 must be killed.
* **Generator.** Split-domain size 1,234 for `D(Bytes)`; first and last tuples of
  the split enumeration pinned; the dependent budget check rejects an over-budget
  product before materialising it (W0706); sequence-aware property lengths reach
  64; the two new result faults yield `not ok` under `WARRANT_EVIDENCE_FAULT`;
  wave-3 fixtures regenerate byte-identically; an instance whose claim body
  exhausts fails with `model-budget-exhausted` and never passes.
* **Symbolic (WX-D3).** The fold `gmul` against the gf256 export's lowered WIR
  at `proved` (static lengths, fixed layout); `leb_encode` labelled `unknown`
  (data-dependent accumulator length); a fixed-width fold over `m: Bytes` at
  `bounded(16)`, never `proved`; a symbolic count (`take(m, to_int(x))`)
  labelled `unknown`; each cap exceeded once, each giving `unknown` with the cap
  recorded.
* **Conversion pilot.** `wasm-leb128`: the `encode_matches_model` claim of 2.1
  over the full `U64` edge domain (377 values), plus `decode_unsigned` round trips
  (WX-A), with the `off-by-one-consumed` control of VER04j §4.8 failing. Second
  pilot: the gf256 document's `gmul` rewritten as the fold of 2.1, every previous
  verdict kept.

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
  with a literal `N`, `List<T>` with `T` one of those, and records whose fields
  are all admissible. Anything else is W0713 (an `Option` cannot be "written into";
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

Line form: `op zeroize export=zeroize_bytes inout=buf:slice faults=stale-post,skip-last-write`.
JSON form: `"inout": { "buf": "slice" }`. For an operation with an `inout`
parameter, `export=` may be a Rust path of at most 8 segments relative to the
crate root (`varint::write`, `Zeroize::zeroize`), called in fully qualified
form (`coding_adventures_zeroize::Zeroize::zeroize(&mut host)`), which reaches
inherent and trait methods without imports; the arguments are passed in the
operation's parameter order. The `ret=` and `errors=` options describe the
*host* return of the slot table (4.2), never the post-states. A missing host form,
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
  objects but does not expose their state.

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
| model frames | 64 | every evaluator | each call | same |
| elements per sequence value | 65,536 | every evaluator | allocation (2.7 step 1) | same |
| owned data per evaluation, cumulative | 64 MiB | every evaluator | allocation | same |
| list literal | 4,096 elements | checker | generation | W0602 |
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
   gap VER04j §14 item 12 recorded for wave 1).

## 8. Recommended implementation order

Precondition: the WX-H/WX-E branch (`claude/warrant-wx-h-e`) has merged.

| Step | Slice | Contents | Size (engineer-days) | Depends on |
|---|---|---|---|---|
| 1 | **WX-D1** sequence core | `take drop slice replicate range to_list to_bytes`; list literals in evidence bodies; the meter in generated code (gated); sequence-aware lists; the split domain; the two result faults | 4–6 | — |
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

## 9. Open questions for the owner

1. **Sections.** Function arguments may bind leading arguments (`gmul_step(b)`),
   defunctionalised, not closures (2.3). The VER04j sketch allowed only bare
   names, which forces context through the accumulator and makes `map` with
   context impossible. Confirm sections.
2. **Count arguments.** `take`/`drop`/`slice`/`replicate`/`range` take `Int` and
   treat negatives as zero (2.2). The alternative is `Nat` counts plus a typing
   change `Nat + Nat : Nat` (which would also close VER04j §14 item 5 for shift
   counts) or a `to_nat` on `Int`. Choose.
3. **v1 stability.** This wave reserves twelve more names in v1. Freeze v1 after
   this wave (any later name needs a `v2` header), or keep v1 as an evolving
   profile with a mechanical scan of in-repository v1 documents?
4. **Data accounting.** Cumulative allocation (deterministic, over-approximate)
   rather than peak live data (2.7). Confirm.
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

- Initial VER04k: normative WX-D (in three slices), WX-F and WX-G, replacing the
  non-normative sketches of VER04j §7. Departures from those sketches, each
  justified in the section cited: function arguments may be sections (2.3);
  count arguments are `Int` with negatives as zero (2.2); `double-finish` is an
  obligation, not a fault, and three further driver faults are added (3.5, 3.6);
  WX-D is split so its non-higher-order part precedes WX-F (8). New diagnostics
  W0711–W0713. No v0 text and no wave-3 v1 meaning changes; VER04j is not edited.
