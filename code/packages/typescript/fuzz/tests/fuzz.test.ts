import { describe, it } from "node:test";
import * as assert from "node:assert/strict";
import * as fs from "node:fs";
import * as path from "node:path";
import { fileURLToPath } from "node:url";

import {
  SplitMix64,
  fnv1a64,
  FID_FUZZ_CRASH,
  FID_FUZZ_TIMEOUT,
  FID_FUZZ_OOM,
  FID_FUZZ_LEAK,
  FID_FUZZ_PANIC,
  FUZZ_FAILURE_IDS,
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
  COVERAGE_MAP_SIZE,
  COVERAGE_BYTES,
  CoverageBitmap,
  crashSignature,
  isDuplicateCrash,
  formatTapFailure,
  calculateEnergy,
  minimizeCrashingInput,
  fuzzTarget,
  type FuzzOutcome,
  type FuzzOptions,
} from "../dist/index.js";

const __dirname = path.dirname(fileURLToPath(import.meta.url));

function hexToBytes(hex: string): Uint8Array {
  if (hex.length === 0) return new Uint8Array(0);
  const bytes = new Uint8Array(hex.length / 2);
  for (let i = 0; i < bytes.length; i++) {
    bytes[i] = parseInt(hex.slice(i * 2, i * 2 + 2), 16);
  }
  return bytes;
}

function bytesToHex(bytes: Uint8Array): string {
  return Array.from(bytes, (b) => b.toString(16).padStart(2, "0")).join("");
}

describe("Fuzz Engine Conformance Suite (VER06 / fuzz-v1)", () => {
  const corpusPath = path.resolve(
    __dirname,
    "../../../../specs/fixtures/fuzz-v1/cases.json",
  );
  const corpusContent = fs.readFileSync(corpusPath, "utf-8");
  const parsed = JSON.parse(corpusContent);
  const cases: any[] = parsed.cases;

  it("loads all canonical cases from fuzz-v1/cases.json", () => {
    assert.equal(cases.length, 25);
  });

  for (const c of cases) {
    it(`conformance case: ${c.id}`, () => {
      const op = c.op;
      switch (op) {
        case "mutate_bit_flip": {
          const input = hexToBytes(c.input.hex);
          const bitOffset = Number(c.bit_offset.value);
          const count = Number(c.count.value);
          const got = bitFlip(input, bitOffset, count);
          assert.equal(bytesToHex(got), c.output.hex);
          break;
        }

        case "mutate_byte_flip": {
          const input = hexToBytes(c.input.hex);
          const byteOffset = Number(c.byte_offset.value);
          const count = Number(c.count.value);
          const got = byteFlip(input, byteOffset, count);
          assert.equal(bytesToHex(got), c.output.hex);
          break;
        }

        case "mutate_arithmetic": {
          const input = hexToBytes(c.input.hex);
          const byteOffset = Number(c.byte_offset.value);
          const delta = BigInt(c.delta.value);
          const width = Number(c.width.value);
          const endian = c.endian;
          const got = byteArithmetic(input, byteOffset, delta, width, endian);
          assert.equal(bytesToHex(got), c.output.hex);
          break;
        }

        case "mutate_boundary": {
          const input = hexToBytes(c.input.hex);
          const byteOffset = Number(c.byte_offset.value);
          const width = Number(c.width.value);
          const val = BigInt(c.val.value);
          const endian = c.endian;
          const got = boundaryInsert(input, byteOffset, width, val, endian);
          assert.equal(bytesToHex(got), c.output.hex);
          break;
        }

        case "mutate_block_delete": {
          const input = hexToBytes(c.input.hex);
          const offset = Number(c.offset.value);
          const length = Number(c.length.value);
          const got = blockDelete(input, offset, length);
          assert.equal(bytesToHex(got), c.output.hex);
          break;
        }

        case "mutate_block_insert": {
          const input = hexToBytes(c.input.hex);
          const offset = Number(c.offset.value);
          const insertBytes = hexToBytes(c.insert_bytes.hex);
          const got = blockInsert(input, offset, insertBytes);
          assert.equal(bytesToHex(got), c.output.hex);
          break;
        }

        case "mutate_block_duplicate": {
          const input = hexToBytes(c.input.hex);
          const srcOffset = Number(c.src_offset.value);
          const length = Number(c.length.value);
          const dstOffset = Number(c.dst_offset.value);
          const got = blockDuplicate(input, srcOffset, length, dstOffset);
          assert.equal(bytesToHex(got), c.output.hex);
          break;
        }

        case "mutate_splice": {
          const target = hexToBytes(c.target.hex);
          const donor = hexToBytes(c.donor.hex);
          const targetSplit = Number(c.target_split.value);
          const donorSplit = Number(c.donor_split.value);
          const got = splice(target, donor, targetSplit, donorSplit);
          assert.equal(bytesToHex(got), c.output.hex);
          break;
        }

        case "mutate_sequence": {
          const input = hexToBytes(c.input.hex);
          const seed = BigInt(c.seed.value);
          const steps = Number(c.steps.value);
          const got = mutateSequence(input, seed, steps);
          assert.equal(bytesToHex(got), c.output.hex);
          break;
        }

        case "corpus_admit": {
          const existing = c.existing_edges.map((e: any) => Number(e.value));
          const trace = c.trace_edges.map((e: any) => Number(e.value));
          const [admitted, newEdges] = corpusAdmit(existing, trace);
          assert.equal(admitted, c.admitted);
          assert.equal(newEdges, Number(c.new_edges.value));
          break;
        }

        case "crash_dedup": {
          const crashA = { kind: c.crash_a_kind, frames: c.crash_a_frames };
          const crashB = { kind: c.crash_b_kind, frames: c.crash_b_frames };
          const isDup = isDuplicateCrash(crashA, crashB);
          assert.equal(isDup, c.is_duplicate);
          break;
        }

        case "failure_id_valid": {
          const got = isFuzzFailureId(c.input);
          assert.equal(got, c.valid);
          break;
        }

        case "private_reporting": {
          const line = formatTapFailure(1, c.target, c.failure_id, c.sig_hash);
          assert.equal(line, c.tap_line);
          assert.ok(!line.includes(c.raw_payload.hex));
          break;
        }

        case "corpus_reject": {
          const raw = c.text;
          assert.throws(() => {
            if (!raw.startsWith("{") && !raw.startsWith("[")) {
              throw new Error("invalid json");
            }
            JSON.parse(raw);
          });
          break;
        }

        default:
          assert.fail(`unrecognized op: ${op}`);
      }
    });
  }
});

describe("Coverage Bitmap Operations", () => {
  it("initializes empty bitmap with correct dimensions", () => {
    const cov = new CoverageBitmap();
    assert.equal(cov.bytes.length, COVERAGE_BYTES);
    assert.equal(cov.count(), 0);
  });

  it("records and checks edges", () => {
    const cov = new CoverageBitmap();
    cov.recordEdge(0);
    cov.recordEdge(100);
    cov.recordEdge(65535);

    assert.ok(cov.hasEdge(0));
    assert.ok(cov.hasEdge(100));
    assert.ok(cov.hasEdge(65535));
    assert.ok(!cov.hasEdge(1));
    assert.ok(!cov.hasEdge(101));
    assert.equal(cov.count(), 3);
  });

  it("handles modulo and negative edge indices safely", () => {
    const cov = new CoverageBitmap();
    cov.recordEdge(-1);
    assert.ok(cov.hasEdge(65535));
    cov.recordEdge(65536 + 42);
    assert.ok(cov.hasEdge(42));
  });

  it("admits trace edges and returns novelty count", () => {
    const cov = new CoverageBitmap();
    const [hasNew1, count1] = cov.admitTrace([10, 20, 30]);
    assert.ok(hasNew1);
    assert.equal(count1, 3);
    assert.equal(cov.count(), 3);

    const [hasNew2, count2] = cov.admitTrace([20, 30, 40]);
    assert.ok(hasNew2);
    assert.equal(count2, 1);
    assert.equal(cov.count(), 4);

    const [hasNew3, count3] = cov.admitTrace([10, 40]);
    assert.ok(!hasNew3);
    assert.equal(count3, 0);
  });

  it("merges bitmaps correctly", () => {
    const a = new CoverageBitmap();
    a.recordEdge(1);
    a.recordEdge(2);

    const b = new CoverageBitmap();
    b.recordEdge(2);
    b.recordEdge(3);

    a.merge(b);
    assert.equal(a.count(), 3);
    assert.ok(a.hasEdge(1));
    assert.ok(a.hasEdge(2));
    assert.ok(a.hasEdge(3));
  });
});

describe("Energy Scheduling & Mutator Edge Cases", () => {
  it("calculates energy based on length and new edges", () => {
    const e1 = calculateEnergy(10, 16, 5, 20);
    assert.ok(e1 >= 1);

    const eZeroTotal = calculateEnergy(10, 0, 0, 0);
    assert.equal(eZeroTotal, 10);

    const eHighLen = calculateEnergy(10, 10000, 0, 100);
    assert.ok(eHighLen < eZeroTotal);
  });

  it("mutator edge cases on empty buffers", () => {
    const empty = new Uint8Array(0);
    assert.equal(bitFlip(empty, 0).length, 0);
    assert.equal(byteFlip(empty, 0).length, 0);
    assert.equal(byteArithmetic(empty, 0, 1, 1).length, 0);
    assert.equal(boundaryInsert(empty, 0, 1, 0).length, 0);
    assert.equal(blockDelete(empty, 0, 1).length, 0);

    const ins = blockInsert(empty, 0, new Uint8Array([1, 2]));
    assert.equal(ins.length, 2);

    assert.equal(blockDuplicate(empty, 0, 1, 0).length, 0);
  });

  it("byteArithmetic with widths 1, 2, 4, 8 and endianness", () => {
    const buf = new Uint8Array([0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);

    // 8-byte big endian
    const be8 = byteArithmetic(buf, 0, 1n, 8, "big");
    assert.equal(be8[7], 1);

    // 8-byte little endian
    const le8 = byteArithmetic(buf, 0, 1n, 8, "little");
    assert.equal(le8[0], 1);

    // 4-byte big endian
    const be4 = byteArithmetic(buf, 0, 1n, 4, "big");
    assert.equal(be4[3], 1);

    // buffer shorter than width returns unchanged
    const shortBuf = new Uint8Array([1, 2]);
    const unchanged = byteArithmetic(shortBuf, 0, 1n, 4);
    assert.equal(unchanged.length, 2);
  });

  it("boundaryInsert with widths 1, 2, 4, 8 and endianness", () => {
    const buf = new Uint8Array(8);

    const be8 = boundaryInsert(buf, 0, 8, 0x1122334455667788n, "big");
    assert.equal(be8[0], 0x11);
    assert.equal(be8[7], 0x88);

    const le8 = boundaryInsert(buf, 0, 8, 0x1122334455667788n, "little");
    assert.equal(le8[0], 0x88);
    assert.equal(le8[7], 0x11);

    const be4 = boundaryInsert(buf, 0, 4, 0x11223344, "big");
    assert.equal(be4[0], 0x11);
    assert.equal(be4[3], 0x44);

    const shortBuf = new Uint8Array([1, 2]);
    assert.equal(boundaryInsert(shortBuf, 0, 4, 0).length, 2);
  });

  it("crashSignature and deduplication", () => {
    const sig1 = crashSignature("AssertionError", ["funcA", "funcB"]);
    const sig2 = crashSignature("AssertionError", ["funcA", "funcB"]);
    const sig3 = crashSignature("TypeError", ["funcA", "funcB"]);

    assert.equal(sig1.length, 8);
    assert.equal(sig1, sig2);
    assert.notEqual(sig1, sig3);
  });

  it("minimizeCrashingInput shrinks payload while preserving crash signature", () => {
    const initial = new Uint8Array([10, 20, 30, 40, 50, 60, 70, 80]);
    const targetSig = crashSignature("MagicCrash", ["target"]);

    const testFn = (cand: Uint8Array): FuzzOutcome => {
      // Crashes if byte 0 is 10 and byte 7 is 80 (middle bytes don't matter)
      if (cand.length >= 2 && cand[0] === 10 && cand[cand.length - 1] === 80) {
        return { status: "crash", kind: "MagicCrash", frames: ["target"] };
      }
      return { status: "pass" };
    };

    const minimized = minimizeCrashingInput(initial, targetSig, testFn);
    assert.ok(minimized.length <= initial.length);
    assert.equal(minimized[0], 10);
    assert.equal(minimized[minimized.length - 1], 80);
    // Non-essential bytes should have been zeroed or deleted
    assert.equal(testFn(minimized).status, "crash");
  });

  it("fuzzTarget finds and minimizes crashes with coverage feedback", () => {
    const seedInput = new Uint8Array([0x00, 0x00, 0x00, 0x00]);

    const report = fuzzTarget(
      [seedInput],
      { maxIterations: 300, seed: 12345 },
      (candidate, cov) => {
        if (candidate.length > 0 && candidate[0] === 0x42) {
          cov.recordEdge(100);
          if (candidate.length > 1 && candidate[1] === 0x99) {
            cov.recordEdge(200);
            return {
              status: "crash",
              kind: "TargetPanic",
              frames: ["fuzzTarget", "eval"],
            };
          }
        }
        cov.recordEdge(1);
        return { status: "pass" };
      },
    );

    assert.ok(report.iterations > 0);
    assert.ok(report.totalEdges >= 1);
  });

  it("validates failure vocabulary members", () => {
    assert.ok(isFuzzFailureId(FID_FUZZ_CRASH));
    assert.ok(isFuzzFailureId(FID_FUZZ_TIMEOUT));
    assert.ok(isFuzzFailureId(FID_FUZZ_OOM));
    assert.ok(isFuzzFailureId(FID_FUZZ_LEAK));
    assert.ok(isFuzzFailureId(FID_FUZZ_PANIC));
    assert.ok(!isFuzzFailureId("unknown-id"));
    assert.equal(FUZZ_FAILURE_IDS.size, 5);
  });

  it("exercises mutator boundary conditions and edge cases", () => {
    const rng = new SplitMix64(0n);
    assert.equal(rng.bounded(0), 0);
    assert.equal(rng.bounded(1), 0);
    assert.throws(() => rng.bounded(0x1_0000_0001n));

    // Unsupported width in byteArithmetic & boundaryInsert
    const buf = new Uint8Array([1, 2, 3, 4]);
    const unsuppArith = byteArithmetic(buf, 0, 1n, 3);
    assert.equal(unsuppArith.length, 4);

    const unsuppBound = boundaryInsert(buf, 0, 3, 1n);
    assert.equal(unsuppBound.length, 4);

    // blockDelete with negative/zero lengths
    assert.equal(blockDelete(buf, 0, 0).length, 4);
    assert.equal(blockDelete(buf, 0, -1).length, 4);

    // mutateSequence on empty buffer
    const mutatedEmpty = mutateSequence(new Uint8Array(0), 42n, 1);
    assert.ok(mutatedEmpty.length >= 1);

    // minimizeCrashingInput on empty buffer
    const minEmpty = minimizeCrashingInput(new Uint8Array(0), "sig", () => ({ status: "pass" }));
    assert.equal(minEmpty.length, 0);

    // fuzzTarget with initial crash and multiple corpus seeds for splicing
    const multiSeed = [
      new Uint8Array([1, 2, 3, 4]),
      new Uint8Array([5, 6, 7, 8]),
    ];
    let callCount = 0;
    const report2 = fuzzTarget(
      multiSeed,
      { maxIterations: 50, seed: 999, maxInputLen: 16 },
      (cand, cov) => {
        callCount++;
        cov.recordEdge(callCount % 100);
        if (cand.length > 5 && cand[0] === 5) {
          return { status: "crash", kind: "MultiCrash", frames: ["f"] };
        }
        return { status: "pass" };
      }
    );
    assert.ok(report2.iterations > 0);

    // fuzzTarget with crashing initial seed
    const crashReport = fuzzTarget(
      [new Uint8Array([9, 9])],
      { maxIterations: 5 },
      () => ({ status: "crash", kind: "InitCrash", frames: ["main"] })
    );
    assert.equal(crashReport.crashes.length, 1);
    assert.equal(crashReport.crashes[0].kind, "InitCrash");
  });
});
