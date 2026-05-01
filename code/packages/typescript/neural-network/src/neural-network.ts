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

export interface FeedForwardLayer {
  readonly name?: string;
  readonly weights: readonly (readonly number[])[];
  readonly biases: readonly number[];
  readonly activation?: ActivationKind;
  readonly outputNames?: readonly string[];
}

export interface FeedForwardNetworkOptions {
  readonly name?: string;
  readonly inputNames: readonly string[];
  readonly layers: readonly FeedForwardLayer[];
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

export function createFeedForwardNetwork(
  options: FeedForwardNetworkOptions
): NeuralNetwork {
  if (options.inputNames.length === 0) {
    throw new Error("feed-forward network must have at least one input");
  }
  if (options.layers.length === 0) {
    throw new Error("feed-forward network must have at least one layer");
  }

  const network = createNeuralNetwork(options.name);
  const biasNode = "bias";
  network.constant(biasNode, 1, { "nn.role": "bias" });

  let previousNodes = options.inputNames.map((inputName, index) => {
    const node = `input_${index}`;
    network.input(node, inputName, {
      "nn.layer": "input",
      "nn.index": index,
    });
    return node;
  });

  for (const [layerIndex, layer] of options.layers.entries()) {
    const layerName = layer.name ?? `layer_${layerIndex}`;
    validateFeedForwardLayer(layer, previousNodes.length, layerName);

    const nextNodes: string[] = [];
    for (let unit = 0; unit < layer.biases.length; unit += 1) {
      const sumNode = `${layerName}_${unit}_sum`;
      const activationNode = `${layerName}_${unit}`;
      network
        .weightedSum(
          sumNode,
          [
            ...previousNodes.map((node, inputIndex) => ({
              from: node,
              weight: layer.weights[inputIndex]![unit],
              edgeId: `${node}_to_${sumNode}`,
              properties: {
                "nn.trainable": true,
                "nn.layer": layerName,
              },
            })),
            {
              from: biasNode,
              weight: layer.biases[unit],
              edgeId: `${biasNode}_to_${sumNode}`,
              properties: {
                "nn.trainable": true,
                "nn.role": "bias_weight",
                "nn.layer": layerName,
              },
            },
          ],
          {
            "nn.layer": layerName,
            "nn.index": unit,
            "nn.role": "weighted_sum",
          }
        )
        .activation(
          activationNode,
          sumNode,
          layer.activation ?? "none",
          {
            "nn.layer": layerName,
            "nn.index": unit,
            "nn.role": "activation",
          },
          `${sumNode}_to_${activationNode}`
        );
      nextNodes.push(activationNode);
    }

    if (layerIndex === options.layers.length - 1) {
      for (const [unit, node] of nextNodes.entries()) {
        const outputName = layer.outputNames?.[unit] ?? (
          nextNodes.length === 1 ? "prediction" : `output${unit}`
        );
        network.output(
          `${layerName}_${unit}_out`,
          node,
          outputName,
          {
            "nn.layer": layerName,
            "nn.index": unit,
            "nn.role": "output",
          },
          `${node}_to_${layerName}_${unit}_out`
        );
      }
    }

    previousNodes = nextNodes;
  }

  return network;
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

function validateFeedForwardLayer(
  layer: FeedForwardLayer,
  inputCount: number,
  layerName: string
): void {
  if (layer.biases.length === 0) {
    throw new Error(`${layerName} must have at least one unit`);
  }
  if (layer.weights.length !== inputCount) {
    throw new Error(
      `${layerName} weight row count must match previous layer width`
    );
  }
  if (
    layer.outputNames !== undefined &&
    layer.outputNames.length !== layer.biases.length
  ) {
    throw new Error(`${layerName} output name count must match unit count`);
  }
  for (const [rowIndex, row] of layer.weights.entries()) {
    if (row.length !== layer.biases.length) {
      throw new Error(
        `${layerName} weight row ${rowIndex} width must match bias count`
      );
    }
    for (const value of row) {
      if (!Number.isFinite(value)) {
        throw new Error(`${layerName} weights must be finite`);
      }
    }
  }
  for (const value of layer.biases) {
    if (!Number.isFinite(value)) {
      throw new Error(`${layerName} biases must be finite`);
    }
  }
}
