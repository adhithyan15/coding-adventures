---
category: Workspace & package metadata
---

# Python downstream tests should not assert exact dependency versions

Assert minimum-compatible (`__version__ >= "0.3.0"`) or capability — exact-version asserts fail when a foundational package bumps and downstream gets force-rebuilt.
