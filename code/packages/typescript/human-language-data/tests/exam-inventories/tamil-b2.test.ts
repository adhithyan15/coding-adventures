import { describe, expect, it } from "vitest";
import { loadExamInventory, loadTaskShapeInventory, loadTrackLessons } from "../../src/loader.js";
import {
  EXAM_CONTENT_DIMENSIONS,
  formatExamCoverage,
  isExamInventoryComplete,
  measureExamCoverage,
} from "../../src/exam-inventory.js";

describe("the committed Tamil B2 inventory", () => {
  const inventory = loadExamInventory("tamil", "B2");

  it("is a source-bounded partial project inventory, not a borrowed certificate", () => {
    expect(inventory.about).toContain("PROJECT-DEFINED");
    expect(inventory.about).toContain("NOT AN EXTERNAL TAMIL QUALIFICATION");
    expect(isExamInventoryComplete(inventory)).toBe(false);
    expect(Object.keys(inventory.scope).sort()).toEqual([...EXAM_CONTENT_DIMENSIONS].sort());
    for (const dimension of EXAM_CONTENT_DIMENSIONS) {
      expect(inventory.scope[dimension].status, dimension).toBe("partial");
      expect(inventory.scope[dimension].source.trim().length, dimension).toBeGreaterThan(20);
      expect(inventory.scope[dimension].note.trim().length, dimension).toBeGreaterThan(40);
    }
  });

  it("names each primary boundary without promoting a corpus or school ladder to B2", () => {
    const anchors = (inventory as unknown as {
      anchors: { id: string; kind: string; url: string; note: string }[];
    }).anchors;
    expect(anchors.map((anchor) => anchor.id)).toEqual([
      "CEFR-CV-2020",
      "TVA-CERTIFICATE-RULES",
      "UNICODE-18-TAMIL",
      "LDCIL-TAMIL-TEXT",
      "LDCIL-TAMIL-RAW-SPEECH",
      "LDCIL-TAMIL-SENTENCE-SPEECH",
      "CA-TAMIL-B2-TASK-SHAPE",
    ]);
    for (const anchor of anchors) {
      expect(anchor.url).toMatch(/^https:\/\//);
      expect(anchor.note.trim().length, anchor.id).toBeGreaterThan(40);
    }
    expect(inventory.scope.lexicon.note).toContain("not a B2 word list");
    expect(inventory.scope.grammar.note).toContain("not a finite Tamil B2 grammar");
  });

  it("preserves the Tamil variety boundary instead of borrowing a generic register claim", () => {
    expect(inventory.about).toContain("contemporary standard written Tamil");
    expect(inventory.about).toContain("standard spoken Tamil");
    const labels = inventory.points.map((point) => point.label).join("\n");
    expect(labels).toContain("contemporary spoken, neutral written, and formal written Tamil");
    expect(labels).toContain("documented regional Tamil voices");
  });

  it("enumerates all four dimensions with stable Tamil-owned ids", () => {
    expect(inventory.points).toHaveLength(36);
    expect(new Set(inventory.points.map((point) => point.id)).size).toBe(36);
    expect(inventory.points.every((point) => /^TA-B2-(FUNC|GRAM|ORTH|LEX)-\d{2}$/.test(point.id))).toBe(true);
    const categories = new Map<string, number>();
    for (const point of inventory.points) {
      categories.set(point.category, (categories.get(point.category) ?? 0) + 1);
      expect(point).toHaveProperty("probe");
      expect(Array.isArray(point.probe) ? point.probe.length : 1, point.id).toBeGreaterThan(0);
    }
    expect([...categories.values()].sort((a, b) => a - b)).toEqual([9, 9, 9, 9]);
  });

  it("covers the B2 task genres without confusing task shape with content coverage", () => {
    const labels = inventory.points.map((point) => point.label).join("\n");
    for (const demand of [
      "reasoned argument",
      "reported feature",
      "detailed procedure",
      "literary or cultural prose",
      "multiparty discussion",
      "collaborative problem",
      "formal interaction text",
    ]) expect(labels).toContain(demand);

    expect(loadTaskShapeInventory("tamil", "B2").sections.map((section) => section.skill)).toEqual([
      "reading",
      "listening",
      "writing",
      "speaking",
    ]);
  });

  it("pins conservative first coverage without turning foundations into readiness", () => {
    const coverage = measureExamCoverage(inventory, loadTrackLessons("tamil"));
    expect(coverage).toMatchObject({
      enumerated: 36,
      covered: 0,
      unmapped: 36,
      percent: 0,
      inventoryComplete: false,
      byCategory: {
        "Communicative functions and discourse": { enumerated: 9, covered: 0 },
        "Tamil grammar and register": { enumerated: 9, covered: 0 },
        "Tamil script, sound, and orthography": { enumerated: 9, covered: 0 },
        "Lexicon and genre domains": { enumerated: 9, covered: 0 },
      },
    });
    expect(formatExamCoverage(coverage)).toContain("tamil B2 (partial inventory)");
  }, 60_000);
});
