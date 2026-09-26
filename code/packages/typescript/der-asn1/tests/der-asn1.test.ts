import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { describe, expect, it, vi } from "vitest";
import type { DerLimits } from "@coding-adventures/der-tlv";
import {
  Asn1Cursor,
  Asn1Decoder,
  Asn1Element,
  Asn1Error,
  Asn1ErrorKind,
  type Asn1Limits,
  decodeBitString,
  decodeBoolean,
  decodeIA5String,
  decodeImplicitIA5String,
  decodeImplicitObjectIdentifier,
  decodeImplicitOctetString,
  decodeInteger,
  decodeNull,
  decodeObjectIdentifier,
  decodeOctetString,
  defaultAsn1Limits,
} from "../src/index.js";

interface Segment { hex?: string; repeat_hex?: string; count?: number }
interface FixtureCase {
  id: string;
  operation: string;
  input: Segment[];
  limits?: Record<string, unknown>;
  actions?: string[];
  tag_number?: number;
  der_tlv_case_id?: string;
  redacted_input_hex?: string;
  expected: unknown;
}
interface Fixture {
  defaults: Record<string, unknown>;
  error_ids: string[];
  cases: FixtureCase[];
}

function fixture(name: string): Fixture {
  const path = fileURLToPath(new URL(`../../../../specs/fixtures/${name}/cases.json`, import.meta.url));
  return JSON.parse(readFileSync(path, "utf8")) as Fixture;
}

const contract = fixture("der-asn1-v1");
const upstream = fixture("der-tlv-v1");

function fromHex(value: string): number[] {
  const output: number[] = [];
  for (let index = 0; index < value.length; index += 2) output.push(Number.parseInt(value.slice(index, index + 2), 16));
  return output;
}

function materialize(segments: Segment[]): Uint8Array {
  const output: number[] = [];
  for (const segment of segments) {
    const octets = fromHex(segment.hex ?? segment.repeat_hex!);
    const count = segment.hex === undefined ? segment.count! : 1;
    for (let index = 0; index < count; index += 1) output.push(...octets);
  }
  return Uint8Array.from(output);
}

function hex(value: Uint8Array): string {
  return [...value].map((octet) => octet.toString(16).padStart(2, "0")).join("");
}

function numeric(value: unknown): number {
  if (typeof value !== "number") throw new Error("expected numeric fixture limit");
  return value;
}

function derLimits(defaults: Record<string, unknown>, overrides: Record<string, unknown> = {}): DerLimits {
  const value = (name: string): unknown => overrides[name] ?? defaults[name];
  return {
    maxInputLen: numeric(value("max_input_len")),
    maxValueLen: value("max_value_len") === "host-max" ? BigInt(Number.MAX_SAFE_INTEGER) : BigInt(numeric(value("max_value_len"))),
    maxElements: numeric(value("max_elements")),
    maxTagNumber: numeric(value("max_tag_number")),
  };
}

function limits(testCase: FixtureCase): Asn1Limits {
  const overrides = testCase.limits ?? {};
  const nested = (overrides.der ?? {}) as Record<string, unknown>;
  const value = (name: string): unknown => overrides[name] ?? contract.defaults[name];
  return {
    der: derLimits(contract.defaults.der as Record<string, unknown>, nested),
    maxDepth: numeric(value("max_depth")),
    maxTotalElements: numeric(value("max_total_elements")),
    maxOidArcs: numeric(value("max_oid_arcs")),
  };
}

function failure(error: unknown, scope: string): Record<string, unknown> {
  expect(error).toBeInstanceOf(Asn1Error);
  const typed = error as Asn1Error;
  const result: Record<string, unknown> = {
    outcome: "error",
    error_id: typed.kind,
    offset: typed.offset,
    offset_scope: scope,
  };
  if (typed.framingKind !== undefined) result.framing_error_id = typed.framingKind;
  return result;
}

function verifyUpstream(id: string): unknown {
  const referenced = upstream.cases.find((candidate) => candidate.id === id);
  if (referenced === undefined) throw new Error(`missing upstream case ${id}`);
  const configured = { ...defaultAsn1Limits(), der: derLimits(upstream.defaults, referenced.limits ?? {}) };
  try {
    const input = materialize(referenced.input);
    const element = new Asn1Decoder(configured).decodeExact(input);
    const expected = referenced.expected as Record<string, unknown>;
    expect(expected.outcome, id).toBe("element");
    expect(element.tag, id).toEqual(expected.tag);
    expect(element.header.length, id).toBe(expected.header_len);
    expect(element.encoded.length, id).toBe(expected.encoded_len);
    expect(element.encoded.length, id).toBe(input.length);
  } catch (error: unknown) {
    const typed = error as Asn1Error;
    const expected = referenced.expected as Record<string, unknown>;
    expect(expected.outcome, id).toBe("error");
    expect(typed.kind, id).toBe(Asn1ErrorKind.Framing);
    expect(typed.framingKind, id).toBe(expected.error_id);
    expect(typed.offset, id).toBe(expected.offset);
    if (referenced.redacted_input_hex !== undefined) {
      expect(String(typed)).not.toContain(referenced.redacted_input_hex);
      expect(JSON.stringify(failure(typed, "operation-input"))).not.toContain(referenced.redacted_input_hex);
    }
  }
  return { outcome: "upstream" };
}

function primitive(operation: string, element: Asn1Element, configured: Asn1Limits, tagNumber = 0): unknown {
  switch (operation) {
    case "decode-boolean": return { outcome: "value", boolean: decodeBoolean(element), elements_read: 1 };
    case "decode-integer":
    case "integer-to-u64": {
      const value = decodeInteger(element);
      const result: Record<string, unknown> = { outcome: "value", signed_hex: hex(value.signedBytes), negative: value.isNegative };
      if (operation === "integer-to-u64") result.u64_decimal = value.toU64().toString();
      return result;
    }
    case "decode-bit-string": {
      const value = decodeBitString(element);
      return { outcome: "value", bytes_hex: hex(value.bytes), unused_bits: value.unusedBits, bit_length: value.bitLength };
    }
    case "decode-octet-string": return { outcome: "value", bytes_hex: hex(decodeOctetString(element)) };
    case "decode-implicit-octet-string": return { outcome: "value", bytes_hex: hex(decodeImplicitOctetString(element, tagNumber)) };
    case "decode-ia5-string": return { outcome: "value", text: decodeIA5String(element) };
    case "decode-implicit-ia5-string": return { outcome: "value", text: decodeImplicitIA5String(element, tagNumber) };
    case "decode-null": decodeNull(element); return { outcome: "value" };
    case "decode-object-identifier":
    case "decode-implicit-object-identifier": {
      const value = operation === "decode-object-identifier"
        ? decodeObjectIdentifier(element, configured)
        : decodeImplicitObjectIdentifier(element, tagNumber, configured);
      return { outcome: "value", bytes_hex: hex(value.encoded), arcs_decimal: value.arcs.map(String), arc_count: value.arcCount };
    }
    default: throw new Error(`unsupported operation ${operation}`);
  }
}

function cursorResult(testCase: FixtureCase, decoder: Asn1Decoder, root: Asn1Element): unknown {
  const cursor = decoder.sequence(root);
  const total = cursor.remaining.length;
  const events: unknown[] = [];
  for (const action of testCase.actions ?? []) {
    if (action === "finish") {
      try { cursor.finish(); events.push({ outcome: "finished" }); }
      catch (error: unknown) { events.push(failure(error, "container-value")); }
      continue;
    }
    if (action !== "read" && action !== "read-with-different-limits" && action !== "read-nested-sequence") throw new Error(`unsupported action ${action}`);
    const active = action !== "read-with-different-limits" ? decoder : new Asn1Decoder({ ...decoder.limits, maxTotalElements: decoder.limits.maxTotalElements + 1 });
    try {
      const child = cursor.read(active);
      if (action === "read-nested-sequence") {
        if (child === undefined) throw new Error("nested child required");
        const nested = decoder.sequence(child);
        const grandchild = nested.read(decoder);
        if (grandchild === undefined) throw new Error("nested grandchild required");
        nested.finish();
        events.push({ outcome: "value", tag: grandchild.tag, depth: grandchild.depth });
        continue;
      }
      events.push(child === undefined ? { outcome: "end" } : { outcome: "value", tag: child.tag, depth: child.depth });
    } catch (error: unknown) { events.push(failure(error, "container-value")); }
  }
  return { outcome: "value", elements_read: decoder.elementsRead, remaining_offset: total - cursor.remaining.length, events };
}

function runCase(testCase: FixtureCase): unknown {
  if (testCase.der_tlv_case_id !== undefined) return verifyUpstream(testCase.der_tlv_case_id);
  const configured = limits(testCase);
  const decoder = new Asn1Decoder(configured);
  try {
    const root = decoder.decodeExact(materialize(testCase.input));
    switch (testCase.operation) {
      case "decode-exact": return {
        outcome: "value", tag: root.tag, header_hex: hex(root.header), value_hex: hex(root.value),
        encoded_hex: hex(root.encoded), depth: root.depth, elements_read: decoder.elementsRead,
      };
      case "cursor-script": return cursorResult(testCase, decoder, root);
      case "sequence":
      case "set": {
        const cursor = testCase.operation === "sequence" ? decoder.sequence(root) : decoder.set(root);
        return { outcome: "value", elements_read: decoder.elementsRead, remaining_offset: root.value.length - cursor.remaining.length };
      }
      case "explicit": {
        const child = decoder.explicit(root, testCase.tag_number!);
        return { outcome: "value", tag: child.tag, value_hex: hex(child.value), depth: child.depth, elements_read: decoder.elementsRead };
      }
      default: return primitive(testCase.operation, root, configured, testCase.tag_number);
    }
  } catch (error: unknown) {
    const scope = testCase.operation === "explicit" && error instanceof Asn1Error && error.kind === Asn1ErrorKind.Framing
      ? "container-value" : "operation-input";
    return failure(error, scope);
  }
}

describe("DER ASN.1 v1 portable conformance", () => {
  it("pins the closed profile", () => {
    expect(contract.cases).toHaveLength(122);
    expect(contract.error_ids).toHaveLength(22);
    const references = contract.cases.flatMap((testCase) => testCase.der_tlv_case_id === undefined ? [] : [testCase.der_tlv_case_id]);
    expect(references).toHaveLength(46);
    expect(new Set(references).size).toBe(46);
  });

  for (const testCase of contract.cases) {
    it(testCase.id, () => {
      const actual = runCase(testCase);
      expect(actual).toEqual(testCase.expected);
      if (testCase.redacted_input_hex !== undefined) expect(JSON.stringify(actual)).not.toContain(testCase.redacted_input_hex);
    });
  }

  it("keeps typed wrappers unforgeable and validates limits", () => {
    const tokenless = Asn1Element as unknown as new (...args: unknown[]) => Asn1Element;
    expect(() => new tokenless()).toThrow(TypeError);
    for (const valueType of [Asn1Cursor, decodeInteger(new Asn1Decoder().decodeExact(Uint8Array.of(0x02, 0x01, 0x01))).constructor,
      decodeBitString(new Asn1Decoder().decodeExact(Uint8Array.of(0x03, 0x01, 0x00))).constructor,
      decodeObjectIdentifier(new Asn1Decoder().decodeExact(Uint8Array.of(0x06, 0x01, 0x2a))).constructor]) {
      const untrusted = valueType as unknown as new (...args: unknown[]) => unknown;
      expect(() => new untrusted()).toThrow(TypeError);
    }
    for (const configured of [
      { ...defaultAsn1Limits(), maxDepth: -1 },
      { ...defaultAsn1Limits(), maxTotalElements: Number.NaN },
      { ...defaultAsn1Limits(), maxOidArcs: 1.5 },
      { ...defaultAsn1Limits(), der: { ...defaultAsn1Limits().der, maxInputLen: Number.NaN } },
    ]) expect(() => new Asn1Decoder(configured)).toThrow(RangeError);
  });

  it("does not derive construction authority from ambient Symbol", async () => {
    const original = globalThis.Symbol;
    const descriptions: unknown[] = [];
    globalThis.Symbol = new Proxy(original, {
      apply(target, thisArgument, argumentsList) {
        descriptions.push(argumentsList[0]);
        return Reflect.apply(target, thisArgument, argumentsList) as symbol;
      },
    });
    try {
      vi.resetModules();
      await import("../src/index.js?ambient-symbol-tamper");
    } finally {
      globalThis.Symbol = original;
    }
    expect(descriptions).not.toContain("coding-adventures.der-asn1.element");
  });

  it("rejects prototype forgeries and ignores overridden public getters", () => {
    const fake = Object.create(Asn1Element.prototype) as Asn1Element;
    Object.defineProperties(fake, {
      tag: { value: { class: "universal", constructed: false, number: 1 }, configurable: true },
      value: { value: Uint8Array.of(0xff), configurable: true },
      valueOffset: { value: 2, configurable: true },
      depth: { value: -1, configurable: true },
    });
    expect(() => decodeBoolean(fake)).toThrowError(expect.objectContaining({ kind: Asn1ErrorKind.UnexpectedTag }));
    expect(() => new Asn1Decoder({ ...defaultAsn1Limits(), maxDepth: 1 }).sequence(fake))
      .toThrowError(expect.objectContaining({ kind: Asn1ErrorKind.UnexpectedTag }));

    const real = new Asn1Decoder().decodeExact(Uint8Array.of(0x01, 0x01, 0xff));
    const descriptor = Object.getOwnPropertyDescriptor(Asn1Element.prototype, "tag")!;
    Object.defineProperty(Asn1Element.prototype, "tag", { configurable: true, get: () => ({ class: "universal", constructed: false, number: 4 }) });
    try { expect(decodeBoolean(real)).toBe(true); }
    finally { Object.defineProperty(Asn1Element.prototype, "tag", descriptor); }
  });

  it("keeps validated typed bytes stable after returned views are mutated", () => {
    const integer = decodeInteger(new Asn1Decoder().decodeExact(Uint8Array.of(0x02, 0x01, 0x01)));
    integer.signedBytes[0] = 0xff;
    expect(integer.isNegative).toBe(false);
    expect(integer.toU64()).toBe(1n);

    const bits = decodeBitString(new Asn1Decoder().decodeExact(Uint8Array.of(0x03, 0x02, 0x01, 0x80)));
    bits.bytes[0] = 0x81;
    expect([...bits.bytes]).toEqual([0x80]);
    expect(bits.bitLength).toBe(7);

    const oid = decodeObjectIdentifier(new Asn1Decoder().decodeExact(Uint8Array.of(0x06, 0x01, 0x2a)));
    oid.encoded[0] = 0x80;
    expect([...oid.encoded]).toEqual([0x2a]);
    expect(oid.arcs).toEqual([1n, 2n]);
  });

  it("shares cursors across structurally equal limits", () => {
    const first = new Asn1Decoder();
    const cursor: Asn1Cursor = first.sequence(first.decodeExact(Uint8Array.of(0x30, 0x02, 0x05, 0x00)));
    const second = new Asn1Decoder(defaultAsn1Limits());
    expect(cursor.read(second)).toBeInstanceOf(Asn1Element);
    expect(second.elementsRead).toBe(1);
    expect(cursor.remaining.length).toBe(0);
  });

  it("snapshots validated bytes and returns defensive byte views", () => {
    const input = Uint8Array.of(0x04, 0x01, 0x2a);
    const element = new Asn1Decoder().decodeExact(input);
    input[2] = 0x7f;
    expect([...element.value]).toEqual([0x2a]);
    const value = element.value;
    value[0] = 0x7f;
    expect([...element.value]).toEqual([0x2a]);
    const octets = decodeOctetString(element);
    octets[0] = 0x7f;
    expect([...decodeOctetString(element)]).toEqual([0x2a]);

    const decoder = new Asn1Decoder();
    const cursor = decoder.sequence(decoder.decodeExact(Uint8Array.of(0x30, 0x02, 0x05, 0x00)));
    const remaining = cursor.remaining;
    remaining[0] = 0x01;
    expect(cursor.read(decoder)?.tag.number).toBe(5);
    expect(Object.isFrozen(element.tag)).toBe(true);
  });

  it("freezes stable runtime error identifiers", () => {
    const mutableKinds = Asn1ErrorKind as unknown as Record<string, string>;
    expect(() => { mutableKinds.InvalidBooleanValue = "spoofed-error-id"; }).toThrow(TypeError);
    expect(Asn1ErrorKind.InvalidBooleanValue).toBe("invalid-boolean-value");
  });

  it("compares every cursor-limit field structurally", () => {
    const variants: Asn1Limits[] = [
      { ...defaultAsn1Limits(), maxDepth: 31 },
      { ...defaultAsn1Limits(), maxTotalElements: 16_383 },
      { ...defaultAsn1Limits(), maxOidArcs: 127 },
      { ...defaultAsn1Limits(), der: { ...defaultAsn1Limits().der, maxInputLen: 1_048_575 } },
      { ...defaultAsn1Limits(), der: { ...defaultAsn1Limits().der, maxValueLen: 1_048_575n } },
      { ...defaultAsn1Limits(), der: { ...defaultAsn1Limits().der, maxElements: 4_095 } },
      { ...defaultAsn1Limits(), der: { ...defaultAsn1Limits().der, maxTagNumber: 0xffff_fffe } },
    ];
    for (const variant of variants) {
      const decoder = new Asn1Decoder();
      const cursor = decoder.sequence(decoder.decodeExact(Uint8Array.of(0x30, 0x02, 0x05, 0x00)));
      expect(() => cursor.read(new Asn1Decoder(variant))).toThrowError(expect.objectContaining({ kind: Asn1ErrorKind.DecoderLimitMismatch }));
    }
    const raw = Asn1Decoder.readCursor as unknown as (...args: unknown[]) => unknown;
    expect(() => raw(Symbol(), undefined, 0, defaultAsn1Limits(), new Asn1Decoder())).toThrow(TypeError);
  });

  it("compares OID arcs exactly", () => {
    const oid = decodeObjectIdentifier(new Asn1Decoder().decodeExact(Uint8Array.of(0x06, 0x03, 0x2a, 0x03, 0x04)));
    expect(oid.equalsArcs([1n, 2n, 3n, 4n])).toBe(true);
    expect(oid.equalsArcs([1n, 2n, 3n])).toBe(false);
    expect(oid.equalsArcs([1n, 2n, 3n, 5n])).toBe(false);
  });

  it("redacts hostile input from public errors", () => {
    const element = new Asn1Decoder().decodeExact(Uint8Array.of(0x16, 0x02, 0x61, 0xff));
    expect(() => decodeIA5String(element)).toThrowError(expect.objectContaining({ kind: Asn1ErrorKind.NonAsciiIa5String }));
    try { decodeIA5String(element); } catch (error: unknown) {
      expect(String(error)).not.toContain("61ff");
      expect(String(error)).not.toContain("255");
    }
  });
});
