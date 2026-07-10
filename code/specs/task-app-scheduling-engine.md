# task-app — Scheduling Engine

> Part of the [task-app spec series](task-app-overview.md). Defines how `task-core` turns the stored
> inputs from [`task-app-data-model.md`](task-app-data-model.md) into a schedule: early/late dates,
> slack, the critical path, summary rollups, and (via the enhanced constraint VM) resource leveling.

## Principle: the Critical Path Method *is* a forward/backward pass

Microsoft Project's scheduling semantics are, at their core, the classical **Critical Path Method
(CPM)**. That is a linear-time graph algorithm, and MS-Project behavior is a near-direct
transcription of it. So `task-core` computes the schedule directly — it does **not** hand the
problem to a general solver. (The constraint VM is used only for the genuinely optimization-shaped
problems: resource leveling and feasibility checking — see below and
[`task-app-constraint-vm-enhancement.md`](task-app-constraint-vm-enhancement.md).)

The dependency network is loaded into the existing **`directed-graph`** crate; we reuse
`topological_sort`, `has_cycle`, `predecessors`, `successors`, and `affected_nodes`. We do **not**
write a new graph.

## Inputs and outputs

- **Inputs** (stored): tasks with `TaskSchedule` (duration, work, type, constraint, deadline,
  calendar), `DependencyLink`s (FS/SS/FF/SF + lag), calendars, assignments, and the project start date.
- **Output**: a `ScheduledDates` per task (early/late start & finish, scheduled start & finish after
  constraints, total & free slack, `critical`), cached in `schedule_cache` and recomputed on change.

## Working time is the unit of measure

All duration/lag/work math is in **working minutes**, resolved against calendars — never raw
wall-clock. Three calendar primitives (new code in `task-core`, built on `datetime-core`) underpin
everything:

```rust
fn is_working(cal, at: DateTime) -> bool;
fn add_working(cal, from: DateTime, minutes: i64) -> DateTime;   // walk forward/back over working intervals
fn working_between(cal, a: DateTime, b: DateTime) -> i64;        // count working minutes in [a,b)
```

Calendar resolution order per task: **task calendar → (driving resource calendar) → project
calendar → base**. `add_working` skips non-working days/holidays and honors intra-day intervals
(e.g. 09:00–12:00, 13:00–17:00). Elapsed durations (`Duration.elapsed = true`) bypass the calendar
(wall-clock), for things like "wait 24h for paint to dry."

## Stage 1 — Topological order & cycle check

```
graph = directed-graph from dependencies (node = TaskId, edge = predecessor → successor)
if graph.has_cycle(): return SchedulingError::Cycle(path)   // surfaced to UI, never a panic
order = graph.topological_sort()
```

Summary tasks are excluded from the network (they are rolled up, not scheduled — Stage 4);
milestones participate with zero duration.

## Stage 2 — Forward pass (early dates)

Process tasks in topological order. A task's Early Start is the latest constraint imposed by all its
incoming dependencies; Early Finish adds the calendar-walked duration. Per predecessor link type:

| Link | Constraint on successor |
|---|---|
| **FS** (finish→start) | `ES_succ ≥ EF_pred + lag` |
| **SS** (start→start) | `ES_succ ≥ ES_pred + lag` |
| **FF** (finish→finish) | `EF_succ ≥ EF_pred + lag` |
| **SF** (start→finish) | `EF_succ ≥ ES_pred + lag` |

```
ES = max(project_start, max over preds of the link constraint above)
EF = add_working(task.calendar, ES, task.duration.working_minutes)
```

`lag` is added via `add_working` (or raw if the lag is elapsed). A task with no predecessors starts
at the project start date (or its own constraint floor).

## Stage 3 — Constraints (may override dependencies)

After the raw early dates, apply the task's `Constraint`, honoring MS-Project rigidity precedence
(inflexible > semi-flexible > flexible):

| Constraint | Effect on scheduled dates |
|---|---|
| `Asap` | scheduled = early dates (default) |
| `Alap` | scheduled = late dates (computed in Stage 5) |
| `StartNoEarlierThan(d)` | `scheduled_start = max(ES, d)` |
| `StartNoLaterThan(d)` | cap; flag conflict if `ES > d` |
| `FinishNoEarlierThan(d)` | `scheduled_finish = max(EF, d)` |
| `FinishNoLaterThan(d)` | cap; flag conflict if `EF > d` |
| `MustStartOn(d)` | `scheduled_start = d` (overrides predecessors) |
| `MustFinishOn(d)` | `scheduled_finish = d` (overrides predecessors) |

Inflexible constraints (`MustStartOn/On`) pin the date even against a predecessor; when that
contradicts a dependency, we record a **conflict** (surfaced in the UI) rather than silently
choosing one — matching MS-Project's "planning wizard" warning behavior. `deadline` is *not* a
constraint: it never moves a date; it only sets a `deadline_missed` flag when `scheduled_finish >
deadline`.

## Stage 4 — Summary rollups

Walk the WBS bottom-up (reverse of the outline order): a Summary task's start = min(child starts),
finish = max(child finishes), work = Σ child work, cost = Σ child cost, percent-complete =
work-weighted mean of children. Summaries are read-only in the scheduler.

## Stage 5 — Backward pass (late dates, slack, critical path)

Process in **reverse** topological order from the project finish (or an imposed
`FinishNoLaterThan`/deadline horizon):

```
LF = min(project_finish, min over succs of the mirror-image link constraint)
LS = sub_working(task.calendar, LF, task.duration.working_minutes)
total_slack = working_between(cal, ES, LS)      // == LS − ES in working minutes
free_slack  = min over succs of (ES_succ − EF_this) honoring link type
critical    = total_slack <= 0
```

The **critical path** is the connected chain of `critical` tasks from a start node to the project
finish. Because CPM yields early, late, *and* slack in two linear passes, all of it comes "for free"
— which is precisely why a hand-written pass beats routing this through a general solver (the
constraint VM can decide feasibility but cannot produce slack or the critical path; see its spec).

## Stage 6 — Effort-driven recompute (the scheduling triangle)

When assignments or an input change, reconcile `Work = Duration × Units` per the task's `TaskType`
(see the data-model table): `FixedUnits` recomputes duration, `FixedDuration` recomputes units,
`FixedWork` recomputes duration and forces effort-driven. This runs before Stage 2 (it changes
durations) and is the reason assignment edits trigger a full reschedule.

## Incremental recomputation

A naive reschedule is already linear, but for large plans we recompute only what changed:
`directed-graph.affected_nodes(changed_set)` returns the transitive successors that need new dates.
Formula/rollup field recomputation uses the same crate over the field-dependency graph (see
[`task-app-formula-fields.md`](task-app-formula-fields.md)). The reducer tags each `TaskCommand`
with what it invalidates (schedule cache, formula cache, or both) so the facade recomputes minimally.

## Resource leveling & makespan optimization (the optimization layer)

CPM assumes infinite resources. **Resource leveling** resolves over-allocation (a resource assigned
>`max_units` at an instant) by delaying tasks within their slack — an *optimization* problem
(minimize makespan / total delay subject to capacity), not a satisfiability one. This is where the
**enhanced constraint VM** is used:

- Encode task `start`/`finish` as integer variables, dependencies + constraints as linear
  inequalities (`start_B ≥ finish_A + lag`, `MustStartOn` as equalities), and resource capacity as
  per-time-window sums.
- Add an **objective** (`minimize` project finish, or `minimize` total start-delay from the CPM
  dates) — the new capability specified in
  [`task-app-constraint-vm-enhancement.md`](task-app-constraint-vm-enhancement.md).
- Solve for a leveled schedule; fall back to a deterministic **priority-based serial leveler**
  (order by CPM start, then slack, then priority; greedily place respecting capacity) when the
  network is too large for the solver or the solver returns `Unknown`.

Leveling is opt-in per project and runs *after* CPM; the un-leveled CPM schedule (with its true
critical path) always remains available.

## Feasibility validation

Independently of leveling, the constraint VM's `check_sat` detects **over-constrained** plans that
CPM would silently fudge: `MustStartOn` vs. a predecessor, `FinishNoLaterThan` earlier than the
earliest possible finish, negative-lag loops. The facade surfaces these as conflicts with the
offending task/link ids, so the UI can explain *why* a date can't be met.

## Determinism & testing

The engine is pure and deterministic (calendar math is integer working-minutes; ties broken by
outline order). Tests live in `task-core` and assert against **hand-computed** early/late/float on
canonical CPM networks from the literature, plus: each of FS/SS/FF/SF with positive and negative
lag; each of the 8 constraints including conflict cases; multi-interval and holiday calendars;
summary rollups; effort-driven recompute for all three task types; and leveling on small
known-optimal instances (solver path vs. serial-leveler path agree on makespan where the instance is
small enough to be provably optimal).
