---
category: TypeScript / JavaScript
---

# Multi-pass validator pattern (collect all violations, throw once)

`validateConfig` / `validateManifest` / `validateStyleDocument` should aggregate every violation into a single error rather than throw on the first. Users want the full punch list; chasing errors one at a time is the slowest possible feedback loop. Pattern established in FM03 §2.4's `ConfigError` and adopted across FM02 (`ManifestError`), FM03 (`ConfigError`), FM04 (`StyleError`).
