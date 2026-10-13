import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { defaultCurriculumRoot, loadTaskShapeInventory } from "../src/loader.js";

const root = join(defaultCurriculumRoot(), "kannada");
const paper = readFileSync(join(root, "mocks", "a1", "writing-paper-1.md"), "utf8");
const rubric = readFileSync(join(root, "mocks", "a1", "writing-paper-1-rubric.md"), "utf8");
const lesson = readFileSync(join(root, "lessons", "KA-W77-a1-timed-production.md"), "utf8");

describe("Kannada A1 writing-only paper", () => {
  it("keeps a genuine 20-minute assessment outside the five-minute lesson envelope", () => {
    const writing = loadTaskShapeInventory("kannada", "A1").sections.find((section) => section.skill === "writing");
    expect(writing?.minutes).toBe(20);
    expect(writing?.parts.map((part) => part.scoring.maxRawPoints)).toEqual([40, 60]);
    expect(paper).toContain("**20-minute** clock");
    expect(paper).toContain("**7 minutes**");
    expect(paper).toContain("**13 minutes**");
    expect(paper).toContain("**seven**");
    expect(paper).toContain("**30–40 Kannada words**");
    expect(paper).toContain("7 minutes; 40 points");
    expect(paper).toContain("13 minutes; 60 points");
    expect(paper.match(/^\| [^\n]+ \| _+ \|$/gm)).toHaveLength(7);
    expect(paper).toContain("writing-only");
    expect(lesson).toContain("max_seconds: 280");
    expect(rubric).toContain("not one of");
    expect(rubric).toContain("Two complete timed mock passes");
  });

  it("uses an independent walk prompt and keeps scoring guidance off the learner sheet", () => {
    expect(paper).toContain("morning walk");
    expect(paper).toContain("ಅನು");
    expect(paper).not.toContain("Mira");
    expect(paper).not.toContain("language exchange");
    expect(paper).not.toContain("0–5 per dimension");
    expect(rubric).toContain("0–5 per dimension");
    expect(rubric).toMatch(/Documented regional\s+forms/);
    expect(paper).not.toContain("romanized sample");
  });
});
