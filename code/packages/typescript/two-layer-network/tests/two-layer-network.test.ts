import { describe, expect, it } from "vitest";
import {
  TwoLayerNetwork,
  createSeededParameters,
  createXorWarmStartParameters,
  forwardTwoLayer,
  trainOneEpochTwoLayer,
  traceExampleTwoLayer,
  type MatrixData,
} from "../src/index.js";

const XOR_INPUTS = [
  [0, 0],
  [0, 1],
  [1, 0],
  [1, 1],
];

const XOR_TARGETS = [
  [0],
  [1],
  [1],
  [0],
];

describe("two-layer network", () => {
  it("runs a forward pass through hidden activations", () => {
    const pass = forwardTwoLayer(XOR_INPUTS, createXorWarmStartParameters());

    expect(pass.hiddenActivations).toHaveLength(4);
    expect(pass.hiddenActivations[0]).toHaveLength(2);
    expect(pass.predictions[1]![0]).toBeGreaterThan(0.7);
    expect(pass.predictions[0]![0]).toBeLessThan(0.3);
  });

  it("exposes gradients for both layers", () => {
    const step = trainOneEpochTwoLayer(
      XOR_INPUTS,
      XOR_TARGETS,
      createXorWarmStartParameters(),
      0.5,
    );

    expect(step.inputToHiddenWeightGradients).toHaveLength(2);
    expect(step.inputToHiddenWeightGradients[0]).toHaveLength(2);
    expect(step.hiddenToOutputWeightGradients).toHaveLength(2);
    expect(step.hiddenToOutputWeightGradients[0]).toHaveLength(1);
    expect(step.hiddenBiasGradients).toHaveLength(2);
    expect(step.outputBiasGradients).toHaveLength(1);
  });

  it("traces one example through every neuron", () => {
    const trace = traceExampleTwoLayer(
      XOR_INPUTS,
      createXorWarmStartParameters(),
      1,
      XOR_TARGETS[1],
    );

    expect(trace.inputs).toEqual([0, 1]);
    expect(trace.target).toEqual([1]);
    expect(trace.prediction).toHaveLength(1);
    expect(trace.layers).toHaveLength(2);
    expect(trace.layers[0]!.neurons[0]!.incoming).toEqual([
      { source: "input[0]", value: 0, weight: 4, contribution: 0 },
      { source: "input[1]", value: 1, weight: 4, contribution: 4 },
    ]);
    expect(trace.layers[1]!.neurons[0]!.incoming[0]!.source).toBe("hidden[0]");
    expect(trace.layers[1]!.neurons[0]!.delta).toBeTypeOf("number");
  });

  it("traces from a fitted network instance", () => {
    const network = new TwoLayerNetwork({
      inputCount: 2,
      hiddenCount: 2,
      outputCount: 1,
      initialParameters: createXorWarmStartParameters(),
    });

    const trace = network.trace(XOR_INPUTS, 2, XOR_TARGETS[2]);

    expect(trace.layers[0]!.neurons[0]!.neuron).toBe("hidden[0]");
    expect(trace.layers[1]!.neurons[0]!.neuron).toBe("output[0]");
  });

  it("learns XOR with a hidden layer", () => {
    const network = new TwoLayerNetwork({
      inputCount: 2,
      hiddenCount: 2,
      outputCount: 1,
      learningRate: 1.8,
      seed: 11,
      initialScale: 2,
    });

    const history = network.fit(XOR_INPUTS, XOR_TARGETS, {
      epochs: 12000,
      logEvery: 3000,
    });
    const predictions = network.predict(XOR_INPUTS).map(row => row[0]!);

    expect(history[history.length - 1]!.loss).toBeLessThan(history[0]!.loss);
    expect(predictions[0]).toBeLessThan(0.15);
    expect(predictions[1]).toBeGreaterThan(0.85);
    expect(predictions[2]).toBeGreaterThan(0.85);
    expect(predictions[3]).toBeLessThan(0.15);
  });

  it("returns useful history with default fit options", () => {
    const network = new TwoLayerNetwork({
      inputCount: 2,
      hiddenCount: 2,
      outputCount: 1,
      learningRate: 1.8,
      seed: 11,
      initialScale: 2,
    });

    const history = network.fit(XOR_INPUTS, XOR_TARGETS, { epochs: 10 });

    expect(history.length).toBeGreaterThan(0);
    expect(history[history.length - 1]!.epoch).toBe(10);
  });

  it("runs the hidden-layer teaching examples through one training step", () => {
    const cases: Array<{ name: string; inputs: MatrixData; targets: MatrixData; hiddenCount: number }> = [
      { name: "XNOR", inputs: XOR_INPUTS, targets: [[1], [0], [0], [1]], hiddenCount: 3 },
      { name: "absolute value", inputs: [[-1], [-0.5], [0], [0.5], [1]], targets: [[1], [0.5], [0], [0.5], [1]], hiddenCount: 4 },
      { name: "piecewise pricing", inputs: [[0.1], [0.3], [0.5], [0.7], [0.9]], targets: [[0.12], [0.25], [0.55], [0.88], [0.88]], hiddenCount: 4 },
      { name: "circle classifier", inputs: [[0, 0], [0.5, 0], [1, 1], [-0.5, 0.5], [-1, 0]], targets: [[1], [1], [0], [1], [0]], hiddenCount: 5 },
      { name: "two moons", inputs: [[1, 0], [0, 0.5], [0.5, 0.85], [0.5, -0.35], [-1, 0], [2, 0.5]], targets: [[0], [1], [0], [1], [0], [1]], hiddenCount: 5 },
      { name: "interaction features", inputs: [[0.2, 0.25, 0], [0.6, 0.5, 1], [1, 0.75, 1], [1, 1, 0]], targets: [[0.08], [0.72], [0.96], [0.76]], hiddenCount: 5 },
    ];

    for (const item of cases) {
      const step = trainOneEpochTwoLayer(
        item.inputs,
        item.targets,
        createSeededParameters(item.inputs[0]!.length, item.hiddenCount, 1, item.hiddenCount, 0.8),
        0.4,
      );

      expect(step.loss, item.name).toBeGreaterThanOrEqual(0);
      expect(step.inputToHiddenWeightGradients, item.name).toHaveLength(item.inputs[0]!.length);
      expect(step.hiddenToOutputWeightGradients, item.name).toHaveLength(item.hiddenCount);
    }
  });
});
