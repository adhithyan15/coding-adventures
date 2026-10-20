---
category: CI & GitHub Actions
---

# Focused language tests do not validate new changelog shard names

The Spanish mock-stem repair passed its 42 focused tests and TypeScript check,
but its first PR run failed the repo-wide metadata job before any package
build: the new changelog fragment had a plausible slug and an issue number,
not the required eight-hex digest of its exact heading. The filename rule was
already documented; the missed step was running the metadata gate before the
first push. For a new `CHANGELOG.d` fragment, compute its name with `docSlug`
and `headingDigest`, then run `check:doc-shards` alongside focused tests.
