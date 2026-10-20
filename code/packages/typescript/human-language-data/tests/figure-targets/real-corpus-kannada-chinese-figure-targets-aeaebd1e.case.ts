import { describe, expect, it } from "vitest";
import { realCorpusTargets as targets } from "./real-corpus-fixture.js";

describe("Kannada and Chinese real-corpus figure targets", () => {
  it("keeps their current cited targets isolated rather than inventing word composition", () => {
    for (const script of ["kannada", "chinese"]) {
      const owned = targets.filter((target) => target.script === script);
      expect(owned.length, `${script} has cited real-corpus targets`).toBeGreaterThan(0);
      expect(owned.every((target) => target.letters === undefined), script).toBe(true);
      expect(owned.every((target) => target.composition === undefined), script).toBe(true);
    }
  });
});
