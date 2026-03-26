/**
 * Core mathematical limits bounding neural predictors natively.
 */

export function sigmoid(x: number): number {
  if (x < -709) return 0.0;
  if (x > 709) return 1.0;
  return 1.0 / (1.0 + Math.exp(-x));
}

export function sigmoidDerivative(x: number): number {
  const sig = sigmoid(x);
  return sig * (1.0 - sig);
}

export function relu(x: number): number {
  return Math.max(0.0, x);
}

export function reluDerivative(x: number): number {
  return x > 0 ? 1.0 : 0.0;
}

export function tanh(x: number): number {
  return Math.tanh(x);
}

export function tanhDerivative(x: number): number {
  const t = Math.tanh(x);
  return 1.0 - (t * t);
}
