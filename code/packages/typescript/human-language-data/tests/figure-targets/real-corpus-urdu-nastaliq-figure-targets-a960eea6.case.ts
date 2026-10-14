import { describe, expect, it } from "vitest";
import { realCorpusTargets as targets } from "./real-corpus-fixture.js";

describe("Urdu Nastaliq real-corpus figure targets", () => {
  it("does not draw the joined word as isolated letters", () => {
    expect(targets.some((target) => target.lessonId === "UR-W04-joining")).toBe(false);
  });
});
