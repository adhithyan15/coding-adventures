import { describe, expect, it } from "vitest";
import { loadTaskShapeInventory } from "../../../src/loader.js";

describe("Sanskrit C2 task shapes", () => {
  it("defines the exact project-owned four-skill envelope with independent papers", () => {
    const inventory = loadTaskShapeInventory("sanskrit", "C2");

    expect(inventory).toMatchObject({ version: 1, language: "sanskrit", level: "C2" });
    expect(inventory.target).toEqual({
      name: "Coding Adventures Sanskrit C2 Assessment — project-defined equivalent",
      basis: "project-defined",
    });
    expect(inventory.sources.map((source) => source.id)).toEqual([
      "ca-hl16-assessment-policy",
      "ca-hl18-task-shapes",
      "ca-sanskrit-assessment",
    ]);
    expect(inventory.administration).toEqual({
      writtenMinutes: 240,
      speakingMinutes: 25,
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
    expect(inventory.sections.map((section) => section.minutes)).toEqual([90, 60, 90, 25]);
    expect(inventory.sections.map((section) => section.parts.map((part) => part.id))).toEqual([
      [
        "c2-reading-specialist-argument",
        "c2-reading-public-discourse",
        "c2-reading-specialist-synthesis",
        "c2-reading-literary-intertext",
      ],
      [
        "c2-listening-rapid-multiparty-interaction",
        "c2-listening-extended-argument",
        "c2-listening-stylistic-narrative",
        "c2-listening-culturally-dense-media",
      ],
      ["c2-writing-synthesis-reframing", "c2-writing-contrasting-extended-text"],
      [
        "c2-speaking-synthesis-mediation",
        "c2-speaking-multiparty-negotiation",
        "c2-speaking-defence-and-reformulation",
      ],
    ]);
    expect(inventory.sections.map((section) => section.parts.map((part) => part.items))).toEqual([
      [10, 10, 10, 10],
      [8, 8, 8, 8],
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
    expect(inventory.sections.every((section) => section.parts.every((part) =>
      JSON.stringify(part.sourceIds) === JSON.stringify([
        "ca-hl16-assessment-policy",
        "ca-hl18-task-shapes",
        "ca-sanskrit-assessment",
      ])
    ))).toBe(true);
  });

  it("pins every reading band and the 4,000–5,000-word total", () => {
    const inventory = loadTaskShapeInventory("sanskrit", "C2");
    const reading = inventory.sections.find((section) => section.skill === "reading");

    expect(reading?.parts.map((part) => part.stimulusLength)).toEqual([
      { unit: "words", minimum: 950, maximum: 1200, approximate: false },
      { unit: "words", minimum: 1000, maximum: 1250, approximate: false },
      { unit: "words", minimum: 1000, maximum: 1250, approximate: false },
      { unit: "words", minimum: 1050, maximum: 1300, approximate: false },
    ]);
    expect(reading?.parts.reduce((sum, part) =>
      sum + (part.stimulusLength?.minimum ?? 0), 0
    )).toBe(4000);
    expect(reading?.parts.reduce((sum, part) =>
      sum + (part.stimulusLength?.maximum ?? 0), 0
    )).toBe(5000);
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
    const inventory = loadTaskShapeInventory("sanskrit", "C2");
    const listening = inventory.sections.find((section) => section.skill === "listening");

    expect(listening?.parts.map((part) => part.stimulusLength)).toEqual([
      { unit: "seconds", minimum: 360, maximum: 480, approximate: false },
      { unit: "seconds", minimum: 420, maximum: 540, approximate: false },
      { unit: "seconds", minimum: 420, maximum: 540, approximate: false },
      { unit: "seconds", minimum: 420, maximum: 540, approximate: false },
    ]);
    expect(listening?.parts.every((part) =>
      part.promptModes.includes(
        "recorded Sanskrit at a natural 165-200 words per minute with variation",
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
      "multiple overlapping speakers including one documented regional Sanskrit pronunciation tradition",
      "multiple speakers including a second documented regional Sanskrit pronunciation tradition",
    ]);
  });

  it("pins source synthesis, writing lengths, speaking timings, and preparation", () => {
    const inventory = loadTaskShapeInventory("sanskrit", "C2");
    const writing = inventory.sections.find((section) => section.skill === "writing");
    const speaking = inventory.sections.find((section) => section.skill === "speaking");

    expect(writing?.parts.map((part) => part.stimulusLength)).toEqual([
      { unit: "words", minimum: 750, maximum: 1000, approximate: false },
      { unit: "words", minimum: 100, maximum: 180, approximate: false },
    ]);
    expect(writing?.parts.map((part) => part.responseLength)).toEqual([
      { unit: "words", minimum: 220, maximum: 280, approximate: false },
      { unit: "words", minimum: 420, maximum: 520, approximate: false },
    ]);
    expect(writing?.parts[0]?.promptModes).toContain(
      "three short written Devanagari Sanskrit sources with external sandhi in contrasting genres or registers",
    );
    expect(writing?.parts[0]?.responseModes).toContain(
      "independent handwritten Sanskrit synthesis reframed for a named audience in Devanagari with external sandhi",
    );
    expect(writing?.parts[0]?.scoring.criteria).toEqual(expect.arrayContaining([
      "accurate multi-source synthesis",
      "selection and reframing",
    ]));

    expect(speaking?.parts.map((part) => part.responseLength)).toEqual([
      { unit: "seconds", minimum: 330, maximum: 360, approximate: false },
      { unit: "seconds", minimum: 360, maximum: 450, approximate: false },
      { unit: "seconds", minimum: 390, maximum: 480, approximate: false },
    ]);
    expect(speaking?.parts.map((part) => part.stimulusLength)).toEqual([
      { unit: "words", minimum: 550, maximum: 750, approximate: false },
      { unit: "words", minimum: 80, maximum: 130, approximate: false },
      { unit: "seconds", minimum: 20, maximum: 90, approximate: false },
    ]);
    expect(speaking?.parts[0]?.promptModes).toContain(
      "multiple short written Devanagari Sanskrit sources with external sandhi",
    );
    expect(speaking?.parts[0]?.scoring.criteria).toContain("source synthesis and mediation");
    expect(speaking?.parts[0]?.aids.allowed).toContain(
      "paper notes made during fifteen-minute preparation",
    );
    expect(speaking?.parts.every((part) =>
      part.aids.forbidden.includes("written answer script")
      && part.aids.forbidden.includes("electronic notes")
    )).toBe(true);
  });

  it("keeps Sanskrit script, sandhi, pronunciation, and scope boundaries explicit", () => {
    const inventory = loadTaskShapeInventory("sanskrit", "C2");
    const reading = inventory.sections.find((section) => section.skill === "reading");
    const listening = inventory.sections.find((section) => section.skill === "listening");
    const writing = inventory.sections.find((section) => section.skill === "writing");
    const speaking = inventory.sections.find((section) => section.skill === "speaking");

    expect(reading?.parts.every((part) =>
      part.promptModes.includes("written-devanagari Sanskrit with external sandhi")
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
    expect([reading, listening, writing, speaking].every((section) =>
      section?.parts.every((part) =>
        part.aids.forbidden.includes("dictionary")
        && part.aids.forbidden.includes("grammar reference")
        && part.aids.forbidden.includes("translator")
        && part.aids.forbidden.includes("generative assistant")
      )
    )).toBe(true);
    expect(inventory.passRule.note).toContain("Classical Sanskrit");
    expect(inventory.passRule.note).toContain("Vedic accent and morphology remain out of scope");
    expect(inventory.passRule.note).toContain(
      "project-defined assessment, not an external qualification",
    );
    expect(inventory.passRule.note).not.toContain("learner proven");
    expect(inventory.passRule.note).not.toContain("mock ready");
  });
});
