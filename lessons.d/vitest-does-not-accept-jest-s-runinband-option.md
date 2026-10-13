---
category: TypeScript / JavaScript
---

# Vitest does not accept Jest's runInBand option

I passed Jest's `--runInBand` flag through `npm test`, but this package runs
Vitest 4 and its CLI rejected the unknown option before collecting tests. Run
the package's declared `npm test` command unchanged for the full suite, or use
Vitest's documented pool and worker options only when concurrency really must
be constrained. Do not assume runner-specific flags transfer between Jest and
Vitest.
