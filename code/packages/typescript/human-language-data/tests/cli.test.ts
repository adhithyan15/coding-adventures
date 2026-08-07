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
  // EXPLICIT TIMEOUT, and it is not decoration. This case builds the ENTIRE gap report
  // twice — once as JSON, once as text — over the whole corpus, and the report gained a
  // continuity section (HL09 step 1) on top of modality, levels, verbs, chapters and ramp.
  // At 1,249 lessons it runs ~5.1s locally against vitest's 5,000ms default, so it was
  // already over the line on CI's slower runner and would have failed for whichever
  // content PR landed next, not this one specifically. The corpus only grows, so pin a
  // budget with real headroom rather than leaving a test that fails by calendar.
  it("prints JSON or text reports for the real curriculum", { timeout: 60_000 }, () => {
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
