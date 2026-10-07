// Exact real-corpus evidence owned by the Gurmukhi inventory.
// See HL24: unrelated script authors must not share an executable edit surface.
//
// The Gurmukhi inventory is the Punjabi track's OWN letter set: every letter,
// sign and digit a Punjabi lesson reads, and nothing it does not (the track's
// letter-anchoring ceiling holds unread inventory letters at zero). 27 letters
// carry a cited order from the Apache-2.0 Alphabet Tracing lesson of GNPS's
// Gurmukhi Sikho app, pinned to one commit and one line per letter; every other
// row, and every sign, is recognition-only.

import { expect } from "vitest";
import type { ScriptEvidenceContext } from "./helpers.js";

const SOURCE =
  "https://github.com/codemanxdev/gnps_learning_hub/blob/de1e56014cb0caf7e56de2320681781cae6a7e41/lib/data/lessons/lesson_tracing.dart#L";

const CITED: Record<string, { penLifts: number; line: number }> = {
  "ਅ": { penLifts: 1, line: 62 },
  "ਸ": { penLifts: 2, line: 127 },
  "ਹ": { penLifts: 1, line: 157 },
  "ਕ": { penLifts: 1, line: 190 },
  "ਖ": { penLifts: 2, line: 218 },
  "ਗ": { penLifts: 2, line: 247 },
  "ਘ": { penLifts: 1, line: 276 },
  "ਚ": { penLifts: 1, line: 346 },
  "ਛ": { penLifts: 1, line: 373 },
  "ਜ": { penLifts: 2, line: 408 },
  "ਟ": { penLifts: 1, line: 509 },
  "ਠ": { penLifts: 1, line: 535 },
  "ਡ": { penLifts: 1, line: 564 },
  "ਣ": { penLifts: 2, line: 629 },
  "ਤ": { penLifts: 1, line: 665 },
  "ਥ": { penLifts: 3, line: 696 },
  "ਦ": { penLifts: 1, line: 730 },
  "ਨ": { penLifts: 2, line: 785 },
  "ਪ": { penLifts: 1, line: 826 },
  "ਫ": { penLifts: 1, line: 854 },
  "ਬ": { penLifts: 2, line: 885 },
  "ਭ": { penLifts: 1, line: 920 },
  "ਮ": { penLifts: 1, line: 952 },
  "ਰ": { penLifts: 1, line: 1014 },
  "ਲ": { penLifts: 3, line: 1040 },
  "ਵ": { penLifts: 2, line: 1076 },
  "ੜ": { penLifts: 2, line: 1108 },
};

export const scriptInventoryEvidence = {
  name: "Gurmukhi",
  assert({ lessons, scripts, missingByScript }: ScriptEvidenceContext): void {
    const gurmukhi = scripts.gurmukhi!;
    expect(gurmukhi.font).toBe("_fonts/NotoSansGurmukhi-Static.ttf");
    expect(gurmukhi.complete).toBe(false);
    expect(gurmukhi.letters).toHaveLength(33);
    expect(gurmukhi.marks).toHaveLength(14);
    expect(gurmukhi.digits?.map((digit) => digit.glyph).join("")).toBe("੦੧੨੫");

    // Every Gurmukhi code point a Punjabi headword uses is in the inventory.
    expect(missingByScript.get("gurmukhi.json")).toBeUndefined();
    const used = new Set(
      lessons
        .filter((lesson) => lesson.language === "punjabi")
        .flatMap((lesson) => [...(lesson.realization.headword ?? "").normalize("NFD")])
        .filter((character) => /\p{Script=Gurmukhi}/u.test(character)),
    );
    const listed = new Set([
      ...gurmukhi.letters.map((letter) => letter.glyph),
      ...(gurmukhi.marks ?? []).map((mark) => mark.mark),
      ...(gurmukhi.digits ?? []).map((digit) => digit.glyph),
    ]);
    expect([...used].filter((character) => !listed.has(character))).toEqual([]);
    expect([...listed].filter((character) => !used.has(character))).toEqual([]);

    // Exactly the 27 cited letters carry an order, a lift count and the source;
    // no sign does (vowel signs, bindi, tippi, addak, halant and the dot below
    // have no stroke-order source).
    const cited = gurmukhi.letters.filter((letter) => letter.strokeOrderSource !== undefined);
    expect(cited.map((letter) => letter.glyph).sort()).toEqual(Object.keys(CITED).sort());
    for (const letter of cited) {
      const claim = CITED[letter.glyph]!;
      expect(letter.penLifts, letter.glyph).toBe(claim.penLifts);
      expect(letter.strokeOrder.length, letter.glyph).toBeGreaterThan(0);
      expect(letter.strokeOrderSource!.url).toBe(`${SOURCE}${claim.line}`);
      expect(letter.strokeOrderSource!.citation).toContain(
        `Alphabet Tracing lesson, ${letter.glyph} (`,
      );
      expect(letter.strokeOrderSource!.citation).toContain(`line ${claim.line}, commit de1e560; Apache-2.0`);
    }
    for (const mark of gurmukhi.marks ?? []) {
      expect(mark.strokeOrderSource, mark.mark).toBeUndefined();
    }
    for (const letter of gurmukhi.letters.filter((entry) => !(entry.glyph in CITED))) {
      expect(letter.strokeOrder, letter.glyph).toEqual([]);
      expect(letter.penLifts, letter.glyph).toBeUndefined();
    }
  },
};
