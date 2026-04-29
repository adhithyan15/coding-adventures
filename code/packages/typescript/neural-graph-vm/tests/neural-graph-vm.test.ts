import {
  addActivation,
  addInput,
  addOutput,
  addWeightedSum,
  createNeuralGraph,
  createNeuralNetwork,
  type NeuralGraph,
} from "@coding-adventures/neural-network";
import { describe, expect, it } from "vitest";

import {
  NeuralGraphCompileError,
  compileNeuralGraphToBytecode,
  compileNeuralNetworkToBytecode,
  runNeuralBytecodeForward,
} from "../src/index.js";

function makeTinyWeightedSumGraph(): NeuralGraph {
  const graph = createNeuralGraph("tiny-weighted-sum");

  addInput(graph, "x0");
  addInput(graph, "x1");
  addWeightedSum(graph, "sum", [
    { from: "x0", weight: 0.25, edgeId: "w0", properties: { "nn.trainable": true } },
    { from: "x1", weight: 0.75, edgeId: "w1", properties: { "nn.trainable": true } },
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
      "sum_to_relu",
      "relu_to_out",
    ]);
    expect(bytecode.functions[0].instructions.map((insn) => insn.op)).toEqual([
      "LOAD_INPUT",
      "LOAD_INPUT",
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

    expect(outputs).toEqual({ prediction: 7 });
  });

  it("supports negative weighted sums through relu", () => {
    const graph = makeTinyWeightedSumGraph();
    const bytecode = compileNeuralGraphToBytecode(graph);
    const outputs = runNeuralBytecodeForward(bytecode, { x0: -4, x1: -8 });

    expect(outputs).toEqual({ prediction: 0 });
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
