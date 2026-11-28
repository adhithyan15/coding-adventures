---
category: Python
---

# Mypy cannot see a dynamically imported test dependency

Mypy rejected tests that accessed a module's imported `platform` attribute directly, even though runtime monkeypatching worked. Use pytest's dotted-string target to patch a test-only dependency; keep the assertion that the production CLI never calls the host-probing methods, and let Ruff check the test helper too.
