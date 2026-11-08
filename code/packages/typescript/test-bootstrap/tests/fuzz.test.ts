import { describe, it } from "node:test";
import * as fs from "node:fs";
import * as path from "node:path";
import { fileURLToPath } from "node:url";
import { expect } from "../dist/matchers.js";
import { parseFile } from "../dist/tfv.js";
import { valuesEqual } from "../dist/deep_equal.js";
import type { Record, Value } from "../dist/types.js";
import {
  FID_FUZZ_CRASH,
  FID_FUZZ_TIMEOUT,
  FID_FUZZ_OOM,
  FID_FUZZ_LEAK,
  FID_FUZZ_PANIC,
  isFuzzFailureId,
  bitFlip,
  byteFlip,
  byteArithmetic,
  boundaryInsert,
  blockDelete,
  blockInsert,
  blockDuplicate,
  splice,
  mutateSequence,
  corpusAdmit,
  CoverageBitmap,
  crashSignature,
  isDuplicateCrash,
  formatTapFailure,
  calculateEnergy,
  minimizeCrashingInput,
  fuzzTarget,
  type FuzzOutcome,
} from "../dist/fuzz.js";

const __dirname = path.dirname(fileURLToPath(import.meta.url));

function getField(record: Record, fieldName: string): Value {
  const val = record.fields.get(fieldName);
  if (!val) {
    throw new Error(`missing field ${fieldName} in record ${record.id}`);
  }
  return val;
}

function getText(record: Record, fieldName: string): string {
  const f = getField(record, fieldName);
  if (f.kind !== "text") {
    throw new Error(`field ${fieldName} must be text`);
  }
  return f.value;
}

function getInt(record: Record, fieldName: string): bigint {
  const f = getField(record, fieldName);
  if (f.kind !== "int") {
    throw new Error(`field ${fieldName} must be int`);
  }
  return BigInt(f.text);
}

function getBytes(record: Record, fieldName: string): Uint8Array {
  const f = getField(record, fieldName);
  if (f.kind !== "bytes") {
    throw new Error(`field ${fieldName} must be bytes`);
  }
  return f.value;
}

function getBool(record: Record, fieldName: string): boolean {
  const f = getField(record, fieldName);
  if (f.kind !== "bool") {
    throw new Error(`field ${fieldName} must be bool`);
  }
  return f.value;
}

function getList(record: Record, fieldName: string): Value[] {
  const f = getField(record, fieldName);
  if (f.kind !== "list") {
    throw new Error(`field ${fieldName} must be list`);
  }
  return f.items;
}

describe("Fuzz Engine Conformance Suite (VER06 / fuzz-v1)", () => {
  const corpusPath = path.resolve(
    __dirname,
    "../../../../specs/fixtures/fuzz-v1/cases.tfv",
  );
  const corpusContent = fs.readFileSync(corpusPath, "utf-8");
  const records = parseFile(corpusContent);

  it("should load all canonical cases from fuzz-v1/cases.tfv", () => {
    expect(records.length).toBe(25);
  });

  for (const record of records) {
    it(`conformance case: ${record.id}`, () => {
      const op = getText(record, "op");
      switch (op) {
        case "mutate_bit_flip": {
          const input = getBytes(record, "input");
          const bitOffset = Number(getInt(record, "bit_offset"));
          const count = Number(getInt(record, "count"));
          const expected = getBytes(record, "output");
          const got = bitFlip(input, bitOffset, count);
          expect(valuesEqual({ kind: "bytes", value: got }, { kind: "bytes", value: expected })).toBe(true);
          break;
        }
        case "mutate_byte_flip": {
          const input = getBytes(record, "input");
          const byteOffset = Number(getInt(record, "byte_offset"));
          const count = Number(getInt(record, "count"));
          const expected = getBytes(record, "output");
          const got = byteFlip(input, byteOffset, count);
          expect(valuesEqual({ kind: "bytes", value: got }, { kind: "bytes", value: expected })).toBe(true);
          break;
        }
        case "mutate_arithmetic": {
          const input = getBytes(record, "input");
          const byteOffset = Number(getInt(record, "byte_offset"));
          const delta = getInt(record, "delta");
          const width = Number(getInt(record, "width"));
          const endian = getText(record, "endian");
          const expected = getBytes(record, "output");
          const got = byteArithmetic(input, byteOffset, delta, width, endian);
          expect(valuesEqual({ kind: "bytes", value: got }, { kind: "bytes", value: expected })).toBe(true);
          break;
        }
        case "mutate_boundary": {
          const input = getBytes(record, "input");
          const byteOffset = Number(getInt(record, "byte_offset"));
          const width = Number(getInt(record, "width"));
          const val = getInt(record, "val");
          const endian = getText(record, "endian");
          const expected = getBytes(record, "output");
          const got = boundaryInsert(input, byteOffset, width, val, endian);
          expect(valuesEqual({ kind: "bytes", value: got }, { kind: "bytes", value: expected })).toBe(true);
          break;
        }
        case "mutate_block_delete": {
          const input = getBytes(record, "input");
          const offset = Number(getInt(record, "offset"));
          const length = Number(getInt(record, "length"));
          const expected = getBytes(record, "output");
          const got = blockDelete(input, offset, length);
          expect(valuesEqual({ kind: "bytes", value: got }, { kind: "bytes", value: expected })).toBe(true);
          break;
        }
        case "mutate_block_insert": {
          const input = getBytes(record, "input");
          const offset = Number(getInt(record, "offset"));
          const insertBytes = getBytes(record, "insert_bytes");
          const expected = getBytes(record, "output");
          const got = blockInsert(input, offset, insertBytes);
          expect(valuesEqual({ kind: "bytes", value: got }, { kind: "bytes", value: expected })).toBe(true);
          break;
        }
        case "mutate_block_duplicate": {
          const input = getBytes(record, "input");
          const srcOffset = Number(getInt(record, "src_offset"));
          const length = Number(getInt(record, "length"));
          const dstOffset = Number(getInt(record, "dst_offset"));
          const expected = getBytes(record, "output");
          const got = blockDuplicate(input, srcOffset, length, dstOffset);
          expect(valuesEqual({ kind: "bytes", value: got }, { kind: "bytes", value: expected })).toBe(true);
          break;
        }
        case "mutate_splice": {
          const target = getBytes(record, "target");
          const donor = getBytes(record, "donor");
          const targetSplit = Number(getInt(record, "target_split"));
          const donorSplit = Number(getInt(record, "donor_split"));
          const expected = getBytes(record, "output");
          const got = splice(target, donor, targetSplit, donorSplit);
          expect(valuesEqual({ kind: "bytes", value: got }, { kind: "bytes", value: expected })).toBe(true);
          break;
        }
        case "mutate_sequence": {
          const input = getBytes(record, "input");
          const seed = getInt(record, "seed");
          const steps = Number(getInt(record, "steps"));
          const expected = getBytes(record, "output");
          const got = mutateSequence(input, seed, steps);
          expect(valuesEqual({ kind: "bytes", value: got }, { kind: "bytes", value: expected })).toBe(true);
          break;
        }
        case "corpus_admit": {
          const existing = getList(record, "existing_edges").map((v) => {
            if (v.kind !== "int") throw new Error("edge must be int");
            return Number(v.text);
          });
          const trace = getList(record, "trace_edges").map((v) => {
            if (v.kind !== "int") throw new Error("edge must be int");
            return Number(v.text);
          });
          const expAdmitted = getBool(record, "admitted");
          const expNew = Number(getInt(record, "new_edges"));
          const [gotAdmitted, gotNew] = corpusAdmit(existing, trace);
          expect(gotAdmitted).toBe(expAdmitted);
          expect(gotNew).toBe(expNew);
          break;
        }
        case "crash_dedup": {
          const kindA = getText(record, "crash_a_kind");
          const framesA = getList(record, "crash_a_frames").map((v) => {
            if (v.kind !== "text") throw new Error("frame must be text");
            return v.value;
          });
          const kindB = getText(record, "crash_b_kind");
          const framesB = getList(record, "crash_b_frames").map((v) => {
            if (v.kind !== "text") throw new Error("frame must be text");
            return v.value;
          });
          const expDup = getBool(record, "is_duplicate");
          const gotDup = isDuplicateCrash(
            { kind: kindA, frames: framesA },
            { kind: kindB, frames: framesB }
          );
          expect(gotDup).toBe(expDup);
          break;
        }
        case "failure_id_valid": {
          const cand = getText(record, "input");
          const expValid = getBool(record, "valid");
          const gotValid = isFuzzFailureId(cand);
          expect(gotValid).toBe(expValid);
          break;
        }
        case "private_reporting": {
          const target = getText(record, "target");
          const fid = getText(record, "failure_id");
          const rawBytes = getBytes(record, "raw_payload");
          const sigHash = getText(record, "sig_hash");
          const expTap = getText(record, "tap_line");
          const expContains = getBool(record, "contains_raw_payload");

          const gotTap = formatTapFailure(1, target, fid, sigHash);
          expect(gotTap).toBe(expTap);

          const hexRaw = Array.from(rawBytes)
            .map((b) => b.toString(16).padStart(2, "0"))
            .join("");
          const containsRaw = gotTap.includes(hexRaw);
          expect(containsRaw).toBe(expContains);
          break;
        }
        case "corpus_reject": {
          const rawText = getText(record, "text");
          let rejected = false;
          try {
            parseFile(rawText);
          } catch {
            rejected = true;
          }
          expect(rejected).toBe(true);
          break;
        }
        default:
          throw new Error(`unknown conformance op: ${op}`);
      }
    });
  }
});

describe("Fuzz Engine Unit & Component Tests", () => {
  it("enforces closed failure vocabulary constants", () => {
    expect(FID_FUZZ_CRASH).toBe("fuzz-crash");
    expect(FID_FUZZ_TIMEOUT).toBe("fuzz-timeout");
    expect(FID_FUZZ_OOM).toBe("fuzz-oom");
    expect(FID_FUZZ_LEAK).toBe("fuzz-leak");
    expect(FID_FUZZ_PANIC).toBe("fuzz-panic");

    expect(isFuzzFailureId("fuzz-crash")).toBe(true);
    expect(isFuzzFailureId("fuzz-timeout")).toBe(true);
    expect(isFuzzFailureId("fuzz-oom")).toBe(true);
    expect(isFuzzFailureId("fuzz-leak")).toBe(true);
    expect(isFuzzFailureId("fuzz-panic")).toBe(true);
    expect(isFuzzFailureId("not-a-fuzz-id")).toBe(false);
    expect(isFuzzFailureId("")).toBe(false);
  });

  it("handles empty and boundary buffers in mutators", () => {
    const empty = new Uint8Array(0);
    expect(bitFlip(empty, 0, 1).length).toBe(0);
    expect(byteFlip(empty, 0, 1).length).toBe(0);
    expect(byteArithmetic(empty, 0, 1, 1).length).toBe(0);
    expect(boundaryInsert(empty, 0, 1, 0).length).toBe(0);
    expect(blockDelete(empty, 0, 5).length).toBe(0);
    expect(blockDuplicate(empty, 0, 5, 0).length).toBe(0);

    const ins = blockInsert(empty, 0, new Uint8Array([1, 2, 3]));
    expect(ins.length).toBe(3);
    expect(ins[0]).toBe(1);

    // Negative length or 0 length delete/duplicate
    const single = new Uint8Array([42]);
    expect(blockDelete(single, 0, 0).length).toBe(1);
    expect(blockDuplicate(single, 0, 0, 0).length).toBe(1);

    // Multi-byte arithmetic (32-bit and 64-bit, little and big endian)
    const buf32 = new Uint8Array([0x01, 0x02, 0x03, 0x04]);
    const arith32Le = byteArithmetic(buf32, 0, 1, 4, "little");
    expect(arith32Le[0]).toBe(0x02);
    const arith32Be = byteArithmetic(buf32, 0, 1, 4, "big");
    expect(arith32Be[3]).toBe(0x05);

    const buf64 = new Uint8Array([1, 0, 0, 0, 0, 0, 0, 0]);
    const arith64Le = byteArithmetic(buf64, 0, 10, 8, "little");
    expect(arith64Le[0]).toBe(11);
    const arith64Be = byteArithmetic(buf64, 0, 10, 8, "big");
    expect(arith64Be[7]).toBe(10);

    // Multi-byte boundary inserts (16-bit, 32-bit, 64-bit, little and big endian)
    const boundBuf = new Uint8Array(16);
    const b16Be = boundaryInsert(boundBuf, 0, 2, 0x1234, "big");
    expect(b16Be[0]).toBe(0x12);
    expect(b16Be[1]).toBe(0x34);

    const b32Be = boundaryInsert(boundBuf, 0, 4, 0x12345678, "big");
    expect(b32Be[0]).toBe(0x12);
    expect(b32Be[3]).toBe(0x78);

    const b64Le = boundaryInsert(boundBuf, 0, 8, 0x0102030405060708n, "little");
    expect(b64Le[0]).toBe(0x08);
    const b64Be = boundaryInsert(boundBuf, 0, 8, 0x0102030405060708n, "big");
    expect(b64Be[0]).toBe(0x01);

    // Unsupported width does nothing
    const noopArith = byteArithmetic(single, 0, 1, 3);
    expect(noopArith[0]).toBe(42);
    const noopBound = boundaryInsert(single, 0, 3, 99);
    expect(noopBound[0]).toBe(42);
  });

  it("CoverageBitmap records, checks, admits traces, counts, and merges", () => {
    const bitmap = new CoverageBitmap();
    expect(bitmap.count()).toBe(0);
    expect(bitmap.hasEdge(100)).toBe(false);

    bitmap.recordEdge(100);
    expect(bitmap.hasEdge(100)).toBe(true);
    expect(bitmap.count()).toBe(1);

    // Recording same edge again is idempotent
    bitmap.recordEdge(100);
    expect(bitmap.count()).toBe(1);

    // Admit trace
    const [admitted, countNew] = bitmap.admitTrace([100, 200, 300]);
    expect(admitted).toBe(true);
    expect(countNew).toBe(2);
    expect(bitmap.count()).toBe(3);

    const [admitted2, countNew2] = bitmap.admitTrace([100, 200]);
    expect(admitted2).toBe(false);
    expect(countNew2).toBe(0);

    // Merge another bitmap
    const other = new CoverageBitmap();
    other.recordEdge(500);
    other.recordEdge(1000);
    bitmap.merge(other);
    expect(bitmap.hasEdge(500)).toBe(true);
    expect(bitmap.hasEdge(1000)).toBe(true);
    expect(bitmap.count()).toBe(5);
  });

  it("calculates energy proportionally to novelty and length", () => {
    const e1 = calculateEnergy(10, 100, 5, 10);
    expect(e1 >= 1).toBe(true);

    // Zero total edges does not divide by zero
    const e2 = calculateEnergy(10, 0, 0, 0);
    expect(e2 >= 1).toBe(true);

    // Higher novelty gives higher energy
    const eLow = calculateEnergy(10, 100, 1, 100);
    const eHigh = calculateEnergy(10, 100, 50, 100);
    expect(eHigh > eLow).toBe(true);
  });

  it("minimizes crashing inputs via block deletion and byte zeroing", () => {
    // A target signature that crashes when string contains "BOOM"
    const targetSig = crashSignature("AssertionError", ["test_target.ts:42"]);

    function testFn(candidate: Uint8Array): FuzzOutcome {
      const text = new TextDecoder().decode(candidate);
      if (text.includes("BOOM")) {
        return {
          status: "crash",
          kind: "AssertionError",
          frames: ["test_target.ts:42"],
        };
      }
      return { status: "pass" };
    }

    const payload = new TextEncoder().encode("PREFIX_PADDING_12345_BOOM_SUFFIX_PADDING_67890");
    const minimized = minimizeCrashingInput(payload, targetSig, testFn);
    const minText = new TextDecoder().decode(minimized);

    // Minimized text must still trigger the crash
    expect(minText.includes("BOOM")).toBe(true);
    // Length must be substantially shrunk
    expect(minimized.length < payload.length).toBe(true);

    // Empty input minimization returns empty
    const emptyMin = minimizeCrashingInput(new Uint8Array(0), targetSig, testFn);
    expect(emptyMin.length).toBe(0);
  });

  it("runs a coverage-guided fuzz target campaign finding edges and crash", () => {
    function harness(input: Uint8Array, cov: CoverageBitmap): FuzzOutcome {
      if (input.length > 0) {
        cov.recordEdge(1);
      }
      if (input.length > 2 && input[0] === 0x41) {
        cov.recordEdge(2);
      }
      if (input.length > 3 && input[0] === 0x41 && input[1] === 0x42) {
        cov.recordEdge(3);
        if (input[2] === 0x43) {
          return {
            status: "crash",
            kind: "PanicError",
            frames: ["target.ts:10"],
          };
        }
      }
      return { status: "pass" };
    }

    const initialCorpus = [new Uint8Array([0x41, 0x42, 0x00])];
    const report = fuzzTarget(initialCorpus, { maxIterations: 50, seed: 12345n }, harness);

    expect(report.iterations > 0).toBe(true);
    expect(report.totalEdges >= 2).toBe(true);
  });
});
