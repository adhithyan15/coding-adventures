import {
  MultiDirectedGraph,
  type GraphPropertyValue,
} from "@coding-adventures/multi-directed-graph";

export type NeuralBytecodeOpcode =
  | "LOAD_INPUT"
  | "LOAD_CONST"
  | "LOAD_EDGE_WEIGHT"
  | "MUL"
  | "ADD"
  | "ACTIVATE"
  | "STORE_OUTPUT";

export interface NeuralBytecodeInstruction {
  readonly op: NeuralBytecodeOpcode;
  readonly dst?: string;
  readonly inputName?: string;
  readonly outputName?: string;
  readonly edgeId?: string;
  readonly value?: number;
  readonly left?: string;
  readonly right?: string;
  readonly inputs?: readonly string[];
  readonly input?: string;
  readonly activation?: string;
  readonly sourceNode?: string;
  readonly sourceEdge?: string;
}

export interface NeuralBytecodeFunction {
  readonly id: string;
  readonly kind: "forward";
  readonly instructions: readonly NeuralBytecodeInstruction[];
}

export interface NeuralBytecodeModule {
  readonly magic: "CANN";
  readonly version: 0;
  readonly graph: {
    readonly nodes: readonly string[];
    readonly edges: readonly {
      readonly id: string;
      readonly from: string;
      readonly to: string;
      readonly weight: number;
    }[];
  };
  readonly functions: readonly NeuralBytecodeFunction[];
}

export class NeuralGraphCompileError extends Error {
  constructor(
    message: string,
    public readonly nodeId?: string,
    public readonly edgeId?: string
  ) {
    super(message);
    this.name = "NeuralGraphCompileError";
  }
}

export function compileNeuralGraphToBytecode(
  graph: MultiDirectedGraph
): NeuralBytecodeModule {
  const order = graph.topologicalSort();
  const instructions: NeuralBytecodeInstruction[] = [];
  const values = new Map<string, string>();
  let nextValueId = 0;

  const allocateValue = (): string => `v${nextValueId++}`;

  for (const node of order) {
    const properties = graph.nodeProperties(node);
    const op = stringProperty(properties["nn.op"], "weighted_sum");

    if (op === "input") {
      const dst = allocateValue();
      values.set(node, dst);
      instructions.push({
        op: "LOAD_INPUT",
        dst,
        inputName: stringProperty(properties["nn.input"], node),
        sourceNode: node,
      });
      continue;
    }

    if (op === "weighted_sum") {
      const terms: string[] = [];
      for (const edge of graph.incomingEdges(node).sort(compareEdgesById)) {
        const sourceValue = values.get(edge.from);
        if (sourceValue === undefined) {
          throw new NeuralGraphCompileError(
            `Source node has no value: ${edge.from}`,
            edge.from,
            edge.id
          );
        }
        const weightValue = allocateValue();
        const termValue = allocateValue();
        instructions.push({
          op: "LOAD_EDGE_WEIGHT",
          dst: weightValue,
          edgeId: edge.id,
          sourceEdge: edge.id,
        });
        instructions.push({
          op: "MUL",
          dst: termValue,
          left: sourceValue,
          right: weightValue,
          sourceEdge: edge.id,
        });
        terms.push(termValue);
      }

      const dst = allocateValue();
      values.set(node, dst);
      instructions.push({
        op: terms.length === 0 ? "LOAD_CONST" : "ADD",
        dst,
        value: terms.length === 0 ? 0 : undefined,
        inputs: terms.length === 0 ? undefined : terms,
        sourceNode: node,
      });
      continue;
    }

    if (op === "activation") {
      const inputValue = singleInputValue(graph, values, node);
      const dst = allocateValue();
      values.set(node, dst);
      instructions.push({
        op: "ACTIVATE",
        dst,
        input: inputValue,
        activation: stringProperty(properties["nn.activation"], "relu"),
        sourceNode: node,
      });
      continue;
    }

    if (op === "output") {
      const inputValue = singleInputValue(graph, values, node);
      values.set(node, inputValue);
      instructions.push({
        op: "STORE_OUTPUT",
        outputName: stringProperty(properties["nn.output"], node),
        input: inputValue,
        sourceNode: node,
      });
      continue;
    }

    throw new NeuralGraphCompileError(`Unsupported neural graph op: ${op}`, node);
  }

  return {
    magic: "CANN",
    version: 0,
    graph: {
      nodes: graph.nodes(),
      edges: graph.edges().map((edge) => ({
        id: edge.id,
        from: edge.from,
        to: edge.to,
        weight: edge.weight,
      })),
    },
    functions: [
      {
        id: "forward",
        kind: "forward",
        instructions,
      },
    ],
  };
}

export function runNeuralBytecodeForward(
  module: NeuralBytecodeModule,
  inputs: Record<string, number>
): Record<string, number> {
  const values = new Map<string, number>();
  const edgeWeights = new Map(
    module.graph.edges.map((edge) => [edge.id, edge.weight])
  );
  const outputs: Record<string, number> = {};
  const forward = module.functions.find((fn) => fn.kind === "forward");
  if (forward === undefined) {
    throw new Error("Neural bytecode module has no forward function");
  }

  for (const instruction of forward.instructions) {
    switch (instruction.op) {
      case "LOAD_INPUT":
        requireDst(instruction);
        values.set(instruction.dst, readInput(inputs, instruction.inputName));
        break;
      case "LOAD_CONST":
        requireDst(instruction);
        values.set(instruction.dst, instruction.value ?? 0);
        break;
      case "LOAD_EDGE_WEIGHT":
        requireDst(instruction);
        values.set(instruction.dst, edgeWeights.get(instruction.edgeId ?? "") ?? 1);
        break;
      case "MUL":
        requireDst(instruction);
        values.set(
          instruction.dst,
          readValue(values, instruction.left) * readValue(values, instruction.right)
        );
        break;
      case "ADD":
        requireDst(instruction);
        values.set(
          instruction.dst,
          (instruction.inputs ?? []).reduce(
            (sum, valueId) => sum + readValue(values, valueId),
            0
          )
        );
        break;
      case "ACTIVATE":
        requireDst(instruction);
        values.set(
          instruction.dst,
          applyScalarActivation(
            readValue(values, instruction.input),
            instruction.activation ?? "relu"
          )
        );
        break;
      case "STORE_OUTPUT":
        outputs[instruction.outputName ?? "output"] = readValue(
          values,
          instruction.input
        );
        break;
    }
  }

  return outputs;
}

function singleInputValue(
  graph: MultiDirectedGraph,
  values: Map<string, string>,
  node: string
): string {
  const incoming = graph.incomingEdges(node).sort(compareEdgesById);
  if (incoming.length !== 1) {
    throw new NeuralGraphCompileError(
      `Expected exactly one input edge for ${node}, got ${incoming.length}`,
      node
    );
  }
  const value = values.get(incoming[0].from);
  if (value === undefined) {
    throw new NeuralGraphCompileError(
      `Source node has no value: ${incoming[0].from}`,
      incoming[0].from,
      incoming[0].id
    );
  }
  return value;
}

function compareEdgesById(left: { id: string }, right: { id: string }): number {
  return left.id.localeCompare(right.id);
}

function stringProperty(
  value: GraphPropertyValue | undefined,
  fallback: string
): string {
  return typeof value === "string" ? value : fallback;
}

function requireDst(instruction: NeuralBytecodeInstruction): asserts instruction is
  NeuralBytecodeInstruction & { readonly dst: string } {
  if (instruction.dst === undefined) {
    throw new Error(`Instruction ${instruction.op} is missing dst`);
  }
}

function readInput(inputs: Record<string, number>, inputName: string | undefined): number {
  if (inputName === undefined || !(inputName in inputs)) {
    throw new Error(`Missing input: ${inputName ?? "<undefined>"}`);
  }
  return inputs[inputName];
}

function readValue(values: Map<string, number>, valueId: string | undefined): number {
  if (valueId === undefined || !values.has(valueId)) {
    throw new Error(`Missing value: ${valueId ?? "<undefined>"}`);
  }
  return values.get(valueId)!;
}

function applyScalarActivation(value: number, activation: string): number {
  switch (activation) {
    case "relu":
      return Math.max(0, value);
    case "sigmoid":
      return 1 / (1 + Math.exp(-Math.max(-500, Math.min(500, value))));
    case "tanh":
      return Math.tanh(value);
    case "none":
      return value;
    default:
      return value;
  }
}
