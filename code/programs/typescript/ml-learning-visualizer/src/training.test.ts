import { describe, expect, it } from "vitest";
import React from "react";
import { fireEvent, render, screen } from "@testing-library/react";
import { activate } from "./activation.js";
import { HiddenLayerWorkbench } from "./HiddenLayerWorkbench.js";
import { forwardLayered } from "./layered-network.js";
import {
  CELSIUS_DATASET,
  fitLinearClosedForm,
  loss,
  trainStep,
  trainSteps,
  type ModelState,
} from "./training.js";
import { LABS } from "./labs.js";
import {
  HIDDEN_LAYER_EXAMPLES,
  createInitialHiddenState,
  hiddenLoss,
  trainHiddenStep,
  trainHiddenSteps,
} from "./hidden-layer-examples.js";
import { predictLayeredWithVm, predictLinearWithVm } from "./neural-vm.js";
import { renderHiddenNetworkSvg, renderLinearNetworkSvg } from "./NetworkDiagram.js";

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

  it("runs linear predictions through the neural graph VM", () => {
    const result = predictLinearWithVm([0, 10, 100], { weight: 1.8, bias: 32 });

    expect(result.predictions).toEqual([32, 50, 212]);
    expect(result.bytecodeInstructionCount).toBeGreaterThan(0);
    expect(result.matrixInstructionCount).toBeGreaterThan(0);
  });

  it("renders a Paint VM neural graph view for the linear model", () => {
    const svg = renderLinearNetworkSvg(
      { weight: 1.8, bias: 32, epoch: 3 },
      null,
      0.0005,
      "mse",
      { x: 0, y: 32 },
      7,
    );

    expect(svg).toContain("<svg");
    expect(svg).toContain("<line");
    expect(svg).toContain("font-size");
    expect(svg).toContain("gradient descent");
  });

  it("applies activation functions used by the lab preview", () => {
    expect(activate(-2, "relu")).toBe(0);
    expect(activate(-2, "leakyRelu")).toBeCloseTo(-0.2);
    expect(activate(0, "sigmoid")).toBeCloseTo(0.5);
    expect(activate(0, "tanh")).toBeCloseTo(0);
  });

  it("registers the hidden-layer teaching examples without sine yet", () => {
    expect(HIDDEN_LAYER_EXAMPLES.map((example) => example.id)).toEqual([
      "xnor",
      "absolute-value",
      "piecewise-pricing",
      "circle-classifier",
      "two-moons",
      "interaction-features",
    ]);
    expect(HIDDEN_LAYER_EXAMPLES.every((example) => example.rows.length > 0)).toBe(true);
  });

  it("runs a hidden-layer training step for every teaching example", () => {
    for (const example of HIDDEN_LAYER_EXAMPLES) {
      const initial = createInitialHiddenState(example);
      const step = trainHiddenStep(example, initial, example.defaultLearningRate);

      expect(Number.isFinite(step.loss)).toBe(true);
      expect(step.state.epoch).toBe(1);
      expect(step.step.weightGradients[0]).toHaveLength(example.inputLabels.length);
      expect(step.step.weightGradients[step.step.weightGradients.length - 1]).toHaveLength(example.hiddenCount);
    }
  });

  it("trains a hidden-layer example with additional hidden layers", () => {
    const example = HIDDEN_LAYER_EXAMPLES[0]!;
    const initial = createInitialHiddenState(example, 3);
    const step = trainHiddenStep(example, initial, example.defaultLearningRate);

    expect(initial.hiddenLayerCount).toBe(3);
    expect(initial.parameters.layers).toHaveLength(4);
    expect(step.step.weightGradients).toHaveLength(4);
    expect(Number.isFinite(step.loss)).toBe(true);
  });

  it("matches hidden-layer visualizer predictions with the shared graph VM", () => {
    const example = HIDDEN_LAYER_EXAMPLES[0]!;
    const initial = createInitialHiddenState(example);
    const inputs = example.rows.map((row) => row.input);
    const direct = forwardLayered(inputs, initial.parameters).predictions;
    const vm = predictLayeredWithVm(inputs, initial.parameters, {
      inputNames: example.inputLabels,
      outputNames: [example.outputLabel],
    });

    expect(vm.predictions).toHaveLength(direct.length);
    for (const [index, row] of direct.entries()) {
      expect(vm.predictions[index]![0]).toBeCloseTo(row[0]!);
    }
  });

  it("renders a Paint VM neural graph view for hidden-layer examples", () => {
    const example = HIDDEN_LAYER_EXAMPLES[0]!;
    const initial = createInitialHiddenState(example);
    const svg = renderHiddenNetworkSvg(
      example,
      initial,
      example.rows[0]!,
      0,
      0.42,
      null,
      example.defaultLearningRate,
    );

    expect(svg).toContain("<svg");
    expect(svg).toContain("<ellipse");
    expect(svg).toContain("<line");
    expect(svg).toContain("parameter update");
  });

  it("renders every hidden-layer example in the workbench", () => {
    render(React.createElement(HiddenLayerWorkbench));

    for (const example of HIDDEN_LAYER_EXAMPLES) {
      fireEvent.click(screen.getByRole("button", { name: `${example.title} ${example.category}` }));
      expect(screen.getByRole("heading", { name: example.title })).toBeTruthy();
      expect(screen.getByLabelText("Neuron trace")).toBeTruthy();
    }
  });

  it("lets the hidden-layer workbench increase network depth", () => {
    render(React.createElement(HiddenLayerWorkbench));
    const depthControls = screen.getAllByLabelText("Hidden layers");

    fireEvent.change(depthControls[0]!, { target: { value: "3" } });
    fireEvent.click(screen.getByRole("button", { name: "Step" }));

    expect(screen.getAllByText("3 hidden layers").length).toBeGreaterThan(0);
    expect(screen.getAllByText("3 x hidden[3]").length).toBeGreaterThan(0);
  });

  it("moves downhill on XNOR and absolute value with batch updates", () => {
    for (const example of HIDDEN_LAYER_EXAMPLES.slice(0, 2)) {
      const initial = createInitialHiddenState(example);
      const before = hiddenLoss(example, initial);
      const steps = trainHiddenSteps(example, initial, example.defaultLearningRate, 40);
      const after = hiddenLoss(example, steps[steps.length - 1]!.state);

      expect(after).toBeLessThan(before);
    }
  });
});
