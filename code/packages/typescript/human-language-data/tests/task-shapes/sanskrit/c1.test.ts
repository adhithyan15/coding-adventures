import { describe, expect, it } from "vitest";
import { loadTaskShapeInventory } from "../../../src/loader.js";

describe("Sanskrit C1 task shapes", () => {
  it("defines the exact project-owned four-skill envelope with independent papers", () => {
    const inventory = loadTaskShapeInventory("sanskrit", "C1");

    expect(inventory).toMatchObject({ version: 1, language: "sanskrit", level: "C1" });
    expect(inventory.target).toEqual({
      name: "Coding Adventures Sanskrit C1 Assessment — project-defined equivalent",
      basis: "project-defined",
    });
    expect(inventory.administration).toEqual({
      writtenMinutes: 200,
      speakingMinutes: 22,
      speakingGroupMaximum: 3,
      speakingPreparationMinutes: 15,
      deliveryModes: ["paper", "recorded-audio", "live-interlocutor"],
    });
    expect(inventory.sections.map((section) => section.skill)).toEqual([
      "reading",
      "listening",
      "writing",
      "speaking",
    ]);
    expect(inventory.sections.map((section) => section.minutes)).toEqual([75, 50, 75, 22]);
    expect(inventory.sections.map((section) => section.parts.map((part) => part.id))).toEqual([
      [
        "c1-reading-dense-public-text",
        "c1-reading-professional-text",
        "c1-reading-academic-text",
        "c1-reading-literary-text",
      ],
      [
        "c1-listening-unscripted-discussion",
        "c1-listening-lecture",
        "c1-listening-interview",
        "c1-listening-narrative",
      ],
      ["c1-writing-source-synthesis", "c1-writing-extended-response"],
      [
        "c1-speaking-source-based-presentation",
        "c1-speaking-collaborative-synthesis",
        "c1-speaking-challenge-and-defence",
      ],
    ]);
    expect(inventory.sections.map((section) => section.parts.map((part) => part.items))).toEqual([
      [9, 9, 9, 9],
      [7, 7, 7, 7],
      [1, 1],
      [1, 1, 6],
    ]);
    expect(inventory.passRule).toMatchObject({
      maximumPoints: 400,
      passPoints: 240,
      requiresEverySectionAttempted: true,
      independentSkillThresholds: {
        reading: 0.6,
        listening: 0.6,
        writing: 0.6,
        speaking: 0.6,
      },
    });
    expect(inventory.sections.map((section) =>
      section.parts.reduce((sum, part) => sum + (part.scoring.maxRawPoints ?? 0), 0)
    )).toEqual([100, 100, 100, 100]);
  });

  it("pins every reading band and the 2,800–3,500-word total", () => {
    const inventory = loadTaskShapeInventory("sanskrit", "C1");
    const reading = inventory.sections.find((section) => section.skill === "reading");

    expect(reading?.parts.map((part) => part.stimulusLength)).toEqual([
      { unit: "words", minimum: 650, maximum: 800, approximate: false },
      { unit: "words", minimum: 700, maximum: 850, approximate: false },
      { unit: "words", minimum: 700, maximum: 880, approximate: false },
      { unit: "words", minimum: 750, maximum: 970, approximate: false },
    ]);
    expect(reading?.parts.reduce((sum, part) =>
      sum + (part.stimulusLength?.minimum ?? 0), 0
    )).toBe(2800);
    expect(reading?.parts.reduce((sum, part) =>
      sum + (part.stimulusLength?.maximum ?? 0), 0
    )).toBe(3500);
    expect(reading?.parts.map((part) => part.responseLength)).toEqual(
      Array.from({ length: 4 }, () => ({
        unit: "items",
        minimum: 1,
        maximum: 1,
        approximate: false,
      })),
    );
  });

  it("pins natural one-play listening with two documented regional traditions", () => {
    const inventory = loadTaskShapeInventory("sanskrit", "C1");
    const listening = inventory.sections.find((section) => section.skill === "listening");

    expect(listening?.parts.map((part) => part.stimulusLength)).toEqual([
      { unit: "seconds", minimum: 300, maximum: 420, approximate: false },
      { unit: "seconds", minimum: 360, maximum: 480, approximate: false },
      { unit: "seconds", minimum: 330, maximum: 450, approximate: false },
      { unit: "seconds", minimum: 330, maximum: 450, approximate: false },
    ]);
    expect(listening?.parts.every((part) =>
      part.promptModes.includes(
        "recorded Sanskrit at 160-185 words per minute with natural variation",
      )
      && part.replayCount === 1
      && part.aids.allowed.includes("one unscored orienting preview")
      && part.aids.forbidden.includes("transcript")
      && part.aids.forbidden.includes("audio controls")
    )).toBe(true);

    const regionalModes = listening?.parts.flatMap((part) =>
      part.promptModes.filter((mode) =>
        mode.includes("documented regional Sanskrit pronunciation tradition")
      )
    );
    expect(regionalModes).toEqual([
      "multiple speakers including one documented regional Sanskrit pronunciation tradition",
      "multiple speakers including a second documented regional Sanskrit pronunciation tradition",
    ]);
  });

  it("pins source synthesis, writing lengths, speaking timings, and preparation", () => {
    const inventory = loadTaskShapeInventory("sanskrit", "C1");
    const writing = inventory.sections.find((section) => section.skill === "writing");
    const speaking = inventory.sections.find((section) => section.skill === "speaking");

    expect(writing?.parts.map((part) => part.stimulusLength)).toEqual([
      { unit: "words", minimum: 450, maximum: 650, approximate: false },
      { unit: "words", minimum: 90, maximum: 150, approximate: false },
    ]);
    expect(writing?.parts.map((part) => part.responseLength)).toEqual([
      { unit: "words", minimum: 180, maximum: 220, approximate: false },
      { unit: "words", minimum: 300, maximum: 380, approximate: false },
    ]);
    expect(writing?.parts[0]?.promptModes).toContain(
      "two short written Devanagari Sanskrit source texts with external sandhi",
    );
    expect(writing?.parts[0]?.responseModes).toContain(
      "independent handwritten Sanskrit synthesis in Devanagari with external sandhi",
    );
    expect(writing?.parts[0]?.scoring.criteria).toEqual(expect.arrayContaining([
      "accurate source synthesis",
      "selection and integration",
    ]));

    expect(speaking?.parts.map((part) => part.responseLength)).toEqual([
      { unit: "seconds", minimum: 270, maximum: 300, approximate: false },
      { unit: "seconds", minimum: 300, maximum: 360, approximate: false },
      { unit: "seconds", minimum: 330, maximum: 420, approximate: false },
    ]);
    expect(speaking?.parts.map((part) => part.stimulusLength)).toEqual([
      { unit: "words", minimum: 350, maximum: 500, approximate: false },
      { unit: "words", minimum: 60, maximum: 100, approximate: false },
      { unit: "seconds", minimum: 20, maximum: 75, approximate: false },
    ]);
    expect(speaking?.parts[0]?.promptModes).toContain(
      "two short written Devanagari Sanskrit sources with external sandhi",
    );
    expect(speaking?.parts[0]?.scoring.criteria).toContain("source synthesis");
    expect(speaking?.parts[0]?.aids.allowed).toContain(
      "paper notes made during fifteen-minute preparation",
    );
    expect(speaking?.parts.every((part) =>
      part.aids.forbidden.includes("written answer script")
      && part.aids.forbidden.includes("electronic notes")
    )).toBe(true);
  });

  it("keeps Sanskrit script, sandhi, pronunciation, and scope boundaries explicit", () => {
    const inventory = loadTaskShapeInventory("sanskrit", "C1");
    const reading = inventory.sections.find((section) => section.skill === "reading");
    const writing = inventory.sections.find((section) => section.skill === "writing");
    const speaking = inventory.sections.find((section) => section.skill === "speaking");

    expect(reading?.parts.every((part) =>
      part.promptModes.includes("written-devanagari with external sandhi")
      && part.scoring.criteria.includes("external-sandhi decoding")
    )).toBe(true);
    expect(writing?.parts.every((part) =>
      part.responseModes.some((mode) =>
        mode.includes("Sanskrit")
        && mode.includes("Devanagari with external sandhi")
      )
      && part.scoring.criteria.includes("Sanskrit vocabulary and grammar")
      && part.scoring.criteria.includes("external sandhi")
      && part.scoring.criteria.includes("word-final visarga or anusvara")
      && part.scoring.criteria.includes("continuous śirorekhā and word-boundary control")
      && part.aids.forbidden.includes("copyable answer model")
      && part.aids.forbidden.includes("romanization")
      && part.aids.forbidden.includes("non-Devanagari response")
      && part.aids.forbidden.includes("dictionary")
      && part.aids.forbidden.includes("translator")
      && part.aids.forbidden.includes("spell-checker")
    )).toBe(true);
    expect(speaking?.parts.every((part) =>
      part.scoring.criteria.includes("Sanskrit vocabulary and grammar")
      && part.scoring.criteria.includes("pronunciation and fluency in a documented tradition")
    )).toBe(true);
    expect(inventory.passRule.note).toContain("Classical Sanskrit");
    expect(inventory.passRule.note).toContain("Vedic accent and morphology remain out of scope");
    expect(inventory.passRule.note).toContain(
      "project-defined assessment, not an external qualification",
    );
  });
});
