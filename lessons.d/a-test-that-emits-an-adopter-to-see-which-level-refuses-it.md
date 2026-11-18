---
category: Python
---

# A test that emits an adopter to see which level refuses it becomes a full enumeration the day the level is evaluated

`test_later_adopters_report_not_live_by_level` called `emit(adopter_bytes(name))` for wasm-leb128, logic-gates,
lzw, arithmetic and x25519 and asserted how many suites came back `not-live` at which level. While
`ir.fixed` was refused that was instant: every suite that needed it was rejected before a single case ran.
The moment the evaluator learned `ir.fixed` (CV2-S10c-1) the same call became a *real* enumeration of every
suite, wasm-leb128 alone being 46 million cases, and the whole test run went from two minutes to more than
ten before anyone noticed which test was hanging.

Fix: the test asks the question it means, "which level blocks each suite", through `emit.census`, which
parses and lowers and enumerates nothing. A test that wants to prove a document *evaluates* picks the
cheap suites on purpose (`only=` / `SuitePlan` of one claim) or measures the whole derive once, offline
(`tests/test_fixed_lzw.py` records that lzw's takes about 21 seconds and derives only what is cheap).
When a slice teaches a tool a new level, grep the tests for every call that used to end in a refusal:
each one is now a run.
