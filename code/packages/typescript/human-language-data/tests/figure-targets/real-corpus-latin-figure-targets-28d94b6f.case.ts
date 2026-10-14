import { describe, expect, it } from "vitest";
import { realCorpusTargets as targets } from "./real-corpus-fixture.js";

describe("Latin real-corpus figure targets", () => {
  it("draws the Latin lessons whose every letter is cited, one-storey a included", () => {
    // 21 lessons over all six Latin-script tracks. The a waited for an outline
    // that prints the one-storey a every source teaches (LatinPrint-Subset.ttf,
    // from SIL's Andika); the acute vowels follow UJIpenchars2's writers; v
    // follows the Grundschrift-App; è ê ç ï ë ä ö and ē are drawn BY ANALOGY
    // with the cited ü, acute and tilde (base letter, then the mark last), and
    // their records say they are not separately sourced.
    const latin = targets.filter((target) => target.script === "latin");
    expect(
      Object.fromEntries(latin.map((target) => [target.lessonId, (target.letters ?? [target.glyph]).join(" ")])),
    ).toEqual({
      "ES-W00-hola-delayed-copy": "h o l a",
      "ES-W00-hola-guided-copy": "h o l a",
      "ES-W00-hola-observe": "h o l a",
      "ES-W01-acento": "á é í ó ú",
      "ES-W02-enye": "ñ",
      "ES-W03-inverted": "¿ ¡",
      "FR-W01-salut-delayed-copy": "s a l u t",
      "FR-W01-salut-guided-copy": "s a l u t",
      "FR-W01-salut-observe": "s a l u t",
      "FR-W01-accents": "é è ê",
      "FR-W02-cedille": "ç",
      "FR-W03-trema": "ï ë ü",
      "GE-W01-eszett": "ß",
      "GE-W01-hallo-delayed-copy": "H a l l o",
      "GE-W01-hallo-guided-copy": "H a l l o",
      "GE-W02-umlauts": "ä ö ü",
      "IT-W01-ciao-delayed-copy": "c i a o",
      "IT-W01-ciao-guided-copy": "c i a o",
      "LA-W01-salve-delayed-copy": "s a l v ē",
      "PT-W01-ola-delayed-copy": "o l á",
      "PT-W01-ola-guided-copy": "o l á",
    });
    // Still undrawn: œ (a ligature, not a marked letter, so not covered by the
    // analogy), punctuation inside a word, a slash between two words, or more
    // than MAX_SEQUENCE_PIECES.
    const lessonIds = new Set(targets.map((target) => target.lessonId));
    for (const id of [
      "FR-C10-oe",
      "GE-W03-capitalization",
      "LA-W01-salve-guided-copy",
      "ES-C03-como-acento",
      "ES-W01-tilde-diacritica",
      "ES-W02-enye-formas",
      "ES-W03-question-span",
    ]) {
      expect(lessonIds.has(id), id).toBe(false);
    }
  });

});
