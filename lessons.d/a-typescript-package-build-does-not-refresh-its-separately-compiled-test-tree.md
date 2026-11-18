---
category: TypeScript / JavaScript
---

# A TypeScript package build does not refresh its separately compiled test tree

For packages with separate `tsconfig.json` and `tsconfig.test.json`, `npm run build` compiles the production output but does not necessarily rebuild `dist-test`. If validating a source mutation or editing tests and invoking `node --test` directly against `dist-test`, first run `node node_modules/typescript/bin/tsc -p tsconfig.test.json`; otherwise the result can come from stale compiled tests/source and give a false pass. The package's normal `npm test` script already performs this test compilation. In `dsp-filters`, an old-reciprocal mutation appeared to pass when the production build was refreshed alone; recompiling `dist-test` made the intended public regression fail with `Infinity` instead of `1`.
