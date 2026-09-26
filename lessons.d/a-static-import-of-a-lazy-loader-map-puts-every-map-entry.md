---
category: Repo policy / workflow reminders
---

# A static import of a lazy-loader map puts every map entry on first paint

language-ladder's `validate` job (`check:bundle`) failed on the six-track
vocabulary PR. The entry chunk was 528,322 bytes against a 500 kB budget.

The chunk held `narration-sources.ts`'s `import.meta.glob` map: one lazy loader
per narration chapter file, about 2,500 path strings. The module's own header
says nothing may import it statically, but `main.ts` did, so the map grew with
every chapter the corpus added. A 300-chapter tranche tipped it over.

The fix: `main.ts` reaches the module through `await import(...)`. The entry
chunk fell to 163,900 bytes.

When a corpus tranche is large, build language-ladder
(`npx vite build && node scripts/check-bundle.mjs`) before pushing. Grep for
static imports of `*-sources.ts` modules.
