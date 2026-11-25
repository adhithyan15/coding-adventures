/**
 * # Types: the section 2.1 grammar, the section 2.2 table, and the type relations
 *
 * One grammar names a type everywhere a type is named: ops, binders, stream
 * contexts, manifests, bindings and the model IR.
 *
 *     type = "Bool" | "U8" | "U16" | "U32" | "U64" | "I8" | "I16" | "I32" | "I64"
 *          | "Nat" | "Bytes" | "Bytes<" N ">" | "(" type { "," type } ")"
 *          | "List<" type ">" | "Option<" type ">" | "Result<" type "," ErrName ">"
 *          | Name
 *
 * `Int` is the mathematical integer.  It exists *inside the IR only*, as the
 * type of an intermediate (`Nat - Nat`) and of a `lit` node, so the parser only
 * accepts it when the caller says it may.
 *
 * A runner reads the type syntax of **every** defined level, implemented or
 * not ([D64], R69): a tuple type, a record or `Option` name, and a `List<T>`
 * parse here, and {@link typeLevels} says which level each one needs.  What
 * this lane cannot *run* it can still *name*.
 *
 * ## The static type of an IR expression
 *
 * The IR carries no types, so the checker infers one per node.  Besides the
 * declared types it needs three "unfinished" shapes (section 16.15 R41):
 *
 *   * a bare `ok(e)` has an *undetermined error type*: `Result<T, ?>`;
 *   * a bare `err("CASE")` has an *undetermined payload*: `Result<?, E>`, with
 *     `E` one of the error types that declare `CASE`;
 *   * and a bytes literal is `Bytes<len>`: **a width is part of the type**
 *     (R51), so that `Bytes<2>` flows to `Bytes` and to nothing else.
 *
 * Two relations decide what fits where (R51): `flows(s, t)` ("a value of `s`
 * may be used where `t` is wanted") and `unify(a, b)` ("the common type", used
 * for the branches of an `if` and the operands of `eq`).
 */

import { KitError, type KitCode } from "./errors.ts";

/** Fixed-width integer widths. */
export type Width = 8 | 16 | 32 | 64;

/** A Warrant type, as the checker sees it. */
export type Ty =
  | { readonly k: "Bool" }
  | { readonly k: "Int" }
  | { readonly k: "Nat" }
  | { readonly k: "U"; readonly w: Width }
  | { readonly k: "I"; readonly w: Width }
  /** `n` is a literal width, a named static width (`Bytes<N>` in an op), or `null` for plain `Bytes`. */
  | { readonly k: "Bytes"; readonly n: number | string | null }
  | { readonly k: "Tuple"; readonly of: readonly Ty[] }
  | { readonly k: "List"; readonly of: Ty }
  | { readonly k: "Option"; readonly of: Ty | null }
  /**
   * `ok === null` is an undetermined payload (bare `err`), `err === null` an
   * undetermined error type (bare `ok`, or a bare `err` whose case more than one
   * error type declares, in which case `cands` lists them).
   */
  | { readonly k: "Result"; readonly ok: Ty | null; readonly err: string | null; readonly cands?: readonly string[] }
  /** A record or variant declared in `types` (level `ir.data`). */
  | { readonly k: "Name"; readonly name: string };

export const BOOL: Ty = { k: "Bool" };
export const INT: Ty = { k: "Int" };
export const NAT: Ty = { k: "Nat" };
export const BYTES: Ty = { k: "Bytes", n: null };

/**
 * The scalar type names.  A `Map`, never a plain object: a name read from input
 * (`constructor`, `toString`, `__proto__`...) must not find a member inherited
 * from `Object.prototype` (section 2.2 rule 4).
 */
const SCALARS: ReadonlyMap<string, Ty> = new Map<string, Ty>([
  ["Bool", BOOL],
  ["Nat", NAT],
  ["Int", INT],
  ["U8", { k: "U", w: 8 }],
  ["U16", { k: "U", w: 16 }],
  ["U32", { k: "U", w: 32 }],
  ["U64", { k: "U", w: 64 }],
  ["I8", { k: "I", w: 8 }],
  ["I16", { k: "I", w: 16 }],
  ["I32", { k: "I", w: 32 }],
  ["I64", { k: "I", w: 64 }],
]);

/** The largest `Bytes<N>` width and the largest `Bytes` value in a corpus (VER04h section 3). */
export const MAX_WIDTH = 4096;
/** Type depth: the number of constructors enclosing a name (section 2.2 rule 1, R40). */
export const MAX_TYPE_DEPTH = 16;

const NAME = /^[A-Za-z_][A-Za-z0-9_]*$/;
const DECIMAL = /^(?:0|[1-9][0-9]*)$/;

/** Is `text` a name in the section 2.2 grammar (`[A-Za-z_][A-Za-z0-9_]*`)? */
export function isName(text: string): boolean {
  return NAME.test(text);
}

/** Options of {@link parseType}. */
export interface ParseOptions {
  /** Accept `Int` (only a `lit` node's `type` field may write it). */
  readonly allowInt?: boolean;
  /** Accept a named static width, `Bytes<N>` with `N` a name (an op signature only). */
  readonly allowNamedWidth?: boolean;
  /** The deepest wrapper nesting accepted (default 16; the domain checker is lenient, see there). */
  readonly maxDepth?: number;
  /** The code a refusal carries (default `type`). */
  readonly code?: KitCode;
}

/**
 * Parse type text.  A hostile string cannot make this recurse deeply: it stops
 * at `maxDepth` wrappers, so recursion is bounded by 17 frames at the default.
 */
export function parseType(text: string, options: ParseOptions = {}): Ty {
  const code = options.code ?? "type";
  const maxDepth = options.maxDepth ?? MAX_TYPE_DEPTH;
  let at = 0;
  const fail = (why: string): never => {
    throw new KitError(code, `type text refused: ${why}`);
  };
  const identifier = (): string => {
    const start = at;
    while (at < text.length && /[A-Za-z0-9_]/.test(text.charAt(at))) at += 1;
    return text.slice(start, at);
  };
  const expect = (char: string): void => {
    if (text.charAt(at) !== char) fail(`expected ${char}`);
    at += 1;
  };

  const parse = (depth: number): Ty => {
    if (depth > maxDepth) fail("nested deeper than 16 constructors");
    if (text.charAt(at) === "(") {
      at += 1;
      const of: Ty[] = [parse(depth + 1)];
      while (text.charAt(at) === ",") {
        at += 1;
        of.push(parse(depth + 1));
        if (of.length > 8) fail("tuple wider than 8 components");
      }
      expect(")");
      if (of.length < 2) fail("tuple with fewer than 2 components");
      return { k: "Tuple", of };
    }
    const word = identifier();
    if (word === "") return fail("a type was expected");
    if (word === "Bytes") {
      if (text.charAt(at) !== "<") return BYTES;
      at += 1;
      const close = text.indexOf(">", at);
      if (close < 0) return fail("unclosed Bytes<");
      const inside = text.slice(at, close);
      at = close + 1;
      if (DECIMAL.test(inside)) {
        const width = Number(inside);
        if (inside.length > 4 || width > MAX_WIDTH) return fail("width over 4096");
        return { k: "Bytes", n: width };
      }
      if (options.allowNamedWidth === true && NAME.test(inside)) return { k: "Bytes", n: inside };
      return fail("not a decimal width");
    }
    if (word === "List" || word === "Option") {
      expect("<");
      const inner = parse(depth + 1);
      expect(">");
      return word === "List" ? { k: "List", of: inner } : { k: "Option", of: inner };
    }
    if (word === "Result") {
      expect("<");
      const ok = parse(depth + 1);
      expect(",");
      const err = identifier();
      if (err === "" || !NAME.test(err)) return fail("an error type name was expected");
      expect(">");
      return { k: "Result", ok, err };
    }
    if (word === "Int" && options.allowInt !== true) return fail("Int is IR-internal");
    const scalar = SCALARS.get(word);
    if (scalar !== undefined) return scalar;
    return { k: "Name", name: word };
  };

  const result = parse(0);
  if (at !== text.length) fail("trailing text");
  return result;
}

// ---------------------------------------------------------------------------
// The `types` table (section 2.2)
// ---------------------------------------------------------------------------

/**
 * The part of the `types` table this lane needs: which names are error types
 * (and their cases, in declaration order) and which are records or variants
 * (their *level* is all this lane can say about them).
 */
export interface TypeTable {
  readonly errors: ReadonlyMap<string, readonly string[]>;
  readonly data: ReadonlyMap<string, "record" | "variant">;
}

export const EMPTY_TABLE: TypeTable = { errors: new Map(), data: new Map() };

/** Section 2.2 rules 3 and 4: a record field is a `[name, type]` pair, `name` in the name grammar and `type` text. */
function readField(field: unknown, code: KitCode): void {
  if (!Array.isArray(field) || field.length !== 2 || typeof field[0] !== "string" || !NAME.test(field[0]) || typeof field[1] !== "string") {
    throw new KitError(code, "a record field is [name, type] with a name in the name grammar");
  }
}

/**
 * Read a `types` object.  The rules are those of section 2.2: unique names, an
 * error type has one to 64 cases (unique, name grammar), a record one to 32
 * fields, a variant two to 64 constructors of zero to 8 fields.  Violations are
 * refused with `code` (the caller's category decides which).
 */
export function readTypeTable(raw: unknown, code: KitCode = "corpus"): TypeTable {
  if (typeof raw !== "object" || raw === null || Array.isArray(raw)) {
    throw new KitError(code, "types is not an object");
  }
  const errors = new Map<string, readonly string[]>();
  const data = new Map<string, "record" | "variant">();
  for (const [name, row] of Object.entries(raw as Record<string, unknown>)) {
    if (!NAME.test(name) || SCALARS.has(name) || ["Bytes", "List", "Option", "Result"].includes(name)) {
      throw new KitError(code, "a type name is not usable");
    }
    if (typeof row !== "object" || row === null || Array.isArray(row)) throw new KitError(code, "a types row is not an object");
    const entry = row as Record<string, unknown>;
    const kind = entry["kind"];
    if (kind === "error") {
      const cases = entry["cases"];
      if (!Array.isArray(cases) || cases.length < 1 || cases.length > 64) throw new KitError(code, "an error type has 1 to 64 cases");
      const seen = new Set<string>();
      for (const item of cases) {
        if (typeof item !== "string" || !NAME.test(item) || seen.has(item)) throw new KitError(code, "an error case is not a unique name");
        seen.add(item);
      }
      errors.set(name, cases as string[]);
    } else if (kind === "record") {
      const fields = entry["fields"];
      if (!Array.isArray(fields) || fields.length < 1 || fields.length > 32) throw new KitError(code, "a record has 1 to 32 fields");
      for (const field of fields) readField(field, code);
      data.set(name, "record");
    } else if (kind === "variant") {
      const ctors = entry["ctors"];
      if (!Array.isArray(ctors) || ctors.length < 2 || ctors.length > 64) throw new KitError(code, "a variant has 2 to 64 constructors");
      for (const ctor of ctors) {
        if (!Array.isArray(ctor) || ctor.length !== 2 || typeof ctor[0] !== "string" || !NAME.test(ctor[0])) {
          throw new KitError(code, "a constructor is [name, [types]] with a name in the name grammar");
        }
        const members: unknown = ctor[1];
        if (!Array.isArray(members) || members.length > 8) throw new KitError(code, "a constructor has 0 to 8 fields");
        if (members.some((member) => typeof member !== "string")) throw new KitError(code, "a constructor field is type text");
      }
      data.set(name, "variant");
    } else {
      throw new KitError(code, "a types row has an unknown kind");
    }
  }
  return { errors, data };
}

/**
 * Check that every name in `ty` is declared: an error type where a `Result`
 * wants one, a record or variant everywhere else.  A missing name is `code`.
 */
export function checkNames(ty: Ty, table: TypeTable, code: KitCode = "type"): void {
  switch (ty.k) {
    case "Tuple":
      for (const part of ty.of) checkNames(part, table, code);
      return;
    case "List":
      checkNames(ty.of, table, code);
      return;
    case "Option":
      if (ty.of !== null) checkNames(ty.of, table, code);
      return;
    case "Result":
      if (ty.ok !== null) checkNames(ty.ok, table, code);
      if (ty.err !== null && !table.errors.has(ty.err)) throw new KitError(code, "an error type is not declared");
      return;
    case "Name":
      if (!table.data.has(ty.name)) throw new KitError(code, "a type name is not declared");
      return;
    default:
  }
}

// ---------------------------------------------------------------------------
// Levels of a type (R69 (b))
// ---------------------------------------------------------------------------

/**
 * Add to `into` the levels `ty` needs: `Bool`, `Nat`, `Int`, fixed-width types,
 * `Bytes` and `Result<T,E>` are `ir.core` (a `Result` adds the level of `T`); a
 * tuple type is `ir.tuple`; a record, a variant and an `Option<T>` are
 * `ir.data`; `List<T>` is `ir.list`.  Components count too.
 */
export function typeLevels(ty: Ty, into: Set<string>): void {
  switch (ty.k) {
    case "Tuple":
      into.add("ir.tuple");
      for (const part of ty.of) typeLevels(part, into);
      return;
    case "List":
      into.add("ir.list");
      typeLevels(ty.of, into);
      return;
    case "Option":
      into.add("ir.data");
      if (ty.of !== null) typeLevels(ty.of, into);
      return;
    case "Name":
      into.add("ir.data");
      return;
    case "Result":
      into.add("ir.core");
      if (ty.ok !== null) typeLevels(ty.ok, into);
      return;
    default:
      into.add("ir.core");
  }
}

// ---------------------------------------------------------------------------
// Printing and structure
// ---------------------------------------------------------------------------

/**
 * The canonical text of a type.  A `Bytes` width is part of the static type
 * (R51, R73, Appendix A), so `Bytes<2>` prints as such: this is the text of a
 * kit case's `expect.type`.  (R51's remark that a width is "written nowhere"
 * is about the *corpus IR*, whose literals and signatures never state a result
 * width; it does not say the checker forgets one.)  With `plain`, the width is
 * dropped, which only a human-facing message wants.
 */
export function showType(ty: Ty, plain = false): string {
  switch (ty.k) {
    case "Bool":
    case "Int":
    case "Nat":
      return ty.k;
    case "U":
      return `U${ty.w}`;
    case "I":
      return `I${ty.w}`;
    case "Bytes":
      return ty.n === null || plain ? "Bytes" : `Bytes<${ty.n}>`;
    case "Tuple":
      return `(${ty.of.map((part) => showType(part, plain)).join(",")})`;
    case "List":
      return `List<${showType(ty.of, plain)}>`;
    case "Option":
      return `Option<${ty.of === null ? "?" : showType(ty.of, plain)}>`;
    case "Result":
      return `Result<${ty.ok === null ? "?" : showType(ty.ok, plain)},${ty.err ?? "?"}>`;
    case "Name":
      return ty.name;
  }
}

/** Structural equality, widths included. */
export function sameType(a: Ty, b: Ty): boolean {
  if (a.k !== b.k) return false;
  switch (a.k) {
    case "U":
    case "I":
      return a.w === (b as typeof a).w;
    case "Bytes":
      return a.n === (b as typeof a).n;
    case "Tuple": {
      const other = b as typeof a;
      return a.of.length === other.of.length && a.of.every((part, i) => sameType(part, other.of[i] as Ty));
    }
    case "List":
      return sameType(a.of, (b as typeof a).of);
    case "Option": {
      const other = (b as typeof a).of;
      return a.of === null || other === null ? a.of === other : sameType(a.of, other);
    }
    case "Result": {
      const other = b as typeof a;
      const okSame = a.ok === null || other.ok === null ? a.ok === other.ok : sameType(a.ok, other.ok);
      return okSame && a.err === other.err && (a.cands ?? []).join(",") === (other.cands ?? []).join(",");
    }
    case "Name":
      return a.name === (b as typeof a).name;
    default:
      return true;
  }
}

/** Does `ty` mention an undetermined component (a bare `ok`, a bare `err`)? */
export function isUndetermined(ty: Ty): boolean {
  switch (ty.k) {
    case "Result":
      return ty.ok === null || ty.err === null || isUndetermined(ty.ok);
    case "Option":
      return ty.of === null || isUndetermined(ty.of);
    case "Tuple":
      return ty.of.some(isUndetermined);
    case "List":
      return isUndetermined(ty.of);
    default:
      return false;
  }
}

/** Is `ty` a fixed-width integer type? */
export function isFixed(ty: Ty): ty is { readonly k: "U" | "I"; readonly w: Width } {
  return ty.k === "U" || ty.k === "I";
}

/** Is `ty` `Nat` or `Int` (the exact, unbounded-in-principle integers)? */
export function isExact(ty: Ty): boolean {
  return ty.k === "Nat" || ty.k === "Int";
}

/** Is `ty` any integer type? */
export function isInteger(ty: Ty): boolean {
  return isFixed(ty) || isExact(ty);
}

// ---------------------------------------------------------------------------
// flows and unify (R51)
// ---------------------------------------------------------------------------

/** Can a value of error type `err` (or an undetermined one with `cands`) stand where `target` is wanted? */
function errorsMeet(a: { err: string | null; cands?: readonly string[] }, b: { err: string | null; cands?: readonly string[] }): string | null | undefined {
  if (a.err !== null && b.err !== null) return a.err === b.err ? a.err : undefined;
  if (a.err === null && b.err === null) {
    if (a.cands === undefined) return b.cands === undefined ? null : b.cands.length === 1 ? (b.cands[0] as string) : null;
    if (b.cands === undefined) return a.cands.length === 1 ? (a.cands[0] as string) : null;
    const both = a.cands.filter((name) => (b.cands as readonly string[]).includes(name));
    return both.length === 0 ? undefined : both.length === 1 ? (both[0] as string) : null;
  }
  const known = a.err !== null ? a : b;
  const open = a.err !== null ? b : a;
  if (open.cands !== undefined && !open.cands.includes(known.err as string)) return undefined;
  return known.err;
}

/**
 * `flows(s, t)`: a value of type `s` may be used where `t` is wanted (R51).
 * A type flows to itself; `Nat` to `Int`; `Bytes<N>` to `Bytes`; `Option` and
 * `Result` lift by their payload (the same error type); nothing else -- a
 * tuple, a record, a variant and a list are invariant.
 */
export function flows(s: Ty, t: Ty): boolean {
  if (sameType(s, t)) return true;
  if (s.k === "Nat" && t.k === "Int") return true;
  if (s.k === "Bytes" && t.k === "Bytes") return t.n === null;
  if (s.k === "Option" && t.k === "Option") return s.of === null || t.of === null ? true : flows(s.of, t.of);
  if (s.k === "Result" && t.k === "Result") {
    const payload = s.ok === null || t.ok === null ? true : flows(s.ok, t.ok);
    return payload && errorsMeet(s, t) !== undefined;
  }
  return false;
}

/**
 * `unify(a, b)`: the common type (R51's `common_type`), or `null` when there is
 * none.  Equal types; `Nat` and `Int` to `Int`; two `Bytes` types of different
 * widths to plain `Bytes` (but a literal width against a named static width is
 * not unified); `Option` and `Result` by their payloads.
 */
export function unify(a: Ty, b: Ty): Ty | null {
  if (sameType(a, b)) return a;
  if ((a.k === "Nat" && b.k === "Int") || (a.k === "Int" && b.k === "Nat")) return INT;
  if (a.k === "Bytes" && b.k === "Bytes") {
    if ((typeof a.n === "string") !== (typeof b.n === "string") && a.n !== null && b.n !== null) return null;
    return BYTES;
  }
  if (a.k === "Option" && b.k === "Option") {
    if (a.of === null) return b;
    if (b.of === null) return a;
    const inner = unify(a.of, b.of);
    return inner === null ? null : { k: "Option", of: inner };
  }
  if (a.k === "Result" && b.k === "Result") {
    const ok = a.ok === null ? b.ok : b.ok === null ? a.ok : unify(a.ok, b.ok);
    if (ok === null && a.ok !== null && b.ok !== null) return null;
    const err = errorsMeet(a, b);
    if (err === undefined) return null;
    const cands = a.err === null && b.err === null ? commonCands(a, b) : undefined;
    return cands === undefined ? { k: "Result", ok, err } : { k: "Result", ok, err, cands };
  }
  return null;
}

function commonCands(a: { cands?: readonly string[] }, b: { cands?: readonly string[] }): readonly string[] | undefined {
  if (a.cands === undefined) return b.cands;
  if (b.cands === undefined) return a.cands;
  return a.cands.filter((name) => (b.cands as readonly string[]).includes(name));
}

// ---------------------------------------------------------------------------
// Static widths of ops (R70)
// ---------------------------------------------------------------------------

/**
 * Replace each named width `N` of `ty` by the width at its position in `names`
 * (R70: a `subject_op` call site substitutes its static widths into the op's
 * signature before typing the arguments).
 */
export function substituteWidths(ty: Ty, names: readonly string[], widths: readonly number[]): Ty {
  const go = (t: Ty): Ty => {
    switch (t.k) {
      case "Bytes": {
        if (typeof t.n !== "string") return t;
        const index = names.indexOf(t.n);
        return index < 0 ? t : { k: "Bytes", n: widths[index] as number };
      }
      case "Tuple":
        return { k: "Tuple", of: t.of.map(go) };
      case "List":
        return { k: "List", of: go(t.of) };
      case "Option":
        return t.of === null ? t : { k: "Option", of: go(t.of) };
      case "Result":
        return t.ok === null ? t : { ...t, ok: go(t.ok) };
      default:
        return t;
    }
  };
  return go(ty);
}

/**
 * Leave every named width unstated: `Bytes<N>` becomes plain `Bytes` (R70 for a
 * fault row, which has no call site, and for the stand-in of a `v2-ir` case).
 */
export function eraseNamedWidths(ty: Ty): Ty {
  const go = (t: Ty): Ty => {
    switch (t.k) {
      case "Bytes":
        return typeof t.n === "string" ? BYTES : t;
      case "Tuple":
        return { k: "Tuple", of: t.of.map(go) };
      case "List":
        return { k: "List", of: go(t.of) };
      case "Option":
        return t.of === null ? t : { k: "Option", of: go(t.of) };
      case "Result":
        return t.ok === null ? t : { ...t, ok: go(t.ok) };
      default:
        return t;
    }
  };
  return go(ty);
}

/** The number of bits of a fixed-width type, or `undefined`. */
export function widthOf(ty: Ty): Width | undefined {
  return ty.k === "U" || ty.k === "I" ? ty.w : undefined;
}
