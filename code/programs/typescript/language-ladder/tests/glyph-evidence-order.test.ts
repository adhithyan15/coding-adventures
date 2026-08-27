import { describe, expect, it } from "vitest";
import {
  assertValidGlyphEvidenceRanks,
  compareGlyphEvidence,
  type LocatedGlyphEvidence,
} from "./glyph-evidence/types";

const evidence = (
  overrides: Partial<LocatedGlyphEvidence> = {},
): LocatedGlyphEvidence => ({
  suite: "suite",
  suiteOrder: 10,
  caseOrder: 20,
  name: "case",
  modulePath: "./glyph-evidence/example.evidence.ts",
  verify: () => undefined,
  ...overrides,
});

describe("glyph evidence ordering", () => {
  it("accepts positive safe-integer ranks", () => {
    expect(() => assertValidGlyphEvidenceRanks(evidence())).not.toThrow();
  });

  it.each([0, -1, 1.5, Number.NaN, Number.POSITIVE_INFINITY, Number.MAX_SAFE_INTEGER + 1])(
    "rejects invalid suiteOrder %s",
    (suiteOrder) => {
      expect(() => assertValidGlyphEvidenceRanks(evidence({ suiteOrder }))).toThrow(
        /invalid suiteOrder/,
      );
    },
  );

  it.each([0, -1, 1.5, Number.NaN, Number.NEGATIVE_INFINITY, Number.MAX_SAFE_INTEGER + 1])(
    "rejects invalid caseOrder %s",
    (caseOrder) => {
      expect(() => assertValidGlyphEvidenceRanks(evidence({ caseOrder }))).toThrow(
        /invalid caseOrder/,
      );
    },
  );

  it("breaks same-rank ties by module path and then case name", () => {
    const entries = [
      evidence({ modulePath: "./z.evidence.ts", name: "alpha" }),
      evidence({ modulePath: "./a.evidence.ts", name: "zulu" }),
      evidence({ modulePath: "./a.evidence.ts", name: "alpha" }),
    ];

    expect(entries.sort(compareGlyphEvidence).map(({ modulePath, name }) => [modulePath, name]))
      .toEqual([
        ["./a.evidence.ts", "alpha"],
        ["./a.evidence.ts", "zulu"],
        ["./z.evidence.ts", "alpha"],
      ]);
  });
});
