---
category: Testing & coverage
---

# Build dist before running checks that execute compiled CLIs

The human-language-data build and three `check:*` scripts were launched in
parallel. TypeScript compilation recreates `dist`, while each check immediately
executes a CLI from that directory, so all three checks raced the build and
reported `MODULE_NOT_FOUND`. Run `npm run build` to completion first; only then
may the shard, doc-shard, and assessment-artifact checks run in parallel.
