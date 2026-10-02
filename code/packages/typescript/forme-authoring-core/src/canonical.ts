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

export function encodeCanonicalJson(value: unknown): Uint8Array {
  return new TextEncoder().encode(canonicalJson(value));
}
