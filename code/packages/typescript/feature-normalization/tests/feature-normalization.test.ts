import { describe, expect, it } from "vitest";
import {
  fitMinMaxScaler,
  fitStandardScaler,
  transformMinMax,
  transformStandard,
} from "../src/index";

const rows = [
  [1000, 3, 1],
  [1500, 4, 0],
  [2000, 5, 1],
];

describe("feature normalization", () => {
  it("fits and applies standard scaling", () => {
    const scaler = fitStandardScaler(rows);
    expect(scaler.means).toEqual([1500, 4, 2 / 3]);

    const transformed = transformStandard(rows, scaler);
    expect(transformed[0][0]).toBeCloseTo(-1.224744871391589);
    expect(transformed[1][0]).toBeCloseTo(0);
    expect(transformed[2][0]).toBeCloseTo(1.224744871391589);
  });

  it("fits and applies min-max scaling", () => {
    const transformed = transformMinMax(rows, fitMinMaxScaler(rows));

    expect(transformed).toEqual([
      [0, 0, 1],
      [0.5, 0.5, 0],
      [1, 1, 1],
    ]);
  });

  it("maps constant columns to zero", () => {
    const constantRows = [[1, 7], [2, 7]];

    expect(transformStandard(constantRows, fitStandardScaler(constantRows))[0][1]).toBe(0);
    expect(transformMinMax(constantRows, fitMinMaxScaler(constantRows))[0][1]).toBe(0);
  });
});
