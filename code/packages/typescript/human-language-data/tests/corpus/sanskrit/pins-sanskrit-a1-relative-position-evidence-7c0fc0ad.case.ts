import { expect, it } from "vitest";
import { loadExamInventory } from "../../../src/loader.js";
import { sanskritA1PointCoverage } from "../sanskrit-a1-point-coverage.js";

it("keeps the Sanskrit A1 relative-position evidence independently owned", () => {
  const relativePosition = loadExamInventory("sanskrit", "A1").points.find(
    (point) => point.id === "SA-A1-SP-02",
  );
  expect(relativePosition).toMatchObject({
    label: "relative position: left, right, above, below, in front, behind",
    probe: [
      "SA-LEX-C93-TIME93-03",
      "SA-LEX-C93-TIME93-04",
      "SA-LEX-C94-LOC94-02",
      "SA-LEX-C94-LOC94-03",
      "SA-LEX-C94-LOC94-04",
      "SA-LEX-C94-LOC94-05",
    ],
  });
  expect(sanskritA1PointCoverage("SA-A1-SP-02")).toMatchObject({
    covered: true,
    missingAtoms: [],
    unmapped: false,
  });
});
