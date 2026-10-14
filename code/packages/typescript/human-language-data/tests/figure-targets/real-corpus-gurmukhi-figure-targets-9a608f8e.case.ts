import { describe, expect, it } from "vitest";
import { realCorpusTargets as targets } from "./real-corpus-fixture.js";

describe("Gurmukhi real-corpus figure targets", () => {
  it("draws bare-letter words with one shared headline", () => {
    const words = targets.filter(
      (target) =>
        target.script === "gurmukhi" &&
        target.composition === "shared-headline" &&
        target.letters === undefined,
    );
    expect(Object.fromEntries(words.map((target) => [target.lessonId, target.glyph]))).toEqual({
      "PA-C39-par-write": "ਪਰ",
      "PA-W02-aman": "ਅਮਨ",
      "PA-W02-manan": "ਮਨਨ",
    });

    const lessonIds = new Set(targets.map((target) => target.lessonId));
    for (const id of [
      "PA-C42-oh-write",
      "PA-W07-age-label",
      "PA-C37-nahin-write",
      "PA-C38-ate-write",
    ]) {
      expect(lessonIds.has(id), id).toBe(false);
    }
  });

  it("keeps listed letters isolated rather than composing a word", () => {
    expect(
      Object.fromEntries(
        targets
          .filter((target) => target.script === "gurmukhi" && target.letters !== undefined)
          .map((target) => [target.lessonId, target.letters]),
      ),
    ).toEqual({
      "PA-S02-sassa-tatta-sihari": ["ਸ", "ਤ", "ਕ"],
      "PA-S03-retroflex-row": ["ਟ", "ਠ", "ਡ"],
      "PA-S07-naana-jajja-vavva": ["ਣ", "ਜ", "ਵ"],
      "PA-S07-rara-ghagga-dadda": ["ੜ", "ਘ", "ਦ"],
    });
  });
});
