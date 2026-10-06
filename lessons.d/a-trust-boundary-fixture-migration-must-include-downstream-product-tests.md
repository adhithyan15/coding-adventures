---
category: Security boundaries
---

# A trust-boundary fixture migration must include downstream product tests

Moving a native sandbox's acceptance fixture from a shared CI toolcache into a
private trusted tree fixed that package's gate, but a downstream CLI product
test still configured the original toolcache root. The native launcher
correctly rejected it, while the product layer could report only that the
readiness pipe closed without an attestation. A package-local green test was
therefore not evidence that every product acceptance consumer had crossed the
new trust boundary.

When a security check changes which host resources are acceptable, search the
entire dependency cone for tests that discover or inject the same resource.
Each end-to-end consumer should prove the shared root is rejected with the
production verifier, copy the quiescent runner-controlled distribution without
following links or junctions, verify the finished private tree, and only then
launch through the product path. Keep cleanup suite-scoped and leave the shared
toolcache untouched. This preserves the strict production policy while making
the product test's fixture assumptions explicit instead of hiding them behind
a generic child-startup failure.
