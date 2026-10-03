/** Deterministic JSON used for persistence and byte-limit accounting. */

import { AuthoringError } from "./error.js";

function canonicalValue(value: unknown): unknown {
  if (Array.isArray(value)) return value.map(canonicalValue);
  if (value !== null && typeof value === "object") {
    const result: Record<string, unknown> = {};
    for (const key of Object.keys(value).sort()) {
      result[key] = canonicalValue((value as Record<string, unknown>)[key]);
    }
    return result;
  }
  return value;
}

export function canonicalJson(value: unknown): string {
  const result = JSON.stringify(canonicalValue(value));
  if (result === undefined) {
    throw new AuthoringError("INVALID_STATE", "The value cannot be serialized");
  }
  return result;
}

/** Measure canonical UTF-8 bytes incrementally and stop before a large JSON allocation. */
export function canonicalJsonByteLength(value: unknown, maximum: number): number {
  let total = 0;
  const add = (count: number): void => {
    total += count;
    if (total > maximum) throw new AuthoringError("INVALID_STATE", "The canonical value exceeds the byte limit");
  };
  const walk = (item: unknown): void => {
    if (Array.isArray(item)) {
      add(2 + Math.max(0, item.length - 1));
      for (const child of item) walk(child);
    } else if (item !== null && typeof item === "object") {
      const keys = Object.keys(item).sort();
      add(2 + Math.max(0, keys.length - 1));
      for (const key of keys) {
        add(new TextEncoder().encode(JSON.stringify(key)).byteLength + 1);
        walk((item as Record<string, unknown>)[key]);
      }
    } else {
      const scalar = JSON.stringify(item);
      if (scalar === undefined) throw new AuthoringError("INVALID_STATE", "The value cannot be serialized");
      add(new TextEncoder().encode(scalar).byteLength);
    }
  };
  walk(value);
  return total;
}

export function encodeCanonicalJson(value: unknown): Uint8Array {
  return new TextEncoder().encode(canonicalJson(value));
}
