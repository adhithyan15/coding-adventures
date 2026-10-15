import { describe, expect, it } from "vitest";
import { realCorpusTargets as targets } from "./real-corpus-fixture.js";

describe("Japanese real-corpus figure targets", () => {
  it("draws the cited stand-apart words letter by letter", () => {
    expect(
      Object.fromEntries(
        targets
          .filter((target) => target.script === "japanese" && target.letters !== undefined)
          .map((target) => [target.lessonId, target.letters]),
      ),
    ).toEqual({
      "JA-W01-hai-read": ["は", "い"],
      "JA-W01-konnichiwa-read": ["こ", "ん", "に", "ち", "は"],
      "JA-W03-arigatou-read": ["あ", "り", "が", "と", "う"],
      "JA-W08-sayounara-read": ["さ", "よ", "う", "な", "ら"],
    });
  });
});
