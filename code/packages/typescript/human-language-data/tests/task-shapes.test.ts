import { describe, expect, it } from "vitest";
import { loadLanguageRegistry, loadTaskShapeInventory, listTaskShapeInventories } from "../src/loader.js";
import { buildTaskShapeBacklog, parseTaskShapeInventory } from "../src/task-shapes.js";

describe("four-skill task-shape inventories (HL18)", () => {
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
      { language: "french", level: "A1" },
      { language: "german", level: "A1" },
    ]);
    expect(backlog).toHaveLength(registry.languages.length * 6 - 2);
    expect(backlog.filter((item) => item.level === "A1")).toHaveLength(registry.languages.length - 2);
    expect(backlog.some((item) => item.id === "task-shape/french/A1")).toBe(false);
    expect(backlog.some((item) => item.id === "task-shape/french/A2")).toBe(true);
    expect(backlog.some((item) => item.id === "task-shape/german/A1")).toBe(false);
    expect(backlog.some((item) => item.id === "task-shape/german/A2")).toBe(true);
  });

  it("rejects a missing skill instead of treating it as exam-ready", () => {
    const value = JSON.parse(JSON.stringify(loadTaskShapeInventory("german", "A1")));
    value.sections = value.sections.filter((section: { skill: string }) => section.skill !== "writing");
    expect(() => parseTaskShapeInventory(value, "german")).toThrow(/missing writing section/);
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
  });
});
