import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { DenseNetwork, type Activation } from "../src/index.js";

const fixture = JSON.parse(readFileSync(new URL(
  "../../../../specs/fixtures/neural-framework-v1/dense.json", import.meta.url,
), "utf8"));
function close(actual: any, expected: any, tolerance = 1e-12): void {
  if (typeof expected === "number") expect(Math.abs(actual - expected)).toBeLessThanOrEqual(tolerance);
  else if (Array.isArray(expected)) {
    expect(actual).toHaveLength(expected.length);
    expected.forEach((v, i) => close(actual[i], v, tolerance));
  } else if (expected !== null && typeof expected === "object") {
    expect(Object.keys(actual).sort()).toEqual(Object.keys(expected).sort());
    Object.keys(expected).forEach(key => close(actual[key], expected[key], tolerance));
  } else expect(actual).toEqual(expected);
}
const paperModel = () => DenseNetwork.fromCheckpoint(fixture.paper.checkpoint);

describe("NN37 dense framework", () => {
  it("matches every hand-computed gradient, updated parameter, and prediction", () => {
    const c = fixture.paper, model = paperModel();
    expect(fixture.absolute_tolerance).toBe(1e-12);
    const trace = model.gradients(c.inputs, c.targets);
    close(trace.predictions, c.predictions);
    close(trace.loss, c.loss);
    close(trace.weightGradients, c.weight_gradients);
    close(trace.biasGradients, c.bias_gradients);
    expect(model.step).toBe(0);
    expect(model.trainStep(c.inputs, c.targets, c.learning_rate)).toEqual(trace);
    close(model.toCheckpoint(), c.updated_checkpoint);
    close(model.predict(c.inputs), c.updated_predictions);
    close(model.gradients(c.inputs, c.targets).loss, c.updated_loss);
  });
  it("pins the language-neutral random stream", () => {
    const c = fixture.seed;
    const model = DenseNetwork.seeded(c.widths, c.activations, c.value);
    close(model.toCheckpoint().layers[0].weights, c.first_weights);
    expect(model.toCheckpoint().layers[0].biases).toEqual([0, 0]);
  });
  it.each<Activation>(["linear", "sigmoid", "tanh", "relu"])("checks every %s gradient numerically", kind => {
    const doc = DenseNetwork.seeded([2, 3, 2, 2], [kind, "tanh", "linear"], 42).toCheckpoint();
    if (kind === "relu") doc.layers[0].biases = [0.7, -0.6, 0.8];
    const model = DenseNetwork.fromCheckpoint(doc);
    const inputs = [[0.2, -0.4], [0.6, 0.3]], targets = [[0.3, -0.5], [-0.1, 0.7]];
    const trace = model.gradients(inputs, targets), epsilon = 1e-6;
    doc.layers.forEach((layer, index) => {
      for (const field of ["weights", "biases"] as const) {
        const rows = field === "weights" ? layer.weights : [layer.biases];
        rows.forEach((row, r) => row.forEach((_, c) => {
          const losses = [-1, 1].map(direction => {
            const altered = structuredClone(doc), changed = altered.layers[index];
            const values = field === "weights" ? changed.weights[r] : changed.biases;
            values[c] += direction * epsilon;
            return DenseNetwork.fromCheckpoint(altered).gradients(inputs, targets).loss;
          });
          const analytical = field === "weights" ? trace.weightGradients[index][r][c] : trace.biasGradients[index][c];
          expect(Math.abs((losses[1] - losses[0]) / (2 * epsilon) - analytical)).toBeLessThan(1e-8);
        }));
      }
    });
  });
  it("defines ReLU's zero derivative and handles sigmoid extremes", () => {
    const zero = new DenseNetwork([{ weights: [[1]], biases: [0], activation: "relu" }]).gradients([[0]], [[1]]);
    expect(zero.weightGradients).toEqual([[[0]]]);
    expect(zero.biasGradients).toEqual([[0]]);
    const model = new DenseNetwork([{ weights: [[1]], biases: [0], activation: "sigmoid" }]);
    expect(model.predict([[-1000], [1000]])).toEqual([[0], [1]]);
  });
  it("learns XOR from random initialization and resumes a JSON checkpoint", () => {
    const c = fixture.xor, model = DenseNetwork.seeded(c.widths, c.activations, c.seed);
    const losses = model.fit(c.inputs, c.targets, c.epochs, c.learning_rate);
    expect(losses).toHaveLength(c.epochs + 1);
    expect(model.step).toBe(c.epochs);
    expect(losses.at(-1)).toBeLessThan(c.maximum_final_loss);
    expect(losses[0]).toBeGreaterThan(c.maximum_final_loss);
    expect(model.predict(c.inputs).map(row => Number(row[0] >= 0.5))).toEqual([0, 1, 1, 0]);
    const restored = DenseNetwork.fromCheckpoint(JSON.parse(JSON.stringify(model.toCheckpoint())));
    close(restored.predict(c.inputs), model.predict(c.inputs));
    model.fit(c.inputs, c.targets, 2, c.learning_rate);
    restored.fit(c.inputs, c.targets, 2, c.learning_rate);
    expect(restored.toCheckpoint()).toEqual(model.toCheckpoint());
  });
  it("performs zero and one updates exactly and never aliases inputs or outputs", () => {
    const c = fixture.paper, doc = structuredClone(c.checkpoint);
    const model = DenseNetwork.fromCheckpoint(doc);
    doc.layers[0].weights[0][0] = 999;
    const before = model.toCheckpoint();
    expect(model.fit(c.inputs, c.targets, 0, 0.01)).toEqual([c.loss]);
    expect(model.toCheckpoint()).toEqual(before);
    close(model.fit(c.inputs, c.targets, 1, 0.01), [c.loss, c.updated_loss]);
    expect(model.step).toBe(1);
    const exported = model.toCheckpoint();
    exported.layers[0].biases[0] = 999;
    const trace = model.gradients(c.inputs, c.targets);
    trace.predictions[0][0] = 999;
    trace.weightGradients[0][0][0] = 999;
    close(model.toCheckpoint(), c.updated_checkpoint);
  });
  it.each([true, "1", null, NaN, Infinity, undefined])("rejects a non-number %s", value => {
    expect(() => new DenseNetwork([{ weights: [[value as never]], biases: [0], activation: "linear" }])).toThrow();
  });
  it.each([
    null, [], [null], [{ weights: [], biases: [0], activation: "linear" }],
    [{ weights: [[1], [1, 2]], biases: [0], activation: "linear" }],
    [{ weights: [[1]], biases: [], activation: "linear" }],
    [{ weights: [[1, 2]], biases: [0], activation: "linear" }],
    [{ weights: [[1]], biases: [0], activation: "softmax" }],
    [{ weights: [[1]], biases: [0], activation: "linear" }, { weights: [[1], [2]], biases: [0], activation: "linear" }],
  ].map(layers => ({ layers })))("rejects invalid layers %#", ({ layers }) => {
    expect(() => new DenseNetwork(layers as never)).toThrow();
  });
  it.each([
    [[], [], 7], [[1], [], 7], [[1, 0], ["linear"], 7], [[true, 1], ["linear"], 7],
    [[1, 1], [], 7], [[1, 1], ["bad"], 7], [[1, 1], ["linear"], -1],
    [[1, 1], ["linear"], 2 ** 32], [[1, 1], ["linear"], 0.5],
  ])("rejects invalid initialization %#", (widths, kinds, seed) => {
    expect(() => DenseNetwork.seeded(widths as never, kinds as never, seed as number)).toThrow();
  });
  it.each([
    [[], [[1]]], [[[1]], [[1]]], [[[1, 2]], []], [[[1, 2]], [[1, 2]]],
    [[[1, 2]], [[1], [2]]], [[[true, 2]], [[1]]], [[[1, 2]], [[NaN]]], [[[1e308, 1e308]], [[1]]],
  ])("rejects invalid batches %#", (inputs, targets) => {
    expect(() => paperModel().gradients(inputs as never, targets)).toThrow();
  });
  it.each([0, -1, true, null, NaN, Infinity, 1e308])("refuses a failed update atomically %#", rate => {
    const model = paperModel(), before = model.toCheckpoint();
    expect(() => model.trainStep(fixture.paper.inputs, fixture.paper.targets, rate as never)).toThrow();
    expect(model.toCheckpoint()).toEqual(before);
  });
  it.each([[-1, 1], [true, 1], [0.5, 1], [0, 0]])("rejects invalid fit controls %#", (epochs, rate) => {
    expect(() => paperModel().fit([[1, 2]], [[1]], epochs as never, rate)).toThrow();
  });
  it.each([
    { extra: 1 }, { version: true }, { version: 2 }, { format: "pickle" },
    { step: true }, { step: -1 }, { step: 2 ** 53 }, { layers: null }, { layers: [] },
  ])("rejects invalid checkpoint metadata %#", patch => {
    expect(() => DenseNetwork.fromCheckpoint({ ...fixture.paper.checkpoint, ...patch })).toThrow();
  });
  it("rejects malformed, sparse, accessor, and extra-field checkpoint data", () => {
    for (const value of [null, [], {}, "checkpoint"]) expect(() => DenseNetwork.fromCheckpoint(value)).toThrow();
    const doc = structuredClone(fixture.paper.checkpoint);
    doc.layers[0].extra = 1;
    expect(() => DenseNetwork.fromCheckpoint(doc)).toThrow();
    doc.layers[0] = null;
    expect(() => DenseNetwork.fromCheckpoint(doc)).toThrow();
    const accessor = { ...fixture.paper.checkpoint };
    Object.defineProperty(accessor, "step", { get: () => { throw new Error("must not execute"); } });
    expect(() => DenseNetwork.fromCheckpoint(accessor)).toThrow("data fields");
    expect(() => DenseNetwork.seeded(Array(2), ["linear"])).toThrow();
    expect(() => DenseNetwork.seeded([1, 1], Array(1))).toThrow();
    expect(() => new DenseNetwork([{ weights: [Array(1)], biases: [0], activation: "linear" }])).toThrow();
    const maximum = new DenseNetwork([{ weights: [[1]], biases: [0], activation: "linear" }], Number.MAX_SAFE_INTEGER);
    expect(() => maximum.trainStep([[1]], [[0]], 0.1)).toThrow();
    expect(maximum.step).toBe(Number.MAX_SAFE_INTEGER);
  });
  it("rejects overflowing candidate execution even when candidate weights are finite", () => {
    const model = new DenseNetwork([{ weights: [[1e100]], biases: [0], activation: "linear" }]);
    const before = model.toCheckpoint();
    expect(() => model.trainStep([[1e100]], [[0]], 1)).toThrow();
    expect(model.toCheckpoint()).toEqual(before);
  });
  it("rejects overflowing gradients even when the forward loss is finite", () => {
    const model = new DenseNetwork([{ weights: [[1e-308]], biases: [0], activation: "linear" }]);
    expect(() => model.gradients([[1e308]], [[0]])).toThrow();
  });
});
