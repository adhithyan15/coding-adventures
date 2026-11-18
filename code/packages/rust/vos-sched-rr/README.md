# vos-sched-rr

The **round-robin scheduler** of the verified-OS program (backlog VOS-012b) and its first
**swap**: it implements the same contract as [`vos-sched-fp-bitmap`](../vos-sched-fp-bitmap),
[`vos-sched-contract`](../vos-sched-contract) (VOS10), for eight threads, ids `0..8`, with a
different policy. The bitmap scheduler always runs the highest-numbered ready thread; this one takes
turns. Crate class **K**, kind **production**; it carries the **representation** and **safety**
tiers (no `canonical`, no `policy`).

## How it works

The state is two bytes: `mask` (bit `t` set when thread `t` is ready) and `ptr` (where the next scan
starts).

```text
  insert(t)   mask | (1 << t)             Changed, Unchanged (already set) or OutOfRange (t >= 8); ptr untouched
  remove(t)   mask & !(1 << t)            Changed, Unchanged (not set) or OutOfRange (t >= 8); ptr untouched
  dispatch    first ready thread t scanning ptr, ptr+1, ... (mod 8): clear its bit, ptr = (t + 1) & 7
              Idle (state unchanged) when the mask is 0
```

The scan is loop-free: rotate the mask right by `ptr`, isolate its lowest set bit, locate it with the
same three-stage `clz8` as the bitmap scheduler. The source walks through a worked example.

Every operation is total and by value (`op(State, In) -> (Out, State)`); a bad id is an
`OutOfRange` answer, never a panic. `#![no_std]`, `#![forbid(unsafe_code)]`, and
`clippy::arithmetic_side_effects` is denied (SH9).

## Why the representation tier

`alpha` (the abstraction to a ready set) forgets `ptr`: states with the same mask and different pointers
are the same abstract state, so `canonical` ("the state is determined by the abstraction") does not hold.
The representation tier (VOS02 2.7) instead says the reachable states are exactly the image of
`rep(m, h) = Impl(<mask of m>, h)` over every ready set `m` and history code `h = ptr`, and that
`insert`, `remove` and `dispatch` refine the abstract operations and stay inside that image.

## The swap exercise (VOS00 section 8)

Swapping `vos-sched-fp-bitmap` for `vos-sched-rr` in a profile regenerates **only this crate**: its
document, its evidence, its model. `vos-sched-contract`, `vos-sched-fp-bitmap` and `vos-sched-any` are
byte-for-byte untouched; the BUILD of each still passes `check-doc` and `gen_evidence --check` against
the documents already checked in (verified when this crate landed).

| | `vos-sched-fp-bitmap` | `vos-sched-rr` |
|---|---|---|
| tiers carried | safety, policy, canonical | safety, representation |
| picks | highest id (highest priority first) | the next ready id after the one last served |
| starvation | a low id can starve | none: a served thread waits for every other ready thread |
| state | 1 byte | 2 bytes |

**What the swap gains:** fairness among equals. **What it loses:** the `policy` tier. Round-robin is not
highest-priority-first, so a profile whose theorem `needs` the `policy` tier (or `canonical`) cannot use
this implementation: `vos-link check-profile` rejects the swap, which is the point of the tiers.

## Layout

| Path | What |
|---|---|
| `src/lib.rs` | `Rr` (private fields), `RoundRobin`, `impl Sched`, and the feature-gated `ghost` face |
| `vos.toml` | class K, kind `production`, `contract`, `obligations = [representation, safety]` |
| `warrant/impl.part` | the hand-written implementation block: bound operations, `rep`, `hist`, the op-free `rr_pick`/`rr_next`, own claims, examples, `bound` rows |
| `evidence.bindings` | the VER04h bindings: every operation is bound to a `vos_ghost_` wrapper with its faults |
| `tests/generated/warrant_evidence.rs` | the generated suites and negative controls (never edited; BUILD checks it for drift) |
| `evidence/test-manifest.json` | the evidence manifest written by the last real run |
| `warrant/check_evidence.sh` | runs the suite and names the three controls that still cannot pass (VOS-D098) |
| `tests/behaviour.rs` | hand-written tests against a plain scan reference, with and without `ghost` |
| `code/specs/vos-sched-rr.warrant` | the **generated** document (`vos-link gen-doc`); never edited by hand |

## The generated evidence

```text
cargo run -p coding_adventures_warrant_grammar --example gen_evidence -- vos-sched-rr          # regenerate
cargo run -p coding_adventures_warrant_grammar --example gen_evidence -- --check vos-sched-rr
sh warrant/check_evidence.sh                                                                   # run it
```

16 exhaustive claims, 1,573,380 cases, the largest 262,144 (65.5% of the 400,000 budget), plus the same 16 as
property and fuzz suites. The 13 contract claims are the representation tier's eight and the safety
tier's five; the own ones are `dispatch_det_out`, `dispatch_det_img` and `pointer_kept`.
Three controls cannot pass: `init` / `field-flip` (VOS-038a, WX-SV2). The only claim whose subject is
`init` is `rep_init`, which says the state is in the image of `rep`, and a flip of `mask` stays in it
(VOS-D087); `warrant/check_evidence.sh` requires cargo status 1, exactly 78 ok and exactly those three
failures, until VOS-D098 lets `safety_init` (which does catch it) count for `init`. The claims
`rep_init`, `dispatch_rep_post` and `safety_init` do have sabotage lines now: `field-flip` aimed at
`alpha_w` (and, for `safety_init`, at `init`) with `WARRANT_EVIDENCE_ATTACK`.

Mutation sanity (scratch edits of the source, restored): an `insert` that resets `ptr` fails
`pointer_kept` (and four behaviour tests); a `dispatch` that sets `ptr = t` or scans from 0 fails
`dispatch_det_img` / `dispatch_det_out`. The representation claims alone do not catch those mutants:
a reset pointer is still inside the image of `rep`, which is what the ungated own claims are for.

Line coverage of `src/*` is a **gate** (VOS-D070, VOS04 5.6): `coverage-floor = 95` in `vos.toml`, enforced on Linux by the
`cargo tarpaulin ... --include-files 'vos-sched-rr/src/*' --fail-under 95` line of BUILD, and `vos-link check-component` fails if the floor,
the line or its `--include-files` is missing or they disagree. Measured (tarpaulin, LLVM engine, `--lib --test behaviour --features ghost`,
2026-11-17): **100%, 84 of 84 lines of `src/lib.rs`**. The generated evidence suite is not part of the measurement (VOS-D106).

