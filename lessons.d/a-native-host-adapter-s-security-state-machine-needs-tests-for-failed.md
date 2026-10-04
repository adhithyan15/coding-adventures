---
category: Testing & coverage
---

# A native host adapter's security state machine needs tests for failed, indeterminate, busy, and late settlements before the program coverage gate

Hardening the Forme desktop publisher added sticky poisoning, bounded active
settlement, known-failure retry, and late-response reconciliation branches.
The happy-path product test still passed, but the program's branch coverage
fell below its 80% gate. Add focused state-machine tests in the same change:
prove known pre-commit failures remain retryable, indeterminate outcomes never
reinvoke authority, concurrent admission returns busy, and disposal waits for
the active native settlement. Run the coverage command immediately after the
hardening edit rather than treating a passing unit-test command as equivalent.
