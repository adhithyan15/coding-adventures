---
category: Testing & coverage
---

# Resolve the complete commit hash before invoking a frozen-source runner

The D3b successor red runner was invoked with an invented suffix after the known
a522cb43 prefix. Its exact-source guard correctly refused before creating any
receipt or starting Cargo. Resolve git rev-parse HEAD directly and pass that
complete value; never reconstruct a commit identity from a remembered prefix.
The refusal provides no compile or semantic-test evidence.
