---
category: Testing & coverage
---

# A roadmap transition must update the CI contract that pins its active item

FM-B074 was completed and FM-B075 was activated in the Forme roadmap, but the
Windows release lane still expected FM-B074 to be active. Package tests and
the newly added subtype checks all passed because the stale assertion lived in
the repository-level spec-map suite, not in either changed package.

When a living roadmap moves an item from `active` to `done`, search for every
repository contract that names the old item and run the roadmap/spec-map suite
alongside package validation. Pin both the completed item and the newly active
item so the contract verifies the transition rather than merely deleting a
stale assertion.
