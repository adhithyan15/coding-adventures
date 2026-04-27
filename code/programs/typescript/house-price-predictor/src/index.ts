/**
 * Multi-Variable Linear Regression: House Price Predictor
 * -------------------------------------------------------
 * Utilizing explicit object-oriented structural patterns mapped directly
 * using hyper-descriptive native variable names indicating physical meaning.
 */
import { Matrix } from "../../../../packages/typescript/matrix/src/matrix";
import { mse as meanSquaredError } from "../../../../packages/typescript/loss-functions/src/loss_functions";

console.log("\n--- Booting Multi-Variable Predictor: House Prices ---\n");

// 1. The Dataset Layout (Inputs)
const houseFeatures = new Matrix([
  [2.0, 3.0],
  [1.5, 2.0],
  [2.5, 4.0],
  [1.0, 1.0]
]);

// 2. The Target Labels (Outputs)
const truePrices = new Matrix([
  [400.0],
  [300.0],
  [500.0],
  [200.0]
]);

// 3. Mathematical Node Layout Parameters
let featureWeights = new Matrix([[0.5], [0.5]]);
let basePriceBias = 0.5;
const learningRate = 0.01;

console.log("Beginning Training Epochs...");
for (let epoch = 0; epoch <= 1500; epoch++) {
  
  // --- INVERSE FORWARD MAPPING ---
  const rawPredictions = houseFeatures.dot(featureWeights);
  const finalPredictions = rawPredictions.add(basePriceBias);

  const linearTruePrices = truePrices.data.map((r: number[]) => r[0]);
  const linearPredictions = finalPredictions.data.map((r: number[]) => r[0]);
  const mseLoss = meanSquaredError(linearTruePrices, linearPredictions);

  // --- BACKPROPAGATION (CALCULATING GRADIENTS) ---
  // How do we figure out exactly how much the SqFt Weight vs Bedroom Weight was responsible for the error?
  // 1. We take our original (N BY 2) Data Grid and physically flip it on its side to become (2 BY N). 
  //    - Row 1 now contains only SqFt values. Row 2 contains only Bedroom values.
  // 2. We Dot Product this (2 BY N) grid against our (N BY 1) Error Vector!
  const predictionErrors = finalPredictions.subtract(truePrices);
  const transposedFeatures = houseFeatures.transpose();
  const featuresDotErrors = transposedFeatures.dot(predictionErrors);
  
  // We multiply by (2 / N) because of the Mean Squared Error derivative scaling.
  const weightGradients = featuresDotErrors.scale(2.0 / truePrices.rows);

  // For the Bias, because it shifts the prediction unconditionally for every house,
  // its "share" of the blame is simply the average of all the mistakes combined!
  let biasGradientTotal = 0.0;
  for (let i = 0; i < predictionErrors.rows; i++) {
    biasGradientTotal += predictionErrors.data[i][0];
  }
  const biasGradient = biasGradientTotal * (2.0 / truePrices.rows);

  // --- OPTIMIZATION STEP ---
  // Finally, we take our original Weights and Bias and nudge them against the slope.
  // We multiply by our Learning Rate (0.01) which acts as a safety brake so we don't explode.
  featureWeights = featureWeights.subtract(weightGradients.scale(learningRate));
  basePriceBias = basePriceBias - (biasGradient * learningRate);

  if (epoch % 150 === 0) {
    console.log(`Epoch ${epoch} | Global Loss: ${mseLoss.toFixed(4)} | Weights [SqFt: ${featureWeights.data[0][0].toFixed(2)}, Bed: ${featureWeights.data[1][0].toFixed(2)}] | Bias: ${basePriceBias.toFixed(2)}`);
  }
}

console.log("\nFinal Optimal Mapping Achieved!");
console.log(`Prediction for House 1 (Target $400k): $${houseFeatures.dot(featureWeights).add(basePriceBias).data[0][0].toFixed(2)}k`);
