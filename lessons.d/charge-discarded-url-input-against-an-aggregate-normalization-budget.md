---
category: Security boundaries
---

# Charge discarded URL input against an aggregate normalization budget

Node and output ceilings do not bound work that a renderer discards. A small
AST can alias the same large control-only or blocked-scheme destination many
times, forcing repeated normalization while emitting very little HTML. Charge
every raw destination visit against a per-render UTF-16 input budget before
allocating or scanning, even when it normalizes empty. Exercise links, images
and autolinks through the public renderer; use small alias witnesses, an
inclusive boundary, astral characters and per-call reset assertions rather
than timing a large denial-of-service workload.
