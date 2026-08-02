import { describe, expect, it } from "vitest";
import React from "react";
import { fireEvent, render, screen, within } from "@testing-library/react";
import { transpileLatticeInBrowser } from "@coding-adventures/lattice-transpiler/src/browser.js";
import { activate } from "./activation.js";
import { AttentionWorkbench } from "./AttentionWorkbench.js";
import { AutoencoderWorkbench } from "./AutoencoderWorkbench.js";
import { traceTwoNumberAutoencoder } from "./autoencoder-lab.js";
import { BackwardOptimizerLoweringWorkbench } from "./BackwardOptimizerLoweringWorkbench.js";
import {
  compileBackwardTrainingIr,
  compileMatrixTrainingIr,
  compileOptimizerTrainingIr,
  traceBackwardOptimizerLowering,
  traceBackwardOptimizerLoweringProgram,
  type BackwardOptimizerLoweringScenario,
} from "./backward-optimizer-lowering-lab.js";
import {
  decoderTrainingRow,
  traceTinyDecoderTraining,
} from "./decoder-language-model-lab.js";
import { ConvolutionWorkbench } from "./ConvolutionWorkbench.js";
import { DeepTrainingWorkbench } from "./DeepTrainingWorkbench.js";
import { DynamicAutogradWorkbench } from "./DynamicAutogradWorkbench.js";
import {
  traceDynamicAutograd,
  traceDynamicAutogradProgram,
  type DynamicAutogradScenario,
} from "./dynamic-autograd-lab.js";
import { ForwardLoweringWorkbench } from "./ForwardLoweringWorkbench.js";
import {
  traceForwardLowering,
  traceForwardLoweringProgram,
  type ForwardLoweringScenario,
} from "./forward-lowering-lab.js";
import { GradientAccumulationWorkbench } from "./GradientAccumulationWorkbench.js";
import {
  traceGradientAccumulation,
  traceGradientAccumulationProgram,
  type GradientAccumulationScenario,
} from "./gradient-accumulation-lab.js";
import { traceOneDimensionalDiffusion } from "./diffusion-lab.js";
import { traceOneDimensionalGan } from "./gan-lab.js";
import { HiddenLayerWorkbench } from "./HiddenLayerWorkbench.js";
import { GraphNeighborhoodWorkbench } from "./GraphNeighborhoodWorkbench.js";
import { traceGraphNeighborhoodComparison } from "./graph-neighborhood-lab.js";
import { GradientFlowWorkbench } from "./GradientFlowWorkbench.js";
import { traceGradientFlow } from "./gradient-flow-lab.js";
import { HopfieldWorkbench } from "./HopfieldWorkbench.js";
import { traceHopfieldRecall } from "./hopfield-memory.js";
import { ImageCnnWorkbench } from "./ImageCnnWorkbench.js";
import { InitializationWorkbench } from "./InitializationWorkbench.js";
import { initializerScale, traceInitializationDistributions } from "./initialization-distribution-lab.js";
import { MessagePassingWorkbench } from "./MessagePassingWorkbench.js";
import { traceTinyMessagePassing } from "./message-passing-lab.js";
import { OptimizationWorkbench } from "./OptimizationWorkbench.js";
import { RecurrentWorkbench } from "./RecurrentWorkbench.js";
import { RepresentationWorkbench } from "./RepresentationWorkbench.js";
import { ResidualWorkbench } from "./ResidualWorkbench.js";
import { TrainingStepMicroscope } from "./TrainingStepMicroscope.js";
import { TrainingStabilizersWorkbench } from "./TrainingStabilizersWorkbench.js";
import { traceTrainingStabilizers } from "./training-stabilizers-lab.js";
import { TensorBroadcastingWorkbench } from "./TensorBroadcastingWorkbench.js";
import { traceBroadcastAdd, traceTensorBroadcasting } from "./tensor-broadcasting-lab.js";
import latticeSource from "./styles/app.lattice?raw";
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
import { traceScalarVariationalAutoencoder } from "./variational-lab.js";
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
import {
  multiHeadAttentionRow,
  traceMultiHeadAttention,
} from "./multi-head-attention-lab.js";

const productionCss = transpileLatticeInBrowser(latticeSource);

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

describe("multi-head attention add-and-norm lab", () => {
  it("runs two independent causal heads for the blue query", () => {
    const blue = multiHeadAttentionRow(traceMultiHeadAttention(), "blue");

    expect(blue.heads[0]!.weights).toEqual([0.5, 0.5, 0]);
    expect(blue.heads[0]!.context).toBe(1);
    expect(blue.heads[1]!.weights[0]).toBeCloseTo(0.2689414213699951);
    expect(blue.heads[1]!.weights[1]).toBeCloseTo(0.7310585786300049);
    expect(blue.heads[1]!.context).toBeCloseTo(0.7310585786300049);
  });

  it("concatenates, projects, adds the residual, and normalizes", () => {
    const blue = multiHeadAttentionRow(traceMultiHeadAttention(), "blue");

    expect(blue.concatenated).toEqual([1, 0.7310585786300049]);
    expect(blue.projectedAttention).toEqual(blue.concatenated);
    expect(blue.residualSum).toEqual([1, 1.7310585786300048]);
    expect(blue.layerNorm.mean).toBeCloseTo(1.3655292893150024);
    expect(blue.layerNorm.variance).toBeCloseTo(0.13361166134713073);
    expect(blue.output[0]).toBeCloseTo(-0.9999625802171532);
    expect(blue.output[1]).toBeCloseTo(0.9999625802171532);
  });

  it("keeps residual and normalization ablations numerically honest", () => {
    const noResidual = multiHeadAttentionRow(
      traceMultiHeadAttention(false, true),
      "blue",
    );
    const rawAttention = multiHeadAttentionRow(
      traceMultiHeadAttention(false, false),
      "blue",
    );

    expect(noResidual.residualSum).toEqual(noResidual.projectedAttention);
    expect(rawAttention.output).toEqual([1, 0.7310585786300049]);
  });

  it("walks from single-head weights through both heads and block ablations", () => {
    render(React.createElement(AttentionWorkbench));
    fireEvent.click(screen.getByRole("button", { name: "Apply softmax and causal mask" }));
    fireEvent.click(screen.getByRole("button", { name: "Open multi-head add and norm" }));

    expect(screen.getByRole("heading", { name: "Multi-head add-and-norm block" })).toBeTruthy();
    expect(screen.getByLabelText("Two attention heads for blue").textContent)
      .toMatch(/Head A - horizontal.*context 1.*scores\[0, 0, 0\].*red0\.5.*blue0\.5.*purpleblocked.*Head B - vertical.*context 0\.731059.*scores\[0, 1, 1\].*red0\.268941.*blue0\.731059.*purpleblocked/s);
    expect(screen.getByLabelText("Concatenate project and add residual trace").textContent)
      .toMatch(/\[1, 0\.731059\].*\[0, 1\].*\[1, 1\.731059\]/s);
    expect(screen.getByLabelText("Layer normalization arithmetic").textContent)
      .toContain("[-0.999963, 0.999963]");

    fireEvent.click(screen.getByRole("checkbox", { name: /Add residual token/ }));
    fireEvent.click(screen.getByRole("checkbox", { name: /Apply layer normalization/ }));
    expect(screen.getByText("residual removed")).toBeTruthy();
    expect(screen.getByText("Layer normalization bypassed")).toBeTruthy();
    expect(screen.getByLabelText("Layer normalization arithmetic").textContent)
      .toContain("block output[1, 0.731059]");

    fireEvent.click(screen.getByRole("button", { name: "Return to single-head weights" }));
    expect(screen.getByRole("heading", { name: "Causal-softmax mixer" })).toBeTruthy();
  });
});

describe("tiny decoder-only language model training lab", () => {
  it("keeps the decoder workbench in the generated production stylesheet", () => {
    const css = productionCss;

    expect(css).toContain(".workspace--decoder");
    expect(css).toContain(".decoder-forward-flow");
  });

  it("shifts one sequence into two causal next-token examples", () => {
    const trace = traceTinyDecoderTraining();

    expect(trace.sequence).toEqual(["red", "blue", "purple"]);
    expect(trace.rows.map((row) => ({
      prefix: row.causalPrefix,
      input: row.inputToken,
      target: row.targetToken,
    }))).toEqual([
      { prefix: ["red"], input: "red", target: "blue" },
      { prefix: ["red", "blue"], input: "blue", target: "purple" },
    ]);
  });

  it("reproduces the hand-calculable logits, probabilities, and losses", () => {
    const trace = traceTinyDecoderTraining();
    const first = decoderTrainingRow(trace, 0);
    const second = decoderTrainingRow(trace, 1);

    expect(first.logitProducts).toEqual([[1, 0], [0, 0], [-1, 0]]);
    expect(first.logits).toEqual([1, 0, -1]);
    expect(first.probabilities.reduce((sum, value) => sum + value, 0)).toBeCloseTo(1);
    expect(first.targetProbability).toBeCloseTo(0.24472847105479764);
    expect(second.targetProbability).toBeCloseTo(0.09003057317038046);
    expect(trace.meanLoss).toBeCloseTo(1.9076059644443801);
  });

  it("reduces shared gradients and lowers loss after one SGD step", () => {
    const trace = traceTinyDecoderTraining();

    expect(trace.unembeddingGradient[0]).toEqual([
      0.3326204778874109,
      -0.37763576447260117,
      0.04501528658519023,
    ]);
    expect(trace.unembeddingGradient[1]).toEqual([
      0.12236423552739882,
      0.3326204778874109,
      -0.4549847134148098,
    ]);
    expect(trace.updatedBias[2]).toBeCloseTo(0.20498471341480978);
    expect(trace.gradientCheck.maxAbsoluteError).toBeLessThan(1e-8);
    expect(trace.postUpdateMeanLoss).toBeCloseTo(1.456094285138867);
    expect(trace.postUpdateMeanLoss).toBeLessThan(trace.meanLoss);
  });

  it("opens the decoder trace and compares the same position before and after update", () => {
    render(React.createElement(AttentionWorkbench));
    fireEvent.click(screen.getByRole("button", { name: "Apply softmax and causal mask" }));
    fireEvent.click(screen.getByRole("button", { name: "Open multi-head add and norm" }));
    fireEvent.click(screen.getByRole("button", { name: "Open tiny decoder training" }));

    expect(screen.getByRole("heading", { name: "Tiny decoder training trace" })).toBeTruthy();
    expect(screen.getByLabelText("Causal next-token sequence shift").textContent)
      .toMatch(/position 0.*red.*blue.*position 1.*red blue.*purple/s);
    expect(screen.getByLabelText("Selected decoder prediction at position 1").textContent)
      .toMatch(/\[0, 1\].*\[0, 1, -1\].*P\(purple\) = 0\.090031.*2\.407606/s);
    expect(screen.getByLabelText("Decoder loss gradient trace").textContent)
      .toContain("[0.122364, 0.33262, -0.454985]");
    expect(screen.getByLabelText("Shared decoder head SGD update").textContent)
      .toMatch(/Central finite-difference audit.*max error 1\.979e-10.*mean loss before1\.907606.*mean loss after one step1\.456094/s);

    fireEvent.click(screen.getByRole("checkbox", { name: /Use updated vocabulary head/ }));
    expect(screen.getByLabelText("Selected decoder prediction at position 1").textContent)
      .toMatch(/updated logits.*P\(purple\) = 0\.15446.*1\.867817/s);

    fireEvent.click(screen.getByRole("button", { name: "Return to multi-head block" }));
    expect(screen.getByRole("heading", { name: "Multi-head add-and-norm block" })).toBeTruthy();
  });
});

describe("two-number autoencoder bottleneck lab", () => {
  it("keeps the autoencoder workbench in the generated production stylesheet", () => {
    const css = productionCss;

    expect(css).toContain(".workspace--autoencoder");
    expect(css).toContain(".autoencoder-bottleneck");
  });

  it("compresses two values and reconstructs both from one scalar", () => {
    const trace = traceTwoNumberAutoencoder();

    expect(trace.forward.encoderProducts).toEqual([1, 0.25]);
    expect(trace.forward.bottleneck).toBe(1.25);
    expect(trace.forward.decoderProducts).toEqual([1.5, -1]);
    expect(trace.forward.reconstruction).toEqual([1.6, -1.2]);
    expect(trace.forward.loss).toBeCloseTo(0.1);
  });

  it("adds both decoder routes into the bottleneck gradient", () => {
    const trace = traceTwoNumberAutoencoder();

    expect(trace.backward.bottleneckGradientContributions).toEqual([
      -0.47999999999999987,
      0.15999999999999998,
    ]);
    expect(trace.backward.bottleneckGradient).toBeCloseTo(-0.32);
    expect(trace.backward.encoderWeightGradients).toEqual([
      -0.6399999999999998,
      0.3199999999999999,
    ]);
  });

  it("audits every parameter and lowers reconstruction loss", () => {
    const trace = traceTwoNumberAutoencoder();

    expect(trace.gradientCheck.parameterOrder).toHaveLength(7);
    expect(trace.gradientCheck.maxAbsoluteError).toBeLessThan(1e-8);
    expect(trace.updatedParameters.encoder.weights).toEqual([0.564, -0.282]);
    expect(trace.postUpdate.bottleneck).toBeCloseTo(1.442);
    expect(trace.postUpdate.reconstruction).toEqual([1.9425, -1.29755]);
    expect(trace.postUpdate.loss).toBeCloseTo(0.04592112625);
    expect(trace.postUpdate.loss).toBeLessThan(trace.forward.loss);
  });

  it("selects either decoder branch and reruns the updated model", () => {
    render(React.createElement(AutoencoderWorkbench));

    expect(screen.getByRole("heading", { name: "Two numbers through one bottleneck" })).toBeTruthy();
    expect(screen.getByLabelText("Autoencoder encode and decode path").textContent)
      .toMatch(/x02.*x1-1.*-1 x -0\.25 = 0\.25.*bottleneck z1\.25.*x_hat01\.6.*x_hat1-1\.2/s);
    expect(screen.getByLabelText("Selected autoencoder reconstruction 0").textContent)
      .toMatch(/1\.25.*1\.2.*0\.1.*1\.6.*2.*-0\.4.*squared 0\.16/s);
    expect(screen.getByLabelText("Autoencoder bottleneck gradient trace").textContent)
      .toMatch(/-0\.4 x 1\.2.*-0\.48.*-0\.2 x -0\.8.*0\.16.*bottleneck gradient-0\.32/s);
    expect(screen.getByLabelText("Autoencoder SGD update and gradient audit").textContent)
      .toMatch(/7 parameters.*max error 2\.032e-11.*loss before0\.1.*loss after0\.04592113/s);

    fireEvent.click(screen.getByRole("button", { name: "output 1" }));
    expect(screen.getByLabelText("Selected autoencoder reconstruction 1").textContent)
      .toContain("error / loss gradient-0.2");

    fireEvent.click(screen.getByRole("checkbox", { name: /Use updated parameters/ }));
    expect(screen.getByLabelText("Autoencoder encode and decode path").textContent)
      .toMatch(/after one SGD step.*bottleneck z1\.442.*x_hat01\.9425.*x_hat1-1\.29755/s);
    expect(screen.getByLabelText("Selected autoencoder reconstruction 1").textContent)
      .toContain("error / loss gradient-0.29755");
  });
});

describe("scalar variational autoencoder lab", () => {
  it("keeps the variational workbench in the generated production stylesheet", () => {
    const css = productionCss;

    expect(css).toContain(".workspace--variational");
    expect(css).toContain(".variational-sample-node");
  });

  it("turns saved noise into a reproducible latent sample", () => {
    const trace = traceScalarVariationalAutoencoder();

    expect(trace.forward.mean).toBeCloseTo(0.4);
    expect(trace.forward.logVariance).toBe(0);
    expect(trace.forward.standardDeviation).toBe(1);
    expect(trace.forward.noiseContribution).toBe(0.5);
    expect(trace.forward.latent).toBe(0.9);
    expect(trace.forward.reconstruction).toBe(0.9);
  });

  it("keeps reconstruction and beta-weighted KL routes separate", () => {
    const trace = traceScalarVariationalAutoencoder();

    expect(trace.forward.reconstructionLoss).toBeCloseTo(0.005);
    expect(trace.forward.kl).toBeCloseTo(0.08);
    expect(trace.forward.weightedKl).toBeCloseTo(0.008);
    expect(trace.forward.totalLoss).toBeCloseTo(0.013);
    expect(trace.backward.reconstructionMeanGradient).toBeCloseTo(-0.1);
    expect(trace.backward.weightedKlMeanGradient).toBeCloseTo(0.04);
    expect(trace.backward.meanGradient).toBeCloseTo(-0.06);
  });

  it("shows beta cancelling and reversing the mean direction", () => {
    const balanced = traceScalarVariationalAutoencoder(0.25);
    const priorLed = traceScalarVariationalAutoencoder(1);

    expect(balanced.backward.meanGradient).toBe(0);
    expect(priorLed.backward.meanGradient).toBeCloseTo(0.3);
    expect(balanced.samplingEpsilon).toBe(priorLed.samplingEpsilon);
  });

  it("audits all parameters and lowers the selected objective", () => {
    const trace = traceScalarVariationalAutoencoder();

    expect(trace.gradientCheck.parameterOrder).toHaveLength(6);
    expect(trace.gradientCheck.maxAbsoluteError).toBeLessThan(1e-8);
    expect(trace.updatedParameters.encoder.mean.weight).toBeCloseTo(0.406);
    expect(trace.updatedParameters.encoder.mean.bias).toBeCloseTo(0.006);
    expect(trace.postUpdate.mean).toBeCloseTo(0.412);
    expect(trace.postUpdate.latent).toBeCloseTo(0.9132515638028976);
    expect(trace.postUpdate.reconstruction).toBeCloseTo(0.9314708278771237);
    expect(trace.postUpdate.totalLoss).toBeCloseTo(0.01083594975889346);
    expect(trace.postUpdate.totalLoss).toBeLessThan(trace.forward.totalLoss);
  });

  it("switches representation labs, beta, gradient target, and updated model", () => {
    render(React.createElement(RepresentationWorkbench));

    expect(screen.getByRole("heading", { name: "Two numbers through one bottleneck" })).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Variational sample" }));

    expect(screen.getByRole("heading", { name: "One Gaussian latent sample, fully unpacked" })).toBeTruthy();
    expect(screen.getByLabelText("Variational encode sample and decode path").textContent)
      .toMatch(/saved epsilon = 0\.5.*mean = 0\.4.*log var = 0.*sigma = 1.*z = 0\.9.*x_hat = 0\.9/s);
    expect(screen.getByLabelText("Variational reconstruction and KL objective").textContent)
      .toMatch(/reconstruction.*0\.005.*KL to Normal\(0, 1\).*0\.08.*beta0\.1.*weighted total.*0\.013/s);

    fireEvent.click(screen.getByRole("button", { name: "beta 0.25" }));
    expect(screen.getByLabelText("Variational mean gradient tradeoff").textContent)
      .toMatch(/reconstruction route-0\.1.*beta x KL route0\.25 x 0\.4.*0\.1.*combined mean gradient0.*routes cancel exactly/s);

    fireEvent.click(screen.getByRole("button", { name: "log-variance output" }));
    expect(screen.getByLabelText("Variational log-variance gradient tradeoff").textContent)
      .toContain("combined log-variance gradient-0.025");

    fireEvent.click(screen.getByRole("checkbox", { name: /Use updated parameters/ }));
    expect(screen.getByLabelText("Variational encode sample and decode path").textContent)
      .toMatch(/after one SGD step.*mean = 0\.4 \+ 0 = 0\.4.*log var = 0\.0025 \+ 0\.0025 = 0\.005.*z = 0\.90125156.*x_hat = 0\.91936283/s);
  });
});

describe("one-dimensional GAN game", () => {
  it("keeps the adversarial workbench in the generated production stylesheet", () => {
    const css = productionCss;

    expect(css).toContain(".workspace--gan");
    expect(css).toContain(".gan-number-line__marker--fake");
  });

  it("scores one real and one generated point before either player moves", () => {
    const trace = traceOneDimensionalGan();

    expect(trace.initial.fakeSample).toBeCloseTo(0.2);
    expect(trace.initial.realProbability).toBeCloseTo(0.7310585786300049);
    expect(trace.initial.fakeProbability).toBeCloseTo(0.549833997312478);
    expect(trace.initial.discriminatorLoss).toBeCloseTo(0.5557002784499074);
    expect(trace.initial.generatorLoss).toBeCloseTo(0.5981388693815918);
  });

  it("detaches the fake and lowers only the discriminator objective", () => {
    const trace = traceOneDimensionalGan();

    expect(trace.discriminatorStep.backward.fakeSampleGradient).toBe(0);
    expect(trace.discriminatorStep.backward.weightGradient)
      .toBeCloseTo(-0.07948731095374975);
    expect(trace.discriminatorStep.backward.biasGradient)
      .toBeCloseTo(0.14044628797124142);
    expect(trace.discriminatorStep.updatedParameters.weight)
      .toBeCloseTo(1.0397436554768749);
    expect(trace.discriminatorStep.updatedParameters.bias)
      .toBeCloseTo(-0.07022314398562071);
    expect(trace.discriminatorStep.state.discriminatorLoss)
      .toBeLessThan(trace.initial.discriminatorLoss);
  });

  it("freezes the updated critic and sends its input slope into the generator", () => {
    const trace = traceOneDimensionalGan();

    expect(trace.generatorStep.backward.fakeSampleGradient)
      .toBeCloseTo(-0.48412848285494775);
    expect(trace.generatorStep.updatedParameters.weight)
      .toBeCloseTo(0.32103212071373693);
    expect(trace.generatorStep.updatedParameters.bias)
      .toBeCloseTo(0.12103212071373694);
    expect(trace.generatorStep.state.fakeSample).toBeCloseTo(0.44206424142747386);
    expect(trace.generatorStep.state.generatorLoss)
      .toBeLessThan(trace.discriminatorStep.state.generatorLoss);
    expect(trace.generatorStep.state.discriminatorLoss)
      .toBeGreaterThan(trace.discriminatorStep.state.discriminatorLoss);
  });

  it("audits each player against its own fixed-opponent objective", () => {
    const trace = traceOneDimensionalGan();

    expect(trace.discriminatorStep.gradientCheck.parameterOrder)
      .toEqual(["discriminator.weight", "discriminator.bias"]);
    expect(trace.generatorStep.gradientCheck.parameterOrder)
      .toEqual(["generator.weight", "generator.bias"]);
    expect(trace.discriminatorStep.gradientCheck.maxAbsoluteError)
      .toBeLessThan(1e-8);
    expect(trace.generatorStep.gradientCheck.maxAbsoluteError)
      .toBeLessThan(1e-8);
  });

  it("switches from the forward pass through both alternating moves", () => {
    render(React.createElement(RepresentationWorkbench));
    fireEvent.click(screen.getByRole("button", { name: "Adversarial game" }));

    expect(screen.getByRole("heading", {
      name: "A generator and discriminator on one number line",
    })).toBeTruthy();
    expect(screen.getByLabelText("GAN samples and discriminator probabilities").textContent)
      .toMatch(/neither has moved.*fake 0\.2.*real 1.*0\.73105858.*0\.549834/s);
    expect(screen.getByLabelText("GAN competing objectives").textContent)
      .toMatch(/D loss 0\.55570028.*G loss 0\.59813887/s);

    fireEvent.click(screen.getByRole("button", { name: /Discriminator moves/ }));
    expect(screen.getByLabelText("GAN active gradient route").textContent)
      .toMatch(/generated value is detached.*-0\.13447071.*0\.274917.*-0\.07948731 \/ 0\.14044629.*gradient into fake = 0/s);
    expect(screen.getByLabelText("GAN competing objectives").textContent)
      .toContain("D loss 0.54296489");

    fireEvent.click(screen.getByRole("button", { name: /Generator responds/ }));
    expect(screen.getByLabelText("GAN samples and discriminator probabilities").textContent)
      .toMatch(/updated discriminator is frozen.*fake 0\.44206424.*0\.59614074/s);
    expect(screen.getByLabelText("GAN active gradient route").textContent)
      .toMatch(/critic becomes a teaching signal.*-0\.46562293.*-0\.48412848.*D parameters stay frozen/s);
    expect(screen.getByLabelText("GAN competing objectives").textContent)
      .toMatch(/D loss 0\.61411974.*G loss 0\.51727849/s);
  });
});

describe("one-dimensional diffusion round trip", () => {
  it("keeps the diffusion workbench in the generated production stylesheet", () => {
    const css = productionCss;

    expect(css).toContain(".workspace--diffusion");
    expect(css).toContain(".diffusion-reverse-lane");
  });

  it("mixes one clean sample with saved noise at two levels", () => {
    const trace = traceOneDimensionalDiffusion();

    expect(trace.forwardSteps.map((row) => row.alphaBar))
      .toEqual([0.64, 0.36]);
    expect(trace.forwardSteps.map((row) => row.signalScale))
      .toEqual([0.8, 0.6]);
    expect(trace.forwardSteps.map((row) => row.noiseScale))
      .toEqual([0.6, 0.8]);
    expect(trace.forwardSteps.map((row) => row.noisySample))
      .toEqual([0.5, 0.19999999999999996]);
  });

  it("reduces both timestep examples into shared denoiser gradients", () => {
    const trace = traceOneDimensionalDiffusion();

    expect(trace.initialMeanLoss).toBeCloseTo(0.125);
    expect(trace.backward.perStep.map((row) => row.predictionGradient))
      .toEqual([0.25, 0.25]);
    expect(trace.backward.sampleWeightGradient).toBeCloseTo(0.175);
    expect(trace.backward.timestepWeightGradient).toBeCloseTo(0.375);
    expect(trace.backward.biasGradient).toBeCloseTo(0.5);
  });

  it("audits three slopes and learns a better noise predictor", () => {
    const trace = traceOneDimensionalDiffusion();

    expect(trace.gradientCheck.parameterOrder).toEqual([
      "denoiser.sample_weight",
      "denoiser.timestep_weight",
      "denoiser.bias",
    ]);
    expect(trace.gradientCheck.maxAbsoluteError).toBeLessThan(1e-8);
    expect(trace.updatedDenoiser).toEqual({
      sampleWeight: -0.0875,
      timestepWeight: -0.1875,
      bias: -0.25,
    });
    expect(trace.postUpdateDenoising.map((row) => row.predictedNoise))
      .toEqual([-0.3875, -0.45499999999999996]);
    expect(trace.postUpdateMeanLoss).toBeCloseTo(0.0036703125);
    expect(trace.postUpdateMeanLoss).toBeLessThan(trace.initialMeanLoss);
  });

  it("runs the updated denoiser through two deterministic reverse means", () => {
    const trace = traceOneDimensionalDiffusion();

    expect(trace.reverseSteps.map((row) => row.t)).toEqual([2, 1]);
    expect(trace.reverseSteps[0]!.outputMean).toBeCloseTo(0.5984375);
    expect(trace.reverseSteps[1]!.predictedNoise)
      .toBeCloseTo(-0.39611328125);
    expect(trace.finalReconstruction).toBeCloseTo(1.0451318359375);
    expect(trace.finalAbsoluteError).toBeCloseTo(0.0451318359375);
  });

  it("rejects a malformed diffusion schedule", () => {
    expect(() => traceOneDimensionalDiffusion(
      1,
      -0.5,
      0.5,
      { sampleWeight: 0, timestepWeight: 0, bias: 0 },
      [
        { t: 1, beta: 0.36, normalizedT: 0.5 },
        { t: 3, beta: 0.4375, normalizedT: 1 },
      ],
    )).toThrow(/consecutive increasing diffusion steps/);
  });

  it("walks through forward noise, learning, and both reverse steps", () => {
    render(React.createElement(RepresentationWorkbench));
    fireEvent.click(screen.getByRole("button", { name: "Diffusion path" }));

    expect(screen.getByRole("heading", {
      name: "One clean number through a diffusion round trip",
    })).toBeTruthy();
    expect(screen.getByLabelText("Diffusion forward noise schedule").textContent)
      .toMatch(/saved epsilon = -0\.5.*x0 = 1.*x1 = 0\.5.*alpha_bar = 0\.64.*x2 = 0\.2.*alpha_bar = 0\.36/s);
    expect(screen.getByLabelText("Diffusion noise prediction objective").textContent)
      .toMatch(/initial mean loss0\.125.*predicted 0.*target -0\.5/s);

    fireEvent.click(screen.getByRole("button", { name: /Predict saved noise/ }));
    expect(screen.getByLabelText("Diffusion noise prediction objective").textContent)
      .toMatch(/mean loss after SGD0\.00367031.*predicted -0\.3875.*predicted -0\.455/s);
    expect(screen.getByLabelText("Diffusion denoiser gradient and update").textContent)
      .toMatch(/sample weight gradient0\.175.*timestep weight gradient0\.375.*bias gradient0\.5.*max error 6\.439e-12.*0\.125 -> 0\.00367031/s);

    fireEvent.click(screen.getByRole("button", { name: /Denoise step 2/ }));
    expect(screen.getByLabelText("Diffusion deterministic reverse mean path").textContent)
      .toMatch(/reverse t = 2.*mean1 = 0\.5984375.*reverse t = 1\?/s);

    fireEvent.click(screen.getByRole("button", { name: /Denoise step 1/ }));
    expect(screen.getByLabelText("Diffusion deterministic reverse mean path").textContent)
      .toMatch(/reverse t = 2.*mean1 = 0\.5984375.*reverse t = 1.*mean0 = 1\.04513184.*absolute error 0\.04513184/s);
  });
});

describe("Hopfield associative memory", () => {
  it("keeps the Hopfield workbench in the generated production stylesheet", () => {
    const css = productionCss;

    expect(css).toContain(".workspace--hopfield");
    expect(css).toContain(".hopfield-recall-lane");
  });

  it("stores one bipolar pattern in symmetric zero-diagonal weights", () => {
    const trace = traceHopfieldRecall();

    expect(trace.weights).toEqual([
      [0, -0.25, 0.25, -0.25],
      [-0.25, 0, -0.25, 0.25],
      [0.25, -0.25, 0, -0.25],
      [-0.25, 0.25, -0.25, 0],
    ]);
    expect(trace.normalization).toBe(4);
  });

  it("starts one bit away at half overlap and zero energy", () => {
    const trace = traceHopfieldRecall();

    expect(trace.initialHammingDistance).toBe(1);
    expect(trace.initialOverlap).toBeCloseTo(0.5);
    expect(trace.initialEnergy).toBeCloseTo(0);
  });

  it("repairs the flipped bit with three positive incoming votes", () => {
    const first = traceHopfieldRecall().updates[0]!;

    expect(first.incoming.map((row) => row.contribution)).toEqual([0, 0.25, 0.25, 0.25]);
    expect(first.localField).toBeCloseTo(0.75);
    expect(first.previousState).toBe(-1);
    expect(first.nextState).toBe(1);
    expect(first.stateAfter).toEqual([1, -1, 1, -1]);
  });

  it("descends in energy and finishes at the saved fixed point", () => {
    const trace = traceHopfieldRecall();

    expect(trace.updates.map((row) => [row.energyBefore, row.energyAfter])).toEqual([
      [0, -1.5],
      [-1.5, -1.5],
      [-1.5, -1.5],
      [-1.5, -1.5],
    ]);
    expect(trace.updates.map((row) => row.changed)).toEqual([true, false, false, false]);
    expect(trace.finalState).toEqual([1, -1, 1, -1]);
    expect(trace.finalOverlap).toBeCloseTo(1);
    expect(trace.finalHammingDistance).toBe(0);
    expect(trace.converged).toBe(true);
  });

  it("rejects non-bipolar states and malformed update orders", () => {
    expect(() => traceHopfieldRecall([1, 0, 1, -1])).toThrow(/bipolar/);
    expect(() => traceHopfieldRecall(
      [1, -1, 1, -1],
      [-1, -1, 1, -1],
      [0, 1, 1, 3],
    )).toThrow(/permutation/);
  });

  it("walks through storage, cue repair, and the stable sweep", () => {
    render(React.createElement(HopfieldWorkbench));

    expect(screen.getByRole("heading", {
      name: "Restore one flipped bit with four connected neurons",
    })).toBeTruthy();
    expect(screen.getByLabelText("Hopfield Hebbian storage rule").textContent)
      .toMatch(/\[\+1, -1, \+1, -1\].*divide by 4.*from 0.*-0\.25.*0\.25/s);

    fireEvent.click(screen.getByRole("button", { name: /One flipped bit/ }));
    expect(screen.getByLabelText("Hopfield asynchronous recall trace").textContent)
      .toMatch(/damaged cue\[-1, -1, \+1, -1\].*distance 1.*normalized overlap0\.5.*Hopfield energy0/s);

    fireEvent.click(screen.getByRole("button", { name: /Update neuron 0/ }));
    expect(screen.getByLabelText("Hopfield active neuron calculation").textContent)
      .toMatch(/active neuron0.*-0\.25 x -1 = 0\.25.*local field -> next state0\.75 -> \+1.*energy before -> after0 -> -1\.5.*overlap before -> after0\.5 -> 1/s);

    fireEvent.click(screen.getByRole("button", { name: /Update neuron 3/ }));
    expect(screen.getByLabelText("Hopfield asynchronous recall trace").textContent)
      .toMatch(/update 0\[\+1, -1, \+1, -1\].*update 3\[\+1, -1, \+1, -1\].*Hopfield energy-1\.5/s);
    expect(screen.getByLabelText("Hopfield phase controls").textContent)
      .toMatch(/fixed point recovered/);
  });
});

describe("tiny graph message passing", () => {
  it("keeps the message-passing workbench in the production stylesheet", () => {
    expect(productionCss).toContain(".workspace--message-passing");
    expect(productionCss).toContain(".message-ledger");
  });

  it("expands two undirected edges into four sorted directed messages", () => {
    const trace = traceTinyMessagePassing();
    expect(trace.directedMessages.map((row) => [row.source, row.target])).toEqual([[1, 0], [0, 1], [2, 1], [1, 2]]);
    expect(trace.directedMessages.map((row) => row.message)).toEqual([1, 0.5, -0.5, 1]);
  });

  it("sums each inbox and applies one shared update", () => {
    const trace = traceTinyMessagePassing();
    expect(trace.nodeUpdates.map((row) => row.aggregate)).toEqual([1, 0, 1]);
    expect(trace.nodeUpdates.map((row) => row.selfContribution)).toEqual([0.25, 0.5, -0.25]);
    expect(trace.nodeUpdates.map((row) => row.preactivation)).toEqual([0.75, 0, 0.25]);
    expect(trace.outputFeatures).toEqual([0.75, 0, 0.25]);
  });

  it("rejects invalid and duplicate undirected edges", () => {
    expect(() => traceTinyMessagePassing([1, 2], [{ source: 0, target: 0 }])).toThrow(/non-self/);
    expect(() => traceTinyMessagePassing([1, 2], [{ source: 0, target: 1 }, { source: 1, target: 0 }])).toThrow(/unique/);
  });

  it("reveals messages, the selected inbox, and final outputs", () => {
    render(React.createElement(MessagePassingWorkbench));
    expect(screen.getByRole("heading", { name: "Pass scalar messages across a three-node path" })).toBeTruthy();
    expect(screen.getByLabelText("Tiny graph and directed messages").textContent).toMatch(/old feature.*1 -> 0.*0 -> 1.*2 -> 1.*1 -> 2/s);

    fireEvent.click(screen.getByRole("button", { name: /Messages/ }));
    expect(screen.getByLabelText("Tiny graph and directed messages").textContent).toMatch(/1 -> 0.*0\.5 x 2.*1.*0 -> 1.*0\.5/s);
    fireEvent.click(screen.getByRole("button", { name: /Aggregate/ }));
    expect(screen.getByLabelText("Selected graph node update").textContent).toMatch(/Selected node 1.*0\.5 \+ -0\.5.*sum aggregate0.*self route0\.25 x 2.*0\.5.*preactivation.*0/s);
    fireEvent.click(screen.getByRole("button", { name: /Update/ }));
    expect(screen.getByLabelText("Selected graph node update").textContent).toMatch(/new feature.*0.*original features.*\[1, 2, -1\]/s);

    fireEvent.click(screen.getByRole("button", { name: /node 2 0\.25 new feature/ }));
    expect(screen.getByLabelText("Selected graph node update").textContent).toMatch(/Selected node 2.*incoming messages1.*sum aggregate1.*self route0\.25 x -1.*-0\.25.*preactivation.*0\.25.*new feature.*0\.25/s);
  });
});

describe("graph convolution and attention", () => {
  it("keeps the graph comparison in the production stylesheet", () => {
    expect(productionCss).toContain(".workspace--graph-neighborhood");
    expect(productionCss).toContain(".graph-softmax-summary");
  });

  it("computes degree-normalized GCN contributions", () => {
    const trace = traceGraphNeighborhoodComparison();
    expect(trace.degrees).toEqual([2, 3, 2]);
    expect(trace.gcn[1]!.rows.map((row) => row.coefficient)).toEqual([1 / Math.sqrt(6), 1 / 3, 1 / Math.sqrt(6)]);
    expect(trace.gcnOutputs[0]).toBeCloseTo(1.3164965809277263);
    expect(trace.gcnOutputs[1]).toBeCloseTo(2 / 3);
    expect(trace.gcnOutputs[2]).toBeCloseTo(0.31649658092772615);
  });

  it("computes stable normalized graph attention", () => {
    const trace = traceGraphNeighborhoodComparison();
    expect(trace.gat[1]!.rows.map((row) => row.shiftedScore)).toEqual([-1, 0, -3]);
    expect(trace.gat[1]!.rows.reduce((sum, row) => sum + row.attentionWeight, 0)).toBeCloseTo(1);
    expect(trace.gatOutputs).toEqual([1.7310585786300048, 1.6351464587795619, 1.8577223804673]);
  });

  it("rejects neighborhoods without self-loops or symmetry", () => {
    expect(() => traceGraphNeighborhoodComparison([1, 2], [[1], [0, 1]])).toThrow(/self-loops/);
    expect(() => traceGraphNeighborhoodComparison([1, 2], [[0], [0, 1]])).toThrow(/symmetric/);
  });

  it("switches targets and weighting rules without changing the graph", () => {
    render(React.createElement(GraphNeighborhoodWorkbench));
    expect(screen.getByRole("heading", { name: "Compare graph convolution with graph attention" })).toBeTruthy();
    expect(screen.getByLabelText("Graph convolution calculation").textContent).toMatch(/source 0.*1 \/ sqrt\(3 x 2\).*0\.408248.*source 1.*0\.333333.*source 2.*-0\.408248.*ReLU -> 0\.666667/s);
    fireEvent.click(screen.getByRole("button", { name: /Graph attention/ }));
    expect(screen.getByLabelText("Graph attention calculation").textContent).toMatch(/row max = 2.*denominator = 1\.417667.*weights sum = 1.*score 1 - max 2 = -1.*alpha = 0\.259496.*score -1 - max 2 = -3.*alpha = 0\.035119.*ReLU -> 1\.635146/s);
    fireEvent.click(screen.getByRole("button", { name: /node 2.*degree 2/ }));
    expect(screen.getByLabelText("Graph attention calculation").textContent).toMatch(/denominator = 1\.049787.*alpha = 0\.952574.*alpha = 0\.047426.*ReLU -> 1\.857722/s);
  });
});

describe("initialization and activation distributions", () => {
  it("keeps the deep-training explorer in the production stylesheet", () => {
    expect(productionCss).toContain(".workspace--initialization");
    expect(productionCss).toContain(".distribution-dot-plot");
  });

  it("derives Xavier and He scales from fan-in", () => {
    expect(initializerScale("xavier", 2)).toBeCloseTo(1 / Math.sqrt(2));
    expect(initializerScale("he", 2)).toBe(1);
  });

  it("traces the canonical Xavier tanh distribution", () => {
    const trace = traceInitializationDistributions("xavier", "tanh");
    expect(trace.layers[0]!.activations[0]).toEqual([Math.tanh(1 / Math.sqrt(2)), Math.tanh(-1 / Math.sqrt(2))]);
    const expectedStandardDeviations = [
      0.6088593650139138,
      0.49271338636057294,
      0.4563673571184874,
    ];
    trace.layers.forEach((layer, index) => {
      expect(layer.summary.standardDeviation).toBeCloseTo(expectedStandardDeviations[index]!, 14);
    });
  });

  it("makes shrinking and exploding ReLU signals visible", () => {
    const tiny = traceInitializationDistributions("tiny", "relu");
    const large = traceInitializationDistributions("large", "relu");
    const expectedTiny = [
      0.05,
      0.006959705453537528,
      0.0008569568250501307,
    ];
    const expectedLarge = [
      1,
      2.7838821814150108,
      6.855654600401044,
    ];
    tiny.layers.forEach((layer, index) => {
      expect(layer.summary.standardDeviation).toBeCloseTo(expectedTiny[index]!, 14);
    });
    large.layers.forEach((layer, index) => {
      expect(layer.summary.standardDeviation).toBeCloseTo(expectedLarge[index]!, 14);
    });
  });

  it("rejects malformed matrices", () => {
    expect(() => traceInitializationDistributions("xavier", "tanh", [[1], [1, 2]])).toThrow(/rectangular/);
    expect(() => traceInitializationDistributions("xavier", "tanh", [[1], [-1]], [[[1, 2]], [[1]]])).toThrow(/fan-in/);
  });

  it("switches initializer, activation, and layer in the explorer", () => {
    render(React.createElement(InitializationWorkbench));
    expect(screen.getByRole("heading", { name: "Initialization and activation distributions" })).toBeTruthy();
    expect(screen.getByLabelText("Selected layer hand calculation").textContent).toMatch(/0\.707107.*tanh = 0\.608859/s);
    fireEvent.click(screen.getByRole("button", { name: /Large.*fixed scale 2/ }));
    expect(screen.getByLabelText("Layer activation distributions").textContent).toMatch(/std 0\.964028.*std 0\.706474.*std 0\.963901/s);
    fireEvent.click(screen.getByRole("button", { name: "ReLU" }));
    fireEvent.click(screen.getByRole("button", { name: /Layer 3.*std 6\.855655/ }));
    expect(screen.getByLabelText("Selected activation distribution").textContent).toMatch(/standard deviation6\.855655.*exact zeros62\.5%/s);
  });
});

describe("vanishing and exploding gradients", () => {
  it("keeps the gradient-flow explorer in the production stylesheet", () => {
    expect(productionCss).toContain(".workspace--gradient-flow");
    expect(productionCss).toContain(".gradient-chain-equation");
  });

  it("traces a small tanh chain from loss to input", () => {
    const trace = traceGradientFlow("small-tanh");
    expect(trace.classification).toBe("vanishing");
    expect(trace.chainJacobian).toBeCloseTo(0.045877150455727246, 14);
    expect(trace.inputGradient).toBeCloseTo(0.0025900181205328957, 14);
    expect(trace.finiteDifferenceError).toBeLessThan(1e-10);
  });

  it("shows saturation can overwhelm large tanh weights", () => {
    const trace = traceGradientFlow("saturated-tanh");
    expect(trace.layers.map((layer) => layer.weight)).toEqual([3, 3, 3, 3]);
    expect(trace.layers[0]!.activationDerivative).toBeCloseTo(0.009866037165440211, 14);
    expect(trace.chainJacobian).toBeCloseTo(8.400447769691746e-7, 14);
  });

  it("keeps unit ReLU stable and makes large ReLU explode", () => {
    const stable = traceGradientFlow("unit-relu");
    const exploding = traceGradientFlow("large-relu");
    expect(stable.chainJacobian).toBe(1);
    expect(stable.inputGradient).toBe(1);
    expect(exploding.layers.map((layer) => layer.activation)).toEqual([2, 4, 8, 16]);
    expect(exploding.layers.map((layer) => layer.weightGradient)).toEqual([128, 128, 128, 128]);
    expect(exploding.chainJacobian).toBe(16);
    expect(exploding.inputGradient).toBe(256);
  });

  it("rejects invalid scenarios and finite-difference steps", () => {
    expect(() => traceGradientFlow("missing")).toThrow(/unknown/);
    expect(() => traceGradientFlow("small-tanh", 0)).toThrow(/positive/);
  });

  it("switches deep-training labs and gradient scenarios", () => {
    render(React.createElement(DeepTrainingWorkbench));
    fireEvent.click(screen.getByRole("button", { name: "Gradient flow" }));
    expect(screen.getByRole("heading", { name: "Vanishing and exploding gradients" })).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: /Large ReLU.*Every layer doubles/ }));
    expect(screen.getByLabelText("Gradient chain product").textContent).toMatch(/2.*2.*2.*2.*=.*16.*16 output error.*256.*input gradient/s);
  });

  it("opens any reverse-mode layer calculation", () => {
    render(React.createElement(GradientFlowWorkbench));
    fireEvent.click(screen.getByRole("button", { name: /layer 1.*upstream 0\.006587/ }));
    expect(screen.getByLabelText("Selected gradient calculation").textContent).toMatch(/0\.006587 x 0\.786448 = 0\.00518.*0\.00518 x 0\.5 = 0\.00259/s);
  });
});

describe("normalization dropout and residual comparisons", () => {
  it("keeps the stabilizer comparison in the production stylesheet", () => {
    expect(productionCss).toContain(".workspace--stabilizers");
    expect(productionCss).toContain(".stabilizer-equations");
  });

  it("pins the shared branch and exact layer-normalization statistics", () => {
    const trace = traceTrainingStabilizers();
    expect(trace.branch).toEqual([0.5, 0.5, 1.5, 1.5]);
    expect(trace.normalization.mean).toBe(1);
    expect(trace.normalization.variance).toBe(0.25);
    expect(trace.normalization.standardDeviation).toBe(0.5);
    expect(trace.normalization.normalized).toEqual([-1, -1, 1, 1]);
    expect(trace.normalization.upstreamDotNormalized).toBe(-2);
  });

  it("separates the four mechanisms in forward and reverse mode", () => {
    const routes = Object.fromEntries(
      traceTrainingStabilizers().routes.map((route) => [route.id, route]),
    );
    expect(routes.plain!.inputGradient).toEqual([0.5, 0, 0, -0.5]);
    expect(routes.normalization!.branchGradient).toEqual([1, -1, 1, -1]);
    expect(routes.normalization!.weightGradient).toBe(0);
    expect(routes.dropout!.output).toEqual([1, 0, 3, 0]);
    expect(routes.dropout!.inputGradient).toEqual([1, 0, 0, 0]);
    expect(routes.residual!.skipGradient).toEqual([1, 0, 0, -1]);
    expect(routes.residual!.inputGradient).toEqual([1.5, 0, 0, -1.5]);
  });

  it("keeps inverted-dropout expectation equal to evaluation output", () => {
    const dropout = traceTrainingStabilizers().dropout;
    expect(dropout.scaledMask).toEqual([2, 0, 2, 0]);
    expect(dropout.evaluationOutput).toEqual([0.5, 0.5, 1.5, 1.5]);
    expect(dropout.trainingExpectation).toEqual(dropout.evaluationOutput);
  });

  it("checks every analytical input and weight gradient numerically", () => {
    traceTrainingStabilizers().routes.forEach((route) => {
      expect(Math.max(...route.inputGradientAbsoluteError)).toBeLessThan(1e-8);
      expect(route.weightGradientAbsoluteError).toBeLessThan(1e-8);
    });
  });

  it("rejects malformed vectors masks probabilities and zero variance", () => {
    expect(() => traceTrainingStabilizers([1, 2])).toThrow(/four finite/);
    expect(() => traceTrainingStabilizers([1, 1, 3, 3], 0.5, [1, 0, 0, -1], [1, 2, 1, 0])).toThrow(/binary mask/);
    expect(() => traceTrainingStabilizers([1, 1, 3, 3], 0.5, [1, 0, 0, -1], [1, 0, 1, 0], 0)).toThrow(/probability/);
    expect(() => traceTrainingStabilizers([1, 1, 1, 1])).toThrow(/variance/);
  });

  it("switches deep-training routes and opens coupled coordinate arithmetic", () => {
    render(React.createElement(DeepTrainingWorkbench));
    fireEvent.click(screen.getByRole("button", { name: "Stabilizers" }));
    expect(screen.getByRole("heading", { name: "Normalization, dropout, and residual paths" })).toBeTruthy();
    const comparison = screen.getByLabelText("Training stabilizer route comparison");
    fireEvent.click(within(comparison).getByRole("button", { name: /Layer normalization/ }));
    expect(screen.getByLabelText("Selected stabilizer forward trace").textContent).toMatch(/mean1.*variance.*0\.25.*standard deviation0\.5.*normalized output.*-1.*-1.*1.*1/s);
    fireEvent.click(screen.getByRole("button", { name: "Open stabilizer coordinate 2" }));
    expect(screen.getByLabelText("Selected stabilizer coordinate calculation").textContent).toMatch(/4 × 0 - 0 - -1 × -2.*= -1.*0\.5 × -1 \+ 0 = -0\.5/s);
    fireEvent.click(within(comparison).getByRole("button", { name: /Inverted dropout/ }));
    expect(screen.getByLabelText("Selected stabilizer forward trace").textContent).toMatch(/binary mask.*1.*0.*1.*0.*evaluation.*0\.5.*0\.5.*1\.5.*1\.5.*expectation/s);
    fireEvent.click(within(comparison).getByRole("button", { name: /Identity residual/ }));
    expect(screen.getByLabelText("Selected stabilizer backward trace").textContent).toMatch(/through identity skip.*1.*0.*0.*-1.*total dS\/dinput.*1\.5.*0.*0.*-1\.5/s);
  });

  it("renders the stabilizer workbench directly", () => {
    render(React.createElement(TrainingStabilizersWorkbench));
    expect(screen.getByLabelText("Training stabilizer finite difference audit").textContent).toMatch(/0\.5.*0\.5.*6\.989e-11.*-2.*-2.*5\.351e-11/s);
  });
});

describe("tensor shapes and broadcasting", () => {
  it("keeps the tensor broadcasting microscope in the production stylesheet", () => {
    expect(productionCss).toContain(".workspace--tensor-broadcasting");
    expect(productionCss).toContain(".tensor-mapping-equation");
  });

  it("maps a column and row into one outer addition grid", () => {
    const trace = traceTensorBroadcasting("outer-grid");
    expect(trace.compatible).toBe(true);
    if (!trace.compatible) throw new Error("expected compatible trace");

    expect(trace.paddedLeftShape).toEqual([2, 1]);
    expect(trace.paddedRightShape).toEqual([1, 3]);
    expect(trace.outputShape).toEqual([2, 3]);
    expect(trace.outputValues).toEqual([11, 21, 31, 12, 22, 32]);
    expect(trace.leftExpandedAxes).toEqual([1]);
    expect(trace.rightExpandedAxes).toEqual([0]);
    expect(trace.mappings[4]).toMatchObject({
      outputIndex: [1, 1],
      leftIndex: [1, 0],
      rightIndex: [0, 1],
      leftValue: 2,
      rightValue: 20,
      outputValue: 22,
      upstream: 5,
    });
  });

  it("sums returning gradients over every expanded axis", () => {
    const outer = traceTensorBroadcasting("outer-grid");
    const row = traceTensorBroadcasting("row-over-batch");
    const scalar = traceTensorBroadcasting("scalar-over-matrix");
    if (!outer.compatible || !row.compatible || !scalar.compatible) {
      throw new Error("expected compatible traces");
    }

    expect(outer.leftGradient).toEqual([6, 15]);
    expect(outer.rightGradient).toEqual([5, 7, 9]);
    expect(row.paddedRightShape).toEqual([1, 3]);
    expect(row.rightGradient).toEqual([2, 2, 2]);
    expect(scalar.paddedLeftShape).toEqual([1, 1]);
    expect(scalar.leftGradient).toEqual([0]);
    expect(Math.max(
      outer.maxGradientAbsoluteError,
      row.maxGradientAbsoluteError,
      scalar.maxGradientAbsoluteError,
    )).toBeLessThan(1e-8);
  });

  it("rejects incompatible trailing dimensions before execution", () => {
    const trace = traceTensorBroadcasting("incompatible-tail");
    expect(trace).toMatchObject({
      compatible: false,
      paddedLeftShape: [2, 3],
      paddedRightShape: [1, 2],
      mismatchAxis: 1,
      leftDimension: 3,
      rightDimension: 2,
      error: "axis 1: dimensions 3 and 2 are incompatible",
    });
  });

  it("fails closed on malformed tensors upstream shapes and epsilon", () => {
    expect(() => traceBroadcastAdd(
      { shape: [2, 0], values: [] },
      { shape: [1], values: [1] },
      null,
    )).toThrow(/positive integers/);
    expect(() => traceBroadcastAdd(
      { shape: [2], values: [1, Number.NaN] },
      { shape: [2], values: [1, 2] },
      { shape: [2], values: [1, 1] },
    )).toThrow(/finite/);
    expect(() => traceBroadcastAdd(
      { shape: [2, 1], values: [1, 2] },
      { shape: [1, 3], values: [10, 20, 30] },
      { shape: [3, 2], values: [1, 2, 3, 4, 5, 6] },
    )).toThrow(/upstream shape/);
    expect(() => traceBroadcastAdd(
      { shape: [], values: [1] },
      { shape: [], values: [2] },
      { shape: [], values: [1] },
      0,
    )).toThrow(/epsilon/);
    expect(() => traceBroadcastAdd(
      { shape: [], values: { length: 1 } as unknown as number[] },
      { shape: [], values: [2] },
      { shape: [], values: [1] },
    )).toThrow(/shape and values arrays/);
    expect(() => traceBroadcastAdd(
      { shape: [], values: [1e308] },
      { shape: [], values: [1e308] },
      { shape: [], values: [1] },
    )).toThrow(/bounded/);
  });

  it("opens cells and switches through rank padding scalar and mismatch cases", () => {
    render(React.createElement(TensorBroadcastingWorkbench));
    expect(screen.getByRole("heading", { name: "Shape and broadcasting microscope" })).toBeTruthy();
    expect(screen.getByLabelText("Right aligned tensor shapes").textContent)
      .toMatch(/\[2, 1\].*\[1, 3\].*\[2, 3\].*2 ↔ 1.*1 ↔ 3/s);

    fireEvent.click(screen.getByRole("button", { name: /Open output \[1, 1\] value 22/ }));
    expect(screen.getByLabelText("Selected broadcast index calculation").textContent)
      .toMatch(/Output \[1, 1\].*left source.*\[1, 0\].*2.*right source.*\[0, 1\].*20.*output.*22.*upstream gradient 5/s);
    expect(screen.getByLabelText("Broadcast gradient reduction").textContent)
      .toMatch(/left gradient.*\[6, 15\].*right gradient.*\[5, 7, 9\].*maximum absolute error/s);

    fireEvent.click(screen.getByRole("button", { name: /Matrix \+ rank-one row/ }));
    expect(screen.getByLabelText("Right aligned tensor shapes").textContent)
      .toMatch(/\[2, 3\].*\[3\].*\[2, 3\]/s);
    expect(screen.getByLabelText("Broadcast gradient reduction").textContent)
      .toMatch(/right gradient.*\[2, 2, 2\]/s);

    fireEvent.click(screen.getByRole("button", { name: /Scalar \+ matrix/ }));
    expect(screen.getByLabelText("Right aligned tensor shapes").textContent)
      .toMatch(/\[\] scalar.*\[2, 2\].*\[2, 2\]/s);

    fireEvent.click(screen.getByRole("button", { name: /^Mismatch/ }));
    expect(screen.getByLabelText("Broadcast shape mismatch").textContent)
      .toMatch(/Axis 1 cannot broadcast.*3 is not 2.*neither dimension is 1/s);
  });
});

describe("dynamic autograd and saved values", () => {
  it("keeps the dynamic autograd microscope in the production stylesheet", () => {
    expect(productionCss).toContain(".workspace--dynamic-autograd");
    expect(productionCss).toContain(".autograd-backward-equations");
  });

  it("builds and reverses the complete scalar graph", () => {
    const trace = traceDynamicAutograd("multiply_add_square");
    expect(Object.fromEntries(trace.nodes.map((node) => [node.id, node.forwardValue]))).toEqual({
      x: 2, w: 3, b: 1, m: 6, z: 7, loss: 49,
    });
    expect(trace.topologicalOrder).toEqual(["x", "w", "m", "b", "z", "loss"]);
    expect(trace.backwardOrder).toEqual(["loss", "z", "b", "m", "w", "x"]);
    expect(trace.gradients).toMatchObject({ x: 42, w: 28, b: 14, m: 14, z: 14, loss: 1 });
    expect(trace.nodes.find((node) => node.id === "m")!.savedValues).toEqual([
      { name: "left", sourceId: "x", value: 2 },
      { name: "right", sourceId: "w", value: 3 },
    ]);
  });

  it("records only the operation chosen by runtime control flow", () => {
    const trace = traceDynamicAutograd("negative_branch");
    expect(trace.branchChoices).toEqual({ abs_x: "negative" });
    expect(trace.nodes.find((node) => node.id === "abs_x")!.operation).toBe("negate");
    expect(trace.nodes.some((node) => node.operation === "identity")).toBe(false);
    expect(trace.gradients.x).toBe(-4);
  });

  it("keeps a saved snapshot isolated from later live mutation", () => {
    const mutated = traceDynamicAutograd("saved_snapshot", true);
    const restored = traceDynamicAutograd("saved_snapshot", false);
    expect(mutated.liveInputValues).toMatchObject({ x: 2, w: 100 });
    expect(restored.liveInputValues).toMatchObject({ x: 2, w: 3 });
    expect(mutated.nodes.find((node) => node.id === "product")!.savedValues[1]).toEqual({
      name: "right", sourceId: "w", value: 3,
    });
    expect(mutated.gradients.x).toBe(3);
    expect(restored.gradients).toEqual(mutated.gradients);
  });

  it("checks every leaf gradient with fresh forward executions", () => {
    (["multiply_add_square", "negative_branch", "saved_snapshot"] as const).forEach((id) => {
      const trace = traceDynamicAutograd(id);
      expect(trace.maxGradientAbsoluteError).toBeLessThan(1e-8);
      expect(Object.values(trace.finiteDifferenceGradients).every(Number.isFinite)).toBe(true);
    });
  });

  it("handles prototype-like ids and the inclusive input boundary", () => {
    const trace = traceDynamicAutogradProgram({
      id: "saved_snapshot",
      title: "boundary",
      summary: "boundary",
      expression: "negative = -constructor",
      inputs: [{ id: "constructor", value: 1e6, requiresGradient: true }],
      steps: [{ id: "negative", operation: "negate", inputs: ["constructor"] }],
      output: "negative",
      mutationsAfterForward: {},
    });
    expect(trace.nodes[0]!.forwardValue).toBe(1e6);
    expect(trace.gradients.constructor).toBe(-1);
    expect(trace.finiteDifferenceGradients.constructor).toBeCloseTo(-1, 5);
  });

  it("snapshots and freezes caller-owned scenario data", () => {
    const input = { id: "x", value: 2, requiresGradient: true as const };
    const scenario: DynamicAutogradScenario = {
      id: "saved_snapshot",
      title: "snapshot",
      summary: "snapshot",
      expression: "negative = -x",
      inputs: [input],
      steps: [{ id: "negative", operation: "negate", inputs: ["x"] }],
      output: "negative",
      mutationsAfterForward: {},
    };
    const trace = traceDynamicAutogradProgram(scenario);
    input.value = 99;
    expect(trace.scenario.inputs[0]!.value).toBe(2);
    expect(Object.isFrozen(trace.scenario)).toBe(true);
    expect(Object.isFrozen(trace.scenario.inputs[0])).toBe(true);
  });

  it("fails closed on malformed graphs values epsilon and derived overflow", () => {
    const valid = traceDynamicAutograd("saved_snapshot").scenario;
    expect(() => traceDynamicAutogradProgram({
      ...valid,
      inputs: { length: 2 } as unknown as DynamicAutogradScenario["inputs"],
    })).toThrow(/bounded arrays/);
    expect(() => traceDynamicAutogradProgram({
      ...valid,
      inputs: [{ id: "x", value: Number.NaN, requiresGradient: true }],
    })).toThrow(/finite and bounded/);
    expect(() => traceDynamicAutogradProgram({
      ...valid,
      steps: [{ id: "product", operation: "multiply", inputs: ["x", "ghost"] }],
    })).toThrow(/must already exist/);
    expect(() => traceDynamicAutogradProgram(valid, 0)).toThrow(/epsilon/);

    const squareSteps = Array.from({ length: 12 }, (_, index) => ({
      id: `s${index}`,
      operation: "square" as const,
      inputs: [index === 0 ? "x" : `s${index - 1}`],
    }));
    expect(() => traceDynamicAutogradProgram({
      id: "saved_snapshot",
      title: "overflow",
      summary: "overflow",
      expression: "overflow",
      inputs: [{ id: "x", value: 1e6, requiresGradient: true }],
      steps: squareSteps,
      output: "s11",
      mutationsAfterForward: {},
    })).toThrow(/remain finite/);
  });

  it("opens nodes reverse steps branches and the mutation comparison", () => {
    render(React.createElement(DynamicAutogradWorkbench));
    expect(screen.getByRole("heading", { name: "Dynamic graph and saved-value microscope" })).toBeTruthy();
    expect(screen.getByLabelText("Executed dynamic computation graph").textContent)
      .toMatch(/x → w → m → b → z → loss.*m = 6.*loss = 49/s);

    fireEvent.click(screen.getByRole("button", { name: /Open node loss, square, value 49/ }));
    expect(screen.getByLabelText("Selected node forward and saved value trace").textContent)
      .toMatch(/loss = z².*value 49.*input ← z = 7/s);
    fireEvent.click(screen.getByRole("button", { name: /Open backward node m, upstream 14/ }));
    expect(screen.getByLabelText("Selected backward calculation").textContent)
      .toMatch(/toward x.*14 × 3 = 42.*saved:right.*toward w.*14 × 2 = 28.*saved:left/s);

    fireEvent.click(screen.getByRole("button", { name: /Runtime branch/ }));
    expect(screen.getByLabelText("Executed dynamic computation graph").textContent)
      .toMatch(/abs_x.*negate.*negative branch.*other operation is absent/s);

    fireEvent.click(screen.getByRole("button", { name: /Mutation snapshot/ }));
    expect(screen.getByLabelText("Selected node forward and saved value trace").textContent)
      .toMatch(/forward 3.*live 100.*saved forward snapshots/s);
    fireEvent.click(screen.getByRole("button", { name: "Restore forward-time live values" }));
    expect(screen.getByLabelText("Selected node forward and saved value trace").textContent)
      .toMatch(/forward 3.*live 3/s);
  });
});

describe("gradient accumulation and zeroing", () => {
  it("keeps the gradient buffer timeline in the production stylesheet", () => {
    expect(productionCss).toContain(".workspace--gradient-buffer");
    expect(productionCss).toContain(".gradient-buffer-event-lane");
  });

  it("adds two backward calls into one persistent buffer", () => {
    const trace = traceGradientAccumulation("accumulate_two_calls");
    expect(trace.steps.map((step) => [step.kind, step.bufferBefore, step.bufferAfter])).toEqual([
      ["backward", 0, 2],
      ["backward", 2, 4],
    ]);
    expect(trace.finalParameter).toBe(1);
    expect(trace.finalGradientBuffer).toBe(4);
  });

  it("makes zeroing an explicit state transition", () => {
    const trace = traceGradientAccumulation("zero_between_calls");
    expect(trace.steps.map((step) => step.kind)).toEqual([
      "backward", "zero_grad", "backward",
    ]);
    expect(trace.steps[1]).toMatchObject({
      parameterBefore: 1,
      parameterAfter: 1,
      bufferBefore: 2,
      bufferAfter: 0,
    });
    expect(trace.finalGradientBuffer).toBe(2);
  });

  it("shows that optimizer steps read but do not clear the buffer", () => {
    const clean = traceGradientAccumulation("mean_then_zero");
    const stale = traceGradientAccumulation("stale_next_batch");
    expect(clean.steps[2]).toMatchObject({
      kind: "optimizer_step",
      divisor: 2,
      appliedGradient: 2,
      parameterBefore: 1,
      parameterAfter: 0.8,
      bufferBefore: 4,
      bufferAfter: 4,
    });
    expect(clean.finalGradientBuffer).toBe(0);
    expect(stale.steps[3]).toMatchObject({
      kind: "backward",
      localGradient: 0.8,
      bufferBefore: 4,
      bufferAfter: 4.8,
    });
    expect(stale.finalParameter).toBeCloseTo(0.32, 12);
    expect(stale.finalGradientBuffer).toBeCloseTo(4.8, 12);
  });

  it("checks every backward event with fresh forward passes", () => {
    ([
      "accumulate_two_calls",
      "zero_between_calls",
      "mean_then_zero",
      "stale_next_batch",
    ] as const).forEach((id) => {
      const trace = traceGradientAccumulation(id);
      expect(trace.maxGradientAbsoluteError).toBeLessThan(1e-8);
      trace.steps.forEach((step) => {
        if (step.kind === "backward") {
          expect(step.numericalGradient).toBeCloseTo(step.localGradient, 8);
        }
      });
    });
  });

  it("fails closed on malformed schedules and snapshots caller data", () => {
    const sample = { id: "constructor", input: 1, target: 0 };
    const scenario: GradientAccumulationScenario = {
      id: "accumulate_two_calls",
      title: "prototype-like id",
      summary: "prototype-like id",
      initialParameter: 1,
      learningRate: 0.1,
      samples: [sample],
      events: [{ kind: "backward", sampleId: "constructor" }],
    };
    const trace = traceGradientAccumulationProgram(scenario);
    sample.input = 99;
    expect(trace.steps[0]).toMatchObject({ localGradient: 1, bufferAfter: 1 });
    expect(trace.scenario.samples[0]!.input).toBe(1);
    expect(Object.isFrozen(trace.scenario.samples[0])).toBe(true);

    expect(() => traceGradientAccumulationProgram({
      ...scenario,
      samples: { length: 1 } as unknown as GradientAccumulationScenario["samples"],
    })).toThrow(/arrays/);
    expect(() => traceGradientAccumulationProgram({
      ...scenario,
      samples: [{ id: "x", input: Number.NaN, target: 0 }],
      events: [{ kind: "backward", sampleId: "x" }],
    })).toThrow(/finite and bounded/);
    expect(() => traceGradientAccumulationProgram({
      ...scenario,
      events: [{ kind: "backward", sampleId: "ghost" }],
    })).toThrow(/unknown sample/);
    expect(() => traceGradientAccumulationProgram({
      ...scenario,
      events: [
        { kind: "backward", sampleId: "constructor" },
        { kind: "optimizer_step", divisor: 0 },
      ],
    })).toThrow(/divisor/);
    expect(() => traceGradientAccumulationProgram(scenario, 0)).toThrow(/epsilon/);

    const hostileId = {
      toString: () => { throw new Error("identifier was coerced"); },
    } as unknown as string;
    expect(() => traceGradientAccumulationProgram({
      ...scenario,
      samples: [{ id: hostileId, input: 1, target: 0 }],
      events: [{ kind: "backward", sampleId: "constructor" }],
    })).toThrow(/bounded identifier/);
    expect(() => traceGradientAccumulationProgram({
      ...scenario,
      events: [{ kind: "backward", sampleId: hostileId }],
    })).toThrow(/bounded identifier/);
    expect(() => traceGradientAccumulationProgram({
      ...scenario,
      title: [] as unknown as string,
    })).toThrow(/bounded strings/);
  });

  it("opens buffer events and compares clean with stale schedules", () => {
    render(React.createElement(GradientAccumulationWorkbench));
    expect(screen.getByRole("heading", { name: "Gradient buffer timeline" })).toBeTruthy();
    expect(screen.getByLabelText("Gradient schedule timeline").textContent)
      .toMatch(/backward\(a\).*grad 0 → 2.*backward\(b\).*grad 2 → 4/s);

    fireEvent.click(screen.getByRole("button", { name: /Open event 2, backward\(b\)/ }));
    expect(screen.getByLabelText("Selected gradient buffer calculation").textContent)
      .toMatch(/local gradient.*\(-1 - 1\) × -1.*dL\/dw = 2.*buffer addition.*2 \+ 2.*w.grad = 4/s);

    fireEvent.click(screen.getByRole("button", { name: /Mean, step, zero/ }));
    fireEvent.click(screen.getByRole("button", { name: /Open event 3, step\(grad \/ 2\)/ }));
    expect(screen.getByLabelText("Selected gradient buffer calculation").textContent)
      .toMatch(/4 \/ 2 = 2.*1 - 0.1 × 2.*w = 0.8.*left that buffer unchanged/s);

    fireEvent.click(screen.getByRole("button", { name: /Forgotten zero/ }));
    fireEvent.click(screen.getByRole("button", { name: /Open event 4, backward\(c\)/ }));
    expect(screen.getByLabelText("Selected gradient buffer calculation").textContent)
      .toMatch(/dL\/dw = 0.8.*4 \+ 0.8.*w.grad = 4.8/s);
  });
});

describe("forward graph lowering", () => {
  it("keeps the three-lane compiler map in the production stylesheet", () => {
    expect(productionCss).toContain(".workspace--forward-lowering");
    expect(productionCss).toContain(".forward-lowering-instruction-lane");
    expect(productionCss).toContain(".forward-lowering-matrix-lane");
  });

  it("emits the canonical twelve-instruction NeuralIR stream", () => {
    const trace = traceForwardLowering("single_row");
    expect(trace.graph.topologicalOrder).toEqual(["bias", "x0", "x1", "sum", "relu", "out"]);
    expect(trace.neuralIr.magic).toBe("CANN");
    expect(trace.neuralIr.instructions.map((instruction) => instruction.op)).toEqual([
      "LOAD_CONST",
      "LOAD_INPUT",
      "LOAD_INPUT",
      "LOAD_EDGE_WEIGHT",
      "MUL",
      "LOAD_EDGE_WEIGHT",
      "MUL",
      "LOAD_EDGE_WEIGHT",
      "MUL",
      "ADD",
      "ACTIVATE",
      "STORE_OUTPUT",
    ]);
    expect(trace.neuralValueRows[0]).toEqual([
      1, 4, 8, -1, -1, 0.25, 1, 0.75, 6, 6, 6,
    ]);
  });

  it("fuses seven scalar instructions into one weighted matrix operation", () => {
    const trace = traceForwardLowering("single_row");
    expect(trace.matrixIr.magic).toBe("CANM");
    expect(trace.matrixIr.operations.map((operation) => operation.op)).toEqual([
      "LOAD_CONST_MATRIX",
      "LOAD_INPUT_MATRIX",
      "LOAD_INPUT_MATRIX",
      "WEIGHTED_SUM_MATRIX",
      "ACTIVATE_MATRIX",
      "STORE_OUTPUT_MATRIX",
    ]);
    expect(trace.matrixIr.operations[3]).toMatchObject({
      inputs: ["v0", "v1", "v2"],
      attributes: {
        edge_ids: ["bias_to_sum", "w0", "w1"],
        weights: [-1, 0.25, 0.75],
      },
      sourceInstructions: ["i3", "i4", "i5", "i6", "i7", "i8", "i9"],
      sourceEdges: ["bias_to_sum", "w0", "w1"],
    });
  });

  it("keeps direct NeuralIR and MatrixIR execution in parity for both batches", () => {
    (["single_row", "two_row_batch"] as const).forEach((id) => {
      const trace = traceForwardLowering(id);
      expect(trace.directOutputs).toEqual(trace.neuralIrOutputs);
      expect(trace.neuralIrOutputs).toEqual(trace.matrixIrOutputs);
      expect(trace.maxParityError).toBe(0);
    });
    expect(traceForwardLowering("single_row").directOutputs).toEqual([6]);
    expect(traceForwardLowering("two_row_batch").directOutputs).toEqual([6, 13]);
  });

  it("fails closed on malformed inputs and snapshots caller data", () => {
    const x0 = [4];
    const scenario: ForwardLoweringScenario = {
      id: "custom",
      title: "custom",
      summary: "custom",
      inputs: { x0, x1: [8] },
    };
    const trace = traceForwardLoweringProgram(scenario);
    x0[0] = 999;
    expect(trace.scenario.inputs.x0).toEqual([4]);
    expect(Object.isFrozen(trace.scenario.inputs.x0)).toBe(true);
    expect(Object.isFrozen(trace.neuralIr.instructions[0])).toBe(true);

    expect(() => traceForwardLoweringProgram({
      ...scenario,
      inputs: { x0: [4], x1: [8, 16] },
    })).toThrow(/same bounded length/);
    expect(() => traceForwardLoweringProgram({
      ...scenario,
      inputs: { x0: [Number.NaN], x1: [8] },
    })).toThrow(/finite and bounded/);
    expect(() => traceForwardLoweringProgram({
      ...scenario,
      inputs: [] as unknown as ForwardLoweringScenario["inputs"],
    })).toThrow(/inputs must be an object/);
    expect(() => traceForwardLoweringProgram({
      ...scenario,
      inputs: { x0: [4], x1: [8], constructor: [0] } as unknown as ForwardLoweringScenario["inputs"],
    })).toThrow(/exactly x0 and x1/);

    const hostileId = {
      toString: () => { throw new Error("identifier was coerced"); },
    } as unknown as string;
    expect(() => traceForwardLoweringProgram({ ...scenario, id: hostileId }))
      .toThrow(/bounded string/);
  });

  it("opens scalar instructions fused operations and the two-row parity table", () => {
    render(React.createElement(ForwardLoweringWorkbench));
    expect(screen.getByRole("heading", { name: "Forward graph lowering map" })).toBeTruthy();
    expect(screen.getAllByRole("columnheader")).toHaveLength(6);
    expect(screen.getAllByRole("cell")).toHaveLength(6);
    expect(screen.getByLabelText("Forward lowering execution parity").textContent)
      .toMatch(/Three paths, the same prediction.*0.*4.*8.*6.*6.*6/s);

    fireEvent.click(screen.getByRole("button", { name: "Open NeuralIR i9, ADD" }));
    expect(screen.getByLabelText("Selected lowering detail").textContent)
      .toMatch(/ADD.*v4=-1, v6=1, v8=6.*v9=6.*sum/s);

    fireEvent.click(screen.getByRole("button", { name: "Open MatrixIR m3, WEIGHTED_SUM_MATRIX" }));
    expect(screen.getByLabelText("Selected lowering detail").textContent)
      .toMatch(/WEIGHTED_SUM_MATRIX.*i3, i4, i5, i6, i7, i8, i9.*bias_to_sum, w0, w1/s);

    fireEvent.click(screen.getByRole("button", { name: "The same plan, two rows" }));
    expect(screen.getAllByRole("cell")).toHaveLength(12);
    expect(screen.getByLabelText("Forward lowering execution parity").textContent)
      .toMatch(/1.*8.*16.*13.*13.*13/s);
  });
});

describe("backward and optimizer lowering", () => {
  it("pins separate backward optimizer and matrix training streams", () => {
    expect(compileBackwardTrainingIr().instructions.map((item) => item.op)).toEqual([
      "SEED_LOSS_GRAD",
      "HALF_SQUARED_ERROR_GRAD",
      "PROPAGATE_GRAD",
      "PARAMETER_LOCAL_GRAD",
      "ACCUMULATE_GRAD",
      "INPUT_GRAD",
    ]);
    expect(compileOptimizerTrainingIr().instructions.map((item) => item.op)).toEqual([
      "READ_GRAD_BUFFER",
      "DIVIDE_GRAD",
      "SGD_UPDATE",
      "KEEP_GRAD_BUFFER",
    ]);
    expect(compileMatrixTrainingIr().instructions.map((item) => item.op)).toEqual([
      "LOAD_SAVED_COLUMN",
      "LOAD_SAVED_COLUMN",
      "LOSS_GRAD_COLUMN",
      "PARAMETER_LOCAL_GRAD_COLUMN",
      "INPUT_GRAD_COLUMN",
      "REDUCE_SUM_GRAD",
      "ACCUMULATE_GRAD_BUFFER",
      "DIVIDE_GRAD",
      "SGD_UPDATE_SCALAR",
      "KEEP_GRAD_BUFFER",
    ]);
  });

  it("uses the production forward compiler before lowering training", () => {
    const trace = traceBackwardOptimizerLowering("one_row_by_hand");
    expect(trace.forward.neuralOps).toEqual([
      "LOAD_INPUT",
      "LOAD_EDGE_WEIGHT",
      "MUL",
      "ADD",
      "ACTIVATE",
      "STORE_OUTPUT",
    ]);
    expect(trace.forward.matrixOps).toEqual([
      "LOAD_INPUT_MATRIX",
      "WEIGHTED_SUM_MATRIX",
      "ACTIVATE_MATRIX",
      "STORE_OUTPUT_MATRIX",
    ]);
    expect(trace.forward.directOutputs).toEqual([1]);
    expect(trace.forward.neuralIrOutputs).toEqual([1]);
    expect(trace.forward.matrixIrOutputs).toEqual([1]);
    expect(trace.forward.maxError).toBe(0);
  });

  it("replays the one-row backward and SGD calculation", () => {
    const trace = traceBackwardOptimizerLowering("one_row_by_hand");
    expect(trace.savedValues).toEqual({
      x: [2], target: [0], prediction: [1], residual: [1], loss: [0.5],
    });
    expect(trace.backward).toEqual({
      dLoss: [1], dResidual: [1], dPrediction: [1], localDW: [2], dX: [0.5],
      gradientBufferBefore: 0, batchGradient: 2, gradW: 2,
    });
    expect(trace.optimizer.appliedGradient).toBe(2);
    expect(trace.optimizer.parameterAfter).toBeCloseTo(0.3);
    expect(trace.optimizer.gradientBufferAfterStep).toBe(2);
    expect(trace.maxPathError).toBe(0);
    expect(trace.gradientAudit.numerical).toBeCloseTo(2, 8);
  });

  it("keeps IDs fixed while two row gradients reduce and average", () => {
    const one = traceBackwardOptimizerLowering("one_row_by_hand");
    const two = traceBackwardOptimizerLowering("two_row_mean");
    expect(two.backwardIr.instructions.map((item) => item.id))
      .toEqual(one.backwardIr.instructions.map((item) => item.id));
    expect(two.optimizerIr.instructions.map((item) => item.id))
      .toEqual(one.optimizerIr.instructions.map((item) => item.id));
    expect(two.matrixTrainingIr.instructions.map((item) => item.id))
      .toEqual(one.matrixTrainingIr.instructions.map((item) => item.id));
    expect(two.backward.localDW).toEqual([2, 2]);
    expect(two.backward.gradW).toBe(4);
    expect(two.optimizer.appliedGradient).toBe(2);
    expect(two.optimizer.parameterAfter).toBeCloseTo(0.8);
    expect(two.matrixTraining.columns.dX).toEqual([1, -2]);
    expect(two.gradientAudit.numerical).toBeCloseTo(4, 8);
  });

  it("adds a new batch contribution to a persistent gradient buffer", () => {
    const trace = traceBackwardOptimizerLowering("persistent_buffer");
    expect(trace.backward.gradientBufferBefore).toBe(3);
    expect(trace.backward.batchGradient).toBe(2);
    expect(trace.backward.gradW).toBe(5);
    expect(trace.matrixTraining.batchGradient).toBe(2);
    expect(trace.matrixTraining.gradW).toBe(5);
    expect(trace.optimizer.parameterAfter).toBe(0);
    expect(trace.optimizer.gradientBufferAfterStep).toBe(5);
    expect(trace.gradientAudit.analytical).toBe(2);
    expect(trace.gradientAudit.numerical).toBeCloseTo(2, 8);
    expect(trace.maxPathError).toBe(0);
  });

  it("fails closed on hostile scenarios and snapshots caller arrays", () => {
    const inputs = [2];
    const scenario: BackwardOptimizerLoweringScenario = {
      id: "custom",
      title: "custom",
      summary: "custom",
      initialParameter: 0.5,
      learningRate: 0.1,
      inputs,
      targets: [0],
      gradientBufferBefore: 0,
      divisor: 1,
    };
    const trace = traceBackwardOptimizerLoweringProgram(scenario);
    inputs[0] = 999;
    expect(trace.scenario.inputs).toEqual([2]);
    expect(Object.isFrozen(trace.backwardIr.instructions[0])).toBe(true);

    expect(() => traceBackwardOptimizerLoweringProgram({ ...scenario, targets: [0, 1] }))
      .toThrow(/same bounded length/);
    expect(() => traceBackwardOptimizerLoweringProgram({ ...scenario, inputs: [Number.NaN] }))
      .toThrow(/finite and bounded/);
    expect(() => traceBackwardOptimizerLoweringProgram({ ...scenario, divisor: 2 }))
      .toThrow(/batch length/);
    expect(() => traceBackwardOptimizerLoweringProgram({
      ...scenario,
      constructor: "hostile",
    } as unknown as BackwardOptimizerLoweringScenario)).toThrow(/contain exactly/);
    const hostileId = { toString: () => { throw new Error("coerced"); } } as unknown as string;
    expect(() => traceBackwardOptimizerLoweringProgram({ ...scenario, id: hostileId }))
      .toThrow(/bounded string/);
  });

  it("opens every training lane and the responsive parity tables", () => {
    render(React.createElement(BackwardOptimizerLoweringWorkbench));
    expect(screen.getByRole("heading", { name: "Backward and optimizer lowering map" })).toBeTruthy();
    const savedTable = screen.getByRole("table", { name: "Saved forward row values" });
    const gradientTable = screen.getByRole("table", { name: "Backward row gradient values" });
    expect(within(savedTable).getAllByRole("columnheader")).toHaveLength(6);
    expect(within(savedTable).getAllByRole("cell")).toHaveLength(6);
    expect(within(gradientTable).getAllByRole("cell")).toHaveLength(6);

    fireEvent.click(screen.getByRole("button", { name: "Open Backward IR b3, PARAMETER_LOCAL_GRAD" }));
    expect(screen.getByLabelText("Selected training lowering detail").textContent)
      .toMatch(/PARAMETER_LOCAL_GRAD.*x, d_prediction.*local_d_w.*\[2\].*parameter_id=w/s);
    fireEvent.click(screen.getByRole("button", { name: "Open Optimizer IR o2, SGD_UPDATE" }));
    expect(screen.getByLabelText("Selected training lowering detail").textContent)
      .toMatch(/SGD_UPDATE.*w, applied_d_w.*w_next.*0\.3/s);
    fireEvent.click(screen.getByRole("button", { name: "Open Matrix training IR t5, REDUCE_SUM_GRAD" }));
    expect(screen.getByLabelText("Selected training lowering detail").textContent)
      .toMatch(/REDUCE_SUM_GRAD.*local_d_w_col.*batch_d_w.*2.*row_ascending/s);

    fireEvent.click(screen.getByRole("button", { name: "The same plan, two-row mean" }));
    expect(within(savedTable).getAllByRole("cell")).toHaveLength(12);
    expect(within(gradientTable).getAllByRole("cell")).toHaveLength(12);
    expect(screen.getByLabelText("Backward optimizer execution parity").textContent)
      .toMatch(/0 before \+ 2 \+ 2.*grad_w = 4.*4 \/ 2.*applied = 2.*w_next = 0\.8/s);

    fireEvent.click(screen.getByRole("button", { name: "Continue a persistent buffer" }));
    expect(screen.getByLabelText("Backward optimizer execution parity").textContent)
      .toMatch(/3 before \+ 2.*grad_w = 5.*5 \/ 1.*applied = 5.*w_next = 0/s);
  });
});
