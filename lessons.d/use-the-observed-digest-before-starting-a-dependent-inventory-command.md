---
category: CI & GitHub Actions
---

# Use the observed digest before starting a dependent inventory command

A read-only owned/main overlap inventory was started with a literal placeholder
SHA before the preceding tool result had been consumed. Its expected-digest
guard refused before creating an inventory or writing source/index state.
The correct retry used the actual observed virtual-merge receipt SHA, with the
refusal preserved rather than relabeled as acceptance. Keep dependency steps
sequential: read and validate the actual digest first, then construct the next
command from that observed value. Never execute a draft placeholder argument.

This filled CLI-generated draft is retained outside the verified published
checkout for installation at the next bounded source-carrier boundary; it does
not modify or invalidate the current published native/documentation receipts.
