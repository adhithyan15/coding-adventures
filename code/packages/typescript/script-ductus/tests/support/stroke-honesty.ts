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


export const registerStrokeHonestyTests = (
  letters: LetterDuctus[],
  minimumInkFitOverrides: Readonly<Record<string, number>> = {},
  maximumUntracedOverrides: Readonly<Record<string, number>> = {},
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
        const pts = inkPoints(glyph().contours);
        const paths = letter.strokes.map((stroke) => penPath(stroke));
        const nearest = (x: number, y: number) =>
          Math.min(...paths.map((path) => distanceToPath(x, y, path)));
        const strayed = pts.filter(([x, y]) => nearest(x, y) > 100);
        const maximumUntraced = maximumUntracedOverrides[letter.glyph] ?? 0.02;
        expect(
          strayed.length / pts.length,
          "large parts of the letter are never traced",
        ).toBeLessThan(maximumUntraced);
      });
    });
  }
};
