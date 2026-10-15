---
category: Testing & coverage
---

# Font queries are cached across text nodes in a single paint scene

A Railroad resolver-trace test assumed one resolution per text node, but the
shared paint font cache resolves one query for all nodes with the same font.
The executed test reported one complete Courier New query instead of four.
Check the exact cached query and the complete ordered glyph outputs; do not
change production caching to satisfy an incorrect per-node call-count oracle.
