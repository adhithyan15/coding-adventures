---
category: Repo policy / workflow reminders
---

# Git merge-file returns a conflict count rather than a boolean status

A native18 preflight accepted only exit0/1. The compiler has three conflicts,
so that assertion stopped before a repository write. Preserve the partial
preflight and failing helper. Save actual exit/stdout/stderr before checking;
accept a nonnegative normal conflict count only when it equals independently
counted conflict markers, and reject tool errors. A wrapper failure is separate
from a compiler semantic red or a completed closed-merge proof.
