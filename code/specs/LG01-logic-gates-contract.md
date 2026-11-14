# LG01 — Logic Gates and Combinational Blocks: the stateless contract

## Overview

A logic gate takes one or two bits and returns one bit; a combinational block
(a multiplexer, a decoder, an encoder) is a fixed arrangement of gates whose
output depends on its inputs and on nothing else: no memory, no clock, no
history. This spec is the contract of the **stateless** half of the Rust
package `code/packages/rust/logic-gates` and the prose companion of its Warrant
document [`logic-gates.warrant`](logic-gates.warrant), which is the executable
specification: every example and claim there is compiled into a test harness
and run against the real exports (VER04h).

This is **layer 1 of the Intel 4004 verification chain** (VER27 section 4.15,
backlog LV-006): "each gate's truth table" and "block function", proved by
enumeration, so that every layer above can assume them by name (VER27
section 4.13) instead of re-proving them.

In scope (22 operations):

| Group | Operations |
|---|---|
| Primitive gates | `not_gate`, `and_gate`, `or_gate`, `xor_gate`, `nand_gate`, `nor_gate`, `xnor_gate` |
| NAND-built gates | `nand_not`, `nand_and`, `nand_or`, `nand_xor` (functional completeness of NAND) |
| n-input gates | `and_n`, `or_n`, `xor_n` |
| Combinational blocks | `mux2`, `mux4`, `mux_n`, `demux`, `decoder`, `encoder`, `priority_encoder`, `tri_state` |

**Out of scope: `sequential.rs`** (SR latch, D latch, D flip-flop, register,
shift register, counter). Those carry state from one call to the next, so a
claim about them is a claim about a sequence of calls. See "Next slice".

The detailed gate semantics, the transistor-level construction and the
pedagogy stay in [10-logic-gates.md](10-logic-gates.md); this document states
only what is *checked*.

## Layer position

`logic-gates` depends on `transistors` (each gate delegates to a CMOS model).
`arithmetic` (adders, ALU), `intel4004-gatelevel`, `riscv-gatelevel` and the
other gate-level simulators sit above it. The VER13 catalogue entry
`gate.primitives.correct` ("`logic-gates`/`arithmetic` primitives implement
the Boolean functions") is the assumption this layer gives evidence for, of
kind `exhaustive-truth-table`. The catalogue entry is not edited here (LV-005
owns linking); the transistor-model-equivalence half of its discharge plan is
not done (see "Trust boundary").

## Bit representation

A wire is a `u8` whose value is **0 or 1**. The crate uses no `bool` and no
newtype. The domain of every operation is therefore a strict subset of what
the type allows, and the contract says what happens outside it:

> **Rejected by a panic, in every build profile.** A value other than 0 or 1
> is never masked (`a & 1`), never passed through, and never "read as" 0 or 1.

The same rule covers shapes: a slice whose length the operation cannot accept
(`mux4` select of one bit, `mux_n` of three inputs, `encoder` of a non-power of
two, `and_n` of one input) and an `encoder` input that is not one-hot are
rejected the same way. A panic is not a Warrant error case, so the binding
(`evidence.bindings`, `error=INVALID_INPUT`) normalises it, as `gf256` does for
a division by zero: the generated harness contains the panic and reads it as
`Err(INVALID_INPUT)`. Consequences, stated so nobody reads more into it:

* the single case means "this call panicked"; it cannot tell a non-bit from a
  bad length;
* a rejection claim passes only when the crate really panics, and a spurious
  panic on a valid input fails the claim that expected `Ok`;
* the `rejects` rows of the Warrant file state the *length* rules only. A byte
  string's content (every byte 0 or 1) cannot be a predicate without indexing,
  so the content rule is carried by the `*_rejects_non_bit_*` claims;
* the panic itself, in the debug and the release profile, is pinned by
  `tests/bit_domain.rs` and `tests/decoder_width.rs`, and the whole generated
  harness is also run in release (BUILD).

"Reject by panic" is a design choice, recorded as such: a hardware-simulator
primitive called with a non-bit has no sensible result, and a `Result` on
every gate would change every dependent crate (about 47 `Cargo.toml` files name this package) for no caller that wants to
recover. The alternative (a `Result` or a masking API) is not excluded for a
later release; the claims would then state the new rows.

### Width bound

`decoder` and `demux` have **at most `MAX_DECODER_WIDTH` = 16** inputs / select
lines: a decoder with n inputs has 2^n lines, so n is an amplification factor.
More than 16 panics before any allocation (finding SEC-008).

## The operations

Bits are written least significant first: `decoder(&[1, 0])` selects line 1,
`mux4`'s `sel[0]` is the low select bit, `encoder` returns the index of its
active input LSB first.

| Operation | Function | Panics (in every profile) |
|---|---|---|
| `not_gate(a)` | `1 - a` | `a > 1` |
| `and_gate(a, b)`, `or_gate`, `xor_gate` | `a & b`, `a \| b`, `a ^ b` | either input `> 1` |
| `nand_gate`, `nor_gate`, `xnor_gate` | the complements | either input `> 1` |
| `nand_not`, `nand_and`, `nand_or`, `nand_xor` | the same functions built only from `nand_gate` | as above |
| `and_n(xs)`, `or_n`, `xor_n` | all ones, any one, odd parity | fewer than 2 inputs, any element `> 1` |
| `mux2(d0, d1, sel)` | `d0` if `sel == 0` else `d1` | any argument `> 1` |
| `mux4(d0, d1, d2, d3, sel)` | `d[sel[0] + 2 sel[1]]` | any bit `> 1`, `sel.len() != 2` |
| `mux_n(xs, sel)` | `xs[sel as an LSB-first number]` | `xs.len()` not a power of 2 `>= 2`, `sel.len() != log2(xs.len())`, any bit `> 1` |
| `demux(d, sel, n)` | `n` lines, all 0 but line `sel` which carries `d` | `d > 1`, `n` not a power of 2 `>= 2`, `sel.len() != log2(n)`, more than 16 select lines |
| `decoder(xs)` | `2^len` lines, one-hot at the input value | empty, more than 16 inputs, any element `> 1` |
| `encoder(xs)` | the position of the single 1, `log2(len)` bits | length not a power of 2 `>= 2`, not exactly one 1, any element `> 1` |
| `priority_encoder(xs)` | `(position of the highest 1, valid)`; `((0..), 0)` when none | length not a power of 2 `>= 2`, any element `> 1` |
| `tri_state(d, en)` | `Some(d)` if `en == 1`, `None` (floating) if `en == 0` | `d > 1` or `en > 1`, **also when disabled** |

## Rust contract

* The stateless operations are exported from the crate root
  (`logic_gates::and_gate`) as well as from `gates` and `combinational`: the
  Warrant binding resolves an `export=` by name at the root. Same functions, so
  no existing call site changes. `sequential` is not re-exported.
* `MAX_DECODER_WIDTH: usize = 16`, public.
* Every function is a pure function of its arguments. None reads the clock, the
  environment, a file or a random source (the runtime-capability gate below
  checks this at run time).
* A bit is a `u8`; a bit vector is `&[u8]`; the multi-output blocks return
  `Vec<u8>`; `tri_state` returns `Option<u8>` (`None` is the high-impedance
  state); `priority_encoder` returns `(Vec<u8>, u8)`.

## Findings

Found while writing the contract (VER27 section 7.4; each with a backlog row, a
test that failed first, a fix, a Warrant rejection/claim and a lesson):

* **SEC-007: a non-bit was accepted silently in a release build.** The crate's
  own guard `validate_bit` was a `debug_assert!`. In release, `tri_state(2, 1)`
  returned `Some(2)`, `tri_state(2, 0)` returned `None`, `priority_encoder`
  read a 2 as an inactive input, and `encoder` summed its inputs in a `u8` so
  257 active inputs counted as 1 and passed the one-hot check (a debug build
  panics on that overflow instead). The gates, `mux` and `decoder` panicked in
  release only because the transistor model underneath rejects the value, which
  is luck, not a contract. `nand_gate`, `nor_gate` and `xnor_gate` had no check
  of their own at all. Fixed: `assert!` everywhere, `usize` count, the three
  gates validate. Pinned by `tests/bit_domain.rs` (fails under
  `cargo test --release` before the fix) and by the `*_rejects_non_bit*` claims
  (`tri_state_rejects_non_bit_data` includes the disabled case).
* **SEC-008: decoder width was unbounded.** `1usize << n` for any n: 17 to 40
  inputs attempt a 128 KiB to 1 TiB allocation and 2^n loop turns; 64 inputs
  wrap the shift to 1 in release and return one line silently. Fixed by the
  cap above. Pinned by `tests/decoder_width.rs` and the examples
  `decoder_seventeen_inputs_rejected`, `decoder_sixty_four_inputs_rejected`,
  `decoder_at_the_cap_has_65536_lines`, `demux_seventeen_select_lines_rejected`.
* **Proof-front-end refusals as design findings (VER27 section 7.4, P11).** The
  Rust-subset front end refuses `gates.rs` (the `&str` argument of
  `validate_bit`, a `None` argument to the CMOS constructors) and
  `combinational.rs`. Per function (recorded in `evidence/proof-manifest.json`):
  `mux2`, `mux4`, `nand_not`, `nand_and`, `nand_or`, `nand_xor` lower on their own;
  `mux_n` is stopped by `(n as f64).log2()` (a floating-point logarithm used
  to count bits, which `trailing_zeros()` states exactly); `decoder`, `demux`
  and `encoder` by closures (`|&b| ...`); the n-input gates by `iter().reduce()`;
  `priority_encoder` and `tri_state` by their `(Vec<u8>, u8)` and `Option<u8>`
  return types; `and_gate` and the other CMOS-backed gates by the `None`
  argument of the transistor constructors. None of these is a defect of the crate; the `f64` logarithm is the
  one worth changing before a step-bound proof is attempted (backlog LG-001).

## Contracts this layer publishes (pending LV-005)

Layer-contract linking (VER27 section 4.14, backlog LV-005) is not built. These
are the stable names under which a later layer will link to what this layer
proved. **A claim name is a published identifier: renaming one breaks every
upper layer's assumption**, so a rename needs a deprecation note here. Each
claim is verified by an exhaustive suite (every input of the stated domain), a
property suite (4,096 cases) and a fuzz suite (5,000 iterations).

The claims fall in four families:

* `<gate>_truth_table`: the function of the gate over `Bool x Bool`, against an
  oracle built from Warrant's own `and`/`or`/`not`/`!=`/`==`, never from the
  crate. `nand_*_equals_*`: the NAND-built gate equals the primitive.
* `<op>_rejects_*`: the rejection rows of the table above, over the whole `u8`
  domain for the scalar gates and over every non-bit value (`junk(x)` runs over
  all 254 of them) at every position for the blocks.
* `<block>_*_exhaustive`, `*_every_width`, `decoder_one_hot_*`,
  `encoder_inverts_decoder`: the block function, over every data word and every
  select, widths 1..=8 (decoder, demux, encoder, n-input gates), up to 16 inputs
  (`mux_n`, `priority_encoder`).
* `*_equals_*`: relations between operations (`mux_n` of two or four inputs
  equals `mux2`/`mux4`; the n-input gates of two inputs equal the 2-input gates;
  the encoder inverts the decoder).

The case count is the exhaustive suite's `cases` in
`evidence/test-manifest.json` (a claim with a refinement counts only the cases
the refinement admits).

| Claim | Operation | Exhaustive cases | Domain |
|---|---|---|---|
| `not_truth_table` | `not_gate` | 2 | a: Bool set(2) |
| `not_rejects_non_bit` | `not_gate` | 254 | a: U8 full(256) |
| `and_truth_table` | `and_gate` | 4 | a: Bool set(2); b: Bool set(2) |
| `and_rejects_non_bit` | `and_gate` | 65,532 | a: U8 full(256); b: U8 full(256) |
| `or_truth_table` | `or_gate` | 4 | a: Bool set(2); b: Bool set(2) |
| `or_rejects_non_bit` | `or_gate` | 65,532 | a: U8 full(256); b: U8 full(256) |
| `xor_truth_table` | `xor_gate` | 4 | a: Bool set(2); b: Bool set(2) |
| `xor_rejects_non_bit` | `xor_gate` | 65,532 | a: U8 full(256); b: U8 full(256) |
| `nand_truth_table` | `nand_gate` | 4 | a: Bool set(2); b: Bool set(2) |
| `nand_rejects_non_bit` | `nand_gate` | 65,532 | a: U8 full(256); b: U8 full(256) |
| `nor_truth_table` | `nor_gate` | 4 | a: Bool set(2); b: Bool set(2) |
| `nor_rejects_non_bit` | `nor_gate` | 65,532 | a: U8 full(256); b: U8 full(256) |
| `xnor_truth_table` | `xnor_gate` | 4 | a: Bool set(2); b: Bool set(2) |
| `xnor_rejects_non_bit` | `xnor_gate` | 65,532 | a: U8 full(256); b: U8 full(256) |
| `nand_not_truth_table` | `nand_not` | 2 | a: Bool set(2) |
| `nand_not_rejects_non_bit` | `nand_not` | 254 | a: U8 full(256) |
| `nand_not_equals_not` | `nand_not` | 2 | a: Bool set(2) |
| `nand_and_truth_table` | `nand_and` | 4 | a: Bool set(2); b: Bool set(2) |
| `nand_and_rejects_non_bit` | `nand_and` | 65,532 | a: U8 full(256); b: U8 full(256) |
| `nand_and_equals_and` | `nand_and` | 4 | a: Bool set(2); b: Bool set(2) |
| `nand_or_truth_table` | `nand_or` | 4 | a: Bool set(2); b: Bool set(2) |
| `nand_or_rejects_non_bit` | `nand_or` | 65,532 | a: U8 full(256); b: U8 full(256) |
| `nand_or_equals_or` | `nand_or` | 4 | a: Bool set(2); b: Bool set(2) |
| `nand_xor_truth_table` | `nand_xor` | 4 | a: Bool set(2); b: Bool set(2) |
| `nand_xor_rejects_non_bit` | `nand_xor` | 65,532 | a: U8 full(256); b: U8 full(256) |
| `nand_xor_equals_xor` | `nand_xor` | 4 | a: Bool set(2); b: Bool set(2) |
| `and_n_truth_table_every_width` | `and_n` | 16,640 | k: U8 full(256); n: Nat set(65) |
| `and_n_of_two_equals_and_gate` | `and_n` | 4 | a: Bool set(2); b: Bool set(2) |
| `and_n_rejects_a_short_input` | `and_n` | 256 | x: U8 full(256) |
| `and_n_rejects_non_bit_at_either_end` | `and_n` | 33,280 | j: U8 full(256); n: Nat set(65); front: Bool set(2) |
| `and_n_rejects_non_bit_in_the_middle` | `and_n` | 16,640 | j: U8 full(256); n: Nat set(65) |
| `or_n_truth_table_every_width` | `or_n` | 16,640 | k: U8 full(256); n: Nat set(65) |
| `or_n_of_two_equals_or_gate` | `or_n` | 4 | a: Bool set(2); b: Bool set(2) |
| `or_n_rejects_a_short_input` | `or_n` | 256 | x: U8 full(256) |
| `or_n_rejects_non_bit_at_either_end` | `or_n` | 33,280 | j: U8 full(256); n: Nat set(65); front: Bool set(2) |
| `or_n_rejects_non_bit_in_the_middle` | `or_n` | 16,640 | j: U8 full(256); n: Nat set(65) |
| `xor_n_truth_table_every_width` | `xor_n` | 16,640 | k: U8 full(256); n: Nat set(65) |
| `xor_n_of_two_equals_xor_gate` | `xor_n` | 4 | a: Bool set(2); b: Bool set(2) |
| `xor_n_rejects_a_short_input` | `xor_n` | 256 | x: U8 full(256) |
| `xor_n_rejects_non_bit_at_either_end` | `xor_n` | 33,280 | j: U8 full(256); n: Nat set(65); front: Bool set(2) |
| `xor_n_rejects_non_bit_in_the_middle` | `xor_n` | 16,640 | j: U8 full(256); n: Nat set(65) |
| `mux2_truth_table` | `mux2` | 8 | d0: Bool set(2); d1: Bool set(2); s: Bool set(2) |
| `mux2_rejects_non_bit_d0` | `mux2` | 1,024 | j: U8 full(256); x: Bool set(2); s: Bool set(2) |
| `mux2_rejects_non_bit_d1` | `mux2` | 1,024 | j: U8 full(256); x: Bool set(2); s: Bool set(2) |
| `mux2_rejects_non_bit_sel` | `mux2` | 1,024 | j: U8 full(256); x: Bool set(2); y: Bool set(2) |
| `tri_state_truth_table` | `tri_state` | 4 | d: Bool set(2); e: Bool set(2) |
| `tri_state_rejects_non_bit_data` | `tri_state` | 512 | j: U8 full(256); e: Bool set(2) |
| `tri_state_rejects_non_bit_enable` | `tri_state` | 512 | d: Bool set(2); j: U8 full(256) |
| `mux4_truth_table` | `mux4` | 64 | x0: Bool set(2); x1: Bool set(2); x2: Bool set(2); x3: Bool set(2); s0: Bool set(2); s1: Bool set(2) |
| `mux4_rejects_non_bit_d0` | `mux4` | 1,024 | j: U8 full(256); s0: Bool set(2); s1: Bool set(2) |
| `mux4_rejects_non_bit_d1` | `mux4` | 1,024 | j: U8 full(256); s0: Bool set(2); s1: Bool set(2) |
| `mux4_rejects_non_bit_d2` | `mux4` | 1,024 | j: U8 full(256); s0: Bool set(2); s1: Bool set(2) |
| `mux4_rejects_non_bit_d3` | `mux4` | 1,024 | j: U8 full(256); s0: Bool set(2); s1: Bool set(2) |
| `mux4_rejects_non_bit_sel_0` | `mux4` | 512 | j: U8 full(256); s: Bool set(2) |
| `mux4_rejects_non_bit_sel_1` | `mux4` | 512 | j: U8 full(256); s: Bool set(2) |
| `mux4_rejects_wrong_select_width` | `mux4` | 331 | s: Bytes set(347) |
| `mux_n_2_inputs_exhaustive` | `mux_n` | 512 | x: U8 full(256); s0: Bool set(2) |
| `mux_n_4_inputs_exhaustive` | `mux_n` | 1,024 | x: U8 full(256); s0: Bool set(2); s1: Bool set(2) |
| `mux_n_8_inputs_exhaustive` | `mux_n` | 2,048 | x: U8 full(256); s0: Bool set(2); s1: Bool set(2); s2: Bool set(2) |
| `mux_n_16_inputs_exhaustive` | `mux_n` | 1,048,576 | x: U16 full(65536); s0: Bool set(2); s1: Bool set(2); s2: Bool set(2); s3: Bool set(2) |
| `mux_n_of_two_equals_mux2` | `mux_n` | 8 | d0: Bool set(2); d1: Bool set(2); s: Bool set(2) |
| `mux_n_of_four_equals_mux4` | `mux_n` | 64 | x0: Bool set(2); x1: Bool set(2); x2: Bool set(2); x3: Bool set(2); s0: Bool set(2); s1: Bool set(2) |
| `mux_n_rejects_a_bad_shape` | `mux_n` | 116,153 | a: Bytes set(347); s: Bytes set(347) |
| `mux_n_rejects_non_bit_input_0` | `mux_n` | 262,144 | j: U8 full(256); x: U8 full(256); s0: Bool set(2); s1: Bool set(2) |
| `mux_n_rejects_non_bit_input_1` | `mux_n` | 262,144 | j: U8 full(256); x: U8 full(256); s0: Bool set(2); s1: Bool set(2) |
| `mux_n_rejects_non_bit_input_2` | `mux_n` | 262,144 | j: U8 full(256); x: U8 full(256); s0: Bool set(2); s1: Bool set(2) |
| `mux_n_rejects_non_bit_input_3` | `mux_n` | 262,144 | j: U8 full(256); x: U8 full(256); s0: Bool set(2); s1: Bool set(2) |
| `mux_n_rejects_non_bit_sel_0` | `mux_n` | 131,072 | j: U8 full(256); x: U8 full(256); s: Bool set(2) |
| `mux_n_rejects_non_bit_sel_1` | `mux_n` | 131,072 | j: U8 full(256); x: U8 full(256); s: Bool set(2) |
| `decoder_one_hot_every_width` | `decoder` | 16,640 | w: U8 full(256); n: Nat set(65) |
| `decoder_rejects_non_bit_last` | `decoder` | 16,640 | j: U8 full(256); n: Nat set(65) |
| `decoder_rejects_non_bit_first` | `decoder` | 16,640 | j: U8 full(256); n: Nat set(65) |
| `decoder_rejects_non_bit_alone` | `decoder` | 256 | x: U8 full(256) |
| `demux_routes_data` | `demux` | 33,280 | d: Bool set(2); w: U8 full(256); n: Nat set(65) |
| `demux_rejects_non_bit_data` | `demux` | 16,640 | j: U8 full(256); n: Nat set(65) |
| `demux_rejects_non_bit_select` | `demux` | 16,640 | j: U8 full(256); n: Nat set(65) |
| `demux_rejects_a_bad_shape` | `demux` | 44,426 | d: Bool set(2); s: Bytes set(347); n: Nat set(65) |
| `encoder_inverts_decoder` | `encoder` | 16,640 | w: U8 full(256); n: Nat set(65) |
| `encoder_rejects_no_active_input` | `encoder` | 65 | n: Nat set(65) |
| `encoder_rejects_two_active_inputs` | `encoder` | 16,640 | w: U8 full(256); n: Nat set(65) |
| `encoder_rejects_non_bit` | `encoder` | 16,640 | j: U8 full(256); n: Nat set(65) |
| `encoder_rejects_a_bad_shape` | `encoder` | 327 | a: Bytes set(347) |
| `priority_encoder_every_word_2_4_8` | `priority_encoder` | 16,640 | w: U8 full(256); n: Nat set(65) |
| `priority_encoder_rejects_non_bit_first` | `priority_encoder` | 65,536 | j: U8 full(256); x: U8 full(256) |
| `priority_encoder_rejects_non_bit_last` | `priority_encoder` | 65,536 | j: U8 full(256); x: U8 full(256) |
| `priority_encoder_rejects_non_bit_middle` | `priority_encoder` | 65,536 | j: U8 full(256); x: U8 full(256) |
| `priority_encoder_rejects_a_bad_shape` | `priority_encoder` | 327 | a: Bytes set(347) |
| `priority_encoder_16_inputs_exhaustive` | `priority_encoder` | 65,536 | x: U16 full(65536) |

Total: **3,225,774 exhaustive + 544,349 bounded-set (3,770,123 total) cases over 88 claims**, 83 examples, 204 negative controls. A bounded-set case is one whose domain is a fixed set rather than every value: a Bytes `set(347)`, or a `Nat set(65)` width or position.

A claim's formula is in the Warrant file under the same name; the text of an
upper layer's assumption must be that formula, not a paraphrase (VER27 rule (c)).

### What the claims do not cover (rule (a), coverage)

* **Time.** These are functional contracts: output as a function of inputs.
  They say nothing about delay, so no constant-time or timing claim can rest on
  them alone; a timing contract ("one evaluation per call, regardless of
  data") is a separate row for the day a chain needs it (VER27 section 4.15,
  "Timing contracts"). Nothing in this document claims the gates are constant
  time, and no operand is secret, so the report records CWE-208 as not
  applicable rather than as proved.
* **Widths past what is enumerated.** `decoder`, `demux`, `encoder` and the
  n-input gates are enumerated to 8 bits, `mux_n` and `priority_encoder` to 16
  inputs. A 9..=16 bit decoder is exercised in the runtime-capability run and
  by the width-16 example, but is not enumerated: that is a stated limit, not a
  generalisation (the honest sentence is "exhaustive over these domains").
* **Hostile byte-string content beyond the sampled positions.** A non-bit is
  rejected at every position of the short vectors; for a long vector the
  rejection is checked at the first, last and middle positions (the property
  and fuzz suites also draw arbitrary hostile bytes).

## Trust boundary

What the proof by enumeration assumes (VER27 section 4.15, "Trust boundary"):

* **The Rust host and toolchain.** The evidence runs the compiled crate. That
  `u8` arithmetic, slices, `Vec` and `catch_unwind` do what the language says,
  and that the compiler preserves the semantics, is assumed.
* **The `transistors` crate is part of the code under test, not an oracle.**
  Every gate delegates to it, and the expected values here come from Warrant's
  Boolean operators, not from `transistors`. A claim therefore says "the CMOS
  model, as compiled, computes these functions"; it does **not** say the CMOS
  model matches a physical transistor. `transistors` has no Warrant file; the
  transistor-model-equivalence half of the VER13 discharge plan is open.
* **The datasheet.** The truth tables are the standard Boolean functions; they
  are not transcribed from a datasheet. (The 4004 datasheet enters at the CPU
  layer.)
* **Warrant itself** and its evidence generator, as for every adopter.
* **Nothing about silicon**, the L0 floor entries stay printed.

"Exhaustive" is proof by enumeration of the Rust function over a finite,
stated domain. It is not a proof about the source in the Rust-subset sense:
the step-bound proofs are `not-live` (the front end refuses the source).

## Cross-language expectations

The Warrant file is language-neutral except for the binding. The same truth
tables, shapes and width cap are expected of the other `logic-gates` ports
(Python, TypeScript, Go and the rest); `logic-gates.warrant` is the specimen a
polyglot binding (VER04i) would re-run. The panic-as-rejection rule is Rust's;
a port in a language without panics should raise its idiomatic error, and the
single `INVALID_INPUT` case covers it. No other port has been checked against
this contract.

## Verification status (Warrant-derived)

1. **Examples and claims.** 83 examples (every row of the seven primitive truth
   tables, written out as a datasheet prints them, plus rejection and
   boundary examples) and 88 claims, each with an exhaustive, a property and a
   fuzz suite.
2. **Exhaustive means exhaustive.** 3,225,774 exhaustive + 544,349 bounded-set (3,770,123 total) cases in the
   debug profile, 0 unreachable by type. The largest are `mux_n_16_inputs_exhaustive`
   (1,048,576: every 16-bit data word and every 4-bit select) and the rejection
   claims of the 2-input gates (65,532 each: every pair of `u8` with a non-bit).
3. **Negative controls.** 204 (51 operation/fault pairs x example, exhaustive,
   property, fuzz), each catching its fault by a named claim; `BUILD` runs the 51
   pairs as sabotage lines, each printing a `not ok`.
4. **Release profile.** The whole generated harness (551 entries) and
   `tests/bit_domain.rs`, `tests/decoder_width.rs` also run under
   `--release`, where the original SEC-007 hid.
5. **Wall clock** on the authoring machine: about 2 minutes for the generated
   target in the debug profile (the 2^20-case multiplexer claim and the
   65,532-case rejection claims, each a contained panic, dominate), about 1 minute
   including the compile in release.
6. **Heap** is not bounded (no `heap` row): the transistor model allocates per
   call, and a `heap` row would put every instance of the 2^20-case claim under
   the allocation observer.
7. **Steps and depth** rows are declarations; the proof manifest records every
   steps-bound obligation `not-live` with the front end's own refusal, and
   depth `unchecked`.
8. **CWE-208 does not apply:** every operand is `public` (a gate is a public
   Boolean function). The report records it as the unapproved exemption
   `EXEMPT-CWE-208-NO-SECRET-OPERANDS`, backed by the derived attestation
   `warrant-spec::all-operands-public`, not as a constant-time claim.

### Runtime capability evidence (VER08b)

`required_capabilities.json` declares `"capabilities": []`; the static analyzer
agrees (`effect_certificate.json`). `tests/runtime_capability.rs` adds the
run-time half: phase A builds the vectors unconstrained, phase B arms a
seccomp-bpf + Landlock jail derived mechanically from the manifest (minimal
baseline: allocator, `write` of TAP lines, `exit_group`) and calls every public
stateless operation: all eleven scalar gates against Rust's Boolean operators,
the n-input gates over every vector of width 2..=6, `mux2` and `mux4` over every
input, `mux_n` for 2, 4, 8 and 16 inputs and every select, `decoder` over every
input of width 1..=6 and the 16-input decoder (65,536 lines), `demux`,
`encoder`, `priority_encoder` and `tri_state`. Only valid inputs are used in the
jail (a rejection panics and prints to stderr, which the baseline forbids);
rejections are pinned unjailed. 11 named checks pass. An independent `strace -f`
observer agrees, and 18 negative controls are each caught by both rings. The
container ring is `NOT-LIVE` where no Docker daemon is reachable. `BUILD_linux`
runs the gate with `RCG_STRICT=1`; the plain `BUILD` (macOS) does not;
`BUILD_windows` does not run it. Known residual: vDSO time reads (CAP-RT-006).

The assurance report (`--profile boolean-logic`, a new profile: the
`varint-codec` sentences name LEB128, which would be false here) is
`report-only`: rungs E0, E1, E3 and E4 `pass`, E2 and E5 `not-live`;
CWE-208 is the not-applicable exemption above and CWE-400 the unapproved "no
step-bound proof is live" exemption.

## Next slice: `sequential.rs`

`sequential.rs` (SR latch, D latch, D flip-flop, register, shift register,
counter) is deliberately not bound; it is the next adoption slice (backlog
LV-006, "Sequential"). It is stateful in the plain sense: `d_flip_flop(data,
clock, &mut FlipFlopState)` returns outputs *and* changes the state it was
given, and its behaviour is defined by clock **edges**, so no single call has a
function to check. What Warrant needs to say it:

1. **A state-carrying operation form.** `&mut FlipFlopState`, `&mut
   [FlipFlopState]` and `&mut CounterState` are in-out parameters; WX-G
   (VER04k section 4, backlog WX-G: `inout` and `ensures`) is the feature, with
   the `&mut` adapter. Records for the state exist today (WX-A).
2. **Sequences of calls.** A transition such as "Q takes D on the rising
   edge and holds otherwise" is a property of a *trace* of `(data, clock)`
   pairs. WX-F (VER04k section 3, backlog WX-F: binding-level drivers) drives
   the constructor/step/finish pattern and sweeps split points; the layer also
   needs the driver to enumerate **every trace up to length k** over the small
   input alphabet (a flip-flop over `(d, clk) in {0,1}^2` has 4^k traces, 4,096
   at k = 6), which is the stateful analogue of the truth table.
3. **A state-machine model.** The oracle is a pure `step(state, inputs) ->
   (state', outputs)` over a record, written as models today; the new part is
   *checking the implementation's every step against it* (refinement), not
   writing the model.
4. **Edge detection and the latch settling loop.** The master/slave flip-flop
   detects an edge by holding the previous clock level in its state; the
   SR latch iterates until its feedback settles and has an *invalid* input
   (S = R = 1) whose behaviour must be stated. These are model content, and
   decisions for the spec (reject, or define the outputs), not Warrant features.
5. **Timing contracts** (rule (a)): "one step per call regardless of data" if a
   constant-time chain ever stands on these.

No new Warrant feature is requested here (P7, feature freeze): items 1 and 2 are
already specified (WX-F, WX-G) and waiting for an adopter; this slice is that
adopter's request.
