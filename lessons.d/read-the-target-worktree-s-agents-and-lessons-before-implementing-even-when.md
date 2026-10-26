---
category: Repo policy / workflow reminders
---

# Read the target worktree's AGENTS and lessons before implementing even when the primary checkout hides them

The primary checkout's file inventory did not expose `AGENTS.md`, so a new
sibling worktree was created and implementation began before reading the
instructions that became visible there. Those instructions required reading
`CLAUDE.md`, `lessons.d/_meta.md`, and the relevant lesson categories before
implementation. The code still followed the major spec-first and branch rules,
but relying on another checkout's sparse view skipped the explicit preflight.

Immediately after creating a target worktree, discover and read instruction
files from that worktree itself before editing anything. Treat instructions in
the primary checkout as insufficient evidence about a differently materialized
tree.
