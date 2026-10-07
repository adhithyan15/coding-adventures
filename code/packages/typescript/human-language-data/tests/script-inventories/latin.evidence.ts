// Exact real-corpus evidence owned by the Latin inventory.
// See HL24: unrelated script authors must not share an executable edit surface.
//
// The Latin inventory is the six Latin-script tracks' OWN character set: every
// Latin character a Spanish, French, German, Italian, Portuguese or Latin
// headword uses once decomposed (NFD), and nothing they do not. Letter
// anchoring and script closure skip Latin tracks (their reader arrives able to
// write the alphabet), so this file is the only closure gate on it. 22 letters
// carry a cited order from the Grundschrift-App (one level per letter, pinned
// to a commit), and the precomposed ñ á é í ó ú ü, ¿ and ¡ from UJIpenchars2's
// native Spanish writers (the base letter after the Grundschrift-App); every
// other row, and every combining mark, is recognition-only. The paths are
// fitted to LatinPrint-Subset.ttf, a renamed subset of SIL's literacy typeface
// Andika, because it prints the one-storey a every source teaches.

import { expect } from "vitest";
import type { ScriptEvidenceContext } from "./helpers.js";

const LATIN_TRACKS = new Set(["spanish", "french", "german", "italian", "portuguese", "latin"]);
const GRUNDSCHRIFT =
  "https://github.com/Medien-Treibhaus/grundschrift-app-source/blob/f6dbd807adbb3fc2f94207fe578def439f6e9c49/assets/levels/";
const UJI = "https://archive.ics.uci.edu/dataset/177/uji+pen+characters+version+2";

/** Each cited glyph: its lift count and where its order comes from. */
const CITED: Record<string, { penLifts: number; source: string }> = {
  b: { penLifts: 0, source: `${GRUNDSCHRIFT}kleinbuchstaben/b/metadata.json` },
  c: { penLifts: 0, source: `${GRUNDSCHRIFT}kleinbuchstaben/c/metadata.json` },
  e: { penLifts: 0, source: `${GRUNDSCHRIFT}kleinbuchstaben/e/metadata.json` },
  g: { penLifts: 0, source: `${GRUNDSCHRIFT}kleinbuchstaben/g/metadata.json` },
  h: { penLifts: 0, source: `${GRUNDSCHRIFT}kleinbuchstaben/h/metadata.json` },
  i: { penLifts: 1, source: `${GRUNDSCHRIFT}kleinbuchstaben/i/metadata.json` },
  l: { penLifts: 0, source: `${GRUNDSCHRIFT}kleinbuchstaben/l/metadata.json` },
  n: { penLifts: 0, source: `${GRUNDSCHRIFT}kleinbuchstaben/n/metadata.json` },
  o: { penLifts: 0, source: `${GRUNDSCHRIFT}kleinbuchstaben/o/metadata.json` },
  r: { penLifts: 0, source: `${GRUNDSCHRIFT}kleinbuchstaben/r/metadata.json` },
  s: { penLifts: 0, source: `${GRUNDSCHRIFT}kleinbuchstaben/s/metadata.json` },
  u: { penLifts: 0, source: `${GRUNDSCHRIFT}kleinbuchstaben/u/metadata.json` },
  w: { penLifts: 0, source: `${GRUNDSCHRIFT}kleinbuchstaben/w/metadata.json` },
  "ß": { penLifts: 0, source: `${GRUNDSCHRIFT}kleinbuchstaben/sz/metadata.json` },
  G: { penLifts: 0, source: `${GRUNDSCHRIFT}GROSSBUCHSTABEN/G/metadata.json` },
  a: { penLifts: 0, source: `${GRUNDSCHRIFT}kleinbuchstaben/a/metadata.json` },
  d: { penLifts: 0, source: `${GRUNDSCHRIFT}kleinbuchstaben/d/metadata.json` },
  p: { penLifts: 0, source: `${GRUNDSCHRIFT}kleinbuchstaben/p/metadata.json` },
  q: { penLifts: 0, source: `${GRUNDSCHRIFT}kleinbuchstaben/q/metadata.json` },
  t: { penLifts: 1, source: `${GRUNDSCHRIFT}kleinbuchstaben/t/metadata.json` },
  y: { penLifts: 1, source: `${GRUNDSCHRIFT}kleinbuchstaben/y/metadata.json` },
  H: { penLifts: 2, source: `${GRUNDSCHRIFT}GROSSBUCHSTABEN/H/metadata.json` },
  "ñ": { penLifts: 1, source: UJI },
  "á": { penLifts: 1, source: UJI },
  "é": { penLifts: 1, source: UJI },
  "í": { penLifts: 1, source: UJI },
  "ó": { penLifts: 1, source: UJI },
  "ú": { penLifts: 1, source: UJI },
  "ü": { penLifts: 2, source: UJI },
  "¿": { penLifts: 1, source: UJI },
  "¡": { penLifts: 1, source: UJI },
};

export const scriptInventoryEvidence = {
  name: "Latin",
  assert({ lessons, scripts, missingByScript }: ScriptEvidenceContext): void {
    const latin = scripts.latin!;
    expect(latin.font).toBe("_fonts/LatinPrint-Subset.ttf");
    expect(latin.complete).toBe(false);
    expect(latin.letters).toHaveLength(63);
    expect(latin.marks?.map((mark) => mark.mark).join(" ")).toBe(
      "̀ ́ ̂ ̃ ̄ ̈ ̧",
    );

    // Every Latin code point a Latin-track headword uses, once decomposed, is
    // in the inventory, and every row's code points are used by one.
    expect(missingByScript.get("latin.json")).toBeUndefined();
    const used = new Set(
      lessons
        .filter((lesson) => LATIN_TRACKS.has(lesson.language))
        .flatMap((lesson) => [...(lesson.realization.headword ?? "").normalize("NFD")])
        .filter((character) => /\p{Script=Latin}|\p{M}|[¿¡]/u.test(character)),
    );
    const listed = new Set(
      [
        ...latin.letters.map((letter) => letter.glyph),
        ...(latin.marks ?? []).map((mark) => mark.mark),
      ].flatMap((glyph) => [...glyph.normalize("NFD")]),
    );
    expect([...used].filter((character) => !listed.has(character))).toEqual([]);
    expect([...listed].filter((character) => !used.has(character))).toEqual([]);
    // No X or Y capital is read, so none is listed.
    expect(latin.letters.map((letter) => letter.glyph)).not.toContain("X");
    expect(latin.letters.map((letter) => letter.glyph)).not.toContain("Y");

    // Exactly the 31 cited glyphs carry an order, a lift count and a source;
    // no combining mark does (a mark is drawn only on its precomposed letter).
    const cited = latin.letters.filter((letter) => letter.strokeOrderSource !== undefined);
    expect(cited.map((letter) => letter.glyph).sort()).toEqual(Object.keys(CITED).sort());
    for (const letter of cited) {
      const claim = CITED[letter.glyph]!;
      expect(letter.penLifts, letter.glyph).toBe(claim.penLifts);
      expect(letter.strokeOrder.length, letter.glyph).toBeGreaterThan(0);
      expect(letter.strokeOrderSource!.url, letter.glyph).toBe(claim.source);
      expect(letter.strokeOrderSource!.citation, letter.glyph).toMatch(
        claim.source === UJI ? /^UJIpenchars2 .*\(CC BY 4\.0\)/ : /commit f6dbd80; no licence, facts only\)$/,
      );
    }
    for (const mark of latin.marks ?? []) {
      expect(mark.strokeOrderSource, mark.mark).toBeUndefined();
    }
    for (const letter of latin.letters.filter((entry) => !(entry.glyph in CITED))) {
      expect(letter.strokeOrder, letter.glyph).toEqual([]);
      expect(letter.penLifts, letter.glyph).toBeUndefined();
    }
  },
};
