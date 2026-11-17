# 0.2.1 (CV2-S3b review fixes)

## Fixed

- **H1, stack overflow inside the documented limits.** The evaluator (`eval`, `trace`,
  `static_calls`) is now iterative, with an explicit heap work stack (`MAX_EVAL_TASKS`) and a
  64-frame model-call bound (`MAX_MODEL_FRAMES`); the `#[inline(never)]` workaround is gone.
  64 model frames of deep expressions now evaluate in a debug build on a default 2 MiB test
  thread (`tests/limits.rs`; a recursive evaluator aborted at 8 frames).
- **M1, generation missed pure failures.** `trace` evaluates every pure node that may wrap or
  fail (`Program::may_fail_or_wrap`: any `add sub mul neg`, any `concat`, and models whose
  closure has them), so an exact `Nat`/`Int` leaving the 2^127 window or a `concat` past
  `MAX_IR_VALUE` in a subject argument is `W0701` at generation.
- **M3.** `enumerate_document` resolves and charges every suite (IR work, ordinals) before it
  plans any, then plans, runs and drops one suite at a time.
- **L2.** `stream::encode` rejects a fixed-width value outside its type and a `Bytes<N>` of
  another length. **L3.** a zero product is refused before allocation.
- `slim_stride` is clamped per suite to its automatic stride, never an error (owner decision).

## Changed

- **L1.** `Sabotage`, `Program::with_sabotage`, `with_max_tasks` and `sabotage_swap` exist only
  with the `test-hooks` feature (or in unit tests); the crate's integration tests enable it
  through a self dev-dependency.
- Third enumeration parity document `enum_probe.warrant` (13 suites over concat/take/drop, the
  connectives, ordering, exact `Nat`/`Int`, U16/U32/U64 wraps, literals, a model call in `admit`,
  wrapping pure guards) with its oracle snapshot.
