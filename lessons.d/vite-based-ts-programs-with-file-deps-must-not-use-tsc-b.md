---
category: Workspace & package metadata
---

# Vite-based TS programs with `file:` deps must NOT use `tsc -b` in build script

`tsc -b` follows imports into nested `node_modules` (npm copies, not symlinks on Windows) and fails on un-installed transitives. Use plain `vite build`; type-check via vitest.
