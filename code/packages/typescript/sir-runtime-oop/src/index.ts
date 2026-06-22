// @coding-adventures/sir-runtime-oop
//
// OOP runtime primitives for Semantic-IR-emitted TypeScript / JavaScript.
//
// ## Why this package exists
//
// Most SIR constructs translate to *native* TypeScript (a sequence is an
// `Array`, a loop is a `for`).  Ruby-style object orientation does not survive
// that translation cleanly, for one structural reason:
//
//   **The Ruby→SIR frontend HOISTS every method to a detached top-level
//   function with no receiver (no `self`).**
//
// So inside an emitted method there is no `this`/`self` to hang an instance
// variable on, and a class-variable assignment carries no enclosing-class
// context.  Native member access (`this.x`) is therefore impossible.  This
// package supplies the missing object model as an explicit, in-process runtime:
//
//   - a **class registry** (`defineClass`) + ancestry-aware `isA`,
//   - an **instance-variable store** addressed through a *current-self* stack
//     (`pushSelf` / `popSelf` / `ivarGet` / `ivarSet`),
//   - a **class-variable store** (`cvarGet` / `cvarSet`),
//   - **method dispatch** (`callMethod`) covering the reflective built-ins the
//     frontend emits (`is_a?`, `kind_of?`, `instance_of?`, `class`) plus a
//     `defineMethod` table for singleton-method attachment.
//
// ## Honest v0 limitation
//
// Because the frontend does not thread receivers, the *current self* is a
// process-global stack rather than a true per-call binding, and class variables
// share a single namespace keyed by bare name.  This faithfully models
// single-instance / single-class programs and never crashes on the OO surface,
// but full multi-object Ruby semantics await a frontend that carries receivers
// into method bodies (out of scope for the backend).  See
// `code/specs/sir-runtime.md`.

// Block-taking catalog methods (each/map/select/…) invoke a Ruby block.  A block
// reaches us as a trailing `Closure` from sir-runtime-core; `apply` calls it with
// proc-lenient arity, and `truthy` applies SIR truthiness (only `false`/`nil` are
// falsy) to predicate results.
import { apply, Closure, intern, truthy } from "@coding-adventures/sir-runtime-core";

/**
 * The SIR universal value type at this package's boundary.  Kept as `unknown`'s
 * permissive sibling so emitted code can pass `@coding-adventures/sir-runtime-core`
 * `Val`s in and assign results back without casts.
 */
// eslint-disable-next-line @typescript-eslint/no-explicit-any
export type Val = any;

// ── Class registry ──────────────────────────────────────────────────────────

interface ClassInfo {
  readonly name: string;
  readonly superName: string | null;
}

const classes = new Map<string, ClassInfo>();

/**
 * Register a class and (optionally) its superclass.  Emitted from a SIR
 * `ClassDef` so later `isA` queries can walk the ancestry chain.  Re-defining a
 * class replaces its prior registration (matching Ruby's open classes).
 */
export function defineClass(name: string, superName: string | null = null): void {
  classes.set(name, { name, superName });
}

/** The registered superclass name of `name`, or `null` if none/unknown. */
export function superclassOf(name: string): string | null {
  return classes.get(name)?.superName ?? null;
}

// ── Instances ────────────────────────────────────────────────────────────────

/**
 * A SIR object instance: a class tag plus a bag of instance variables.  Created
 * by `newInstance`; instance variables are read/written through the current-self
 * stack rather than direct field access (see module docs).
 */
export class SirInstance {
  readonly sirClass: string;
  readonly ivars: Map<string, Val> = new Map<string, Val>();
  constructor(sirClass: string) {
    this.sirClass = sirClass;
  }
}

/** Allocate a fresh instance tagged with `className`. */
export function newInstance(className: string): SirInstance {
  return new SirInstance(className);
}

// ── Current-self stack + instance-variable store ─────────────────────────────

const selfStack: SirInstance[] = [];

// A program that never pushes a self (the common detached-method case) still
// needs a place to put instance variables; this default object provides it so
// `@x` reads/writes never throw.
const defaultSelf = new SirInstance("Object");

function currentSelf(): SirInstance {
  return selfStack.length > 0 ? selfStack[selfStack.length - 1]! : defaultSelf;
}

/** Make `obj` the receiver for subsequent `ivarGet`/`ivarSet` calls. */
export function pushSelf(obj: SirInstance): void {
  selfStack.push(obj);
}

/** Pop the most recently pushed receiver. */
export function popSelf(): void {
  selfStack.pop();
}

/**
 * Read instance variable `name` (including the leading `@`) on the current self.
 * Unset instance variables read as `null` (Ruby's `nil`), never throw.
 */
export function ivarGet(name: string): Val {
  const v = currentSelf().ivars.get(name);
  return v === undefined ? null : v;
}

/** Set instance variable `name` on the current self; returns the value. */
export function ivarSet(name: string, value: Val): Val {
  currentSelf().ivars.set(name, value);
  return value;
}

// ── Class-variable store ─────────────────────────────────────────────────────

const cvars = new Map<string, Val>();

/** Read class variable `name` (including `@@`); unset reads as `null`. */
export function cvarGet(name: string): Val {
  const v = cvars.get(name);
  return v === undefined ? null : v;
}

/** Set class variable `name`; returns the value. */
export function cvarSet(name: string, value: Val): Val {
  cvars.set(name, value);
  return value;
}

// ── Class identity / ancestry ────────────────────────────────────────────────

/**
 * The Ruby class name of a value: registered `SirInstance` tag for objects, or
 * the conventional built-in name for primitives (`Integer`, `Float`, `String`,
 * `Array`, `Hash`, `NilClass`, `TrueClass`, `FalseClass`, else `Object`).
 */
export function classOf(value: Val): string {
  if (value instanceof SirInstance) return value.sirClass;
  if (value === null || value === undefined) return "NilClass";
  switch (typeof value) {
    case "boolean":
      return value ? "TrueClass" : "FalseClass";
    case "number":
      return Number.isInteger(value) ? "Integer" : "Float";
    case "string":
      return "String";
    default:
      if (Array.isArray(value)) return "Array";
      if (value instanceof Map) return "Hash";
      return "Object";
  }
}

/**
 * `true` if `value` is an instance of class `className` or any of its
 * registered ancestors.  Primitive built-in names are matched structurally;
 * `Numeric` matches both `Integer` and `Float`; `Object`/`BasicObject` match
 * everything (the universal Ruby roots).
 */
export function isA(value: Val, className: string): boolean {
  if (className === "Object" || className === "BasicObject") return true;
  const actual = classOf(value);
  if (className === "Numeric") return actual === "Integer" || actual === "Float";
  // Walk the registered ancestry chain (guards against cycles).
  let cur: string | null = actual;
  const seen = new Set<string>();
  while (cur !== null && !seen.has(cur)) {
    if (cur === className) return true;
    seen.add(cur);
    cur = superclassOf(cur);
  }
  return false;
}

// ── Method dispatch ──────────────────────────────────────────────────────────

const methods = new Map<string, (recv: Val, args: Val[]) => Val>();

/**
 * Attach a (singleton/instance) method implementation under `name`.  Used to
 * model `def obj.m` / `class << self` once a frontend supplies bodies; today it
 * backs the `callMethod` fallback.
 */
export function defineMethod(name: string, fn: (recv: Val, args: Val[]) => Val): void {
  methods.set(name, fn);
}

// ── Built-in method catalog (SIR method-dispatch spec, M1) ───────────────────
//
// `recv.meth(args…)` reaches the backend as `BuiltinCall("__method__", [recv,
// "meth", …])` and is dispatched here.  Before this catalog every method outside
// the reflective four + the `defineMethod` table returned `null` — so
// `[1,2,3].reverse` evaluated to nil instead of running.  This catalog gives the
// everyday Ruby built-ins their faithful native behaviour, dispatched by the
// receiver's runtime type.  See `code/specs/sir-method-dispatch.md`.
//
// **This file (M1a) covers the *non-block* Array surface plus the universal
// Object methods.**  Block-taking methods (`each`/`map`/`select`/…) and the
// Hash/String/Numeric/Symbol catalogs arrive in follow-up PRs that take the
// `@coding-adventures/sir-runtime-core` `apply` dependency for proc-lenient block
// invocation.
//
// Resolution order (see `callMethod`): reflective built-ins → user `defineMethod`
// table → this catalog → `null` floor.  `respond_to?` reports catalog membership
// honestly, so an out-of-catalog method is both `null` and `respond_to? == false`.

// Sentinel meaning "this name is not in the catalog for this receiver" — distinct
// from a catalog method that legitimately returns `null` (Ruby `nil`).
const MISS: unique symbol = Symbol("sir-oop-miss");

const OBJECT_METHODS = new Set<string>([
  "nil?",
  "==",
  "!=",
  "equal?",
  "respond_to?",
  "freeze",
  "frozen?",
  "dup",
  "clone",
  "itself",
  "to_a",
]);

// Non-block `Array` methods (M1a); block methods land in a later PR, kept absent
// here so `respond_to?` stays honest.
const ARRAY_METHODS = new Set<string>([
  "length",
  "size",
  "count",
  "first",
  "last",
  "include?",
  "index",
  "push",
  "append",
  "<<",
  "pop",
  "shift",
  "unshift",
  "prepend",
  "reverse",
  "sort",
  "min",
  "max",
  "sum",
  "uniq",
  "flatten",
  "compact",
  "empty?",
  "to_a",
]);

// Block-taking `Array`/`Enumerable` methods (M1b); each invokes a trailing
// `Closure` block via `apply`. Listed so `respond_to?` reports them.
const ARRAY_BLOCK_METHODS = new Set<string>([
  "each",
  "each_with_index",
  "map",
  "collect",
  "select",
  "filter",
  "reject",
  "reduce",
  "inject",
  "find",
  "detect",
  "flat_map",
  "any?",
  "all?",
  "none?",
]);

// Non-block `Hash` methods (M1c). Hash is a JS `Map`.
const HASH_METHODS = new Set<string>([
  "keys",
  "values",
  "has_key?",
  "key?",
  "include?",
  "member?",
  "has_value?",
  "value?",
  "fetch",
  "size",
  "length",
  "empty?",
  "to_a",
  "dig",
  "store",
  "[]=",
  "merge",
  "delete",
  "clear",
  "invert",
]);

// Block-taking `Hash` methods (M1c); the block receives `[key, value]`.
const HASH_BLOCK_METHODS = new Set<string>([
  "each",
  "each_pair",
  "map",
  "select",
  "filter",
  "reject",
  "each_key",
  "each_value",
]);

// Non-block `String` methods (M1c).  A Ruby `String` is a JS `string`, which is
// **immutable** — so every method here is non-mutating and returns a fresh value
// (the in-place `upcase!` family is out of v0 scope).  `sub`/`gsub` here are the
// *literal* forms: the pattern is matched as a plain substring (never a regex)
// and the replacement is inserted verbatim — crucially side-stepping JS's
// `String.prototype.replace` special-replacement parsing (`$&`, `$1`, `$$`).
const STRING_METHODS = new Set<string>([
  "length",
  "size",
  "upcase",
  "downcase",
  "capitalize",
  "reverse",
  "strip",
  "lstrip",
  "rstrip",
  "chomp",
  "chars",
  "bytes",
  "split",
  "include?",
  "start_with?",
  "end_with?",
  "index",
  "replace",
  "sub",
  "gsub",
  "to_i",
  "to_f",
  "to_sym",
  "empty?",
  "*",
  "+",
]);

// Block-taking `String` methods (M1c); `each_char` yields one character.
const STRING_BLOCK_METHODS = new Set<string>(["each_char"]);

/** SIR value equality used by `include?`/`index`/`==` — `===` for primitives,
 * structural for arrays and `Map`s (Ruby `==` is deep). */
function valEq(a: Val, b: Val): boolean {
  if (a === b) return true;
  if (Array.isArray(a) && Array.isArray(b)) {
    return a.length === b.length && a.every((x: Val, i: number) => valEq(x, b[i]));
  }
  if (a instanceof Map && b instanceof Map) {
    if (a.size !== b.size) return false;
    for (const [k, v] of a) {
      if (!b.has(k) || !valEq(v, b.get(k))) return false;
    }
    return true;
  }
  return false;
}

/** Coerce a `respond_to?` argument (a core `Symbol`, `":m"`-ish string, or bare
 * name) to the plain method name used as the catalog key. */
function methodNameArg(arg: Val): string {
  if (arg !== null && typeof arg === "object" && typeof arg.name === "string") {
    return arg.name as string;
  }
  return String(arg);
}

/** Whether dispatch on `recv` resolves `name` — across the reflective built-ins,
 * the `defineMethod` table, and the type-specific catalog. */
function respondsTo(recv: Val, name: string): boolean {
  if (name === "is_a?" || name === "kind_of?" || name === "instance_of?" || name === "class") {
    return true;
  }
  if (methods.has(name)) return true;
  if (OBJECT_METHODS.has(name)) return true;
  if (typeof recv === "string" && (STRING_METHODS.has(name) || STRING_BLOCK_METHODS.has(name))) {
    return true;
  }
  if (Array.isArray(recv) && (ARRAY_METHODS.has(name) || ARRAY_BLOCK_METHODS.has(name))) {
    return true;
  }
  if (recv instanceof Map && (HASH_METHODS.has(name) || HASH_BLOCK_METHODS.has(name))) {
    return true;
  }
  return false;
}

function flattenArray(seq: Val[]): Val[] {
  const out: Val[] = [];
  for (const item of seq) {
    if (Array.isArray(item)) out.push(...flattenArray(item));
    else out.push(item);
  }
  return out;
}

function uniqArray(seq: Val[]): Val[] {
  const out: Val[] = [];
  for (const item of seq) {
    if (!out.some((x: Val) => valEq(x, item))) out.push(item);
  }
  return out;
}

/** Universal `Object` methods.  Returns `MISS` if `name` is not universal. */
function objectMethod(recv: Val, name: string, args: Val[]): Val | typeof MISS {
  switch (name) {
    case "nil?":
      return recv === null || recv === undefined;
    case "==":
      return valEq(recv, args[0]);
    case "!=":
      return !valEq(recv, args[0]);
    case "equal?":
      return recv === args[0];
    case "respond_to?":
      return respondsTo(recv, methodNameArg(args[0]));
    case "itself":
      return recv;
    case "freeze":
      // No true immutability in v0 — identity-returning, matching Ruby's shape.
      return recv;
    case "frozen?":
      return (
        recv === null ||
        recv === undefined ||
        typeof recv === "number" ||
        typeof recv === "boolean"
      );
    case "dup":
    case "clone":
      if (Array.isArray(recv)) return [...recv];
      if (recv instanceof Map) return new Map(recv);
      return recv;
    case "to_a":
      // Ruby: nil.to_a == [], Array#to_a == self; others fall through.
      if (recv === null || recv === undefined) return [];
      if (Array.isArray(recv)) return recv;
      return MISS;
    default:
      return MISS;
  }
}

/** Non-block `Array` methods.  Returns `MISS` if `name` is not catalogued. */
function arrayMethod(recv: Val[], name: string, args: Val[]): Val | typeof MISS {
  switch (name) {
    case "length":
    case "size":
      return recv.length;
    case "count":
      return args.length > 0 ? recv.filter((x: Val) => valEq(x, args[0])).length : recv.length;
    case "first":
      if (args.length > 0) return recv.slice(0, args[0]);
      return recv.length > 0 ? recv[0] : null;
    case "last":
      if (args.length > 0) return args[0] ? recv.slice(-args[0]) : [];
      return recv.length > 0 ? recv[recv.length - 1] : null;
    case "include?":
      return recv.some((x: Val) => valEq(x, args[0]));
    case "index": {
      const i = recv.findIndex((x: Val) => valEq(x, args[0]));
      return i === -1 ? null : i;
    }
    case "push":
    case "append":
      recv.push(...args);
      return recv;
    case "<<":
      recv.push(args[0]);
      return recv;
    case "pop":
      return recv.length > 0 ? recv.pop() : null;
    case "shift":
      return recv.length > 0 ? recv.shift() : null;
    case "unshift":
    case "prepend":
      recv.unshift(...args);
      return recv;
    case "reverse":
      return [...recv].reverse();
    case "sort":
      // `<`/`>` ordering keeps numbers numeric (JS default sort is lexicographic).
      return [...recv].sort((a: Val, b: Val) => (a < b ? -1 : a > b ? 1 : 0));
    case "min":
      return recv.length > 0 ? recv.reduce((a: Val, b: Val) => (b < a ? b : a)) : null;
    case "max":
      return recv.length > 0 ? recv.reduce((a: Val, b: Val) => (b > a ? b : a)) : null;
    case "sum": {
      let total: Val = args.length > 0 ? args[0] : 0;
      for (const item of recv) total = total + item;
      return total;
    }
    case "uniq":
      return uniqArray(recv);
    case "flatten":
      return flattenArray(recv);
    case "compact":
      return recv.filter((x: Val) => x !== null && x !== undefined);
    case "empty?":
      return recv.length === 0;
    case "to_a":
      return recv;
    default:
      return MISS;
  }
}

/** Block-taking `Array`/`Enumerable` methods.  `block` is applied via `apply`
 * (proc-lenient); predicate results route through SIR `truthy`.  Returns `MISS`
 * if `name` is not a block method. */
function arrayBlockMethod(
  recv: Val[],
  name: string,
  args: Val[],
  block: Closure,
): Val | typeof MISS {
  switch (name) {
    case "each":
      for (const item of recv) apply(block, [item]);
      return recv;
    case "each_with_index":
      recv.forEach((item: Val, index: number) => apply(block, [item, index]));
      return recv;
    case "map":
    case "collect":
      return recv.map((item: Val) => apply(block, [item]));
    case "select":
    case "filter":
      return recv.filter((item: Val) => truthy(apply(block, [item])));
    case "reject":
      return recv.filter((item: Val) => !truthy(apply(block, [item])));
    case "reduce":
    case "inject": {
      let acc: Val;
      let rest: Val[];
      if (args.length > 0) {
        acc = args[0];
        rest = recv;
      } else if (recv.length > 0) {
        acc = recv[0];
        rest = recv.slice(1);
      } else {
        return null;
      }
      for (const item of rest) acc = apply(block, [acc, item]);
      return acc;
    }
    case "find":
    case "detect":
      for (const item of recv) {
        if (truthy(apply(block, [item]))) return item;
      }
      return null;
    case "flat_map": {
      const out: Val[] = [];
      for (const item of recv) {
        const mapped = apply(block, [item]);
        if (Array.isArray(mapped)) out.push(...mapped);
        else out.push(mapped);
      }
      return out;
    }
    case "any?":
      return recv.some((item: Val) => truthy(apply(block, [item])));
    case "all?":
      return recv.every((item: Val) => truthy(apply(block, [item])));
    case "none?":
      return !recv.some((item: Val) => truthy(apply(block, [item])));
    default:
      return MISS;
  }
}

/** Non-block `Hash` methods (Hash is a `Map`). Returns `MISS` if not catalogued. */
function hashMethod(recv: Map<Val, Val>, name: string, args: Val[]): Val | typeof MISS {
  switch (name) {
    case "keys":
      return [...recv.keys()];
    case "values":
      return [...recv.values()];
    case "has_key?":
    case "key?":
    case "include?":
    case "member?":
      return recv.has(args[0]);
    case "has_value?":
    case "value?":
      return [...recv.values()].some((v: Val) => valEq(v, args[0]));
    case "fetch":
      if (recv.has(args[0])) return recv.get(args[0]);
      return args.length > 1 ? args[1] : null;
    case "size":
    case "length":
      return recv.size;
    case "empty?":
      return recv.size === 0;
    case "to_a":
      return [...recv.entries()].map(([k, v]: [Val, Val]) => [k, v]);
    case "dig":
      // v0: single-level dig.
      return recv.has(args[0]) ? recv.get(args[0]) : null;
    case "store":
    case "[]=":
      recv.set(args[0], args[1]);
      return args[1];
    case "merge":
      return new Map<Val, Val>([...recv, ...(args[0] as Map<Val, Val>)]);
    case "delete": {
      if (!recv.has(args[0])) return null;
      const v = recv.get(args[0]);
      recv.delete(args[0]);
      return v;
    }
    case "clear":
      recv.clear();
      return recv;
    case "invert":
      return new Map<Val, Val>([...recv].map(([k, v]: [Val, Val]) => [v, k]));
    default:
      return MISS;
  }
}

/** Block-taking `Hash` methods; the block receives `[key, value]` (or a single
 * key/value for `each_key`/`each_value`). Returns `MISS` if not a block method. */
function hashBlockMethod(recv: Map<Val, Val>, name: string, block: Closure): Val | typeof MISS {
  switch (name) {
    case "each":
    case "each_pair":
      for (const [k, v] of [...recv]) apply(block, [k, v]);
      return recv;
    case "each_key":
      for (const k of [...recv.keys()]) apply(block, [k]);
      return recv;
    case "each_value":
      for (const v of [...recv.values()]) apply(block, [v]);
      return recv;
    case "map":
      return [...recv].map(([k, v]: [Val, Val]) => apply(block, [k, v]));
    case "select":
    case "filter":
      return new Map<Val, Val>([...recv].filter(([k, v]: [Val, Val]) => truthy(apply(block, [k, v]))));
    case "reject":
      return new Map<Val, Val>([...recv].filter(([k, v]: [Val, Val]) => !truthy(apply(block, [k, v]))));
    default:
      return MISS;
  }
}

// Leading-numeric extractors for `String#to_i` / `String#to_f`.  Ruby parses an
// optional sign and the longest leading numeric run, ignoring surrounding
// whitespace, and yields `0` / `0.0` when nothing numeric leads — never an error
// (unlike JS `Number(...)`, which yields `NaN`).
const INT_PREFIX = /^[+-]?\d+/;
const FLOAT_PREFIX = /^[+-]?(?:\d+\.?\d*|\.\d+)(?:[eE][+-]?\d+)?/;

function strToI(s: string): number {
  const match = INT_PREFIX.exec(s.trim());
  return match ? parseInt(match[0], 10) : 0;
}

function strToF(s: string): number {
  const match = FLOAT_PREFIX.exec(s.trim());
  return match ? parseFloat(match[0]) : 0.0;
}

// Upper bound on the character count `String#*` will produce.  A repeat count
// can come from untrusted input (e.g. `gets.to_i`); without a cap, `repeat`
// throws `RangeError: Invalid string length` (a host-implementation leak) or
// attempts a huge allocation.  Past the cap we yield `""` rather than throw —
// honouring the runtime's "never crash on the OO surface" invariant.
const MAX_REPEAT_LEN = 100_000_000;

function strRepeat(s: string, count: Val): string {
  const n = typeof count === "number" ? Math.trunc(count) : 0;
  if (n <= 0 || s.length === 0) return "";
  const capped = s.length * n > MAX_REPEAT_LEN ? Math.floor(MAX_REPEAT_LEN / s.length) : n;
  return s.repeat(capped);
}

/** Ruby `String#chomp`: drop a trailing record separator.  With an explicit
 * `sep`, drop exactly that suffix; with none, drop one trailing `\r\n`, `\n`, or
 * `\r` (Ruby's default line-ending handling). */
function chompStr(s: string, sep: Val): string {
  if (sep !== null && sep !== undefined) {
    return sep && s.endsWith(sep) ? s.slice(0, -(sep as string).length) : s;
  }
  if (s.endsWith("\r\n")) return s.slice(0, -2);
  if (s.endsWith("\n") || s.endsWith("\r")) return s.slice(0, -1);
  return s;
}

/** Non-block `String` methods.  Returns `MISS` if `name` is not catalogued.
 * Every result is a fresh value — JS strings are immutable, so nothing mutates
 * `recv` in place. */
function stringMethod(recv: string, name: string, args: Val[]): Val | typeof MISS {
  switch (name) {
    case "length":
    case "size":
      return recv.length;
    case "upcase":
      return recv.toUpperCase();
    case "downcase":
      return recv.toLowerCase();
    case "capitalize":
      // Ruby: first char upcased, the rest downcased.
      return recv.length === 0 ? recv : recv.charAt(0).toUpperCase() + recv.slice(1).toLowerCase();
    case "reverse":
      return [...recv].reverse().join("");
    case "strip":
      return recv.trim();
    case "lstrip":
      return recv.trimStart();
    case "rstrip":
      return recv.trimEnd();
    case "chomp":
      return chompStr(recv, args.length > 0 ? args[0] : null);
    case "chars":
      return [...recv];
    case "bytes":
      return [...new TextEncoder().encode(recv)];
    case "split": {
      // No argument ⇒ split on runs of whitespace (Ruby's awk-style default,
      // dropping leading/trailing empties); with a separator ⇒ literal split.
      if (args.length === 0) {
        const trimmed = recv.trim();
        return trimmed === "" ? [] : trimmed.split(/\s+/);
      }
      return recv.split(args[0]);
    }
    case "include?":
      return recv.includes(args[0]);
    case "start_with?":
      return recv.startsWith(args[0]);
    case "end_with?":
      return recv.endsWith(args[0]);
    case "index": {
      const i = recv.indexOf(args[0]);
      return i === -1 ? null : i;
    }
    case "replace":
      // Ruby `String#replace` overwrites the whole content; for an immutable
      // string that is just the replacement value.
      return args[0];
    case "sub": {
      // Literal first-occurrence replacement — done by index, so the
      // replacement is inserted verbatim (no `$&`/`$1` expansion).
      const search = args[0] as string;
      const idx = recv.indexOf(search);
      return idx === -1 ? recv : recv.slice(0, idx) + args[1] + recv.slice(idx + search.length);
    }
    case "gsub":
      // Literal global replacement via split/join — immune to special-replacement
      // parsing that `String.prototype.replaceAll` would apply to a string arg.
      return recv.split(args[0]).join(args[1]);
    case "to_i":
      return strToI(recv);
    case "to_f":
      return strToF(recv);
    case "to_sym":
      return intern(recv);
    case "empty?":
      return recv.length === 0;
    case "*":
      return strRepeat(recv, args[0]);
    case "+":
      return recv + args[0];
    default:
      return MISS;
  }
}

/** Block-taking `String` methods.  Returns `MISS` if `name` is not a string
 * block method. */
function stringBlockMethod(recv: string, name: string, block: Closure): Val | typeof MISS {
  switch (name) {
    case "each_char":
      for (const ch of recv) apply(block, [ch]);
      return recv;
    default:
      return MISS;
  }
}

/**
 * Dispatch method `name` on `recv`.  Resolution order:
 *
 * 1. **Reflective built-ins** the SIR frontend emits as `__method__` calls —
 *    `is_a?`/`kind_of?`/`instance_of?` (predicate against a class) and `class`.
 * 2. The user `defineMethod` table.
 * 3. The **built-in method catalog** (universal `Object` methods, and — when
 *    `recv` is an array — the non-block `Array` methods).
 * 4. `null` (Ruby `nil`) for anything still unresolved — the honest floor;
 *    `respond_to?` reports exactly which names resolve.
 *
 * The class argument to a predicate may arrive as a class-name **string** or as
 * a value whose class is taken; `instance_of?` additionally requires an exact
 * (non-ancestor) match.
 */
export function callMethod(recv: Val, name: string, ...args: Val[]): Val {
  switch (name) {
    case "is_a?":
    case "kind_of?":
      return isA(recv, classNameArg(args[0]));
    case "instance_of?":
      return classOf(recv) === classNameArg(args[0]);
    case "class":
      return classOf(recv);
  }

  const m = methods.get(name);
  if (m) return m(recv, args);

  if (typeof recv === "string") {
    // A block method (each_char) dispatches only with a trailing Closure.
    const last = args[args.length - 1];
    if (STRING_BLOCK_METHODS.has(name) && args.length > 0 && last instanceof Closure) {
      const blkResult = stringBlockMethod(recv, name, last);
      if (blkResult !== MISS) return blkResult;
    }
    const strResult = stringMethod(recv, name, args);
    if (strResult !== MISS) return strResult;
  } else if (Array.isArray(recv)) {
    // A block method (each/map/…) is dispatched only when an actual trailing
    // Closure block is present; the block is split off the positional args.
    const last = args[args.length - 1];
    if (ARRAY_BLOCK_METHODS.has(name) && args.length > 0 && last instanceof Closure) {
      const blkResult = arrayBlockMethod(recv, name, args.slice(0, -1), last);
      if (blkResult !== MISS) return blkResult;
    }
    const arrResult = arrayMethod(recv, name, args);
    if (arrResult !== MISS) return arrResult;
  } else if (recv instanceof Map) {
    const last = args[args.length - 1];
    if (HASH_BLOCK_METHODS.has(name) && args.length > 0 && last instanceof Closure) {
      const blkResult = hashBlockMethod(recv, name, last);
      if (blkResult !== MISS) return blkResult;
    }
    const hashResult = hashMethod(recv, name, args);
    if (hashResult !== MISS) return hashResult;
  }
  const objResult = objectMethod(recv, name, args);
  if (objResult !== MISS) return objResult;

  return null;
}

function classNameArg(arg: Val): string {
  return typeof arg === "string" ? arg : classOf(arg);
}

/**
 * Reset all OOP runtime state — class registry, self stack, instance/class
 * variable stores, and the method table.  Primarily for test isolation.
 */
export function resetOop(): void {
  classes.clear();
  selfStack.length = 0;
  defaultSelf.ivars.clear();
  cvars.clear();
  methods.clear();
}
