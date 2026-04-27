import { penguinRegressionPoints, type PenguinMeasure } from "./data/palmer-penguins.js";
import { fitLinearClosedForm, type LossKind, type ModelState, type TrainingPoint } from "./training.js";

export type LabCategory =
  | "Basics"
  | "Learning rate"
  | "Loss functions"
  | "Scaling"
  | "Noise"
  | "Generalization"
  | "Real data";

export interface DataSource {
  name: string;
  kind: "synthetic" | "local-csv";
  license: string;
  url?: string;
}

export interface LabDefinition {
  id: string;
  title: string;
  category: LabCategory;
  summary: string;
  lesson: string;
  xLabel: string;
  yLabel: string;
  points: TrainingPoint[];
  defaultLoss: LossKind;
  defaultLearningRate: number;
  learningRateMin: number;
  learningRateMax: number;
  learningRateStep: number;
  initialModel: ModelState;
  idealModel: ModelState;
  source: DataSource;
}

const SYNTHETIC_SOURCE: DataSource = {
  name: "Generated in browser from deterministic formulas",
  kind: "synthetic",
  license: "Generated example data",
};

const PENGUIN_SOURCE: DataSource = {
  name: "Palmer Penguins sample",
  kind: "local-csv",
  license: "CC0 1.0 Universal",
  url: "https://github.com/allisonhorst/palmerpenguins",
};

const STANDARD_XS = [-8, -6, -4, -2, 0, 2, 4, 6, 8];
const WIDE_XS = [-40, -10, 0, 8, 15, 22, 38, 60, 100];
const UNIT_XS = [0, 0.12, 0.25, 0.38, 0.5, 0.62, 0.75, 0.88, 1];

interface SyntheticOptions {
  xs?: number[];
  noise?: number;
  seed?: number;
  outlierIndex?: number;
  outlierShift?: number;
  curve?: number;
}

function slug(value: string): string {
  return value.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "");
}

function makeSyntheticPoints(
  slope: number,
  bias: number,
  options: SyntheticOptions = {},
): TrainingPoint[] {
  const xs = options.xs ?? STANDARD_XS;
  const noise = options.noise ?? 0;
  const seed = options.seed ?? 1;
  const curve = options.curve ?? 0;

  return xs.map((x, index) => {
    const wobble = Math.sin((index + 1) * (seed + 1.7)) * noise;
    const outlier = index === options.outlierIndex ? options.outlierShift ?? 0 : 0;
    return {
      x,
      y: slope * x + bias + curve * x * x + wobble + outlier,
    };
  });
}

function makeSyntheticLab(args: {
  title: string;
  category: LabCategory;
  summary: string;
  lesson: string;
  xLabel?: string;
  yLabel?: string;
  points: TrainingPoint[];
  defaultLoss?: LossKind;
  defaultLearningRate?: number;
  initialModel?: ModelState;
  source?: DataSource;
}): LabDefinition {
  const idealModel = fitLinearClosedForm(args.points);
  const defaultLearningRate = args.defaultLearningRate ?? 0.01;

  return {
    id: slug(`${args.category}-${args.title}`),
    title: args.title,
    category: args.category,
    summary: args.summary,
    lesson: args.lesson,
    xLabel: args.xLabel ?? "Input",
    yLabel: args.yLabel ?? "Target",
    points: args.points,
    defaultLoss: args.defaultLoss ?? "mse",
    defaultLearningRate,
    learningRateMin: defaultLearningRate / 20,
    learningRateMax: defaultLearningRate * 40,
    learningRateStep: defaultLearningRate / 20,
    initialModel: args.initialModel ?? { weight: 0, bias: 0, epoch: 0 },
    idealModel,
    source: args.source ?? SYNTHETIC_SOURCE,
  };
}

const basics = [
  ["Celsius to Fahrenheit", "Exact unit conversion with a real slope and intercept.", 1.8, 32, WIDE_XS, 0.0005],
  ["Inches to centimeters", "A clean proportional relationship with almost no intercept.", 2.54, 0, STANDARD_XS, 0.01],
  ["Miles to kilometers", "Another unit conversion where the slope carries the lesson.", 1.609, 0, STANDARD_XS, 0.01],
  ["Hours to wages", "A wage model where the intercept acts like a fixed bonus.", 18, 40, STANDARD_XS, 0.002],
  ["Study time to quiz score", "A friendly positive trend with a meaningful baseline.", 6, 52, STANDARD_XS, 0.005],
  ["Screen brightness to battery draw", "A line where increasing input increases cost.", 0.42, 1.2, STANDARD_XS, 0.02],
  ["Discount to final price", "A negative slope: more discount means lower price.", -0.8, 100, STANDARD_XS, 0.004],
  ["Altitude to air temperature", "A negative physical trend with an intercept.", -3.5, 70, STANDARD_XS, 0.006],
  ["Recipe servings to flour", "A proportional recipe scaling example.", 120, 0, UNIT_XS, 0.02],
  ["Parking time to fee", "A simple line with a starting fee and per-hour growth.", 3.5, 4, STANDARD_XS, 0.012],
] as const;

const learningRates = Array.from({ length: 15 }, (_, index) => {
  const rate = [0.0002, 0.0005, 0.001, 0.002, 0.004][index % 5]!;
  const slope = 1.2 + (index % 3) * 0.45;
  return makeSyntheticLab({
    title: `Learning rate ${index + 1}: ${rate}`,
    category: "Learning rate",
    summary: "Compare how step size changes convergence speed and stability.",
    lesson: "A useful learning rate moves downhill visibly without bouncing across the valley.",
    points: makeSyntheticPoints(slope, 8 + index, { xs: WIDE_XS, noise: index % 2 === 0 ? 0 : 2, seed: index }),
    defaultLearningRate: rate,
  });
});

const lossLabs = Array.from({ length: 15 }, (_, index) => {
  const hasOutlier = index % 2 === 1;
  return makeSyntheticLab({
    title: `${hasOutlier ? "Outlier" : "Clean"} loss comparison ${index + 1}`,
    category: "Loss functions",
    summary: "Switch between MSE and MAE to see how error shape changes the update.",
    lesson: "MSE squares large mistakes, so a single bad point can pull the fitted line harder than MAE.",
    points: makeSyntheticPoints(2.4, 12, {
      xs: STANDARD_XS,
      noise: 0.8 + (index % 4) * 0.4,
      seed: index + 2,
      outlierIndex: hasOutlier ? 7 : undefined,
      outlierShift: hasOutlier ? 22 + index : 0,
    }),
    defaultLoss: hasOutlier ? "mae" : "mse",
    defaultLearningRate: 0.008,
  });
});

const scalingLabs = Array.from({ length: 15 }, (_, index) => {
  const xs = index % 3 === 0 ? UNIT_XS : index % 3 === 1 ? STANDARD_XS : WIDE_XS;
  const scaleName = index % 3 === 0 ? "normalized" : index % 3 === 1 ? "centered" : "wide";
  return makeSyntheticLab({
    title: `Feature scale ${index + 1}: ${scaleName}`,
    category: "Scaling",
    summary: "The same visual idea becomes easier or harder to optimize depending on input scale.",
    lesson: "Large input values make gradients large; normalized inputs usually tolerate larger learning rates.",
    points: makeSyntheticPoints(1.1 + index * 0.08, 4, { xs, noise: 0.5, seed: index + 4 }),
    defaultLearningRate: scaleName === "wide" ? 0.0006 : 0.015,
  });
});

const noiseLabs = Array.from({ length: 15 }, (_, index) => {
  const noise = 0.5 + index * 0.5;
  return makeSyntheticLab({
    title: `Noise level ${index + 1}`,
    category: "Noise",
    summary: "Watch the line chase a pattern when the points stop landing exactly on it.",
    lesson: "Noise means the best line is not the line through every point; it is the line that balances errors.",
    points: makeSyntheticPoints(3.1, -6, { noise, seed: index + 6 }),
    defaultLearningRate: 0.007,
  });
});

const generalizationLabs = Array.from({ length: 12 }, (_, index) => {
  const curved = index % 2 === 0;
  return makeSyntheticLab({
    title: `${curved ? "Curved data" : "Sparse data"} ${index + 1}`,
    category: "Generalization",
    summary: "Use a line even when the world is not perfectly linear.",
    lesson: curved
      ? "A linear model can still be useful on curved data, but the residuals reveal its limits."
      : "With only a few points, the line can look confident while still being fragile.",
    points: makeSyntheticPoints(1.7, 5, {
      xs: curved ? STANDARD_XS : [-8, -2, 1, 7],
      noise: 0.7,
      seed: index + 8,
      curve: curved ? 0.12 + index * 0.01 : 0,
    }),
    defaultLearningRate: 0.007,
  });
});

const penguinPairs: Array<[PenguinMeasure, PenguinMeasure, string, string]> = [
  ["flipper_length_mm", "body_mass_g", "Flipper length to body mass", "Longer flippers usually come with larger body mass."],
  ["bill_length_mm", "body_mass_g", "Bill length to body mass", "Bill length has signal, but the relationship is messier."],
  ["bill_depth_mm", "body_mass_g", "Bill depth to body mass", "A weak feature shows why not every measurement predicts well."],
  ["flipper_length_mm", "bill_length_mm", "Flipper length to bill length", "A moderate trend shows shared body-size information."],
  ["bill_length_mm", "bill_depth_mm", "Bill length to bill depth", "This relationship is noisy because species mix differently."],
  ["year", "body_mass_g", "Observation year to body mass", "A poor predictor is useful because the loss does not improve much."],
];

const penguinLabs = penguinPairs.flatMap(([xKey, yKey, title, lesson]) =>
  ["MSE view", "MAE view", "small learning rate"].map((mode, modeIndex) =>
    makeSyntheticLab({
      title: `${title}: ${mode}`,
      category: "Real data",
      summary: "A checked-in CC0 CSV sample from Palmer Penguins, used without runtime network loading.",
      lesson,
      xLabel: xKey.replaceAll("_", " "),
      yLabel: yKey.replaceAll("_", " "),
      points: penguinRegressionPoints(xKey, yKey),
      defaultLoss: modeIndex === 1 ? "mae" : "mse",
      defaultLearningRate: modeIndex === 2 ? 0.0000004 : 0.000001,
      initialModel: { weight: 0, bias: 3000, epoch: 0 },
      source: PENGUIN_SOURCE,
    }),
  ),
);

export const LABS: LabDefinition[] = [
  ...basics.map(([title, summary, slope, bias, xs, rate]) =>
    makeSyntheticLab({
      title,
      category: "Basics",
      summary,
      lesson: "Start with simple lines so weight, bias, prediction, error, and loss become visible.",
      xLabel: title === "Celsius to Fahrenheit" ? "Celsius" : "Input",
      yLabel: title === "Celsius to Fahrenheit" ? "Fahrenheit" : "Target",
      points: makeSyntheticPoints(slope, bias, { xs }),
      defaultLearningRate: rate,
      initialModel: title === "Celsius to Fahrenheit" ? { weight: 0.5, bias: 0.5, epoch: 0 } : undefined,
    }),
  ),
  ...learningRates,
  ...lossLabs,
  ...scalingLabs,
  ...noiseLabs,
  ...generalizationLabs,
  ...penguinLabs,
];

export const LAB_CATEGORIES: LabCategory[] = [
  "Basics",
  "Learning rate",
  "Loss functions",
  "Scaling",
  "Noise",
  "Generalization",
  "Real data",
];
