import { expect, it } from "vitest";
import {
  formatExamCoverage,
  measureExamCoverage,
  trackIntroducedAtoms,
} from "../../../src/exam-inventory.js";
import { loadExamInventory, loadTrackLessons } from "../../../src/loader.js";

it("probes only Malayalam atoms that EXIST, so a guessed id cannot under-report", () => {
  const lessons = loadTrackLessons("malayalam");
  const taught = trackIntroducedAtoms(lessons, "malayalam");
  const unknown: string[] = [];
  for (const point of loadExamInventory("malayalam", "A1").points) {
    for (const atom of point.probe ?? []) if (!taught.has(atom)) unknown.push(`${point.id}:${atom}`);
  }
  expect(unknown).toEqual([]);
}, 60_000);

it("pins Malayalam A1 coverage, and the ordinal point the tranche closed", () => {
  const inventory = loadExamInventory("malayalam", "A1");
  const coverage = measureExamCoverage(inventory, loadTrackLessons("malayalam"));
  const mapped = inventory.points.filter((point) => point.probe !== null);
  const unmapped = inventory.points.filter((point) => point.probe === null);

  expect(coverage.enumerated).toBe(inventory.points.length);
  expect(coverage.covered).toBe(mapped.length);
  expect(coverage.unmapped).toBe(unmapped.length);
  expect(coverage.partial).toBe(0);
  expect(coverage.points.find((point) => point.id === "ML-A1-NUM-05")).toMatchObject({
    covered: true,
    missingAtoms: [],
    unmapped: false,
  });
  expect(coverage.points.filter(
    (point) => point.category === "Sankhya (numerals and quantity)",
  )).toHaveLength(9);
  expect(formatExamCoverage(coverage)).toContain(
    `malayalam A1 (partial inventory): ${coverage.covered}/${coverage.enumerated} points covered (${coverage.percent}%)`,
  );
}, 60_000);
