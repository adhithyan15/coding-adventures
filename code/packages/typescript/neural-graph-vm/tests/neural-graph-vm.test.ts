import {
  addActivation,
  addInput,
  addOutput,
  addWeightedSum,
  createNeuralGraph,
  createNeuralNetwork,
  createXorNetwork,
  type NeuralGraph,
} from "@coding-adventures/neural-network";
import { describe, expect, it } from "vitest";

import {
  NeuralGraphCompileError,
  compileNeuralGraphToBytecode,
  compileNeuralNetworkToBytecode,
  runNeuralBytecodeForward,
  runNeuralBytecodeForwardWithTrace,
} from "../src/index.js";

function makeTinyWeightedSumGraph(): NeuralGraph {
  const graph = createNeuralGraph("tiny-weighted-sum");

  addInput(graph, "x0");
  addInput(graph, "x1");
  graph.addNode("bias", { "nn.op": "constant", "nn.value": 1 });
  addWeightedSum(graph, "sum", [
    { from: "x0", weight: 0.25, edgeId: "w0", properties: { "nn.trainable": true } },
    { from: "x1", weight: 0.75, edgeId: "w1", properties: { "nn.trainable": true } },
    { from: "bias", weight: -1, edgeId: "bias_to_sum" },
  ]);
  addActivation(graph, "relu", "sum", "relu", {}, "sum_to_relu");
  addOutput(graph, "out", "relu", "prediction", {}, "relu_to_out");

  return graph;
}

describe("neural graph vm", () => {
  it("compiles a multi-directed graph into forward bytecode", () => {
    const bytecode = compileNeuralGraphToBytecode(makeTinyWeightedSumGraph());

    expect(bytecode.magic).toBe("CANN");
    expect(bytecode.version).toBe(0);
    expect(bytecode.graph.edges.map((edge) => edge.id)).toEqual([
      "w0",
      "w1",
      "bias_to_sum",
      "sum_to_relu",
      "relu_to_out",
    ]);
    expect(bytecode.functions[0].instructions.map((insn) => insn.op)).toEqual([
      "LOAD_CONST",
      "LOAD_INPUT",
      "LOAD_INPUT",
      "LOAD_EDGE_WEIGHT",
      "MUL",
      "LOAD_EDGE_WEIGHT",
      "MUL",
      "LOAD_EDGE_WEIGHT",
      "MUL",
      "ADD",
      "ACTIVATE",
      "STORE_OUTPUT",
    ]);
  });

  it("compiles a generic NeuralNetwork package model", () => {
    const network = createNeuralNetwork("tiny-network")
      .input("x0")
      .input("x1")
      .weightedSum("sum", [
        { from: "x0", weight: 0.25, edgeId: "w0" },
        { from: "x1", weight: 0.75, edgeId: "w1" },
      ])
      .output("out", "sum", "prediction", {}, "sum_to_out");

    const bytecode = compileNeuralNetworkToBytecode(network);
    const outputs = runNeuralBytecodeForward(bytecode, { x0: 4, x1: 8 });

    expect(outputs).toEqual({ prediction: 7 });
  });

  it("runs the forward bytecode through the scalar reference interpreter", () => {
    const bytecode = compileNeuralGraphToBytecode(makeTinyWeightedSumGraph());
    const outputs = runNeuralBytecodeForward(bytecode, { x0: 4, x1: 8 });

    expect(outputs).toEqual({ prediction: 6 });
  });

  it("supports negative weighted sums through relu", () => {
    const graph = makeTinyWeightedSumGraph();
    const bytecode = compileNeuralGraphToBytecode(graph);
    const outputs = runNeuralBytecodeForward(bytecode, { x0: -4, x1: -8 });

    expect(outputs).toEqual({ prediction: 0 });
  });

  it("runs XOR through compiled bytecode", () => {
    const bytecode = compileNeuralNetworkToBytecode(createXorNetwork());
    const predictions = [
      runNeuralBytecodeForward(bytecode, { x0: 0, x1: 0 }).prediction,
      runNeuralBytecodeForward(bytecode, { x0: 0, x1: 1 }).prediction,
      runNeuralBytecodeForward(bytecode, { x0: 1, x1: 0 }).prediction,
      runNeuralBytecodeForward(bytecode, { x0: 1, x1: 1 }).prediction,
    ];

    expect(predictions[0]).toBeLessThan(0.01);
    expect(predictions[1]).toBeGreaterThan(0.99);
    expect(predictions[2]).toBeGreaterThan(0.99);
    expect(predictions[3]).toBeLessThan(0.01);
  });

  it("traces bytecode values back to graph nodes and edges", () => {
    const bytecode = compileNeuralGraphToBytecode(makeTinyWeightedSumGraph());
    const trace = runNeuralBytecodeForwardWithTrace(bytecode, { x0: 4, x1: 8 });
    const biasLoad = trace.instructions.find((entry) => (
      entry.instruction.op === "LOAD_CONST" && entry.sourceNode === "bias"
    ));
    const biasTerm = trace.instructions.find((entry) => (
      entry.instruction.op === "MUL" && entry.sourceEdge === "bias_to_sum"
    ));
    const store = trace.instructions.find((entry) => entry.instruction.op === "STORE_OUTPUT");

    expect(trace.outputs).toEqual({ prediction: 6 });
    expect(biasLoad?.write?.value).toBe(1);
    expect(biasTerm?.reads.map((read) => read.value)).toContain(-1);
    expect(store?.output).toEqual({ outputName: "prediction", value: 6 });
  });

  it("rejects unsupported neural graph ops", () => {
    const graph = createNeuralGraph();
    graph.addNode("custom", { "nn.op": "custom_kernel" });

    expect(() => compileNeuralGraphToBytecode(graph)).toThrow(
      NeuralGraphCompileError
    );
  });

  it("requires runtime inputs", () => {
    const bytecode = compileNeuralGraphToBytecode(makeTinyWeightedSumGraph());

    expect(() => runNeuralBytecodeForward(bytecode, { x0: 1 })).toThrow(
      "Missing input: x1"
    );
  });
});
