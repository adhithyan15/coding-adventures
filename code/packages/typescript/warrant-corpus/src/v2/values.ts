/**
 * # Values: decode by declared type, encode to the canonical stream, compare
 *
 * ## Representation inside the evaluator
 *
 * The evaluator uses **one exact integer representation with a type tag and
 * wraps per node** (section 7.2).  In JavaScript that means two host forms,
 * chosen by the static type so that no value is ever held in a type that
 * cannot hold it:
 *
 * | Warrant type            | host form  | why                                          |
 * |-------------------------|------------|----------------------------------------------|
 * | `Bool`                  | `boolean`  |                                              |
 * | `U8 U16 U32 I8 I16 I32` | `number`   | integral, in range, never `-0`; exact in a double |
 * | `U64 I64 Nat Int`       | `bigint`   | a double rounds past 2^53 (`MAX` and `MAX ^ 1` would be the *same* double) |
 * | `Bytes`, `Bytes<N>`     | `Uint8Array` |                                            |
 * | `Ok(v)` / `Err(CASE)`   | `{ok: v}` / `{err: "CASE"}` |                             |
 *
 * `number` is the host type of the narrow integers *only at this boundary and
 * inside the evaluator's own wrapping rules* (a 32-bit product goes through
 * `Math.imul`, a 64-bit one through `BigInt.asUintN`): the arithmetic is still
 * exact, which is what section 7.2 requires of a TypeScript lane.
 *
 * ## JSON values are decoded by declared type (section 2.3)
 *
 * The corpus never tags a value with its own type.  The reader holds the
 * declared type and decodes to it, range-checking; a value that does not fit is
 * a refusal (`value`), never a truncation.  Every integer is **text**:
 * `"18446744073709551615"`, with no JavaScript safe-integer hazard.
 *
 * ## The canonical stream, version 2 (section 2.4)
 *
 * | Value                        | Bytes                                         |
 * |------------------------------|-----------------------------------------------|
 * | `Bool`                       | `00` or `01`                                  |
 * | `U8 U16 U32 U64 Nat`         | `10`, then 8 bytes little-endian, unsigned    |
 * | `I8 I16 I32 I64`             | `11`, then 8 bytes two's complement LE, sign-extended |
 * | `Bytes`, `Bytes<N>`          | `20`, u32-LE length, the bytes                |
 * | `Ok(v)` / `Err(E.V)`         | `50` plus inner / `51`, u32-LE length, UTF-8 case name |
 *
 * (The tuple, `Option`, record and variant rows belong to `ir.tuple` and
 * `ir.data`; this lane does not implement them.)
 */

import type { TfvValue } from "../values.ts";
import { KitError } from "./errors.ts";
import { firstMissing } from "./levels.ts";
import { MAX_WIDTH, type Ty, type TypeTable, type Width, showType, typeLevels } from "./types.ts";

/** A runtime value (see the table above). */
export type Value = boolean | number | bigint | Uint8Array | { readonly ok: unknown } | { readonly err: string };

/** The largest `Nat`/`U64` value. */
export const U64_MAX = (1n << 64n) - 1n;
/** The window of the exact integers: `-(2^127-1) ..= 2^127-1` (section 3.2). */
export const INT_WINDOW = (1n << 127n) - 1n;

/** Does the static type hold its values as a `number`? (an integer of at most 32 bits) */
export function isSmall(ty: Ty): boolean {
  return (ty.k === "U" || ty.k === "I") && ty.w <= 32;
}

/** The inclusive range of a fixed-width type, as `bigint`s. */
export function fixedRange(kind: "U" | "I", width: Width): readonly [bigint, bigint] {
  const w = BigInt(width);
  return kind === "U" ? [0n, (1n << w) - 1n] : [-(1n << (w - 1n)), (1n << (w - 1n)) - 1n];
}

// ---------------------------------------------------------------------------
// Hex
// ---------------------------------------------------------------------------

const HEX = "0123456789abcdef";

/** Lowercase hex of `data`. */
export function toHex(data: Uint8Array): string {
  let out = "";
  for (const byte of data) out += HEX.charAt(byte >> 4) + HEX.charAt(byte & 15);
  return out;
}

/** Decode lowercase hex; `undefined` for odd length, upper case or a non-hex digit. */
export function fromHex(text: string): Uint8Array | undefined {
  if (text.length % 2 !== 0 || !/^[0-9a-f]*$/.test(text)) return undefined;
  const out = new Uint8Array(text.length / 2);
  for (let i = 0; i < out.length; i += 1) out[i] = parseInt(text.slice(2 * i, 2 * i + 2), 16);
  return out;
}

// ---------------------------------------------------------------------------
// Decimal text
// ---------------------------------------------------------------------------

const DECIMAL = /^-?(?:0|[1-9][0-9]*)$/;

/**
 * Decode the section 2.3 decimal text: digits only, one leading `-` allowed
 * (and only for a non-zero value: `"-0"` is refused), no `+`, no leading zero
 * except `"0"`, at most 20 digits.  Returns `undefined` for anything else.
 */
export function parseDecimal(text: unknown): bigint | undefined {
  if (typeof text !== "string" || !DECIMAL.test(text)) return undefined;
  if (text === "-0") return undefined;
  const digits = text.startsWith("-") ? text.length - 1 : text.length;
  if (digits > 20) return undefined;
  return BigInt(text);
}

// ---------------------------------------------------------------------------
// JSON -> value
// ---------------------------------------------------------------------------

/**
 * The level a type this lane cannot hold would need (for the `level-missing` refusal), named by the one rule of
 * {@link firstMissing} used everywhere else: `List<Option<U8>>` names `ir.list`, not `ir.data` ([D63]).
 */
function lackedLevel(ty: Ty): string {
  const levels = new Set<string>();
  typeLevels(ty, levels);
  return firstMissing(levels, new Set(["ir.core"])) ?? "ir.data";
}

/**
 * Decode `raw` (a parsed JSON value) to the host form of `ty`.  A type this
 * lane does not implement (a tuple, `Option`, record, variant or list) is
 * `level-missing`; every refusal of section 2.3 is `value`.
 */
export function decodeJson(raw: unknown, ty: Ty, table: TypeTable): Value {
  const bad = (why: string): never => {
    throw new KitError("value", why);
  };
  switch (ty.k) {
    case "Bool":
      if (typeof raw !== "boolean") return bad("a Bool is a bare JSON boolean");
      return raw;
    case "U":
    case "I":
    case "Nat":
    case "Int": {
      const number = parseDecimal(raw);
      if (number === undefined) return bad("an integer is canonical decimal text");
      if (ty.k === "Nat") {
        if (number < 0n || number > U64_MAX) return bad("a Nat is in 0..=2^64-1");
        return number;
      }
      if (ty.k === "Int") return number;
      const [low, high] = fixedRange(ty.k, ty.w);
      if (number < low || number > high) return bad("an integer is outside its type");
      return ty.w <= 32 ? Number(number) : number;
    }
    case "Bytes": {
      if (typeof raw !== "string") return bad("Bytes is a hex string");
      const data = fromHex(raw);
      if (data === undefined) return bad("Bytes is lowercase hex of even length");
      if (data.length > MAX_WIDTH) return bad("a Bytes value is at most 4096 bytes");
      if (typeof ty.n === "number" && data.length !== ty.n) return bad("a Bytes<N> value has exactly N bytes");
      return data;
    }
    case "Result": {
      if (typeof raw !== "object" || raw === null || Array.isArray(raw)) return bad("a Result is an object");
      const keys = Object.keys(raw);
      if (keys.length !== 1) return bad("a Result value is an object with a single key");
      const record = raw as Record<string, unknown>;
      if (keys[0] === "ok") {
        if (ty.ok === null) return bad("an undetermined Result payload cannot be decoded");
        return { ok: decodeJson(record["ok"], ty.ok, table) };
      }
      if (keys[0] === "err") {
        const cases = ty.err === null ? undefined : table.errors.get(ty.err);
        const name = record["err"];
        if (cases === undefined || typeof name !== "string" || !cases.includes(name)) {
          return bad("an Err case is declared by its error type");
        }
        return { err: name };
      }
      return bad("a Result key is ok or err");
    }
    default:
      throw new KitError("level-missing", "this lane does not hold values of this type", { level: lackedLevel(ty) });
  }
}

// ---------------------------------------------------------------------------
// TFV -> value (section 2.5)
// ---------------------------------------------------------------------------

/**
 * Decode a parsed TFV value (the TFV v1 reader of `../tfv.ts` produces it) of
 * declared type `ty`.
 *
 * `cases.tfv` stays TFV v1 and holds **examples only** (section 2.5): booleans
 * are `T`/`F`, integers `I<decimal>` (a leading `-` allowed for a signed type),
 * bytes `X<hex>`, and **`Result` stays the `M` form of VER04i section 2.7**: a
 * one-entry map whose key is the text `ok` (the value) or `err` (the error case
 * name as text).  The tuple, variant and list forms belong to levels this lane
 * lacks.  Every refusal is `value`: the same range checks as the JSON decoder.
 */
export function decodeTfv(raw: TfvValue, ty: Ty, table: TypeTable): Value {
  const bad = (why: string): never => {
    throw new KitError("value", why);
  };
  switch (ty.k) {
    case "Bool":
      if (raw[0] !== "bool") return bad("expected a TFV boolean");
      return raw[1];
    case "U":
    case "I":
    case "Nat":
    case "Int": {
      if (raw[0] !== "int") return bad("expected a TFV integer");
      const number = raw[1];
      if (ty.k === "Nat") {
        if (number < 0n || number > U64_MAX) return bad("a Nat is in 0..=2^64-1");
        return number;
      }
      if (ty.k === "Int") return number;
      const [low, high] = fixedRange(ty.k, ty.w);
      if (number < low || number > high) return bad("an integer is outside its type");
      return ty.w <= 32 ? Number(number) : number;
    }
    case "Bytes": {
      if (raw[0] !== "bytes") return bad("expected TFV bytes");
      if (raw[1].length > MAX_WIDTH) return bad("a Bytes value is at most 4096 bytes");
      if (typeof ty.n === "number" && raw[1].length !== ty.n) return bad("a Bytes<N> value has exactly N bytes");
      return raw[1];
    }
    case "Result": {
      if (raw[0] !== "map" || raw[1].length !== 1) return bad("expected a one-entry TFV map");
      const [key, body] = raw[1][0] as readonly [TfvValue, TfvValue];
      if (key[0] !== "text") return bad("a Result key is text");
      if (key[1] === "ok" && ty.ok !== null) return { ok: decodeTfv(body, ty.ok, table) };
      const cases = ty.err === null ? undefined : table.errors.get(ty.err);
      if (key[1] === "err" && body[0] === "text" && cases !== undefined && cases.includes(body[1])) return { err: body[1] };
      return bad("a Result is ok or a declared error case");
    }
    default:
      throw new KitError("level-missing", "this lane does not hold values of this type", { level: lackedLevel(ty) });
  }
}

// ---------------------------------------------------------------------------
// value -> JSON (for the kit's expectations)
// ---------------------------------------------------------------------------

/**
 * The section 2.3 JSON form of `value` as the declared type `ty` (the inverse
 * of {@link decodeJson}, without the range checks: a fault may produce a value
 * that is not a value of the declared type, and that is reported as it is).
 */
export function encodeJson(value: unknown, ty: Ty): unknown {
  switch (ty.k) {
    case "Bool":
      return value as boolean;
    case "U":
    case "I":
    case "Nat":
    case "Int":
      return (value as number | bigint).toString();
    case "Bytes":
      return toHex(value as Uint8Array);
    case "Result": {
      const result = value as { ok?: unknown; err?: string };
      if (result.err !== undefined) return { err: result.err };
      return { ok: ty.ok === null ? null : encodeJson(result.ok, ty.ok) };
    }
    default:
      throw new KitError("level-missing", "this lane does not hold values of this type", { level: lackedLevel(ty) });
  }
}

// ---------------------------------------------------------------------------
// Structural equality (section 4.2 rule 5)
// ---------------------------------------------------------------------------

/**
 * Structural, type-directed equality: sequence order matters, errors are
 * nominal, `Bool` is not `U8`.  (Two values of one *static* type always have
 * one host form, so comparing the host forms is comparing the values.)
 */
export function valuesEqual(a: unknown, b: unknown): boolean {
  if (a instanceof Uint8Array) {
    if (!(b instanceof Uint8Array) || a.length !== b.length) return false;
    for (let i = 0; i < a.length; i += 1) if (a[i] !== b[i]) return false;
    return true;
  }
  if (typeof a === "object" && a !== null) {
    if (typeof b !== "object" || b === null) return false;
    const left = a as { ok?: unknown; err?: string };
    const right = b as { ok?: unknown; err?: string };
    if (left.err !== undefined || right.err !== undefined) return left.err === right.err;
    return valuesEqual(left.ok, right.ok);
  }
  return a === b;
}

// ---------------------------------------------------------------------------
// The stream sink
// ---------------------------------------------------------------------------

const CAPACITY = 64 * 1024;

/**
 * A buffer that stream bytes are written into and handed to `onFlush` when it
 * fills.  Hashing a 2^24-case suite one `update` call per value would spend
 * its time crossing into the native hasher; one 64 KiB buffer, flushed when
 * full, does not, and a writer allocates nothing per case.
 *
 * `onFlush` must consume the bytes before returning: the buffer is reused.
 */
export class StreamSink {
  readonly buf = new Uint8Array(CAPACITY);
  readonly view = new DataView(this.buf.buffer);
  pos = 0;
  private readonly onFlush: (bytes: Uint8Array) => void;

  constructor(onFlush: (bytes: Uint8Array) => void) {
    this.onFlush = onFlush;
  }

  /** Make room for `count` more bytes (at most 64 KiB at a time). */
  ensure(count: number): void {
    if (this.pos + count > CAPACITY) this.flush();
  }

  /** Hand what is buffered to `onFlush`. */
  flush(): void {
    if (this.pos > 0) {
      this.onFlush(this.buf.subarray(0, this.pos));
      this.pos = 0;
    }
  }

  /** Write `data` as it is (a long value goes straight through). */
  raw(data: Uint8Array): void {
    if (data.length > CAPACITY) {
      this.flush();
      this.onFlush(data);
      return;
    }
    this.ensure(data.length);
    this.buf.set(data, this.pos);
    this.pos += data.length;
  }

  /** Write a u32 little-endian. */
  u32(value: number): void {
    this.ensure(4);
    this.view.setUint32(this.pos, value, true);
    this.pos += 4;
  }

  /** Write UTF-8 text (length is the caller's business). */
  text(value: string): void {
    this.raw(new TextEncoder().encode(value));
  }
}

/**
 * A writer specialised to `ty`: the dispatch on the type happens once, when the
 * suite is compiled, not once per case.
 */
export function makeWriter(ty: Ty): (sink: StreamSink, value: unknown) => void {
  switch (ty.k) {
    case "Bool":
      return (sink, value) => {
        sink.ensure(1);
        sink.buf[sink.pos++] = value === true ? 1 : 0;
      };
    case "U":
    case "I": {
      if (ty.w <= 32) {
        const signed = ty.k === "I";
        return (sink, value) => {
          sink.ensure(9);
          const b = sink.buf;
          const v = value as number;
          let p = sink.pos;
          b[p] = signed ? 0x11 : 0x10;
          b[p + 1] = v & 255;
          b[p + 2] = (v >>> 8) & 255;
          b[p + 3] = (v >>> 16) & 255;
          b[p + 4] = (v >>> 24) & 255;
          const fill = signed && v < 0 ? 255 : 0;
          b[p + 5] = fill;
          b[p + 6] = fill;
          b[p + 7] = fill;
          b[p + 8] = fill;
          p += 9;
          sink.pos = p;
        };
      }
      if (ty.k === "U") {
        return (sink, value) => {
          sink.ensure(9);
          sink.buf[sink.pos] = 0x10;
          sink.view.setBigUint64(sink.pos + 1, value as bigint, true);
          sink.pos += 9;
        };
      }
      return (sink, value) => {
        sink.ensure(9);
        sink.buf[sink.pos] = 0x11;
        sink.view.setBigInt64(sink.pos + 1, value as bigint, true);
        sink.pos += 9;
      };
    }
    case "Nat":
      return (sink, value) => {
        const v = value as bigint;
        if (v < 0n || v > U64_MAX) throw new KitError("value", "a Nat value is in 0..=2^64-1");
        sink.ensure(9);
        sink.buf[sink.pos] = 0x10;
        sink.view.setBigUint64(sink.pos + 1, v, true);
        sink.pos += 9;
      };
    case "Bytes":
      return (sink, value) => {
        const data = value as Uint8Array;
        sink.ensure(5);
        sink.buf[sink.pos] = 0x20;
        sink.view.setUint32(sink.pos + 1, data.length, true);
        sink.pos += 5;
        sink.raw(data);
      };
    case "Result": {
      const inner = ty.ok === null ? undefined : makeWriter(ty.ok);
      return (sink, value) => {
        const result = value as { ok?: unknown; err?: string };
        if (result.err !== undefined) {
          const name = new TextEncoder().encode(result.err);
          sink.ensure(5);
          sink.buf[sink.pos] = 0x51;
          sink.view.setUint32(sink.pos + 1, name.length, true);
          sink.pos += 5;
          sink.raw(name);
          return;
        }
        sink.ensure(1);
        sink.buf[sink.pos++] = 0x50;
        (inner as (s: StreamSink, v: unknown) => void)(sink, result.ok);
      };
    }
    default:
      throw new KitError("level-missing", `no stream writer for ${showType(ty)} in this lane`, { level: lackedLevel(ty) });
  }
}

/** The section 2.4 bytes of one value (no header, no context). */
export function streamBytes(value: unknown, ty: Ty): Uint8Array {
  const chunks: Uint8Array[] = [];
  const sink = new StreamSink((bytes) => chunks.push(bytes.slice()));
  makeWriter(ty)(sink, value);
  sink.flush();
  const total = chunks.reduce((sum, chunk) => sum + chunk.length, 0);
  const out = new Uint8Array(total);
  let at = 0;
  for (const chunk of chunks) {
    out.set(chunk, at);
    at += chunk.length;
  }
  return out;
}
