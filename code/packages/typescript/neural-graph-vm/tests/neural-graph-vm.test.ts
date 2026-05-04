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
  WebGpuMatrixBackend,
  runNeuralMatrixForwardScalars,
  type AsyncNeuralMatrixBackend,
  type MatrixBackend,
  type WebGpuLike,
} from "../src/index.js";

const FLOAT_BYTES = Float32Array.BYTES_PER_ELEMENT;

class FakeGpuBuffer {
  readonly bytes: Uint8Array;
  destroyed = false;

  constructor(size: number) {
    this.bytes = new Uint8Array(size);
  }

  mapAsync(): Promise<void> {
    return Promise.resolve();
  }

  getMappedRange(offset = 0, size = this.bytes.byteLength - offset): Uint8Array {
    return this.bytes.slice(offset, offset + size);
  }

  unmap(): void {
    // Fake buffers expose a copied mapped range, so unmap has no work to do.
  }

  destroy(): void {
    this.destroyed = true;
  }
}

interface FakeBindGroup {
  readonly entries: readonly {
    readonly binding: number;
    readonly resource: { readonly buffer: FakeGpuBuffer };
  }[];
}

interface FakePipeline {
  readonly label: string;
}

interface FakeCommandBuffer {
  readonly copies: readonly {
    readonly source: FakeGpuBuffer;
    readonly sourceOffset: number;
    readonly destination: FakeGpuBuffer;
    readonly destinationOffset: number;
    readonly size: number;
  }[];
}

class FakeGpuDevice {
  readonly queue = {
    writeBuffer: (
      buffer: FakeGpuBuffer,
      bufferOffset: number,
      data: ArrayBuffer | ArrayBufferView
    ): void => {
      const bytes = data instanceof ArrayBuffer
        ? new Uint8Array(data)
        : new Uint8Array(data.buffer, data.byteOffset, data.byteLength);
      buffer.bytes.set(bytes, bufferOffset);
    },
    submit: (commands: readonly FakeCommandBuffer[]): void => {
      for (const command of commands) {
        for (const copy of command.copies) {
          bufferCopy(copy.source, copy.sourceOffset, copy.destination, copy.destinationOffset, copy.size);
        }
      }
    },
    onSubmittedWorkDone: (): Promise<void> => Promise.resolve(),
  };

  createBuffer(descriptor: { readonly size: number }): FakeGpuBuffer {
    return new FakeGpuBuffer(descriptor.size);
  }

  createShaderModule(): object {
    return {};
  }

  createBindGroupLayout(): object {
    return {};
  }

  createPipelineLayout(): object {
    return {};
  }

  createComputePipeline(descriptor: { readonly label?: string }): FakePipeline {
    return { label: descriptor.label ?? "" };
  }

  createBindGroup(descriptor: { readonly entries: FakeBindGroup["entries"] }): FakeBindGroup {
    return { entries: descriptor.entries };
  }

  createCommandEncoder(): FakeCommandEncoder {
    return new FakeCommandEncoder();
  }
}

class FakeCommandEncoder {
  private readonly copies: FakeCommandBuffer["copies"][number][] = [];

  beginComputePass(): FakeComputePass {
    return new FakeComputePass();
  }

  copyBufferToBuffer(
    source: FakeGpuBuffer,
    sourceOffset: number,
    destination: FakeGpuBuffer,
    destinationOffset: number,
    size: number
  ): void {
    this.copies.push({ source, sourceOffset, destination, destinationOffset, size });
  }

  finish(): FakeCommandBuffer {
    return { copies: [...this.copies] };
  }
}

class FakeComputePass {
  private pipeline: FakePipeline | undefined;
  private bindGroup: FakeBindGroup | undefined;

  setPipeline(pipeline: FakePipeline): void {
    this.pipeline = pipeline;
  }

  setBindGroup(_index: number, bindGroup: FakeBindGroup): void {
    this.bindGroup = bindGroup;
  }

  dispatchWorkgroups(): void {
    if (this.pipeline === undefined || this.bindGroup === undefined) {
      throw new Error("Fake compute pass was dispatched before setup");
    }
    runFakeKernel(this.pipeline, this.bindGroup);
  }

  end(): void {
    // The fake pass computes during dispatch so tests can stay synchronous.
  }
}

class FakeGpuAdapter {
  constructor(private readonly device: FakeGpuDevice) {}

  requestDevice(): FakeGpuDevice {
    return this.device;
  }
}

class FakeGpu implements WebGpuLike {
  readonly device = new FakeGpuDevice();

  requestAdapter(): FakeGpuAdapter {
    return new FakeGpuAdapter(this.device);
  }
}

function bufferCopy(
  source: FakeGpuBuffer,
  sourceOffset: number,
  destination: FakeGpuBuffer,
  destinationOffset: number,
  size: number
): void {
  destination.bytes.set(
    source.bytes.slice(sourceOffset, sourceOffset + size),
    destinationOffset
  );
}

function runFakeKernel(pipeline: FakePipeline, bindGroup: FakeBindGroup): void {
  const buffers = new Map(
    bindGroup.entries.map((entry) => [entry.binding, entry.resource.buffer])
  );
  const input = readFloatBuffer(requireFakeBuffer(buffers, 0));
  const parameter = requireFakeBuffer(buffers, 1);
  const outputBuffer = requireFakeBuffer(buffers, 2);
  const output = new Float32Array(outputBuffer.bytes.buffer);

  if (pipeline.label.includes("add")) {
    const right = readFloatBuffer(parameter);
    for (let index = 0; index < output.length; index += 1) {
      output[index] = (input[index] ?? 0) + (right[index] ?? 0);
    }
    return;
  }

  if (pipeline.label.includes("scale")) {
    const scalar = readFloatBuffer(parameter)[0] ?? 0;
    for (let index = 0; index < output.length; index += 1) {
      output[index] = (input[index] ?? 0) * scalar;
    }
    return;
  }

  if (pipeline.label.includes("activation")) {
    const activation = new Uint32Array(parameter.bytes.buffer)[0] ?? 0;
    for (let index = 0; index < output.length; index += 1) {
      output[index] = fakeActivation(input[index] ?? 0, activation);
    }
    return;
  }

  throw new Error(`Unknown fake WebGPU pipeline: ${pipeline.label}`);
}

function requireFakeBuffer(
  buffers: ReadonlyMap<number, FakeGpuBuffer>,
  binding: number
): FakeGpuBuffer {
  const buffer = buffers.get(binding);
  if (buffer === undefined) {
    throw new Error(`Missing fake WebGPU binding ${binding}`);
  }
  return buffer;
}

function readFloatBuffer(buffer: FakeGpuBuffer): Float32Array {
  return new Float32Array(buffer.bytes.buffer, 0, buffer.bytes.byteLength / FLOAT_BYTES);
}

function fakeActivation(value: number, activation: number): number {
  switch (activation) {
    case 1:
      return Math.max(value, 0);
    case 2:
      return 1 / (1 + Math.exp(-value));
    case 3: {
      const exponent = Math.exp(Math.min(40, Math.max(-40, 2 * value)));
      return (exponent - 1) / (exponent + 1);
    }
    default:
      return value;
  }
}

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

  it("runs matrix plans through the WebGPU backend contract", async () => {
    const backend = await WebGpuMatrixBackend.create(new FakeGpu());
    const bytecode = compileNeuralGraphToBytecode(makeTinyWeightedSumGraph());
    const plan = compileBytecodeToMatrixPlan(bytecode);
    const result = await runNeuralMatrixForwardAsync(
      plan,
      { x0: [4, 8], x1: [8, 16] },
      backend
    );

    expect(result.outputs).toEqual({ prediction: [6, 13] });
    expect(result.values).toMatchObject({
      v0: [1, 1],
      v1: [4, 8],
      v2: [8, 16],
    });
  });

  it("runs sigmoid networks and row conversion through the WebGPU backend", async () => {
    const backend = await WebGpuMatrixBackend.create(new FakeGpu());
    const rows = await backend.toRows(await backend.fromRows([
      [1, 2],
      [3, 4],
    ]));
    const bytecode = compileNeuralNetworkToBytecode(createXorNetwork());
    const plan = compileBytecodeToMatrixPlan(bytecode);
    const result = await runNeuralMatrixForwardAsync(
      plan,
      { x0: [0, 0, 1, 1], x1: [0, 1, 0, 1] },
      backend
    );

    expect(rows).toEqual([
      [1, 2],
      [3, 4],
    ]);
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
