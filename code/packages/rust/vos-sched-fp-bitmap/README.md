# vos-sched-fp-bitmap

The **fixed-priority bitmap scheduler** of the verified-OS program (backlog VOS-011): the
first real component. It implements [`vos-sched-contract`](../vos-sched-contract) (VOS10)
for eight threads, ids `0..8`, where a larger id is a higher priority. Crate class **K**,
kind **production**; it carries the **safety**, **policy** and **canonical** tiers.

## How it works

The ready set is one byte; bit `t` is set when thread `t` is ready.

```text
  insert(t)   mask | (1 << t)            Changed, Unchanged (already set) or OutOfRange (t >= 8)
  remove(t)   mask & !(1 << t)           Changed, Unchanged (not set) or OutOfRange (t >= 8)
  dispatch    highest set bit, cleared   Run(7 - clz8(mask)), or Idle when the mask is 0
```

`clz8` is a three-stage binary search (top four bits zero? top two? top one?): no loop, no
branch, no division, so the cost is visible and `u8::leading_zeros` (which may become a
runtime call on RV32I) is not needed. The source explains it with a worked example.

Every operation is total and by value (`op(State, In) -> (Out, State)`); a bad id is an
`OutOfRange` answer, never a panic. `#![no_std]`, `#![forbid(unsafe_code)]`, and
`clippy::arithmetic_side_effects` is denied (SH9).

## Where it fits

```text
 vos-sched-contract (trait Sched, warrant/*.part)  ->  vos-sched-fp-bitmap (this)  ->  the spine (VOS-026)
                                                       code/specs/vos-sched-fp-bitmap.warrant (generated)
```

## Layout

| Path | What |
|---|---|
| `src/lib.rs` | `Bitmap` (private field), `FpBitmap`, `impl Sched`, and the feature-gated `ghost` face |
| `vos.toml` | class K, kind `production`, `contract`, `obligations = [canonical, policy, safety]` |
| `warrant/impl.part` | the hand-written implementation block (bound operations, `bound` rows, labels, one example) |
| `warrant/evidence.bindings.pending` | the VER04h bindings, parked until the generator can read the document (see below) |
| `tests/behaviour.rs` | hand-written tests against a plain reference model, with and without `ghost` |
| `warrant/warrant_evidence.rs.pending` | the shim that will run the generated harness (parked with the bindings) |
| `code/specs/vos-sched-fp-bitmap.warrant` | the **generated** document (`vos-link gen-doc`); never edited by hand |

## The ghost face

The shipped `Bitmap` keeps its field private. With the cargo feature `ghost` (evidence build
only; a plain `cargo build` never sees it) the `ghost` module adds a public-field `Image`,
`AbsW` and `CfgN` records and `vos_ghost_*` wrappers (exhaustive destructure, one UFCS call
into the production trait, restructure) re-exported at the crate root, so the harness runs
the code that ships (VOS02 section 4).

## Checks

```text
cargo test -p vos-sched-fp-bitmap                    # behaviour tests
cargo test -p vos-sched-fp-bitmap --features ghost   # plus the ghost wrappers
cargo run -q -p vos-shape-check -- .                 # SH1-SH18, class K production
vos-link check-doc .                                 # the generated document is current and lawful
```

Line coverage of `src/lib.rs` is 100% (llvm-cov, hand-instrumented).

## Status of the generated evidence

The Warrant evidence harness is **not generated yet** (VOS-D064): the contract block holds
nine model-level lemmas, and today's generator rejects a claim that calls no operation
(`W0701`). With the nine lemmas removed from a scratch copy, the other 20 claims and 2 examples
generate, compile and run: 394,498 exhaustive cases, all at or below 65,536 per claim; the
only failing controls are `init` / `field-zero`, inert on the zero initial state (VOS-D065).
