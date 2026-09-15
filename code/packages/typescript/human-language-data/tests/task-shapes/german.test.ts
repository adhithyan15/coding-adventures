import { describe, expect, it } from "vitest";
import { listTaskShapeInventories, loadLessons, loadTaskShapeInventory } from "../../src/loader.js";
import { measureReadingReach } from "../../src/reading-reach.js";

describe("German task-shape inventories", () => {
  it("loads the official German A1 performance target", () => {
    const inventory = loadTaskShapeInventory("german", "A1");
    expect(inventory.target).toEqual({
      name: "Goethe-Zertifikat A1: Start Deutsch 1",
      basis: "external",
    });
    expect(inventory.sections.map((section) => section.skill)).toEqual([
      "reading",
      "listening",
      "writing",
      "speaking",
    ]);
    expect(inventory.sections.flatMap((section) => section.parts)).toHaveLength(11);
    expect(inventory.sections.every((section) => section.variants.length === 0)).toBe(true);
    expect(inventory.passRule).toMatchObject({ maximumPoints: 100, passPoints: 60 });
    expect(Object.values(inventory.passRule.independentSkillThresholds)).toEqual([null, null, null, null]);
  });

  it("builds a precursor BELOW Start Deutsch 1, using the name the spec already fixed", () => {
    const inventory = loadTaskShapeInventory("german", "pre-A1");

    // The German assessment spec named this target before the inventory existed
    // and deferred the inventory to backlog. Taking the name verbatim from the
    // spec rather than coining a new one is what keeps the two documents talking
    // about the same thing.
    expect(inventory.target).toEqual({
      name: "Coding Adventures German pre-A1 Assessment — project-defined Goethe precursor",
      basis: "project-defined",
    });

    // "gentler and shorter than A1" was the spec's requirement for this rung, so
    // it is asserted against A1 rather than against a remembered constant.
    const a1 = loadTaskShapeInventory("german", "A1");
    expect(inventory.administration.writtenMinutes).toBeLessThan(a1.administration.writtenMinutes);
    expect(inventory.administration.speakingMinutes).toBeLessThan(a1.administration.speakingMinutes);
  });

  it("is stricter than the Goethe rule it leads to, and says which way", () => {
    const inventory = loadTaskShapeInventory("german", "pre-A1");

    // Goethe-Zertifikat A1 awards the certificate on 60/100 overall and
    // publishes NO per-skill threshold, so one strong paper can carry a paper
    // that did nothing. This rung refuses that. Where the project is stricter
    // than its target it has to say so rather than imply the target agrees.
    expect(Object.values(inventory.passRule.independentSkillThresholds)).toEqual([0.6, 0.6, 0.6, 0.6]);
    expect(inventory.passRule.note).toContain("STRICTER");
    expect(inventory.passRule.note).toContain("no per-skill threshold at all");
    expect(inventory.passRule.note).toContain("no Goethe examination below A1");
  });

  it("scores the one German orthographic rule an English-trained hand cannot infer", () => {
    const inventory = loadTaskShapeInventory("german", "pre-A1");
    const writing = inventory.sections.find((section) => section.skill === "writing");
    const criteria = writing!.parts.flatMap((part) => part.scoring.criteria);

    // German capitalises EVERY noun, not only proper ones. Nothing in the sound
    // carries it and nothing in English practice predicts it, so it is scored by
    // name in all three writing parts rather than folded into "orthographic
    // control" where a rater could quietly forgive it.
    expect(criteria.filter((c) => c === "noun capitalised")).toHaveLength(3);
    expect(criteria).toContain("umlaut or ß retained");

    // And the reading part guards the mirror-image error: a capital letter in
    // German is not the sentence-position cue an English reader reads it as.
    const reading = inventory.sections.find((section) => section.skill === "reading");
    expect(reading!.parts.flatMap((part) => part.scoring.criteria))
      .toContain("capital-letter cue not mistaken for sentence position");
  });

  it("declares a published stimulus length, so reading reach can measure the rung", () => {
    const inventories = listTaskShapeInventories().map(({ language, level }) =>
      loadTaskShapeInventory(language, level));
    const report = measureReadingReach(loadLessons(), inventories);
    const row = report.rows.find((r) => r.language === "german" && r.level === "pre-A1");
    expect(row?.status).toBe("measurable");
    expect(row?.partsWithinReach).toBe(2);
  });
});
