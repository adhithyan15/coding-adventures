---
category: Repo policy / workflow reminders
---

# Locate an implementation with rg files before guessing its module filename

A read-only chronology query guessed `src/typed_pipeline.rs`, which does not exist in closurec. `rg --files` identified the actual files; `run_typed_pipeline` and `TypedPipelineRun` live in `src/run.rs`. Enumerate paths first, then search the observed file or the real source directory. The failed query did not modify the frozen reviewed checkout. Carry this pending lesson into the next unfrozen documentation checkpoint.
