/**
 * Coverage-Guided Fuzz Engine (VER06 / VER06b).
 *
 * Level 2 coverage-guided fuzz testing engine providing a deterministic mutator
 * algebra, 65,536-bit coverage bitmap feedback, seed energy scheduling, crash
 * deduplication via normalized FNV-1a stack hashing, crashing input minimization,
 * closed failure vocabulary, and privacy-preserving TAP reporting.
 *
 * Conforms to specifications:
 * - code/specs/VER06-coverage-guided-fuzz-engine.md
 * - code/specs/VER06b-typescript-fuzz-engine.md
 */

import { SplitMix64, fnv1a64 } from "./prng.js";

// --- Closed Failure Vocabulary (VER06 §5.1, §6) -----------------------------

export const FID_FUZZ_CRASH = "fuzz-crash";
export const FID_FUZZ_TIMEOUT = "fuzz-timeout";
export const FID_FUZZ_OOM = "fuzz-oom";
export const FID_FUZZ_LEAK = "fuzz-leak";
export const FID_FUZZ_PANIC = "fuzz-panic";

export const FUZZ_FAILURE_IDS: ReadonlySet<string> = new Set<string>([
  FID_FUZZ_CRASH,
  FID_FUZZ_TIMEOUT,
  FID_FUZZ_OOM,
  FID_FUZZ_LEAK,
  FID_FUZZ_PANIC,
]);

/**
 * Check if a failure ID is in the closed fuzz vocabulary (VER06 §5.1).
 */
export function isFuzzFailureId(id: string): boolean {
  return FUZZ_FAILURE_IDS.has(id);
}

// --- Mutator Algebra (VER06 §3) ---------------------------------------------

/**
 * Invert `count` consecutive bits starting at `bitOffset` modulo total bit length.
 * If `data` is empty, returns an empty Uint8Array.
 */
export function bitFlip(data: Uint8Array, bitOffset: number, count: number = 1): Uint8Array {
  if (data.length === 0) {
    return new Uint8Array(0);
  }
  const buf = new Uint8Array(data);
  const totalBits = buf.length * 8;
  const base = ((bitOffset % totalBits) + totalBits) % totalBits;
  for (let i = 0; i < count; i++) {
    const pos = (base + (i % totalBits)) % totalBits;
    const byteIdx = Math.floor(pos / 8);
    const bitIdx = pos % 8;
    buf[byteIdx] ^= 1 << bitIdx;
  }
  return buf;
}

/**
 * Invert `count` consecutive bytes via XOR 0xFF starting at `byteOffset` modulo total byte length.
 * If `data` is empty, returns an empty Uint8Array.
 */
export function byteFlip(data: Uint8Array, byteOffset: number, count: number = 1): Uint8Array {
  if (data.length === 0) {
    return new Uint8Array(0);
  }
  const buf = new Uint8Array(data);
  const totalBytes = buf.length;
  const base = ((byteOffset % totalBytes) + totalBytes) % totalBytes;
  for (let i = 0; i < count; i++) {
    const pos = (base + (i % totalBytes)) % totalBytes;
    buf[pos] ^= 0xff;
  }
  return buf;
}

/**
 * Add signed `delta` modulo `2^(8 * width)` to integer at `byteOffset`.
 * Supports widths 1, 2, 4, and 8 bytes in "little" or "big" endianness.
 * If `data.length < width`, returns a copy of `data` unchanged.
 */
export function byteArithmetic(
  data: Uint8Array,
  byteOffset: number,
  delta: number | bigint,
  width: number = 1,
  endian: "little" | "big" | string = "little"
): Uint8Array {
  if (data.length < width) {
    return new Uint8Array(data);
  }
  const buf = new Uint8Array(data);
  const offset = ((byteOffset % (buf.length - width + 1)) + (buf.length - width + 1)) % (buf.length - width + 1);
  const isLittle = endian !== "big";
  const view = new DataView(buf.buffer, buf.byteOffset, buf.byteLength);
  const bigDelta = BigInt(delta);

  switch (width) {
    case 1: {
      const val = BigInt(buf[offset]);
      const newVal = BigInt.asUintN(8, val + bigDelta);
      buf[offset] = Number(newVal);
      break;
    }
    case 2: {
      const val = BigInt(view.getUint16(offset, isLittle));
      const newVal = BigInt.asUintN(16, val + bigDelta);
      view.setUint16(offset, Number(newVal), isLittle);
      break;
    }
    case 4: {
      const val = BigInt(view.getUint32(offset, isLittle));
      const newVal = BigInt.asUintN(32, val + bigDelta);
      view.setUint32(offset, Number(newVal), isLittle);
      break;
    }
    case 8: {
      const val = view.getBigUint64(offset, isLittle);
      const newVal = BigInt.asUintN(64, val + bigDelta);
      view.setBigUint64(offset, newVal, isLittle);
      break;
    }
    default:
      break;
  }
  return buf;
}

/**
 * Overwrite `width` bytes at `byteOffset` with boundary interest value `val`.
 * Supports widths 1, 2, 4, and 8 bytes in "little" or "big" endianness.
 * If `data.length < width`, returns a copy of `data` unchanged.
 */
export function boundaryInsert(
  data: Uint8Array,
  byteOffset: number,
  width: number,
  val: number | bigint,
  endian: "little" | "big" | string = "little"
): Uint8Array {
  if (data.length < width) {
    return new Uint8Array(data);
  }
  const buf = new Uint8Array(data);
  const offset = ((byteOffset % (buf.length - width + 1)) + (buf.length - width + 1)) % (buf.length - width + 1);
  const isLittle = endian !== "big";
  const view = new DataView(buf.buffer, buf.byteOffset, buf.byteLength);
  const bigVal = BigInt(val);

  switch (width) {
    case 1: {
      buf[offset] = Number(BigInt.asUintN(8, bigVal));
      break;
    }
    case 2: {
      view.setUint16(offset, Number(BigInt.asUintN(16, bigVal)), isLittle);
      break;
    }
    case 4: {
      view.setUint32(offset, Number(BigInt.asUintN(32, bigVal)), isLittle);
      break;
    }
    case 8: {
      view.setBigUint64(offset, BigInt.asUintN(64, bigVal), isLittle);
      break;
    }
    default:
      break;
  }
  return buf;
}

/**
 * Delete `length` bytes at `offset % len(data)`.
 * Clamps length to the remaining buffer size. If `data` is empty or `length <= 0`,
 * returns a copy of `data` unchanged.
 */
export function blockDelete(data: Uint8Array, offset: number, length: number): Uint8Array {
  if (data.length === 0 || length <= 0) {
    return new Uint8Array(data);
  }
  const actualOffset = ((offset % data.length) + data.length) % data.length;
  const actualLength = Math.min(length, data.length - actualOffset);
  const out = new Uint8Array(data.length - actualLength);
  out.set(data.subarray(0, actualOffset), 0);
  out.set(data.subarray(actualOffset + actualLength), actualOffset);
  return out;
}

/**
 * Insert `insertBytes` at `offset % (len(data) + 1)`.
 * If `data` is empty, returns a copy of `insertBytes`.
 */
export function blockInsert(data: Uint8Array, offset: number, insertBytes: Uint8Array): Uint8Array {
  if (data.length === 0) {
    return new Uint8Array(insertBytes);
  }
  const actualOffset = ((offset % (data.length + 1)) + (data.length + 1)) % (data.length + 1);
  const out = new Uint8Array(data.length + insertBytes.length);
  out.set(data.subarray(0, actualOffset), 0);
  out.set(insertBytes, actualOffset);
  out.set(data.subarray(actualOffset), actualOffset + insertBytes.length);
  return out;
}

/**
 * Duplicate `length` bytes starting at `srcOffset` and insert them at `dstOffset`.
 * Clamps copied length to buffer boundary. If `data` is empty or `length <= 0`,
 * returns a copy of `data` unchanged.
 */
export function blockDuplicate(
  data: Uint8Array,
  srcOffset: number,
  length: number,
  dstOffset: number
): Uint8Array {
  if (data.length === 0 || length <= 0) {
    return new Uint8Array(data);
  }
  const actualSrc = ((srcOffset % data.length) + data.length) % data.length;
  const actualLen = Math.min(length, data.length - actualSrc);
  const chunk = data.subarray(actualSrc, actualSrc + actualLen);
  const actualDst = ((dstOffset % (data.length + 1)) + (data.length + 1)) % (data.length + 1);

  const out = new Uint8Array(data.length + actualLen);
  out.set(data.subarray(0, actualDst), 0);
  out.set(chunk, actualDst);
  out.set(data.subarray(actualDst), actualDst + actualLen);
  return out;
}

/**
 * Splice prefix of `target` before `targetSplit` with suffix of `donor` from `donorSplit`.
 */
export function splice(
  target: Uint8Array,
  donor: Uint8Array,
  targetSplit: number,
  donorSplit: number
): Uint8Array {
  const tSplit = Math.max(0, Math.min(targetSplit, target.length));
  const dSplit = Math.max(0, Math.min(donorSplit, donor.length));
  const out = new Uint8Array(tSplit + (donor.length - dSplit));
  out.set(target.subarray(0, tSplit), 0);
  out.set(donor.subarray(dSplit), tSplit);
  return out;
}

/**
 * Deterministic compound mutation sequence using SplitMix64 PRNG (VER06 §3.7).
 * Executes `steps` mutator invocations chosen uniformly by `rng.bounded(7)`.
 */
export function mutateSequence(data: Uint8Array, seed: bigint | number, steps: number): Uint8Array {
  const rng = new SplitMix64(seed);
  let current: Uint8Array = new Uint8Array(data);

  for (let i = 0; i < steps; i++) {
    if (current.length === 0) {
      current = new Uint8Array([0]);
    }
    const opChoice = rng.bounded(7);
    switch (opChoice) {
      case 0: {
        const bitOff = rng.bounded(current.length * 8);
        current = bitFlip(current, bitOff, 1);
        break;
      }
      case 1: {
        const byteOff = rng.bounded(current.length);
        current = byteFlip(current, byteOff, 1);
        break;
      }
      case 2: {
        const deltas = [-35, -8, -1, 1, 8, 35];
        const delta = deltas[rng.bounded(deltas.length)];
        const byteOff = rng.bounded(current.length);
        current = byteArithmetic(current, byteOff, delta, 1, "little");
        break;
      }
      case 3: {
        const interest = [0, 1, 127, 128, 255];
        const val = interest[rng.bounded(interest.length)];
        const byteOff = rng.bounded(current.length);
        current = boundaryInsert(current, byteOff, 1, val, "little");
        break;
      }
      case 4: {
        if (current.length > 1) {
          const maxDel = Math.max(1, Math.floor(current.length / 2));
          const delLen = 1 + rng.bounded(maxDel);
          const off = rng.bounded(current.length);
          current = blockDelete(current, off, delLen);
        }
        break;
      }
      case 5: {
        const insByte = new Uint8Array([rng.bounded(256)]);
        const off = rng.bounded(current.length + 1);
        current = blockInsert(current, off, insByte);
        break;
      }
      default: {
        if (current.length > 1) {
          const maxDup = Math.max(1, Math.floor(current.length / 2));
          const dupLen = 1 + rng.bounded(maxDup);
          const srcOff = rng.bounded(current.length);
          const dstOff = rng.bounded(current.length + 1);
          current = blockDuplicate(current, srcOff, dupLen, dstOff);
        }
        break;
      }
    }
  }
  return current;
}

// --- Corpus Admission & Coverage Bitmap (VER06 §4) ---------------------------

/**
 * Pure check whether `traceEdges` discovers any novel edge absent from `existingEdges`.
 * Returns `[admitted, newEdgesCount]`.
 */
export function corpusAdmit(existingEdges: number[], traceEdges: number[]): [boolean, number] {
  const existing = new Set<number>(existingEdges);
  const uniqueNew = new Set<number>();
  for (const edge of traceEdges) {
    if (!existing.has(edge)) {
      uniqueNew.add(edge);
    }
  }
  if (uniqueNew.size > 0) {
    return [true, uniqueNew.size];
  }
  return [false, 0];
}

export const COVERAGE_MAP_SIZE = 65536;
export const COVERAGE_BYTES = COVERAGE_MAP_SIZE / 8; // 8192

/**
 * Shared 65,536-bit execution coverage bitmap (VER06 §4.1).
 */
export class CoverageBitmap {
  readonly bytes: Uint8Array;

  constructor() {
    this.bytes = new Uint8Array(COVERAGE_BYTES);
  }

  /**
   * Record that edge `edge` (modulo 65,536) was executed.
   */
  recordEdge(edge: number): void {
    const e = ((edge % COVERAGE_MAP_SIZE) + COVERAGE_MAP_SIZE) % COVERAGE_MAP_SIZE;
    const byteIdx = Math.floor(e / 8);
    const bitMask = 1 << (e % 8);
    this.bytes[byteIdx] |= bitMask;
  }

  /**
   * Check whether edge `edge` has been covered.
   */
  hasEdge(edge: number): boolean {
    const e = ((edge % COVERAGE_MAP_SIZE) + COVERAGE_MAP_SIZE) % COVERAGE_MAP_SIZE;
    const byteIdx = Math.floor(e / 8);
    const bitMask = 1 << (e % 8);
    return (this.bytes[byteIdx] & bitMask) !== 0;
  }

  /**
   * Admit execution trace edges: updates bitmap with unseen edges.
   * Returns `[hasNew, countNew]`.
   */
  admitTrace(trace: number[]): [boolean, number] {
    let countNew = 0;
    for (const edge of trace) {
      const e = ((edge % COVERAGE_MAP_SIZE) + COVERAGE_MAP_SIZE) % COVERAGE_MAP_SIZE;
      const byteIdx = Math.floor(e / 8);
      const bitMask = 1 << (e % 8);
      if ((this.bytes[byteIdx] & bitMask) === 0) {
        this.bytes[byteIdx] |= bitMask;
        countNew++;
      }
    }
    return [countNew > 0, countNew];
  }

  /**
   * Total number of distinct covered edges.
   */
  count(): number {
    let total = 0;
    for (let i = 0; i < this.bytes.length; i++) {
      let b = this.bytes[i];
      while (b > 0) {
        total += b & 1;
        b >>= 1;
      }
    }
    return total;
  }

  /**
   * Merge cumulative coverage from another bitmap.
   */
  merge(other: CoverageBitmap): void {
    for (let i = 0; i < this.bytes.length; i++) {
      this.bytes[i] |= other.bytes[i];
    }
  }
}

// --- Crash Deduplication & Privacy-Preserving TAP (VER06 §5, VER00 §7.3) -----

/**
 * Compute 8-hex-digit normalized crash signature hash (VER06 §5.2).
 * Hashes `kind:frame_1:frame_2:...` with FNV-1a 64-bit and formats lower 32 bits.
 */
export function crashSignature(kind: string, frames: string[]): string {
  const raw = `${kind}:${frames.join(":")}`;
  const encoder = new TextEncoder();
  const h = fnv1a64(encoder.encode(raw));
  const lower32 = Number(h & 0xffff_ffffn);
  return lower32.toString(16).padStart(8, "0");
}

/**
 * Check if two crash reports share an identical normalized signature.
 */
export function isDuplicateCrash(
  crashA: { kind: string; frames: string[] },
  crashB: { kind: string; frames: string[] }
): boolean {
  return crashSignature(crashA.kind, crashA.frames) === crashSignature(crashB.kind, crashB.frames);
}

/**
 * Format privacy-preserving TAP failure line (VER00 §7.3 / VER06 §5.2).
 * Contains NO raw input bytes: only test number, target ID, failure vocabulary ID, and sig hash.
 */
export function formatTapFailure(
  testNum: number,
  targetId: string,
  failureId: string,
  sigHash: string
): string {
  return `not ok ${testNum} - ${targetId} [${failureId}:${sigHash}]`;
}

// --- Energy Scheduling (VER06 §4.3) -----------------------------------------

/**
 * Compute power schedule energy (mutation budget) for a corpus seed.
 * energy(S) = base_energy * (1 + new_edges / total_edges) / sqrt(len)
 */
export function calculateEnergy(
  baseEnergy: number,
  seedLen: number,
  newEdges: number,
  totalEdges: number
): number {
  const base = Math.max(1, baseEnergy);
  const edgeBonus = totalEdges > 0 ? 1.0 + newEdges / totalEdges : 1.0;
  const lenFactor = Math.sqrt(Math.max(1, seedLen));
  const energy = Math.round((base * edgeBonus) / lenFactor);
  return Math.max(1, energy);
}

// --- Crash Minimization & Engine Runner (VER06 §5.3, §7) ---------------------

/**
 * Outcome of a single target invocation.
 */
export type FuzzOutcome =
  | { status: "pass" }
  | { status: "crash"; kind: string; frames: string[] }
  | { status: "timeout" }
  | { status: "oom" };

/**
 * Configuration options for a bounded fuzz campaign.
 */
export interface FuzzOptions {
  maxIterations?: number;
  seed?: bigint | number;
  maxInputLen?: number;
  baseEnergy?: number;
}

/**
 * Deduplicated crash record.
 */
export interface CrashRecord {
  signature: string;
  kind: string;
  frames: string[];
  minimalInput: Uint8Array;
  hits: number;
}

/**
 * Summary report returned after a bounded fuzzing run.
 */
export interface FuzzReport {
  iterations: number;
  admittedSeeds: number;
  totalEdges: number;
  crashes: CrashRecord[];
}

/**
 * Minimize a crashing input while preserving its exact crash signature (VER06 §5.3).
 *
 * 1. Tries block deletion of chunks (halving chunk sizes).
 * 2. Tries byte zeroing on individual non-essential bytes.
 */
export function minimizeCrashingInput(
  input: Uint8Array,
  targetSig: string,
  testFn: (candidate: Uint8Array) => FuzzOutcome
): Uint8Array {
  let current: Uint8Array = new Uint8Array(input);
  if (current.length === 0) {
    return current;
  }

  // Pass 1: recursive block deletion with halving chunk sizes
  let chunkSize = Math.floor(current.length / 2);
  while (chunkSize > 0) {
    let offset = 0;
    while (offset < current.length) {
      const actualLen = Math.min(chunkSize, current.length - offset);
      const candidate: Uint8Array = new Uint8Array(current.length - actualLen);
      candidate.set(current.subarray(0, offset), 0);
      candidate.set(current.subarray(offset + actualLen), offset);

      const outcome = testFn(candidate);
      if (outcome.status === "crash") {
        if (crashSignature(outcome.kind, outcome.frames) === targetSig) {
          current = candidate;
          continue;
        }
      }
      offset += chunkSize;
    }
    chunkSize = Math.floor(chunkSize / 2);
  }

  // Pass 2: byte zeroing / normalization
  for (let i = 0; i < current.length; i++) {
    if (current[i] === 0) {
      continue;
    }
    const originalByte = current[i];
    current[i] = 0;
    const outcome = testFn(current);
    let matched = false;
    if (outcome.status === "crash") {
      if (crashSignature(outcome.kind, outcome.frames) === targetSig) {
        matched = true;
      }
    }
    if (!matched) {
      current[i] = originalByte;
    }
  }

  return current;
}

/**
 * Seed entry held in the active corpus.
 */
export interface CorpusSeed {
  data: Uint8Array;
  newEdges: number;
  generation: number;
}

/**
 * Run a coverage-guided fuzz campaign over `initialCorpus` seeds (VER06 §7).
 */
export function fuzzTarget(
  initialCorpus: Uint8Array[],
  opts: FuzzOptions = {},
  target: (input: Uint8Array, cov: CoverageBitmap) => FuzzOutcome
): FuzzReport {
  const maxIterations = opts.maxIterations ?? 1000;
  const seed = BigInt(opts.seed ?? 42);
  const maxInputLen = opts.maxInputLen ?? 4096;
  const baseEnergy = opts.baseEnergy ?? 10;

  const rng = new SplitMix64(seed);
  const cumulativeCoverage = new CoverageBitmap();
  const corpus: CorpusSeed[] = [];
  const crashes: CrashRecord[] = [];

  // 1. Initial corpus triage
  const seeds = initialCorpus.length === 0 ? [new Uint8Array(1)] : initialCorpus;

  for (const s of seeds) {
    const stepCov = new CoverageBitmap();
    const outcome = target(s, stepCov);
    if (outcome.status === "pass") {
      const trace: number[] = [];
      for (let edge = 0; edge < COVERAGE_MAP_SIZE; edge++) {
        if (stepCov.hasEdge(edge)) {
          trace.push(edge);
        }
      }
      const [hasNew, newCount] = cumulativeCoverage.admitTrace(trace);
      if (hasNew || corpus.length === 0) {
        corpus.push({
          data: new Uint8Array(s),
          newEdges: newCount,
          generation: 0,
        });
      }
    } else if (outcome.status === "crash") {
      const sig = crashSignature(outcome.kind, outcome.frames);
      crashes.push({
        signature: sig,
        kind: outcome.kind,
        frames: outcome.frames,
        minimalInput: new Uint8Array(s),
        hits: 1,
      });
    }
  }

  // 2. Fuzzing loop
  let iterations = 0;
  while (iterations < maxIterations && corpus.length > 0) {
    const seedIdx = rng.bounded(corpus.length);
    const chosenSeed = corpus[seedIdx];
    const totalCov = cumulativeCoverage.count();
    const energy = calculateEnergy(baseEnergy, chosenSeed.data.length, chosenSeed.newEdges, totalCov);

    for (let e = 0; e < energy; e++) {
      if (iterations >= maxIterations) {
        break;
      }
      iterations++;

      const mutationSteps = 1 + rng.bounded(4);
      const subSeedVal = rng.nextU64();
      let candidate: Uint8Array = mutateSequence(chosenSeed.data, subSeedVal, mutationSteps);

      // Splicing with another seed if corpus > 1 and rng chooses
      if (corpus.length > 1 && rng.bounded(4) === 0) {
        const donorIdx = rng.bounded(corpus.length);
        const donor = corpus[donorIdx].data;
        const tSplit = rng.bounded(candidate.length + 1);
        const dSplit = rng.bounded(donor.length + 1);
        candidate = splice(candidate, donor, tSplit, dSplit);
      }

      if (candidate.length > maxInputLen) {
        candidate = candidate.subarray(0, maxInputLen);
      }

      const stepCov = new CoverageBitmap();
      const outcome = target(candidate, stepCov);

      if (outcome.status === "pass") {
        const trace: number[] = [];
        for (let edge = 0; edge < COVERAGE_MAP_SIZE; edge++) {
          if (stepCov.hasEdge(edge)) {
            trace.push(edge);
          }
        }
        const [hasNew, newCount] = cumulativeCoverage.admitTrace(trace);
        if (hasNew) {
          corpus.push({
            data: candidate,
            newEdges: newCount,
            generation: chosenSeed.generation + 1,
          });
        }
      } else if (outcome.status === "crash") {
        const sig = crashSignature(outcome.kind, outcome.frames);
        const existing = crashes.find((c) => c.signature === sig);
        if (existing) {
          existing.hits++;
          if (candidate.length < existing.minimalInput.length) {
            existing.minimalInput = candidate;
          }
        } else {
          crashes.push({
            signature: sig,
            kind: outcome.kind,
            frames: outcome.frames,
            minimalInput: candidate,
            hits: 1,
          });
        }
      }
    }
  }

  return {
    iterations,
    admittedSeeds: corpus.length,
    totalEdges: cumulativeCoverage.count(),
    crashes,
  };
}
