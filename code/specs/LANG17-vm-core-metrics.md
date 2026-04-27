# LANG17 — vm-core Runtime Metrics Expansion

## Overview

`vm-core` (LANG02) today exposes a thin observation surface: the profiler
writes `observed_type` and `observation_count` onto each `IIRInstr`, and
`VMMetrics` exposes four aggregate counters (instructions executed,
frames pushed, JIT hits, per-function call counts).

The legacy `tetrad-vm` — which `vm-core` is intended to replace — exposes
a **much richer** surface used by `tetrad-jit` and by external analysis
tools: feedback slots with a V8 Ignition state machine, per-branch
taken/not-taken counters, per-back-edge loop iteration counts, and an
`execute_traced` path that records `VMTrace` records per instruction.

LANG17 brings `vm-core`'s surface up to parity without coupling it to
Tetrad specifics.  The additions are all in `vm-core` and
`interpreter-ir`; any language frontend (Tetrad, Lisp, BASIC, …) gets
them for free.  `tetrad-runtime` in turn re-exposes them through the
legacy `TetradVM` API shape so the two runtimes are indistinguishable to
callers.

This spec covers:

1. The **feedback slot state machine** (UNINIT → MONO → POLY → MEGA) and
   how it lives alongside the existing `observed_type` field on
   `IIRInstr`.
2. **Branch statistics** — taken / not-taken counters per conditional
   branch, keyed by `(fn_name, ip)`.
3. **Loop iteration counts** — back-edge hit counts per jump that
   targets an earlier instruction.
4. **Instruction tracing** — an opt-in `execute_traced` path that
   produces a list of `VMTrace` records.
5. The **public API** on `VMCore` and `VMMetrics` for consuming these.
6. How **`tetrad-runtime`** surfaces them through the legacy
   `TetradVM.feedback_vector(...)` / `.branch_profile(...)` / etc.
   shape.

---

## Why this is a separate spec

`vm-core` is already in production use by `tetrad-runtime` and will
soon be used by other frontends.  Adding the metrics surface without a
spec would:

- Silently change the semantics of `observed_type` / `observation_count`
  (the state machine is a strict refinement).
- Add multiple fields to `VMMetrics` that callers compose against.
- Add a new public method (`execute_traced`) with a non-trivial
  payload type (`VMTrace`).

These are all contract changes.  The spec pins down the data shapes and
the semantic rules so the rollout across frontends is painless.

---

## Feedback slot state machine

### Motivation

The current LANG01 observation schema on `IIRInstr` is:

```python
observed_type: str | None = None     # "u8", "str", …, "polymorphic", or None
observation_count: int = 0
```

This encodes **three** states (None / concrete / `"polymorphic"`).  The
V8 Ignition IC-style machine that Tetrad implements has **four**
(`UNINIT`, `MONO`, `POLY`, `MEGA`) and the distinction matters for JIT
decisions:

| State      | Meaning                         | JIT behaviour           |
|------------|---------------------------------|-------------------------|
| UNINIT     | Not yet reached                 | Wait for data           |
| MONO       | 1 type seen                     | Fast specialise         |
| POLY       | 2–4 distinct types seen         | Emit dispatch table     |
| MEGA       | ≥5 distinct types seen          | Give up specialisation  |

Without distinguishing POLY from MEGA, the JIT either over-specialises
(emits dispatch tables for megamorphic sites, wasting memory) or
under-specialises (bails out at 2 types, missing the 2–4-type sweet
spot).

### Schema additions on `IIRInstr`

LANG01's `IIRInstr` dataclass gains one optional, runtime-only field:

```python
from vm_core.metrics import SlotState

@dataclass
class IIRInstr:
    # ... existing fields unchanged ...

    observed_slot: SlotState | None = field(
        default=None, repr=False, compare=False
    )
```

`observed_slot` is `None` until the profiler samples this instruction
for the first time; thereafter it holds a live `SlotState` object the
profiler updates in place.

The legacy `observed_type` / `observation_count` fields stay as
**derived views** of `observed_slot`:

- `observed_type` = `observed_slot.observations[0]` if MONO, otherwise
  `"polymorphic"` for POLY/MEGA, else `None`.
- `observation_count` = `observed_slot.count`.

### `SlotState` definition

`SlotState` lives in `vm_core.metrics` (not in `interpreter-ir`, to
keep LANG01 free of runtime-state concerns):

```python
from enum import Enum
from dataclasses import dataclass, field

class SlotKind(Enum):
    UNINITIALIZED = "uninitialized"
    MONOMORPHIC   = "monomorphic"
    POLYMORPHIC   = "polymorphic"
    MEGAMORPHIC   = "megamorphic"

@dataclass
class SlotState:
    kind: SlotKind = SlotKind.UNINITIALIZED
    observations: list[str] = field(default_factory=list)
    count: int = 0

    def record(self, type_name: str) -> None:
        """Advance the state machine by one observation.

        State transitions:
            UNINIT       →  MONO        (first observation)
            MONO same    →  MONO        (repeat)
            MONO new     →  POLY        (2nd distinct type, count now 2)
            POLY 2..4    →  POLY        (new types in 3rd or 4th slot)
            POLY ≥5      →  MEGA        (5th distinct type — list is discarded
                                         to cap allocation)
            MEGA         →  MEGA        (never downgrades)
        """
```

The state machine is monotonic: once MEGA, always MEGA.  The
`observations` list is **capped at 4 entries** so MEGA sites do not
grow the list without bound.

### Slot-index mapping (frontend-owned)

Legacy Tetrad indexes feedback by slot number per function: the
compiler emits `ADD r, slot` instructions where `slot` is a per-function
index.  vm-core's model is per-instruction.  The bridge is a
**frontend-owned** side table on `IIRFunction`:

```python
@dataclass
class IIRFunction:
    # ... existing fields unchanged ...

    feedback_slots: dict[int, int] = field(default_factory=dict)
    """Optional: slot_index → instruction index within this function.

    Populated by frontends that allocate named slots (Tetrad, SpiderMonkey,
    V8).  Not interpreted by vm-core itself — used only by
    VMMetrics.feedback_vector() to return a list indexed by slot number.
    """
```

`tetrad-runtime`'s translator populates this dict as it walks the
CodeObject; frontends that don't need slot numbering (Lisp, BASIC)
leave it empty and consume per-instruction state directly.

---

## Branch statistics

### Schema

```python
@dataclass
class BranchStats:
    taken_count: int = 0
    not_taken_count: int = 0

    @property
    def taken_ratio(self) -> float:
        total = self.taken_count + self.not_taken_count
        return self.taken_count / total if total > 0 else 0.0
```

### Collection

`vm-core`'s existing `handle_jmp_if_true` and `handle_jmp_if_false`
opcode handlers gain a `_bump_branch_stats` call:

```python
def handle_jmp_if_true(vm, frame, instr):
    cond = frame.resolve(instr.srcs[0])
    _bump_branch(vm, frame, taken=bool(cond))
    if cond:
        frame.ip = frame.fn.label_index(str(instr.srcs[1]))
```

`_bump_branch` writes into `vm._metrics_branch_stats`, a
`dict[str, dict[int, BranchStats]]` keyed by `(fn_name, ip)` where `ip`
is the index **in the IIR instruction list of the currently-executing
function** — **not** the original Tetrad byte-code index.  (Translators
that need the original index can carry it via a
`source_map: list[tuple[int, int]]` side table on `IIRFunction`; see
"Source-map handling" below.)

Unconditional `jmp` is **not** counted: there's no branch decision.
Only the two conditional ops feed `branch_stats`.

### Overhead

One dict lookup + one integer increment per conditional branch.
Comparable to the profiler overhead; always-on.  A future optimisation
could gate it behind a `VMConfig.collect_branch_stats=True` flag, but
the default is always-on to match legacy Tetrad.

---

## Loop iteration counts

### Definition

A **back-edge** is any `jmp` (or taken `jmp_if_true`/`jmp_if_false`)
whose target instruction index is **strictly less** than the source
index — i.e., a jump that goes backward in the instruction list.

The dispatch loop detects back-edges at the opcode-handler level and
increments a counter keyed by `(fn_name, source_ip)`:

```python
def handle_jmp(vm, frame, instr):
    target = frame.fn.label_index(str(instr.srcs[0]))
    if target < frame.ip:
        _bump_loop_iteration(vm, frame)
    frame.ip = target
```

(Conditional branches also check, but only when the branch is actually
taken.)

### Collection state

`vm._metrics_loop_counts: dict[str, dict[int, int]]` — same nesting
shape as `branch_stats`.

### Semantic note

This is a **coarse** iteration count.  It counts individual back-edge
executions, which for a `while (cond) { … }` loop equals (iterations −
1) since the final exit is a non-taken branch.  Legacy `TetradVM` used
the same definition, so this is by design for parity; richer
loop-level counts (entries, exits, max-depth) are future work.

---

## Instruction tracing

### API

```python
class VMCore:
    def execute_traced(
        self,
        module: IIRModule,
        *,
        fn: str = "main",
        args: list[Any] | None = None,
    ) -> tuple[Any, list[VMTrace]]:
        """Execute with per-instruction trace recording.

        Returns (result, traces).  Overhead is one VMTrace allocation
        and one register-file snapshot per instruction — do not use
        for benchmarks or hot-path runs.  Intended for debuggers, test
        harnesses, and reproducer generation.
        """
```

### `VMTrace` schema

```python
@dataclass
class VMTrace:
    frame_depth: int
    fn_name: str
    ip: int
    instr: IIRInstr
    registers_before: list[Any]
    registers_after: list[Any]
    slot_delta: list[tuple[int, SlotState]]   # (instr_idx, new_state)
```

`slot_delta` lists the `IIRInstr`s whose `observed_slot` state changed
during this instruction's execution (almost always 0 or 1 entries — a
single instruction produces at most one observation).  Frontends that
care about slot indices (Tetrad) can re-key through
`fn.feedback_slots` when displaying.

### Implementation sketch

`execute_traced` sets a `self._tracer` attribute, runs the normal
dispatch loop, and the loop calls `self._tracer.observe(...)` after
each instruction when `_tracer is not None`.  The tracer accumulates
`VMTrace` records in a list that is returned alongside the result.

Recursion (function calls) naturally nests within the same tracer
because `_tracer` is a per-VMCore attribute, not per-frame.

---

## Public API additions on `VMCore`

```python
class VMCore:
    # ... existing methods unchanged ...

    # Hot-function helpers
    def hot_functions(self, threshold: int = 100) -> list[str]:
        """Return names of functions called at least ``threshold`` times."""

    # Feedback-slot introspection
    def feedback_vector(self, fn_name: str) -> list[SlotState]:
        """Return the feedback vector for ``fn_name``, indexed by slot.

        Requires ``fn.feedback_slots`` to be populated by the frontend.
        Returns an empty list if the function has no named slots or if
        it has never been called.
        """

    def slot_state(self, fn_name: str, slot_index: int) -> SlotState | None:
        """Return the SlotState for one slot in one function, or None."""

    # Branch introspection
    def branch_profile(self, fn_name: str, ip: int) -> BranchStats | None:
        """Return BranchStats for the conditional at instruction ``ip``."""

    # Loop introspection
    def loop_iterations(self, fn_name: str) -> dict[int, int]:
        """Return back-edge counts keyed by source instruction index."""

    # Lifetime reset (for REPL snapshot rollback)
    def reset_metrics(self) -> None:
        """Zero all accumulators and per-instruction observations."""
```

### Extended `VMMetrics`

```python
@dataclass
class VMMetrics:
    # ... existing fields unchanged ...

    branch_stats: dict[str, dict[int, BranchStats]] = field(default_factory=dict)
    loop_back_edge_counts: dict[str, dict[int, int]] = field(default_factory=dict)
```

`VMMetrics` remains an immutable snapshot of the running VM's state at
the moment `metrics()` is called.  The added fields are deep copies of
the live state so callers can mutate the snapshot without affecting
the running VM.

---

## Source-map handling

Tetrad's legacy metrics keyed branch stats by the **original Tetrad
byte-code IP**, not the IIR instruction index.  For users migrating
from `tetrad-vm` to `tetrad-runtime`, the two are different.

Rather than force vm-core to know about source-language byte-code
indices, we expose the **IIR index** in vm-core's metrics and let
frontends re-project through a side table on `IIRFunction`:

```python
@dataclass
class IIRFunction:
    # ... existing fields unchanged ...

    source_map: list[tuple[int, int, int]] = field(default_factory=list)
    """Optional: (iir_index, source_line, source_col) triples.

    For Tetrad: (iir_index, original_tetrad_ip, 0) is how tetrad-runtime
    populates it, so legacy callers can re-key branch_profile() / etc.
    by Tetrad IP.
    """
```

`tetrad-runtime` re-projects on the read side when surfacing
`.branch_profile(fn_name, tetrad_ip)` — keeping the exact legacy
signature — by walking `source_map` to find the corresponding IIR
index, then calling `vm.branch_profile(fn_name, iir_ip)`.

---

## `tetrad-runtime` surface (parity with legacy TetradVM)

`TetradRuntime` grows the following wrappers, each a thin re-projection
over vm-core's generic surface:

```python
class TetradRuntime:
    def hot_functions(self, threshold: int = 100) -> list[str]: ...

    def feedback_vector(self, fn_name: str) -> list[SlotState] | None: ...
    def type_profile(self, fn_name: str, slot: int) -> SlotState | None: ...
    def call_site_shape(self, fn_name: str, slot: int) -> SlotKind: ...

    def branch_profile(self, fn_name: str, tetrad_ip: int) -> BranchStats | None:
        """Look up branch stats by the *original Tetrad IP*.  Re-projects
        through IIRFunction.source_map to find the IIR index, then
        consults vm-core."""

    def loop_iterations(self, fn_name: str) -> dict[int, int]:
        """Return loop back-edge counts keyed by *Tetrad IP*."""

    def execute_traced(self, source: str) -> tuple[int, list[VMTrace]]: ...

    def reset_metrics(self) -> None: ...
```

The method signatures match `TetradVM` exactly so existing callers can
switch from `TetradVM` to `TetradRuntime` without code changes.

---

## Migration plan

The spec lands first (this document).  Implementation follows in four
small PRs:

1. **`SlotState` + state machine in vm-core**
   - Add `SlotState` / `SlotKind` to `vm_core.metrics`.
   - Add `observed_slot` field to `IIRInstr` (LANG01 schema bump).
   - Update `VMProfiler.observe` to advance the state machine.
   - Derive legacy `observed_type` / `observation_count` from
     `observed_slot`; existing tests unchanged.

2. **Branch & loop counters in vm-core**
   - Extend `VMMetrics` with `branch_stats` and `loop_back_edge_counts`.
   - Wire into `handle_jmp_if_true` / `handle_jmp_if_false` / `handle_jmp`.
   - New `VMCore.branch_profile()`, `.loop_iterations()`.

3. **`execute_traced` in vm-core**
   - Add `VMTrace` dataclass and `VMTracer` helper.
   - Add `VMCore.execute_traced(module, …) -> (result, list[VMTrace])`.
   - Opt-in; the normal `execute` path pays zero tracing overhead.

4. **Tetrad-runtime re-projection layer**
   - Populate `IIRFunction.feedback_slots` and `.source_map` in
     `code_object_to_iir`.
   - Add `TetradRuntime` wrappers with the legacy signatures.
   - Add parity tests that run the same program through both
     `TetradVM` and `TetradRuntime` and assert identical metrics.

Each PR is independently mergeable behind the others: PR 1 doesn't
affect branch / loop code paths; PR 2 doesn't depend on tracing; PR 3
is isolated; PR 4 only activates once PRs 1–3 are available.

---

## Out of scope

- **Function-call feedback slots** — Tetrad has a "call site shape"
  slot per CALL instruction.  The `observed_slot` machinery covers
  this; the `call_site_shape` wrapper on `TetradRuntime` just re-uses
  the generic slot API.  No separate ICache design.
- **Inlining hints** — JIT decisions about function inlining are
  LANG03's concern; the metrics feed into that but the decision
  policy is out of scope here.
- **Multi-threaded metrics** — vm-core is explicitly single-threaded.
  Concurrent metric collection is a future concern behind a
  `ThreadSafeVMCore` wrapper, not this spec.
- **Persistent metrics** — `VMMetrics` lives for the lifetime of a
  `VMCore`.  Persisting observations across processes (for offline
  JIT training) is an orthogonal concern and a future spec.
- **Rich loop analytics** — exit counts, max-depth, average-iteration,
  and similar are all derivable from back-edge data + branch data
  post-hoc; we do not compute them inline.

---

## Relationship to other specs

| Spec | Relationship |
|------|--------------|
| LANG01 (InterpreterIR) | LANG17 adds `observed_slot` (runtime-only field on `IIRInstr`) and `feedback_slots` / `source_map` (optional side tables on `IIRFunction`) |
| LANG02 (vm-core) | LANG17 extends `VMProfiler`, `VMMetrics`, and `VMCore`'s public API |
| LANG03 (jit-core) | jit-core will eventually read `observed_slot.kind` instead of `observed_type` to pick between MONO specialise and POLY dispatch; no change required now |
| LANG11 (jit-profiling-insights) | LANG17's richer observations make LANG11's suggestions more accurate; no schema change required |
| TET04 (tetrad-vm) | `tetrad-runtime` re-projects LANG17's surface to match `TetradVM`'s exact legacy signatures |

Once LANG17 lands and PR 4 in the migration plan ships, `tetrad-vm`
and `tetrad-jit` can be retired: every API they expose is now
available in vm-core + tetrad-runtime.
