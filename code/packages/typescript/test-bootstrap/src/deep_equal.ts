import { Value } from "./types.js";

const CANONICAL_NAN = 0x7ff8_0000_0000_0000n;

/**
 * Normalizes NaN bit patterns to the single canonical quiet NaN (TF01 §3.1).
 * Leaves +infinity, -infinity, -0, and ordinary floats unchanged.
 */
export function canonicalF64(bits: bigint): bigint {
  const exponent = (bits >> 52n) & 0x7ffn;
  const mantissa = bits & ((1n << 52n) - 1n);
  if (exponent === 0x7ffn && mantissa !== 0n) {
    return CANONICAL_NAN;
  }
  return bits;
}

/**
 * Decides whether two Value instances are deep-equal (TF01 §3.1).
 */
export function valuesEqual(a: Value, b: Value): boolean {
  if (a.kind !== b.kind) {
    return false;
  }
  switch (a.kind) {
    case "null":
      return true;
    case "bool":
      return a.value === (b as { kind: "bool"; value: boolean }).value;
    case "int":
      return a.text === (b as { kind: "int"; text: string }).text;
    case "f64":
      return (
        canonicalF64(a.bits) ===
        canonicalF64((b as { kind: "f64"; bits: bigint }).bits)
      );
    case "bytes": {
      const bBytes = (b as { kind: "bytes"; value: Uint8Array }).value;
      if (a.value.length !== bBytes.length) {
        return false;
      }
      for (let i = 0; i < a.value.length; i++) {
        if (a.value[i] !== bBytes[i]) {
          return false;
        }
      }
      return true;
    }
    case "text":
      return a.value === (b as { kind: "text"; value: string }).value;
    case "list": {
      const bList = (b as { kind: "list"; items: Value[] }).items;
      if (a.items.length !== bList.length) {
        return false;
      }
      for (let i = 0; i < a.items.length; i++) {
        if (!valuesEqual(a.items[i], bList[i])) {
          return false;
        }
      }
      return true;
    }
    case "map": {
      const bEntries = (b as { kind: "map"; entries: [Value, Value][] }).entries;
      if (a.entries.length !== bEntries.length) {
        return false;
      }
      const matchedB = new Set<number>();
      for (const [aKey, aVal] of a.entries) {
        let found = false;
        for (let j = 0; j < bEntries.length; j++) {
          if (matchedB.has(j)) continue;
          const [bKey, bVal] = bEntries[j];
          if (valuesEqual(aKey, bKey) && valuesEqual(aVal, bVal)) {
            matchedB.add(j);
            found = true;
            break;
          }
        }
        if (!found) {
          return false;
        }
      }
      return matchedB.size === bEntries.length;
    }
  }
}

/**
 * Encodes a Value into TFV token syntax for reporting and diagnostic messages.
 */
export function encodeValue(value: Value): string {
  switch (value.kind) {
    case "null":
      return "N";
    case "bool":
      return value.value ? "T" : "F";
    case "int":
      return `I${value.text}`;
    case "f64":
      return `D${value.bits.toString(16).padStart(16, "0")}`;
    case "bytes": {
      const hex = Array.from(value.value)
        .map((b) => b.toString(16).padStart(2, "0"))
        .join("");
      return `X${hex}`;
    }
    case "text": {
      const encoder = new TextEncoder();
      const bytes = encoder.encode(value.value);
      const hex = Array.from(bytes)
        .map((b) => b.toString(16).padStart(2, "0"))
        .join("");
      return `S${hex}`;
    }
    case "list": {
      const parts = [`L${value.items.length}`];
      for (const item of value.items) {
        parts.push(encodeValue(item));
      }
      return parts.join(" ");
    }
    case "map": {
      const parts = [`M${value.entries.length}`];
      for (const [k, v] of value.entries) {
        parts.push(encodeValue(k));
        parts.push(encodeValue(v));
      }
      return parts.join(" ");
    }
  }
}

/**
 * Converts a JavaScript float64 number to raw 64-bit IEEE bits as bigint.
 */
export function numberToF64Bits(num: number): bigint {
  const buffer = new ArrayBuffer(8);
  const view = new DataView(buffer);
  view.setFloat64(0, num, false); // Big endian
  return view.getBigUint64(0, false);
}

/**
 * Converts raw 64-bit IEEE bits to a JavaScript float64 number.
 */
export function f64BitsToNumber(bits: bigint): number {
  const buffer = new ArrayBuffer(8);
  const view = new DataView(buffer);
  view.setBigUint64(0, bits, false);
  return view.getFloat64(0, false);
}

/**
 * Converts an arbitrary JavaScript data structure to a TF01 Value.
 */
export function toValue(val: unknown): Value {
  if (val === null || val === undefined) {
    return { kind: "null" };
  }
  if (typeof val === "boolean") {
    return { kind: "bool", value: val };
  }
  if (typeof val === "bigint") {
    return { kind: "int", text: val.toString() };
  }
  if (typeof val === "number") {
    return { kind: "f64", bits: numberToF64Bits(val) };
  }
  if (typeof val === "string") {
    return { kind: "text", value: val };
  }
  if (val instanceof Uint8Array) {
    return { kind: "bytes", value: val };
  }
  if (Array.isArray(val)) {
    return { kind: "list", items: val.map(toValue) };
  }
  if (val instanceof Map) {
    const entries: [Value, Value][] = [];
    for (const [k, v] of val.entries()) {
      entries.push([toValue(k), toValue(v)]);
    }
    return { kind: "map", entries };
  }
  if (val instanceof Set) {
    return { kind: "list", items: Array.from(val).map(toValue) };
  }
  if (typeof val === "object") {
    const entries: [Value, Value][] = [];
    const keys = Object.keys(val as object).sort();
    for (const k of keys) {
      entries.push([{ kind: "text", value: k }, toValue((val as Record<string, unknown>)[k])]);
    }
    return { kind: "map", entries };
  }
  return { kind: "text", value: String(val) };
}
