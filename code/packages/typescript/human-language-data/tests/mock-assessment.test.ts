import { cpSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { defaultCurriculumRoot, loadTaskShapeInventory } from "../src/loader.js";
import { listMockAssessmentStatuses } from "../src/mock-assessment.js";
import { runAssessmentArtifactCli } from "../src/assessment-artifact-cli.js";
import type { TaskShapeInventory } from "../src/task-shapes.js";

let root: string;
const source = defaultCurriculumRoot();

function write(relative: string, content: string): void {
  const path = join(root, relative);
  mkdirSync(join(path, ".."), { recursive: true });
  writeFileSync(path, content, "utf8");
}

beforeEach(() => {
  root = mkdtempSync(join(tmpdir(), "hl-mock-assessment-"));
  mkdirSync(join(root, "core"), { recursive: true });
  cpSync(join(source, "core", "assessment-policy.json"), join(root, "core", "assessment-policy.json"));
  write("core/languages.json", `${JSON.stringify({
    version: 1, languages: [{ id: "spanish", name: "Spanish", family: "Romance", script: "latin", status: "active", bridges: [] }],
  })}\n`);
  const contract = JSON.parse(readFileSync(join(source, "spanish", "assessment.json"), "utf8"));
  const a1 = contract.levels.find((entry: { level: string }) => entry.level === "A1");
  a1.fullMocks[0].paper = "mocks/a1/mock-1.md";
  a1.fullMocks[0].manifest = "mocks/a1/mock-1-manifest.json";
  write("spanish/assessment.json", `${JSON.stringify(contract)}\n`);
  write("spanish/task-shapes/a1.json", readFileSync(join(source, "spanish", "task-shapes", "a1.json"), "utf8"));
  const shape: TaskShapeInventory = loadTaskShapeInventory("spanish", "A1", root);
  const manifest = {
    version: 1, language: "spanish", level: "A1", mockId: "a1-mock-1",
    sections: shape.sections.map((section) => {
      const variant = section.variants[0];
      return {
        skill: section.skill,
        minutes: typeof section.minutes === "number" ? section.minutes : section.minutes.minimum,
        ...(variant === undefined ? {} : { variantId: variant.id }),
        partIds: variant === undefined ? section.parts.map((part) => part.id) : variant.partIds,
      };
    }),
  };
  write("spanish/mocks/a1/mock-1-manifest.json", `${JSON.stringify(manifest)}\n`);
  write("spanish/mocks/a1/mock-1.md", "# Learner paper\n" +
    manifest.sections.flatMap((section) => section.partIds.map((id) =>
      `<!-- mock-part:${id} -->\nRead or perform this ${section.skill} prompt.\n<!-- /mock-part:${id} -->\n`,
    )).join("\n"));
  write("spanish/mocks/a1/rubric.md", "# Scoring rubric\n" +
    manifest.sections.flatMap((section) => section.partIds.map((id) =>
      `<!-- mock-part:${id} -->\nScore this ${section.skill} response against the published criteria.\n<!-- /mock-part:${id} -->\n`,
    )).join("\n"));
  write("spanish/mocks/a1/mock-1-answer-key.md", "# Answer key\n" +
    manifest.sections.flatMap((section) => section.partIds.map((id) =>
      `<!-- mock-part:${id} -->\nExpected ${section.skill} response or examiner guidance.\n<!-- /mock-part:${id} -->\n`,
    )).join("\n"));
});

afterEach(() => rmSync(root, { recursive: true, force: true }));

describe("full-mock coverage measurement", () => {
  it("counts only the mock with a present paper, distinct key and rubric, and valid manifest", () => {
    const statuses = listMockAssessmentStatuses(root);
    expect(statuses.find((status) => status.level === "A1")).toMatchObject({
      requiredMocks: 2, completeMocks: 1, missingMocks: ["a1-mock-2"], blockedBy: [],
    });
    expect(statuses.find((status) => status.level === "A2")).toMatchObject({
      completeMocks: 0, blockedBy: ["task-shape/spanish/A2"],
    });
  });

  it("refuses a declared manifest whose parts no longer cover the task inventory", () => {
    const path = join(root, "spanish", "mocks", "a1", "mock-1-manifest.json");
    const manifest = JSON.parse(readFileSync(path, "utf8"));
    manifest.sections[0].partIds.pop();
    write("spanish/mocks/a1/mock-1-manifest.json", `${JSON.stringify(manifest)}\n`);
    expect(() => listMockAssessmentStatuses(root)).toThrow(/part IDs/);
  });

  it("does not promote an absent paper to a complete mock", () => {
    rmSync(join(root, "spanish", "mocks", "a1", "mock-1.md"));
    expect(listMockAssessmentStatuses(root).find((status) => status.level === "A1"))
      .toMatchObject({ completeMocks: 0, missingMocks: ["a1-mock-1", "a1-mock-2"] });
  });

  it("refuses a paper that names no task parts even when the manifest is valid", () => {
    write("spanish/mocks/a1/mock-1.md", "# Learner paper\n");
    expect(() => listMockAssessmentStatuses(root)).toThrow(/missing task-shape part marker/);
    expect(() => runAssessmentArtifactCli(["--check"], root, () => {}, () => {}, () => {}))
      .toThrow(/missing task-shape part marker/);
  });

  it("does not count comment-only markup as learner-facing part content", () => {
    const path = join(root, "spanish", "mocks", "a1", "mock-1.md");
    const paper = readFileSync(path, "utf8");
    write("spanish/mocks/a1/mock-1.md", paper.replace(
      /Read or perform this reading prompt\./,
      "<!-- TODO: write a prompt --><div></div>&nbsp;",
    ));
    expect(() => listMockAssessmentStatuses(root)).toThrow(/no visible content/);
  });

  it("rejects a rubric with no per-part scoring spans", () => {
    write("spanish/mocks/a1/rubric.md", "# Rubric\n");
    expect(() => listMockAssessmentStatuses(root)).toThrow(/mock rubric: missing task-shape part marker/);
  });

  it("rejects an answer key with no per-part response spans", () => {
    write("spanish/mocks/a1/mock-1-answer-key.md", "# Answer key\n");
    expect(() => listMockAssessmentStatuses(root)).toThrow(/mock answer key: missing task-shape part marker/);
  });

  it("the required artifact CLI rejects a malformed declared manifest", () => {
    const path = join(root, "spanish", "mocks", "a1", "mock-1-manifest.json");
    const manifest = JSON.parse(readFileSync(path, "utf8"));
    manifest.sections[0].partIds.pop();
    write("spanish/mocks/a1/mock-1-manifest.json", `${JSON.stringify(manifest)}\n`);
    expect(() => runAssessmentArtifactCli(["--check"], root, () => {}, () => {}, () => {}))
      .toThrow(/part IDs/);
  });
});
