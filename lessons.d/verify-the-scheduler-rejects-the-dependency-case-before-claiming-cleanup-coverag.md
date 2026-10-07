---
category: Testing & coverage
---

# Verify the scheduler rejects the dependency case before claiming cleanup coverage

The cleanup test used a missing dependency to imply a topological-order rejection. The scheduler deliberately ignores missing dependencies, so that scenario merely repeated the direct pass error. Register duplicate names and assert the actual `<duplicate>` pass identifier instead; its message does not contain the word duplicate. Independent review caught the coverage gap. When constructing a candidate Program in production, qualify its AST type if the previous Program import existed only inside the test module. Verify these code paths with real return values and compilation before reporting them covered.
