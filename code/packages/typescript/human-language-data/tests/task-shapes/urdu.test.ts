import { describe, expect, it } from "vitest";
import { loadTaskShapeInventory } from "../../src/loader.js";

describe("Urdu task-shape inventories", () => {
  it("loads the project-defined Urdu A1 envelope with its full four-skill boundary", () => {
    const inventory = loadTaskShapeInventory("urdu", "A1");
    expect(inventory.target).toEqual({
      name: "Coding Adventures Urdu A1 Assessment — project-defined equivalent",
      basis: "project-defined",
    });
    expect(inventory.administration).toMatchObject({
      writtenMinutes: 58,
      speakingMinutes: 10,
      speakingPreparationMinutes: 5,
    });
    expect(inventory.sections.map((section) => [section.skill, section.minutes])).toEqual([
      ["reading", 20],
      ["listening", 18],
      ["writing", 20],
      ["speaking", 10],
    ]);
    expect(inventory.sections.map((section) =>
      section.parts.reduce((sum, part) => sum + (part.scoring.maxRawPoints ?? 0), 0)
    )).toEqual([100, 100, 100, 100]);
    expect(Object.values(inventory.passRule.independentSkillThresholds)).toEqual([0.6, 0.6, 0.6, 0.6]);
  });

  it("keeps the published A1 input, response, replay, and timing limits exact", () => {
    const inventory = loadTaskShapeInventory("urdu", "A1");
    const reading = inventory.sections.find((section) => section.skill === "reading");
    expect(reading?.parts.reduce((sum, part) => sum + part.items, 0)).toBe(20);
    expect(reading?.parts.reduce((sum, part) => sum + (part.stimulusLength?.minimum ?? 0), 0)).toBe(250);
    expect(reading?.parts.reduce((sum, part) => sum + (part.stimulusLength?.maximum ?? 0), 0)).toBe(350);
    expect(reading?.parts.every((part) => part.promptGenres.every((genre) => genre.includes("no source over 90 words")))).toBe(true);

    const listening = inventory.sections.find((section) => section.skill === "listening");
    expect(listening?.parts.reduce((sum, part) => sum + part.items, 0)).toBe(17);
    expect(listening?.parts.every((part) => part.replayCount === 2)).toBe(true);
    expect(listening?.parts.every((part) =>
      part.promptModes.includes("recorded contemporary standard Urdu at 90-110 words per minute")
    )).toBe(true);

    const writing = inventory.sections.find((section) => section.skill === "writing");
    expect(writing?.parts.map((part) => part.id)).toEqual([
      "a1-writing-practical-form",
      "a1-writing-reader-purpose-message",
    ]);
    expect(writing?.parts[0]?.responseLength).toMatchObject({ unit: "items", minimum: 6, maximum: 8 });
    expect(writing?.parts[1]?.responseLength).toMatchObject({ unit: "words", minimum: 30, maximum: 40 });

    const speaking = inventory.sections.find((section) => section.skill === "speaking");
    expect(speaking?.parts.map((part) => part.id)).toEqual([
      "a1-speaking-personal-interview",
      "a1-speaking-prepared-description",
      "a1-speaking-transactional-role-play",
    ]);
    expect(speaking?.parts[1]?.responseLength).toMatchObject({ unit: "seconds", minimum: 60, maximum: 60 });
  });

  it("preserves Urdu script, accessibility, and scoring rather than borrowing a generic Latin-script contract", () => {
    const inventory = loadTaskShapeInventory("urdu", "A1");
    const parts = inventory.sections.flatMap((section) => section.parts);
    const written = parts.filter((part) => part.promptModes.some((mode) => mode.includes("written Urdu")));
    expect(written.length).toBeGreaterThan(0);
    expect(written.every((part) =>
      part.promptModes.some((mode) => mode.includes("approved Nastaliq or accessibility Naskh"))
    )).toBe(true);

    for (const section of inventory.sections) {
      for (const part of section.parts) {
        expect(part.aids.forbidden).toContain("Roman Urdu");
        expect(part.aids.forbidden).toContain("Devanagari substitution");
      }
    }

    const writing = inventory.sections.find((section) => section.skill === "writing");
    expect(writing?.parts.every((part) => part.scoring.criteria.includes(
      "range and control of Urdu vocabulary, grammar, and register",
    ))).toBe(true);
    expect(writing?.parts.every((part) => part.scoring.criteria.includes("Urdu orthographic control"))).toBe(true);
    expect(writing?.parts.every((part) => part.notPublished.some((rule) => rule.includes("unwritten short vowels")))).toBe(true);
    expect(writing?.parts.every((part) => part.notPublished.some((rule) => rule.includes("calligraphic ligatures")))).toBe(true);
  });

  it("loads the project-defined Urdu A2 envelope with four independent 100-point papers", () => {
    const inventory = loadTaskShapeInventory("urdu", "A2");
    expect(inventory.target).toEqual({
      name: "Coding Adventures Urdu A2 Assessment — project-defined equivalent",
      basis: "project-defined",
    });
    expect(inventory.administration).toMatchObject({
      writtenMinutes: 85,
      speakingMinutes: 12,
      speakingPreparationMinutes: 5,
    });
    expect(inventory.sections.map((section) => [section.skill, section.minutes])).toEqual([
      ["reading", 30],
      ["listening", 25],
      ["writing", 30],
      ["speaking", 12],
    ]);
    expect(inventory.sections.map((section) =>
      section.parts.reduce((sum, part) => sum + (part.scoring.maxRawPoints ?? 0), 0)
    )).toEqual([100, 100, 100, 100]);
    expect(Object.values(inventory.passRule.independentSkillThresholds)).toEqual([0.6, 0.6, 0.6, 0.6]);
  });

  it("keeps the published Urdu A2 task, length, speed, replay, and timing boundaries exact", () => {
    const inventory = loadTaskShapeInventory("urdu", "A2");
    const reading = inventory.sections.find((section) => section.skill === "reading");
    expect(reading?.parts.reduce((sum, part) => sum + part.items, 0)).toBe(24);
    expect(reading?.parts.reduce((sum, part) => sum + (part.stimulusLength?.minimum ?? 0), 0)).toBe(550);
    expect(reading?.parts.reduce((sum, part) => sum + (part.stimulusLength?.maximum ?? 0), 0)).toBe(750);

    const listening = inventory.sections.find((section) => section.skill === "listening");
    expect(listening?.parts.reduce((sum, part) => sum + part.items, 0)).toBe(20);
    expect(listening?.parts.every((part) => part.replayCount === 2)).toBe(true);
    expect(listening?.parts.every((part) =>
      part.promptModes.includes("recorded contemporary standard Urdu at 110-130 words per minute")
    )).toBe(true);
    expect(listening?.parts[2]?.stimulusLength).toMatchObject({ unit: "seconds", minimum: 120, maximum: 180 });

    const writing = inventory.sections.find((section) => section.skill === "writing");
    expect(writing?.parts.map((part) => part.responseLength)).toMatchObject([
      { unit: "words", minimum: 25, maximum: 35 },
      { unit: "words", minimum: 70, maximum: 90 },
    ]);

    const speaking = inventory.sections.find((section) => section.skill === "speaking");
    expect(speaking?.parts.map((part) => part.id)).toEqual([
      "a2-speaking-personal-interview",
      "a2-speaking-prepared-account",
      "a2-speaking-information-exchange-role-play",
    ]);
    expect(speaking?.parts[1]?.responseLength).toMatchObject({ unit: "seconds", minimum: 90, maximum: 120 });
  });

  it("makes A2 genuinely Urdu-script and keeps its language-specific scoring rules", () => {
    const inventory = loadTaskShapeInventory("urdu", "A2");
    const parts = inventory.sections.flatMap((section) => section.parts);
    expect(parts.every((part) => part.aids.forbidden.includes("Roman Urdu"))).toBe(true);
    expect(parts.every((part) => part.aids.forbidden.includes("Devanagari substitution"))).toBe(true);

    const written = parts.filter((part) => part.promptModes.some((mode) => mode.includes("written Urdu")));
    expect(written.length).toBeGreaterThan(0);
    expect(written.every((part) =>
      part.promptModes.some((mode) => mode.includes("approved Nastaliq or accessibility Naskh"))
    )).toBe(true);

    const writing = inventory.sections.find((section) => section.skill === "writing");
    expect(writing?.parts.every((part) => part.scoring.criteria.includes(
      "range and control of Urdu vocabulary, grammar, and register",
    ))).toBe(true);
    expect(writing?.parts.every((part) => part.scoring.criteria.includes("Urdu orthographic control"))).toBe(true);
    expect(writing?.parts.every((part) => part.notPublished.some((rule) => rule.includes("unwritten short vowels")))).toBe(true);
    expect(writing?.parts.every((part) => part.notPublished.some((rule) => rule.includes("calligraphic ligatures")))).toBe(true);
  });

  it("makes B1 an exact four-paper contract without losing Urdu's script boundary", () => {
    const inventory = loadTaskShapeInventory("urdu", "B1");
    expect(inventory.target).toEqual({
      name: "Coding Adventures Urdu B1 Assessment — project-defined equivalent",
      basis: "project-defined",
    });
    expect(inventory.administration).toMatchObject({
      writtenMinutes: 125,
      speakingMinutes: 15,
      speakingPreparationMinutes: 10,
    });
    expect(inventory.sections.map((section) => section.minutes)).toEqual([45, 35, 45, 15]);
    expect(inventory.sections.map((section) => section.parts.map((part) => part.items))).toEqual([
      [7, 7, 7, 7],
      [7, 6, 6, 6],
      [1, 1],
      [5, 1, 1, 4],
    ]);

    const [reading, listening, writing, speaking] = inventory.sections;
    expect(reading?.parts.reduce((sum, part) => sum + (part.stimulusLength?.minimum ?? 0), 0)).toBe(1100);
    expect(reading?.parts.reduce((sum, part) => sum + (part.stimulusLength?.maximum ?? 0), 0)).toBe(1400);
    expect(listening?.parts.every((part) =>
      part.promptModes.includes("recorded contemporary standard Urdu at 130-150 words per minute")
    )).toBe(true);
    expect(listening?.parts.map((part) => part.replayCount)).toEqual([2, 2, 1, 1]);
    expect(writing?.parts.map((part) => part.responseLength)).toEqual([
      { unit: "words", minimum: 50, maximum: 70, approximate: false },
      { unit: "words", minimum: 130, maximum: 170, approximate: false },
    ]);
    expect(speaking?.parts[1]?.responseLength).toEqual({
      unit: "seconds",
      minimum: 150,
      maximum: 180,
      approximate: false,
    });
    expect(inventory.sections.map((section) =>
      section.parts.reduce((sum, part) => sum + (part.scoring.maxRawPoints ?? 0), 0)
    )).toEqual([100, 100, 100, 100]);
    expect(Object.values(inventory.passRule.independentSkillThresholds)).toEqual([0.6, 0.6, 0.6, 0.6]);

    const parts = inventory.sections.flatMap((section) => section.parts);
    expect(parts.every((part) => part.aids.forbidden.includes("Roman Urdu"))).toBe(true);
    expect(parts.every((part) => part.aids.forbidden.includes("Devanagari substitution"))).toBe(true);
    const written = parts.filter((part) => part.promptModes.some((mode) => mode.includes("written Urdu")));
    expect(written.length).toBeGreaterThan(0);
    expect(written.every((part) =>
      part.promptModes.some((mode) => mode.includes("approved Nastaliq or accessibility Naskh"))
    )).toBe(true);
    expect(writing?.parts.every((part) => part.scoring.criteria.includes(
      "range and control of Urdu vocabulary, grammar, and register",
    ))).toBe(true);
    expect(writing?.parts.every((part) => part.scoring.criteria.includes("Urdu orthographic control"))).toBe(true);
    expect(writing?.parts.every((part) => part.notPublished.some((rule) => rule.includes("unwritten short vowels")))).toBe(true);
    expect(writing?.parts.every((part) => part.notPublished.some((rule) => rule.includes("calligraphic ligatures")))).toBe(true);
  });
});
