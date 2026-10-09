import { describe, expect, it } from "vitest";
import { SCRIPTS } from "../../src/scriptdata";
import { joinGaps, penPath, type LetterDuctus } from "../../src/strokes";
import { parsedFont } from "./font-fixtures";
import { distanceToPath, fractionOnInk, inkPoints, makeInInk } from "../../src/ink";

// The ink measurements live in src/ink.ts (a word's headline is checked with
// them at build time); they are re-exported here for the tests that import
// them from this file.
export { distanceToPath, fractionOnInk, inkPoints, makeInInk };

export const fontForDuctus = (letter: LetterDuctus) => {
  const script = SCRIPTS.find(
    (candidate) => candidate.script === letter.script,
  );
  if (!script) throw new Error(`no verified script/font owns ${letter.glyph}`);
  // Digits with a cited ductus (Kannada ೧-೯) are claimed like letters.
  const letterClaim = [
    ...script.letters,
    ...(script.independentVowels ?? []),
    ...(script.finalConsonants ?? []),
    ...(script.digits ?? []),
  ].find(
    (entry) =>
      entry.glyph === letter.glyph &&
      entry.strokeOrderSource?.url === letter.source.url,
  );
  const ligatureClaim = script.ligatures?.find(
    (entry) =>
      entry.displayGlyph === letter.glyph &&
      entry.strokeOrderSource?.url === letter.source.url,
  );
  const markClaim = script.marks?.find(
    (entry) =>
      entry.mark === letter.glyph &&
      entry.strokeOrderSource?.url === letter.source.url,
  );
  if (!letterClaim && !ligatureClaim && !markClaim)
    throw new Error(`${letter.script} does not verify ${letter.glyph}`);
  return parsedFont(script.font.split("/").pop()!);
};


/**
 * A rectangle of PRINTED ink, in font units (y up), that a glyph's coverage
 * check leaves out of its count, and why.
 *
 * This is the narrowest exception the coverage check knows. Raising a glyph's
 * `maximumUntracedOverrides` ceiling would excuse ANY untraced ink up to the
 * new share — a dropped bowl as readily as the part meant. An excused
 * rectangle excuses only the ink inside it: every sample outside it must still
 * be traced at the default 2%, and the ink inside it still counts against
 * nothing else (the on-ink check of every stroke is unchanged).
 *
 * It exists for one case so far: a sign whose printed glyph carries ink that
 * belongs to something the sign is not — the short piece of headline Noto
 * Sans Devanagari prints on ी, ो and ः, which in handwriting is part of the
 * word's one headline (see HEADLINE_STUBS in tests/strokes/devanagari.test.ts).
 */
export interface ExcusedInk {
  readonly x0: number;
  readonly x1: number;
  readonly y0: number;
  readonly y1: number;
  readonly why: string;
}

/** Whether an ink sample lies inside an excused rectangle (edges included). */
export const insideExcused = ([x, y]: readonly [number, number], box: ExcusedInk): boolean =>
  x >= box.x0 && x <= box.x1 && y >= box.y0 && y <= box.y1;

/**
 * The share of a glyph's ink samples that no stroke comes within 100 font
 * units of — the number the coverage check bounds — counting only samples
 * outside `excused`, if given.
 */
export const untracedShare = (letter: LetterDuctus, excused?: ExcusedInk): number => {
  const all = inkPoints(fontForDuctus(letter).glyphFor(letter.glyph)!.contours);
  const pts = excused ? all.filter((point) => !insideExcused(point, excused)) : all;
  const paths = letter.strokes.map((stroke) => penPath(stroke));
  const nearest = (x: number, y: number) =>
    Math.min(...paths.map((path) => distanceToPath(x, y, path)));
  return pts.filter(([x, y]) => nearest(x, y) > 100).length / pts.length;
};

export const registerStrokeHonestyTests = (
  letters: LetterDuctus[],
  minimumInkFitOverrides: Readonly<Record<string, number>> = {},
  maximumUntracedOverrides: Readonly<Record<string, number>> = {},
  excusedInk: Readonly<Record<string, ExcusedInk>> = {},
): void => {
  for (const letter of letters) {
    describe(`${letter.glyph}`, () => {
      const glyph = () => fontForDuctus(letter).glyphFor(letter.glyph)!;
      it("every stroke's pen path lies on the real letter", () => {
        const inInk = makeInInk(glyph().contours);
        for (let s = 0; s < letter.strokes.length; s++) {
          const frac = fractionOnInk(penPath(letter.strokes[s]), inInk);
          const minimumInkFit = minimumInkFitOverrides[letter.glyph] ?? 0.97;
          expect(frac, `stroke ${s} strays off the glyph`).toBeGreaterThan(
            minimumInkFit,
          );
        }
      });

      it("parts within a stroke connect — no gap, so no pen lift", () => {
        for (const stroke of letter.strokes) {
          for (const gap of joinGaps(stroke)) {
            expect(gap).toBeLessThan(2);
          }
        }
      });

      it("the strokes trace the WHOLE letter, not just part of it", () => {
        const maximumUntraced = maximumUntracedOverrides[letter.glyph] ?? 0.02;
        expect(
          untracedShare(letter, excusedInk[letter.glyph]),
          "large parts of the letter are never traced",
        ).toBeLessThan(maximumUntraced);
      });
    });
  }
};
