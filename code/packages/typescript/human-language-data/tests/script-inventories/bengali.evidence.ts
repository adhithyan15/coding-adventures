// Exact real-corpus evidence owned by the Bengali inventory.
// See HL24: unrelated script authors must not share an executable edit surface.
//
// The Bengali inventory is the track's OWN letter set: every letter, sign and
// digit a Bengali lesson reads, and nothing it does not (the track's letter-
// anchoring ceiling holds unread inventory letters at zero). Twenty-four rows
// carry a cited order from native writers' pen traces in HP Labs India's LipiTk
// Bangla recognizer; every other row is recognition-only. Fourteen of them draw
// the headline last by the documented convention (no placement wins a majority
// in their traces); their records say so, with the counts.

import { expect } from "vitest";
import type { ScriptEvidenceContext } from "./helpers.js";

const CITED: Record<string, { penLifts: number; cls: number }> = {
  "এ": { penLifts: 0, cls: 7 },
  "ও": { penLifts: 0, cls: 9 },
  "খ": { penLifts: 0, cls: 12 },
  "ঞ": { penLifts: 1, cls: 20 },
  "থ": { penLifts: 0, cls: 27 },
  "ব": { penLifts: 0, cls: 33 },
  "র": { penLifts: 1, cls: 37 },
  "ঃ": { penLifts: 1, cls: 47 },
  "ঁ": { penLifts: 1, cls: 48 },
  "ং": { penLifts: 1, cls: 46 },
  "ই": { penLifts: 2, cls: 2 },
  "চ": { penLifts: 1, cls: 16 },
  "ছ": { penLifts: 1, cls: 17 },
  "জ": { penLifts: 2, cls: 18 },
  "ড": { penLifts: 1, cls: 23 },
  "ত": { penLifts: 1, cls: 26 },
  "দ": { penLifts: 1, cls: 28 },
  "ন": { penLifts: 1, cls: 30 },
  "ফ": { penLifts: 1, cls: 32 },
  "ভ": { penLifts: 1, cls: 34 },
  "ম": { penLifts: 1, cls: 35 },
  "য": { penLifts: 1, cls: 36 },
  "ল": { penLifts: 1, cls: 38 },
  "হ": { penLifts: 1, cls: 42 },
};

// The rows whose headline is drawn LAST by convention, because no headline
// placement wins a majority of their traces. Each must say so and end with the
// headline; no other row may.
const HEADLINE_LAST_BY_CONVENTION = ["ই", "চ", "ছ", "জ", "ড", "ত", "দ", "ন", "ফ", "ভ", "ম", "য", "ল", "হ"];

export const scriptInventoryEvidence = {
  name: "Bengali",
  assert({ lessons, scripts, missingByScript }: ScriptEvidenceContext): void {
    const bengali = scripts.bengali!;
    expect(bengali.font).toBe("_fonts/NotoSansBengali-Static.ttf");
    expect(bengali.complete).toBe(false);
    expect(bengali.letters).toHaveLength(30);
    expect(bengali.marks).toHaveLength(11);
    expect(bengali.digits?.map((digit) => digit.glyph).join("")).toBe("০১২৩৪৫৬৭৮৯");

    // Every Bengali code point a Bengali headword uses is in the inventory.
    expect(missingByScript.get("bengali.json")).toBeUndefined();
    const used = new Set(
      lessons
        .filter((lesson) => lesson.language === "bengali")
        .flatMap((lesson) => [...(lesson.realization.headword ?? "").normalize("NFD")])
        .filter((character) => /\p{Script=Bengali}/u.test(character)),
    );
    const listed = new Set([
      ...bengali.letters.map((letter) => letter.glyph),
      ...(bengali.marks ?? []).map((mark) => mark.mark),
      ...(bengali.digits ?? []).map((digit) => digit.glyph),
    ]);
    expect([...used].filter((character) => !listed.has(character))).toEqual([]);

    // Exactly the twenty-four cited rows carry an order, a lift count and the source.
    const rows = [...bengali.letters, ...(bengali.marks ?? [])].map((row) => ({
      glyph: "glyph" in row ? row.glyph : row.mark,
      row,
    }));
    const cited = rows.filter(({ row }) => row.strokeOrderSource !== undefined);
    expect(cited.map(({ glyph }) => glyph).sort()).toEqual(Object.keys(CITED).sort());
    for (const { glyph, row } of cited) {
      expect(row.penLifts, glyph).toBe(CITED[glyph]!.penLifts);
      expect(row.strokeOrder?.length ?? 0, glyph).toBeGreaterThan(0);
      expect(row.strokeOrderSource!.url).toBe("https://lipitk.sourceforge.net/lipi-reco.htm");
      expect(row.strokeOrderSource!.citation).toContain(
        `Bangla recognizer, class ${CITED[glyph]!.cls} (${glyph}, `,
      );
    }
    for (const { glyph, row } of cited) {
      const byConvention = HEADLINE_LAST_BY_CONVENTION.includes(glyph);
      const variation = row.strokeOrderSource!.variation ?? "";
      expect(variation.includes("By the convention"), glyph).toBe(byConvention);
      expect(row.strokeOrder?.at(-1) === "lift, then draw the headline last, from left to right", glyph).toBe(
        byConvention,
      );
      if (byConvention) expect(variation, glyph).toMatch(/\d+ of the \d+ stored prototypes.*so no placement wins a majority/);
    }
    for (const letter of bengali.letters.filter((entry) => !(entry.glyph in CITED))) {
      expect(letter.strokeOrder, letter.glyph).toEqual([]);
      expect(letter.penLifts, letter.glyph).toBeUndefined();
    }
  },
};
