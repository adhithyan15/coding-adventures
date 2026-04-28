export type ActivationKind = "linear" | "relu" | "leakyRelu" | "sigmoid" | "tanh" | "softplus";

export interface ActivationDefinition {
  kind: ActivationKind;
  label: string;
  summary: string;
}

export const ACTIVATIONS: ActivationDefinition[] = [
  {
    kind: "linear",
    label: "Linear",
    summary: "Passes the signal through unchanged; useful for regression outputs.",
  },
  {
    kind: "relu",
    label: "ReLU",
    summary: "Clips negative values to zero and keeps positive values, creating sparse activations.",
  },
  {
    kind: "leakyRelu",
    label: "Leaky ReLU",
    summary: "Keeps a small negative slope so negative inputs do not become completely silent.",
  },
  {
    kind: "sigmoid",
    label: "Sigmoid",
    summary: "Squashes values into 0 to 1, which is useful for probability-style outputs.",
  },
  {
    kind: "tanh",
    label: "Tanh",
    summary: "Squashes values into -1 to 1 and stays centered around zero.",
  },
  {
    kind: "softplus",
    label: "Softplus",
    summary: "A smooth ReLU-like curve that never has a sharp corner.",
  },
];

export function activate(value: number, kind: ActivationKind): number {
  switch (kind) {
    case "linear":
      return value;
    case "relu":
      return Math.max(0, value);
    case "leakyRelu":
      return value >= 0 ? value : value * 0.1;
    case "sigmoid":
      return 1 / (1 + Math.exp(-value));
    case "tanh":
      return Math.tanh(value);
    case "softplus":
      return Math.log1p(Math.exp(-Math.abs(value))) + Math.max(value, 0);
  }
}

export function activationByKind(kind: ActivationKind): ActivationDefinition {
  return ACTIVATIONS.find((activation) => activation.kind === kind) ?? ACTIVATIONS[0]!;
}
