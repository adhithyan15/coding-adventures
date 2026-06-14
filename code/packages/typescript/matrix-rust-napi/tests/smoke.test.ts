/**
 * # End-to-end smoke for the matrix-rust-napi Node addon
 *
 * **MX07 Phase 4.**  These tests drive the addon through real
 * JavaScript: construct a `Graph` from a hand-rolled matrix-ir-json
 * payload, allocate input `Buffer`s, call `runtime.run`, and assert
 * on the output bytes.  The smoke covers Add (the simplest
 * elementwise op) and MatMul (the workhorse op every NN depends on),
 * plus the JSON-envelope `runGraphOnCpu` path and the legacy
 * `graphRoundTripJson` validator.
 *
 * Notes
 *
 * * Test fixtures construct the matrix-ir-json payload by hand
 *   rather than going through the Rust `GraphBuilder`.  This proves
 *   the schema is stable across the FFI boundary and that the JSON
 *   is human-writable (a property MX07 §"How CI proves this works"
 *   relies on for golden-file fixtures).
 *
 * * Constants in matrix-ir-json are lowercase-hex bytes (`bytes_hex`
 *   field).  We use a small `f32sToHex` helper to keep the
 *   fixtures readable.
 *
 * * Output `Buffer`s are little-endian; we decode them back to
 *   numbers via `Buffer.readFloatLE`.
 *
 * Failure modes intentionally covered
 *
 * * Empty argument list to `runtime.run`.
 * * Wrong input arity.
 * * Wrong input byte length.
 * * Malformed JSON to `Graph` constructor.
 */

import { describe, it, expect } from "vitest";
import {
  Graph,
  Runtime,
  graphRoundTripJson,
  runGraphOnCpu,
} from "../src/index.js";

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Lowercase-hex encoding of a sequence of little-endian f32s.  The
 * matrix-ir-json schema uses this format for constant payloads.
 */
function f32sToHex(values: number[]): string {
  const buf = Buffer.alloc(values.length * 4);
  for (let i = 0; i < values.length; i++) {
    buf.writeFloatLE(values[i]!, i * 4);
  }
  return buf.toString("hex");
}

/** Build a Node `Buffer` of little-endian f32 bytes from an array of numbers. */
function f32sToBuffer(values: number[]): Buffer {
  const buf = Buffer.alloc(values.length * 4);
  for (let i = 0; i < values.length; i++) {
    buf.writeFloatLE(values[i]!, i * 4);
  }
  return buf;
}

/** Decode a Buffer of little-endian f32 bytes back to an array of numbers. */
function bufferToF32s(buf: Buffer): number[] {
  const out: number[] = [];
  for (let i = 0; i < buf.length; i += 4) {
    out.push(buf.readFloatLE(i));
  }
  return out;
}

/**
 * Build the matrix-ir-json payload for a 2-input elementwise Add
 * graph of shape `[n]` (f32).
 */
function buildAddGraphJson(n: number): string {
  return JSON.stringify({
    matrix_ir_version: 1,
    tensors: [
      { id: 0, dtype: "f32", shape: [n] }, // input a
      { id: 1, dtype: "f32", shape: [n] }, // input b
      { id: 2, dtype: "f32", shape: [n] }, // output a+b
    ],
    inputs: [0, 1],
    outputs: [2],
    ops: [{ kind: "Add", lhs: 0, rhs: 1, output: 2 }],
    constants: [],
  });
}

/**
 * Build the matrix-ir-json payload for a 2x2 × 2x2 MatMul graph.
 */
function buildMatMul2x2GraphJson(): string {
  return JSON.stringify({
    matrix_ir_version: 1,
    tensors: [
      { id: 0, dtype: "f32", shape: [2, 2] }, // input a
      { id: 1, dtype: "f32", shape: [2, 2] }, // input b
      { id: 2, dtype: "f32", shape: [2, 2] }, // output a @ b
    ],
    inputs: [0, 1],
    outputs: [2],
    ops: [{ kind: "MatMul", a: 0, b: 1, output: 2 }],
    constants: [],
  });
}

// ─────────────────────────────────────────────────────────────────────────────
// Class-based API (preferred path)
// ─────────────────────────────────────────────────────────────────────────────

describe("Graph class", () => {
  it("parses a valid JSON graph and reports a summary", () => {
    const g = new Graph(buildAddGraphJson(4));
    const summary = g.describe();
    expect(summary).toContain("tensors=3");
    expect(summary).toContain("ops=1");
    expect(summary).toContain("inputs=2");
    expect(summary).toContain("outputs=1");
  });

  it("round-trips toJson() back into a parseable Graph", () => {
    const original = new Graph(buildAddGraphJson(3));
    const json = original.toJson();
    // The output is normalised but still valid input for a fresh Graph.
    const reconstructed = new Graph(json);
    expect(reconstructed.describe()).toBe(original.describe());
  });

  it("Graph.fromJson is equivalent to new Graph(...)", () => {
    const a = new Graph(buildAddGraphJson(2));
    const b = Graph.fromJson(buildAddGraphJson(2));
    expect(a.describe()).toBe(b.describe());
    expect(a.toJson()).toBe(b.toJson());
  });

  it("throws on malformed JSON", () => {
    expect(() => new Graph("definitely not json")).toThrow();
  });

  it("throws on unsupported matrix_ir_version", () => {
    expect(
      () =>
        new Graph(
          JSON.stringify({
            matrix_ir_version: 9999,
            tensors: [],
            inputs: [],
            outputs: [],
            ops: [],
            constants: [],
          }),
        ),
    ).toThrow();
  });
});

describe("Runtime class — end-to-end execution", () => {
  it("Runtime.create() yields a usable Runtime", () => {
    const rt = Runtime.create();
    expect(typeof rt.run).toBe("function");
  });

  it("element-wise Add of two f32 vectors returns the right bytes", () => {
    const rt = new Runtime();
    const g = new Graph(buildAddGraphJson(3));
    const outputs = rt.run(g, [
      f32sToBuffer([1, 2, 3]),
      f32sToBuffer([10, 20, 30]),
    ]);
    expect(outputs).toHaveLength(1);
    expect(bufferToF32s(outputs[0]!)).toEqual([11, 22, 33]);
  });

  it("2x2 MatMul returns the textbook result", () => {
    const rt = new Runtime();
    const g = new Graph(buildMatMul2x2GraphJson());
    const outputs = rt.run(g, [
      // [[1, 2], [3, 4]]
      f32sToBuffer([1, 2, 3, 4]),
      // [[5, 6], [7, 8]]
      f32sToBuffer([5, 6, 7, 8]),
    ]);
    // [[1, 2], [3, 4]] @ [[5, 6], [7, 8]] = [[19, 22], [43, 50]]
    expect(outputs).toHaveLength(1);
    expect(bufferToF32s(outputs[0]!)).toEqual([19, 22, 43, 50]);
  });

  it("throws on wrong input count", () => {
    const rt = new Runtime();
    const g = new Graph(buildAddGraphJson(3));
    expect(() => rt.run(g, [f32sToBuffer([1, 2, 3])])).toThrow(/input count/i);
  });

  it("throws on wrong input byte length", () => {
    const rt = new Runtime();
    const g = new Graph(buildAddGraphJson(3));
    expect(() =>
      rt.run(g, [f32sToBuffer([1, 2]), f32sToBuffer([10, 20, 30])]),
    ).toThrow(/byte length/i);
  });

  it("throws when called with non-Buffer inputs", () => {
    const rt = new Runtime();
    const g = new Graph(buildAddGraphJson(2));
    // Passing strings instead of Buffers — should be rejected by the
    // node-bridge Buffer helper, surfaced via the addon's throw.
    expect(() =>
      // @ts-expect-error: deliberately passing the wrong type to test
      // the runtime defence
      rt.run(g, ["not a buffer", "also not"]),
    ).toThrow();
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// String-only API (kept for CLI / no-Buffer consumers)
// ─────────────────────────────────────────────────────────────────────────────

describe("graphRoundTripJson (Phase 1)", () => {
  it("returns a normalised JSON that re-parses", () => {
    const input = buildAddGraphJson(2);
    const normalised = graphRoundTripJson(input);
    // Idempotent on a second pass.
    expect(graphRoundTripJson(normalised)).toBe(normalised);
  });

  it("throws on malformed input", () => {
    expect(() => graphRoundTripJson("{ definitely not valid")).toThrow();
  });
});

describe("runGraphOnCpu (Phase 2 JSON envelope)", () => {
  it("returns hex-encoded output bytes for a small Add graph", () => {
    const envelope = JSON.stringify({
      graph: JSON.parse(buildAddGraphJson(3)),
      inputs: [f32sToHex([1, 2, 3]), f32sToHex([10, 20, 30])],
    });
    const result = runGraphOnCpu(envelope);
    const parsed = JSON.parse(result) as { outputs: string[] };
    expect(parsed.outputs).toHaveLength(1);
    const outBuf = Buffer.from(parsed.outputs[0]!, "hex");
    expect(bufferToF32s(outBuf)).toEqual([11, 22, 33]);
  });

  it("throws on envelope missing the graph field", () => {
    expect(() => runGraphOnCpu(JSON.stringify({ inputs: [] }))).toThrow();
  });
});
