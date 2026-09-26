---
category: Rust
---

# Inserting code just above a Rust fn by matching its signature line steals the fn's doc comment

Twice in one session a scripted edit inserted a new item (a helper fn, a
`const`) by replacing the target function's signature line (for example
`fn emit_input_jsx(`) with "new item + signature". The target's `///` doc
comment sits *above* the signature, so the new item landed between the doc
comment and the function. The doc text silently transferred to the new item.
Clippy flagged the first case (`doc list item without indentation`) but not
the second, where a security reviewer spotted it.

**What to do:** when inserting before a documented item, anchor on the first
line of its doc block, not its signature. Walk back over consecutive `///`
lines, or anchor on a unique line above the doc block. After editing, check
that the line directly above each touched `fn`/`const` is still its own
doc comment.
