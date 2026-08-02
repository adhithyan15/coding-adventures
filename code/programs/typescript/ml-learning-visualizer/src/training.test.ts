import { describe, expect, it } from "vitest";
import React from "react";
import { fireEvent, render, screen } from "@testing-library/react";
import { activate } from "./activation.js";
import { AttentionWorkbench } from "./AttentionWorkbench.js";
import { ConvolutionWorkbench } from "./ConvolutionWorkbench.js";
import { HiddenLayerWorkbench } from "./HiddenLayerWorkbench.js";
import { ImageCnnWorkbench } from "./ImageCnnWorkbench.js";
import { OptimizationWorkbench } from "./OptimizationWorkbench.js";
import { RecurrentWorkbench } from "./RecurrentWorkbench.js";
import { ResidualWorkbench } from "./ResidualWorkbench.js";
import { TrainingStepMicroscope } from "./TrainingStepMicroscope.js";
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
import {
  DEFAULT_OPTIMIZATION_STATE,
  OPTIMIZATION_DATASET,
  analyticalGradient,
  batchIndices,
  checkGradient,
  meanSquaredError,
  runOptimization,
} from "./optimization-lab.js";
import {
  DEFAULT_MICROSCOPE_STATE,
  traceTrainingStep,
} from "./training-microscope.js";
import {
  DEFAULT_CONVOLUTION_KERNEL,
  DEFAULT_CONVOLUTION_LEARNING_RATE,
  DEFAULT_CONVOLUTION_SIGNAL,
  DEFAULT_CONVOLUTION_TARGETS,
  numericalKernelGradient,
  parseNumberList,
  proposeConvolutionStep,
  traceConvolutionTraining,
  traceValidCorrelation,
} from "./convolution-lab.js";
import {
  DEFAULT_IMAGE_CHANNELS,
  DEFAULT_IMAGE_FILTERS,
  traceTinyImageCnn,
} from "./image-cnn-lab.js";
import {
  DEFAULT_RESIDUAL_INPUT,
  sameCorrelation,
  traceResidualBlock,
} from "./residual-field-lab.js";
import {
  DEFAULT_RECURRENT_INITIAL_STATE,
  DEFAULT_RECURRENT_INPUTS,
  DEFAULT_RECURRENT_PARAMETERS,
  traceRecurrentBptt,
  traceRecurrentUnroll,
} from "./recurrent-unroll-lab.js";
import {
  traceGateCounterfactual,
  traceGatedRecurrent,
} from "./gated-recurrent-lab.js";
import {
  attentionCell,
  traceAttentionQkv,
} from "./attention-qkv-lab.js";
import {
  attentionSoftmaxRow,
  traceAttentionSoftmax,
} from "./attention-softmax-lab.js";

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

  it("traces the hand-calculated one-neuron training update", () => {
    const trace = traceTrainingStep(DEFAULT_MICROSCOPE_STATE);

    expect(trace.weightedInput).toBeCloseTo(1);
    expect(trace.preActivation).toBeCloseTo(1.1);
    expect(trace.prediction).toBeCloseTo(1.1);
    expect(trace.loss).toBeCloseTo(0.01);
    expect(trace.gradientWeight).toBeCloseTo(0.4);
    expect(trace.gradientBias).toBeCloseTo(0.2);
    expect(trace.nextWeight).toBeCloseTo(0.46);
    expect(trace.nextBias).toBeCloseTo(0.08);
    expect(trace.nextPrediction).toBeCloseTo(1);
    expect(trace.nextLoss).toBeCloseTo(0);
  });

  it("reveals and applies one update in the training microscope", () => {
    render(React.createElement(TrainingStepMicroscope));

    expect(screen.getByRole("heading", { name: "Choose one training example" })).toBeTruthy();
    for (let phase = 1; phase < 7; phase += 1) {
      fireEvent.click(screen.getByRole("button", { name: "Next phase" }));
    }
    expect(screen.getByRole("heading", { name: "Move the parameters against the gradient" })).toBeTruthy();
    expect(screen.getByText("After proposed update")).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "Apply this update" }));
    expect(Number((screen.getByLabelText("Weight w") as HTMLInputElement).value)).toBeCloseTo(0.46);
    expect(Number((screen.getByLabelText("Bias b") as HTMLInputElement).value)).toBeCloseTo(0.08);
    expect(screen.getByText("update 1")).toBeTruthy();
  });

  it("matches backpropagation with a central finite-difference gradient", () => {
    const check = checkGradient(OPTIMIZATION_DATASET, DEFAULT_OPTIMIZATION_STATE, 1e-5);

    expect(check.analytical.weight).toBeCloseTo(-8.5);
    expect(check.analytical.bias).toBeCloseTo(-4.5);
    expect(check.numerical.weight).toBeCloseTo(check.analytical.weight, 8);
    expect(check.numerical.bias).toBeCloseTo(check.analytical.bias, 8);
    expect(check.passes).toBe(true);
  });

  it("selects deterministic rows for stochastic, mini-batch, and full-batch training", () => {
    expect(batchIndices("stochastic", 5, 4)).toEqual([1]);
    expect(batchIndices("mini-batch", 0, 4)).toEqual([0, 1]);
    expect(batchIndices("mini-batch", 1, 4)).toEqual([2, 3]);
    expect(batchIndices("full-batch", 99, 4)).toEqual([0, 1, 2, 3]);
  });

  it("reduces full-dataset loss with every deterministic batch strategy", () => {
    const before = meanSquaredError(OPTIMIZATION_DATASET, DEFAULT_OPTIMIZATION_STATE);

    for (const strategy of ["stochastic", "mini-batch", "full-batch"] as const) {
      const trace = runOptimization(strategy, 20, 0.05);
      expect(trace[trace.length - 1]!.loss).toBeLessThan(before);
    }
  });

  it("renders the optimization microscope and its independent gradient audit", () => {
    render(React.createElement(OptimizationWorkbench));

    expect(screen.getByRole("heading", { name: "Optimization microscope" })).toBeTruthy();
    expect(screen.getByRole("heading", { name: "Loss landscape" })).toBeTruthy();
    expect(screen.getByRole("heading", { name: "Finite-difference gradient check" })).toBeTruthy();
    expect(screen.getByText("PASS")).toBeTruthy();

    fireEvent.change(screen.getByLabelText("Optimization weight"), { target: { value: "1" } });
    const gradient = analyticalGradient(OPTIMIZATION_DATASET, { weight: 1, bias: 0, step: 0 });
    expect(screen.getAllByText(formatTestNumber(gradient.weight)).length).toBeGreaterThan(0);
  });

  it("traces every multiply-accumulate for an asymmetric sliding kernel", () => {
    const traces = traceValidCorrelation(
      DEFAULT_CONVOLUTION_SIGNAL,
      DEFAULT_CONVOLUTION_KERNEL,
    );

    expect(traces.map((trace) => trace.output)).toEqual([7, -2, 11, 0]);
    expect(traces[2]!.window).toEqual([3, 0, 4]);
    expect(traces[2]!.products).toEqual([3, 0, 8]);
    expect(traces[2]!.accumulator).toEqual([0, 3, 3, 11]);
  });

  it("does not reverse the neural convolution kernel", () => {
    const reversed = traceValidCorrelation(
      DEFAULT_CONVOLUTION_SIGNAL,
      [...DEFAULT_CONVOLUTION_KERNEL].reverse(),
    );

    expect(reversed.map((trace) => trace.output)).toEqual([6, -1, 10, -2]);
    expect(parseNumberList("2, -1, 1")).toEqual([2, -1, 1]);
    expect(parseNumberList("2, nope, 1")).toBeNull();
  });

  it("lets the sliding-kernel microscope inspect and edit outputs", () => {
    render(React.createElement(ConvolutionWorkbench));

    expect(screen.getByRole("heading", { name: "Sliding-kernel microscope" })).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Next" }));
    expect(screen.getByText("Output y[1]")).toBeTruthy();

    fireEvent.change(screen.getByLabelText("Kernel weights"), {
      target: { value: "2, -1, 1" },
    });
    expect(screen.getByRole("button", { name: "Select output 1" }).textContent).toContain("-1");
  });

  it("accumulates a shared kernel gradient from every output window", () => {
    const trace = traceConvolutionTraining(
      DEFAULT_CONVOLUTION_SIGNAL,
      DEFAULT_CONVOLUTION_KERNEL,
      DEFAULT_CONVOLUTION_TARGETS,
    );

    expect(trace.loss).toBeCloseTo(0.5);
    expect(trace.errors).toEqual([1, 0, 1, 0]);
    expect(trace.outputGradients).toEqual([0.5, 0, 0.5, 0]);
    expect(trace.contributions[0]!.kernelGradient).toEqual([1, 0.5, 1.5]);
    expect(trace.contributions[2]!.kernelGradient).toEqual([1.5, 0, 2]);
    expect(trace.kernelGradient).toEqual([2.5, 0.5, 3.5]);
  });

  it("matches the convolution backward pass with finite differences", () => {
    const trace = traceConvolutionTraining(
      DEFAULT_CONVOLUTION_SIGNAL,
      DEFAULT_CONVOLUTION_KERNEL,
      DEFAULT_CONVOLUTION_TARGETS,
    );
    const numerical = numericalKernelGradient(
      DEFAULT_CONVOLUTION_SIGNAL,
      DEFAULT_CONVOLUTION_KERNEL,
      DEFAULT_CONVOLUTION_TARGETS,
    );

    for (const [index, gradient] of trace.kernelGradient.entries()) {
      expect(numerical[index]).toBeCloseTo(gradient, 8);
    }
  });

  it("proposes a kernel update that lowers convolution loss", () => {
    const proposal = proposeConvolutionStep(
      DEFAULT_CONVOLUTION_SIGNAL,
      DEFAULT_CONVOLUTION_KERNEL,
      DEFAULT_CONVOLUTION_TARGETS,
      DEFAULT_CONVOLUTION_LEARNING_RATE,
    );

    expect(proposal.nextKernel).toEqual([0.95, -1.01, 1.93]);
    expect(proposal.nextOutputs).toEqual([6.68, -2.08, 10.57, -0.18000000000000016]);
    expect(proposal.nextLoss).toBeCloseTo(0.206525);
  });

  it("applies one shared-kernel gradient step in the workbench", () => {
    render(React.createElement(ConvolutionWorkbench));

    expect(screen.getByRole("heading", { name: "Shared weights collect gradients" })).toBeTruthy();
    expect(screen.getByText("PASS")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Apply gradient step" }));

    expect((screen.getByLabelText("Kernel weights") as HTMLInputElement).value)
      .toBe("0.95, -1.01, 1.93");
    expect(screen.getByText("step 1")).toBeTruthy();
  });

  it("reduces two image channels into two convolution maps", () => {
    const trace = traceTinyImageCnn(DEFAULT_IMAGE_CHANNELS, DEFAULT_IMAGE_FILTERS);

    expect(trace.channelContributions[0]).toEqual([
      [[0, 0], [4, 4]],
      [[0, 2], [0, 2]],
    ]);
    expect(trace.convolution).toEqual([
      [[0, 2], [4, 6]],
      [[6, 4], [2, 0]],
    ]);
    expect(trace.positions[0]![1]![1]!.channelSums).toEqual([4, 2]);
  });

  it("normalizes each CNN output channel over its spatial map", () => {
    const trace = traceTinyImageCnn();

    expect(trace.normalization.means).toEqual([3, 3]);
    expect(trace.normalization.variances).toEqual([5, 5]);
    expect(trace.normalization.denominators).toEqual([3, 3]);
    expect(trace.normalization.maps[0]![0]![1]).toBeCloseTo(-1 / 3);
    expect(trace.normalization.maps[1]![1]![0]).toBeCloseTo(-1 / 3);
  });

  it("tracks ReLU and max-pool winners for each feature channel", () => {
    const trace = traceTinyImageCnn();

    expect(trace.activation).toEqual([
      [[0, 0], [1 / 3, 1]],
      [[1, 1 / 3], [0, 0]],
    ]);
    expect(trace.pooling).toEqual({ values: [1, 1], argmax: [[1, 1], [0, 0]] });
  });

  it("steps through an inspectable tiny image CNN pipeline", () => {
    render(React.createElement(ImageCnnWorkbench));

    expect(screen.getByRole("heading", { name: "Open the image pipeline" })).toBeTruthy();
    expect(screen.getByRole("heading", { name: "One image can have several number grids" })).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "Show Convolve stage" }));
    expect(screen.getByRole("heading", { name: "Correlate each channel, then add" })).toBeTruthy();
    expect(screen.getByLabelText("Channel reduction equation").textContent)
      .toMatch(/channel 0.*4.*channel 1.*2.*bias.*0.*output.*6/s);

    fireEvent.click(screen.getByRole("button", { name: "Show Normalize stage" }));
    expect(screen.getByText("mean μ").parentElement?.textContent).toContain("3");
    expect(screen.getByText("denominator").parentElement?.textContent).toContain("3");

    fireEvent.click(screen.getByRole("button", { name: "Show Pool stage" }));
    expect(screen.getAllByText(/from \[/).map((item) => item.textContent)).toEqual([
      "from [1,1]",
      "from [0,0]",
    ]);
  });

  it("runs the two local layers and identity path by hand", () => {
    const block = traceResidualBlock();

    expect(sameCorrelation(DEFAULT_RESIDUAL_INPUT, [1, 1, 1])).toEqual([1, 3, 2, 3, 1]);
    expect(block.main).toEqual([4, 6, 8, 6, 4]);
    expect(block.skip).toEqual([1, 0, 2, 0, 1]);
    expect(block.output).toEqual([5, 6, 10, 6, 5]);
  });

  it("expands center and boundary receptive fields into exact input paths", () => {
    const block = traceResidualBlock();
    const center = block.traces[2]!;

    expect(center.hiddenIndices).toEqual([1, 2, 3]);
    expect(center.inputPathCounts).toEqual([1, 2, 3, 2, 1]);
    expect(center.inputContributions).toEqual([1, 0, 6, 0, 1]);
    expect(center.receptiveFieldIndices).toEqual([0, 1, 2, 3, 4]);
    expect(block.traces[0]!.receptiveFieldIndices).toEqual([0, 1, 2]);
    expect(block.traces[4]!.receptiveFieldIndices).toEqual([2, 3, 4]);
  });

  it("opens residual and receptive-field paths interactively", () => {
    render(React.createElement(ResidualWorkbench));

    expect(screen.getByRole("heading", { name: "Residual-path microscope" })).toBeTruthy();
    expect(screen.getByLabelText("Selected residual addition").textContent)
      .toMatch(/8.*\+.*2.*=.*10.*10/s);
    expect(screen.getByLabelText("Receptive field explorer").textContent)
      .toContain("receptive input indices = [0, 1, 2, 3, 4]");

    fireEvent.click(screen.getByRole("checkbox", { name: /Include identity skip/ }));
    expect(screen.getByLabelText("Selected residual addition").textContent)
      .toMatch(/8.*\+.*0.*=.*8.*8/s);

    fireEvent.click(screen.getByRole("button", { name: "Select residual output 0" }));
    expect(screen.getByLabelText("Receptive field explorer").textContent)
      .toContain("receptive input indices = [0, 1, 2]");
  });

  it("unrolls one shared recurrent state through three exact steps", () => {
    const trace = traceRecurrentUnroll();

    expect(trace.states).toEqual([1, 3.5, 0.75]);
    expect(trace.steps[1]).toMatchObject({
      inputProduct: 4,
      recurrentProduct: 0.5,
      preactivation: 3.5,
      state: 3.5,
    });
    expect(trace.steps[2]).toMatchObject({
      input: 0,
      previousState: 3.5,
      inputProduct: 0,
      recurrentProduct: 1.75,
      state: 0.75,
    });
  });

  it("isolates memory by cutting the recurrent contribution", () => {
    const ablated = traceRecurrentUnroll(
      DEFAULT_RECURRENT_INPUTS,
      DEFAULT_RECURRENT_INITIAL_STATE,
      DEFAULT_RECURRENT_PARAMETERS,
      false,
    );

    expect(ablated.states).toEqual([1, 3, 0]);
    expect(ablated.steps.every((step) => step.recurrentProduct === 0)).toBe(true);
  });

  it("selects an unrolled step and toggles recurrent memory interactively", () => {
    render(React.createElement(RecurrentWorkbench));

    expect(screen.getByRole("heading", { name: "Recurrent-state unroller" })).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Select recurrent step 2" }));
    expect(screen.getByLabelText("Selected recurrent arithmetic").textContent)
      .toMatch(/2.*0.*0.*0\.5.*3\.5.*1\.75.*0\.75.*0\.75/s);

    fireEvent.click(screen.getByRole("checkbox", { name: /Carry the previous state/ }));
    expect(screen.getByText("final state").parentElement?.textContent).toContain("0");
    expect(screen.getByLabelText("Selected recurrent arithmetic").textContent)
      .toMatch(/carried state.*0\.5.*3.*0.*preactivation.*-1.*ReLU state.*0/s);
  });

  it("reverses the recurrent chain and accumulates shared gradients", () => {
    const trace = traceRecurrentBptt();

    expect(trace.loss).toBeCloseTo(0.28125);
    expect(trace.backwardSteps.map((step) => step.time)).toEqual([2, 1, 0]);
    expect(trace.backwardSteps.map((step) => step.stateGradient)).toEqual([
      0.75,
      0.375,
      0.1875,
    ]);
    expect(trace.gradientTotals).toEqual({
      inputWeight: 0.9375,
      recurrentWeight: 3,
      bias: 1.3125,
      initialState: 0.09375,
    });
  });

  it("checks BPTT independently and takes one loss-reducing step", () => {
    const trace = traceRecurrentBptt();

    expect(trace.maxGradientError).toBeLessThan(1e-9);
    expect(trace.update.parameters.inputWeight).toBeCloseTo(1.90625);
    expect(trace.update.parameters.recurrentWeight).toBeCloseTo(0.2);
    expect(trace.update.parameters.bias).toBeCloseTo(-1.13125);
    for (const [index, value] of [0.775, 2.83625, -0.564].entries()) {
      expect(trace.update.preactivations[index]).toBeCloseTo(value);
    }
    for (const [index, value] of [0.775, 2.83625, 0].entries()) {
      expect(trace.update.states[index]).toBeCloseTo(value);
    }
    expect(trace.update.loss).toBe(0);
  });

  it("switches the recurrent workbench into the backward microscope", () => {
    render(React.createElement(RecurrentWorkbench));

    fireEvent.click(screen.getByRole("button", { name: "Trace backward gradients" }));
    expect(screen.getByRole("heading", { name: "Backpropagation-through-time microscope" })).toBeTruthy();
    expect(screen.getByLabelText("Reverse-time gradient steps").textContent)
      .toMatch(/reverse t = 2.*reverse t = 1.*reverse t = 0/s);
    expect(screen.getByLabelText("Shared gradient reduction").textContent)
      .toMatch(/dL\/dW_x.*0\.9375.*dL\/dW_h.*3.*dL\/db.*1\.3125/s);

    fireEvent.click(screen.getByRole("button", { name: "Select backward step 1" }));
    expect(screen.getByLabelText("Selected backward arithmetic").textContent)
      .toMatch(/direct loss.*0.*from future.*0\.375.*dL\/dh\[1\].*0\.375/s);

    fireEvent.click(screen.getByRole("button", { name: "Show forward unroll" }));
    expect(screen.getByRole("heading", { name: "Recurrent-state unroller" })).toBeTruthy();
  });

  it("routes the same memory through scalar GRU and LSTM gates", () => {
    const trace = traceGatedRecurrent();

    expect(trace.gru.resetGate.value).toBeCloseTo(0.5);
    expect(trace.gru.updateGate.value).toBeCloseTo(0.25);
    expect(trace.gru.candidate.value).toBeCloseTo(0.6);
    expect(trace.gru.hiddenState).toBeCloseTo(0.75);
    expect(trace.lstm.cellState).toBeCloseTo(0.55);
    expect(trace.lstm.exposedCell).toBeCloseTo(0.5005202111902353);
    expect(trace.lstm.hiddenState).toBeCloseTo(0.3753901583926764);
  });

  it("changes exactly one recurrent gate in a counterfactual", () => {
    const trace = traceGatedRecurrent();
    const resetOff = traceGateCounterfactual("gru", "reset", 0, trace);
    const outputOff = traceGateCounterfactual("lstm", "output", 0, trace);
    const outputOn = traceGateCounterfactual("lstm", "output", 1, trace);

    expect(resetOff.candidate).toBeCloseTo(0.2850288981936261);
    expect(resetOff.hiddenState).toBeCloseTo(0.6712572245484066);
    expect(outputOff.cellState).toBeCloseTo(0.55);
    expect(outputOff.hiddenState).toBe(0);
    expect(outputOn.cellState).toBeCloseTo(outputOff.cellState!);
    expect(outputOn.hiddenState).toBeCloseTo(0.5005202111902353);
  });

  it("compares GRU and LSTM gates interactively", () => {
    render(React.createElement(RecurrentWorkbench));

    fireEvent.click(screen.getByRole("button", { name: "Trace backward gradients" }));
    fireEvent.click(screen.getByRole("button", { name: "Compare GRU and LSTM gates" }));
    expect(screen.getByRole("heading", { name: "GRU and LSTM gate comparator" })).toBeTruthy();
    expect(screen.getByLabelText("GRU memory lane").textContent)
      .toMatch(/reset r.*0\.5.*candidate n.*0\.6.*update z.*0\.25.*next hidden.*0\.75/s);
    expect(screen.getByLabelText("LSTM memory lane").textContent)
      .toMatch(/forget f.*0\.5.*input i.*0\.25.*private cell.*0\.55.*output o.*0\.75.*next hidden.*0\.37539/s);

    fireEvent.click(screen.getByRole("button", { name: "Select LSTM output gate" }));
    fireEvent.click(screen.getByRole("button", { name: "Force 0" }));
    expect(screen.getByLabelText("Selected gate effect").textContent)
      .toMatch(/selected value.*0.*next c 0\.55.*visible h 0/s);

    fireEvent.click(screen.getByRole("button", { name: "Return to BPTT gradients" }));
    expect(screen.getByRole("heading", { name: "Backpropagation-through-time microscope" })).toBeTruthy();
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

function formatTestNumber(value: number): string {
  return Number(value.toFixed(5)).toString();
}

describe("attention query-key-value lab", () => {
  it("projects three tokens and reproduces the canonical score matrices", () => {
    const trace = traceAttentionQkv();

    expect(trace.projections.map(({ query, key, value }) => ({ query, key, value }))).toEqual([
      { query: [1, 0], key: [1, 1], value: [2, 0] },
      { query: [0, 1], key: [-1, 1], value: [0, 1] },
      { query: [1, 1], key: [0, 2], value: [2, 1] },
    ]);
    expect(trace.rawScoreMatrix).toEqual([
      [1, -1, 0],
      [1, 1, 2],
      [2, 0, 2],
    ]);
    expect(trace.scaledScoreMatrix[1]![2]).toBeCloseTo(Math.SQRT2);
  });

  it("keeps the coordinate products behind every dot product", () => {
    const bluePurple = attentionCell(traceAttentionQkv(), "blue", "purple");
    const purpleBlue = attentionCell(traceAttentionQkv(), "purple", "blue");

    expect(bluePurple.products).toEqual([0, 2]);
    expect(bluePurple.rawScore).toBe(2);
    expect(purpleBlue.products).toEqual([-1, 1]);
    expect(purpleBlue.rawScore).toBe(0);
  });

  it("rejects inputs that leave the fixed V1 shapes", () => {
    expect(() => traceAttentionQkv(undefined, [[1]])).toThrow(/three 2 x 2 matrices/);
  });

  it("opens raw, cancelling, and scaled score arithmetic interactively", () => {
    render(React.createElement(AttentionWorkbench));

    expect(screen.getByRole("heading", { name: "Query-key score microscope" })).toBeTruthy();
    expect(screen.getByLabelText("Selected dot-product arithmetic").textContent)
      .toMatch(/blue asks.*purple matches.*0 × 0 \+ 1 × 2.*= 2.*coordinate products \[0, 2\]/s);
    expect(screen.getByText("v_purple = [2, 1]")).toBeTruthy();

    fireEvent.click(screen.getByRole("gridcell", { name: "Select purple query and blue key" }));
    expect(screen.getByLabelText("Selected dot-product arithmetic").textContent)
      .toMatch(/purple asks.*blue matches.*1 × -1 \+ 1 × 1.*= 0.*coordinate products \[-1, 1\]/s);

    fireEvent.click(screen.getByRole("gridcell", { name: "Select blue query and purple key" }));
    fireEvent.click(screen.getByRole("checkbox", { name: /Scale by sqrt/ }));
    expect(screen.getByLabelText("Selected dot-product arithmetic").textContent)
      .toMatch(/2 \/ sqrt\(2\) = 1\.414214/s);
    expect(screen.getByLabelText("Scaled attention scores").textContent).toContain("1.414214");
  });
});

describe("attention softmax and causal mask lab", () => {
  it("normalizes every causal row while zeroing future keys", () => {
    const trace = traceAttentionSoftmax(true);

    expect(trace.weightMatrix[0]).toEqual([1, 0, 0]);
    expect(trace.weightMatrix[1]).toEqual([0.5, 0.5, 0]);
    expect(trace.rows[0]!.maskedScores).toEqual([
      1 / Math.sqrt(2),
      null,
      null,
    ]);
    for (const row of trace.rows) {
      expect(row.weights.reduce((sum, weight) => sum + weight, 0)).toBeCloseTo(1);
    }
  });

  it("mixes values into the expected causal contexts", () => {
    const trace = traceAttentionSoftmax(true);

    expect(attentionSoftmaxRow(trace, "red").context).toEqual([2, 0]);
    expect(attentionSoftmaxRow(trace, "blue").valueContributions).toEqual([
      [1, 0],
      [0, 0.5],
      [0, 0],
    ]);
    expect(attentionSoftmaxRow(trace, "blue").context).toEqual([1, 0.5]);
    expect(attentionSoftmaxRow(trace, "purple").context[0]).toBeCloseTo(1.7832330964304126);
  });

  it("subtracts the row maximum before exponentiating large scores", () => {
    const trace = traceAttentionSoftmax(
      false,
      [[1000, 999, 998], [0, 0, 0], [0, 0, 0]],
    );
    const row = trace.rows[0]!;

    expect(row.shiftedScores).toEqual([0, -1, -2]);
    expect(row.exponentials.every(Number.isFinite)).toBe(true);
    expect(row.weights.reduce((sum, weight) => sum + weight, 0)).toBeCloseTo(1);
  });

  it("switches between causal and full-context value mixing interactively", () => {
    render(React.createElement(AttentionWorkbench));
    fireEvent.click(screen.getByRole("button", { name: "Apply softmax and causal mask" }));

    expect(screen.getByRole("heading", { name: "Causal-softmax mixer" })).toBeTruthy();
    expect(screen.getByLabelText("Causal attention weight matrix").textContent)
      .toMatch(/red q.*1.*blocked.*blocked.*blue q.*0\.5.*0\.5.*blocked/s);
    expect(screen.getByLabelText("Selected softmax row trace").textContent)
      .toMatch(/0\.707107.*0\.707107.*1\.414214.*0\.707107.*0\.707107.*blocked.*\[1, 1, 0\].*\[0\.5, 0\.5, 0\]/s);
    expect(screen.getByLabelText("Selected weighted value mix").textContent)
      .toMatch(/0\.5 × \[2, 0\].*= \[1, 0\].*0\.5 × \[0, 1\].*= \[0, 0\.5\].*0 × \[2, 1\].*= \[0, 0\]/s);

    fireEvent.click(screen.getByRole("checkbox", { name: /Block future keys/ }));
    expect(screen.getByRole("heading", { name: "Unmasked attention weights" })).toBeTruthy();
    expect(screen.getByLabelText("Selected softmax row trace").textContent)
      .toContain("[0.248255, 0.248255, 0.50349]");
    expect(screen.getByLabelText("Selected weighted value mix").textContent)
      .toContain("[1.50349, 0.751745]");

    fireEvent.click(screen.getByRole("button", { name: "Return to Q/K/V scores" }));
    expect(screen.getByRole("heading", { name: "Query-key score microscope" })).toBeTruthy();
  });
});
