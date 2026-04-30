import {
  MultiDirectedGraph,
  type GraphPropertyBag,
} from "@coding-adventures/multi-directed-graph";

export type NeuralGraph = MultiDirectedGraph<string>;
export type ActivationKind = "relu" | "sigmoid" | "tanh" | "none";

export interface WeightedInput {
  readonly from: string;
  readonly weight?: number;
  readonly edgeId?: string;
  readonly properties?: GraphPropertyBag;
}

export class NeuralNetwork {
  readonly graph: NeuralGraph;

  constructor(name?: string, graph: NeuralGraph = createNeuralGraph(name)) {
    this.graph = graph;
  }

  input(
    node: string,
    inputName: string = node,
    properties: GraphPropertyBag = {}
  ): this {
    addInput(this.graph, node, inputName, properties);
    return this;
  }

  constant(
    node: string,
    value: number,
    properties: GraphPropertyBag = {}
  ): this {
    addConstant(this.graph, node, value, properties);
    return this;
  }

  weightedSum(
    node: string,
    inputs: readonly WeightedInput[],
    properties: GraphPropertyBag = {}
  ): this {
    addWeightedSum(this.graph, node, inputs, properties);
    return this;
  }

  activation(
    node: string,
    input: string,
    activation: ActivationKind,
    properties: GraphPropertyBag = {},
    edgeId?: string
  ): this {
    addActivation(this.graph, node, input, activation, properties, edgeId);
    return this;
  }

  output(
    node: string,
    input: string,
    outputName: string = node,
    properties: GraphPropertyBag = {},
    edgeId?: string
  ): this {
    addOutput(this.graph, node, input, outputName, properties, edgeId);
    return this;
  }
}

export function createNeuralNetwork(name?: string): NeuralNetwork {
  return new NeuralNetwork(name);
}

export function createNeuralGraph(name?: string): NeuralGraph {
  const graph = new MultiDirectedGraph<string>();
  graph.setGraphProperty("nn.version", "0");
  if (name !== undefined) {
    graph.setGraphProperty("nn.name", name);
  }
  return graph;
}

export function addInput(
  graph: NeuralGraph,
  node: string,
  inputName: string = node,
  properties: GraphPropertyBag = {}
): void {
  graph.addNode(node, {
    ...properties,
    "nn.op": "input",
    "nn.input": inputName,
  });
}

export function addConstant(
  graph: NeuralGraph,
  node: string,
  value: number,
  properties: GraphPropertyBag = {}
): void {
  if (!Number.isFinite(value)) {
    throw new Error("constant value must be finite");
  }
  graph.addNode(node, {
    ...properties,
    "nn.op": "constant",
    "nn.value": value,
  });
}

export function addWeightedSum(
  graph: NeuralGraph,
  node: string,
  inputs: readonly WeightedInput[],
  properties: GraphPropertyBag = {}
): void {
  graph.addNode(node, {
    ...properties,
    "nn.op": "weighted_sum",
  });
  for (const input of inputs) {
    graph.addEdge(
      input.from,
      node,
      input.weight ?? 1.0,
      input.properties ?? {},
      input.edgeId
    );
  }
}

export function addActivation(
  graph: NeuralGraph,
  node: string,
  input: string,
  activation: ActivationKind,
  properties: GraphPropertyBag = {},
  edgeId?: string
): string {
  graph.addNode(node, {
    ...properties,
    "nn.op": "activation",
    "nn.activation": activation,
  });
  return graph.addEdge(input, node, 1.0, {}, edgeId);
}

export function createXorNetwork(name = "xor"): NeuralNetwork {
  return createNeuralNetwork(name)
    .input("x0")
    .input("x1")
    .constant("bias", 1, { "nn.role": "bias" })
    .weightedSum("h_or_sum", [
      { from: "x0", weight: 20, edgeId: "x0_to_h_or" },
      { from: "x1", weight: 20, edgeId: "x1_to_h_or" },
      { from: "bias", weight: -10, edgeId: "bias_to_h_or" },
    ], { "nn.layer": "hidden", "nn.role": "weighted_sum" })
    .activation("h_or", "h_or_sum", "sigmoid", {
      "nn.layer": "hidden",
      "nn.role": "activation",
    }, "h_or_sum_to_h_or")
    .weightedSum("h_nand_sum", [
      { from: "x0", weight: -20, edgeId: "x0_to_h_nand" },
      { from: "x1", weight: -20, edgeId: "x1_to_h_nand" },
      { from: "bias", weight: 30, edgeId: "bias_to_h_nand" },
    ], { "nn.layer": "hidden", "nn.role": "weighted_sum" })
    .activation("h_nand", "h_nand_sum", "sigmoid", {
      "nn.layer": "hidden",
      "nn.role": "activation",
    }, "h_nand_sum_to_h_nand")
    .weightedSum("out_sum", [
      { from: "h_or", weight: 20, edgeId: "h_or_to_out" },
      { from: "h_nand", weight: 20, edgeId: "h_nand_to_out" },
      { from: "bias", weight: -30, edgeId: "bias_to_out" },
    ], { "nn.layer": "output", "nn.role": "weighted_sum" })
    .activation("out_activation", "out_sum", "sigmoid", {
      "nn.layer": "output",
      "nn.role": "activation",
    }, "out_sum_to_activation")
    .output("out", "out_activation", "prediction", {
      "nn.layer": "output",
    }, "activation_to_out");
}

export function addOutput(
  graph: NeuralGraph,
  node: string,
  input: string,
  outputName: string = node,
  properties: GraphPropertyBag = {},
  edgeId?: string
): string {
  graph.addNode(node, {
    ...properties,
    "nn.op": "output",
    "nn.output": outputName,
  });
  return graph.addEdge(input, node, 1.0, {}, edgeId);
}
