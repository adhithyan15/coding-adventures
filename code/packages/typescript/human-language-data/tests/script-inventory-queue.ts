import { expect } from "vitest";
import type { ScriptEvidenceContext } from "./script-inventories/helpers.js";

/**
 * Pin the corpus-wide glyph-gap queue outside every script-owned evidence module.
 *
 * A non-empty queue is still allowed by the validator as warning-level debt, so
 * this exact expectation is the regression gate that makes a new queue head an
 * intentional, owner-neutral update rather than work for an unrelated script.
 */
export function assertCorpusGlyphGapQueue({
  affected,
}: ScriptEvidenceContext): void {
  const queue = [...affected.entries()].sort(
    ([leftGlyph, leftCount], [rightGlyph, rightCount]) =>
      rightCount - leftCount || leftGlyph.localeCompare(rightGlyph),
  );
  // The Telugu B1 core (chapters 316-325) teaches the independent vowels ఈ ఓ ఊ
  // ఔ with letter lessons anchored on ఈలోగా, ఓటు, ఊరు and ఔషధం. Their rows in
  // data/scripts/telugu.json are still recognition-only: validate.ts covers an
  // independent vowel in a headword only once its handwriting row is
  // source-backed and font-checked, and no stroke order for these four has been
  // taken from a source yet. So each anchor headword is one glyph gap, held
  // here until a cited row (and its script-ductus ductus) lands. No letter
  // lesson promises a numbered strip for them.
  expect(queue).toEqual([
    ["ఈ", 1],
    ["ఊ", 1],
    ["ఓ", 1],
    ["ఔ", 1],
  ]);
}
