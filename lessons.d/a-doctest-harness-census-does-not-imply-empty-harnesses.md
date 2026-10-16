---
category: Rust
---

# A doctest harness census does not imply empty harnesses

The current-main58 full suite passed2009 assertions across56 executable and15
doctest harnesses, but the checkpoint incorrectly called all15 doctests empty.
Independent per-harness audit showed diagram_layout_graph and dot_parser each
execute one passing doctest; only13 are empty. The actual aggregate is2007
executable-harness passes plus2 doctest passes, still0 failed/ignored/measured/
filtered. Preserve original receipts and the audit's failed empty-harness claim.

Use exact per-runtime-block summary vectors when reporting harness kinds. Correct
backlog/issues and checkpoints without relabeling a new documentation carrier as
the source that executed the tests. Harness count alone says nothing about the
number of tests in a harness.
