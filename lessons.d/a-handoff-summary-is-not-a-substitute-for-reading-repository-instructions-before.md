---
category: Repo policy / workflow reminders
---

# A handoff summary is not a substitute for reading repository instructions before the first patch

A resumed heartbeat supplied a detailed technical handoff, so I began the
Marathi A2 boundary audit and wrote its specification and red regression before
opening the repository's `AGENTS.md`, `CLAUDE.md`, and relevant `lessons.d`
entries.  The handoff accurately described the feature, but it did not carry
the repository's operational rules: specs-first policy, corpus-shard naming,
the suite entry point required to run a shard, generated-artifact ordering, and
the requirement to use the package's `BUILD` gate.

At the start of every fresh or resumed worktree turn, read the repository
instructions and `lessons.d/_meta.md` before the first edit.  Then read the
lessons for the file families being touched.  A conversation summary preserves
task state; it cannot replace the checkout's current working contract.

The repair was to stop, read the checkout-owned instructions in full, add the
missing corpus shard entry point, and rerun the required generation and gate
order before continuing.  Future resumptions must treat the handoff as a map
of unfinished work and the repository files as the authoritative procedure.
