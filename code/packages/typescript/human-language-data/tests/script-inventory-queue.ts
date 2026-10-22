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
  // The Tamil B1 core (chapters 284-293) teaches the independent vowel ஈ with a
  // letter lesson, TA-S141-letter-ii, anchored on ஈரம். data/scripts/tamil.d
  // has no ஈ row: every row there is source-backed and owned by an evidence
  // module, and no stroke order for ஈ has been taken from a source yet
  // (script-ductus has no geometry for it). So the anchor headword is one
  // glyph gap, held here until a cited row (and its ductus) lands. The letter
  // lesson promises no numbered strip, and the vocabulary builder refuses ஈ in
  // every other headword.
  expect(queue).toEqual([
    ["ஈ", 1],
  ]);
}
