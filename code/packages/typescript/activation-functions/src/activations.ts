/**
 * Core mathematical limits bounding neural predictors natively.
 */

const LEAKY_RELU_SLOPE = 0.01;

export function linear(x: number): number {
  return x;
}

export function linearDerivative(_x: number): number {
  return 1.0;
}

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

export function leakyRelu(x: number): number {
  return x > 0 ? x : LEAKY_RELU_SLOPE * x;
}

export function leakyReluDerivative(x: number): number {
  return x > 0 ? 1.0 : LEAKY_RELU_SLOPE;
}

export function tanh(x: number): number {
  return Math.tanh(x);
}

export function tanhDerivative(x: number): number {
  const t = Math.tanh(x);
  return 1.0 - (t * t);
}

export function softplus(x: number): number {
  return Math.log1p(Math.exp(-Math.abs(x))) + Math.max(x, 0.0);
}

export function softplusDerivative(x: number): number {
  return sigmoid(x);
}
