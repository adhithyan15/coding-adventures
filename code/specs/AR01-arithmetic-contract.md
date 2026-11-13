# AR01 — Adders and the ALU: the arithmetic contract

## Overview

An adder takes bits and returns bits: a half adder adds two bits, a full adder
adds two bits and a carry, a ripple-carry adder chains N full adders, and the ALU
selects one of six operations over two N-bit operands and reports four flags.
This spec is the contract of the Rust package
`code/packages/rust/arithmetic` and the prose companion of its Warrant document
[`arithmetic.warrant`](arithmetic.warrant), which is the executable
specification: every example and claim there is compiled into a test harness and
run against the real exports (VER04h).

This is **layer 2 of the Intel 4004 verification chain** (VER27 section 4.15,
backlog LV-006): "4-bit result and carry = `(a+b+c) mod 16`, 512 cases,
exhaustive", extended to the signed-overflow flag, to every ALU operation and to
8 and 16 bits, so that the layers above (`intel4004-gatelevel` and the other
gate-level CPUs) can assume them by name (VER27 section 4.13) instead of
re-proving them.

In scope (5 exported operations, 3 result types):

| Operation | Function |
|---|---|
| `half_adder(a, b)` | `(a xor b, a and b)` |
| `full_adder(a, b, cin)` | `(a + b + cin) mod 2` and `(a + b + cin) div 2` |
| `ripple_carry_adder(a, b)` | the N-bit sum `(a + b) mod 2^N`, the carry out, signed overflow |
| `ripple_carry_adder_with_carry(a, b, cin)` | the same for `a + b + cin` |
| `alu(a, b, op)` | `op` in `Add`, `Sub`, `And`, `Or`, `Xor`, `Not`: the result bits and the flags zero, carry, negative, overflow |

The circuit diagrams, the pedagogy and the long-form design stay in
[09-arithmetic.md](09-arithmetic.md); this document states only what is *checked*.

## Layer position

`arithmetic` depends on `logic-gates` (every bit passes through an `xor_gate`,
`and_gate`, `or_gate` or `not_gate`), which depends on `transistors`. Above it sit
`intel4004-gatelevel` and the other gate-level CPUs, `cpu-simulator`, `gpu-core`
and `compute-unit`. The VER13 catalogue entry `gate.primitives.correct` covers
the Boolean functions; this layer adds the adder and ALU functions, which have
no catalogue entry yet (LV-005 owns linking; no catalogue entry is edited here).

## Domain

### Bit representation

A wire is a `u8` whose value is **0 or 1**, as in LG01; a bit vector is a
`&[u8]`, **least significant bit first** (element 0 is bit 0, so
`ripple_carry_adder(&[1, 0, 1, 0], ..)` reads the number 5).

> **Rejected by a panic, in every build profile.** A value other than 0 or 1,
> in any position of any operand and in the carry-in, is never masked, never
> passed through and never "read as" 0 or 1.

This crate does not validate bits itself. Every bit passes through a
`logic-gates` gate, which rejects a non-bit with `assert!` (finding SEC-007 of
that crate: before it, the guard was a `debug_assert!` and a release build
accepted the value). So **this layer's rejection is inherited from the layer
below**, and that is the reason it is pinned here anyway: `tests/bit_domain.rs`
states, operation by operation and position by position, what must happen, and
BUILD runs it in the release profile too, so a regression of the gates' guard
fails this crate's build, not only the gates'. The Warrant claims
`*_rejects_non_bit_*` carry the same rule; the generated harness also runs in
release.

### Shape

The two operands of `ripple_carry_adder`, `ripple_carry_adder_with_carry` and of
every `alu` operation except `Not` have the **same length**, and `a` has length
**at least 1**. These are `assert!`, panics in every profile. A panic is not a
Warrant error case, so the binding (`evidence.bindings`, `error=INVALID_INPUT`)
normalises it as gf256 and logic-gates do: the generated harness contains the
panic and reads it as `Err(INVALID_INPUT)`. Consequences, stated so nobody reads
more into it:

* the single case means "this call panicked"; it cannot tell a non-bit from a
  bad length;
* a rejection claim passes only when the crate really panics, and a spurious
  panic on a valid input fails the claim that expected `Ok`;
* the `rejects` rows of the Warrant file state the shape rules only (a byte
  string's content cannot be a predicate without indexing); the content rule is
  carried by the `*_rejects_non_bit_*` claims, which sweep the eight positions
  of an 8-bit operand.

### `AluOp::Not` ignores `b`

`Not` is unary: `b` is **never read**. Any `b` (any content, any length, even
empty) is accepted and ignored, and the result is a function of `a` alone. This
is the documented behaviour of the crate (its docs say "B is ignored"), kept
because the dependents pass an empty or zero-filled `b`, and recorded here as a
decision, not a finding: a hostile `b` can neither crash nor influence `Not`.
It is pinned by `alu_not_ignores_b`, by the exemption in the Warrant `input
alu.b` row and by `tests/bit_domain.rs`.

### Width

There is no width-driven allocation. The sum or result is exactly as long as
`a`, and the work is a constant number of passes over the operands, so a
caller's length is not an amplification factor in the sense of SEC-008: the
caller already holds an allocation of that size and the output is the same
size. (A 100,000-bit addition is exercised in `tests/bit_domain.rs`.) There is
therefore no width cap, unlike `MAX_DECODER_WIDTH` in `logic-gates`.

## The operations

| Operation | Function | Panics (in every profile) |
|---|---|---|
| `half_adder(a, b)` | `(a ^ b, a & b)` | either input `> 1` |
| `full_adder(a, b, cin)` | `sum = a ^ b ^ cin`, `carry = majority(a, b, cin)` | any input `> 1` |
| `ripple_carry_adder(a, b)` | `sum = (a + b) mod 2^N`, `carry_out = (a + b) >= 2^N`, `overflow` | unequal lengths, empty, any bit `> 1` |
| `ripple_carry_adder_with_carry(a, b, cin)` | the same with `a + b + cin` | as above, and `cin > 1` |
| `alu(a, b, Add)` | `(a + b) mod 2^N`; carry as above | `a` empty, unequal lengths, any bit `> 1` |
| `alu(a, b, Sub)` | `(a - b) mod 2^N`; carry = `a >= b` (no borrow) | as `Add` |
| `alu(a, b, And / Or / Xor)` | the bitwise operation; carry = overflow = false | as `Add` |
| `alu(a, b, Not)` | `~a` over N bits; `b` ignored; carry = overflow = false | `a` empty, any bit of `a` `> 1` |

The flags, for an N-bit result `r`: **zero** is "every bit of `r` is 0";
**negative** is the top bit of `r`; **carry** is, for `Add`, the carry out of
bit N-1 (the unsigned sum is at least `2^N`) and, for `Sub`, "no borrow is
needed" (`a >= b` unsigned), the carry of `A + NOT(B) + 1` (a convention, the
one ARM uses; x86 reports the borrow, the complement); **overflow** is "the
two's-complement sum or difference does not fit in N bits". The oracle states
the subtraction as `a - b` and the overflow as a range check on the signed
value, not as the crate's `a + NOT(b) + 1` construction, so the construction
is what is under test.

## Rust contract

* The operations and the result types are exported from the crate root
  (`arithmetic::alu`, `arithmetic::AluOp`) as well as from `adders` and `alu`:
  the Warrant binding resolves an `export=` by name at the root. Same items, so
  no existing call site changes (about 45 `Cargo.toml` files name this package).
* Every function is a pure function of its arguments. None reads the clock, the
  environment, a file or a random source (the runtime-capability gate below
  checks this at run time).
* A bit is a `u8`; a bit vector is `&[u8]`; `half_adder` and `full_adder` return
  `(u8, u8)` (sum, carry); `ripple_carry_adder*` return `RippleCarryResult
  { sum: Vec<u8>, carry_out: u8, overflow: bool }`; `alu` returns `AluResult
  { result: Vec<u8>, zero, carry, negative, overflow: bool }`.

## Findings

The domain checks of VER27 section 7.4 were made first (non-bit inputs at every
position, unequal and empty operands, a hostile `b` for `Not`, width-driven
allocation, release-profile behaviour; `tests/bit_domain.rs`, run in debug and
release). **They found no defect in this crate**, so there is no SEC row and no
failing-then-fixed test (the one lesson recorded is about the inherited
rejection, below): every non-bit is rejected
in both profiles because `logic-gates` already rejects it (its SEC-007), and
no operation sizes anything from a caller's width. The exhaustive and sampled
claims also found no disagreement between the crate and the integer
definitions: the ALU's flags, the subtraction's carry convention and the signed
overflow all match the mathematical oracle on every enumerated input.

What was found are two properties worth stating:

* **The rejection is inherited, not owned.** A change to a gate's guard
  changes this layer's behaviour without touching its source. Pinned in release
  (above); the contract assumed from `logic-gates` (below) names the guard.
* **Proof-front-end refusals as design findings (VER27 section 7.4, P11).** The
  Rust-subset front end refuses both files (recorded in
  `evidence/proof-manifest.json`): `adders.rs` at the `(u8, u8)` tuple return type
  of `half_adder` and `full_adder`, and every export that destructures it
  (`let (s, c) = full_adder(..)`) at the tuple pattern; `alu.rs` at the `enum AluOp`
  item. None of these is a defect of the crate; a small `Sum { sum, carry }`
  struct would shape the adders for the prover (backlog AR-001).

## Contracts this layer publishes (pending LV-005)

Layer-contract linking (VER27 section 4.14, VER29, backlog LV-005) is not built.
These are the stable names under which a later layer will link to what this layer
proved. **A claim name is a published identifier: renaming one breaks every upper
layer's assumption**, so a rename needs a deprecation note here. Each claim is
verified by an exhaustive suite (every input of the stated domain, or, for a
`_sampled` claim, the stated boundary-rich domain), a property suite (4,096 cases)
and a fuzz suite (5,000 iterations). The case count is the exhaustive suite's
`cases` in `evidence/test-manifest.json` (a claim with a refinement counts only
the cases the refinement admits).

The claims fall in five families:

* `half_adder_*`, `full_adder_*`: the Boolean definition (`xor`/`and`/majority) and,
  independently, the arithmetic one (`sum + 2 carry = a + b + cin`), and rejection.
* `ripple_carry_adder[_with_carry]_<N>_bit_exhaustive`: the result, carry out and
  signed overflow for **every** input at N = 1, 2, 3, 4 and 8 (the 4-bit claim with
  a carry-in is the 512 cases of VER27 section 4.15); `_16_bit_sampled`: **sampled**,
  the 377-value boundary domain squared, widths above 16 are not covered.
* `alu_<op>_<N>_bit_exhaustive` (Add and Sub at N = 1, 2, 3, 4, 8; the others at 4
  and 8) and `alu_<op>_16_bit_sampled`: each operation against its arithmetic or
  bitwise definition with all four flags.
* `*_rejects_*`: the shapes (unequal lengths, an empty `a`) and the content (a
  non-bit at each of eight positions of `a`, `b` and the carry-in).
* `ripple_carry_adder_equals_with_carry_zero`, `alu_not_ignores_b`: relations.

| Claim | Operation | Exhaustive cases | Domain |
|---|---|---|---|
| `half_adder_truth_table` | `half_adder` | 4 | 2 Bool binders |
| `half_adder_counts_ones` | `half_adder` | 4 | 2 Bool binders |
| `half_adder_rejects_non_bit` | `half_adder` | 65,532 | a: U8 full(256); b: U8 full(256) |
| `full_adder_truth_table` | `full_adder` | 8 | 3 Bool binders |
| `full_adder_counts_ones` | `full_adder` | 8 | 3 Bool binders |
| `full_adder_rejects_non_bit_a` | `full_adder` | 1,024 | j: U8 full(256); 2 Bool binders |
| `full_adder_rejects_non_bit_b` | `full_adder` | 1,024 | j: U8 full(256); 2 Bool binders |
| `full_adder_rejects_non_bit_carry_in` | `full_adder` | 1,024 | j: U8 full(256); 2 Bool binders |
| `ripple_carry_adder_1_bit_exhaustive` | `ripple_carry_adder` | 4 | 2 Bool binders |
| `ripple_carry_adder_with_carry_1_bit_exhaustive` | `ripple_carry_adder_with_carry` | 8 | 3 Bool binders |
| `ripple_carry_adder_2_bit_exhaustive` | `ripple_carry_adder` | 16 | 4 Bool binders |
| `ripple_carry_adder_with_carry_2_bit_exhaustive` | `ripple_carry_adder_with_carry` | 32 | 5 Bool binders |
| `ripple_carry_adder_3_bit_exhaustive` | `ripple_carry_adder` | 64 | 6 Bool binders |
| `ripple_carry_adder_with_carry_3_bit_exhaustive` | `ripple_carry_adder_with_carry` | 128 | 7 Bool binders |
| `ripple_carry_adder_4_bit_exhaustive` | `ripple_carry_adder` | 256 | 8 Bool binders |
| `ripple_carry_adder_with_carry_4_bit_exhaustive` | `ripple_carry_adder_with_carry` | 512 | 9 Bool binders |
| `ripple_carry_adder_8_bit_exhaustive` | `ripple_carry_adder` | 65,536 | a: U8 full(256); b: U8 full(256) |
| `ripple_carry_adder_with_carry_8_bit_exhaustive` | `ripple_carry_adder_with_carry` | 131,072 | a: U8 full(256); b: U8 full(256); 1 Bool |
| `ripple_carry_adder_equals_with_carry_zero` | `ripple_carry_adder` | 65,536 | a: U8 full(256); b: U8 full(256) |
| `ripple_carry_adder_16_bit_sampled` | `ripple_carry_adder` | 142,129 | a: U64 edge(377); b: U64 edge(377) |
| `ripple_carry_adder_with_carry_16_bit_sampled` | `ripple_carry_adder_with_carry` | 284,258 | a: U64 edge(377); b: U64 edge(377); 1 Bool |
| `ripple_carry_adder_rejects_unequal_lengths` | `ripple_carry_adder` | 50,500 | a: Bytes set(347); b: Bytes set(347) |
| `ripple_carry_adder_rejects_an_empty_a` | `ripple_carry_adder` | 347 | b: Bytes set(347) |
| `ripple_carry_adder_rejects_non_bit_in_a` | `ripple_carry_adder` | 16,640 | j: U8 full(256); k: Nat set(65) |
| `ripple_carry_adder_rejects_non_bit_in_b` | `ripple_carry_adder` | 16,640 | j: U8 full(256); k: Nat set(65) |
| `ripple_carry_adder_with_carry_rejects_unequal_lengths` | `ripple_carry_adder_with_carry` | 101,000 | a: Bytes set(347); b: Bytes set(347); 1 Bool |
| `ripple_carry_adder_with_carry_rejects_an_empty_a` | `ripple_carry_adder_with_carry` | 694 | b: Bytes set(347); 1 Bool |
| `ripple_carry_adder_with_carry_rejects_non_bit_in_a` | `ripple_carry_adder_with_carry` | 33,280 | j: U8 full(256); k: Nat set(65); 1 Bool |
| `ripple_carry_adder_with_carry_rejects_non_bit_in_b` | `ripple_carry_adder_with_carry` | 33,280 | j: U8 full(256); k: Nat set(65); 1 Bool |
| `ripple_carry_adder_with_carry_rejects_non_bit_carry_in` | `ripple_carry_adder_with_carry` | 65,536 | j: U8 full(256); a: U8 full(256) |
| `alu_add_1_bit_exhaustive` | `alu` | 4 | 2 Bool binders |
| `alu_add_2_bit_exhaustive` | `alu` | 16 | 4 Bool binders |
| `alu_add_3_bit_exhaustive` | `alu` | 64 | 6 Bool binders |
| `alu_add_4_bit_exhaustive` | `alu` | 256 | 8 Bool binders |
| `alu_add_8_bit_exhaustive` | `alu` | 65,536 | a: U8 full(256); b: U8 full(256) |
| `alu_sub_1_bit_exhaustive` | `alu` | 4 | 2 Bool binders |
| `alu_sub_2_bit_exhaustive` | `alu` | 16 | 4 Bool binders |
| `alu_sub_3_bit_exhaustive` | `alu` | 64 | 6 Bool binders |
| `alu_sub_4_bit_exhaustive` | `alu` | 256 | 8 Bool binders |
| `alu_sub_8_bit_exhaustive` | `alu` | 65,536 | a: U8 full(256); b: U8 full(256) |
| `alu_and_4_bit_exhaustive` | `alu` | 256 | 8 Bool binders |
| `alu_and_8_bit_exhaustive` | `alu` | 65,536 | a: U8 full(256); b: U8 full(256) |
| `alu_or_4_bit_exhaustive` | `alu` | 256 | 8 Bool binders |
| `alu_or_8_bit_exhaustive` | `alu` | 65,536 | a: U8 full(256); b: U8 full(256) |
| `alu_xor_4_bit_exhaustive` | `alu` | 256 | 8 Bool binders |
| `alu_xor_8_bit_exhaustive` | `alu` | 65,536 | a: U8 full(256); b: U8 full(256) |
| `alu_not_4_bit_exhaustive` | `alu` | 16 | 4 Bool binders |
| `alu_not_8_bit_exhaustive` | `alu` | 256 | a: U8 full(256) |
| `alu_not_ignores_b` | `alu` | 88,832 | a: U8 full(256); b: Bytes set(347) |
| `alu_add_16_bit_sampled` | `alu` | 142,129 | a: U64 edge(377); b: U64 edge(377) |
| `alu_sub_16_bit_sampled` | `alu` | 142,129 | a: U64 edge(377); b: U64 edge(377) |
| `alu_and_16_bit_sampled` | `alu` | 142,129 | a: U64 edge(377); b: U64 edge(377) |
| `alu_or_16_bit_sampled` | `alu` | 142,129 | a: U64 edge(377); b: U64 edge(377) |
| `alu_xor_16_bit_sampled` | `alu` | 142,129 | a: U64 edge(377); b: U64 edge(377) |
| `alu_not_16_bit_sampled` | `alu` | 142,129 | a: U64 edge(377); b: U64 edge(377) |
| `alu_rejects_empty_a` | `alu` | 22,555 | b: Bytes set(347); i: Nat set(65) |
| `alu_add_rejects_unequal_lengths` | `alu` | 50,500 | a: Bytes set(347); b: Bytes set(347) |
| `alu_sub_rejects_unequal_lengths` | `alu` | 50,500 | a: Bytes set(347); b: Bytes set(347) |
| `alu_and_rejects_unequal_lengths` | `alu` | 50,500 | a: Bytes set(347); b: Bytes set(347) |
| `alu_or_rejects_unequal_lengths` | `alu` | 50,500 | a: Bytes set(347); b: Bytes set(347) |
| `alu_xor_rejects_unequal_lengths` | `alu` | 50,500 | a: Bytes set(347); b: Bytes set(347) |
| `alu_rejects_non_bit_in_a` | `alu` | 16,640 | j: U8 full(256); k: Nat set(65) |
| `alu_rejects_non_bit_in_b` | `alu` | 16,640 | j: U8 full(256); k: Nat set(65) |

Total: **2,655,437 exhaustive cases over 63 claims** (559,276 more were discarded
by a refinement, all in the unequal-length claims), 53 examples, 52 negative
controls.

A claim's formula is in the Warrant file under the same name; the text of an
upper layer's assumption must be that formula, not a paraphrase (VER27 rule (c)).

## Contracts this layer would assume from logic-gates (pending LV-005)

Nothing is declared in `assume platform:` yet: VER29's `contract_id` parsing is
not implemented, and a Warrant platform row today must resolve to the VER13
catalogue. The assumptions are listed here, and as a TODO comment in
`arithmetic.warrant`, so the day the mechanism lands the upper rows can be
written, not rediscovered. Each is a published claim of
[LG01](LG01-logic-gates-contract.md) (rule (c): by id, exactly):

| Would-be assumption | Used for |
|---|---|
| `contract.logic-gates.xor_truth_table` | the sum bit of the half and full adders |
| `contract.logic-gates.and_truth_table` | the carry of the half adder, and `And` |
| `contract.logic-gates.or_truth_table` | the carry-out of the full adder, and `Or` |
| `contract.logic-gates.not_truth_table` | operand inversion in `Sub`, and `Not` |
| `contract.logic-gates.xor_rejects_non_bit`, `and_rejects_non_bit`, `or_rejects_non_bit`, `not_rejects_non_bit` | this layer's non-bit rejection (the inherited guard) |

Rule (b) (no reaching around): this crate uses `logic-gates` only through those
four gates (`xor_gate`, `and_gate`, `or_gate`, `not_gate`); it calls no helper
outside the contract. **VER29 H3** (an `exhaustive` label needs a stateless,
deterministic binding): every operation is a pure function of its arguments, so
the label applies as is, but the binding format has no way to say so yet (the
`state none` key of VER29 section 6.2 is a later producer change), so none is
declared. When it lands it goes on every `op` row of `evidence.bindings`.

### What the claims do not cover (rule (a), coverage)

* **Time.** These are functional contracts: result as a function of operands.
  They say nothing about delay, so no constant-time or timing claim can rest on
  them alone (a ripple-carry adder's delay is linear in its width, which is the
  textbook reason carry-lookahead exists, and nothing here measures it). A
  timing contract is a separate row for the day a chain needs it. No operand is
  secret, so the report records CWE-208 as not applicable rather than as proved.
* **Widths past what is enumerated.** Exhaustive to 4 and 8 bits (adders also to
  1..3), sampled at 16, nothing about 17 and above. The honest sentence is
  "exhaustive over these domains"; a 12-bit or 32-bit adder is *not* covered by
  the claims, only by the carry-chain structure being the same loop.
* **The ALU's sensitivity to a flag fault.** The generator's faults need a result
  shape (a numeric field, a bytes result) the ALU's record does not have, so the
  ALU's only negative control is `missing-error`. A deliberately broken flag is
  not among the 52 controls; the claims catch it because they enumerate every
  operand pair, not because a control demonstrates it.
* **Hostile byte-string content beyond the swept positions.** A non-bit is
  rejected at each of eight positions of an 8-bit operand; the random suites
  draw arbitrary hostile bytes elsewhere.

## Trust boundary

What the proof by enumeration assumes (VER27 section 4.15, "Trust boundary"):

* **The Rust host and toolchain.** The evidence runs the compiled crate. That
  `u8` arithmetic, slices, `Vec` and `catch_unwind` do what the language says,
  and that the compiler preserves the semantics, is assumed.
* **`logic-gates` and `transistors` are part of the code under test, not
  oracles.** Every bit goes through them, and the expected values here come from
  Warrant's integers and bit operators, not from them. A claim therefore says
  "the adders, as compiled with those gates, compute these functions"; it does
  **not** say the CMOS model matches a physical transistor, and it does not
  re-prove a gate: that is LG01's job, which this layer assumes.
* **The datasheet.** Binary addition and two's complement are the standard
  definitions, not transcribed from a datasheet. (The 4004's own ADD/SUB
  semantics, including how it uses the carry on a subtraction, enter at the CPU layer.)
* **Warrant itself** and its evidence generator, as for every adopter.
* **Nothing about silicon**, the L0 floor entries stay printed.

"Exhaustive" is proof by enumeration of the Rust function over a finite, stated
domain. It is not a proof about the source in the Rust-subset sense: the
step-bound proofs are `not-live` (see below).

## Cross-language expectations

The Warrant file is language-neutral except for the binding. The same truth
tables, flags and shape rules are expected of the other `arithmetic` ports
(Python, TypeScript, Go and the rest); `arithmetic.warrant` is the specimen a
polyglot binding (VER04i) would re-run. The panic-as-rejection rule is Rust's;
a port in a language without panics should raise its idiomatic error, and the
single `INVALID_INPUT` case covers it. No other port has been checked against
this contract.

## Verification status (Warrant-derived)

1. **Examples and claims.** 53 examples (the half and full adder tables as a
   datasheet prints them, worked 4- and 8-bit sums with their flags, rejections)
   and 63 claims, each with an exhaustive, a property and a fuzz suite.
2. **Exhaustive means exhaustive.** 2,655,437 cases in the debug profile, 0
   unreachable by type. The largest are `ripple_carry_adder_with_carry_16_bit_sampled`
   (284,258, sampled) and `ripple_carry_adder_with_carry_8_bit_exhaustive`
   (131,072: every 8-bit pair and carry-in).
3. **Negative controls.** 52 (13 operation/fault pairs x example, exhaustive,
   property, fuzz), each catching its fault by a named claim; `BUILD` runs the 13
   pairs as sabotage lines, each printing a `not ok`. The pairs are the limits
   stated above: the half and full adders are attacked by `missing-error`,
   `drop-consumed`, `off-by-one-consumed` and `swap-components`, the ripple-carry
   adders by `missing-error` and `field-zero`, the ALU by `missing-error`.
4. **Release profile.** The whole generated harness and `tests/bit_domain.rs`
   also run under `--release`.
5. **Wall clock** on the authoring machine: about 2.5 minutes for the generated
   target in the debug profile (the 16-bit sampled claims and the 8-bit
   enumerations, each case 16 or 8 full adders over the CMOS model, dominate).
6. **Heap** is not bounded (no `heap` row).
7. **Steps and depth** rows are declarations; the proof manifest records every
   steps-bound obligation `not-live` with the front end's own refusal, and depth
   `unchecked`. The refusal is a design finding (P11), not a defect of the crate;
   `warrant-ir/tests/arithmetic_proofs.rs` records the construct per function.
8. **CWE-208 does not apply:** every operand is `public`. The report records it as
   the unapproved exemption `EXEMPT-CWE-208-NO-SECRET-OPERANDS`, backed by the
   derived attestation `warrant-spec::all-operands-public`, not as a
   constant-time claim.

### Runtime capability evidence (VER08b)

`required_capabilities.json` declares `"capabilities": []`; the static analyzer
agrees (`effect_certificate.json`). `tests/runtime_capability.rs` adds the
run-time half: phase A builds the operands unconstrained, phase B arms a
seccomp-bpf + Landlock jail derived mechanically from the manifest (minimal
baseline: allocator, `write` of TAP lines, `exit_group`) and calls every public
operation: the half and full adders over every input, both ripple-carry adders
over every 4-bit input and carry-in and the boundary values at 8 and 16 bits, and
the ALU with all six operations over every 4-bit pair (all four flags) and a
stride of the 8-bit space, against Rust's own integer and bitwise operators.
Only valid inputs are used in the jail (a rejection panics and prints to stderr,
which the baseline forbids); rejections are pinned unjailed. 6 named checks pass.
Live here: enforcement pass (Landlock ABI 7, seccomp), observation pass (strace:
8 `write`, 1 `exit_group`, 1 `munmap`, 0 violations), 18 negative controls each
caught by both rings. The container ring is `NOT-LIVE` where no Docker daemon is
reachable. `BUILD_linux` runs the gate with `RCG_STRICT=1`; the plain `BUILD`
(macOS) does not; `BUILD_windows` does not run it. Known residual: vDSO time
reads (CAP-RT-006). The jail's scope is this package's capabilities and those of
its dependency closure (`logic-gates`, `transistors`), all pure.

The assurance report (`--profile bit-arithmetic`, a new profile: the
`boolean-logic` sentences say "single-bit values only" and bound a 2^n decoder,
both false for an adder over a 16-bit operand, and the `varint-codec` ones name
LEB128) is `report-only`: rungs E0, E1, E3 and E4 `pass`, E2 and E5 `not-live`;
CWE-208 is the not-applicable exemption above and CWE-400 the unapproved "no
step-bound proof is live" exemption. The profile changes wording only, never
the verdict logic.

## Next

`intel4004-gatelevel` (the CPU step) and the other gate-level CPUs are the layers
that consume this contract; they need LV-005 to link by id. How a given CPU maps
this crate's flags onto its own (for the subtraction carry, borrow or no borrow)
is that CPU layer's claim, not this one's.
