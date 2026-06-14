import { Perceptron } from "../../../../packages/typescript/perceptron/src/perceptron";
import { createNeuralNetwork } from "@coding-adventures/neural-network";
import {
  compileBytecodeToMatrixPlan,
  compileNeuralNetworkToBytecode,
  runNeuralMatrixForwardScalars,
} from "@coding-adventures/neural-graph-vm";

console.log("\n--- Booting TS Mansion Classifier (OOP V2) ---");

const houseData = [
  [4.5, 6.0], [3.8, 5.0], [1.5, 2.0],
  [0.9, 1.0], [5.5, 7.0], [2.0, 3.0]
];
const targetData = [
  [1.0], [1.0], [0.0], [0.0], [1.0], [0.0]
];

const model = new Perceptron(0.1, 2000);
model.fit(houseData, targetData, 400);

console.log("\n--- Final Inference ---");
const predictions = model.predict(houseData);
predictions.forEach((prob, i) => {
  const truth = targetData[i][0] === 1.0 ? "Mansion" : "Normal";
  const guess = prob > 0.5 ? "Mansion" : "Normal";
  console.log(`House ${i+1} (Truth: ${truth}) -> System: ${guess} (${(prob*100).toFixed(2)}%)`);
});

if (!model.weights) {
  throw new Error("Expected trained perceptron weights before graph VM inference");
}

const graphNetwork = createNeuralNetwork("mansion-classifier")
  .input("bedrooms")
  .input("bathrooms")
  .constant("bias", 1, { "nn.role": "bias" })
  .weightedSum("mansion_logit", [
    { from: "bedrooms", weight: model.weights.data[0][0], edgeId: "bedrooms_weight" },
    { from: "bathrooms", weight: model.weights.data[1][0], edgeId: "bathrooms_weight" },
    { from: "bias", weight: model.bias, edgeId: "bias_weight" },
  ], { "nn.layer": "output", "nn.role": "weighted_sum" })
  .activation("mansion_probability", "mansion_logit", "sigmoid", {
    "nn.layer": "output",
    "nn.role": "activation",
  }, "logit_to_sigmoid")
  .output("mansion_output", "mansion_probability", "mansion_probability", {
    "nn.layer": "output",
  }, "probability_to_output");

const bytecode = compileNeuralNetworkToBytecode(graphNetwork);
const matrixPlan = compileBytecodeToMatrixPlan(bytecode);
console.log("\n--- Graph VM Matrix Inference ---");
houseData.forEach((house, i) => {
  const outputs = runNeuralMatrixForwardScalars(matrixPlan, {
    bedrooms: house[0],
    bathrooms: house[1],
  });
  const probability = outputs.mansion_probability;
  const guess = probability > 0.5 ? "Mansion" : "Normal";
  console.log(`House ${i+1} -> VM: ${guess} (${(probability*100).toFixed(2)}%)`);
});
