---
category: TypeScript / JavaScript
---

# Run real TypeScript CLI fixtures against emitted JavaScript when loader startup consumes their process budget

**Context:** TypeScript build-tool, PR #19644, Windows job 134970964616 and its one authorized failed-job rerun 135008986193 (run 44351985766, attempt 2).

**What happened:** The duplicate-identity and invalid-UTF-8 fixture tests each started a fresh `node.exe` process through `tsx` with a 5,000 ms fixture wall-time limit. Both failed with `spawnSync ETIMEDOUT` on the first run; only the invalid-UTF-8 fixture repeated on the bounded rerun. The same package had passed on an earlier Windows run, so these receipts do not establish a deterministic production failure or its CPU-level cause.

**Fix:** Compile the production entry point once per Vitest run using the locked TypeScript compiler, then keep each fixture's fresh Node process, exact arguments, 5,000 ms process-tree bound, exit-code assertion, and byte-exact diagnostic assertion. Copy and byte-compare the module-relative source registry beside emitted `hasher.js`, and retain package-relative dependency lookup for `createRequire(import.meta.url)`. Bound compilation separately and remove only that run's owned output on setup failure or teardown.

**Generalisation:** When a CLI fixture measures a real executable's startup and work, avoid adding a TypeScript loader to every process if the package can exercise the same production code as emitted JavaScript. Do not enlarge the fixture budget to mask startup overhead, and do not infer a stable production defect from an intermittent timeout alone.
