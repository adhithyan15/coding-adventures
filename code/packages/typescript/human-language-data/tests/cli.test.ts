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
  it("prints JSON or text reports for the real curriculum", () => {
    const out = vi.spyOn(process.stdout, "write").mockImplementation(() => true);
    try {
      expect(runCurriculumGapReport(["--format", "json"])).toBe(0);
      const json = out.mock.calls.map((call) => String(call[0])).join("");
      expect(JSON.parse(json).summary.registeredTracks).toBe(20);
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
