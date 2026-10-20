import { readFileSync, readdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { expect, it } from "vitest";
import type {
  ExamCoverage,
  ExamInventory,
} from "../../../src/exam-inventory.js";
import {
  formatExamCoverage,
  measureExamCoverage,
} from "../../../src/exam-inventory.js";
import { loadEverything, loadExamInventory } from "../../../src/loader.js";

interface CoverageGapOwner {
  readonly pointId: string;
  readonly unmapped: boolean;
  readonly missingAtoms: readonly string[];
}

const OWNER_NAME = /^(SA-A1-[A-Z0-9-]+)\.json$/;
const ownerDirectory = join(
  dirname(fileURLToPath(import.meta.url)),
  "a1-coverage-gaps.d",
);

/**
 * Load the exceptional states, not a second copy of every covered result.
 *
 * Every inventory point without a gap owner must measure covered. An unmapped
 * point owns one tiny file; a partial point additionally owns the exact atoms
 * it still lacks. Closing one point therefore removes only that point's file.
 * The aggregate totals and categories remain exact because they are derived
 * from the complete direct point inventory plus this exact exceptional set.
 */
function loadCoverageGapOwners(
  directory = ownerDirectory,
): ReadonlyMap<string, CoverageGapOwner> {
  const owners = new Map<string, CoverageGapOwner>();
  const foldedNames = new Set<string>();

  for (const entry of readdirSync(directory, { withFileTypes: true })) {
    if (!entry.isFile()) {
      throw new Error(
        `Sanskrit A1 coverage gap owner '${entry.name}' must be a regular file`,
      );
    }
    const match = OWNER_NAME.exec(entry.name);
    if (match === null) {
      throw new Error(
        `unexpected Sanskrit A1 coverage gap owner '${entry.name}'`,
      );
    }
    const folded = entry.name.toLowerCase();
    if (foldedNames.has(folded)) {
      throw new Error(
        `Sanskrit A1 coverage gap owner case-fold collision: '${entry.name}'`,
      );
    }
    foldedNames.add(folded);

    const raw = readFileSync(join(directory, entry.name), "utf8");
    const parsed: unknown = JSON.parse(raw);
    if (
      typeof parsed !== "object" ||
      parsed === null ||
      Array.isArray(parsed)
    ) {
      throw new Error(
        `Sanskrit A1 coverage gap owner '${entry.name}' must be an object`,
      );
    }
    const owner = parsed as Record<string, unknown>;
    if (Object.keys(owner).join(",") !== "pointId,unmapped,missingAtoms") {
      throw new Error(
        `Sanskrit A1 coverage gap owner '${entry.name}' must have exact canonical keys`,
      );
    }
    const pointId = match[1]!;
    if (
      owner.pointId !== pointId ||
      typeof owner.unmapped !== "boolean" ||
      !Array.isArray(owner.missingAtoms) ||
      owner.missingAtoms.some((atom) => typeof atom !== "string")
    ) {
      throw new Error(
        `Sanskrit A1 coverage gap owner '${entry.name}' must bind its filename and state`,
      );
    }
    const missingAtoms = owner.missingAtoms as string[];
    if (new Set(missingAtoms).size !== missingAtoms.length) {
      throw new Error(
        `Sanskrit A1 coverage gap owner '${entry.name}' repeats a missing atom`,
      );
    }
    if (
      (owner.unmapped && missingAtoms.length !== 0) ||
      (!owner.unmapped && missingAtoms.length === 0)
    ) {
      throw new Error(
        `Sanskrit A1 coverage gap owner '${entry.name}' must describe an unmapped or partial point`,
      );
    }
    if (raw !== `${JSON.stringify(parsed, null, 2)}\n`) {
      throw new Error(
        `Sanskrit A1 coverage gap owner '${entry.name}' must use canonical JSON`,
      );
    }
    if (owners.has(pointId)) {
      throw new Error(`duplicate Sanskrit A1 coverage gap owner '${pointId}'`);
    }
    owners.set(pointId, {
      pointId,
      unmapped: owner.unmapped,
      missingAtoms,
    });
  }

  return owners;
}

function assertCoverageMatchesOwners(
  coverage: ExamCoverage,
  inventory: ExamInventory,
  owners: ReadonlyMap<string, CoverageGapOwner>,
): void {
  const inventoryIds = new Set(inventory.points.map((point) => point.id));
  expect(
    [...owners.keys()].filter((pointId) => !inventoryIds.has(pointId)).sort(),
    "every gap owner names a canonical inventory point",
  ).toEqual([]);

  const expectedPoints = inventory.points.map((point) => {
    const gap = owners.get(point.id);
    return {
      id: point.id,
      covered: gap === undefined,
      unmapped: gap?.unmapped ?? false,
      missingAtoms: [...(gap?.missingAtoms ?? [])],
    };
  });
  expect(
    coverage.points.map(({ id, covered, unmapped, missingAtoms }) => ({
      id,
      covered,
      unmapped,
      missingAtoms,
    })),
    "every Sanskrit A1 point matches its direct gap ownership",
  ).toEqual(expectedPoints);

  const enumerated = inventory.points.length;
  const covered = enumerated - owners.size;
  const unmapped = [...owners.values()].filter(
    (owner) => owner.unmapped,
  ).length;
  const partial = owners.size - unmapped;
  const percent =
    enumerated === 0 ? 0 : Math.round((covered / enumerated) * 100);
  expect({
    enumerated: coverage.enumerated,
    covered: coverage.covered,
    unmapped: coverage.unmapped,
    partial: coverage.partial,
    percent: coverage.percent,
  }).toEqual({ enumerated, covered, unmapped, partial, percent });

  const categoryTally = new Map<
    string,
    { enumerated: number; covered: number }
  >();
  for (const point of inventory.points) {
    const bucket = categoryTally.get(point.category) ?? {
      enumerated: 0,
      covered: 0,
    };
    bucket.enumerated += 1;
    if (!owners.has(point.id)) bucket.covered += 1;
    categoryTally.set(point.category, bucket);
  }
  expect(coverage.byCategory).toEqual(Object.fromEntries(categoryTally));

  expect(
    coverage.points.filter((point) => !point.covered).map((point) => point.id),
  ).toEqual(
    inventory.points
      .filter((point) => owners.has(point.id))
      .map((point) => point.id),
  );
  expect(formatExamCoverage(coverage).split("\n").slice(0, 2)).toEqual([
    `sanskrit A1 (partial inventory): ${covered}/${enumerated} points covered (${percent}%)`,
    `  ${unmapped} with no corresponding atom, ${partial} only partly taught`,
  ]);
}

// Whole-corpus parsing is deliberately shared at module scope. The corpus has
// outgrown Vitest's per-test timeout; recomputing it inside each assertion turns
// a deterministic census into a timing failure as the books grow.
const lessons = loadEverything().lessons;
const inventory = loadExamInventory("sanskrit", "A1");
const measuredCoverage = measureExamCoverage(inventory, lessons);
const gapOwners = loadCoverageGapOwners();

it("pins Sanskrit A1 exam coverage, which nothing read before", () => {
  // Keep the load, measurement, and direct read in this block. The repository's
  // inventory census source-scans that complete shape so merely computing a
  // report at module scope cannot masquerade as a coverage pin.
  const pinnedInventory = loadExamInventory("sanskrit", "A1");
  const coverage = measureExamCoverage(pinnedInventory, lessons);
  expect(coverage.enumerated).toBe(pinnedInventory.points.length);
  assertCoverageMatchesOwners(coverage, inventory, gapOwners);
});

it("fails closed when an independently owned gap disappears", () => {
  const withoutOneGap = new Map(gapOwners);
  withoutOneGap.delete(withoutOneGap.keys().next().value!);
  expect(() =>
    assertCoverageMatchesOwners(measuredCoverage, inventory, withoutOneGap),
  ).toThrow();
});

it("fails closed when a gap owner names a non-canonical point", () => {
  const withUnknownGap = new Map(gapOwners);
  withUnknownGap.set("SA-A1-NOT-A-POINT", {
    pointId: "SA-A1-NOT-A-POINT",
    unmapped: true,
    missingAtoms: [],
  });
  expect(() =>
    assertCoverageMatchesOwners(measuredCoverage, inventory, withUnknownGap),
  ).toThrow();
});
