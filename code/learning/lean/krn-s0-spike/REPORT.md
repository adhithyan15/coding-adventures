# KRN-S0 — feasibility spike report

**Backlog item:** KRN-S0 ([VER11a](../../../specs/VER11a-lean4-proof-kernel.md)
§9.2, §14.0).

**Toolchain:**
- Lean `v4.35.0` (released 2026-10-12), core only, pinned in `lean-toolchain`.
- It is the newest stable release that is at least two weeks old. `v4.36.0`
  was only seven days old, so it was skipped.
- elan came from `github.com/leanprover/elan`, installed with
  `-y --no-modify-path`.

**Verdict: feasible, with changes.**

Every numbered part of the spike was proved, and the coordinator re-checked
the result independently: a clean `lake build` succeeds, no `sorry` appears
anywhere, and `#print axioms` gives

```
'KrnS0.ct_eq_equal_length_ct' depends on axioms: [propext, Quot.sound]
```

The main theorem as stated:

```lean
KrnS0.ct_eq_equal_length_ct : ∀ {m₁ m₂ : Mem} {allocs : Nat} {a b : Val},
  SameShape m₁ m₂ → AgreeOutside m₁ m₂ → (∀ q, allocs ≤ q → m₁ q = none) →
  ∀ n, obs (run n ctEq (entryState m₁ allocs a b)) = obs (run n ctEq (entryState m₂ allocs a b))
```

It holds for every slice length, every step count, and every pair of
memories that differ only inside secret-tagged allocations. Fresh allocation
ids are covered.

## What worked

- **Semantics.**
  - `step`, plus `run`, which recurses structurally on the step count.
  - Cost, events, `obs` and tagged memory.
  - A frame stack with `call` and `ret`, and deterministic allocation ids.
- **Concrete witness.** `run 100 ctEq …` reaches `ret true` at 59 cost units.
  Plain `decide` proves it in 0.3 s.
- **Symbolic state needs step lemmas**, as review round 2 predicted. There are
  19 step lemmas, each proved once, including `step_branch_true` and
  `step_branch_false`.
- **One obligation lemma per instruction or terminator kind**, each proved
  once over symbolic states. The kinds are let, alloc, load, store, free, jump,
  branch, ret, trap and unreachable.
- **The per-program part is a decidable check.**
  - The **label table** (untrusted) gives, for each program point, what is
    public. `ok p lab f b i = true` must hold at every point.
  - One bounded `decide` checks the whole program (`ctEq_checked`).
  - Each of the 11 loop-body points is a one-line application of the generic
    lemmas.
- **`lockstep_obs`** is proved once, by induction on the step count, in about
  25 lines with its helpers.
- **Sabotage controls** (`Sabotage.lean`). `decide` refuses each of these:
  - a label table that calls a loaded secret byte public;
  - a branch on a secret byte;
  - a secret stored through a public-tagged slot.

## Changes versus VER11a

### F1. §9.2: generate data plus a decidable check, not tactic proofs

The VC builder emits *data*: the program constant and a label table (22
entries here), both untrusted. The kernel-checked part is the generic lemmas
plus `decide` of `ok` at every program point. The relational case therefore
needs no generated tactic text, and with it goes the implicit-argument and
instance problem of generating Lean. The spike did not test whether the
unary invariants of §9.3 fit the same shape.

### F2. §8.3: the relation needs more than "public variables equal, traces equal"

- **Freshness** (`∀ q ≥ allocs, mem q = none`) is a field of `R`, not only an
  entry premise. `alloc` must prove, at every step, that the new id was
  unused.
- **Pointer tag facts.**
  - The real lowering keeps the loop counter in an 8-byte stack slot and
    reloads `i` from it on every trip. So "`i` is public" depends on that
    slot's tag.
  - The label table therefore lists, per point, `pubPtr` (variables pointing
    into public-tagged allocations) and `secPtr` (variables pointing into
    secret-tagged ones), and `R` carries `PtrOK` for both.
- **`RetOK`.** A callee's result is treated as secret in the caller.
- **No `WfState` in the relational `R`.** An out-of-bounds access is `stuck`
  on both sides alike, because address, size and liveness agree. `WfState` is
  needed only for the unary claims.
- **§8.2's `Lockstep` simplifies.**
  - The `newEvents` and cost clauses are redundant once `R` carries trace and
    step equality.
  - `step` is total, so no `running` guard is needed.
- **Stuckness must not depend on secrets.** The spike makes variables total
  and has ill-typed operators return a default. Either the spec states that
  the WIR type checker rules both out, or `R` carries "same value shape" for
  secrets.

### F3. §6 and cost

- **Cost schedule mismatch.**
  - Return: VER25 §5.1 says 2, `cost.rs` says 1 (`return_cost: 1`).
  - Alloc and free: §5.1 gives no cost, `cost.rs` gives 3 and 2.
  - KRN-006 must choose one schedule.
- **No bounds-check branches.** Real WIR has none: `lower.rs` emits
  `PtrOffset` then `Load`, and an out-of-bounds access is `stuck`. §8.4's
  wording is wrong.
- **Stack slots exist.** The loop slot is allocated and freed; `acc` is
  allocated and never freed. This confirms that `Alloc` needs a kind.

### F4. Witnesses

- Plain `decide` proves a single witness.
- Comparing two whole `obs` values hit the 200,000-heartbeat limit. It needed
  `decide +kernel`, which adds no axiom.
- Witnesses should use the kernel form from the start.

### F5. Axioms

- `propext` comes from `simp` and `rw` on equations.
- `Quot.sound` comes from `funext` in the memory-update lemmas.
- `Classical.choice` was avoidable. A single `omega` call on a negated
  conjunction introduced it, and a 9-line lemma replaced that call.
- The K-M0 kernel must accept `propext` and `Quot.sound`.

## Simplifications (not yet faithful to `warrant-ir`)

- The WIR of `ct_eq_equal_length` is a hand transcription of `lower.rs`.
  `warrant-ir` was not run.
- Integers have no width and do not wrap. The real `i + 1` is `Add(Wrap)` on
  `U64`.
- A load or store addresses one cell, not a byte range.
- There are no capability calls, tuples, dynamic values or overflow traps.
- **A `call` as a lockstep step is not proved yet.** `call` is in `step`, and
  `ret` into a caller frame is proved, but `ok` refuses any program that
  contains a `call`.
- The unary side was not attempted: `invariant_run`, `terminates_by`, the
  step and heap bounds, and `no_panic`.

## Measurements

| Part | Files | Lines (all lines, with code-only lines in brackets) |
|---|---|---|
| Semantics and the program | `Semantics.lean`, `CtEq.lean` | 308 [232], 71 [32] |
| Step lemmas | `StepLemmas.lean` | 161 [127] |
| Relation and helpers | `Relation.lean` | 393 [316] |
| let, alloc and load obligations | `Obligations.lean` | 225 [181] |
| store, free, terminators, `ok`, `lockstep` | `Lockstep.lean` | 367 [332] |
| Body points, labels, `lockstep_obs`, the final theorem | `CtTheorem.lean` | 252 [about 185] |
| Witnesses | `Witness.lean` | 45 [26] |
| Controls and audit | `Sabotage.lean`, `Audit.lean` | about 45 |

- **Size.** About 1,900 lines and 77 theorems.
  - The program-independent relational core is about 1,200 lines.
  - The part specific to this program is about 100 lines.
- **Build time.** A clean `lake build` takes 4.4 s of wall time on Apple
  silicon. The coordinator's re-run took 14 s, including Lake startup.
- **Kernel checking.** The largest file is `CtTheorem`, whose kernel
  type-checking takes 142 ms.

## Tactics used versus LEAN00 L3's planned subset

- **Within the subset, used heavily:** `intro`, `exact`, `apply`, `refine`,
  `rfl`, `rw`, `cases`, `induction`, `constructor`, `assumption`, `decide`
  and `simp only` with named lemmas.
- **Outside the subset, but bookkeeping only:** `have`, `obtain`/`rcases`,
  `by_cases`, `show`, `change`, `subst`, `unfold`, `exfalso`, `<;>` and
  `first`. They make up the bulk of the proof text but add no proof power.
- **Real extra automation:**

  | Tactic | Uses | Note |
  |---|---|---|
  | `↓reduceIte` simproc | 29 | |
  | bare `simp` | 4 | each has a short `rw` form |
  | `omega` | 2 | replaceable by `Nat` lemmas |
  | `decide +kernel` | 1 | |
  | `split` | 4 | |

- **For L3:** its subset should add the bookkeeping tactics and one
  `if`-reduction facility.

## Recommendations

**KRN-006**
- Adopt total `vars` and total ill-typed operators, with the type checker
  ruling both out.
- Settle the cost schedule.
- State that out-of-bounds is `stuck`, not a branch.
- Give allocations a shape (`size`, `alive`, `kind`, `secret`).
- Keep the allocation counter in the state.

**KRN-007**
- Port `Semantics`, `StepLemmas`, `Relation`, `Obligations`, `Lockstep` and
  `lockstep_obs` nearly as they are.
- Prove `call` as a lockstep step, which needs callee label tables.
- Re-estimate `warrant-lib` upward.

**KRN-012**
- For the relational case, emit data, not proofs: `pub`, `pubPtr` and `secPtr`
  per program point.
- Label inference must carry tag facts through copies.
- Before deciding on generated proofs for the unary side, test it in the same
  "obligation lemma plus decidable side condition" shape.
