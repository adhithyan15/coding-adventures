import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { defaultCurriculumRoot, loadTaskShapeInventory } from "../src/loader.js";

const root = join(defaultCurriculumRoot(), "gujarati");
const paper = readFileSync(join(root, "mocks", "a1", "writing-paper-1.md"), "utf8");
const rubric = readFileSync(join(root, "mocks", "a1", "writing-paper-1-rubric.md"), "utf8");
const lesson = readFileSync(join(root, "lessons", "GU-W10-a1-timed-production.md"), "utf8");

describe("Gujarati A1 writing-only paper", () => {
  it("keeps a genuine 20-minute assessment outside the five-minute lesson envelope", () => {
    const writing = loadTaskShapeInventory("gujarati", "A1").sections.find((section) => section.skill === "writing");
    expect(writing?.minutes).toBe(20);
    expect(writing?.parts.map((part) => part.scoring.maxRawPoints)).toEqual([40, 60]);
    expect(paper).toContain("**20-minute** clock");
    expect(paper).toContain("**7 minutes**");
    expect(paper).toContain("**13 minutes**");
    expect(paper).toContain("**seven**");
    expect(paper).toContain("**30–40 Gujarati words**");
    expect(paper).toContain("7 minutes; 40 points");
    expect(paper).toContain("13 minutes; 60 points");
    expect(paper).toContain("writing-only");
    expect(lesson).toContain("max_seconds: 280");
    expect(rubric).toContain("not one of");
    expect(rubric).toContain("Two complete timed mock passes");
  });

  it("uses a distinct library task and keeps scoring guidance out of the learner sheet", () => {
    expect(paper).toContain("local library");
    expect(paper).toContain("કિરણ");
    expect(paper).not.toContain("Mira");
    expect(paper).not.toContain("0–5 per dimension");
    expect(rubric).toContain("0–5 per dimension");
    expect(rubric).toContain("Documented regional forms");
    expect(paper).not.toContain("romanized sample");
  });
});
