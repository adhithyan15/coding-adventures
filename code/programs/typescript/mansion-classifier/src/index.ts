import { Matrix } from "../../../../packages/typescript/matrix/src/matrix";
import { bce, bceDerivative } from "../../../../packages/typescript/loss-functions/src/loss_functions";
import { sigmoid, sigmoidDerivative } from "../../../../packages/typescript/activation-functions/src/activations";

console.log("\n--- Booting TS Mansion Classifier ---");

const houseData = [
  [4.5, 6.0], [3.8, 5.0], [1.5, 2.0],
  [0.9, 1.0], [5.5, 7.0], [2.0, 3.0]
];
const targetData = [
  [1.0], [1.0], [0.0], [0.0], [1.0], [0.0]
];

const features = new Matrix(houseData);
const trueLabels = new Matrix(targetData);
let weights = new Matrix([[0.0], [0.0]]);
let bias = 0.0;
const lr = 0.1;
const epochs = 2000;

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

  if (epoch % 400 === 0) {
    console.log(`Epoch ${epoch.toString().padStart(4)} | BCE Loss: ${logLoss.toFixed(4)} | Bias: ${bias.toFixed(2)}`);
  }
}

console.log("\n--- Final Inference ---");
const finalRaw = features.dot(weights).add(bias);
for (let i = 0; i < trueLabels.rows; i++) {
  const prob = sigmoid(finalRaw.data[i][0]);
  const truth = targetData[i][0] === 1.0 ? "Mansion" : "Normal";
  const guess = prob > 0.5 ? "Mansion" : "Normal";
  console.log(`House ${i+1} (Truth: ${truth}) -> System: ${guess} (${(prob*100).toFixed(2)}%)`);
}
