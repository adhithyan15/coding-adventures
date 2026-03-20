/**
 * Multi-Variable Linear Regression: House Price Predictor
 * -------------------------------------------------------
 * Utilizing explicit object-oriented structural patterns mapped directly
 * against TypeScript node bindings! This loop handles dimensional math effortlessly.
 */
import { Matrix } from "../../../../packages/typescript/matrix/src/matrix";
import { mse as meanSquaredError } from "../../../../packages/typescript/loss-functions/src/loss_functions";

console.log("\n--- Booting Multi-Variable Predictor: House Prices ---\n");

// 1. The Dataset Layout (Inputs)
// X constructs a natively instanced Grid layout storing real-estate properties locally.
// Column 0 maps to SqFt in properties (thousands). Column 1 maps to Bedrooms cleanly.
const x = new Matrix([
  [2.0, 3.0],
  [1.5, 2.0],
  [2.5, 4.0],
  [1.0, 1.0]
]);

// 2. The Target Labels (Y Outputs)
// Evaluated precisely against real-world distributions dynamically natively.
const y = new Matrix([
  [400.0],
  [300.0],
  [500.0],
  [200.0]
]);

// 3. Mathematical Node Layout Parameters statically tracking memory
let w = new Matrix([[0.5], [0.5]]);
let b = 0.5;
const lr = 0.01;

console.log("Beginning Training Epochs...");
for (let epoch = 0; epoch <= 1500; epoch++) {
  
  // --- INVERSE FORWARD MAPPING ---
  // Calculates natively evaluated multiple dimensions explicitly universally.
  const pred = x.dot(w);
  const yPred = pred.add(b);

  // Resolving global loss function boundaries natively dynamically evaluating MSE nodes.
  const yTrueList = y.data.map((r: number[]) => r[0]);
  const yPredList = yPred.data.map((r: number[]) => r[0]);
  const totalLoss = meanSquaredError(yTrueList, yPredList);

  // --- BACKPROPAGATION (CALCULATING GRADIENTS) ---
  // How do we figure out exactly how much the SqFt Weight vs Bedroom Weight was responsible for the error?
  // 1. We take our original (N BY 2) Data Grid (X) and physically flip it on its side to become (2 BY N). 
  //    - Row 1 now contains only SqFt values. Row 2 contains only Bedroom values.
  // 2. We Dot Product this (2 BY N) grid against our (N BY 1) Error Vector!
  //    - This multiplies every single SqFt value by its respective Error, collapsing into a (2 BY 1) Gradient Vector.
  const errMat = yPred.subtract(y);
  const xT = x.transpose();
  const dotErr = xT.dot(errMat);
  
  // We multiply by (2 / N) because of the Mean Squared Error derivative scaling.
  const dw = dotErr.scale(2.0 / y.rows);

  // For the Bias (b), because it shifts the prediction unconditionally for every house,
  // its "share" of the blame is simply the average of all the mistakes combined!
  // We take the raw (N BY 1) Error array, sum up the N values, and scale it by 2/N.
  let dbTotal = 0.0;
  for (let i = 0; i < errMat.rows; i++) {
    dbTotal += errMat.data[i][0];
  }
  const db = dbTotal * (2.0 / y.rows);

  // --- OPTIMIZATION STEP ---
  // Finally, we take our original Weights and Bias and nudge them against the slope.
  // We multiply by our Learning Rate (0.01) which acts as a safety brake so we don't 
  // overshoot the target and cause the math to explode into infinity!
  w = w.subtract(dw.scale(lr));
  b = b - (db * lr);

  if (epoch % 150 === 0) {
    console.log(`Epoch ${epoch} | Global Loss: ${totalLoss.toFixed(4)} | Weights [SqFt: ${w.data[0][0].toFixed(2)}, Bed: ${w.data[1][0].toFixed(2)}] | Bias: ${b.toFixed(2)}`);
  }
}

console.log("\nFinal Optimal Mapping Achieved!");
console.log(`Prediction for House 1 (Target $400k): $${x.dot(w).add(b).data[0][0].toFixed(2)}k`);
