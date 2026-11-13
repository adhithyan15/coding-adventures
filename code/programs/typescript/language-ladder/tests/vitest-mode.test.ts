import { describe, expect, it } from "vitest";
import { watchCurriculumSourcesForVitest } from "../vitest-mode.ts";

describe("curriculum source watches in Vitest", () => {
  const vitest = ["node", "vitest.mjs"];

  it("skips watches only for the one-shot run command", () => {
    expect(watchCurriculumSourcesForVitest([...vitest, "run"]))
      .toBe(false);
    expect(watchCurriculumSourcesForVitest([...vitest, "-t", "run"]))
      .toBe(true);
    expect(watchCurriculumSourcesForVitest(vitest)).toBe(true);
  });

  it.each(["-w", "-w=true", "--watch", "--watch=true"])(
    "honors the %s override",
    (watchFlag) => {
      expect(watchCurriculumSourcesForVitest([...vitest, "run", watchFlag]))
        .toBe(true);
    },
  );
});
