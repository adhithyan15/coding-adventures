import { describe, expect, it } from "vitest";
import { SCRIPTS, verifiedLetterFont } from "../src/scriptdata";
import {
  DUCTUS,
  joinGaps,
  penLifts,
  penPath,
  penPathD,
  penTip,
  type LetterDuctus,
} from "../src/strokes";
import { ductusFor } from "../src/ductusview";

const reference: LetterDuctus = {
  script: "test",
  glyph: "*",
  strokes: [
    {
      segments: [
        {
          label: "down",
          path: [
            { x: 0, y: 100 },
            { x: 0, y: 0 },
          ],
        },
        {
          label: "across",
          path: [
            { x: 0, y: 0 },
            { x: 100, y: 0 },
          ],
        },
        {
          label: "finish",
          path: [
            { x: 100, y: 0 },
            { x: 150, y: -100 },
          ],
        },
      ],
    },
  ],
  source: {
    citation: "synthetic geometry fixture",
    url: "https://example.invalid/fixture",
  },
};

describe("handwriting ductus", () => {
  const letters = Object.values(DUCTUS) as LetterDuctus[];

  it("has at least one authored letter", () => {
    expect(letters.length).toBeGreaterThan(0);
  });

  it("keeps every canonical script inventory free of duplicate glyph rows", () => {
    for (const script of SCRIPTS) {
      const glyphs = script.letters.map((letter) => letter.glyph);
      expect(
        new Set(glyphs).size,
        `${script.script} repeats a canonical glyph`,
      ).toBe(glyphs.length);
    }
  });

  // The PROVENANCE GATE. A stroke's SHAPE is checked against the font above;
  // its ORDER cannot be — so it must trace to a cited source, or it does not
  // ship. This is the counterpart, for hand-authored order, of "facts enter
  // only through a source". Where no source exists, the letter is simply not
  // authored rather than invented.
  it("every letter cites a real source for its stroke order", () => {
    for (const letter of letters) {
      expect(letter.source, `${letter.glyph} has no source`).toBeDefined();
      expect(
        letter.source.citation.length,
        `${letter.glyph} citation is empty`,
      ).toBeGreaterThan(10);
      expect(
        letter.source.url,
        `${letter.glyph} source url is not a real link`,
      ).toMatch(/^https?:\/\/\S+$/);
    }
  });

  it("every verified prose claim has the same font-checked ductus and source", () => {
    const verified = SCRIPTS.flatMap((script) => [
      ...script.letters
        .filter(
          (letter) =>
            letter.penLifts !== undefined ||
            letter.strokeOrderSource !== undefined,
        )
        .map((letter) => ({
          script: script.script,
          identity: letter.glyph,
          glyph: letter.glyph,
          penLifts: letter.penLifts,
          source: letter.strokeOrderSource,
        })),
      ...(script.independentVowels ?? [])
        .filter(
          (letter) =>
            letter.penLifts !== undefined ||
            letter.strokeOrderSource !== undefined,
        )
        .map((letter) => ({
          script: script.script,
          identity: letter.glyph,
          glyph: letter.glyph,
          penLifts: letter.penLifts,
          source: letter.strokeOrderSource,
        })),
      ...(script.finalConsonants ?? [])
        .filter(
          (letter) =>
            letter.penLifts !== undefined ||
            letter.strokeOrderSource !== undefined,
        )
        .map((letter) => ({
          script: script.script,
          identity: letter.glyph,
          glyph: letter.glyph,
          penLifts: letter.penLifts,
          source: letter.strokeOrderSource,
        })),
      ...(script.marks ?? [])
        .filter(
          (mark) =>
            mark.penLifts !== undefined ||
            mark.strokeOrderSource !== undefined,
        )
        .map((mark) => ({
          script: script.script,
          identity: mark.mark,
          glyph: mark.mark,
          penLifts: mark.penLifts,
          source: mark.strokeOrderSource,
        })),
      ...(script.ligatures ?? [])
        .filter(
          (ligature) =>
            ligature.penLifts !== undefined ||
            ligature.strokeOrderSource !== undefined,
        )
        .map((ligature) => ({
          script: script.script,
          identity: ligature.sequence,
          glyph: ligature.displayGlyph,
          penLifts: ligature.penLifts,
          source: ligature.strokeOrderSource,
        })),
    ]);
    expect(verified).toHaveLength(letters.length);
    for (const claim of verified) {
      const ductus = ductusFor(claim.identity, claim.script);
      expect(
        ductus,
        `${claim.script} ${claim.identity} claims verification without a ductus`,
      ).toBeDefined();
      if (!ductus)
        throw new Error(`${claim.script} ${claim.identity} has no ductus`);
      expect(ductus.glyph).toBe(claim.glyph);
      expect(claim.penLifts).toBe(penLifts(ductus));
      expect(claim.source).toEqual(ductus.source);
    }
    for (const ductus of letters) {
      expect(
        verified.some(
          (claim) =>
            claim.script === ductus.script &&
            claim.identity === (ductus.sequence ?? ductus.glyph),
        ),
        `${ductus.glyph} has a ductus but no verified prose claim`,
      ).toBe(true);
    }
  });

  it("rejects a source that owns no verified letter", () => {
    expect(
      verifiedLetterFont("", "https://example.invalid/wrong-source"),
    ).toBeUndefined();
  });
});

describe("pen-path geometry", () => {
  const stroke = reference.strokes[0];

  it("penPath joins segments head-to-tail without duplicating the join", () => {
    const segTotal = stroke.segments.reduce((n, s) => n + s.path.length, 0);
    // Two joins collapse two duplicated points, so the path is 2 shorter.
    expect(penPath(stroke).length).toBe(
      segTotal - (stroke.segments.length - 1),
    );
  });

  it("penPathD grows monotonically with the fraction drawn", () => {
    const q = penPathD(stroke, 0.25).length;
    const h = penPathD(stroke, 0.5).length;
    const f = penPathD(stroke, 1).length;
    expect(q).toBeLessThan(h);
    expect(h).toBeLessThanOrEqual(f);
    expect(penPathD(stroke, 1)).toMatch(/^M/);
  });

  it("penTip advances along the stroke and ends where the pen ends", () => {
    const start = penTip(stroke, 0).at;
    const end = penTip(stroke, 1).at;
    const first = stroke.segments[0].path[0];
    expect(start.x).toBeCloseTo(first.x);
    expect(start.y).toBeCloseTo(first.y);
    // The synthetic path finishes to the right of and below its start.
    expect(end.x).toBeGreaterThan(start.x);
    expect(end.y).toBeLessThan(start.y);
  });

  // -------------------------------------------------------------------------
  // Controls: prove each honesty check above can actually FAIL.
  // -------------------------------------------------------------------------
  it("CONTROL: a broken join is caught by the gap check", () => {
    const broken = {
      segments: [
        {
          label: "a",
          path: [
            { x: 0, y: 0 },
            { x: 100, y: 0 },
          ],
        },
        {
          label: "b",
          path: [
            { x: 100, y: 80 },
            { x: 200, y: 80 },
          ],
        }, // starts 80 away
      ],
    };
    expect(Math.max(...joinGaps(broken))).toBeGreaterThan(2);
  });
});
