import { describe, expect, it } from "vitest";
import { loadLanguageRegistry, loadTaskShapeInventory, listTaskShapeInventories } from "../src/loader.js";
import { buildTaskShapeBacklog, parseTaskShapeInventory } from "../src/task-shapes.js";

describe("four-skill task-shape inventories (HL18)", () => {
  it("loads the project-defined Gujarati pre-A1 floor with four separate 100-point papers", () => {
    const inventory = loadTaskShapeInventory("gujarati", "pre-A1");
    expect(inventory.target).toEqual({
      name: "Coding Adventures Gujarati pre-A1 Assessment — project-defined equivalent",
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
    expect(inventory.sections.map((section) =>
      section.parts.reduce((sum, part) => sum + (part.scoring.maxRawPoints ?? 0), 0)
    )).toEqual([100, 100, 100, 100]);
    expect(inventory.passRule).toMatchObject({ maximumPoints: 400, passPoints: 240 });
    expect(Object.values(inventory.passRule.independentSkillThresholds)).toEqual([0.6, 0.6, 0.6, 0.6]);
    expect(inventory.sections.find((section) => section.skill === "writing")?.parts.map((part) => part.id)).toEqual([
      "prea1-writing-delayed-recall",
      "prea1-writing-dictation",
      "prea1-writing-bounded-production",
    ]);
  });

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
    expect(Object.values(inventory.passRule.independentSkillThresholds)).toEqual([0.6, 0.6, 0.6, 0.6]);
    const paperPoints = inventory.sections.map((section) =>
      section.parts.reduce((sum, part) => sum + (part.scoring.maxRawPoints ?? 0), 0)
    );
    expect(paperPoints).toEqual([100, 100, 100, 100]);
    expect(paperPoints.reduce((sum, points) => sum + points, 0)).toBe(inventory.passRule.maximumPoints);
    expect(inventory.passRule).toMatchObject({ maximumPoints: 400, passPoints: 240 });
    expect(inventory.sections.find((section) => section.skill === "writing")?.parts.map((part) => part.id)).toEqual([
      "prea1-writing-delayed-recall",
      "prea1-writing-dictation",
      "prea1-writing-bounded-production",
    ]);
  });

  it("loads the official Spanish A1 performance target and its grouped pass rule", () => {
    const inventory = loadTaskShapeInventory("spanish", "A1");
    expect(inventory.target).toEqual({ name: "DELE A1", basis: "external" });
    expect(inventory.sections.map((section) => section.skill)).toEqual([
      "reading",
      "listening",
      "writing",
      "speaking",
    ]);
    expect(inventory.sections.flatMap((section) => section.parts)).toHaveLength(13);
    expect(inventory.administration).toMatchObject({
      writtenMinutes: 95,
      speakingMinutes: 10,
      speakingPreparationMinutes: 10,
    });
    expect(Object.values(inventory.passRule.independentSkillThresholds)).toEqual([null, null, null, null]);

    const reading = inventory.sections.find((section) => section.skill === "reading");
    expect(reading?.parts.map((part) => part.items)).toEqual([5, 6, 6, 8]);
    const listening = inventory.sections.find((section) => section.skill === "listening");
    expect(listening?.parts.map((part) => part.replayCount)).toEqual([2, 2, 2, 2]);
    const writing = inventory.sections.find((section) => section.skill === "writing");
    expect(writing?.parts.map((part) => part.responseLength)).toEqual([
      { unit: "words", minimum: 15, maximum: 25, approximate: false },
      { unit: "words", minimum: 30, maximum: 40, approximate: false },
    ]);
  });

  it("loads the external Arabic STAMP 4S target with a four-skill project A1 floor", () => {
    const inventory = loadTaskShapeInventory("arabic", "A1");
    expect(inventory.target).toEqual({
      name: "Avant STAMP 4S Arabic (Modern Standard) — Level 3 / Novice-High project A1 floor",
      basis: "external",
    });
    expect(inventory.sections.map((section) => section.skill)).toEqual([
      "reading",
      "listening",
      "writing",
      "speaking",
    ]);
    expect(inventory.sections.flatMap((section) => section.parts)).toHaveLength(4);
    expect(inventory.administration).toMatchObject({
      writtenMinutes: { minimum: 90, maximum: 105 },
      speakingMinutes: { minimum: 20, maximum: 25 },
      speakingPreparationMinutes: null,
    });
    expect(inventory.sections.map((section) => section.parts[0]?.items)).toEqual([30, 30, 3, 3]);
    expect(inventory.sections.map((section) => section.parts[0]?.scoring.maxRawPoints)).toEqual([null, null, null, null]);
    expect(Object.values(inventory.passRule.independentSkillThresholds)).toEqual([
      1 / 3,
      1 / 3,
      3 / 8,
      3 / 8,
    ]);
  });

  it("loads the project-defined Latin A1 target with four independent thresholds", () => {
    const inventory = loadTaskShapeInventory("latin", "A1");
    expect(inventory.target).toEqual({
      name: "Coding Adventures Latin A1 Assessment — project-defined equivalent",
      basis: "project-defined",
    });
    expect(inventory.sections.map((section) => section.skill)).toEqual([
      "reading",
      "listening",
      "writing",
      "speaking",
    ]);
    expect(inventory.sections.flatMap((section) => section.parts)).toHaveLength(12);
    expect(inventory.sections.map((section) =>
      section.parts.reduce((sum, part) => sum + (part.scoring.maxRawPoints ?? 0), 0)
    )).toEqual([25, 25, 25, 25]);
    expect(Object.values(inventory.passRule.independentSkillThresholds)).toEqual([0.6, 0.6, 0.6, 0.6]);
    expect(inventory.administration).toMatchObject({
      writtenMinutes: 90,
      speakingMinutes: 12,
      speakingPreparationMinutes: 10,
    });
  });

  it("loads the official French A1 performance target without flattening its forms", () => {
    const inventory = loadTaskShapeInventory("french", "A1");
    expect(inventory.target).toEqual({
      name: "DELF A1 tout public",
      basis: "external",
    });
    expect(inventory.sections.flatMap((section) => section.parts)).toHaveLength(14);
    expect(inventory.administration.speakingMinutes).toEqual({ minimum: 5, maximum: 7 });
    expect(Object.values(inventory.passRule.independentSkillThresholds)).toEqual([0.2, 0.2, 0.2, 0.2]);

    const listening = inventory.sections.find((section) => section.skill === "listening");
    expect(listening?.variants.map((variant) => variant.partIds.length)).toEqual([4, 5]);

    const writing = inventory.sections.find((section) => section.skill === "writing");
    expect(writing?.parts[1]?.responseLength).toEqual({
      unit: "words",
      minimum: 40,
      maximum: null,
      approximate: false,
    });

    const speaking = inventory.sections.find((section) => section.skill === "speaking");
    expect(speaking?.parts.map((part) => part.scoring.maxRawPoints)).toEqual([4, 4, 4]);
  });

  it("loads DILF as French's sourced pre-A1 target without inventing per-skill floors", () => {
    const inventory = loadTaskShapeInventory("french", "pre-A1");
    expect(inventory.target).toEqual({ name: "DILF A1.1", basis: "external" });
    expect(inventory.sections.map((section) => section.skill)).toEqual([
      "reading",
      "listening",
      "writing",
      "speaking",
    ]);
    expect(inventory.sections.map((section) => section.minutes)).toEqual([25, 25, 15, 10]);
    expect(inventory.sections.flatMap((section) => section.parts)).toHaveLength(17);
    expect(inventory.administration).toMatchObject({
      writtenMinutes: 65,
      speakingMinutes: 10,
      speakingGroupMaximum: 1,
      speakingPreparationMinutes: null,
    });
    expect(inventory.passRule).toMatchObject({
      maximumPoints: 100,
      passPoints: 50,
      requiresEverySectionAttempted: false,
    });
    expect(Object.values(inventory.passRule.independentSkillThresholds)).toEqual([null, null, null, null]);
    expect(inventory.sections.find((section) => section.skill === "speaking")?.parts.map((part) => part.id)).toEqual([
      "speaking-price-transaction",
      "speaking-present-person-or-place",
      "speaking-express-need-or-request-information",
      "speaking-describe-health-problem",
    ]);
  });

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

  it("keeps the corpus-wide research debt visible and round-robin", () => {
    const registry = loadLanguageRegistry();
    const present = listTaskShapeInventories();
    const backlog = buildTaskShapeBacklog(registry.languages.map((track) => track.id), present);
    expect(present).toEqual([
      { language: "spanish", level: "A1" },
      { language: "latin", level: "A1" },
      { language: "french", level: "pre-A1" },
      { language: "french", level: "A1" },
      { language: "german", level: "A1" },
      { language: "arabic", level: "A1" },
      { language: "marwadi", level: "pre-A1" },
      { language: "gujarati", level: "pre-A1" },
    ]);
    expect(backlog).toHaveLength(registry.languages.length * 7 - 8);
    expect(backlog.filter((item) => item.level === "pre-A1")).toHaveLength(registry.languages.length - 3);
    expect(backlog.filter((item) => item.level === "A1")).toHaveLength(registry.languages.length - 5);
    expect(backlog.some((item) => item.id === "task-shape/marwadi/pre-A1")).toBe(false);
    expect(backlog.some((item) => item.id === "task-shape/gujarati/pre-A1")).toBe(false);
    expect(backlog.some((item) => item.id === "task-shape/spanish/pre-A1")).toBe(true);
    expect(backlog.some((item) => item.id === "task-shape/spanish/A1")).toBe(false);
    expect(backlog.some((item) => item.id === "task-shape/spanish/A2")).toBe(true);
    expect(backlog.some((item) => item.id === "task-shape/latin/A1")).toBe(false);
    expect(backlog.some((item) => item.id === "task-shape/latin/A2")).toBe(true);
    expect(backlog.some((item) => item.id === "task-shape/french/pre-A1")).toBe(false);
    expect(backlog.some((item) => item.id === "task-shape/french/A1")).toBe(false);
    expect(backlog.some((item) => item.id === "task-shape/french/A2")).toBe(true);
    expect(backlog.some((item) => item.id === "task-shape/german/A1")).toBe(false);
    expect(backlog.some((item) => item.id === "task-shape/german/A2")).toBe(true);
    expect(backlog.some((item) => item.id === "task-shape/arabic/A1")).toBe(false);
    expect(backlog.some((item) => item.id === "task-shape/arabic/A2")).toBe(true);
  });

  it("rejects a missing skill instead of treating it as exam-ready", () => {
    const value = JSON.parse(JSON.stringify(loadTaskShapeInventory("german", "A1")));
    value.sections = value.sections.filter((section: { skill: string }) => section.skill !== "writing");
    expect(() => parseTaskShapeInventory(value, "german")).toThrow(/missing writing section/);
  });

  it("rejects a one-skill pre-A1 inventory instead of treating it as readiness", () => {
    const value = JSON.parse(JSON.stringify(loadTaskShapeInventory("marwadi", "pre-A1")));
    value.sections = value.sections.filter((section: { skill: string }) => section.skill === "reading");
    expect(() => parseTaskShapeInventory(value, "marwadi")).toThrow(/missing listening section/);
  });

  it("rejects a fully scored administration whose part ceilings do not reach its declared scale", () => {
    const value = JSON.parse(JSON.stringify(loadTaskShapeInventory("marwadi", "pre-A1")));
    value.sections[0].parts[0].scoring.maxRawPoints = 99;
    expect(() => parseTaskShapeInventory(value, "marwadi")).toThrow(
      /part maxRawPoints sum to 399, not passRule.maximumPoints 400/,
    );
  });

  it("rejects invented-looking null measurements without an explicit source gap", () => {
    const value = JSON.parse(JSON.stringify(loadTaskShapeInventory("german", "A1")));
    value.sections[0].parts[0].notPublished = [];
    expect(() => parseTaskShapeInventory(value, "german")).toThrow(/null measurements but no notPublished explanation/);
  });

  it("preserves a published duration range without inventing one exact minute", () => {
    const value = JSON.parse(JSON.stringify(loadTaskShapeInventory("german", "A1")));
    const range = { minimum: 5, maximum: 7 };
    value.administration.speakingMinutes = range;
    value.sections.find((section: { skill: string }) => section.skill === "speaking").minutes = range;
    expect(parseTaskShapeInventory(value, "german").administration.speakingMinutes).toEqual(range);
  });

  it("rejects a reversed or mismatched duration range", () => {
    const value = JSON.parse(JSON.stringify(loadTaskShapeInventory("german", "A1")));
    value.administration.speakingMinutes = { minimum: 7, maximum: 5 };
    expect(() => parseTaskShapeInventory(value, "german")).toThrow(/minimum cannot exceed maximum/);

    value.administration.speakingMinutes = { minimum: 5, maximum: 7 };
    expect(() => parseTaskShapeInventory(value, "german")).toThrow(/speaking minutes do not match/);
  });

  it("preserves alternate official part sets without administering their union", () => {
    const value = JSON.parse(JSON.stringify(loadTaskShapeInventory("german", "A1")));
    const listening = value.sections.find((section: { skill: string }) => section.skill === "listening");
    const [first, second, third] = listening.parts.map((part: { id: string }) => part.id);
    listening.variants = [
      { id: "short-form", partIds: [first, second], note: "published short form", sourceIds: ["goethe-a1-model-2024"] },
      { id: "long-form", partIds: [first, second, third], note: "published long form", sourceIds: ["goethe-a1-model-2024"] },
    ];
    const parsed = parseTaskShapeInventory(value, "german");
    expect(parsed.sections.find((section) => section.skill === "listening")?.variants).toHaveLength(2);
  });

  it("rejects variant sets that cite unknown parts or strand a union part", () => {
    const value = JSON.parse(JSON.stringify(loadTaskShapeInventory("german", "A1")));
    const listening = value.sections.find((section: { skill: string }) => section.skill === "listening");
    listening.variants = [
      { id: "bad-form", partIds: ["not-a-part"], note: "fixture", sourceIds: ["goethe-a1-model-2024"] },
    ];
    expect(() => parseTaskShapeInventory(value, "german")).toThrow(/cites unknown part/);

    listening.variants[0].partIds = [listening.parts[0].id];
    expect(() => parseTaskShapeInventory(value, "german")).toThrow(/variant sets omit part/);
  });

  it("preserves a published minimum when no maximum is published", () => {
    const value = JSON.parse(JSON.stringify(loadTaskShapeInventory("german", "A1")));
    const writing = value.sections.find((section: { skill: string }) => section.skill === "writing");
    writing.parts[1].responseLength = { unit: "words", minimum: 40, maximum: null, approximate: false };
    const parsed = parseTaskShapeInventory(value, "german");
    expect(parsed.sections.find((section) => section.skill === "writing")?.parts[1]?.responseLength).toEqual({
      unit: "words",
      minimum: 40,
      maximum: null,
      approximate: false,
    });
  });

  it("rejects a length with neither bound", () => {
    const value = JSON.parse(JSON.stringify(loadTaskShapeInventory("german", "A1")));
    value.sections[0].parts[0].stimulusLength = {
      unit: "words",
      minimum: null,
      maximum: null,
      approximate: false,
    };
    expect(() => parseTaskShapeInventory(value, "german")).toThrow(/must publish at least one finite bound/);
  });

  it("rejects path traversal at the loader boundary", () => {
    expect(() => loadTaskShapeInventory("../german", "A1")).toThrow(/unsafe/);
    expect(() => loadTaskShapeInventory("german", "A1\/../A2")).toThrow(/unsafe/);
    expect(() => loadTaskShapeInventory("marwadi", "pre-A1\/../A1")).toThrow(/unsafe/);
  });
});
