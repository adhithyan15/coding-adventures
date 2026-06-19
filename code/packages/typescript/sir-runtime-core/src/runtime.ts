/**
 * Closures, the global store, printing, and the builtin dispatch table.
 *
 * - **Closures** carry their captured values explicitly (the frontend
 *   computed them). `makeClosure` binds captures ahead of the call-time
 *   arguments; `apply` invokes a handle. A uniform `Closure` lets an
 *   `IndirectCall` invoke any target the same way.
 * - **Globals** — a process-global name→value store backing SIR `Globals`.
 * - **Dispatch** — `builtinClosure` wraps a builtin used as a first-class
 *   value; `callBuiltin` looks one up by SIR name.
 */

import { add, div, gt, lt, mul, sub } from "./arithmetic.js";
import { car, cdr, cons, isPair } from "./pairs.js";
import { Sym } from "./symbols.js";
import { eq, isNull, isNumber, isSymbol, toDisplay } from "./values.js";
import type { ClosureLike, Val } from "./values.js";

// --- Closures --------------------------------------------------------------

/**
 * Raised when a closure-shaped call has no closure to invoke.
 *
 * SIR's explicit-block-param ABI threads a method's block as an ordinary
 * trailing parameter and lowers `yield` to an `IndirectCall` through it.
 * When the caller passed **no** block, that parameter is `nil` (`null`), so
 * the `IndirectCall` reaches {@link apply} with a `null` target. Ruby raises
 * `LocalJumpError` ("no block given (yield)") in exactly this situation; we
 * mirror that with a dedicated error so the failure is recognisable rather
 * than a generic `TypeError` about an internal "non-closure". The exact Ruby
 * class identity is not modelled — this is the SIR analogue, keyed to the
 * *shape* of the error, not Ruby's class hierarchy.
 */
export class LocalJumpError extends Error {
  constructor(message = "no block given (yield)") {
    super(message);
    this.name = "LocalJumpError";
  }
}

/** A callable handle wrapping a function. */
export class Closure implements ClosureLike {
  readonly __sirClosure = true as const;
  constructor(public readonly fn: (...args: Val[]) => Val) {}
}

/**
 * Invoke a closure handle with `args`.
 *
 * A `null`/`undefined` target is the no-block-given case (see
 * {@link LocalJumpError}): a `yield` reached through a nil block parameter.
 * It is reported distinctly from other non-closures (a genuine type error)
 * so the two failures don't read alike.
 */
export function apply(c: Val, args: Val[]): Val {
  if (c === null || c === undefined) {
    throw new LocalJumpError();
  }
  if (!(c instanceof Closure)) {
    throw new TypeError("apply on non-closure");
  }
  return c.fn(...args);
}

/** Build a closure that prepends captured values to each call's arguments. */
export function makeClosure(fn: (...args: Val[]) => Val, captures: Val[]): Closure {
  return new Closure((...args: Val[]) => fn(...captures, ...args));
}

// --- Global store ----------------------------------------------------------

const globals = new Map<string, Val>();

function keyOf(name: Val): string {
  return name instanceof Sym ? name.name : String(name);
}

/** Store `value` under `name` (a string or symbol). */
export function globalSet(name: Val, value: Val): Val {
  globals.set(keyOf(name), value);
  return value;
}

/** Fetch a global by `name` (string or symbol). Throws if undefined. */
export function globalGet(name: Val): Val {
  const key = keyOf(name);
  if (!globals.has(key)) {
    throw new Error(`undefined global: ${key}`);
  }
  return globals.get(key)!;
}

/** Fetch a global by a statically-known string name. */
export function globalGetStatic(name: string): Val {
  if (!globals.has(name)) {
    throw new Error(`undefined global: ${name}`);
  }
  return globals.get(name)!;
}

// --- Printing --------------------------------------------------------------

/** Print the SIR display form of `v` followed by a newline. */
export function print(v: Val): null {
  // eslint-disable-next-line no-console
  console.log(toDisplay(v));
  return null;
}

// --- Builtin dispatch ------------------------------------------------------

const builtins: Record<string, (...args: Val[]) => Val> = {
  "+": add,
  "-": sub,
  "*": mul,
  "/": div,
  "=": (a, b) => eq(a!, b!),
  "<": (a, b) => lt(a!, b!),
  ">": (a, b) => gt(a!, b!),
  cons: (a, b) => cons(a!, b!),
  car: (p) => car(p!),
  cdr: (p) => cdr(p!),
  "null?": (v) => isNull(v!),
  "pair?": (v) => isPair(v!),
  "number?": (v) => isNumber(v!),
  "symbol?": (v) => isSymbol(v!),
  print: (v) => print(v!),
};

/** Invoke a builtin by SIR name with a list of arguments. */
export function callBuiltin(name: string, args: Val[]): Val {
  const fn = builtins[name];
  if (fn === undefined) {
    // The SIR backends translate most builtins to native code or a dedicated
    // per-concern runtime package, so this generic dispatch only fires for the
    // small set core registers. An unregistered name means the backend emitted
    // a `callBuiltin("<name>", …)` for a builtin it does not yet lower — a
    // backend coverage gap, not a user error — so name it and point there.
    const known = Object.keys(builtins).sort().join(", ");
    throw new Error(
      `SIR builtin "${name}" is not implemented in sir-runtime-core's dispatch ` +
        `table (known: ${known}). The backend emitted a callBuiltin for a ` +
        `builtin it does not lower natively or via a per-concern runtime ` +
        `package; this is a backend coverage gap.`,
    );
  }
  return fn(...args);
}

/** Wrap a builtin as a first-class {@link Closure}. */
export function builtinClosure(name: string): Closure {
  return new Closure((...args: Val[]) => callBuiltin(name, args));
}
