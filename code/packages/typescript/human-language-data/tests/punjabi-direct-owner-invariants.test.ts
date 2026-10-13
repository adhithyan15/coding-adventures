import { readFileSync } from "node:fs";
import { expect, it } from "vitest";

const PUNJABI_CASE_ROOT = new URL("./corpus/punjabi/", import.meta.url);

function caseSource(name: string): string {
  return readFileSync(new URL(name, PUNJABI_CASE_ROOT), "utf8");
}

it("keeps Punjabi's session-map invariant independent of an assembled row count", () => {
  const source = caseSource(
    "keeps-punjabi-s-272-row-session-map-aligned-with-canonical-order-a11bbd2f.case.ts",
  );
  expect(source).not.toMatch(/toHaveLength\(\d+\)/);
  expect(source).toContain("Array.from({ length: rows.length }");
});

it("lets Punjabi lesson owners grow without a shared generated lesson total", () => {
  const source = caseSource("pins-punjabi-lesson-content-budgets-6d3a195a.case.ts");
  expect(source).not.toMatch(/\blessons:\s*\d+/);
  expect(source).toContain('expectLanguageLessonBudgets("punjabi"');
});

it("keeps the Chapter 3 continuity owner scoped to Chapter 3", () => {
  const source = caseSource(
    "closes-chapter-3-s-oral-r1-r4-windows-without-inventing-script-credit-1a34da76.case.ts",
  );
  expect(source).not.toContain("report.summary.missedByWindow");
  expect(source).toContain("chapter3Atoms.has(defect.atom)");
});

it("pins first writing-stage owners without copying every later repetition", () => {
  const source = caseSource(
    "pins-punjabi-s-complete-pre-a1-writing-runway-f8d07819.case.ts",
  );
  expect(source).not.toMatch(/validEvidence\.map\([\s\S]*?\)\.toEqual\(\[/);
  expect(source).toContain("firstEvidenceByStage");
});
