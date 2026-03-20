export function sgd(weights: number[], gradients: number[], learningRate: number): number[] {
  if (weights.length !== gradients.length || weights.length === 0) {
    throw new Error("Arrays must have the same non-zero length");
  }

  const n = weights.length;
  const res: number[] = new Array(n);
  for (let i = 0; i < n; i++) {
    res[i] = weights[i] - (learningRate * gradients[i]);
  }
  return res;
}
