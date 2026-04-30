import { describe, expect, it } from "vitest";

import {
  addActivation,
  addConstant,
  addInput,
  addOutput,
  addWeightedSum,
  createNeuralGraph,
  createNeuralNetwork,
  createXorNetwork,
} from "../src/index.js";

describe("neural-network primitives", () => {
  it("creates a neural graph with graph metadata", () => {
    const graph = createNeuralGraph("tiny-model");

    expect(graph.graphProperties()).toEqual({
      "nn.version": "0",
      "nn.name": "tiny-model",
    });
  });

  it("authors primitive metadata on a generic graph", () => {
    const graph = createNeuralGraph();

    addInput(graph, "x0");
    addInput(graph, "x1", "feature");
    addConstant(graph, "bias", 1);
    addWeightedSum(graph, "sum", [
      { from: "x0", weight: 0.25, edgeId: "w0", properties: { "nn.trainable": true } },
      { from: "x1", weight: 0.75, edgeId: "w1" },
      { from: "bias", weight: -0.1, edgeId: "bias_to_sum" },
    ]);
    addActivation(graph, "relu", "sum", "relu", {}, "sum_to_relu");
    addOutput(graph, "out", "relu", "prediction", {}, "relu_to_out");

    expect(graph.nodeProperties("x0")).toEqual({
      "nn.op": "input",
      "nn.input": "x0",
    });
    expect(graph.nodeProperties("x1")).toEqual({
      "nn.op": "input",
      "nn.input": "feature",
    });
    expect(graph.nodeProperties("sum")).toEqual({
      "nn.op": "weighted_sum",
    });
    expect(graph.nodeProperties("bias")).toEqual({
      "nn.op": "constant",
      "nn.value": 1,
    });
    expect(graph.nodeProperties("relu")).toEqual({
      "nn.op": "activation",
      "nn.activation": "relu",
    });
    expect(graph.nodeProperties("out")).toEqual({
      "nn.op": "output",
      "nn.output": "prediction",
    });
    expect(graph.edgeProperties("w0")).toEqual({
      "nn.trainable": true,
      weight: 0.25,
    });
  });

  it("offers a chainable NeuralNetwork authoring API", () => {
    const network = createNeuralNetwork("chain")
      .input("x0")
      .input("x1")
      .weightedSum("sum", [
        { from: "x0", weight: 0.5, edgeId: "w0" },
        { from: "x1", weight: 0.5, edgeId: "w1" },
      ])
      .activation("relu", "sum", "relu", {}, "sum_to_relu")
      .output("out", "relu", "prediction", {}, "relu_to_out");

    expect(network.graph.nodes()).toEqual(["x0", "x1", "sum", "relu", "out"]);
    expect(network.graph.topologicalSort()).toEqual([
      "x0",
      "x1",
      "sum",
      "relu",
      "out",
    ]);
  });

  it("authors an XOR network as explicit graph primitives", () => {
    const network = createXorNetwork();

    expect(network.graph.graphProperties()["nn.name"]).toBe("xor");
    expect(network.graph.nodeProperties("bias")).toMatchObject({
      "nn.op": "constant",
      "nn.value": 1,
    });
    expect(network.graph.nodeProperties("h_or")).toMatchObject({
      "nn.op": "activation",
      "nn.activation": "sigmoid",
    });
    expect(network.graph.edgeProperties("bias_to_out")).toMatchObject({
      weight: -30,
    });
    expect(network.graph.topologicalSort()).toContain("out");
  });
});
