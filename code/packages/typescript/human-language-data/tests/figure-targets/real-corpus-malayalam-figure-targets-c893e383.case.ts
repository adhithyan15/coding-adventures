import { describe, expect, it } from "vitest";
import { realCorpusTargets as targets } from "./real-corpus-fixture.js";

describe("Malayalam real-corpus figure targets", () => {
  it("owns every real-corpus sequence and its written pieces", () => {
    expect(
      Object.fromEntries(
        targets
          .filter((target) => target.script === "malayalam" && target.letters !== undefined)
          .map((target) => [target.lessonId, target.letters]),
      ),
    ).toEqual({
      "ML-W01-aa-ra-anusvaram": ["ാ", "ര", "ം"],
      "ML-W01-na-ma-delayed-copy": ["ന", "മ"],
      "ML-W01-na-ma-guided-copy": ["ന", "മ"],
      "ML-W01-na-ma-trace": ["ന", "മ"],
      "ML-W01-namaskaram-read": ["ന", "മ", "സ", "്", "ക", "ാ", "ര", "ം"],
      "ML-W01-sa-chandrakkala-ka": ["സ", "്", "ക"],
      "ML-W07-digits-1-3": ["൧", "൨", "൩"],
      "ML-W07-digits-4-5": ["൪", "൫"],
      "ML-W07-digits-6-8": ["൬", "൭", "൮"],
      "ML-W07-number-words-6-10": ["ഏ", "ഴ"],
      "ML-W07-numbers-1-5-delayed-copy": ["൧", "൨", "൩", "൪", "൫"],
      "ML-W07-numbers-1-5-guided-copy": ["൧", "൨", "൩", "൪", "൫"],
      "ML-W07-numbers-6-10-delayed-copy": ["ഏ", "ഴ", "്", "൭"],
    });
  });
});
