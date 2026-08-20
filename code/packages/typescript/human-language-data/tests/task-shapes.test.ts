import { describe, expect, it } from "vitest";
import { loadLanguageRegistry, loadTaskShapeInventory, listTaskShapeInventories } from "../src/loader.js";
import { buildTaskShapeBacklog, parseTaskShapeInventory } from "../src/task-shapes.js";

describe("four-skill task-shape inventories (HL18)", () => {
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
    expect(inventory.passRule).toMatchObject({ maximumPoints: 100, passPoints: 60 });
    expect(Object.values(inventory.passRule.independentSkillThresholds)).toEqual([null, null, null, null]);
  });

  it("keeps the corpus-wide research debt visible and round-robin", () => {
    const registry = loadLanguageRegistry();
    const present = listTaskShapeInventories();
    const backlog = buildTaskShapeBacklog(registry.languages.map((track) => track.id), present);
    expect(present).toEqual([{ language: "german", level: "A1" }]);
    expect(backlog).toHaveLength(registry.languages.length * 6 - 1);
    expect(backlog.filter((item) => item.level === "A1")).toHaveLength(registry.languages.length - 1);
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

  it("rejects path traversal at the loader boundary", () => {
    expect(() => loadTaskShapeInventory("../german", "A1")).toThrow(/unsafe/);
    expect(() => loadTaskShapeInventory("german", "A1\/../A2")).toThrow(/unsafe/);
  });
});
