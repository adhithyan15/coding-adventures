import { readFileSync } from "node:fs";
import { join } from "node:path";
import { fitStandardScaler, transformStandard, type Matrix as MatrixData } from "coding-adventures-feature-normalization/src/index";
import {
  SingleLayerNetwork,
  trainOneEpochWithMatrices,
  type MatrixTrainingStep,
} from "coding-adventures-single-layer-network/src/index";

export const VERSION = "0.1.0";

export interface EnergyEfficiencyRow {
  features: number[];
  targets: number[];
}

export interface PreparedEnergyData {
  rawFeatures: MatrixData;
  rawTargets: MatrixData;
  normalizedFeatures: MatrixData;
  normalizedTargets: MatrixData;
  targetScaler: { means: number[]; standardDeviations: number[] };
}

const FEATURE_NAMES = [
  "relativeCompactness",
  "surfaceArea",
  "wallArea",
  "roofArea",
  "overallHeight",
  "orientation",
  "glazingArea",
  "glazingAreaDistribution",
];

const TARGET_NAMES = ["heatingLoad", "coolingLoad"];

function parseCsvLine(line: string): string[] {
  return line.split(",").map(value => value.trim());
}

export function loadEnergyEfficiencyRows(csvPath = join(__dirname, "data", "energy-efficiency.csv")): EnergyEfficiencyRow[] {
  const text = readFileSync(csvPath, "utf8").trim();
  const lines = text.split(/\r?\n/);
  const header = parseCsvLine(lines[0]);
  if (header.length !== 10 || header[0] !== "X1" || header[8] !== "Y1") {
    throw new Error("unexpected energy-efficiency CSV header");
  }

  return lines.slice(1).map(line => {
    const values = parseCsvLine(line).map(Number);
    return {
      features: values.slice(0, 8),
      targets: values.slice(8, 10),
    };
  });
}

export function prepareEnergyData(rows: EnergyEfficiencyRow[]): PreparedEnergyData {
  const rawFeatures = rows.map(row => row.features);
  const rawTargets = rows.map(row => row.targets);
  const featureScaler = fitStandardScaler(rawFeatures);
  const targetScaler = fitStandardScaler(rawTargets);

  return {
    rawFeatures,
    rawTargets,
    normalizedFeatures: transformStandard(rawFeatures, featureScaler),
    normalizedTargets: transformStandard(rawTargets, targetScaler),
    targetScaler,
  };
}

export function inverseStandardRow(row: number[], scaler: { means: number[]; standardDeviations: number[] }): number[] {
  return row.map((value, col) => (value * scaler.standardDeviations[col]) + scaler.means[col]);
}

function formatRow(row: number[], digits = 3): string {
  return `[${row.map(value => value.toFixed(digits)).join(", ")}]`;
}

function runExplicitMatrixMathDemo(data: PreparedEnergyData): void {
  console.log("\n--- Version 1: explicit matrix math training ---");
  console.log(`X shape: ${data.normalizedFeatures.length} x ${data.normalizedFeatures[0].length}`);
  console.log(`Y shape: ${data.normalizedTargets.length} x ${data.normalizedTargets[0].length}`);
  console.log(`W shape: ${data.normalizedFeatures[0].length} x ${data.normalizedTargets[0].length}`);
  console.log(`b shape: ${data.normalizedTargets[0].length}`);

  let weights: MatrixData = Array.from({ length: data.normalizedFeatures[0].length }, () => (
    Array(data.normalizedTargets[0].length).fill(0)
  ));
  let biases = Array(data.normalizedTargets[0].length).fill(0);
  let finalStep: MatrixTrainingStep | null = null;
  const learningRate = 0.03;

  for (let epoch = 0; epoch <= 2000; epoch++) {
    finalStep = trainOneEpochWithMatrices(
      data.normalizedFeatures,
      data.normalizedTargets,
      weights,
      biases,
      learningRate,
      "linear",
    );
    weights = finalStep.nextWeights;
    biases = finalStep.nextBiases;

    if (epoch % 400 === 0) {
      console.log(
        `Epoch ${String(epoch).padStart(3, " ")} | normalized MSE ${finalStep.loss.toFixed(5)} | ` +
        `dW shape ${finalStep.weightGradients.length} x ${finalStep.weightGradients[0].length}`,
      );
    }
  }

  if (finalStep === null) {
    throw new Error("training did not run");
  }

  console.log("\nOne sample after explicit matrix training:");
  const predictedLoads = inverseStandardRow(finalStep.predictions[0], data.targetScaler);
  console.log(`  inputs: ${FEATURE_NAMES.map((name, index) => `${name}=${data.rawFeatures[0][index]}`).join(", ")}`);
  console.log(`  actual [${TARGET_NAMES.join(", ")}]: ${formatRow(data.rawTargets[0], 2)}`);
  console.log(`  predicted: ${formatRow(predictedLoads, 2)}`);
  console.log(`  first-row normalized error: ${formatRow(finalStep.errors[0])}`);
}

function runFitPredictDemo(data: PreparedEnergyData): void {
  console.log("\n--- Version 2: fit/predict interface ---");

  const model = new SingleLayerNetwork({ activation: "linear", learningRate: 0.03 });
  const history = model.fit(data.normalizedFeatures, data.normalizedTargets, {
    epochs: 2000,
    logEvery: 400,
    onEpoch(snapshot) {
      console.log(`Epoch ${String(snapshot.epoch).padStart(3, " ")} | normalized MSE ${snapshot.loss.toFixed(5)}`);
    },
  });

  const predictions = model.predict(data.normalizedFeatures.slice(0, 5));
  console.log("\nFirst five predictions in original units:");
  predictions.forEach((prediction, index) => {
    const denormalizedPrediction = inverseStandardRow(prediction, data.targetScaler);
    console.log(
      `  sample ${index + 1}: actual=${formatRow(data.rawTargets[index], 2)} ` +
      `predicted=${formatRow(denormalizedPrediction, 2)}`,
    );
  });

  const last = history.at(-1);
  if (last) {
    console.log(`\nLearned W matrix shape: ${last.weights.length} x ${last.weights[0].length}`);
    console.log(`Learned bias vector: ${formatRow(last.biases)}`);
  }
}

export function runEnergyEfficiencyDemo(): void {
  const rows = loadEnergyEfficiencyRows();
  const data = prepareEnergyData(rows);

  console.log("\n=== Energy Efficiency Multi-Output Predictor ===");
  console.log("Dataset: UCI Energy Efficiency, 768 rows, 8 inputs, 2 outputs.");
  console.log("Outputs: heating load and cooling load.");

  runExplicitMatrixMathDemo(data);
  runFitPredictDemo(data);
}

if (require.main === module) {
  runEnergyEfficiencyDemo();
}
