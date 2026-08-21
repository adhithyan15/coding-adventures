import { describe, expect, it } from "vitest";
import { loadLanguageRegistry, loadTaskShapeInventory, listTaskShapeInventories } from "../src/loader.js";
import { buildTaskShapeBacklog, parseTaskShapeInventory } from "../src/task-shapes.js";

describe("four-skill task-shape inventories (HL18)", () => {
  it("keeps the corpus-wide research debt visible and round-robin", () => {
    const registry = loadLanguageRegistry();
    const present = listTaskShapeInventories();
    const backlog = buildTaskShapeBacklog(registry.languages.map((track) => track.id), present);
    const levels = ["pre-A1", "A1", "A2", "B1", "B2", "C1", "C2"];
    const registryOrder = registry.languages.map((track) => track.id);
    const presentIds = new Set(present.map((item) => `${item.language}/${item.level}`));
    const backlogIds = new Set(backlog.map((item) => item.id.replace("task-shape/", "")));

    expect(presentIds.size).toBe(present.length);
    expect(backlogIds.size).toBe(backlog.length);
    expect(present).toEqual([...present].sort((left, right) => {
      const languageDifference = registryOrder.indexOf(left.language) - registryOrder.indexOf(right.language);
      return languageDifference || levels.indexOf(left.level) - levels.indexOf(right.level);
    }));
    expect(backlog).toHaveLength(registry.languages.length * levels.length - present.length);

    for (const language of registryOrder) {
      for (const level of levels) {
        const id = `${language}/${level}`;
        expect(Number(presentIds.has(id)) + Number(backlogIds.has(id))).toBe(1);
      }
    }
    for (const level of levels) {
      const presentAtLevel = present.filter((item) => item.level === level).length;
      expect(backlog.filter((item) => item.level === level)).toHaveLength(registry.languages.length - presentAtLevel);
    }
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
    value.sections[0].parts[0].scoring.maxRawPoints = 39;
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
