### Changed — bound Windows corpus test contention without weakening gates

- Schedule one Vitest file worker on Windows. The measured host passed the
  chapter and grouped-owner assertions in isolation, but concurrent full-corpus
  readers crossed their existing 30-second budgets even with two workers.
  Test selection, assertions, timeouts, heap ceiling, and the 85% line-coverage
  threshold are unchanged. Other platforms retain their worker scheduling.
- Reuse the bounded asynchronous lesson-source loader for the chapter snapshot
  and the real-corpus loader guard, preserving their full-corpus assertions.
- Compare the global setup's exact real-track narration cache entry with a
  fresh narration of that same track. Multi-track fixtures and the full-corpus
  narration output check continue to exercise the public caller.
- Focused Windows cases pass; the complete coverage suite and final-head CI
  remain required before this change is accepted.
