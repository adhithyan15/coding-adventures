import { existsSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { defaultCurriculumRoot, loadExamInventory } from "../../src/loader.js";

describe("the committed Sanskrit A1 inventory", () => {
  const inventory = loadExamInventory("sanskrit", "A1");

  it("loads exclusively from direct point owners", () => {
    const core = join(defaultCurriculumRoot(), "core");
    const aggregate = join(core, "exam-inventory-sanskrit-a1.json");
    const owners = join(core, "exam-inventory-sanskrit-a1.d");
    const ownerNames = readdirSync(owners).filter(
      (name) => name !== "_meta.json",
    );

    expect(existsSync(aggregate)).toBe(false);
    expect(ownerNames).toHaveLength(inventory.points.length);
    expect(new Set(inventory.points.map((point) => point.id)).size).toBe(
      inventory.points.length,
    );
  });

  it("does not freeze mutable corpus totals in provenance metadata", () => {
    const corpusAnchor = inventory.anchors.find((anchor) => anchor.id === "SA-CORPUS");
    expect(corpusAnchor).toBeDefined();

    // These are evidence-base descriptions, not a versioned census. A bare
    // total becomes false as soon as an independent lesson tranche lands.
    const bareTotal = /\b\d[\d,]*\s+(?:lessons?|chapters?|distinct\s+atoms?)\b/i;
    const atomBreakdown = /\b\d[\d,]*\s+(?:LEX|ETYMON|SCRIPT|GRAMMAR|HISTORY|PATTERN|CULTURE|SOUND|COMPARISON|shown|taught|neither)\b/i;
    expect(inventory.source).not.toMatch(bareTotal);
    expect(inventory.source).not.toMatch(atomBreakdown);
    expect(corpusAnchor?.title).not.toMatch(bareTotal);
    expect(inventory.scope["phonology-orthography"].note).not.toMatch(atomBreakdown);
    expect(inventory.source).toContain("code/learning/human-languages/sanskrit/lessons/");
    expect(corpusAnchor?.title).toContain("code/learning/human-languages/sanskrit/");
  });

  it("distinguishes the project-authored assessment envelope from external validation", () => {
    const sanskrit = join(defaultCurriculumRoot(), "sanskrit");
    for (const authoredArtifact of [
      "assessment-spec.md",
      "assessment.json",
      "task-shapes/a1.json",
      "mocks/a1",
    ]) {
      expect(existsSync(join(sanskrit, authoredArtifact))).toBe(true);
    }

    expect(inventory.about).not.toMatch(/there is no sanskrit\/assessment-spec/i);
    expect(inventory.source).not.toContain("EXAM ENVELOPE: NONE EXISTS");
    expect(inventory.scope["communicative-functions"].note).not.toMatch(/no checked-in mock or task shape/i);
    expect(inventory.source).toMatch(/project-authored.*assessment/i);
    expect(inventory.about).toMatch(/NOT AN EXTERNAL SYLLABUS/);
  });
});
