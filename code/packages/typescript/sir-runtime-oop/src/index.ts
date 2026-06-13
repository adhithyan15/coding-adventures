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

/**
 * Dispatch reflective method `name` on `recv`.  Handles the built-ins the SIR
 * frontend emits as `__method__` calls — `is_a?`/`kind_of?`/`instance_of?`
 * (predicate against a class) and `class` (the class name) — then falls back to
 * a `defineMethod` table, returning `null` (nil) for an unknown method rather
 * than throwing.
 *
 * The class argument to a predicate may arrive as a class-name **string** or as
 * a value whose class is taken; `instance_of?` additionally requires an exact
 * (non-ancestor) match.
 */
export function callMethod(recv: Val, name: string, ...args: Val[]): Val {
  switch (name) {
    case "is_a?":
    case "kind_of?": {
      return isA(recv, classNameArg(args[0]));
    }
    case "instance_of?": {
      return classOf(recv) === classNameArg(args[0]);
    }
    case "class":
      return classOf(recv);
    default: {
      const m = methods.get(name);
      return m ? m(recv, args) : null;
    }
  }
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
