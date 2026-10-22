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
  // The Kannada B1 core (chapters 309-318) teaches the independent vowel ಔ with
  // a letter lesson, KA-S175-letter-au, anchored on ಔಷಧ. Its row in
  // data/scripts/kannada.json is still recognition-only: validate.ts covers an
  // independent vowel in a headword only once its handwriting row is
  // source-backed, and no stroke order for ಔ has been taken from a source yet
  // (script-ductus has no geometry for it). So the anchor headword is one glyph
  // gap, held here until a cited row (and its ductus) lands. The letter lesson
  // promises no numbered strip, and the vocabulary builder refuses ಔ in every
  // other headword.
  expect(queue).toEqual([
    ["ಔ", 1],
  ]);
}
