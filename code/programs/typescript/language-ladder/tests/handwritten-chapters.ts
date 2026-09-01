// The chapters the book generator deliberately skips, read from the ledger.
//
// `bookhashes.test.ts` enumerates the generated-book manifest to decide what to
// check. That is one-way: a chapter which DISAPPEARS from the manifest simply
// stops being checked, and the suite stays green. The app legitimately teaches
// chapters the manifest does not cover -- the hand-written ones -- so the honest
// invariant is that the difference between the two is EXACTLY that set.
//
// This is derived from `core/book-generation.d/handwritten.d/`, one owner file
// per hand-written chapter, so it needs no edit as chapters are retired into the
// generated pipeline. Deriving it from a tree the generator does not write also
// keeps the check from becoming a comparison of the manifest with itself.
//
// The environment is jsdom, so this uses the same build-time `import.meta.glob`
// the lesson loader uses rather than node's fs.
const OWNERS = import.meta.glob<{ language: string; chapter: number }>(
  "../../../../learning/human-languages/core/book-generation.d/handwritten.d/*.json",
  { eager: true, import: "default" },
);

/** `<language>/<chapter>` for every hand-written chapter, sorted. */
export const HANDWRITTEN_CHAPTERS: string[] = Object.entries(OWNERS)
  .filter(([path]) => !path.endsWith("_meta.json"))
  .map(([, owner]) => `${owner.language}/${owner.chapter}`)
  .sort();
