import { describe, expect, it } from "vitest";
import { loadTaskShapeInventory } from "../../src/loader.js";

describe("Marwadi task-shape inventories", () => {
  it("loads the project-defined Marwadi pre-A1 floor as four separate 100-point papers", () => {
    const inventory = loadTaskShapeInventory("marwadi", "pre-A1");
    expect(inventory.target).toEqual({
      name: "Coding Adventures Marwadi pre-A1 Assessment — project-defined equivalent",
      basis: "project-defined",
    });
    expect(inventory.sections.map((section) => section.skill)).toEqual([
      "reading",
      "listening",
      "writing",
      "speaking",
    ]);
    expect(inventory.administration).toMatchObject({
      writtenMinutes: 32,
      speakingMinutes: 8,
      speakingPreparationMinutes: 0,
    });
    expect(inventory.sections.map((section) => section.minutes)).toEqual([10, 10, 12, 8]);
    expect(inventory.sections.map((section) => section.parts.map((part) => part.id))).toEqual([
      [
        "prea1-reading-signs-and-words",
        "prea1-reading-phrases-and-notices",
        "prea1-reading-personal-details",
      ],
      [
        "prea1-listening-sounds-and-words",
        "prea1-listening-greeting-responses",
        "prea1-listening-personal-details",
      ],
      [
        "prea1-writing-delayed-recall",
        "prea1-writing-dictation",
        "prea1-writing-bounded-production",
      ],
      [
        "prea1-speaking-greeting-and-identity",
        "prea1-speaking-familiar-questions",
        "prea1-speaking-short-request",
      ],
    ]);
    expect(inventory.sections.map((section) => section.parts.map((part) => part.items))).toEqual([
      [6, 4, 4],
      [6, 4, 4],
      [2, 4, 2],
      [2, 3, 1],
    ]);
    expect(Object.values(inventory.passRule.independentSkillThresholds)).toEqual([0.6, 0.6, 0.6, 0.6]);
    const paperPoints = inventory.sections.map((section) =>
      section.parts.reduce((sum, part) => sum + (part.scoring.maxRawPoints ?? 0), 0)
    );
    expect(paperPoints).toEqual([100, 100, 100, 100]);
    expect(paperPoints.reduce((sum, points) => sum + points, 0)).toBe(inventory.passRule.maximumPoints);
    expect(inventory.passRule).toMatchObject({ maximumPoints: 400, passPoints: 240 });
  });

  it("loads the project-defined Marwadi A1 destination as an executable four-paper envelope", () => {
    const inventory = loadTaskShapeInventory("marwadi", "A1");
    expect(inventory.target).toEqual({
      name: "Coding Adventures Marwadi A1 Assessment — project-defined equivalent",
      basis: "project-defined",
    });
    expect(inventory.administration).toMatchObject({
      writtenMinutes: 58,
      speakingMinutes: 10,
      speakingPreparationMinutes: 5,
    });
    expect(inventory.sections.map((section) => section.minutes)).toEqual([20, 18, 20, 10]);
    expect(inventory.sections.map((section) => section.parts.map((part) => part.id))).toEqual([
      ["a1-reading-signs-and-forms", "a1-reading-short-messages", "a1-reading-personal-descriptions"],
      ["a1-listening-announcements", "a1-listening-short-exchanges", "a1-listening-personal-account"],
      ["a1-writing-practical-form", "a1-writing-reader-purpose-message"],
      ["a1-speaking-personal-interview", "a1-speaking-prepared-description", "a1-speaking-transactional-role-play"],
    ]);
    expect(inventory.sections.map((section) => section.parts.map((part) => part.items))).toEqual([
      [7, 7, 6],
      [6, 6, 5],
      [1, 1],
      [5, 1, 1],
    ]);

    const [reading, listening, writing] = inventory.sections;
    expect(reading?.parts.reduce((sum, part) => sum + (part.stimulusLength?.minimum ?? 0), 0)).toBe(250);
    expect(reading?.parts.reduce((sum, part) => sum + (part.stimulusLength?.maximum ?? 0), 0)).toBe(350);
    expect(reading?.parts.every((part) =>
      part.promptGenres.every((genre) => genre.includes("no source over 90 words"))
    )).toBe(true);
    expect(listening?.parts.every((part) =>
      part.promptModes.includes("recorded Marwari at 90-110 words per minute") && part.replayCount === 2
    )).toBe(true);
    expect(writing?.parts.map((part) => part.responseLength)).toEqual([
      { unit: "items", minimum: 6, maximum: 8, approximate: false },
      { unit: "words", minimum: 30, maximum: 40, approximate: false },
    ]);

    const paperPoints = inventory.sections.map((section) =>
      section.parts.reduce((sum, part) => sum + (part.scoring.maxRawPoints ?? 0), 0)
    );
    expect(paperPoints).toEqual([100, 100, 100, 100]);
    expect(Object.values(inventory.passRule.independentSkillThresholds)).toEqual([0.6, 0.6, 0.6, 0.6]);
    expect(inventory.passRule).toMatchObject({ maximumPoints: 400, passPoints: 240 });
  });

  it("defines a gentle project-owned A2 envelope without inventing external authority", () => {
    const inventory = loadTaskShapeInventory("marwadi", "A2");

    expect(inventory.sources.some((source) => source.id === "ca-marwadi-assessment" &&
      source.url.endsWith("/marwadi/assessment-spec.md") &&
      source.published === "2026-08-20" && source.accessed === "2026-10-02")).toBe(true);
    expect(inventory.target).toEqual({
      name: "Coding Adventures Marwadi A2 Assessment — project-defined equivalent",
      basis: "project-defined",
    });
    expect(inventory.administration).toMatchObject({
      writtenMinutes: 85,
      speakingMinutes: 12,
      speakingPreparationMinutes: 5,
    });
    expect(inventory.sections.map((section) => section.skill)).toEqual([
      "reading",
      "listening",
      "writing",
      "speaking",
    ]);
    expect(inventory.sections.map((section) => section.minutes)).toEqual([30, 25, 30, 12]);
    expect(inventory.sections.map((section) => section.parts.map((part) => part.items))).toEqual([
      [8, 8, 8],
      [7, 7, 6],
      [1, 1],
      [6, 1, 1],
    ]);

    const [reading, listening, writing] = inventory.sections;
    expect(reading?.parts.every((part) => part.promptModes.includes("written-devanagari"))).toBe(true);
    expect(reading?.parts.reduce((sum, part) => sum + (part.stimulusLength?.minimum ?? 0), 0)).toBe(550);
    expect(reading?.parts.reduce((sum, part) => sum + (part.stimulusLength?.maximum ?? 0), 0)).toBe(750);
    expect(listening?.parts.every((part) =>
      part.promptModes.includes("recorded Marwari at 110-130 words per minute") && part.replayCount === 2,
    )).toBe(true);
    expect(writing?.parts.map((part) => part.responseLength)).toEqual([
      { unit: "words", minimum: 25, maximum: 35, approximate: false },
      { unit: "words", minimum: 70, maximum: 90, approximate: false },
    ]);
    expect(writing?.parts.every((part) =>
      ["copyable answer model", "romanization", "Hindi replacement", "dictionary", "translator", "spell-checker"]
        .every((aid) => part.aids.forbidden.includes(aid)),
    )).toBe(true);

    const paperPoints = inventory.sections.map((section) =>
      section.parts.reduce((sum, part) => sum + (part.scoring.maxRawPoints ?? 0), 0),
    );
    expect(paperPoints).toEqual([100, 100, 100, 100]);
    expect(Object.values(inventory.passRule.independentSkillThresholds)).toEqual([0.6, 0.6, 0.6, 0.6]);
    expect(inventory.passRule).toMatchObject({ maximumPoints: 400, passPoints: 240 });
  });

  it("pins the project-owned B1 four-skill paper to the Marwadi specification", () => {
    const inventory = loadTaskShapeInventory("marwadi", "B1");
    expect(inventory.target).toEqual({
      name: "Coding Adventures Marwadi B1 Assessment — project-defined equivalent",
      basis: "project-defined",
    });
    expect(inventory.sources.some((source) => source.id === "ca-marwadi-assessment" &&
      source.url.endsWith("/marwadi/assessment-spec.md") &&
      source.published === "2026-08-20" && source.accessed === "2026-11-14")).toBe(true);
    expect(inventory.administration).toMatchObject({
      writtenMinutes: 125, speakingMinutes: 15, speakingPreparationMinutes: 10,
    });
    expect(inventory.sections.map((section) => section.minutes)).toEqual([45, 35, 45, 15]);
    expect(inventory.sections.map((section) => section.parts.length)).toEqual([4, 4, 2, 4]);

    const [reading, listening, writing, speaking] = inventory.sections;
    expect(reading?.parts.reduce((sum, part) => sum + part.items, 0)).toBe(28);
    expect(reading?.parts.reduce((sum, part) => sum + (part.stimulusLength?.minimum ?? 0), 0)).toBe(1100);
    expect(reading?.parts.reduce((sum, part) => sum + (part.stimulusLength?.maximum ?? 0), 0)).toBe(1400);
    expect(reading?.parts.every((part) => part.aids.forbidden.includes("Hindi replacement"))).toBe(true);
    expect(listening?.parts.map((part) => part.replayCount)).toEqual([2, 2, 1, 1]);
    expect(listening?.parts.every((part) =>
      part.promptModes.includes("recorded Marwari at 130-150 words per minute"))).toBe(true);
    expect(listening?.parts.every((part) => part.aids.forbidden.includes("romanization"))).toBe(true);
    expect(writing?.parts.map((part) => part.responseLength)).toEqual([
      { unit: "words", minimum: 50, maximum: 70, approximate: false },
      { unit: "words", minimum: 130, maximum: 170, approximate: false },
    ]);
    expect(speaking?.parts[1]?.responseLength?.maximum).toBe(180);
    expect(writing?.parts.every((part) => part.responseModes.some((mode) => mode.includes("Devanagari")))).toBe(true);
    expect(writing?.parts.every((part) =>
      ["copyable answer model", "romanization", "Hindi replacement", "dictionary", "translator", "spell-checker"]
        .every((aid) => part.aids.forbidden.includes(aid)))).toBe(true);
    expect(inventory.sections.map((section) =>
      section.parts.reduce((sum, part) => sum + (part.scoring.maxRawPoints ?? 0), 0))).toEqual([100, 100, 100, 100]);
    expect(Object.values(inventory.passRule.independentSkillThresholds)).toEqual([0.6, 0.6, 0.6, 0.6]);
  });
});
