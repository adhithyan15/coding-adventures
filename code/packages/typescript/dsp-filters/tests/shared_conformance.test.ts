import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { describe, it } from "node:test";
import { Biquad, EmptySignalError, sosfilt } from "../src/index.js";

type FixtureRecord = {
  id: string;
  operation: "biquad.process" | "sosfilt";
  sections: number[][];
  expected: number[];
  chunk_lengths?: number[];
};
type Fixture = { input: number[]; cases: FixtureRecord[] };

const fixtureUrl = new URL(
  "../../../../../specs/fixtures/biquad-sos-v1/cases.json",
  import.meta.url
);
const fixtureText = readFileSync(fixtureUrl, "utf8");
const required = new Map([
  ["biquad.process.impulse16", "biquad.process"],
  ["sosfilt.impulse16.two_sections", "sosfilt"],
]);

function object(value: unknown, where: string): Record<string, unknown> {
  assert.ok(value !== null && typeof value === "object" && !Array.isArray(value), `${where} must be an object`);
  return value as Record<string, unknown>;
}

function exactKeys(value: Record<string, unknown>, keys: string[], where: string): void {
  assert.deepEqual(Object.keys(value).sort(), [...keys].sort(), `${where} has unexpected fields`);
}

function finiteNumber(value: unknown, where: string): number {
  assert.equal(typeof value, "number", `${where} must be a JSON number`);
  assert.ok(Number.isFinite(value), `${where} must be finite`);
  return value as number;
}

function decodeFixture(text: string): Fixture {
  const root = object(JSON.parse(text) as unknown, "fixture");
  exactKeys(root, ["schema_version", "input", "comparison", "cases"], "fixture");
  assert.equal(root.schema_version, 1, "schema_version must be 1");
  assert.equal(root.comparison, "finite_exact_numeric_equal_zero_sign_ignored");

  assert.ok(Array.isArray(root.input) && root.input.length === 16, "input must contain 16 samples");
  const input = root.input.map((sample, i) => finiteNumber(sample, `input[${i}]`));
  assert.deepEqual(input, [1, ...Array<number>(15).fill(0)], "input must be the unit impulse");

  assert.ok(Array.isArray(root.cases) && root.cases.length === 2, "fixture must contain exactly two records");
  const byId = new Map<string, FixtureRecord>();
  for (const [index, raw] of root.cases.entries()) {
    const record = object(raw, `cases[${index}]`);
    const operation = record.operation;
    assert.ok(operation === "biquad.process" || operation === "sosfilt", "unknown operation");
    const expectedKeys = operation === "biquad.process"
      ? ["id", "operation", "sections", "expected", "chunk_lengths"]
      : ["id", "operation", "sections", "expected"];
    exactKeys(record, expectedKeys, `cases[${index}]`);
    assert.equal(typeof record.id, "string", "operation ID must be a string");
    const id = record.id as string;
    assert.equal(byId.has(id), false, `duplicate operation ID ${id}`);
    assert.equal(required.get(id), operation, `unexpected ID/operation pair ${id}`);

    const sectionsRaw = record.sections;
    const sectionCount = operation === "biquad.process" ? 1 : 2;
    assert.ok(Array.isArray(sectionsRaw) && sectionsRaw.length === sectionCount, `${id}: wrong section count`);
    const sections = sectionsRaw.map((section, sectionIndex) => {
      assert.ok(Array.isArray(section) && section.length === 6, `${id}: section must have six coefficients`);
      return section.map((coefficient, coefficientIndex) =>
        finiteNumber(coefficient, `sections[${sectionIndex}][${coefficientIndex}]`)
      );
    });

    assert.ok(Array.isArray(record.expected) && record.expected.length === 16, `${id}: expected must contain 16 samples`);
    const expected = record.expected.map((sample, sampleIndex) => finiteNumber(sample, `expected[${sampleIndex}]`));
    let chunkLengths: number[] | undefined;
    if (operation === "biquad.process") {
      assert.ok(Array.isArray(record.chunk_lengths), "chunk_lengths must be an array");
      chunkLengths = record.chunk_lengths.map((length, i) => {
        assert.ok(typeof length === "number" && Number.isSafeInteger(length) && length > 0, `chunk_lengths[${i}] must be a positive integer`);
        return length;
      });
      assert.equal(chunkLengths.reduce((sum, length) => sum + length, 0), 16, "chunk lengths must sum to 16");
      assert.deepEqual(chunkLengths, [3, 1, 12], "chunk lengths must equal [3, 1, 12]");
    }
    byId.set(id, { id, operation, sections, expected, chunk_lengths: chunkLengths });
  }
  assert.deepEqual([...byId.keys()].sort(), [...required.keys()].sort(), "fixture is missing a required operation ID");
  return { input, cases: [...required.keys()].map((id) => byId.get(id)!) };
}

function assertExactFinite(actual: ArrayLike<number>, expected: number[]): void {
  assert.equal(actual.length, expected.length, "output length mismatch");
  for (let i = 0; i < expected.length; i++) {
    const got = actual[i];
    assert.ok(Number.isFinite(got), `output[${i}] is non-finite: ${got}`);
    assert.equal(got, expected[i], `output[${i}] value mismatch`);
  }
}

function makeBiquad(record: FixtureRecord): Biquad {
  return new Biquad(...record.sections[0] as [number, number, number, number, number, number]);
}

function partitionChunks(input: number[], mask: number): number[][] {
  const chunks: number[][] = [];
  let start = 0;
  for (let boundary = 0; boundary < 15; boundary++) {
    if ((mask & (1 << boundary)) !== 0) {
      chunks.push(input.slice(start, boundary + 1));
      start = boundary + 1;
    }
  }
  chunks.push(input.slice(start));
  return chunks;
}

function processChunks(filter: Biquad, chunks: number[][]): number[] {
  const output: number[] = [];
  for (const chunk of chunks) output.push(...filter.process(chunk));
  return output;
}

describe("shared AF-02 Biquad/SOS fixture", () => {
  it("rejects malformed fixture records and an altered dyadic expected sample", () => {
    const original = JSON.parse(fixtureText) as Record<string, unknown>;
    const missing = structuredClone(original);
    (missing.cases as unknown[]).pop();
    assert.throws(() => decodeFixture(JSON.stringify(missing)), /exactly two/);

    for (const invalidVersion of [true, 2]) {
      const invalid = structuredClone(original);
      invalid.schema_version = invalidVersion;
      assert.throws(() => decodeFixture(JSON.stringify(invalid)), /schema_version/);
    }

    const unknownField = structuredClone(original);
    unknownField.extra = 1;
    assert.throws(() => decodeFixture(JSON.stringify(unknownField)), /unexpected fields/);

    const missingField = structuredClone(original);
    delete missingField.comparison;
    assert.throws(() => decodeFixture(JSON.stringify(missingField)), /unexpected fields/);

    const duplicate = structuredClone(original);
    const duplicateCases = duplicate.cases as Record<string, unknown>[];
    duplicateCases[1] = structuredClone(duplicateCases[0]);
    assert.throws(() => decodeFixture(JSON.stringify(duplicate)), /duplicate operation ID/);

    const malformed = structuredClone(original);
    const malformedCases = malformed.cases as Record<string, unknown>[];
    const malformedSections = malformedCases[0].sections as unknown[][];
    malformedSections[0][0] = true;
    assert.throws(() => decodeFixture(JSON.stringify(malformed)), /JSON number/);

    const extraRecordField = structuredClone(original);
    const extraRecordCases = extraRecordField.cases as Record<string, unknown>[];
    extraRecordCases[0].extra = 0;
    assert.throws(() => decodeFixture(JSON.stringify(extraRecordField)), /unexpected fields/);

    const wrongChunks = structuredClone(original);
    const wrongChunkCases = wrongChunks.cases as Record<string, unknown>[];
    wrongChunkCases[0].chunk_lengths = [16];
    assert.throws(() => decodeFixture(JSON.stringify(wrongChunks)), /equal \[3, 1, 12\]/);

    const nonfinite = fixtureText.replace("0.25,", "1e999,");
    assert.throws(() => decodeFixture(nonfinite), /finite/);

    const changed = structuredClone(original);
    const changedCases = changed.cases as Record<string, unknown>[];
    const expected = changedCases[0].expected as number[];
    expected[3] += 2 ** -20;
    const decoded = decodeFixture(JSON.stringify(changed));
    assert.throws(
      () => assertExactFinite(makeBiquad(decoded.cases[0]).process(decoded.input), decoded.cases[0].expected),
      /value mismatch/
    );
  });

  it("checks whole, fixed chunks, sample calls, reset replay, and all 32,768 partitions", () => {
    const fixture = decodeFixture(fixtureText);
    const record = fixture.cases[0];
    const expected = record.expected;

    assertExactFinite(makeBiquad(record).process(fixture.input), expected);
    const [first, second, third] = record.chunk_lengths!;
    assertExactFinite(
      processChunks(makeBiquad(record), [
        fixture.input.slice(0, first),
        fixture.input.slice(first, first + second),
        fixture.input.slice(first + second, first + second + third),
      ]),
      expected
    );
    const samplewise = makeBiquad(record);
    assertExactFinite(fixture.input.map((sample) => samplewise.processSample(sample)), expected);

    for (let mask = 0; mask < 2 ** 15; mask++) {
      assertExactFinite(
        processChunks(makeBiquad(record), partitionChunks(fixture.input, mask)),
        expected
      );
    }

    const replay = makeBiquad(record);
    replay.process(fixture.input.slice(0, 7));
    replay.reset();
    assertExactFinite(replay.process(fixture.input), expected);
  });

  it("checks one-shot SOS, empty-section identity, and empty-input rejection", () => {
    const fixture = decodeFixture(fixtureText);
    const record = fixture.cases[1];
    assertExactFinite(sosfilt(record.sections, fixture.input), record.expected);
    assertExactFinite(sosfilt(record.sections, fixture.input), record.expected);
    assertExactFinite(sosfilt([], fixture.input), fixture.input);
    assert.throws(() => sosfilt([], []), EmptySignalError);
  });
});
