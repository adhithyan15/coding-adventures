/**
 * # The finite input domains of VER04i section 2.6, enumerated in this lane
 *
 * The corpus does not ship its exhaustive inputs (that would be ~28 MB of text
 * per library). It ships a *descriptor* (`{"domain": "bytes_d1"}`) and each lane
 * re-creates the values itself. The descriptors are defined once, by the spec,
 * and every lane must produce exactly the same values in exactly the same
 * order; the `inputs_sha256` digest in the corpus is what proves it did.
 *
 * | descriptor    | values, in order                                              |
 * |---------------|---------------------------------------------------------------|
 * | `bool`        | `false`, `true`                                               |
 * | `u8_all`      | `0 .. 255`                                                    |
 * | `u64_edge`    | `0..255`; `1<<i` for `i = 8..63`; `MAX`; then `MAX ^ (1<<i)` for `i = 0..63` (377 values) |
 * | `nat_0_64`    | `0 .. 64`                                                     |
 * | `bytes_d1`    | `""`; every single byte; every length-2 and length-3 string over the alphabet `A = {00, 01, 80, ff}` in lexicographic order; for `L = 4..8` the strings `00`x`L` and `a5`x`L` (347 values) |
 * | `bytes_n(N)`  | for `N <= 3` the `bytes_d1` members of length `N`; otherwise the four uniform fills over `A`, then for every position and each of `01, 80, ff` the zero string with that one byte set |
 *
 * Integers are `bigint`: `u64_edge` holds values far above 2^53, which a
 * JavaScript `number` would round (and `MAX` and `MAX ^ 1` would become the
 * *same* double).
 *
 * A multi-binder claim enumerates the Cartesian product with the **last binder
 * varying fastest** (row-major); see {@link product}.
 */

import { CorpusError, need } from "./errors.ts";
import { isObject } from "./jsonio.ts";
import { MAX_WIDTH, U64_MAX } from "./values.ts";

const ALPHABET = [0x00, 0x01, 0x80, 0xff] as const;
const BYTES_N = /^bytes_n\(([0-9]+)\)$/;

/** Every string of `length` symbols over the alphabet, in lexicographic order. */
function alphabetStrings(length: number): Uint8Array[] {
  let strings: number[][] = [[]];
  for (let position = 0; position < length; position += 1) {
    strings = strings.flatMap((prefix) => ALPHABET.map((symbol) => [...prefix, symbol]));
  }
  return strings.map((symbols) => Uint8Array.from(symbols));
}

function bytesD1(): Uint8Array[] {
  const values: Uint8Array[] = [new Uint8Array(0)];
  for (let byte = 0; byte < 256; byte += 1) values.push(Uint8Array.of(byte));
  values.push(...alphabetStrings(2), ...alphabetStrings(3));
  for (let length = 4; length <= 8; length += 1) {
    for (const fill of [0x00, 0xa5]) values.push(new Uint8Array(length).fill(fill));
  }
  return values;
}

function bytesN(width: number): Uint8Array[] {
  need(width >= 0 && width <= MAX_WIDTH, "descriptor", "Bytes width is out of bounds");
  if (width <= 3) return bytesD1().filter((value) => value.length === width);
  const values = ALPHABET.map((fill) => new Uint8Array(width).fill(fill));
  for (let position = 0; position < width; position += 1) {
    for (const symbol of [0x01, 0x80, 0xff]) {
      const buffer = new Uint8Array(width);
      buffer[position] = symbol;
      values.push(buffer);
    }
  }
  return values;
}

function u64Edge(): bigint[] {
  const values: bigint[] = [];
  for (let value = 0n; value < 256n; value += 1n) values.push(value);
  for (let shift = 8n; shift < 64n; shift += 1n) values.push(1n << shift);
  values.push(U64_MAX);
  for (let shift = 0n; shift < 64n; shift += 1n) values.push(U64_MAX ^ (1n << shift));
  return values;
}

function range(count: number): bigint[] {
  return Array.from({ length: count }, (_unused, index) => BigInt(index));
}

/** The values of a descriptor and the Warrant type text they have. */
export interface Domain {
  readonly values: unknown[];
  readonly type: string;
}

/** Enumerate the descriptor `name`. */
export function domainValues(name: string): Domain {
  if (name === "bool") return { values: [false, true], type: "Bool" };
  if (name === "u8_all") return { values: range(256), type: "U8" };
  if (name === "u64_edge") return { values: u64Edge(), type: "U64" };
  if (name === "nat_0_64") return { values: range(65), type: "Nat" };
  if (name === "bytes_d1") return { values: bytesD1(), type: "Bytes" };
  const match = BYTES_N.exec(name);
  if (match) {
    const width = Number(match[1]);
    return { values: bytesN(width), type: `Bytes<${width}>` };
  }
  throw new CorpusError("unsupported-domain", "unknown domain descriptor");
}

/** How many values a descriptor has and which type they are, *without* building them. */
export function domainShape(name: string): { readonly size: number; readonly type: string } {
  if (name === "bool") return { size: 2, type: "Bool" };
  if (name === "u8_all") return { size: 256, type: "U8" };
  if (name === "u64_edge") return { size: 256 + 56 + 1 + 64, type: "U64" };
  if (name === "nat_0_64") return { size: 65, type: "Nat" };
  if (name === "bytes_d1") return { size: 1 + 256 + 16 + 64 + 10, type: "Bytes" };
  const match = BYTES_N.exec(name);
  if (match) {
    const width = Number(match[1]);
    need(width >= 0 && width <= MAX_WIDTH, "descriptor", "Bytes width is out of bounds");
    // Widths 0..3 are the bytes_d1 members of that length: 1, 256, 4^2, 4^3.
    const small = [1, 256, 16, 64][width];
    return { size: small ?? 4 + 3 * width, type: `Bytes<${width}>` };
  }
  throw new CorpusError("unsupported-domain", "unknown domain descriptor");
}

/**
 * Validate one binder's `{"domain": ..., "type": ...}` descriptor and return
 * the domain's name, size and type without enumerating it.
 */
export function binderShape(descriptor: unknown): { readonly name: string; readonly size: number; readonly type: string } {
  need(
    isObject(descriptor) &&
      Object.keys(descriptor).every((key) => key === "domain" || key === "type") &&
      typeof descriptor["domain"] === "string",
    "descriptor",
    "malformed binder domain",
  );
  const name = descriptor["domain"] as string;
  const shape = domainShape(name);
  need(
    descriptor["type"] === undefined || descriptor["type"] === shape.type,
    "descriptor",
    "binder type disagrees with its domain",
  );
  return { name, ...shape };
}

/**
 * Resolve one binder's `{"domain": ..., "type": ...}` descriptor.
 *
 * The declared `type` must agree with the type the descriptor produces, so a
 * corpus cannot label byte strings as integers and have the encoders quietly
 * disagree with the generator.
 */
export function binderDomain(descriptor: unknown): Domain {
  need(
    isObject(descriptor) &&
      Object.keys(descriptor).every((key) => key === "domain" || key === "type") &&
      typeof descriptor["domain"] === "string",
    "descriptor",
    "malformed binder domain",
  );
  const found = domainValues(descriptor["domain"] as string);
  need(
    descriptor["type"] === undefined || descriptor["type"] === found.type,
    "descriptor",
    "binder type disagrees with its domain",
  );
  return found;
}

/**
 * Visit the Cartesian product of `domains` with the last binder fastest.
 *
 * An odometer, not a recursion: a million tuples must not cost a million
 * closures. `visit` returns `false` to stop early. The array handed to
 * `visit` is reused, so a caller that keeps it must copy it.
 */
export function product(domains: readonly (readonly unknown[])[], visit: (values: unknown[]) => boolean): void {
  if (domains.some((domain) => domain.length === 0)) return;
  const counters = domains.map(() => 0);
  const current = domains.map((domain) => domain[0]);
  for (;;) {
    if (!visit(current)) return;
    let position = domains.length - 1;
    while (position >= 0) {
      const domain = domains[position] as readonly unknown[];
      counters[position] = (counters[position] as number) + 1;
      if ((counters[position] as number) < domain.length) {
        current[position] = domain[counters[position] as number];
        break;
      }
      counters[position] = 0;
      current[position] = domain[0];
      position -= 1;
    }
    if (position < 0) return;
  }
}
