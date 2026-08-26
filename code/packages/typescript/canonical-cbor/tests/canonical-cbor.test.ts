import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

import {
  CborArray,
  CborBoolean,
  CborByteString,
  CborError,
  CborMap,
  CborMapEntry,
  CborNegative,
  CborNull,
  CborTag,
  CborText,
  CborUnsigned,
  MAX_ENCODED_BYTES,
  MAX_NESTING_DEPTH,
  decode,
  encodeChecked,
  encodeIntoChecked,
  type CborValue,
} from "../src/index";

type FixtureCase = {
  id: string;
  operation: string;
  input: string;
  expected: string;
};

type Fixture = {
  schema_version: number;
  profile: string;
  limits: { max_nesting_depth: number; max_encoded_bytes: number };
  error_ids: string[];
  cases: FixtureCase[];
};

const EXPECTED_ERRORS = [
  "unexpected-eof",
  "trailing-bytes",
  "reserved",
  "indefinite",
  "non-minimal-integer",
  "invalid-utf8",
  "non-canonical-map-order",
  "unsupported-simple",
  "float-not-supported",
  "too-deep",
  "length-too-large",
  "duplicate-map-key",
  "encode-too-deep",
  "encode-too-large",
];

const fixturePath = join(
  dirname(fileURLToPath(import.meta.url)),
  "../../../../specs/fixtures/canonical-cbor-v1/cases.json",
);
const fixture = JSON.parse(readFileSync(fixturePath, "utf8")) as Fixture;

describe("CBR01 portable conformance", () => {
  it("matches every language-neutral fixture case", () => {
    expect(fixture.schema_version).toBe(1);
    expect(fixture.profile).toBe("rfc8949-section-4.2.3-length-first");
    expect(fixture.limits).toEqual({
      max_nesting_depth: MAX_NESTING_DEPTH,
      max_encoded_bytes: MAX_ENCODED_BYTES,
    });
    expect(fixture.error_ids).toEqual(EXPECTED_ERRORS);
    expect(fixture.cases).toHaveLength(55);

    for (const testCase of fixture.cases) {
      const { id, input, expected, operation } = testCase;
      if (operation === "round-trip") {
        expect(encodeChecked(decode(fromHex(input))), id).toEqual(fromHex(expected));
      } else if (operation === "decode-error") {
        const wire = decodeErrorWire(input);
        expectCborError(expected, () => decode(wire), id);
      } else if (operation === "encode-map") {
        expect(encodeChecked(mapValue(input)), id).toEqual(fromHex(expected));
      } else if (operation === "generated-round-trip") {
        expect(encodeChecked(generatedValue(input)), id).toEqual(generatedWire(expected));
      } else if (operation === "encode-error") {
        const value = input === "duplicate-map-key"
          ? mapValue("6161=00;6161=01")
          : generatedValue(input);
        expectCborError(expected, () => encodeChecked(value), id);
        const destination = [0xaa];
        expectCborError(expected, () => encodeIntoChecked(value, destination), id);
        expect(destination).toEqual([0xaa]);
      } else {
        throw new Error(`unknown fixture operation: ${operation}`);
      }
    }
  });
});

describe("host-language safety edges", () => {
  it("preserves the full unsigned 64-bit domains with bigint", () => {
    const maximum = (1n << 64n) - 1n;
    expect(encodeChecked(new CborUnsigned(maximum))).toEqual(fromHex("1bffffffffffffffff"));
    expect((decode(fromHex("1bffffffffffffffff")) as CborUnsigned).value).toBe(maximum);
    expect(encodeChecked(new CborNegative(maximum))).toEqual(fromHex("3bffffffffffffffff"));
    expect(encodeChecked(new CborTag(maximum, new CborUnsigned(0n)))).toEqual(
      fromHex("dbffffffffffffffff00"),
    );
    expect(() => new CborUnsigned(-1n)).toThrow(RangeError);
    expect(() => new CborTag(1n << 64n, CborNull.instance)).toThrow(RangeError);
  });

  it("defensively copies byte strings and collections", () => {
    const source = new Uint8Array([1, 2, 3]);
    const bytes = new CborByteString(source);
    source[0] = 9;
    const exposed = bytes.value;
    exposed[1] = 9;
    expect(bytes.value).toEqual(new Uint8Array([1, 2, 3]));

    const values: CborValue[] = [new CborUnsigned(1n)];
    const array = new CborArray(values);
    values.push(new CborUnsigned(2n));
    const copied = array.values;
    copied.push(new CborUnsigned(3n));
    expect(array.values).toHaveLength(1);

    const entries = [new CborMapEntry(new CborText("a"), new CborUnsigned(1n))];
    const map = new CborMap(entries);
    entries.push(new CborMapEntry(new CborText("b"), new CborUnsigned(2n)));
    expect(map.entries).toHaveLength(1);
  });

  it("keeps validated values immutable at runtime", () => {
    const unsigned = new CborUnsigned(1n);
    expect(() => {
      (unsigned as unknown as { value: bigint }).value = -1n;
    }).toThrow(TypeError);
    expect(encodeChecked(unsigned)).toEqual(fromHex("01"));

    const text = new CborText("ok");
    expect(() => {
      (text as unknown as { value: string }).value = "\ud800";
    }).toThrow(TypeError);
    expect(encodeChecked(text)).toEqual(fromHex("626f6b"));

    const entry = new CborMapEntry(new CborText("a"), new CborUnsigned(1n));
    const map = new CborMap([entry]);
    expect(() => {
      (entry as unknown as { key: CborValue }).key = new CborText("b");
    }).toThrow(TypeError);
    expect(encodeChecked(map)).toEqual(fromHex("a1616101"));

    expect(() => {
      (CborNull as unknown as { instance: CborValue }).instance = new CborUnsigned(7n);
    }).toThrow(TypeError);
    expect(decode(fromHex("f6"))).toBe(CborNull.instance);
  });

  it("rejects erased TypeScript types at the JavaScript boundary", () => {
    expect(() => new CborText(3 as unknown as string)).toThrow(/must be a string/);
    expect(() => new CborBoolean("false" as unknown as boolean)).toThrow(/must be a boolean/);
    expect(() => new CborByteString([1, 2] as unknown as Uint8Array)).toThrow(/Uint8Array/);
    expect(() => new CborArray("x" as unknown as CborValue[])).toThrow(/array items/);
    expect(() => new CborArray(["x" as unknown as CborValue])).toThrow(/array items/);
    expect(() => new CborMap("x" as unknown as CborMapEntry[])).toThrow(/map entries/);
    expect(() => new CborMap(["x" as unknown as CborMapEntry])).toThrow(/map entries/);
    expect(() => new CborMapEntry("x" as unknown as CborValue, CborNull.instance)).toThrow(
      /CborValue key and value/,
    );
    expect(() => new CborTag(0n, "x" as unknown as CborValue)).toThrow(/tag value/);
    expect(() => decode(16 as unknown as Uint8Array)).toThrow(/decode input/);
    expect(() => decode([0] as unknown as Uint8Array)).toThrow(/decode input/);
    expect(() => encodeIntoChecked(CborNull.instance, {} as number[])).toThrow(/destination/);
  });

  it("revalidates subclass getters before checked encoding", () => {
    class InvalidUnsigned extends CborUnsigned {
      override get value(): bigint {
        return -1n;
      }
    }
    class InvalidText extends CborText {
      override get value(): string {
        return "\ud800";
      }
    }
    class InvalidBoolean extends CborBoolean {
      override get value(): boolean {
        return "true" as unknown as boolean;
      }
    }

    expect(() => encodeChecked(new InvalidUnsigned(0n))).toThrow(/unsigned 64-bit bigint/);
    expect(() => encodeChecked(new InvalidText("safe"))).toThrow(/Unicode scalar/);
    expect(() => encodeChecked(new InvalidBoolean(false))).toThrow(/must be a boolean/);
  });

  it("uses strict Unicode scalar and UTF-8 handling", () => {
    expect(() => new CborText("\ud800")).toThrow(/Unicode scalar/);
    expect(() => new CborText("\udc00")).toThrow(/Unicode scalar/);
    expect(encodeChecked(new CborText("😀"))).toEqual(fromHex("64f09f9880"));
    for (const wire of ["6180", "62e298", "64f4908080"]) {
      expectCborError("invalid-utf8", () => decode(fromHex(wire)), "private-payload");
    }
    expect((decode(fromHex("63efbbbf")) as CborText).value).toBe("\ufeff");
  });

  it("rejects hostile lengths before allocation and preserves append atomicity", () => {
    for (const wire of [
      "5bffffffffffffffff",
      "7bffffffffffffffff",
      "9bffffffffffffffff",
      "bbffffffffffffffff",
    ]) {
      expectCborError("length-too-large", () => decode(fromHex(wire)), "hostile");
    }

    const destination = [0xaa];
    encodeIntoChecked(new CborUnsigned(24n), destination);
    expect(destination).toEqual([0xaa, 0x18, 0x18]);
    const tooLarge = new CborByteString(new Uint8Array(MAX_ENCODED_BYTES));
    expectCborError("encode-too-large", () => encodeIntoChecked(tooLarge, destination), "atomic");
    expect(destination).toEqual([0xaa, 0x18, 0x18]);

    const fullDestination = new Array<number>(0xffff_fffe);
    expect(() => encodeIntoChecked(new CborUnsigned(24n), fullDestination)).toThrow(RangeError);
    expect(fullDestination).toHaveLength(0xffff_fffe);
  });

  it("applies depth and size limits exactly", () => {
    let value: CborValue = CborNull.instance;
    for (let depth = 0; depth < MAX_NESTING_DEPTH; depth += 1) {
      value = new CborTag(0n, value);
    }
    const accepted = new Uint8Array([...new Uint8Array(MAX_NESTING_DEPTH).fill(0xc0), 0xf6]);
    expect(encodeChecked(value)).toEqual(accepted);
    expect(encodeChecked(decode(accepted))).toEqual(accepted);
    expectCborError(
      "encode-too-deep",
      () => encodeChecked(new CborTag(0n, value)),
      "private-depth-payload",
    );
    const rejected = new Uint8Array([...new Uint8Array(MAX_NESTING_DEPTH + 1).fill(0xc0), 0xf6]);
    expectCborError("too-deep", () => decode(rejected), "private-depth-payload");

    const exactText = `${"\u0800".repeat(349_523)}aa`;
    expect(encodeChecked(new CborText(exactText))).toHaveLength(MAX_ENCODED_BYTES);
    expectCborError(
      "encode-too-large",
      () => encodeChecked(new CborText(`${exactText}a`)),
      "cap",
    );
    const oversizedWire = byteStringWire(MAX_ENCODED_BYTES, 0x5a);
    expectCborError("encode-too-large", () => encodeChecked(decode(oversizedWire)), "decode-cap");
  });

  it("sorts encoded map keys and catches encoded duplicates", () => {
    const map = new CborMap([
      new CborMapEntry(new CborByteString(new Uint8Array([0x80])), new CborUnsigned(1n)),
      new CborMapEntry(new CborByteString(new Uint8Array([0x7f])), new CborUnsigned(0n)),
    ]);
    expect(encodeChecked(map)).toEqual(fromHex("a2417f00418001"));
    const duplicate = new CborMap([
      new CborMapEntry(new CborByteString(new Uint8Array([1])), CborNull.instance),
      new CborMapEntry(new CborByteString(new Uint8Array([1])), CborNull.instance),
    ]);
    expectCborError("duplicate-map-key", () => encodeChecked(duplicate), "secret-key-payload");
  });

  it("keeps every codec error stable and payload blind", () => {
    const error = captureCborError(() => decode(fromHex("63e298")));
    expect(error.id).toBe("length-too-large");
    expect(error.message.startsWith("canonical-cbor:")).toBe(true);
    expect(error.message).not.toContain("e298");
    expect(() => new CborBoolean(true)).not.toThrow();
  });

  it("keeps the fixture adapter grammar closed", () => {
    expect(() => decodeErrorWire("nested-array-wire:129:ignored")).toThrow(/fixture hex/);
    expect(() => generatedValue("nested-array:1:ignored")).toThrow(/generated value grammar/);
    expect(() => generatedWire("wire:bytes-repeat:1:00:ignored")).toThrow(
      /generated wire grammar/,
    );
  });
});

function captureCborError(action: () => unknown): CborError {
  try {
    action();
  } catch (error) {
    expect(error).toBeInstanceOf(CborError);
    return error as CborError;
  }
  throw new Error("expected CborError");
}

function expectCborError(id: string, action: () => unknown, secret: string): void {
  const error = captureCborError(action);
  expect(error.id).toBe(id);
  expect(error.message.startsWith("canonical-cbor:")).toBe(true);
  expect(error.message).not.toContain(secret);
}

function mapValue(specification: string): CborMap {
  return new CborMap(specification.split(";").map((fragment) => {
    const parts = fragment.split("=");
    if (parts.length !== 2) throw new Error("invalid map fixture grammar");
    return new CborMapEntry(decode(fromHex(parts[0])), decode(fromHex(parts[1])));
  }));
}

function decodeErrorWire(specification: string): Uint8Array {
  const parts = specification.split(":");
  if (parts[0] === "nested-array-wire" && parts.length === 2) {
    return nestedArrayWire(parseNatural(parts[1]));
  }
  return fromHex(specification);
}

function generatedValue(specification: string): CborValue {
  const parts = specification.split(":");
  if (parts[0] === "nested-array" && parts.length === 2) {
    let value: CborValue = CborNull.instance;
    for (let index = 0; index < parseNatural(parts[1]); index += 1) {
      value = new CborArray([value]);
    }
    return value;
  }
  if (parts[0] === "bytes-repeat" && parts.length === 3) {
    return new CborByteString(new Uint8Array(parseNatural(parts[1])).fill(parseByte(parts[2])));
  }
  throw new Error("invalid generated value grammar");
}

function generatedWire(specification: string): Uint8Array {
  const parts = specification.split(":");
  if (parts[0] === "wire" && parts[1] === "nested-array" && parts.length === 3) {
    return nestedArrayWire(parseNatural(parts[2]));
  }
  if (parts[0] === "wire" && parts[1] === "bytes-repeat" && parts.length === 4) {
    return byteStringWire(parseNatural(parts[2]), parseByte(parts[3]));
  }
  throw new Error("invalid generated wire grammar");
}

function nestedArrayWire(depth: number): Uint8Array {
  return new Uint8Array([...new Uint8Array(depth).fill(0x81), 0xf6]);
}

function byteStringWire(length: number, repeated: number): Uint8Array {
  const result = new Uint8Array(length + 5);
  result.set([0x5a, length >>> 24, length >>> 16, length >>> 8, length]);
  result.fill(repeated, 5);
  return result;
}

function fromHex(value: string): Uint8Array {
  if (!/^(?:[0-9a-fA-F]{2})*$/.test(value)) throw new Error("invalid fixture hex");
  return Uint8Array.from(value.match(/../g)?.map((byte) => Number.parseInt(byte, 16)) ?? []);
}

function parseNatural(value: string): number {
  if (!/^(?:0|[1-9][0-9]*)$/.test(value)) throw new Error("invalid fixture natural");
  const parsed = Number(value);
  if (!Number.isSafeInteger(parsed)) throw new Error("fixture natural is too large");
  return parsed;
}

function parseByte(value: string): number {
  if (!/^[0-9a-fA-F]{2}$/.test(value)) throw new Error("invalid fixture byte");
  return Number.parseInt(value, 16);
}
