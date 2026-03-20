import { Perceptron } from "../../../../packages/typescript/perceptron/src/perceptron";

console.log("\n--- Booting TS Space Launch Predictor (OOP V2) ---");

const shuttleData = [
  [12.0, 15.0], [35.0, 85.0], [5.0, 5.0],
  [40.0, 95.0], [15.0, 30.0], [28.0, 60.0]
];
const targetData = [
  [1.0], [0.0], [1.0], [0.0], [1.0], [0.0]
];

const model = new Perceptron(0.01, 3000);
model.fit(shuttleData, targetData, 500);

console.log("\n--- Final Inference ---");
const predictions = model.predict(shuttleData);
predictions.forEach((prob, i) => {
  const truth = targetData[i][0] === 1.0 ? "Safe" : "Abort";
  const guess = prob > 0.5 ? "Safe" : "Abort";
  console.log(`Scenario ${i+1} (Truth: ${truth}) -> System: ${guess} (${(prob*100).toFixed(2)}%)`);
});
