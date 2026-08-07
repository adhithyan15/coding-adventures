import { describe, it, expect, vi } from "vitest";
import { runValidate } from "../src/cli.js";
import { runCurriculumGapReport } from "../src/report-cli.js";

describe("runValidate", () => {
  it("returns 0 (no errors) on the real curriculum and prints a report", () => {
    const out = vi.spyOn(process.stdout, "write").mockImplementation(() => true);
    const code = runValidate();
    const printed = out.mock.calls.map((c) => String(c[0])).join("");
    out.mockRestore();

    expect(code).toBe(0);
    expect(printed).toMatch(/concepts, \d+ languages/);
    expect(printed).toMatch(/error\(s\)/);
    expect(printed).toContain("spanish");
  });
});

describe("runCurriculumGapReport", () => {
  // 20s, not the 5s default: this builds the WHOLE gap report twice, and since
  // HL09 that includes the continuity walk (~900ms per build on the real corpus,
  // more on a cold CI runner). The cost is real and deliberate — order,
  // reinforcement and forward references are properties of every lesson, so the
  // walk cannot be input-gated the way `ramp` and `levels` are — and hiding it
  // behind a retry would be worse than paying for it here.
  it("prints JSON or text reports for the real curriculum", { timeout: 20_000 }, () => {
    const out = vi.spyOn(process.stdout, "write").mockImplementation(() => true);
    try {
      expect(runCurriculumGapReport(["--format", "json"])).toBe(0);
      const json = out.mock.calls.map((call) => String(call[0])).join("");
      // 22 since HL-C40 registered Japanese, following HL-C39's Mandarin Chinese —
      // the first two tracks outside the Indo-European and Dravidian families.
      expect(JSON.parse(json).summary.registeredTracks).toBe(22);
      out.mockClear();
      expect(runCurriculumGapReport(["--format", "text"])).toBe(0);
      expect(out.mock.calls.map((call) => String(call[0])).join("")).toContain(
        "Human Languages curriculum gap report",
      );
    } finally {
      out.mockRestore();
    }
  });

  it("rejects unknown or incomplete arguments", () => {
    const err = vi.spyOn(process.stderr, "write").mockImplementation(() => true);
    try {
      expect(runCurriculumGapReport(["--wat"])).toBe(2);
      expect(runCurriculumGapReport(["--format"])).toBe(2);
      expect(runCurriculumGapReport(["--format", "xml"])).toBe(2);
      expect(err.mock.calls.map((call) => String(call[0])).join("")).toContain("requires a value");
    } finally {
      err.mockRestore();
    }
  });
});
