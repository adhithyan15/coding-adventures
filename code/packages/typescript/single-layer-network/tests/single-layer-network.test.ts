import { describe, expect, it } from "vitest";
import {
  SingleLayerNetwork,
  VERSION,
  fitSingleLayerNetwork,
  trainOneEpochWithMatrices,
} from "../src/index";

describe("single-layer-network", () => {
  it("has a version", () => {
    expect(VERSION).toBe("0.1.0");
  });

  it("trains an m input to n output linear model", () => {
    const inputs = [
      [0, 0, 1],
      [1, 0, 1],
      [0, 1, 1],
      [1, 1, 1],
    ];
    const targets = inputs.map(([x1, x2, x3]) => [
      x1 + x2,
      (2 * x2) - x3,
    ]);

    const model = new SingleLayerNetwork({ activation: "linear", learningRate: 0.1 });
    const history = model.fit(inputs, targets, { epochs: 600, logEvery: 600 });
    const predictions = model.predict([[1, 1, 1], [1, 0, 1]]);

    expect(history.at(-1)?.loss).toBeLessThan(0.001);
    expect(predictions[0][0]).toBeCloseTo(2, 2);
    expect(predictions[0][1]).toBeCloseTo(1, 2);
    expect(predictions[1][0]).toBeCloseTo(1, 2);
    expect(predictions[1][1]).toBeCloseTo(-1, 2);
  });

  it("exposes the explicit matrix math for a single epoch", () => {
    const step = trainOneEpochWithMatrices(
      [[1, 2]],
      [[3, 5]],
      [[0, 0], [0, 0]],
      [0, 0],
      0.1,
    );

    expect(step.predictions).toEqual([[0, 0]]);
    expect(step.errors).toEqual([[-3, -5]]);
    expect(step.weightGradients).toEqual([[-3, -5], [-6, -10]]);
    expect(step.biasGradients).toEqual([-3, -5]);
    expect(step.nextWeights).toEqual([[0.30000000000000004, 0.5], [0.6000000000000001, 1]]);
    expect(step.nextBiases).toEqual([0.30000000000000004, 0.5]);
  });

  it("can infer shape from fitSingleLayerNetwork", () => {
    const model = fitSingleLayerNetwork(
      [[0], [1], [2]],
      [[0, 0], [1, 2], [2, 4]],
      { learningRate: 0.05, epochs: 800 },
    );

    expect(model.inputCount).toBe(1);
    expect(model.outputCount).toBe(2);
    expect(model.predict([[3]])[0][0]).toBeCloseTo(3, 1);
    expect(model.predict([[3]])[0][1]).toBeCloseTo(6, 1);
  });
});
