import { describe, expect, it } from "vitest";
import {
  VERSION,
  inverseStandardRow,
  loadEnergyEfficiencyRows,
  prepareEnergyData,
} from "../src/index";

describe("energy-efficiency-multi-output-predictor", () => {
  it("has a version", () => {
    expect(VERSION).toBe("0.1.0");
  });

  it("loads the included UCI energy efficiency data", () => {
    const rows = loadEnergyEfficiencyRows();

    expect(rows).toHaveLength(768);
    expect(rows[0].features).toHaveLength(8);
    expect(rows[0].targets).toEqual([15.55, 21.33]);
  });

  it("prepares m input and n output matrices", () => {
    const data = prepareEnergyData(loadEnergyEfficiencyRows().slice(0, 8));

    expect(data.normalizedFeatures[0]).toHaveLength(8);
    expect(data.normalizedTargets[0]).toHaveLength(2);
    expect(inverseStandardRow(data.normalizedTargets[0], data.targetScaler)[0]).toBeCloseTo(15.55);
  });
});
