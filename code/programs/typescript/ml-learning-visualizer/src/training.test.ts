import { describe, expect, it } from "vitest";
import { activate } from "./activation.js";
import {
  CELSIUS_DATASET,
  fitLinearClosedForm,
  loss,
  trainStep,
  trainSteps,
  type ModelState,
} from "./training.js";
import { LABS } from "./labs.js";

describe("training helpers", () => {
  it("reduces MSE loss for a small learning rate", () => {
    const initial: ModelState = { weight: 0.5, bias: 0.5, epoch: 0 };
    const before = loss(CELSIUS_DATASET, initial, "mse");
    const after = trainStep(CELSIUS_DATASET, initial, 0.0005, "mse");

    expect(after.loss).toBeLessThan(before);
    expect(after.state.epoch).toBe(1);
  });

  it("converges near the Celsius to Fahrenheit slope with MSE", () => {
    const initial: ModelState = { weight: 0.5, bias: 0.5, epoch: 0 };
    const steps = trainSteps(CELSIUS_DATASET, initial, 0.0005, "mse", 4500);
    const final = steps[steps.length - 1]!;

    expect(final.state.weight).toBeGreaterThan(1.78);
    expect(final.state.weight).toBeLessThan(1.83);
    expect(final.state.bias).toBeGreaterThan(31);
    expect(final.state.bias).toBeLessThan(32.5);
    expect(final.mae).toBeLessThan(0.6);
  });

  it("registers one hundred teaching labs", () => {
    expect(LABS).toHaveLength(100);
    expect(new Set(LABS.map((lab) => lab.id)).size).toBe(100);
    expect(LABS.some((lab) => lab.source.kind === "local-csv")).toBe(true);
  });

  it("fits the least-squares line for simple points", () => {
    const fit = fitLinearClosedForm([
      { x: 0, y: 1 },
      { x: 1, y: 3 },
      { x: 2, y: 5 },
    ]);

    expect(fit.weight).toBeCloseTo(2);
    expect(fit.bias).toBeCloseTo(1);
  });

  it("applies activation functions used by the lab preview", () => {
    expect(activate(-2, "relu")).toBe(0);
    expect(activate(-2, "leakyRelu")).toBeCloseTo(-0.2);
    expect(activate(0, "sigmoid")).toBeCloseTo(0.5);
    expect(activate(0, "tanh")).toBeCloseTo(0);
  });
});
