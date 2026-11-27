import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const artifactPath = process.argv[2];
assert.ok(artifactPath, "usage: node consumer.mjs path/to/microbenchmark_wasm.wasm");
const wasmBytes = readFileSync(artifactPath);
const module = new WebAssembly.Module(wasmBytes);

function readU32(bytes, offset) {
  let value = 0;
  let shift = 0;
  for (let count = 0; count < 5; count += 1) {
    assert.ok(offset < bytes.length, "truncated unsigned LEB128");
    const byte = bytes[offset];
    offset += 1;
    value += (byte & 0x7f) * 2 ** shift;
    if ((byte & 0x80) === 0) return [value, offset];
    shift += 7;
  }
  throw new Error("overlong unsigned LEB128");
}

function readName(bytes, offset) {
  const [length, bodyOffset] = readU32(bytes, offset);
  const end = bodyOffset + length;
  assert.ok(end <= bytes.length, "truncated WebAssembly name");
  return [new TextDecoder().decode(bytes.subarray(bodyOffset, end)), end];
}

function inspectMemoryLimits(bytes) {
  assert.equal(new TextDecoder().decode(bytes.subarray(0, 4)), "\0asm");
  let offset = 8;
  while (offset < bytes.length) {
    const sectionId = bytes[offset++];
    const [sectionLength, bodyOffset] = readU32(bytes, offset);
    const sectionEnd = bodyOffset + sectionLength;
    assert.ok(sectionEnd <= bytes.length, "truncated WebAssembly section");
    if (sectionId === 5) {
      const [count, memoryOffset] = readU32(bytes, bodyOffset);
      assert.equal(count, 1, "module must define exactly one linear memory");
      const [flags, minOffset] = readU32(bytes, memoryOffset);
      const [minimum, maxOffset] = readU32(bytes, minOffset);
      assert.equal(flags & 1, 1, "memory must declare a maximum");
      const [maximum] = readU32(bytes, maxOffset);
      return { minimum, maximum };
    }
    offset = sectionEnd;
  }
  throw new Error("module has no memory section");
}

const imports = WebAssembly.Module.imports(module).map(({ module, name, kind }) => `${module}.${name}:${kind}`).sort();
assert.deepEqual(imports, [
  "microbenchmark_host.now_ns:function",
  "microbenchmark_host.run_batch:function",
]);

const exportNames = WebAssembly.Module.exports(module).map(({ name }) => name);
for (const required of ["memory", "abi_version", "alloc", "dealloc", "collect_json", "analyze_json", "replay_json"]) {
  assert.ok(exportNames.includes(required), `missing public export ${required}`);
}
const limits = inspectMemoryLimits(wasmBytes);
assert.equal(limits.minimum, 32, "initial memory must be 32 pages");
assert.equal(limits.maximum, 1024, "memory maximum must be 64 MiB");

let instance;
let hostTime = 0;
let nanosecondsPerEntry = 100;
let workChecksum = 42;
let failureStatus = 0;
let returnNegativeClock = false;
let scriptedClock = null;
let callbackCount = 0;
let reentrantInput = null;
let reentrantResult = null;
const importsObject = {
  microbenchmark_host: {
    now_ns() {
      if (scriptedClock !== null) return BigInt(scriptedClock.shift() ?? hostTime);
      return returnNegativeClock ? -1n : BigInt(hostTime);
    },
    run_batch(variant, entries, checksumOut) {
      callbackCount += 1;
      if (reentrantInput !== null && reentrantResult === null) {
        reentrantResult = instance.exports.analyze_json(reentrantInput.pointer, reentrantInput.length);
      }
      if (failureStatus !== 0) return failureStatus;
      const unsignedPointer = checksumOut >>> 0;
      const checksum = BigInt(workChecksum) * BigInt(entries);
      new DataView(instance.exports.memory.buffer).setBigInt64(unsignedPointer, checksum, true);
      hostTime += entries * nanosecondsPerEntry;
      assert.ok(variant === 0 || variant === 1, "host received an unknown workload variant");
      return 0;
    },
  },
};

instance = await WebAssembly.instantiate(module, importsObject);
const api = instance.exports;
assert.equal(api.abi_version(), 1);
assert.equal(api.alloc(-1), 0, "negative signed allocation lengths must fail the unsigned bound check");

function allocateInput(text) {
  const bytes = Buffer.from(text, "utf8");
  const pointer = api.alloc(bytes.length);
  assert.notEqual(pointer, 0, "bounded input allocation failed");
  const address = pointer >>> 0;
  const view = new Uint8Array(api.memory.buffer, address, bytes.length);
  assert.ok(view.every((byte) => byte === 0), "input allocations must be zero-initialized");
  view.set(bytes);
  return { pointer, length: bytes.length };
}

function takeResult(packed) {
  const result = BigInt.asUintN(64, packed);
  assert.notEqual(result, 0n, "module could not register a result allocation");
  const pointer = Number(result & 0xffff_ffffn);
  const length = Number(result >> 32n);
  assert.ok(pointer > 0 && length > 0 && length <= 1024 * 1024);
  const bytes = Buffer.from(new Uint8Array(api.memory.buffer, pointer, length));
  assert.equal(api.dealloc(pointer, length), 0, "exact result allocation should free");
  return JSON.parse(bytes.toString("utf8"));
}

function callWithInput(text, operation) {
  const input = allocateInput(text);
  try {
    return takeResult(operation(input.pointer, input.length));
  } finally {
    assert.equal(api.dealloc(input.pointer, input.length), 0, "exact input allocation should free");
  }
}

const config = {
  pairs: 40,
  confidence_percent: 95,
  minimum_factor: 1.05,
  seed: 3,
  target_ns: 100,
  max_batch: 16,
};
const collectionRequest = JSON.stringify({ config, expected_checksum: 42 });
const collectionInput = allocateInput(collectionRequest);
assert.equal(api.dealloc(collectionInput.pointer, collectionInput.length - 1), 1, "mismatched frees must be rejected");
const collected = takeResult(api.collect_json(collectionInput.pointer, collectionInput.length));
assert.equal(api.dealloc(collectionInput.pointer, collectionInput.length), 0);
assert.equal(api.dealloc(collectionInput.pointer, collectionInput.length), 1, "double frees must be rejected");
assert.equal(collected.ok, true);
assert.equal(collected.value.pairs.length, 40);
assert.equal(collected.value.analysis.verdict, "equivalent");
assert.equal(collected.value.batch_size, 1);
assert.ok(callbackCount >= 82, "collection must call both host workload variants");

const analysisRequest = JSON.stringify({
  confidence_percent: config.confidence_percent,
  minimum_factor: config.minimum_factor,
  pairs: collected.value.pairs,
});
const analysis = callWithInput(analysisRequest, api.analyze_json);
assert.equal(analysis.ok, true);
assert.deepEqual(analysis.value, collected.value.analysis);

const replay = callWithInput(JSON.stringify(collected.value), api.replay_json);
assert.equal(replay.ok, true);
assert.deepEqual(replay.value, collected.value);

const maximumPairs = Array.from({ length: 10_000 }, (_, index) => ({
  baseline_ns: 9_000_000_000_000_000_000,
  candidate_ns: 9_000_000_000_000_000_000,
  order: index < 5_000 ? "baseline-first" : "candidate-first",
}));
const maximumAnalysisInput = JSON.stringify({
  confidence_percent: 95,
  minimum_factor: 1.05,
  pairs: maximumPairs,
});
assert.ok(Buffer.byteLength(maximumAnalysisInput) < 1024 * 1024);
const maximumAnalysis = callWithInput(maximumAnalysisInput, api.analyze_json);
assert.equal(maximumAnalysis.ok, true);
const maximumReport = {
  config: {
    pairs: 10_000,
    confidence_percent: 95,
    minimum_factor: 1.05,
    seed: 0,
    target_ns: 1,
    max_batch: 1,
  },
  batch_size: 1,
  pairs: maximumPairs,
  analysis: maximumAnalysis.value,
};
const maximumReportText = JSON.stringify(maximumReport);
assert.ok(Buffer.byteLength(maximumReportText) < 1024 * 1024);
const maximumReplay = callWithInput(maximumReportText, api.replay_json);
assert.equal(maximumReplay.ok, true);
assert.equal(maximumReplay.value.pairs.length, 10_000);

// Keep seven maximum-sized host allocations live while replay creates its
// largest supported result. The final MiB of the fixed budget is result space.
const retainedBuffers = Array.from({ length: 7 }, () => api.alloc(1024 * 1024));
assert.ok(retainedBuffers.every((pointer) => pointer !== 0));
assert.equal(api.alloc(1), 0, "the caller input budget is seven MiB");
const retainedView = new Uint8Array(api.memory.buffer, retainedBuffers[0] >>> 0, 1024 * 1024);
retainedView.fill(0x20);
retainedView.set(Buffer.from(maximumReportText));
const headroomReplay = takeResult(api.replay_json(retainedBuffers[0], 1024 * 1024));
assert.equal(headroomReplay.ok, true);
for (const pointer of retainedBuffers) assert.equal(api.dealloc(pointer, 1024 * 1024), 0);

// The fixed ownership table has 32 records even when the byte budget allows
// more. Exhaust it with small live buffers and prove the next allocation is
// rejected without disturbing any prior record.
const tableBuffers = Array.from({ length: 32 }, () => api.alloc(1));
assert.ok(tableBuffers.every((pointer) => pointer !== 0));
assert.equal(api.alloc(1), 0, "the fixed allocation table has 32 records");
for (const pointer of tableBuffers) assert.equal(api.dealloc(pointer, 1), 0);

const callbacksBeforeBadJson = callbackCount;
const duplicate = callWithInput(
  '{"confidence_percent":95,"confidence_percent":95,"minimum_factor":1.05,"pairs":[]}',
  api.analyze_json,
);
assert.equal(duplicate.error.code, "invalid_input");
assert.equal(callbackCount, callbacksBeforeBadJson, "invalid requests must not call host workloads");

const unknown = callWithInput(
  '{"confidence_percent":95,"minimum_factor":1.05,"pairs":[],"extra":1}',
  api.analyze_json,
);
assert.equal(unknown.error.code, "invalid_input");
const malformed = callWithInput("{]", api.replay_json);
assert.equal(malformed.error.code, "invalid_report");

const deepJson = `${"[".repeat(17)}0${"]".repeat(17)}`;
const deep = callWithInput(deepJson, api.replay_json);
assert.equal(deep.error.code, "bounds_exceeded");
assert.equal(api.alloc(1024 * 1024 + 1), 0, "input allocation must enforce its byte bound");
assert.equal(takeResult(api.analyze_json(123, 8)).error.code, "invalid_input", "unowned pointers must be rejected safely");

const reentrant = allocateInput(analysisRequest);
reentrantInput = reentrant;
reentrantResult = null;
const reentrantCollection = callWithInput(collectionRequest, api.collect_json);
reentrantInput = null;
assert.equal(reentrantResult, 0n, "reentrant mutating calls must be rejected");
assert.equal(api.dealloc(reentrant.pointer, reentrant.length), 0);
assert.equal(reentrantCollection.ok, true);

failureStatus = 7;
const hostFailure = callWithInput(collectionRequest, api.collect_json);
failureStatus = 0;
assert.equal(hostFailure.error.code, "host_callback_failed");

workChecksum = 41;
const checksumFailure = callWithInput(collectionRequest, api.collect_json);
workChecksum = 42;
assert.equal(checksumFailure.error.code, "checksum_mismatch");

returnNegativeClock = true;
const clockFailure = callWithInput(collectionRequest, api.collect_json);
returnNegativeClock = false;
assert.equal(clockFailure.error.code, "clock_failure");

scriptedClock = [10, 9];
const regressingClockFailure = callWithInput(collectionRequest, api.collect_json);
scriptedClock = null;
assert.equal(regressingClockFailure.error.code, "clock_failure");

const budgetConfig = { ...config, pairs: 10_000, target_ns: 6_553_600, max_batch: 65_536 };
const budgetRequest = JSON.stringify({ config: budgetConfig, expected_checksum: 42 });
const previousRate = nanosecondsPerEntry;
nanosecondsPerEntry = 100;
const budgetFailure = callWithInput(budgetRequest, api.collect_json);
nanosecondsPerEntry = previousRate;
assert.equal(budgetFailure.error.code, "bounds_exceeded");

// Leave the module with no live host buffers or pending result frames.
assert.equal(api.dealloc(reentrant.pointer, reentrant.length), 1);
console.log("external WebAssembly consumer exercised collector, analysis, replay, errors, and ownership");
