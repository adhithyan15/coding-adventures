/**
 * # Resolved descriptor trees: `size()` and `nth()` (VER04r section 5.1)
 *
 * A binder carries a **fully resolved descriptor tree**, emitted after the
 * budget demotion so that a lane never re-runs demotion (the single largest
 * cause of silent drift between a generator and a consumer).  Every descriptor
 * defines two total functions, and this module implements both:
 *
 *   * `size()`  -- how many values the descriptor has, which the consumer
 *     *recomputes* and compares with the declared `size` (checked arithmetic;
 *     an overflow is an error, never zero);
 *   * `nth(i)`  -- the `i`-th value, `0 <= i < size`.  It is what makes chunk
 *     jumping possible: a lane unranks the first product ordinal of a chunk and
 *     never touches an earlier chunk.
 *
 * | node        | JSON                                         | order of its values |
 * |-------------|----------------------------------------------|---------------------|
 * | `bool`      | `{"d":"bool","size":2}`                      | `false`, `true`     |
 * | `u8_all`    | `{"d":"u8_all","size":256}`                  | `0..=255`           |
 * | `u64_edge`  | `{"d":"u64_edge","size":377}`                | `0..=255`; `1<<i` for `i = 8..=63`; `MAX`; `MAX ^ (1<<i)` for `i = 0..=63` |
 * | `nat_0_64`  | `{"d":"nat_0_64","size":65}`                 | `0..=64`            |
 * | `bytes_d1`  | `{"d":"bytes_d1","size":347}`                | `""`, each byte, every length-2 then length-3 string over `A = {00,01,80,ff}`, then `00`x`L` and `a5`x`L` for `L = 4..=8` |
 * | `bytes_n`   | `{"d":"bytes_n","n":N,"size":S}`             | `N <= 3`: the `bytes_d1` members of length `N`; else the four uniform fills over `A`, then for each position and each of `01 80 ff` the zero string with that byte set |
 * | `fx_full`   | `{"d":"fx_full","t":T,"size":S}`             | every value of an 8- or 16-bit `T` other than `U8`, ascending |
 * | `fx_edge`   | `{"d":"fx_edge","t":T,"size":S}`             | 16/32/64-bit `T` other than `U64`: the edge set, ascending for a signed type |
 * | `nat_count` | `{"d":"nat_count","k":K,"size":S}`           | `0..=K`, `K <= 4096` |
 * | `nat_shift` | `{"d":"nat_shift","size":73}`                | `0..=64`, then `65 127 128 129 255 256 65535 65536` |
 * | `result`    | `{"d":"result","ok":D,"errs":[..],"size":S}` | `Ok(x)` for `x` in `D`, then each error case in declaration order |
 *
 * `tuple` and `variant`/`option` are the descriptors of `ir.tuple` and
 * `ir.data`, and `list` is reserved with `ir.list`: this lane **names** them and
 * refuses each as the level it belongs to ([D63] for `list`: never as an
 * unknown `d`).
 *
 * The tree is at most 16 levels deep, counted in nodes on a path: a leaf alone
 * is one level (section 16.15 R40).
 */

import { KitError } from "./errors.ts";
import { type Ty, type TypeTable, isName, sameType, showType } from "./types.ts";
import type { Value } from "./values.ts";

/** A built domain: a size and an indexed accessor. */
export interface Domain {
  readonly size: number;
  /** The `i`-th value, `0 <= i < size` (the caller checks the range). */
  nth(i: number): Value;
}

const MAX_DEPTH = 16;
const ALPHABET = [0x00, 0x01, 0x80, 0xff] as const;
const SHIFT_TAIL = [65n, 127n, 128n, 129n, 255n, 256n, 65535n, 65536n];

/** A refusal of section 5.1. */
function refuse(why: string): never {
  throw new KitError("descriptor", why);
}

// ---------------------------------------------------------------------------
// Leaf value tables
// ---------------------------------------------------------------------------

/** `bytes_d1`: 1 + 256 + 16 + 64 + 10 = 347 strings. */
function bytesD1(): Uint8Array[] {
  const values: Uint8Array[] = [new Uint8Array(0)];
  for (let byte = 0; byte < 256; byte += 1) values.push(Uint8Array.of(byte));
  for (const length of [2, 3]) {
    let strings: number[][] = [[]];
    for (let position = 0; position < length; position += 1) {
      strings = strings.flatMap((prefix) => ALPHABET.map((symbol) => [...prefix, symbol]));
    }
    for (const symbols of strings) values.push(Uint8Array.from(symbols));
  }
  for (let length = 4; length <= 8; length += 1) {
    for (const fill of [0x00, 0xa5]) values.push(new Uint8Array(length).fill(fill));
  }
  return values;
}

const D1 = bytesD1();

/** `bytes_n(n)` size (the descriptor row says `n` is `0..=4096`). */
function bytesNSize(n: number): number {
  return n <= 3 ? [1, 256, 16, 64][n] as number : 4 + 3 * n;
}

/** The `i`-th member of `bytes_n(n)`, freshly allocated. */
function bytesNth(n: number, i: number): Uint8Array {
  if (n <= 3) {
    // The bytes_d1 members of length n, in bytes_d1 order.
    if (n === 0) return new Uint8Array(0);
    if (n === 1) return Uint8Array.of(i);
    const start = n === 2 ? 1 + 256 : 1 + 256 + 16;
    return D1[start + i] as Uint8Array;
  }
  if (i < 4) return new Uint8Array(n).fill(ALPHABET[i] as number);
  const j = i - 4;
  const out = new Uint8Array(n);
  out[Math.floor(j / 3)] = [0x01, 0x80, 0xff][j % 3] as number;
  return out;
}

/** The `u64_edge` values: 377 of them. */
function u64Edge(): bigint[] {
  const out: bigint[] = [];
  for (let v = 0n; v < 256n; v += 1n) out.push(v);
  for (let s = 8n; s < 64n; s += 1n) out.push(1n << s);
  const max = (1n << 64n) - 1n;
  out.push(max);
  for (let s = 0n; s < 64n; s += 1n) out.push(max ^ (1n << s));
  return out;
}

const EDGE_CACHE = new Map<string, bigint[]>();

/** The `fx_edge` set of a type (section 5.1, [D21]): the unsigned set, or its two's-complement image plus `-255..=-1`, ascending. */
function fxEdge(kind: "U" | "I", width: number): bigint[] {
  const key = `${kind}${width}`;
  const cached = EDGE_CACHE.get(key);
  if (cached !== undefined) return cached;
  const w = BigInt(width);
  const max = (1n << w) - 1n;
  const unsigned: bigint[] = [];
  for (let v = 0n; v < 256n; v += 1n) unsigned.push(v);
  for (let s = 8n; s < w; s += 1n) unsigned.push(1n << s);
  unsigned.push(max);
  for (let s = 0n; s < w; s += 1n) unsigned.push(max ^ (1n << s));
  let result: bigint[];
  if (kind === "U") {
    result = unsigned;
  } else {
    const set = new Set<bigint>(unsigned.map((v) => BigInt.asIntN(width, v)));
    for (let v = -255n; v <= -1n; v += 1n) set.add(v);
    result = [...set].sort((a, b) => (a < b ? -1 : a > b ? 1 : 0));
  }
  EDGE_CACHE.set(key, result);
  return result;
}

// ---------------------------------------------------------------------------
// The parsed (and size-checked) tree
// ---------------------------------------------------------------------------

type Node =
  | { readonly d: "bool" | "u8_all" | "u64_edge" | "nat_0_64" | "bytes_d1" | "nat_shift"; readonly size: number }
  | { readonly d: "bytes_n"; readonly n: number; readonly size: number }
  | { readonly d: "fx_full" | "fx_edge"; readonly t: Ty; readonly kind: "U" | "I"; readonly w: number; readonly size: number }
  | { readonly d: "nat_count"; readonly k: number; readonly size: number }
  | { readonly d: "result"; readonly ok: Node; readonly errs: readonly string[]; readonly size: number };

/*
 * On sharing: `nth` hands out the *same* `Uint8Array` or object for one index every time it is asked, and a suite's
 * cases share them.  That is safe here because nothing in this lane writes to a value it was given: `take` and `drop`
 * slice, `concat` allocates, a comparison only reads, and the fault wrappers copy the argument list before a rewrite.
 * A lane that runs a *subject* which may mutate its arguments (S7b's `arg-mutated` fixture) must copy per case instead.
 */
/** Both tables are `Map`s: a descriptor kind read from input must not find an `Object.prototype` member. */
const LEAF_SIZES: ReadonlyMap<string, number> = new Map([["bool", 2], ["u8_all", 256], ["u64_edge", 377], ["nat_0_64", 65], ["bytes_d1", 347], ["nat_shift", 73]]);
const LEVEL_OF_KIND: ReadonlyMap<string, string> = new Map([["tuple", "ir.tuple"], ["variant", "ir.data"], ["option", "ir.data"], ["list", "ir.list"]]);

/** `size` must be a JSON integer (not text, not a boolean) equal to the recomputed one. */
function declaredSize(raw: unknown, recomputed: number): void {
  if (typeof raw !== "number" || !Number.isInteger(raw)) refuse("size is not a JSON integer");
  if (raw !== recomputed) refuse("size is not the recomputed size");
}

/** The exact key set a node must have. */
function keysAre(node: Record<string, unknown>, expected: readonly string[]): void {
  const keys = Object.keys(node);
  if (keys.length !== expected.length || !expected.every((key) => Object.hasOwn(node, key))) refuse("a node has a missing or extra key");
}

function fixedType(text: unknown): { ty: Ty; kind: "U" | "I"; w: number } {
  const match = typeof text === "string" ? /^([UI])(8|16|32|64)$/.exec(text) : null;
  if (match === null) return refuse("t is not a fixed-width type");
  const kind = match[1] as "U" | "I";
  const w = Number(match[2]);
  return { ty: { k: kind, w: w as 8 | 16 | 32 | 64 }, kind, w };
}

function readNode(raw: unknown, depth: number): Node {
  if (depth > MAX_DEPTH) refuse("a tree is at most 16 levels deep");
  if (typeof raw !== "object" || raw === null || Array.isArray(raw)) return refuse("a descriptor is an object");
  const node = raw as Record<string, unknown>;
  const kind = node["d"];
  if (typeof kind !== "string") return refuse("a descriptor has no kind d");
  const reserved = LEVEL_OF_KIND.get(kind);
  if (reserved !== undefined) {
    // [D63]: `list` is the reserved level, never an unknown kind; `tuple`, `variant`
    // and `option` belong to levels this lane does not implement.
    throw new KitError("level-missing", `the ${kind} descriptor belongs to ${reserved}`, { level: reserved });
  }
  const sized = LEAF_SIZES.get(kind);
  if (sized !== undefined) {
    keysAre(node, ["d", "size"]);
    declaredSize(node["size"], sized);
    return { d: kind as "bool", size: sized };
  }
  switch (kind) {
    case "bytes_n": {
      keysAre(node, ["d", "n", "size"]);
      const n = node["n"];
      if (typeof n !== "number" || !Number.isInteger(n) || n < 0 || n > 4096) return refuse("n is outside 0..=4096");
      const size = bytesNSize(n);
      declaredSize(node["size"], size);
      return { d: "bytes_n", n, size };
    }
    case "fx_full": {
      keysAre(node, ["d", "t", "size"]);
      const fixed = fixedType(node["t"]);
      if (fixed.w > 16 || (fixed.kind === "U" && fixed.w === 8)) refuse("fx_full is for I8, U16 and I16");
      declaredSize(node["size"], 2 ** fixed.w);
      return { d: "fx_full", t: fixed.ty, kind: fixed.kind, w: fixed.w, size: 2 ** fixed.w };
    }
    case "fx_edge": {
      keysAre(node, ["d", "t", "size"]);
      const fixed = fixedType(node["t"]);
      if (fixed.w === 8 || (fixed.kind === "U" && fixed.w === 64)) refuse("fx_edge is for 16/32-bit and I64 types");
      const size = fxEdge(fixed.kind, fixed.w).length;
      declaredSize(node["size"], size);
      return { d: "fx_edge", t: fixed.ty, kind: fixed.kind, w: fixed.w, size };
    }
    case "nat_count": {
      keysAre(node, ["d", "k", "size"]);
      const k = node["k"];
      if (typeof k !== "number" || !Number.isInteger(k) || k < 0 || k > 4096) return refuse("k is outside 0..=4096");
      declaredSize(node["size"], k + 1);
      return { d: "nat_count", k, size: k + 1 };
    }
    case "result": {
      keysAre(node, ["d", "ok", "errs", "size"]);
      const errs = node["errs"];
      if (!Array.isArray(errs) || errs.length < 1 || errs.length > 64) return refuse("errs holds 1 to 64 names");
      const seen = new Set<string>();
      for (const name of errs) {
        if (typeof name !== "string" || !isName(name) || seen.has(name)) return refuse("errs holds unique names in the name grammar");
        seen.add(name);
      }
      const ok = readNode(node["ok"], depth + 1);
      const size = ok.size + errs.length;
      declaredSize(node["size"], size);
      return { d: "result", ok, errs: errs as string[], size };
    }
    default:
      return refuse("the descriptor kind is not one this document defines");
  }
}

// ---------------------------------------------------------------------------
// Matching a node against the binder type, and building the accessor
// ---------------------------------------------------------------------------

function mismatch(): never {
  return refuse("the descriptor is not one for the binder type");
}

function build(node: Node, ty: Ty, table: TypeTable, typesPresent: boolean): Domain {
  switch (node.d) {
    case "bool":
      if (ty.k !== "Bool") mismatch();
      return { size: 2, nth: (i) => i === 1 };
    case "u8_all":
      if (!(ty.k === "U" && ty.w === 8)) mismatch();
      return { size: 256, nth: (i) => i };
    case "u64_edge": {
      if (!(ty.k === "U" && ty.w === 64)) mismatch();
      const table64 = u64Edge();
      return { size: 377, nth: (i) => table64[i] as bigint };
    }
    case "nat_0_64":
      if (ty.k !== "Nat") mismatch();
      return { size: 65, nth: (i) => BigInt(i) };
    case "nat_count":
      if (ty.k !== "Nat") mismatch();
      return { size: node.size, nth: (i) => BigInt(i) };
    case "nat_shift": {
      if (ty.k !== "Nat") mismatch();
      return { size: 73, nth: (i) => (i <= 64 ? BigInt(i) : (SHIFT_TAIL[i - 65] as bigint)) };
    }
    case "bytes_d1":
      if (!(ty.k === "Bytes" && ty.n === null)) mismatch();
      return { size: 347, nth: (i) => D1[i] as Uint8Array };
    case "bytes_n": {
      if (!(ty.k === "Bytes" && ty.n === node.n)) mismatch();
      const n = node.n;
      if (node.size <= 4100) {
        // Small domains are materialised once, so enumeration allocates nothing per case.
        if (n * node.size <= 1 << 20) {
          const all = Array.from({ length: node.size }, (_unused, i) => bytesNth(n, i));
          return { size: node.size, nth: (i) => all[i] as Uint8Array };
        }
      }
      return { size: node.size, nth: (i) => bytesNth(n, i) };
    }
    case "fx_full": {
      if (!sameType(ty, node.t)) mismatch();
      const small = node.w <= 32;
      const base = node.kind === "I" ? -(2 ** (node.w - 1)) : 0;
      return { size: node.size, nth: (i) => (small ? base + i : BigInt(base + i)) };
    }
    case "fx_edge": {
      if (!sameType(ty, node.t)) mismatch();
      const edge = fxEdge(node.kind, node.w);
      const small = node.w <= 32;
      const values: Value[] = edge.map((v) => (small ? Number(v) : v));
      return { size: node.size, nth: (i) => values[i] as Value };
    }
    case "result": {
      if (ty.k !== "Result" || ty.ok === null) mismatch();
      const resultTy = ty as Extract<Ty, { k: "Result" }>;
      const inner = build(node.ok, resultTy.ok as Ty, table, typesPresent);
      if (typesPresent) {
        // R72: when a `types` table is present the list equals the declared cases, order included.
        const declared = resultTy.err === null ? undefined : table.errors.get(resultTy.err);
        if (declared === undefined || declared.length !== node.errs.length || declared.some((name, i) => name !== node.errs[i])) {
          refuse("errs differs from the declared cases");
        }
      }
      const okSize = inner.size;
      const errs = node.errs;
      const total = node.size;
      if (total <= 4096) {
        const all: Value[] = [];
        for (let i = 0; i < total; i += 1) all.push(i < okSize ? { ok: inner.nth(i) } : { err: errs[i - okSize] as string });
        return { size: total, nth: (i) => all[i] as Value };
      }
      return { size: total, nth: (i) => (i < okSize ? { ok: inner.nth(i) } : { err: errs[i - okSize] as string }) };
    }
  }
}

/**
 * Validate a descriptor tree against the binder type and build its domain.
 *
 * Refusals: `descriptor` for every refusal of section 5.1 (an unknown `d`, a
 * missing or extra key, a wrong `size`, an illegal `t`, a tree over 16 levels,
 * a descriptor that is not one for `ty`, a bad `errs`); `level-missing` for the
 * reserved or unimplemented kinds.  `typesPresent` says whether `table` is the
 * document's real `types` (then `errs` must equal the declared cases).
 */
export function buildDomain(raw: unknown, ty: Ty, table: TypeTable, typesPresent = false): Domain {
  const node = readNode(raw, 1);
  return build(node, ty, table, typesPresent);
}

/** Describe a descriptor's binder type in a message without echoing values. */
export function describeType(ty: Ty): string {
  return showType(ty, true);
}
