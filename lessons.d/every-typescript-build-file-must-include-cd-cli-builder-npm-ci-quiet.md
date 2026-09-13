---
category: BUILD files & dependency management
---

# Every TypeScript BUILD file must include `cd ../cli-builder && npm ci --quiet`

The build-tool validator checks for this as a required prerequisite ref (`missing prerequisite refs for standalone builds: typescript/cli-builder`). It is a toolchain dep that every TS package needs implicitly — add it immediately after `cd ../directed-graph && npm ci --quiet`. Surfaced in PR #7659 for sql-planner, sql-optimizer, sql-codegen, sql-vm, mini-sqlite.
