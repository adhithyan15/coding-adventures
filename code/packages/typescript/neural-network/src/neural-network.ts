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
