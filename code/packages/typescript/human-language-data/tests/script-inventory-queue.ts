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
  // The Malayalam B1 core (chapters 339-348) teaches the independent vowels
  // ഈ ഐ ഔ ഋ with letter lessons anchored on ഈയിടെ, ഐക്യം, ഔദ്യോഗികമായ and
  // ഋതു. Their rows in data/scripts/malayalam.json are recognition-only:
  // validate.ts covers an independent vowel in a headword only once its
  // handwriting row is source-backed and font-checked, and script-ductus has
  // no ductus for these four yet, so no stroke order is claimed for them. Each
  // anchor headword is therefore one glyph gap, held here until a cited row
  // (and its script-ductus ductus) lands. No letter lesson promises a numbered
  // strip for them.
  expect(queue).toEqual([
    ["ഈ", 1],
    ["ഋ", 1],
    ["ഐ", 1],
    ["ഔ", 1],
  ]);
}
