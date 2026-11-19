---
category: Testing & coverage
---

# A conformance consumer must assert the exact mapped error type

When a shared fixture expects a specific validation error, passing that expected
class to `pytest.raises` is not enough if it is a base class. An index error can
then satisfy a fixture case that requires invalid-range precedence. Capture the
exception and assert its exact class. Also pass invalid fixture values through
the package's public API; adapter-side checks only prove that the adapter can
recognize invalid data, not that the implementation rejects it correctly.
