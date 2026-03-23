import { sgd } from "../src/gradient_descent";

function almostEqual(a: number, b: number): boolean {
  return Math.abs(a - b) <= 1e-6;
}

describe("Gradient Descent Optimizers", () => {
  test("SGD calculates correctly", () => {
    const weights = [1.0, -0.5, 2.0];
    const gradients = [0.1, -0.2, 0.0];
    const lr = 0.1;

    const res = sgd(weights, gradients, lr);
    
    expect(almostEqual(res[0], 0.99)).toBe(true);
    expect(almostEqual(res[1], -0.48)).toBe(true);
    expect(almostEqual(res[2], 2.0)).toBe(true);
  });

  test("Errors on mismatch lengths", () => {
    expect(() => sgd([1.0], [], 0.1)).toThrow("Arrays must have the same non-zero length");
  });

  test("Errors on empty arrays", () => {
    expect(() => sgd([], [], 0.1)).toThrow("Arrays must have the same non-zero length");
  });
});
