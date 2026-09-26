import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import type { DerLimits } from "@coding-adventures/der-tlv";
import { Asn1Decoder, type Asn1Element, type Asn1Limits } from "@coding-adventures/der-asn1";
import {
  X509Extension,
  X509ExtensionError,
  decodeX509Extension,
} from "../src/index.js";

interface Segment { hex?: string; repeat_hex?: string; count?: number }
interface FixtureCase {
  id: string;
  operation: string;
  input: Segment[];
  limits?: Record<string, unknown>;
  actions?: string[];
  redacted_input_hex?: string;
  expected: unknown;
}
interface Fixture { defaults: Record<string, unknown>; cases: FixtureCase[] }

function fixture(name: string): Fixture {
  const path = fileURLToPath(new URL(`../../../../specs/fixtures/${name}/cases.json`, import.meta.url));
  return JSON.parse(readFileSync(path, "utf8")) as Fixture;
}

const contract = fixture("x509-extension-v1");
const upstream = fixture("der-asn1-v1");

function materialize(segments: Segment[]): Uint8Array {
  const output: number[] = [];
  for (const segment of segments) {
    const value = segment.hex ?? segment.repeat_hex!;
    const octets: number[] = [];
    for (let index = 0; index < value.length; index += 2) octets.push(Number.parseInt(value.slice(index, index + 2), 16));
    const count = segment.hex === undefined ? segment.count! : 1;
    for (let index = 0; index < count; index += 1) output.push(...octets);
  }
  return Uint8Array.from(output);
}

function numeric(value: unknown): number {
  if (typeof value !== "number") throw new Error("expected numeric fixture limit");
  return value;
}

function limits(testCase: FixtureCase): Asn1Limits {
  const overrides = testCase.limits ?? {};
  const defaults = upstream.defaults;
  const derDefaults = defaults.der as Record<string, unknown>;
  const derOverrides = (overrides.der ?? {}) as Record<string, unknown>;
  const value = (name: string): unknown => overrides[name] ?? defaults[name];
  const derValue = (name: string): unknown => derOverrides[name] ?? derDefaults[name];
  const der: DerLimits = {
    maxInputLen: numeric(derValue("max_input_len")),
    maxValueLen: derValue("max_value_len") === "host-max"
      ? BigInt(Number.MAX_SAFE_INTEGER)
      : BigInt(numeric(derValue("max_value_len"))),
    maxElements: numeric(derValue("max_elements")),
    maxTagNumber: numeric(derValue("max_tag_number")),
  };
  return {
    der,
    maxDepth: numeric(value("max_depth")),
    maxTotalElements: numeric(value("max_total_elements")),
    maxOidArcs: numeric(value("max_oid_arcs")),
  };
}

function hex(value: Uint8Array): string {
  return [...value].map((octet) => octet.toString(16).padStart(2, "0")).join("");
}

function attempt(decoder: Asn1Decoder, root: Asn1Element): Record<string, unknown> {
  try {
    const value = decodeX509Extension(decoder, root);
    return {
      outcome: "value",
      extension_id_arcs_decimal: value.extensionId.arcs.map((arc) => arc.toString()),
      critical: value.critical,
      extension_value_hex: hex(value.extensionValue),
      elements_read: decoder.elementsRead,
    };
  } catch (error: unknown) {
    expect(error).toBeInstanceOf(X509ExtensionError);
    const typed = error as X509ExtensionError;
    const result: Record<string, unknown> = {
      outcome: "error",
      error_id: typed.kind,
      offset: typed.offset,
      offset_scope: "extension-element",
      elements_read: decoder.elementsRead,
    };
    if (typed.asn1Kind !== undefined) result.asn1_error_id = typed.asn1Kind;
    if (typed.framingKind !== undefined) result.framing_error_id = typed.framingKind;
    return result;
  }
}

describe("portable x509-extension-v1", () => {
  it("consumes every closed case", () => {
    expect(contract.cases).toHaveLength(48);
    for (const testCase of contract.cases) {
      const decoder = new Asn1Decoder(limits(testCase));
      const root = decoder.decodeExact(materialize(testCase.input));
      const actual = testCase.operation === "extension-script"
        ? { outcome: "script", events: testCase.actions!.map(() => attempt(decoder, root)) }
        : attempt(decoder, root);
      expect(actual, testCase.id).toEqual(testCase.expected);
      if (testCase.redacted_input_hex !== undefined) {
        expect(JSON.stringify(actual), testCase.id).not.toContain(testCase.redacted_input_hex);
      }
    }
  });

  it("blocks forged values and returns defensive bytes", () => {
    expect(() => new X509Extension({}, {} as never, false, new Uint8Array())).toThrow(TypeError);
    const decoder = new Asn1Decoder();
    const value = decodeX509Extension(decoder, decoder.decodeExact(materialize([{ hex: "30090603551d1104023000" }])));
    const first = value.extensionValue;
    first[0] = 0xff;
    expect(hex(value.extensionValue)).toBe("3000");
  });
});
