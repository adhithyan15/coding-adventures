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
  // 35s, not the 5s default: this builds the WHOLE gap report twice, and since
  // HL09 that includes the continuity walk. The cost is real and deliberate — order,
  // reinforcement and forward references are properties of every lesson, so the walk
  // cannot be input-gated the way `ramp` and `levels` are.
  //
  // History, because the shape of it matters more than the number. This budget was
  // raised 5s -> 20s at 1,249 lessons and 20s -> 35s at 1,878 lessons, and at 2,771
  // it blew through 35s on CI as well. Each raise was reasonable on its own and the
  // three together were a treadmill, because the walk was QUADRATIC in track length:
  // it asked every lesson about every word its track teaches, building a fresh
  // `\p{L}`-class regex for each pair. Profiling put ~60% of the entire gap report in
  // that one loop. Indexing the taught words by their leading run removed the
  // quadratic term, and `measureContinuity` went 2,065ms -> 218ms on this corpus,
  // taking the pair of builds from ~5.4s to ~3.0s locally. The report output is
  // byte-identical; see src/continuity.ts.
  //
  // So DO NOT read the next slow run as "time to raise it again". What is left here
  // is linear in lesson count — reading and parsing 2,771 files, hashing them, and
  // measuring each one — so the honest expectation is that this grows in proportion
  // to the corpus and that 35s covers roughly 3x today's size. If it runs close
  // before then, something has gone superlinear again: profile it, do not thin the
  // report, and do not just move this number.
  it("prints JSON or text reports for the real curriculum", { timeout: 35_000 }, () => {
    const out = vi.spyOn(process.stdout, "write").mockImplementation(() => true);
    try {
      expect(runCurriculumGapReport(["--format", "json"])).toBe(0);
      const json = out.mock.calls.map((call) => String(call[0])).join("");
      // Marwadi joins Mandarin and Japanese as a complete registry/book track.
      expect(JSON.parse(json).summary.registeredTracks).toBe(23);
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
