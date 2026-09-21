import { expect, it } from "vitest";
import {
  formatExamCoverage,
  measureExamCoverage,
  trackIntroducedAtoms,
} from "../../../src/exam-inventory.js";
import { loadExamInventory, loadTrackLessons } from "../../../src/loader.js";

it("probes only Hindi atoms that EXIST, so a guessed id cannot under-report", () => {
  const lessons = loadTrackLessons("hindi");
  const taught = trackIntroducedAtoms(lessons, "hindi");
  const unknown: string[] = [];
  for (const point of loadExamInventory("hindi", "A1").points) {
    for (const atom of point.probe ?? []) if (!taught.has(atom)) unknown.push(`${point.id}:${atom}`);
  }
  expect(unknown).toEqual([]);
}, 60_000);

it("pins Hindi A1 coverage, and the numeral column the ordinal tranche moved", () => {
  const inventory = loadExamInventory("hindi", "A1");
  const coverage = measureExamCoverage(inventory, loadTrackLessons("hindi"));
  const mapped = inventory.points.filter((point) => point.probe !== null);
  const unmapped = inventory.points.filter((point) => point.probe === null);

  expect(coverage.enumerated).toBe(inventory.points.length);
  expect(coverage.covered).toBe(mapped.length);
  expect(coverage.unmapped).toBe(unmapped.length);
  expect(coverage.partial).toBe(0);
  expect(coverage.points.find((point) => point.id === "HI-A1-NUM-04")).toMatchObject({
    covered: true,
    missingAtoms: [],
    unmapped: false,
  });
  expect(coverage.points.filter(
    (point) => point.category === "Sankhya-vachak (quantifiers and numerals)",
  )).toHaveLength(7);
  expect(formatExamCoverage(coverage)).toContain(
    `hindi A1 (partial inventory): ${coverage.covered}/${coverage.enumerated} points covered (${coverage.percent}%)`,
  );
}, 60_000);
