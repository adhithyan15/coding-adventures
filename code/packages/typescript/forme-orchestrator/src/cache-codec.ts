/** Deterministic tagged codec for materialized Forme stage values. */

type EncodedValue =
  | ["null"]
  | ["undefined"]
  | ["boolean", boolean]
  | ["number", string]
  | ["string", string]
  | ["bytes", string]
  | ["array", EncodedValue[]]
  | ["object", Array<[string, EncodedValue]>];

export interface CachedStageOutput {
  readonly value: unknown;
  readonly isStream: boolean;
}

const encoder = new TextEncoder();
const decoder = new TextDecoder("utf-8", { fatal: true });

export function encodeCacheValue(value: unknown): Uint8Array {
  return encoder.encode(JSON.stringify(encodeValue(value, new Set<object>())));
}

export function decodeCacheValue(payload: Uint8Array): unknown {
  return decodeValue(JSON.parse(decoder.decode(payload)) as unknown);
}

export function encodeCachedStageOutput(output: CachedStageOutput): Uint8Array {
  return encodeCacheValue({
    schema: "forme-stage-output-v1",
    isStream: output.isStream,
    value: output.value,
  });
}

export function decodeCachedStageOutput(payload: Uint8Array): CachedStageOutput {
  const decoded = decodeCacheValue(payload);
  if (typeof decoded !== "object" || decoded === null) throw new Error("cache payload is not an object");
  const candidate = decoded as { schema?: unknown; isStream?: unknown; value?: unknown };
  if (candidate.schema !== "forme-stage-output-v1" || typeof candidate.isStream !== "boolean") {
    throw new Error("cache payload has an unsupported stage-output schema");
  }
  return { value: candidate.value, isStream: candidate.isStream };
}

function encodeValue(value: unknown, ancestors: Set<object>): EncodedValue {
  if (value === null) return ["null"];
  if (value === undefined) return ["undefined"];
  if (typeof value === "boolean") return ["boolean", value];
  if (typeof value === "string") return ["string", value];
  if (typeof value === "number") {
    if (!Number.isFinite(value)) throw new Error("cache codec rejects non-finite numbers");
    return ["number", Object.is(value, -0) ? "-0" : String(value)];
  }
  if (value instanceof Uint8Array) return ["bytes", toHex(value)];
  if (typeof value !== "object") throw new Error(`cache codec rejects ${typeof value} values`);
  if (ancestors.has(value)) throw new Error("cache codec rejects cyclic values");
  ancestors.add(value);
  try {
    if (Array.isArray(value)) return ["array", value.map(item => encodeValue(item, ancestors))];
    const prototype = Object.getPrototypeOf(value);
    if (prototype !== Object.prototype && prototype !== null) {
      throw new Error("cache codec accepts only arrays, bytes, and plain objects");
    }
    return ["object", Object.keys(value as object).sort().map(key => [
      key,
      encodeValue((value as Record<string, unknown>)[key], ancestors),
    ])];
  } finally {
    ancestors.delete(value);
  }
}

function decodeValue(value: unknown): unknown {
  if (!Array.isArray(value) || typeof value[0] !== "string") throw new Error("malformed cache value");
  switch (value[0]) {
    case "null": return null;
    case "undefined": return undefined;
    case "boolean":
      if (typeof value[1] !== "boolean") throw new Error("malformed cached boolean");
      return value[1];
    case "number":
      if (typeof value[1] !== "string") throw new Error("malformed cached number");
      if (value[1] === "-0") return -0;
      {
        const number = Number(value[1]);
        if (!Number.isFinite(number) || String(number) !== value[1]) throw new Error("malformed cached number");
        return number;
      }
    case "string":
      if (typeof value[1] !== "string") throw new Error("malformed cached string");
      return value[1];
    case "bytes":
      if (typeof value[1] !== "string") throw new Error("malformed cached bytes");
      return fromHex(value[1]);
    case "array":
      if (!Array.isArray(value[1])) throw new Error("malformed cached array");
      return value[1].map(decodeValue);
    case "object": {
      if (!Array.isArray(value[1])) throw new Error("malformed cached object");
      const result: Record<string, unknown> = {};
      let previous: string | null = null;
      for (const pair of value[1]) {
        if (!Array.isArray(pair) || pair.length !== 2 || typeof pair[0] !== "string") {
          throw new Error("malformed cached object entry");
        }
        if (previous !== null && pair[0] <= previous) throw new Error("cached object keys are not canonical");
        previous = pair[0];
        Object.defineProperty(result, pair[0], {
          value: decodeValue(pair[1]),
          enumerable: true,
          configurable: true,
          writable: true,
        });
      }
      return result;
    }
    default: throw new Error(`unsupported cache value tag ${JSON.stringify(value[0])}`);
  }
}

function toHex(bytes: Uint8Array): string {
  let result = "";
  for (const byte of bytes) result += byte.toString(16).padStart(2, "0");
  return result;
}

function fromHex(value: string): Uint8Array {
  if (value.length % 2 !== 0 || !/^[0-9a-f]*$/.test(value)) throw new Error("malformed cached bytes");
  const bytes = new Uint8Array(value.length / 2);
  for (let index = 0; index < bytes.length; index++) {
    bytes[index] = Number.parseInt(value.slice(index * 2, index * 2 + 2), 16);
  }
  return bytes;
}
