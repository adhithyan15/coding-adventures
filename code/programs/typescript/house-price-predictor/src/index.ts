/**
 * Multi-Variable Linear Regression: House Price Predictor
 * -------------------------------------------------------
 * Demonstrates n input features -> one output with feature normalization and
 * a short learning-rate sweep before the full training run.
 */
import { Matrix } from "../../../../packages/typescript/matrix/src/matrix";
import { mse as meanSquaredError } from "../../../../packages/typescript/loss-functions/src/loss_functions";
import {
  fitStandardScaler,
  transformStandard,
  type Matrix as FeatureMatrix,
} from "coding-adventures-feature-normalization/src/index";

const houseFeaturesData: FeatureMatrix = [
  [2000.0, 3.0],
  [1500.0, 2.0],
  [2500.0, 4.0],
  [1000.0, 1.0],
];

const truePricesData = [
  [400.0],
  [300.0],
  [500.0],
  [200.0],
];

interface TrainingResult {
  learningRate: number;
  loss: number;
  diverged: boolean;
  weights: Matrix;
  bias: number;
}

function runTraining(
  featuresData: FeatureMatrix,
  pricesData: number[][],
  learningRate: number,
  epochs: number,
  logEvery?: number,
): TrainingResult {
  const houseFeatures = new Matrix(featuresData);
  const truePrices = new Matrix(pricesData);
  let featureWeights = new Matrix([[0.5], [0.5]]);
  let basePriceBias = 0.0;
  let lastLoss = Number.POSITIVE_INFINITY;

  for (let epoch = 0; epoch <= epochs; epoch++) {
    const finalPredictions = houseFeatures.dot(featureWeights).add(basePriceBias);
    const linearTruePrices = truePrices.data.map((row: number[]) => row[0]);
    const linearPredictions = finalPredictions.data.map((row: number[]) => row[0]);
    lastLoss = meanSquaredError(linearTruePrices, linearPredictions);

    if (!Number.isFinite(lastLoss) || lastLoss > 1.0e12) {
      return { learningRate, loss: Number.POSITIVE_INFINITY, diverged: true, weights: featureWeights, bias: basePriceBias };
    }

    if (logEvery !== undefined && epoch % logEvery === 0) {
      console.log(
        `Epoch ${epoch.toString().padStart(4, " ")} | Loss: ${lastLoss.toFixed(4).padStart(10, " ")} | ` +
        `Weights [SqFt: ${featureWeights.data[0][0].toFixed(3).padStart(7, " ")}, ` +
        `Beds: ${featureWeights.data[1][0].toFixed(3).padStart(7, " ")}] | Bias: ${basePriceBias.toFixed(3).padStart(7, " ")}`,
      );
    }

    const predictionErrors = finalPredictions.subtract(truePrices);
    const weightGradients = houseFeatures.transpose().dot(predictionErrors).scale(2.0 / truePrices.rows);

    let biasGradientTotal = 0.0;
    for (let i = 0; i < predictionErrors.rows; i++) {
      biasGradientTotal += predictionErrors.data[i][0];
    }
    const biasGradient = biasGradientTotal * (2.0 / truePrices.rows);

    featureWeights = featureWeights.subtract(weightGradients.scale(learningRate));
    basePriceBias -= biasGradient * learningRate;
  }

  return { learningRate, loss: lastLoss, diverged: false, weights: featureWeights, bias: basePriceBias };
}

function findLearningRate(featuresData: FeatureMatrix, pricesData: number[][]): TrainingResult {
  const candidates = [0.001, 0.003, 0.01, 0.03, 0.1, 0.3, 0.6];
  const results = candidates.map(learningRate => runTraining(featuresData, pricesData, learningRate, 120));

  console.log("\nShort learning-rate sweep over normalized features:");
  for (const result of results) {
    const lossText = result.diverged ? "diverged" : result.loss.toFixed(4);
    console.log(`  lr=${String(result.learningRate).padEnd(6, " ")} -> loss=${lossText}`);
  }

  return results
    .filter(result => !result.diverged)
    .sort((a, b) => a.loss - b.loss)[0];
}

console.log("\n--- Booting Multi-Variable Predictor: House Prices ---");
console.log("Features: square footage and bedroom count. Target: price in $1000s.");

const scaler = fitStandardScaler(houseFeaturesData);
const normalizedFeatures = transformStandard(houseFeaturesData, scaler);
const bestTrial = findLearningRate(normalizedFeatures, truePricesData);

console.log(`\nSelected learning rate: ${bestTrial.learningRate}`);
console.log("Beginning full training run...");
const finalResult = runTraining(normalizedFeatures, truePricesData, bestTrial.learningRate, 1500, 150);

console.log("\nFinal Optimal Mapping Achieved!");
const normalizedTestHouse = transformStandard([[2000.0, 3.0]], scaler);
const finalPrediction = new Matrix(normalizedTestHouse).dot(finalResult.weights).add(finalResult.bias).data[0][0];
console.log(`Prediction for House 1 (Target $400k): $${finalPrediction.toFixed(2)}k`);
