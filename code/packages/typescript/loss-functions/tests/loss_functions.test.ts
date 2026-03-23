import { mse, mae, bce, cce, mseDerivative, maeDerivative, bceDerivative, cceDerivative } from "../src/loss_functions";

describe("Loss Functions", () => {
  const yTrue = [1.0, 0.0];
  const yPred = [0.9, 0.1];

  function almostEqual(a: number, b: number): boolean {
    return Math.abs(a - b) <= 1e-6;
  }

  test("MSE calculates correctly", () => {
    expect(almostEqual(mse(yTrue, yPred), 0.010)).toBe(true);
  });

  test("MAE calculates correctly", () => {
    expect(almostEqual(mae(yTrue, yPred), 0.100)).toBe(true);
  });

  test("BCE calculates correctly", () => {
    expect(almostEqual(bce(yTrue, yPred), 0.1053605)).toBe(true);
  });

  test("CCE calculates correctly", () => {
    expect(almostEqual(cce(yTrue, yPred), 0.0526802)).toBe(true);
  });

  test("Errors on mismatch lengths", () => {
    expect(() => mse([1.0], yPred)).toThrow();
    expect(() => mae([1.0], yPred)).toThrow();
    expect(() => bce([1.0], yPred)).toThrow();
    expect(() => cce([1.0], yPred)).toThrow();
  });

  test("Errors on empty arrays", () => {
    expect(() => mse([], [])).toThrow();
    expect(() => mae([], [])).toThrow();
    expect(() => bce([], [])).toThrow();
    expect(() => cce([], [])).toThrow();
  });

  test("Identical slices return 0 for MAE and MSE", () => {
    const identical = [1.0, 0.5, 0.0];
    expect(almostEqual(mse(identical, identical), 0.0)).toBe(true);
    expect(almostEqual(mae(identical, identical), 0.0)).toBe(true);
  });

  test("MSE Derivative calculates correctly", () => {
    const res = mseDerivative([1.0, 0.0], [0.8, 0.2]);
    expect(almostEqual(res[0], -0.2)).toBe(true);
    expect(almostEqual(res[1], 0.2)).toBe(true);
  });

  test("MAE Derivative calculates correctly", () => {
    const res = maeDerivative([1.0, 0.0, 0.5], [0.8, 0.2, 0.5]);
    expect(almostEqual(res[0], -1.0/3.0)).toBe(true);
    expect(almostEqual(res[1], 1.0/3.0)).toBe(true);
    expect(almostEqual(res[2], 0.0)).toBe(true);
  });

  test("BCE Derivative calculates correctly", () => {
    const res = bceDerivative([1.0, 0.0], [0.8, 0.2]);
    expect(almostEqual(res[0], -0.625)).toBe(true);
    expect(almostEqual(res[1], 0.625)).toBe(true);
  });

  test("CCE Derivative calculates correctly", () => {
    const res = cceDerivative([1.0, 0.0], [0.8, 0.2]);
    expect(almostEqual(res[0], -0.625)).toBe(true);
    expect(almostEqual(res[1], 0.0)).toBe(true);
  });
});
