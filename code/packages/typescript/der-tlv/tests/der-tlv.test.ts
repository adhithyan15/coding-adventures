import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import { DerCursor, DerElement, DerError, DerLimits, decodeExact, decodeOne, defaultLimits } from "../src/index.js";

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
});
