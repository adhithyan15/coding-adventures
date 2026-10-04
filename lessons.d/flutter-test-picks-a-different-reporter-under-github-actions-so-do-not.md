---
category: CI & GitHub Actions
---

# flutter test picks a different reporter under GitHub Actions, so do not match its summary text without naming the reporter

**What went wrong.** `mosaic-emit-flutter/tests/flutter_dialog_host.rs`
(#16637) runs `flutter test` and, besides the exit status, asserted the
output contained "All tests passed" -- the expanded reporter's summary,
which it prints on a terminal and in a plain shell. Under GitHub Actions
(`GITHUB_ACTIONS=true`) `flutter test` picks a different reporter, whose
summary is "🎉 9 tests passed.", so CI failed with every widget test green.

**Fix.** Name the reporter: `flutter test --reporter expanded` prints the
same summary everywhere. Reproduced locally with
`GITHUB_ACTIONS=true cargo test --test flutter_dialog_host` before and after.

**Do differently.** When a harness parses a tool's human-readable output,
pin the output format with a flag, and run it once with `GITHUB_ACTIONS=true`
(and `CI=true`) locally: many tools (Flutter, Gradle, npm test runners) change
their output when they detect CI. Prefer the exit status, or a
machine-readable reporter (`--reporter json`), over matching prose.
