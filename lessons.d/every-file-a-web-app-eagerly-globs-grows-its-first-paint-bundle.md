---
category: TypeScript / JavaScript
---

# Every file a web app eagerly globs grows its first-paint bundle, so a generator that adds hundreds of such files must be checked against check:bundle

HL-C443 generated about 370 stroke-order filmstrip SVGs into
`<track>/book/figures/`. Every data, book and app test passed locally. CI's
`validate` job then failed the language-ladder bundle gate:

```
bundle check: largest eager chunk is 507112 bytes (limit 500000)
```

Two facts combined to cause it:

- **The app globs every book figure eagerly.** `src/figures.ts` runs
  `import.meta.glob("…/*/book/figures/*.svg", { eager: true, query: "?url" })`,
  so every figure file, used or not, adds an entry to the first-paint chunk.
  Small ones are also inlined as data URLs.
- **The app suite never checks the bundle.** `npx vitest run` does not run
  `check:bundle`; the BUILD file runs it after `npm run build`.

The fix narrowed the glob to what the app can use. The new files are
book-only: no lesson's Markdown references a filmstrip. So the glob excludes
`*-filmstrip.svg`. The one lesson that had placed a filmstrip by hand now gets
it from the book generator instead, so the exclusion stays true.

**What to do differently:** when a change adds files under a directory some app
globs, whether figures, fonts or lesson data, run the app's whole BUILD
sequence, not only its tests. That means `npm run build && npm run
check:bundle`. The bundle gate only measures a fresh build (it refuses a stale
`dist/`), so reproduce first and compare the number before and after.
