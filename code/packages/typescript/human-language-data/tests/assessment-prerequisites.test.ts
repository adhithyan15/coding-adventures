import { afterEach, describe, expect, it, vi } from "vitest";
import { mkdtempSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { measureAssessmentPrerequisites } from "../src/assessment-prerequisites.js";
import { CEFR_LEVELS } from "../src/levels.js";
import { defaultCurriculumRoot, listTaskShapeInventories } from "../src/loader.js";
import { runCompletionPlan } from "../src/plan-cli.js";

const roots: string[] = [];
function root(): string {
  const path = mkdtempSync(join(tmpdir(), "hl-assessment-prereqs-"));
  roots.push(path);
  mkdirSync(join(path, "italian", "task-shapes"), { recursive: true });
  return path;
}

afterEach(() => {
  for (const path of roots.splice(0)) rmSync(path, { recursive: true, force: true });
  vi.restoreAllMocks();
});

describe("assessment contract prerequisites", () => {
  it("names the missing full-ladder shapes and mock evidence without calling a precursor ready", () => {
    const path = root();
    for (const slug of ["pre-a1", "a1"]) {
      writeFileSync(join(path, "italian", "task-shapes", `${slug}.json`), "{}\n");
    }
    const status = measureAssessmentPrerequisites(path, ["italian"], [
      { language: "italian", level: "pre-A1" },
      { language: "italian", level: "A1" },
    ], 2)[0];
    expect(status?.missing).toContain("task-shapes/a2.json");
    expect(status?.missing).toContain("mocks/a1/ (timed papers, rubric, answer keys)");
    expect(status?.missing).not.toContain("task-shapes/a1.json");
  });

  it("allows a canonical complete source set to reach the final contract gate", () => {
    const path = root();
    const shapes = CEFR_LEVELS.map((level) => ({ language: "italian", level }));
    for (const { level } of shapes) {
      const slug = level.toLowerCase();
      writeFileSync(join(path, "italian", "task-shapes", `${slug}.json`), "{}\n");
      const mockDir = join(path, "italian", "mocks", slug);
      mkdirSync(mockDir, { recursive: true });
      writeFileSync(join(mockDir, "rubric.md"), "rubric\n");
      for (let number = 1; number <= 2; number += 1) {
        writeFileSync(join(mockDir, `mock-${number}-paper.md`), "paper\n");
        writeFileSync(join(mockDir, `mock-${number}-answer-key.md`), "key\n");
      }
    }
    expect(measureAssessmentPrerequisites(path, ["italian"], shapes, 2)[0]?.missing).toEqual([]);
  });

  it("does not mistake directories named like source files for mock evidence", () => {
    const path = root();
    const mockDir = join(path, "italian", "mocks", "a1");
    mkdirSync(join(mockDir, "rubric.md"), { recursive: true });
    mkdirSync(join(mockDir, "mock-1-paper.md"));
    mkdirSync(join(mockDir, "mock-1-answer-key.md"));
    const missing = measureAssessmentPrerequisites(path, ["italian"], [], 1)[0]?.missing ?? [];
    expect(missing).toContain("mocks/a1/mock-1.md or mock-1-paper.md");
    expect(missing).toContain("mocks/a1/mock-1-answer-key.md");
    expect(missing).toContain("mocks/a1/rubric.md or mock-1-rubric.md");
  });

  it("rejects an unsafe track id before probing outside the root", () => {
    const path = root();
    expect(() => measureAssessmentPrerequisites(path, ["../other"], [], 2)).toThrow("unsafe track id");
  });

  it("keeps the actual Italian contract blocked by absent mocks and upper-rung shapes", () => {
    const status = measureAssessmentPrerequisites(
      defaultCurriculumRoot(),
      ["italian"],
      listTaskShapeInventories().filter((entry) => entry.language === "italian"),
      2,
    )[0];
    expect(status?.missing).toContain("task-shapes/a2.json");
    expect(status?.missing).toContain("mocks/a1/ (timed papers, rubric, answer keys)");
  });

  it("does not publish the premature Italian contract as an actionable head item", () => {
    const output: string[] = [];
    vi.spyOn(process.stdout, "write").mockImplementation((chunk) => {
      output.push(String(chunk));
      return true;
    });
    expect(runCompletionPlan(["--root", defaultCurriculumRoot(), "--format", "json", "--head", "100"])).toBe(0);
    const plan = JSON.parse(output.join("")) as { head: Array<{ id: string }>; projection: Array<{ kind: string; items: number }> };
    expect(plan.head.some((item) => item.id === "assessment-contract/italian")).toBe(false);
    expect(plan.projection.find((entry) => entry.kind === "assessment-contract")?.items).toBeGreaterThan(0);
  }, 120_000);
});
