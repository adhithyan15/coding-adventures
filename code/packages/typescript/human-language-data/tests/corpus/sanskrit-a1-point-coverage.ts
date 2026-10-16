import { measureExamCoverage } from "../../src/exam-inventory.js";
import { loadEverything, loadExamInventory } from "../../src/loader.js";

type SanskritA1Coverage = ReturnType<typeof measureExamCoverage>;
type SanskritA1PointCoverage = SanskritA1Coverage["points"][number];

let cachedCoverage: SanskritA1Coverage | undefined;

/**
 * Give a feature shard only its own Sanskrit A1 point result.
 *
 * Aggregate totals stay behind this module boundary so adding one covered
 * point cannot make every historical feature owner edit the same counters.
 */
export function sanskritA1PointCoverage(pointId: string): SanskritA1PointCoverage {
  cachedCoverage ??= measureExamCoverage(
    loadExamInventory("sanskrit", "A1"),
    loadEverything().lessons,
  );
  const point = cachedCoverage.points.find((candidate) => candidate.id === pointId);
  if (!point) throw new Error(`Unknown Sanskrit A1 coverage point: ${pointId}`);
  return point;
}
