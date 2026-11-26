---
category: Testing & coverage
---

# Keep the real-strace dying-call refusal oracle tied to a parsed process exit

- `TestSandboxStraceDyingThreads` now recognizes only the observed bare-unknown
  resumed `openat` refusal when the full trace contains a later parsed terminal
  exit for that same PID without an intervening PID-generation ambiguity. The
  production trace readers continue to refuse the incomplete trace.
- Platform-neutral tests exercise the exact predicate and fail-closed cases,
  including wrong or missing PID exits, reused PIDs, trailing text, other calls,
  and invalid terminal records. The Linux integration test keeps its 20 child
  iterations, stderr check, and reader agreement.
