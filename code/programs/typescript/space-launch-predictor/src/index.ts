import { Matrix } from "../../../../packages/typescript/matrix/src/matrix";
import { bce, bceDerivative } from "../../../../packages/typescript/loss-functions/src/loss_functions";
import { sigmoid, sigmoidDerivative } from "../../../../packages/typescript/activation-functions/src/activations";

console.log("\n--- Booting TS Space Launch Predictor ---");

const shuttleData = [
  [12.0, 15.0], [35.0, 85.0], [5.0, 5.0],
  [40.0, 95.0], [15.0, 30.0], [28.0, 60.0]
];
const targetData = [
  [1.0], [0.0], [1.0], [0.0], [1.0], [0.0]
];

const features = new Matrix(shuttleData);
const trueLabels = new Matrix(targetData);
let weights = new Matrix([[0.0], [0.0]]);
let bias = 0.0;
const lr = 0.01;
const epochs = 3000;

for (let epoch = 0; epoch <= epochs; epoch++) {
  const raw = features.dot(weights).add(bias);

  const linearProbs = new Array(features.rows);
  const linearTruth = new Array(features.rows);
  
  for (let i = 0; i < features.rows; i++) {
    linearProbs[i] = sigmoid(raw.data[i][0]);
    linearTruth[i] = trueLabels.data[i][0];
  }

  const logLoss = bce(linearTruth, linearProbs);
  const lossGrad = bceDerivative(linearTruth, linearProbs);

  let biasGrad = 0.0;
  const gradData: number[][] = [];
  for (let i = 0; i < features.rows; i++) {
    const actGrad = sigmoidDerivative(raw.data[i][0]);
    const combined = lossGrad[i] * actGrad;
    gradData.push([combined]);
    biasGrad += combined;
  }

  const gradMatrix = new Matrix(gradData);
  const weightGrads = features.transpose().dot(gradMatrix);

  weights = weights.subtract(weightGrads.scale(lr));
  bias -= biasGrad * lr;

  if (epoch % 500 === 0) {
    console.log(`Epoch ${epoch.toString().padStart(4)} | BCE Loss: ${logLoss.toFixed(4)} | Bias: ${bias.toFixed(2)}`);
  }
}

console.log("\n--- Final Inference ---");
const finalRaw = features.dot(weights).add(bias);
for (let i = 0; i < trueLabels.rows; i++) {
  const prob = sigmoid(finalRaw.data[i][0]);
  const truth = targetData[i][0] === 1.0 ? "Safe" : "Abort";
  const guess = prob > 0.5 ? "Safe" : "Abort";
  console.log(`Scenario ${i+1} (Truth: ${truth}) -> System: ${guess} (${(prob*100).toFixed(2)}%)`);
}
