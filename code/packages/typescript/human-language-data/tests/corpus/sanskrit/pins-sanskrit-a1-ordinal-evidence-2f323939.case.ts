import { expect, it } from "vitest";
import { loadExamInventory } from "../../../src/loader.js";
import { sanskritA1PointCoverage } from "../sanskrit-a1-point-coverage.js";

it("keeps the Sanskrit A1 ordinal point's independently owned evidence exact", () => {
  const ordinals = loadExamInventory("sanskrit", "A1").points.find(
    (point) => point.id === "SA-A1-Q-02",
  );
  expect(ordinals?.probe).toEqual([
    "SA-LEX-C62-ORDINAL-01",
    "SA-GRAMMAR-C62-ORDINAL-MA-02",
    "SA-LEX-C62-ORDINAL-03",
    "SA-LEX-C62-ORDINAL-04",
    "SA-LEX-C62-ORDINAL-05",
    "SA-LEX-C62-ORDINAL-06",
    "SA-LEX-C63-ORDINAL-01",
    "SA-GRAMMAR-C63-ORDINAL-THA-02",
    "SA-LEX-C63-ORDINAL-03",
    "SA-LEX-C63-ORDINAL-04",
    "SA-GRAMMAR-C63-ORDINAL-TIYA-05",
    "SA-LEX-C63-ORDINAL-06",
    "SA-LEX-C63-ORDINAL-07",
  ]);
  expect(sanskritA1PointCoverage("SA-A1-Q-02")).toMatchObject({
    covered: true,
    missingAtoms: [],
    unmapped: false,
  });
});
