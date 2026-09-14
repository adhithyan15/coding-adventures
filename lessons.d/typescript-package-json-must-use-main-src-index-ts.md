---
category: Workspace & package metadata
---

# TypeScript `package.json` must use `"main": "src/index.ts"`

(not `dist/index.js`) because Vitest resolves `file:` deps via `main` and we don't pre-compile. Also: `"type": "module"`, `@vitest/coverage-v8` in devDeps, run real coverage gate locally before pushing. Never commit `.js`/`.d.ts` transpile outputs alongside `.ts` sources.
