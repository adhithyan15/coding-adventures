/** Suites, chunk files and chunk enumeration (VER04r sections 5.2 to 5.6, 14). */
import { createHash } from "node:crypto";
import { describe, expect, it } from "vitest";
import {
  checkSlimChunks, enumerateChunk, enumerateChunks, framedDigest, loadSuite, readChunkFile, readCorpusPart, streamContext,
} from "../../src/v2/chunks.ts";
import { KitError } from "../../src/v2/errors.ts";
import { fromHex } from "../../src/v2/values.ts";
import { kitCase } from "./support.ts";

type Json = any; // eslint-disable-line @typescript-eslint/no-explicit-any
const LEVELS = new Set(["ir.core"]);
const clone = <X>(value: X): X => JSON.parse(JSON.stringify(value)) as X;
const code = (run: () => unknown): string => {
  try {
    run();
  } catch (error) {
    if (error instanceof KitError) return error.code;
    throw error;
  }
  return "none";
};
const ragged = kitCase("v2-chunk/ragged-short-last-chunk");
const gap = kitCase("v2-chunk/gap-chunk-with-zero-admitted-cases");
const predicate = kitCase("v2-chunk/short-predicate-chunk");
const loadRagged = (edit: (c: Json) => void = () => undefined): ReturnType<typeof loadSuite> => {
  const c = clone(ragged);
  edit(c);
  return loadSuite(readCorpusPart(c.corpus, LEVELS), c.suite);
};

describe("the corpus part", () => {
  const read = (edit: (corpus: Json) => void): string => {
    const corpus = clone(ragged.corpus);
    edit(corpus);
    return code(() => readCorpusPart(corpus, LEVELS));
  };
  it("accepts the kit's and refuses a malformed one with `corpus`", () => {
    expect(read(() => undefined)).toBe("none");
    expect(code(() => readCorpusPart(null, LEVELS))).toBe("corpus");
    expect(code(() => readCorpusPart([], LEVELS))).toBe("corpus");
    expect(read((c) => delete c.types)).toBe("corpus");
    expect(read((c) => (c.extra = 1))).toBe("corpus");
    for (const size of [1000, 2000, 512, 131072, "1024", -1024, 1.5, 0]) expect(read((c) => (c.chunk_size = size)), String(size)).toBe("corpus");
    for (const cost of [1999, 65537, "2000", 0]) expect(read((c) => (c.max_ir_cost = cost)), String(cost)).toBe("corpus");
    expect(read((c) => (c.models = [])), "models").toBe("corpus");
    // a refusal found while building the program is mapped to the `corpus` code
    expect(read((c) => (c.models = { m: { params: [], body: ["frobnicate"] } }))).toBe("corpus");
    expect(read((c) => (c.model_order = ["ghost"]))).toBe("corpus");
  });
});

describe("a suite row", () => {
  const refuse = (edit: (suite: Json) => void): string => code(() => loadRagged((c) => edit(c.suite)));

  it("loads, with its bounds, from the kit", () => {
    const suite = loadRagged();
    expect(suite).toMatchObject({ chunks: 17, product: 16640, reference: true, stream: "warrant-stream-v1" });
    expect(suite.binders.length).toBeGreaterThan(0);
    expect(suite.slimChunks.length).toBeGreaterThan(0);
    expect(loadSuite(readCorpusPart(predicate.corpus, LEVELS), predicate.suite).reference).toBe(false);
    expect(loadSuite(readCorpusPart(gap.corpus, LEVELS), gap.suite).admitAlways).toBe(false);
  });

  it("refuses every violation of the closed row with `corpus`", () => {
    expect(code(() => loadSuite(readCorpusPart(ragged.corpus, LEVELS), null))).toBe("corpus");
    expect(refuse((s) => delete s.id)).toBe("corpus");
    expect(refuse((s) => (s.requires = ["ir.fancy"]))).toBe("corpus");
    expect(refuse((s) => (s.requires = "ir.core"))).toBe("corpus");
    expect(refuse((s) => (s.extra = 1))).toBe("corpus");
    expect(refuse((s) => delete s.expected_sha256)).toBe("corpus");
    expect(refuse((s) => delete s.v1_inputs_sha256)).toBe("corpus");
    expect(refuse((s) => (s.oracle = 5))).toBe("corpus");
    expect(refuse((s) => (s.oracle.kind = "other"))).toBe("corpus");
    expect(refuse((s) => (s.oracle.extra = 1))).toBe("corpus");
    expect(refuse((s) => (s.stream = "warrant-stream-v3"))).toBe("corpus");
    expect(refuse((s) => (s.stream = "warrant-stream-v2"))).toBe("corpus");
    expect(refuse((s) => (s.id = 7))).toBe("corpus");
    expect(refuse((s) => (s.claim = "bad name"))).toBe("corpus");
    expect(refuse((s) => (s.claim = "a/b/c"))).toBe("corpus");
    expect(refuse((s) => (s.kind = "random"))).toBe("corpus");
    expect(refuse((s) => (s.cases = "1"))).toBe("corpus");
    expect(refuse((s) => (s.cases = -1))).toBe("corpus");
    expect(refuse((s) => (s.chunks_sha256 = "ABC"))).toBe("corpus");
    expect(refuse((s) => (s.inputs_sha256 = 5))).toBe("corpus");
    expect(refuse((s) => (s.product += 1))).toBe("corpus");
    expect(refuse((s) => (s.chunks += 1))).toBe("corpus");
    expect(refuse((s) => (s.ir_cost += 1))).toBe("corpus");
    expect(refuse((s) => (s.result_type = "Bytes<"))).toBe("corpus");
    expect(refuse((s) => (s.result_type = "Nope"))).toBe("corpus");
    expect(refuse((s) => (s.result_type = "Bool"))).toBe("corpus");
    expect(refuse((s) => (s.admit = ["lit", { type: "U8", value: "1" }]))).toBe("corpus");
    expect(refuse((s) => (s.body = ["lit", { type: "U8", value: "1" }]))).toBe("corpus");
    expect(refuse((s) => (s.oracle.expected = ["frobnicate"]))).toBe("corpus");
    expect(refuse((s) => (s.oracle.subject = ["frobnicate"]))).toBe("corpus");
    expect(refuse((s) => (s.body = ["subject_op", "ghost", []]))).toBe("corpus");
    // a reference suite's expected must fit its result type; a predicate suite's result type is Bool
    expect(refuse((s) => (s.oracle.expected = ["lit", true]))).toBe("corpus");
  });

  it("refuses binders that break the bounds with `corpus`", () => {
    expect(refuse((s) => (s.binders = 5))).toBe("corpus");
    expect(refuse((s) => (s.binders = Array.from({ length: 9 }, () => s.binders[0])))).toBe("corpus");
    expect(refuse((s) => (s.binders = [5]))).toBe("corpus");
    expect(refuse((s) => (s.binders = [["a", 5]]))).toBe("corpus");
    expect(refuse((s) => (s.binders[0][1].type = 5))).toBe("corpus");
    expect(refuse((s) => (s.binders[0][1].extra = 1))).toBe("corpus");
    expect(refuse((s) => (s.binders[0][1].type = "U8"))).toBe("corpus");
    expect(refuse((s) => (s.binders[0][1].type = "Bytes<"))).toBe("corpus");
    expect(refuse((s) => (s.binders[0][1].type = "Nope"))).toBe("corpus");
    expect(refuse((s) => (s.binders[0][1].domain.size += 1))).toBe("corpus");
    expect(refuse((s) => (s.binders = [s.binders[0], s.binders[0], s.binders[0], s.binders[0], s.binders[0], s.binders[0]]))).toBe("corpus");
  });

  it("names a level this lane lacks when a binder or a result type needs one", () => {
    const level = (edit: (suite: Json) => void): string | undefined => {
      try {
        loadRagged((c) => edit(c.suite));
      } catch (error) {
        return (error as KitError).code === "level-missing" ? (error as KitError).level : "other";
      }
      return undefined;
    };
    expect(level((s) => (s.binders[0][1] = { type: "(U8,U8)", domain: { d: "tuple", of: [], size: 1 } }))).toBe("ir.tuple");
    expect(level((s) => (s.result_type = "(U8,U8)"))).toBe("ir.tuple");
    expect(level((s) => (s.body = ["bit_not", ["lit", true]]))).toBe("ir.fixed");
  });

  it("checks that errs equals the declared cases (R72)", () => {
    const res = kitCase("v2-chunk/corpus-errs-in-another-order");
    expect(code(() => loadSuite(readCorpusPart(res.corpus, LEVELS), res.suite))).toBe("corpus");
  });

  it("accepts a ground suite and refuses the product past 2^28 (R75)", () => {
    const ground = kitCase("v2-chunk/ground-suite-no-binder");
    expect(loadSuite(readCorpusPart(ground.corpus, LEVELS), ground.suite).product).toBe(1);
    const big = kitCase("v2-chunk/corpus-product-over-2-28");
    expect(code(() => loadSuite(readCorpusPart(big.corpus, LEVELS), big.suite))).toBe("corpus");
  });
});

describe("slim_chunks", () => {
  it("is ascending, duplicate-free, below m, holds chunk 0 and m-1 and at most 32 entries", () => {
    expect(code(() => checkSlimChunks([0, 3, 16], 17))).toBe("none");
    expect(code(() => checkSlimChunks([0], 1))).toBe("none");
    for (const bad of [[], "0", null, [3, 16], [0, 3], [0, 3, 3, 16], [0, 5, 3, 16], [0, 17], [0, 1.5, 16], [0, -1, 16], [0, "1", 16], Array.from({ length: 33 }, (_u, i) => i)]) {
      expect(code(() => checkSlimChunks(bad, 17)), JSON.stringify(bad)).toBe("slim-chunks");
    }
  });
});

describe("the chunk file", () => {
  const suite = loadRagged();
  const file = fromHex(ragged.chunk_file_hex) as Uint8Array;
  const text = new TextDecoder().decode(file);
  const read = (edit: (lines: string[]) => string): string => code(() => readChunkFile(new TextEncoder().encode(edit(text.slice(0, -1).split("\n"))), suite));

  it("parses and cross-checks the committed file", () => {
    const rows = readChunkFile(file, suite);
    expect(rows).toHaveLength(17);
    expect(rows[16]?.firstAdmitted).toBe(16384);
    expect(rows.reduce((sum, r) => sum + r.cases, 0)).toBe(suite.cases);
  });

  it("refuses every violation of the line grammar and the cross-checks with `chunk-file`", () => {
    expect(read((l) => `${l.join("\n")}\n`)).toBe("none");
    expect(code(() => readChunkFile(new Uint8Array(0), suite))).toBe("chunk-file");
    expect(code(() => readChunkFile(new Uint8Array(2 * 1024 * 1024 + 1), suite))).toBe("chunk-file");
    expect(code(() => readChunkFile(Uint8Array.of(0xc3, 0xa9), suite))).toBe("chunk-file");
    expect(read((l) => l.join("\n"))).toBe("chunk-file");
    expect(read((l) => `${l.join("\r\n")}\r\n`)).toBe("chunk-file");
    expect(read((l) => `${l.slice(1).join("\n")}\n`)).toBe("chunk-file");
    expect(read((l) => `${[...l, l[0]].join("\n")}\n`)).toBe("chunk-file");
    expect(read((l) => `${l.map((x, i) => (i === 2 ? x.replace(/^2 /, "3 ") : x)).join("\n")}\n`)).toBe("chunk-file");
    expect(read((l) => `${l.map((x, i) => (i === 2 ? x.replace(" ", "  ") : x)).join("\n")}\n`)).toBe("chunk-file");
    expect(read((l) => `${l.map((x, i) => (i === 2 ? x.replace(/^2 (\d+)/, "2 0$1") : x)).join("\n")}\n`)).toBe("chunk-file");
    expect(read((l) => `${l.map((x, i) => (i === 2 ? x.replace(/ [0-9a-f]{64}$/, "") : x)).join("\n")}\n`)).toBe("chunk-file");
    expect(read((l) => `${l.map((x, i) => (i === 2 ? x.toUpperCase() : x)).join("\n")}\n`)).toBe("chunk-file");
    expect(read((l) => `${l.map((x, i) => (i === 2 ? `${x}0` : x)).join("\n")}\n`)).toBe("chunk-file");
    expect(read((l) => `${l.map((x, i) => (i === 2 ? x.replace(/ [0-9a-f]{64} /, " 0 ") : x)).join("\n")}\n`)).toBe("chunk-file");
    // the sums, the digests and the discarded count
    expect(code(() => readChunkFile(file, { ...suite, cases: suite.cases + 1 }))).toBe("chunk-file");
    expect(code(() => readChunkFile(file, { ...suite, discarded: suite.discarded + 1 }))).toBe("chunk-file");
    expect(code(() => readChunkFile(file, { ...suite, expectedCalls: suite.expectedCalls + 1 }))).toBe("chunk-file");
    expect(code(() => readChunkFile(file, { ...suite, chunksSha256: "0".repeat(64) }))).toBe("chunk-file");
    expect(code(() => readChunkFile(file, { ...suite, inputsSha256: "0".repeat(64) }))).toBe("chunk-file");
    expect(code(() => readChunkFile(file, { ...suite, expectedSha256: "0".repeat(64) }))).toBe("chunk-file");
  });

  it("frames the suite digest over the listed chunk digests", () => {
    const rows = readChunkFile(file, suite);
    const expected = createHash("sha256")
      .update("warrant-chunks-v1\0")
      .update(Buffer.from([17, 0, 0, 0]))
      .update(Buffer.concat(rows.map((r) => Buffer.from(r.inputs, "hex"))))
      .digest("hex");
    expect(framedDigest(rows.map((r) => r.inputs))).toBe(expected);
    expect(expected).toBe(suite.inputsSha256);
  });
});

describe("enumeration", () => {
  const suite = loadRagged();
  const file = fromHex(ragged.chunk_file_hex) as Uint8Array;
  const rows = readChunkFile(file, suite);

  it("chunk c alone equals the sequential walk, and the file's line", () => {
    for (const c of [0, 1, 5, 15, 16]) {
      const alone = enumerateChunk(suite, c);
      const walked = enumerateChunks(suite, 0, c).results[c];
      expect(walked).toEqual(alone);
      expect(alone).toMatchObject({ cases: rows[c]?.cases, calls: rows[c]?.calls, inputs: rows[c]?.inputs, expected: rows[c]?.expected });
    }
  });

  it("reproduces the whole-suite v1 digests of the unchunked stream", () => {
    const walked = enumerateChunks(suite, 0, suite.chunks - 1, { whole: true });
    expect(walked.whole).toEqual({ inputs: suite.v1InputsSha256, expected: suite.v1ExpectedSha256 });
    expect(walked.results.reduce((sum, r) => sum + r.cases, 0)).toBe(suite.cases);
    expect(enumerateChunks(suite, 0, 0).whole).toBeUndefined();
  });

  it("counts the calls of a predicate suite under guards and walks a ground suite", () => {
    const p = loadSuite(readCorpusPart(predicate.corpus, LEVELS), predicate.suite);
    const result = enumerateChunk(p, 0);
    expect(result.expected).toBeUndefined();
    expect(result).toMatchObject({ cases: 65280, calls: 64770 });
    const ground = kitCase("v2-chunk/ground-suite-no-binder");
    const g = loadSuite(readCorpusPart(ground.corpus, LEVELS), ground.suite);
    expect(enumerateChunk(g, 0)).toMatchObject({ cases: 1, calls: 1 });
    const whole = enumerateChunks(p, 0, 0, { whole: true }).whole;
    expect(whole?.expected).toBeUndefined();
    expect(whole?.inputs).toBe(p.v1InputsSha256);
  });

  it("starts its stream with the VER04i context", () => {
    const context = streamContext(suite);
    expect(new TextDecoder().decode(context.subarray(0, 18))).toBe("warrant-stream-v1\0");
    expect(context.length).toBeGreaterThan(30);
  });
});
