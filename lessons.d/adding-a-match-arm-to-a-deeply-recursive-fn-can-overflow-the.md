# Adding a match arm to a deeply-recursive fn can overflow the stack (macOS CI only)

Symptom: `build (macos-latest)` failed while `windows`/`ubuntu` passed —
`logic-engine` test `deeply_nested_expression_is_a_clean_error_not_a_stack_overflow`
aborted with "has overflowed its stack, fatal runtime error: stack overflow"
(PR #7299, adding `ComputeOp::Abs`).

Cause: `logic-engine/src/compute.rs::eval` recurses up to `MAX_EVAL_DEPTH` (256)
levels. In **debug builds** (which `cargo test` / CI use) the compiler reserves
stack for ALL match arms' locals in the function's single frame — it does not
scope stack slots per-arm. Adding a new `ComputeExpr::Unary` arm with several
locals (`operand, dim, exact, result, …`) enlarged every one of the up-to-256
recursive `eval` frames. 256 × a fatter frame overflowed the macOS test thread's
~2 MB stack BEFORE the depth guard could return a clean `TooDeep`. Windows/ubuntu
have more headroom so they passed, and the local `cargo test` on macOS passed too
(margin is razor-thin and runner-dependent) — so it only surfaced on CI macOS.

Fix: move the new arm's body into a separate `#[inline(never)]` helper (e.g.
`eval_unary`) so its locals live in their own frame instead of bloating every
recursive `eval` frame — restoring `eval` to ~its pre-change size. Behavior is
identical; the deep-nesting test's guarantee ("clean `TooDeep`, never a stack
overflow") is preserved.

Rule: when adding a match arm to a function that RECURSES up to a large fixed
depth (`eval`, tree walkers, parsers with a depth cap), keep the arm's body in an
`#[inline(never)]` helper rather than inline — otherwise its locals multiply
across every recursive frame and can overflow a small (macOS 2 MB) test-thread
stack in debug builds. `cargo test -p <crate>` locally is NOT sufficient to catch
it (margin is runner-dependent); the guard is the deep-nesting stack test on CI
macOS.

Recurrence (PR #7343, adding binary `Min2`/`Max2`): the same overflow re-appeared,
and the FIRST two fixes made it WORSE — a cautionary tale about the mechanism:
  1. Adding a separate `if op == Min2 || Max2 { let result_dim…; let (x,y)…;
     let result…; let exact…; }` block BEFORE the general path DUPLICATED those
     locals (the general path has the identical set), so the frame grew by a full
     extra copy → still overflowed.
  2. Extracting the whole `Bin` arm into `#[inline(never)] fn eval_binary`
     (mirroring `eval_unary`) made it WORSE, not better: the `deeply_nested` test
     nests `Bin(Add, …)` 306 deep, so the recursion is `eval → eval_binary → eval
     → eval_binary → …` — **two** stack frames per nesting level instead of one.
     Even though each `eval` frame shrank, the total (`eval`+`eval_binary`) × 256
     exceeded the single inline frame × 256. Extraction only helps when the
     extracted helper is NOT on the deep-recursion path (e.g. `eval_unary` is fine
     because the test doesn't nest unary ops 256-deep).
The fix that WORKED: keep the `Bin` arm inline and FOLD min/max into the EXISTING
`result`/`exact` `match op { … }` arms (and map them through `dim_op` like
addition), adding ZERO new persistent locals — match arms share the frame's slots,
they don't each get their own. Frame(after) ≈ frame(main), so CI behaviour matches
the passing baseline.
Corrected rule: to add an op to a depth-capped recursive `eval`, FOLD it into the
existing match arms (reuse the shared locals); do NOT add a parallel `if`-block
(duplicates locals) and do NOT extract the recursive arm into a helper that then
sits ON the recursion path (doubles frame COUNT). `cargo test` locally cannot
verify the margin — local debug frames are so fat that even `main` overflows at
`RUST_MIN_STACK=5MB`, yet passes on CI at 2MB; the only reliable check is the
delta-vs-main (no new locals) plus the CI macOS run itself.

Discovered: 2026-07-02 during logic-engine abs-value CI (PR #7299 fix commit adf710c3).

---

### Java mini-sqlite Level 1 graduation — plan tree normalization required

`SqlPlanner.planSelect()` produces `Project(Sort(Limit(Distinct(core))))` — Project is the
outermost (last) node.  `SqlCodegen.compilePlan()` expects Sort/Limit/Distinct to be
OUTERMOST so it can peel them in its while-loop and then call `compileCore(Project(core))`.
If Project is outermost and Sort is inside it, `compileScanBody(Sort(...))` throws
"Unsupported plan node in scan body: Sort".

**Fix**: normalize the plan before calling `SqlCodegen.compile()`:

1. Peel Sort/Limit/Distinct from under Project (in the order they appear in the plan,
   i.e., outermost-first via `addLast`).
2. Rebuild: apply wrappers LAST-to-FIRST so Sort is outermost:
   `Project(Limit(Sort(core)))` → `Sort(Limit(Project(core)))`.
3. When Sort key columns are NOT in the SELECT list (e.g. `SELECT label FROM t ORDER BY rank`),
   inject them as extra hidden OutputColumn.Expr entries into the Project so SortResult can
   find them by name; strip those extra columns from the final QueryResult.

Additional mini-sqlite Level 1 lessons:

- **SqlPlanner passes `OutputColumn.Star` through unchanged.** `SqlCodegen.emitProjectColumns`
  silently skips Star columns ("the planner should have resolved them"), producing an empty
  result schema. Fix: expand `*` to explicit column references before planning by querying
  `backend.columns(table)` for each table in the FROM/JOIN list.
- **`INSERT INTO table VALUES (...)` with no column list** sends an empty columns list to
  `InsertRow`, causing the instruction to pop 0 values and store an empty row.  Fix: expand
  the column list from `backend.columns(table)` before planning.
- **`SqlCodegen.compileCore` only handles `Project(Aggregate)` directly.** If `Having` wraps
  the Aggregate (`Project(Having(Aggregate(...)))`), it falls through to
  `compileScanBody(Having(Aggregate))` which strips Having then throws "Unsupported: Aggregate".
  Fix: strip Having from between Project and Aggregate during normalization, save the Having
  predicate, and post-filter result rows using a simple expression evaluator after execution.
- **`NullOrder` default in SqlTextParser must be NULLS_FIRST for ASC** to match SqlVm's
  sort semantics (null rank = 0 = lowest, which makes NULLs sort first in ASC).  Using
  NULLS_LAST for both ASC and DESC (the initial incorrect default) puts NULLs last in ASC,
  violating the VM's sort-null-first-by-default contract.
- **Jacoco `excludes` on violationRules** does NOT exclude classes from measurement — it only
  applies to class-level rules.  To exclude old/unrelated classes from the COVEREDRATIO
  check, use `classDirectories.setFrom(files(...).map { fileTree(it) { exclude(...) }})` on
  the `jacocoTestCoverageVerification` task.

Discovered: 2026-07-01 during Java mini-sqlite Level 1 graduation (PR #7153).
