---
category: Testing & coverage
---

# Adversarial test helpers must preserve the malformed value under test

A result factory used `candidate ?? []`, so an explicit hostile `null` was
quietly replaced by the valid empty-array default. The test then exercised the
empty case while claiming to cover malformed input. When a test intentionally
injects invalid data, assign that field after creating the otherwise-valid
fixture, or distinguish `undefined` from every supplied value.
