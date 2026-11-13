# arithmetic (Rust)

Adder circuits and ALU — the computational heart of a CPU.

## What This Is

This crate implements arithmetic building blocks from logic gates: half adders, full adders, ripple-carry adders, and a complete Arithmetic Logic Unit (ALU) with status flags. Everything is built on top of the `logic-gates` crate.

## How It Fits in the Stack

The arithmetic crate sits on Layer 2, directly above logic-gates (layer 1 of the Intel 4004 verification chain; this is layer 2). The ALU is the core "calculator" inside every processor — every ADD, SUB, AND, OR instruction passes through it. Higher layers (floating-point arithmetic, pipeline stages) build on these primitives.

## Modules

- **`adders`** — half_adder, full_adder, ripple_carry_adder
- **`alu`** — ALU with ADD, SUB, AND, OR, XOR, NOT operations and status flags (zero, carry, negative, overflow)

## Usage

```rust
use arithmetic::adders::{half_adder, full_adder, ripple_carry_adder};
use arithmetic::alu::{alu, AluOp};

// Half adder: 1 + 1 = 0, carry 1
assert_eq!(half_adder(1, 1), (0, 1));

// Ripple-carry: 5 + 3 = 8 (LSB-first bit vectors)
let result = ripple_carry_adder(&[1,0,1,0], &[1,1,0,0]);
assert_eq!(result.sum, vec![0,0,0,1]); // 8

// ALU: 3 + 5 = 8 with flags
let a = vec![1,1,0,0,0,0,0,0]; // 3 LSB-first
let b = vec![1,0,1,0,0,0,0,0]; // 5 LSB-first
let result = alu(&a, &b, AluOp::Add);
// result.result = 8, result.zero = false, result.carry = false
```

## Domain

A wire is a `u8` that must be 0 or 1, and a bit vector is a `&[u8]`, least
significant bit first. Anything else is rejected by a **panic, in every build
profile**: a value that is not a bit (in any position, and in the carry-in),
operands of unequal length, an empty operand. The rejection of a non-bit is
inherited from the `logic-gates` gates every bit passes through, and is pinned
here (`tests/bit_domain.rs`, also run under `--release`). `AluOp::Not` never reads
`b`: any `b`, even empty, is accepted and ignored. There is no width-driven
allocation, so no width cap. After a `Sub` the ALU's `carry` means *no borrow*
(`a >= b`). See [AR01](../../../specs/AR01-arithmetic-contract.md).

## Verification

The operations are specified by
[`code/specs/arithmetic.warrant`](../../../specs/arithmetic.warrant), a Warrant
v1 document, with the prose contract in
[AR01](../../../specs/AR01-arithmetic-contract.md). It is layer 2 of the Intel
4004 verification chain (VER27 section 4.15), on top of `logic-gates`.
`tests/generated/warrant_evidence.rs` is *generated* from it and
`evidence.bindings` (never edit it; `BUILD` fails if it drifts) and runs against
the real exports:

- **exhaustive suites, 2,655,437 cases in 63 claims:** the half and full adder
  truth tables (a Boolean oracle and, independently, an arithmetic one); both
  ripple-carry adders at **every** input for 1, 2, 3, 4 and 8 bits (the 4-bit
  adder with a carry-in is the 512 cases `(a + b + cin) mod 16` plus carry and
  signed overflow); each of the six ALU operations with all four flags at 4 and 8
  bits (Add and Sub also at 1 to 3); and the rejection of every non-bit at each of
  eight positions, of unequal lengths and of an empty operand. The oracle is
  Warrant's own integers (the subtraction is `a - b`, not `a + NOT(b) + 1`),
  never the crate. That is proof by enumeration of the Rust function as compiled;
  it assumes the host language and toolchain, and `logic-gates`/`transistors` are
  under test, not an oracle (AR01 "Trust boundary");
- **16 bits is sampled, and named so** (`*_16_bit_sampled`: the 377-value
  boundary domain squared); nothing is claimed above 16 bits;
- **property and fuzz suites** for every claim, and **sabotage controls** that
  inject wrong results and a swallowed rejection, and must fail. The generator's
  faults need a numeric or bytes result, so the ALU's only control is a swallowed
  rejection (AR01 "Verification status").

The claim names (`ripple_carry_adder_with_carry_4_bit_exhaustive`,
`alu_sub_8_bit_exhaustive`, ...) are stable identifiers a later layer links to
(AR01, "Contracts this layer publishes"), as is the list of contracts this layer
would assume from `logic-gates` (pending layer-contract linking, VER29).
`evidence/test-manifest.json` is written by that run; `assurance_report.json` is
derived from it (verdict `report-only`, audit profile `bit-arithmetic`). What is
*not* claimed: timing, widths above 16, the step-bound proofs (**not live**: the
WIR front end refuses the sources; `evidence/proof-manifest.json` records why),
and CWE-208 is recorded as not applicable because every operand is public.

## Runtime capability conformance

The library is pure (`required_capabilities.json` declares no capabilities).
`cargo test -p arithmetic --test runtime_capability` proves it at run time on
Linux: every public operation runs inside a seccomp/Landlock sandbox derived from
that manifest while an independent `strace` observer confirms nothing but
`write`/`exit_group` (and allocator) syscalls happen (VER08b). Set
`RCG_STRICT=1` to fail instead of reporting NOT-LIVE where the sandbox is
unavailable.

## Running Tests

```bash
# Unit, doc and hand-written integration tests (the generated evidence target is separate)
cargo test -p arithmetic --lib
cargo test -p arithmetic --doc
cargo test -p arithmetic --test adders_test --test alu_test
# The bit domain, in both profiles (the gates' SEC-007 guard is what rejects a non-bit)
cargo test -p arithmetic --test bit_domain
cargo test -p arithmetic --release --test bit_domain

# Regenerate the Warrant evidence harness after editing the .warrant document
cargo run -p coding_adventures_warrant_grammar --example gen_evidence -- arithmetic

# Everything CI runs, including the evidence, sabotage and report checks: see BUILD
```
