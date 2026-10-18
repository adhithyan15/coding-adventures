---
category: Testing & coverage
---

# Regular expression count tuples must be compared with tuples in validation gates

The first full seeded-painter validation executed167P0F0I/zero filters and all24
complete markers, but its wrapper failed before persisting the command row:
re.findall with capture groups returns tuples, so comparing c[1:] with a list
always failed. Preserve the original receipt/log as wrapper-refused, not an
accepted full run. Compare against a tuple, verify both zero and nonzero control
rows, and persist the actual command result before secondary census checks so
an orchestration error cannot discard execution evidence. Run a fresh successor
validation with its own frozen source/helper/receipt; do not rewrite the failed
receipt or invent a successful strict step that never ran.

The native31 v2 red wrapper repeated this tuple/list mistake after a real
compiled1P4F/342complete refusals/12guard controls. Its failed receipt remains
unaccepted. Preflight of the separate v3 also replayed the actual Rust1.99
panic format, whose optional numeric thread ID precedes panicked. The corrected
helper used a tuple and explicit optional-ID pattern; its fresh actual run
completed the exact census and independent executable/input snapshot. This is
compiled regression evidence, not successful positive VM execution or a green
implementation. Keep command rows durable before every secondary parser gate.
