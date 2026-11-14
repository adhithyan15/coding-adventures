/**
 * # Warrant types and values, as this lane represents them
 *
 * A Warrant value crosses three boundaries, and each has its own encoding:
 *
 * | boundary       | encoding                                              |
 * |----------------|-------------------------------------------------------|
 * | corpus JSON    | tagged objects: `{"type": "Bytes", "value": "00"}`    |
 * | `cases.tfv`    | TFV tokens (see `tfv.ts`)                             |
 * | stream digest  | `warrant-stream-v1` bytes (below)                     |
 *
 * ## Two forms of a value
 *
 * Inside the runner a value is *canonical*: the same shape whatever the library
 * under test prefers, so the model, the digests and the comparison never depend
 * on a binding.
 *
 * | Warrant type    | canonical form              | library form (the binding) |
 * |-----------------|-----------------------------|----------------------------|
 * | `Bool`          | `boolean`                   | `boolean`                  |
 * | `U8`            | `bigint`                    | `number`                   |
 * | `U64`, `Nat`    | `bigint`                    | `bigint`                   |
 * | `Bytes`         | `Uint8Array`                | `Uint8Array`               |
 * | `Result<T,E>`   | `["ok", v]` / `["err", "V"]`| the same, around library T |
 *
 * `U64` is `bigint` everywhere, because a JavaScript `number` is a double and
 * silently rounds above 2^53: a `U64` that went through one would still look
 * like a number and compare equal to its neighbour. `U8` fits a `number`
 * exactly, and that is how a library naturally takes a byte.
 *
 * ## Why exact types matter
 *
 * JavaScript's `==` is generous in exactly the wrong places (`1 == true`,
 * `"1" == 1`), and `typeof new Uint8Array()` and `typeof Buffer.alloc(0)` are
 * both `"object"`. A comparison that generous would let a library return the
 * wrong *type* and still pass. So a library result is first checked with
 * {@link checkValue}, which asks for the exact type (a `bigint`, a `Uint8Array`
 * whose prototype *is* `Uint8Array.prototype`, not a `Buffer`), and only then is
 * it compared.
 *
 * ## The stream encoding (VER04i section 2.6)
 *
 * Digests are over bytes that every lane can produce identically:
 *
 * | value          | bytes                                       |
 * |----------------|---------------------------------------------|
 * | `Bool`         | `00` or `01`                                |
 * | `U8/U64/Nat`   | `10` + 8 bytes little-endian (unsigned)     |
 * | `Bytes`        | `20` + u32-LE length + the bytes            |
 * | `Ok(v)`        | `50` + encoding of `v`                      |
 * | `Err(E.V)`     | `51` + u32-LE length + UTF-8 variant name   |
 */

import { createHash } from "node:crypto";
import { CorpusError, need } from "./errors.ts";

export const U64_MAX = (1n << 64n) - 1n;
export const NAT_MAX = U64_MAX;
/** VER04h `MAX_FIXED_WIDTH` bounds every `Bytes<N>`. */
export const MAX_WIDTH = 4096;

const DECIMAL = /^(?:0|[1-9][0-9]*)$/;
const HEX = /^(?:[0-9a-f]{2})*$/;
const BYTES_TYPE = /^Bytes(?:<([0-9]+|[A-Z][A-Za-z0-9_]*)>)?$/;
const ERROR_NAME = /^[A-Z][A-Za-z0-9_]*$/;

export type Kind = "Bool" | "U8" | "U64" | "Nat" | "Bytes" | "Result";

/** A canonical `Result` value (also the shape of an `err`/`name` IR value). */
export type Pair = readonly [string, unknown];

/**
 * A parsed Warrant type.
 *
 * `width` is set for `Bytes<N>`: a number for a literal width, or the string
 * name of a width parameter (`"N"`) in an operation signature. `ok` and
 * `error` are set for `Result`.
 */
export interface Type {
  readonly kind: Kind;
  readonly text: string;
  readonly width?: number | string;
  readonly ok?: Type;
  readonly error?: string;
}

/** Each declared error type and its variant names. */
export type ErrorTypes = ReadonlyMap<string, readonly string[]>;

/** An operation's signature as the corpus declares it. */
export interface OpSig {
  readonly name: string;
  readonly params: readonly (readonly [string, Type])[];
  readonly ret: Type;
  readonly widthParams: readonly string[];
  readonly fresh: boolean;
}

export const INT_KINDS: readonly Kind[] = ["U8", "U64", "Nat"];

/** The largest value of an integer kind. */
export function upperBound(kind: Kind): bigint {
  return kind === "U8" ? 255n : U64_MAX;
}

function splitTopLevelComma(text: string): [string, string] {
  let depth = 0;
  for (let index = 0; index < text.length; index += 1) {
    const char = text.charAt(index);
    if (char === "<") depth += 1;
    else if (char === ">") depth -= 1;
    else if (char === "," && depth === 0) return [text.slice(0, index), text.slice(index + 1)];
  }
  throw new CorpusError("unsupported-type", "malformed Result type");
}

/** Parse a type name; anything outside the closed profile is an error. */
export function parseType(text: string): Type {
  if (text === "Bool" || text === "U8" || text === "U64" || text === "Nat") {
    return { kind: text, text };
  }
  const match = BYTES_TYPE.exec(text);
  if (match) {
    const raw = match[1];
    if (raw === undefined) return { kind: "Bytes", text };
    const width = /^[0-9]+$/.test(raw) ? Number(raw) : raw;
    need(
      typeof width !== "number" || width <= MAX_WIDTH,
      "unsupported-type",
      "Bytes width exceeds the fixed-width bound",
    );
    return { kind: "Bytes", text, width };
  }
  if (text.startsWith("Result<") && text.endsWith(">")) {
    const [okText, error] = splitTopLevelComma(text.slice("Result<".length, -1));
    need(ERROR_NAME.test(error), "unsupported-type", "Result error type must be a declared error name");
    return { kind: "Result", text, ok: parseType(okText), error };
  }
  throw new CorpusError("unsupported-type", "type is outside the warrant-corpus-v1 profile");
}

// ---------------------------------------------------------------------------
// Comparison and hex
// ---------------------------------------------------------------------------

/** Decode lowercase even-length hex (the caller has already validated it). */
export function fromHex(text: string): Uint8Array {
  const out = new Uint8Array(text.length / 2);
  for (let index = 0; index < out.length; index += 1) {
    out[index] = parseInt(text.slice(index * 2, index * 2 + 2), 16);
  }
  return out;
}

/** Lowercase hex of `data`. */
export function toHex(data: Uint8Array): string {
  return Array.from(data, (byte) => byte.toString(16).padStart(2, "0")).join("");
}

/** Do two byte strings hold the same bytes? */
export function bytesEqual(left: Uint8Array, right: Uint8Array): boolean {
  if (left.length !== right.length) return false;
  for (let index = 0; index < left.length; index += 1) {
    if (left[index] !== right[index]) return false;
  }
  return true;
}

/**
 * Structural equality of two *canonical* values.
 *
 * Strict: a `boolean` never equals a `bigint`, and a `Uint8Array` equals only
 * another `Uint8Array` with the same bytes.
 */
export function valuesEqual(left: unknown, right: unknown): boolean {
  if (typeof left === "boolean" || typeof left === "bigint" || typeof left === "string") {
    return left === right;
  }
  if (left instanceof Uint8Array) {
    return right instanceof Uint8Array && bytesEqual(left, right);
  }
  if (Array.isArray(left)) {
    return (
      Array.isArray(right) &&
      left.length === right.length &&
      left.every((item, index) => valuesEqual(item, right[index]))
    );
  }
  return false;
}

// ---------------------------------------------------------------------------
// Corpus JSON  ->  canonical value
// ---------------------------------------------------------------------------
function isTagged(raw: unknown, text: string): raw is Record<string, unknown> {
  return (
    typeof raw === "object" &&
    raw !== null &&
    !Array.isArray(raw) &&
    Object.keys(raw).length === 2 &&
    "type" in raw &&
    "value" in raw &&
    (raw as Record<string, unknown>)["type"] === text
  );
}

function canonicalInt(text: unknown, upper: bigint, what: string): bigint {
  need(typeof text === "string" && DECIMAL.test(text), "value", `${what} must be canonical decimal text`);
  const number = BigInt(text);
  need(number <= upper, "value", `${what} is out of range`);
  return number;
}

/**
 * Decode a corpus JSON value of declared type `ty`.
 *
 * `errors` maps each declared error type to its variant names, so a `Result`
 * can only carry a variant the Warrant module declared. Integers and byte
 * strings are tagged objects, never bare JSON numbers: a `U64` must not lose its
 * low bits to a double.
 */
export function decodeJson(raw: unknown, ty: Type, errors: ErrorTypes): unknown {
  if (ty.kind === "Bool") {
    need(typeof raw === "boolean", "value", "Bool value must be a JSON boolean");
    return raw;
  }
  if (ty.kind === "Result") {
    need(isTagged(raw, ty.text), "value", "Result value must be tagged with its type");
    const body = (raw as Record<string, unknown>)["value"];
    need(
      typeof body === "object" && body !== null && !Array.isArray(body) && Object.keys(body).length === 1,
      "value",
      "invalid Result body",
    );
    const fields = body as Record<string, unknown>;
    if ("ok" in fields) return ["ok", decodeJson(fields["ok"], ty.ok as Type, errors)];
    const variant = fields["err"];
    need(
      typeof variant === "string" && (errors.get(ty.error as string) ?? []).includes(variant),
      "value",
      "Result error is not a declared variant",
    );
    return ["err", variant];
  }
  need(isTagged(raw, ty.text), "value", `${ty.text} value must be a tagged object`);
  const text = (raw as Record<string, unknown>)["value"];
  if (ty.kind === "Bytes") {
    need(typeof text === "string" && HEX.test(text), "value", "Bytes value must be lowercase even-length hex");
    const data = fromHex(text);
    if (typeof ty.width === "number") need(data.length === ty.width, "value", "Bytes width mismatch");
    return data;
  }
  return canonicalInt(text, upperBound(ty.kind), ty.kind);
}

// ---------------------------------------------------------------------------
// Canonical <-> library form
// ---------------------------------------------------------------------------

/** The form a library takes an argument in: a `U8` is a `number`, `Bytes` a fresh copy. */
export function lower(value: unknown, ty: Type): unknown {
  if (ty.kind === "U8") return Number(value as bigint);
  if (ty.kind === "Bytes") return (value as Uint8Array).slice();
  return value;
}

/** Library form -> canonical form. The value has already passed {@link checkValue}. */
export function lift(value: unknown, ty: Type): unknown {
  if (ty.kind === "U8") return BigInt(value as number);
  if (ty.kind === "Result") {
    const pair = value as Pair;
    return pair[0] === "ok" ? ["ok", lift(pair[1], ty.ok as Type)] : pair;
  }
  return value;
}

// ---------------------------------------------------------------------------
// Strict type check of a *result* (the thing a library returned)
// ---------------------------------------------------------------------------

/**
 * Is `value` exactly a `ty`, in library form?
 *
 * "Exactly" is the point: a `number` is not a `U64`, a `Buffer` or a subclass is
 * not `Bytes`, a detached array is not `Bytes`, and a `Result` must carry a
 * declared variant.
 */
export function checkValue(value: unknown, ty: Type, errors: ErrorTypes): boolean {
  switch (ty.kind) {
    case "Bool":
      return typeof value === "boolean";
    case "Bytes":
      return (
        value instanceof Uint8Array &&
        Object.getPrototypeOf(value) === Uint8Array.prototype &&
        !isDetached(value)
      );
    case "U8":
      return typeof value === "number" && Number.isInteger(value) && value >= 0 && value <= 255;
    case "U64":
    case "Nat":
      return typeof value === "bigint" && value >= 0n && value <= U64_MAX;
    case "Result": {
      if (!Array.isArray(value) || value.length !== 2) return false;
      if (value[0] === "ok") return checkValue(value[1], ty.ok as Type, errors);
      return (
        value[0] === "err" &&
        typeof value[1] === "string" &&
        (errors.get(ty.error as string) ?? []).includes(value[1])
      );
    }
  }
}

/** A detached `ArrayBuffer` (transferred away) leaves a zero-length view behind. */
function isDetached(value: Uint8Array): boolean {
  return (value.buffer as { detached?: boolean }).detached === true;
}

// ---------------------------------------------------------------------------
// Stream encoding
// ---------------------------------------------------------------------------

const CHUNK = 64 * 1024;

/**
 * A SHA-256 over the `warrant-stream-v1` bytes of a sequence of values.
 *
 * Hashing 500,000 tiny values one `update` call at a time would spend most of
 * its time crossing into the native hasher, so values are encoded into a
 * 64 KiB buffer that is flushed when full.
 */
export class StreamHasher {
  private readonly hash = createHash("sha256");
  private readonly buffer = new Uint8Array(CHUNK);
  private readonly view = new DataView(this.buffer.buffer);
  private used = 0;

  constructor(prefix: Uint8Array) {
    this.hash.update(prefix);
  }

  private reserve(count: number): void {
    if (this.used + count > CHUNK) this.flush();
  }

  private flush(): void {
    this.hash.update(this.buffer.subarray(0, this.used));
    this.used = 0;
  }

  /** Encode `value` (canonical form) of type `ty` into the stream. */
  write(value: unknown, ty: Type): void {
    switch (ty.kind) {
      case "Bool":
        this.reserve(1);
        this.buffer[this.used++] = value ? 1 : 0;
        return;
      case "U8":
      case "U64":
      case "Nat":
        this.reserve(9);
        this.buffer[this.used++] = 0x10;
        this.view.setBigUint64(this.used, value as bigint, true);
        this.used += 8;
        return;
      case "Bytes": {
        const data = value as Uint8Array;
        this.reserve(5);
        this.buffer[this.used++] = 0x20;
        this.view.setUint32(this.used, data.length, true);
        this.used += 4;
        if (data.length > CHUNK) {
          this.flush();
          this.hash.update(data);
        } else {
          this.reserve(data.length);
          this.buffer.set(data, this.used);
          this.used += data.length;
        }
        return;
      }
      case "Result": {
        const pair = value as Pair;
        if (pair[0] === "ok") {
          this.reserve(1);
          this.buffer[this.used++] = 0x50;
          this.write(pair[1], ty.ok as Type);
          return;
        }
        const name = new TextEncoder().encode(String(pair[1]));
        this.reserve(5);
        this.buffer[this.used++] = 0x51;
        this.view.setUint32(this.used, name.length, true);
        this.used += 4;
        this.flush();
        this.hash.update(name);
      }
    }
  }

  /** The lowercase hex digest of everything written. */
  hexdigest(): string {
    this.flush();
    return this.hash.digest("hex");
  }
}

function u32(number: number): Uint8Array {
  const out = new Uint8Array(4);
  new DataView(out.buffer).setUint32(0, number, true);
  return out;
}

function concat(parts: readonly Uint8Array[]): Uint8Array {
  const out = new Uint8Array(parts.reduce((sum, part) => sum + part.length, 0));
  let at = 0;
  for (const part of parts) {
    out.set(part, at);
    at += part.length;
  }
  return out;
}

/**
 * The context both digests start with (VER04i 2.6).
 *
 * `"warrant-stream-v1\0"`, the length-prefixed suite id, the binder count, each
 * binder's canonical type name, then the result type name. Because the context
 * fixes every type, the per-case bytes need no delimiters.
 */
export function streamPrefix(suiteId: string, binderTypes: readonly string[], resultType: string): Uint8Array {
  const encoder = new TextEncoder();
  const field = (text: string): Uint8Array => {
    const raw = encoder.encode(text);
    return concat([u32(raw.length), raw]);
  };
  return concat([
    encoder.encode("warrant-stream-v1\u0000"),
    field(suiteId),
    u32(binderTypes.length),
    ...binderTypes.map(field),
    field(resultType),
  ]);
}

// ---------------------------------------------------------------------------
// TFV value  ->  canonical value
// ---------------------------------------------------------------------------

/** A parsed TFV value: `["bool", b]`, `["int", n]`, `["bytes", b]`, `["text", s]`, `["list", ...]`, `["map", ...]`, `["null"]`. */
export type TfvValue =
  | readonly ["null"]
  | readonly ["bool", boolean]
  | readonly ["int", bigint]
  | readonly ["bytes", Uint8Array]
  | readonly ["text", string]
  | readonly ["list", readonly TfvValue[]]
  | readonly ["map", readonly (readonly [TfvValue, TfvValue])[]];

/**
 * Decode a parsed TFV value of declared type `ty` (VER04i section 2.7).
 *
 * `Bool` is `T`/`F`, integers are `I<n>`, bytes are `X<hex>`, and a `Result` is
 * a one-entry map whose key is the text `ok` (value) or `err` (the *variant
 * name* as text).
 */
export function decodeTfv(raw: TfvValue, ty: Type, errors: ErrorTypes): unknown {
  if (ty.kind === "Bool") {
    need(raw[0] === "bool", "tfv-value", "expected a TFV boolean");
    return raw[1];
  }
  if (ty.kind === "Bytes") {
    need(raw[0] === "bytes", "tfv-value", "expected TFV bytes");
    if (typeof ty.width === "number") need(raw[1].length === ty.width, "tfv-value", "TFV Bytes width mismatch");
    return raw[1];
  }
  if (ty.kind !== "Result") {
    need(raw[0] === "int" && raw[1] >= 0n && raw[1] <= upperBound(ty.kind), "tfv-value", "TFV integer out of range");
    return raw[1];
  }
  need(raw[0] === "map" && raw[1].length === 1, "tfv-value", "expected a one-entry TFV map");
  const [key, body] = raw[1][0] as readonly [TfvValue, TfvValue];
  need(key[0] === "text", "tfv-value", "Result key must be text");
  if (key[1] === "ok") return ["ok", decodeTfv(body, ty.ok as Type, errors)];
  need(
    key[1] === "err" && body[0] === "text" && (errors.get(ty.error as string) ?? []).includes(body[1]),
    "tfv-value",
    "Result error is not a declared variant",
  );
  return ["err", (body as readonly ["text", string])[1]];
}
