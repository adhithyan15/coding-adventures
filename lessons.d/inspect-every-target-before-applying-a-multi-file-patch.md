---
category: Repo policy / workflow reminders
---

# Inspect every target before applying a multi-file patch

A multi-file implementation patch inspected the Rust insertion point but
guessed the README's existing line wrap, so exact verification rejected the
entire patch. Before one patch touches several files, inspect each target's
current context; otherwise split the edits so one stale context cannot block
all of them.
