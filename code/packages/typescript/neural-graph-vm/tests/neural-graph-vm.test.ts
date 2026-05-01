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
  compileBytecodeToMatrixPlan,
  compileNeuralGraphToBytecode,
  compileNeuralNetworkToBytecode,
  runNeuralBytecodeForward,
  runNeuralBytecodeForwardWithTrace,
  runNeuralMatrixForward,
  runNeuralMatrixForwardAsync,
  runNeuralMatrixForwardScalars,
  type AsyncNeuralMatrixBackend,
  type MatrixBackend,
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

  it("lowers forward bytecode into a matrix plan", () => {
    const bytecode = compileNeuralGraphToBytecode(makeTinyWeightedSumGraph());
    const plan = compileBytecodeToMatrixPlan(bytecode);

    expect(plan.magic).toBe("CANM");
    expect(plan.instructions.map((insn) => insn.op)).toEqual([
      "LOAD_CONST_MATRIX",
      "LOAD_INPUT_MATRIX",
      "LOAD_INPUT_MATRIX",
      "WEIGHTED_SUM_MATRIX",
      "ACTIVATE_MATRIX",
      "STORE_OUTPUT_MATRIX",
    ]);
    expect(plan.instructions[3].terms?.map((term) => term.edgeId)).toEqual([
      "bias_to_sum",
      "w0",
      "w1",
    ]);
  });

  it("runs lowered matrix plans through the default matrix backend", () => {
    const bytecode = compileNeuralGraphToBytecode(makeTinyWeightedSumGraph());
    const plan = compileBytecodeToMatrixPlan(bytecode);
    const outputs = runNeuralMatrixForwardScalars(plan, { x0: 4, x1: 8 });

    expect(outputs).toEqual({ prediction: 6 });
  });

  it("runs matrix plans across a small batch", () => {
    const bytecode = compileNeuralNetworkToBytecode(createXorNetwork());
    const plan = compileBytecodeToMatrixPlan(bytecode);
    const result = runNeuralMatrixForward(plan, {
      x0: [0, 0, 1, 1],
      x1: [0, 1, 0, 1],
    });
    const predictions = result.outputs.prediction;

    expect(predictions[0]).toBeLessThan(0.01);
    expect(predictions[1]).toBeGreaterThan(0.99);
    expect(predictions[2]).toBeGreaterThan(0.99);
    expect(predictions[3]).toBeLessThan(0.01);
  });

  it("runs matrix plans against a swappable backend interface", () => {
    const calls: string[] = [];
    const backend: MatrixBackend<number[]> = {
      fromRows(rows) {
        calls.push("fromRows");
        return rows.map((row) => row[0] ?? 0);
      },
      toRows(matrix) {
        calls.push("toRows");
        return matrix.map((value) => [value]);
      },
      column(values) {
        calls.push("column");
        return [...values];
      },
      constant(value, rows) {
        calls.push("constant");
        return Array(rows).fill(value);
      },
      add(left, right) {
        calls.push("add");
        return left.map((value, index) => value + right[index]);
      },
      scale(matrix, scalar) {
        calls.push("scale");
        return matrix.map((value) => value * scalar);
      },
      dot() {
        calls.push("dot");
        throw new Error("dot is not used by this v0 plan");
      },
      map(matrix, fn) {
        calls.push("map");
        return matrix.map(fn);
      },
      toColumn(matrix) {
        calls.push("toColumn");
        return [...matrix];
      },
    };
    const bytecode = compileNeuralGraphToBytecode(makeTinyWeightedSumGraph());
    const plan = compileBytecodeToMatrixPlan(bytecode);
    const result = runNeuralMatrixForward(
      plan,
      { x0: [4, 8], x1: [8, 16] },
      backend
    );

    expect(result.outputs).toEqual({ prediction: [6, 13] });
    expect(calls).toContain("scale");
    expect(calls).toContain("add");
    expect(calls).toContain("map");
  });

  it("runs matrix plans against an async backend interface", async () => {
    const calls: string[] = [];
    const backend: AsyncNeuralMatrixBackend<number[]> = {
      async column(values) {
        calls.push("column");
        return [...values];
      },
      async constant(value, rows) {
        calls.push("constant");
        return Array(rows).fill(value);
      },
      async add(left, right) {
        calls.push("add");
        return left.map((value, index) => value + right[index]);
      },
      async scale(matrix, scalar) {
        calls.push("scale");
        return matrix.map((value) => value * scalar);
      },
      async activate(matrix, activation) {
        calls.push(`activate:${activation}`);
        return matrix.map((value) => Math.max(0, value));
      },
      async toColumn(matrix) {
        calls.push("toColumn");
        return [...matrix];
      },
    };
    const bytecode = compileNeuralGraphToBytecode(makeTinyWeightedSumGraph());
    const plan = compileBytecodeToMatrixPlan(bytecode);
    const result = await runNeuralMatrixForwardAsync(
      plan,
      { x0: [4, 8], x1: [8, 16] },
      backend
    );

    expect(result.outputs).toEqual({ prediction: [6, 13] });
    expect(calls).toContain("scale");
    expect(calls).toContain("add");
    expect(calls).toContain("activate:relu");
  });

  it("runs async matrix plans through the default CPU backend", async () => {
    const bytecode = compileNeuralNetworkToBytecode(createXorNetwork());
    const plan = compileBytecodeToMatrixPlan(bytecode);
    const result = await runNeuralMatrixForwardAsync(plan, {
      x0: [0, 0, 1, 1],
      x1: [0, 1, 0, 1],
    });

    expect(result.outputs.prediction[0]).toBeLessThan(0.01);
    expect(result.outputs.prediction[1]).toBeGreaterThan(0.99);
    expect(result.outputs.prediction[2]).toBeGreaterThan(0.99);
    expect(result.outputs.prediction[3]).toBeLessThan(0.01);
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
