import { describe, expect, it } from "vitest";
import { realCorpusTargets as targets } from "./real-corpus-fixture.js";

describe("Cyrillic real-corpus figure targets", () => {
  it("does not draw the connected school-hand word as isolated letters", () => {
    expect(
      targets.some((target) => target.lessonId === "RU-W05-privet-guided-copy"),
    ).toBe(false);
  });
});
