import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import {
  DEFAULT_MAX_ELEMENTS,
  DEFAULT_MAX_VALUE_LEN,
  DerCursor,
  DerElement,
  DerError,
  DerLimits,
  decodeExact,
  decodeOne,
  defaultLimits,
} from "../src/index.js";

interface Segment { hex?: string; repeat_hex?: string; count?: number }
interface FixtureCase { id: string; operation: "decode-exact" | "decode-one" | "cursor"; input: Segment[]; limits?: Record<string, number | string>; actions?: ("read" | "finish")[]; redacted_input_hex?: string; expected: unknown }
interface Fixture { defaults: Record<string, number>; error_ids: string[]; cases: FixtureCase[] }

const path = fileURLToPath(new URL("../../../../specs/fixtures/der-tlv-v1/cases.json", import.meta.url));
const fixture = JSON.parse(readFileSync(path, "utf8")) as Fixture;

function bytes(segments: Segment[]): Uint8Array {
  const output: number[] = [];
  for (const segment of segments) {
    if (segment.hex !== undefined) {
      for (let index = 0; index < segment.hex.length; index += 2) output.push(Number.parseInt(segment.hex.slice(index, index + 2), 16));
    } else {
      output.push(...new Array(segment.count!).fill(Number.parseInt(segment.repeat_hex!, 16)) as number[]);
    }
  }
  return Uint8Array.from(output);
}

function configured(testCase: FixtureCase): DerLimits {
  const values: Record<string, number | string> = { ...fixture.defaults, ...testCase.limits };
  return {
    maxInputLen: values.max_input_len as number,
    maxValueLen: values.max_value_len === "host-max" ? BigInt(Number.MAX_SAFE_INTEGER) : BigInt(values.max_value_len as number),
    maxElements: values.max_elements as number,
    maxTagNumber: values.max_tag_number as number,
  };
}

function elementResult(element: DerElement, offset: number): unknown {
  return { outcome: "element", element_offset: offset, tag: element.tag, header_len: element.header.length, encoded_len: element.encoded.length, remainder_offset: offset + element.encoded.length };
}

function errorResult(error: unknown): unknown {
  expect(error).toBeInstanceOf(DerError);
  const failure = error as DerError;
  return { outcome: "error", error_id: failure.kind, offset: failure.offset };
}

function runDecode(testCase: FixtureCase, input: Uint8Array, limits: DerLimits): unknown {
  try {
    if (testCase.operation === "decode-one") {
      const [element, remainder] = decodeOne(input, limits);
      const projection = elementResult(element, 0) as { remainder_offset: number };
      expect(projection.remainder_offset).toBe(input.length - remainder.length);
      return projection;
    }
    return elementResult(decodeExact(input, limits), 0);
  } catch (error: unknown) { return errorResult(error); }
}

function runCursor(testCase: FixtureCase, input: Uint8Array, limits: DerLimits): unknown {
  const cursor = new DerCursor(input, limits);
  const events: unknown[] = [];
  for (const action of testCase.actions!) {
    if (action === "finish") {
      try { cursor.finish(); events.push({ outcome: "finished" }); } catch (error: unknown) { events.push(errorResult(error)); }
    } else {
      const offset = input.length - cursor.remaining.length;
      try {
        const element = cursor.read();
        events.push(element === undefined ? { outcome: "end" } : elementResult(element, offset));
      } catch (error: unknown) { events.push(errorResult(error)); }
    }
  }
  return { events, elements_read: cursor.elementsRead, remaining_offset: input.length - cursor.remaining.length };
}

describe("DER TLV v1 portable conformance", () => {
  it("pins the closed profile", () => {
    expect(fixture.cases).toHaveLength(54);
    expect(fixture.error_ids).toHaveLength(17);
  });
  for (const testCase of fixture.cases) {
    it(testCase.id, () => {
      const input = bytes(testCase.input);
      const result = testCase.operation === "cursor" ? runCursor(testCase, input, configured(testCase)) : runDecode(testCase, input, configured(testCase));
      expect(result).toEqual(testCase.expected);
      if (testCase.redacted_input_hex !== undefined) expect(JSON.stringify(result)).not.toContain(testCase.redacted_input_hex);
    });
  }
  it("returns views over the caller buffer", () => {
    const input = Uint8Array.of(0x04, 0x01, 0x2a);
    const element = decodeExact(input, defaultLimits());
    input[2] = 0x7f;
    expect([...element.value]).toEqual([0x7f]);
  });
  it.each([
    ["non-finite input limit", { ...defaultLimits(), maxInputLen: Number.NaN }],
    ["infinite element limit", { ...defaultLimits(), maxElements: Number.POSITIVE_INFINITY }],
    ["fractional element limit", { ...defaultLimits(), maxElements: 1.5 }],
    ["negative tag limit", { ...defaultLimits(), maxTagNumber: -1 }],
    ["oversized tag limit", { ...defaultLimits(), maxTagNumber: 0x1_0000_0000 }],
    ["negative value limit", { ...defaultLimits(), maxValueLen: -1n }],
  ])("rejects %s", (_name, limits) => {
    const input = Uint8Array.of(0x05, 0x00);
    expect(() => decodeExact(input, limits)).toThrow(RangeError);
    expect(() => new DerCursor(input, limits)).toThrow(RangeError);
  });
  it("snapshots cursor limits against caller mutation", () => {
    const limits = { ...defaultLimits(), maxElements: 1 };
    const cursor = new DerCursor(Uint8Array.of(0x05, 0x00, 0x05, 0x00), limits);
    expect(cursor.read()).toBeInstanceOf(DerElement);
    limits.maxElements = 2;
    expect(() => cursor.read()).toThrowError(expect.objectContaining({
      kind: "element-limit-exceeded",
      offset: 2,
    }));
    expect(cursor.elementsRead).toBe(1);
    expect(cursor.remaining).toEqual(Uint8Array.of(0x05, 0x00));
  });
  it("reads each limit property once before validation and snapshotting", () => {
    let inputLimitReads = 0;
    const limits: DerLimits = {
      get maxInputLen() {
        inputLimitReads += 1;
        return inputLimitReads === 1 ? 2 : Number.POSITIVE_INFINITY;
      },
      maxValueLen: DEFAULT_MAX_VALUE_LEN,
      maxElements: DEFAULT_MAX_ELEMENTS,
      maxTagNumber: 0xffff_ffff,
    };
    expect(() => new DerCursor(Uint8Array.of(0x05, 0x00, 0x05, 0x00), limits))
      .toThrowError(expect.objectContaining({ kind: "input-limit-exceeded", offset: 0 }));
    expect(inputLimitReads).toBe(1);
  });
  it("keeps a cursor within the input extent captured at construction", () => {
    const ResizableArrayBuffer = ArrayBuffer as unknown as new (
      byteLength: number,
      options: { maxByteLength: number },
    ) => ArrayBuffer;
    const buffer = new ResizableArrayBuffer(2, { maxByteLength: 4 });
    const bytes = new Uint8Array(buffer);
    bytes.set([0x05, 0x00]);
    const cursor = new DerCursor(bytes, { ...defaultLimits(), maxInputLen: 2 });
    expect(cursor.read()).toBeInstanceOf(DerElement);
    (buffer as ArrayBuffer & { resize(byteLength: number): void }).resize(4);
    new Uint8Array(buffer).set([0x05, 0x00], 2);
    expect(cursor.read()).toBeUndefined();
    expect(cursor.elementsRead).toBe(1);
    expect(cursor.remaining).toEqual(new Uint8Array());
  });
  it("uses typed-array intrinsics instead of shadowable extent and slice properties", () => {
    const oversized = new Uint8Array(20);
    for (let index = 0; index < oversized.byteLength; index += 2) oversized.set([0x05, 0x00], index);
    Object.defineProperties(oversized, {
      length: { get: () => Number.NaN },
      subarray: { value: () => oversized },
    });
    expect(() => new DerCursor(oversized, { ...defaultLimits(), maxInputLen: 2 }))
      .toThrowError(expect.objectContaining({ kind: "input-limit-exceeded", offset: 0 }));

    const encoded = Uint8Array.of(0x04, 0x01, 0x2a);
    Object.defineProperties(encoded, {
      length: { get: () => Number.NaN },
      subarray: { value: () => encoded },
    });
    const element = decodeExact(encoded);
    expect(element.header).toEqual(Uint8Array.of(0x04, 0x01));
    expect(element.value).toEqual(Uint8Array.of(0x2a));
    expect(element.encoded).toEqual(Uint8Array.of(0x04, 0x01, 0x2a));
  });
});
