---
category: Python
---

# A standard-library state model can keep generated history tests replayable without a test dependency

The Python heap profile initially added Hypothesis to the package's test-only
dependencies for stateful model histories. The dependency policy treats a
package's optional test dependencies and tools installed by its BUILD recipe as
separate monotone inventories. That choice would therefore require a reviewed
name in each allowlist, plus the normal pinned-hash maintenance for Hypothesis
and its dependency.

The heap already had exhaustive short histories, longer fixed-seed histories,
and a byte-to-operation harness. I replaced only the stateful driver with a
small standard-library generator: each fixed seed produces 200 histories of
at most 100 operations, and the independent sorted-list model checks the full
heap state after every operation. A bounded reducer removes chunks and
simplifies operations/values; a deliberately faulty pop adapter verifies the
reducer still preserves and shortens a failing history. Failures report the
seed and replayable sequence.

While pinning the reducer regression, I found that trying all numeric
simplifications and accepting every still-failing candidate can undo a better
reduction later in the same pass. Compare a simple value metric before
accepting a replacement, then keep the regression pinned to the expected short
history.

When a test dependency triggers policy review, first ask whether an existing
standard-library generator and independent model can satisfy the same testing
goal. Keep the sample count, state transitions, replay information, and failure
reduction explicit. Do not describe deterministic finite histories as
universal proof or native coverage, and do not change the central allowlist to
hide a local test-tool choice.
