import {
  createSeededParameters,
  trainOneEpochTwoLayer,
  traceExampleTwoLayer,
  type ExampleTrace,
  type MatrixData,
  type TwoLayerParameters,
  type TwoLayerTrainingStep,
} from "coding-adventures-two-layer-network/src/index";
import { predictTwoLayerWithVm } from "./neural-vm.js";

export type HiddenLayerChartKind = "curve" | "surface" | "table";

export interface HiddenLayerExampleRow {
  input: number[];
  target: number;
  label: string;
  group?: string;
}

export interface HiddenLayerExample {
  id: string;
  title: string;
  category: string;
  summary: string;
  lesson: string;
  inputLabels: string[];
  outputLabel: string;
  rows: HiddenLayerExampleRow[];
  hiddenCount: number;
  initialScale: number;
  seed: number;
  defaultLearningRate: number;
  learningRateMin: number;
  learningRateMax: number;
  learningRateStep: number;
  chartKind: HiddenLayerChartKind;
}

export interface HiddenLayerModelState {
  parameters: TwoLayerParameters;
  epoch: number;
}

export interface HiddenLayerStepResult {
  state: HiddenLayerModelState;
  step: TwoLayerTrainingStep;
  loss: number;
  mae: number;
}

export interface HiddenLayerHistoryPoint {
  epoch: number;
  loss: number;
  mae: number;
}

function makeExample(args: Omit<HiddenLayerExample, "learningRateMin" | "learningRateMax" | "learningRateStep">): HiddenLayerExample {
  return {
    ...args,
    learningRateMin: args.defaultLearningRate / 20,
    learningRateMax: args.defaultLearningRate * 8,
    learningRateStep: args.defaultLearningRate / 20,
  };
}

function targetRows(example: HiddenLayerExample): MatrixData {
  return example.rows.map((row) => [row.target]);
}

export function exampleInputs(example: HiddenLayerExample): MatrixData {
  return example.rows.map((row) => row.input);
}

export function exampleTargets(example: HiddenLayerExample): MatrixData {
  return targetRows(example);
}

export function createInitialHiddenState(example: HiddenLayerExample): HiddenLayerModelState {
  return {
    epoch: 0,
    parameters: createSeededParameters(
      example.inputLabels.length,
      example.hiddenCount,
      1,
      example.seed,
      example.initialScale,
    ),
  };
}

export function predictHidden(example: HiddenLayerExample, state: HiddenLayerModelState): number[] {
  return predictTwoLayerWithVm(exampleInputs(example), state.parameters, {
    inputNames: example.inputLabels,
    outputNames: [example.outputLabel],
  }).predictions.map((row) => row[0]!);
}

export function hiddenLoss(example: HiddenLayerExample, state: HiddenLayerModelState): number {
  const predictions = predictHidden(example, state);
  return predictions.reduce((sum, prediction, index) => {
    const error = prediction - example.rows[index]!.target;
    return sum + error * error;
  }, 0) / predictions.length;
}

export function hiddenMeanAbsoluteError(example: HiddenLayerExample, state: HiddenLayerModelState): number {
  const predictions = predictHidden(example, state);
  return predictions.reduce((sum, prediction, index) => {
    return sum + Math.abs(prediction - example.rows[index]!.target);
  }, 0) / predictions.length;
}

export function hiddenHistoryPoint(example: HiddenLayerExample, state: HiddenLayerModelState): HiddenLayerHistoryPoint {
  return {
    epoch: state.epoch,
    loss: hiddenLoss(example, state),
    mae: hiddenMeanAbsoluteError(example, state),
  };
}

export function trainHiddenStep(
  example: HiddenLayerExample,
  state: HiddenLayerModelState,
  learningRate: number,
): HiddenLayerStepResult {
  const step = trainOneEpochTwoLayer(
    exampleInputs(example),
    targetRows(example),
    state.parameters,
    learningRate,
  );
  const nextState = {
    epoch: state.epoch + 1,
    parameters: step.nextParameters,
  };

  return {
    state: nextState,
    step,
    loss: hiddenLoss(example, nextState),
    mae: hiddenMeanAbsoluteError(example, nextState),
  };
}

export function trainHiddenSteps(
  example: HiddenLayerExample,
  state: HiddenLayerModelState,
  learningRate: number,
  count: number,
): HiddenLayerStepResult[] {
  const results: HiddenLayerStepResult[] = [];
  let current = state;
  for (let index = 0; index < count; index += 1) {
    const result = trainHiddenStep(example, current, learningRate);
    results.push(result);
    current = result.state;
  }
  return results;
}

export function traceHiddenExample(
  example: HiddenLayerExample,
  state: HiddenLayerModelState,
  rowIndex: number,
): ExampleTrace {
  return {
    ...traceExampleTwoLayer(
      [example.rows[rowIndex]!.input],
      state.parameters,
      0,
      [example.rows[rowIndex]!.target],
    ),
    exampleIndex: rowIndex,
  };
}

const circleRows: HiddenLayerExampleRow[] = [];
for (const x of [-1, -0.5, 0, 0.5, 1]) {
  for (const y of [-1, -0.5, 0, 0.5, 1]) {
    circleRows.push({
      input: [x, y],
      target: x * x + y * y <= 0.55 ? 1 : 0,
      label: `(${x}, ${y})`,
      group: x * x + y * y <= 0.55 ? "inside" : "outside",
    });
  }
}

const moonRows: HiddenLayerExampleRow[] = [];
for (let index = 0; index < 12; index += 1) {
  const angle = (Math.PI * index) / 11;
  moonRows.push({
    input: [Math.cos(angle), Math.sin(angle)],
    target: 0,
    label: `upper ${index + 1}`,
    group: "upper",
  });
  moonRows.push({
    input: [1 - Math.cos(angle), 0.5 - Math.sin(angle)],
    target: 1,
    label: `lower ${index + 1}`,
    group: "lower",
  });
}

export const HIDDEN_LAYER_EXAMPLES: HiddenLayerExample[] = [
  makeExample({
    id: "xnor",
    title: "XNOR Gate",
    category: "Logic",
    summary: "Outputs 1 when the two inputs match and 0 when they differ.",
    lesson: "The hidden layer learns two useful regions: both inputs off and both inputs on. The output neuron combines those regions into one decision.",
    inputLabels: ["A", "B"],
    outputLabel: "same?",
    rows: [
      { input: [0, 0], target: 1, label: "A=0, B=0", group: "same" },
      { input: [0, 1], target: 0, label: "A=0, B=1", group: "different" },
      { input: [1, 0], target: 0, label: "A=1, B=0", group: "different" },
      { input: [1, 1], target: 1, label: "A=1, B=1", group: "same" },
    ],
    hiddenCount: 3,
    initialScale: 2,
    seed: 31,
    defaultLearningRate: 1.4,
    chartKind: "surface",
  }),
  makeExample({
    id: "absolute-value",
    title: "Absolute Value",
    category: "Regression",
    summary: "Learns the V-shaped relationship y = |x| on normalized inputs.",
    lesson: "A single line cannot bend at zero. Hidden neurons can split the input range into left and right regions, then recombine them into a V.",
    inputLabels: ["x"],
    outputLabel: "|x|",
    rows: [-1, -0.75, -0.5, -0.25, 0, 0.25, 0.5, 0.75, 1].map((x) => ({
      input: [x],
      target: Math.abs(x),
      label: `x=${x}`,
    })),
    hiddenCount: 6,
    initialScale: 3,
    seed: 12,
    defaultLearningRate: 1.8,
    chartKind: "curve",
  }),
  makeExample({
    id: "piecewise-pricing",
    title: "Piecewise Pricing",
    category: "Regression",
    summary: "Approximates a stepped shipping-price schedule from package weight.",
    lesson: "Hidden neurons can behave like soft thresholds. Several thresholds together make a stair-step curve.",
    inputLabels: ["weight"],
    outputLabel: "price tier",
    rows: [
      [0.05, 0.12],
      [0.15, 0.12],
      [0.25, 0.25],
      [0.35, 0.25],
      [0.45, 0.55],
      [0.55, 0.55],
      [0.7, 0.88],
      [0.85, 0.88],
      [1, 0.88],
    ].map(([x, y]) => ({
      input: [x],
      target: y,
      label: `${Math.round(x * 40)} lb`,
    })),
    hiddenCount: 6,
    initialScale: 3,
    seed: 19,
    defaultLearningRate: 2,
    chartKind: "curve",
  }),
  makeExample({
    id: "circle-classifier",
    title: "Circle Classifier",
    category: "Classification",
    summary: "Classifies whether a point is inside a circle.",
    lesson: "The hidden layer combines several soft boundaries. Together they can carve out a round-ish region even though each neuron is simple.",
    inputLabels: ["x", "y"],
    outputLabel: "inside?",
    rows: circleRows,
    hiddenCount: 8,
    initialScale: 3,
    seed: 37,
    defaultLearningRate: 2.2,
    chartKind: "surface",
  }),
  makeExample({
    id: "two-moons",
    title: "Two Moons",
    category: "Classification",
    summary: "Separates two curved bands that no single straight boundary can split.",
    lesson: "The hidden layer remaps curved geometry into features the output neuron can combine into a useful decision.",
    inputLabels: ["x", "y"],
    outputLabel: "moon",
    rows: moonRows,
    hiddenCount: 10,
    initialScale: 3,
    seed: 43,
    defaultLearningRate: 1.8,
    chartKind: "surface",
  }),
  makeExample({
    id: "interaction-features",
    title: "Interaction Features",
    category: "Tabular",
    summary: "Predicts a normalized house-value score from bedrooms, bathrooms, and garage.",
    lesson: "The hidden layer can learn combinations, like garage plus enough rooms, instead of treating each input as a separate straight-line effect.",
    inputLabels: ["bedrooms", "bathrooms", "garage"],
    outputLabel: "value score",
    rows: [
      { input: [0.2, 0.25, 0], target: 0.08, label: "1 bed, 1 bath, no garage" },
      { input: [0.4, 0.25, 0], target: 0.18, label: "2 bed, 1 bath, no garage" },
      { input: [0.4, 0.5, 0], target: 0.32, label: "2 bed, 2 bath, no garage" },
      { input: [0.6, 0.5, 0], target: 0.45, label: "3 bed, 2 bath, no garage" },
      { input: [0.6, 0.5, 1], target: 0.72, label: "3 bed, 2 bath, garage" },
      { input: [0.8, 0.5, 0], target: 0.58, label: "4 bed, 2 bath, no garage" },
      { input: [0.8, 0.75, 1], target: 0.9, label: "4 bed, 3 bath, garage" },
      { input: [1, 0.75, 1], target: 0.96, label: "5 bed, 3 bath, garage" },
      { input: [1, 1, 0], target: 0.76, label: "5 bed, 4 bath, no garage" },
      { input: [0.2, 0.5, 1], target: 0.35, label: "1 bed, 2 bath, garage" },
    ],
    hiddenCount: 7,
    initialScale: 3,
    seed: 51,
    defaultLearningRate: 1.8,
    chartKind: "table",
  }),
];
