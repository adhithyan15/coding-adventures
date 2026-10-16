---
category: Rust
---

# Reconcile ignored native lock versions before locked integration tests

After merging current main, the previously accepted ignored native Cargo.lock
still held earlier workspace package versions. Both locked integration commands
failed before compiling. Preserve those failed receipts and back up the lock
before reconciling it, then compare the complete registry package records.

The first offline metadata command updated workspace versions while preserving
every registry record, but then failed on an uncached Android-only package.
That is not metadata or test acceptance. Restrict metadata to the actual host
platform and rerun locked tests only after consuming the failed producer. A
partial metadata failure can still mutate a lock: hash its actual final bytes.
