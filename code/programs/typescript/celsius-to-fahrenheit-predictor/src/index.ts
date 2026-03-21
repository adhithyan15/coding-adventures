import { mse, mae, mseDerivative, maeDerivative } from "coding-adventures-loss-functions/src/loss_functions";
import { sgd } from "coding-adventures-gradient-descent/src/gradient_descent";

const celsius = [-40.0, -10.0, 0.0, 8.0, 15.0, 22.0, 38.0];
const fahrenheit = [-40.0, 14.0, 32.0, 46.4, 59.0, 71.6, 100.4];

function train(lossName: string, lossFn: Function, derivFn: Function, lr: number, maxEpochs = 10000) {
  let w = 0.5;
  let b = 0.5;

  console.log(`\n--- Celsius to Fahrenheit Predictor: Training with ${lossName} ---`);

  for (let epoch = 0; epoch < maxEpochs; epoch++) {
    const yPred = celsius.map(c => w * c + b);
    const err = lossFn(fahrenheit, yPred);

    if (err < 0.5) {
      console.log(`Converged beautifully in ${epoch + 1} epochs! (Loss: ${err.toFixed(6)})`);
      console.log(`Final Formula: F = C * ${w.toFixed(6)} + ${b.toFixed(6)}`);
      break;
    }

    const gradients = derivFn(fahrenheit, yPred);
    
    let gradW = 0;
    let gradB = 0;
    for (let i = 0; i < gradients.length; i++) {
        gradW += gradients[i] * celsius[i];
        gradB += gradients[i];
    }

    const [newW, newB] = sgd([w, b], [gradW, gradB], lr);
    w = newW;
    b = newB;

    if ((epoch + 1) % 1000 === 0) {
      console.log(`Epoch ${(epoch + 1).toString().padStart(4, '0')} -> Loss: ${err.toFixed(6)} | w: ${w.toFixed(4)} | b: ${b.toFixed(4)}`);
    }
  }

  const predF = w * 100.0 + b;
  console.log(`Prediction for 100.0 C -> ${predF.toFixed(2)} F (Expected ~212.00 F)`);
}

train("Mean Squared Error (MSE)", mse, mseDerivative, 0.0005);
train("Mean Absolute Error (MAE)", mae, maeDerivative, 0.01);
