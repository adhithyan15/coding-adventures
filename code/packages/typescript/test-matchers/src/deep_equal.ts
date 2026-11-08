import { Value } from "./types.js";

const CANONICAL_NAN = 0x7ff8_0000_0000_0000n;
const MAX_DEPTH = 64;

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
  if (a === b) {
    return true;
  }
  if (a.kind !== b.kind) {
    return false;
  }
  switch (a.kind) {
    case "null":
      return true;
    case "undefined":
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
    case "date":
      return a.ms === (b as { kind: "date"; ms: number }).ms;
    case "regexp": {
      const bRe = b as { kind: "regexp"; source: string; flags: string };
      return a.source === bRe.source && a.flags === bRe.flags;
    }
    case "error": {
      const bErr = b as { kind: "error"; name: string; message: string };
      return a.name === bErr.name && a.message === bErr.message;
    }
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
    default:
      return false;
  }
}

/**
 * Encodes a Value into token syntax for reporting and diagnostic messages.
 */
export function encodeValue(value: Value): string {
  switch (value.kind) {
    case "null":
      return "null";
    case "undefined":
      return "undefined";
    case "bool":
      return value.value ? "true" : "false";
    case "int":
      return `${value.text}n`;
    case "f64": {
      const num = f64BitsToNumber(value.bits);
      if (Number.isNaN(num)) return "NaN";
      if (Object.is(num, -0)) return "-0";
      return String(num);
    }
    case "bytes": {
      const hex = Array.from(value.value)
        .map((b) => b.toString(16).padStart(2, "0"))
        .join("");
      return `Uint8Array([${hex}])`;
    }
    case "text":
      return JSON.stringify(value.value);
    case "date":
      return `Date(${value.ms})`;
    case "regexp":
      return `/${value.source}/${value.flags}`;
    case "error":
      return `${value.name}: ${value.message}`;
    case "list": {
      const parts = value.items.map(encodeValue);
      return `[${parts.join(", ")}]`;
    }
    case "map": {
      const parts = value.entries.map(
        ([k, v]) => `${encodeValue(k)}: ${encodeValue(v)}`
      );
      return `{${parts.join(", ")}}`;
    }
    default:
      return "unknown";
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
 * Converts an arbitrary JavaScript data structure to a Value.
 * Guards against circular references using an active recursion stack,
 * memoizes DAG nodes to prevent exponential re-traversal,
 * and enforces a maximum depth ceiling of 64 (TF01 §2 rule 5).
 */
export function toValue(
  val: unknown,
  depth = 0,
  activeStack = new Set<unknown>(),
  memo = new WeakMap<object, Value>()
): Value {
  if (depth > MAX_DEPTH) {
    throw new Error(`Maximum nesting depth ${MAX_DEPTH} exceeded`);
  }
  if (val === null) {
    return { kind: "null" };
  }
  if (val === undefined) {
    return { kind: "undefined" };
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
  if (val instanceof Date) {
    return { kind: "date", ms: val.getTime() };
  }
  if (val instanceof RegExp) {
    return { kind: "regexp", source: val.source, flags: val.flags };
  }
  if (val instanceof Error) {
    return { kind: "error", name: val.name, message: val.message };
  }

  if (typeof val === "object") {
    if (activeStack.has(val)) {
      return { kind: "text", value: "[Circular]" };
    }
    const cached = memo.get(val as object);
    if (cached) {
      return cached;
    }

    activeStack.add(val);
    let result: Value;
    try {
      if (Array.isArray(val)) {
        result = {
          kind: "list",
          items: val.map((item) => toValue(item, depth + 1, activeStack, memo)),
        };
      } else if (val instanceof Set) {
        result = {
          kind: "list",
          items: Array.from(val).map((item) =>
            toValue(item, depth + 1, activeStack, memo)
          ),
        };
      } else if (val instanceof Map) {
        const entries: [Value, Value][] = [];
        for (const [k, v] of val.entries()) {
          entries.push([
            toValue(k, depth + 1, activeStack, memo),
            toValue(v, depth + 1, activeStack, memo),
          ]);
        }
        result = { kind: "map", entries };
      } else {
        const entries: [Value, Value][] = [];
        const keys = Object.keys(val as object).sort();
        for (const k of keys) {
          entries.push([
            { kind: "text", value: k },
            toValue(
              (val as Record<string, unknown>)[k],
              depth + 1,
              activeStack,
              memo
            ),
          ]);
        }
        result = { kind: "map", entries };
      }
    } finally {
      activeStack.delete(val);
    }
    memo.set(val as object, result);
    return result;
  }

  return { kind: "text", value: String(val) };
}

/**
 * Convenience helper to compare two JavaScript values for deep equality.
 */
export function deepEqual(a: unknown, b: unknown): boolean {
  return valuesEqual(toValue(a), toValue(b));
}
