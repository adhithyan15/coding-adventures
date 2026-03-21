import { Perceptron } from "../../../../packages/typescript/perceptron/src/perceptron";

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
