import { describe, expect, it } from "vitest";
import { listTaskShapeInventories, loadLessons, loadTaskShapeInventory } from "../../src/loader.js";
import { measureReadingReach } from "../../src/reading-reach.js";

describe("Spanish task-shape inventories", () => {
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

  it("transcribes the official DELE A2 structure, tarea by tarea", () => {
    const inventory = loadTaskShapeInventory("spanish", "A2");
    expect(inventory.target).toEqual({ name: "DELE A2", basis: "external" });
    expect(inventory.administration).toMatchObject({
      writtenMinutes: 145,
      speakingMinutes: 12,
      speakingPreparationMinutes: 12,
    });

    // 60 + 40 + 45 written, and the item counts per tarea. Both totals are 25,
    // which is the arithmetic the guide states and the loader independently
    // re-checks against administration.writtenMinutes.
    const reading = inventory.sections.find((section) => section.skill === "reading");
    const listening = inventory.sections.find((section) => section.skill === "listening");
    expect(reading?.parts.map((part) => part.items)).toEqual([5, 8, 6, 6]);
    expect(listening?.parts.map((part) => part.items)).toEqual([6, 6, 6, 7]);
    expect(reading?.parts.reduce((sum, part) => sum + (part.items ?? 0), 0)).toBe(25);
    expect(listening?.parts.reduce((sum, part) => sum + (part.items ?? 0), 0)).toBe(25);

    // The two writing tasks are where A2 stops being A1 with longer texts: the
    // candidate writes 130-150 words in total, against A1's 45-65.
    const writing = inventory.sections.find((section) => section.skill === "writing");
    expect(writing?.parts.map((part) => part.responseLength)).toEqual([
      { unit: "words", minimum: 60, maximum: 70, approximate: false },
      { unit: "words", minimum: 70, maximum: 80, approximate: false },
    ]);
  });

  it("records the grouped pass rule as READING-WITH-WRITING, which is the counter-intuitive half", () => {
    const inventory = loadTaskShapeInventory("spanish", "A2");

    // The obvious guess -- and what one secondary summary of the Cervantes page
    // asserted while this file was being written -- is that DELE groups the two
    // receptive skills together and the two productive ones together. It does
    // not. Grupo 1 is comprensión de lectura + expresión e interacción ESCRITAS,
    // and Grupo 2 is comprensión auditiva + expresión e interacción ORALES.
    //
    // The difference is not cosmetic: under the real rule a candidate who reads
    // well and writes badly can fail Grupo 1 outright while clearing 60 points
    // overall. Pinning the wording is what stops a future editor from "fixing"
    // it into the intuitive pairing.
    expect(inventory.passRule.note).toContain("Grupo 1 is comprensión de lectura plus expresión e interacción escritas");
    expect(inventory.passRule.note).toContain("Grupo 2 is comprensión auditiva plus expresión e interacción orales");
    expect(inventory.passRule.note).toContain("60/100 alone is not sufficient");

    // Cervantes publishes no per-skill threshold, so these stay null rather than
    // being invented and misattributed to the awarding body.
    expect(Object.values(inventory.passRule.independentSkillThresholds)).toEqual([null, null, null, null]);
  });

  it("measures how far Spanish's own reading is from the A2 inputs it now declares", () => {
    const inventories = listTaskShapeInventories().map(({ language, level }) =>
      loadTaskShapeInventory(language, level));
    const report = measureReadingReach(loadLessons(), inventories);
    const row = report.rows.find((r) => r.language === "spanish" && r.level === "A2");

    // Declaring the inventory is what makes this measurable at all, and the
    // first measurement is not flattering: the track's longest comprehension
    // passage is 61 words and only ONE of the four reading tareas has a minimum
    // input that short. That is the honest state of an A2 rung whose reading
    // content has not been written yet, and it is better recorded than left
    // invisible.
    expect(row?.status).toBe("measurable");
    expect(row?.partsMeasurable).toBe(4);
    expect(row?.partsWithinReach).toBe(1);
  });
});
