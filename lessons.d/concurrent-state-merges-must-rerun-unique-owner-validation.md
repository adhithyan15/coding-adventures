---
category: Repo policy / workflow reminders
---

# Concurrent state merges must rerun unique-owner validation

**What went wrong.** Two independently valid parity branches registered the
same backlog concept at different locations in the JSON item array. Git merged
both objects without a textual conflict, so the file still parsed even though
the semantic owner ID was duplicated.

**Fix.** Keep the newer decomposed aggregate, remove the superseded broad
record, and rerun uniqueness, dependency, cycle, and merged-edge validation on
the fully reconciled state.

**Do differently.**

- After every concurrent state merge, validate owner-ID uniqueness even when
  JSON parsing and conflict-marker checks pass.
- Reconcile duplicate concepts by preserving the more precise dependency
  decomposition and its children, not by renaming both copies.
- Do not publish a conflict-resolution head until semantic registry checks have
  passed after the merge commit.
