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

/**
 * Emit a single `puts` argument, honouring Ruby's per-value rules.
 *
 * Ruby `puts` is deceptively subtle. For one argument:
 * - **Array** → recurse over the *elements*, one per line, flattening
 *   nesting (`puts [1, [2, 3]]` → `1\n2\n3\n`). An **empty** array writes
 *   nothing here (the top-level `puts []` still emits one newline — see
 *   {@link puts}).
 * - **nil** → a blank line (`nil.to_s` is `""`, then the newline).
 * - **anything else** → its display string then a newline, *unless* the
 *   string already ends in `"\n"`, in which case Ruby does not add a second
 *   (`puts "x\n"` → `x\n`, not `x\n\n`).
 *
 * We write via `process.stdout.write` rather than `console.log` because
 * `console.log` unconditionally appends its own newline, which would defeat
 * the trailing-newline suppression rule.
 */
function putsOne(v: Val): void {
  if (Array.isArray(v)) {
    for (const item of v) {
      putsOne(item);
    }
    return;
  }
  if (v === null) {
    process.stdout.write("\n");
    return;
  }
  const text = toDisplay(v);
  // Suppress the added newline when the rendered text already ends in one,
  // so `puts "x\n"` and `puts "x"` produce identical output.
  process.stdout.write(text.endsWith("\n") ? text : text + "\n");
}

/**
 * Ruby `puts`: write each argument on its own line (see {@link putsOne}).
 *
 * - `puts()` (no args) → a single newline.
 * - `puts(x)` → `x` on its own line (arrays flattened element-per-line; a
 *   value already ending in `"\n"` is not double-spaced; `nil` is a blank
 *   line).
 * - `puts(a, b)` → each argument handled independently, in order.
 * - `puts([])` → a single newline: Ruby prints a blank line when an argument
 *   flattens to nothing, which is why the no-arg and empty-array cases
 *   converge on one newline.
 */
export function puts(...args: Val[]): null {
  if (args.length === 0) {
    process.stdout.write("\n");
    return null;
  }
  for (const a of args) {
    if (Array.isArray(a) && a.length === 0) {
      // Empty array as an argument still writes one newline.
      process.stdout.write("\n");
    } else {
      putsOne(a);
    }
  }
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
  puts: (...args) => puts(...args),
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

// --- Keyword-argument splat (call-position `**h`) ---------------------------

/**
 * Merge double-splatted maps into a single keyword-argument map.
 *
 * Ruby `f(**h1, **h2)` splices each map's entries into the call's keyword
 * arguments, later entries winning on key collision. Python has a native
 * `**` for exactly this; **JavaScript has no keyword-argument call form**, so
 * the TypeScript backend cannot emit a faithful native `**`. Instead it
 * collapses the trailing run of `**` arguments at a call site into ONE
 * trailing argument built by this helper — the conventional JS "options
 * object" convention, except the option bag is a SIR `Map<Val, Val>` so any
 * `Val` key (symbol, number, string, …) round-trips.
 *
 * Semantics (matching Ruby's `**` merge):
 *   - returns a **fresh** `Map` (callers may mutate it without aliasing a
 *     source map — a defensive copy is what Ruby's `**` produces);
 *   - maps are merged left-to-right, so a key present in a later map
 *     **overwrites** the earlier value;
 *   - every argument must be a `Map` (a SIR map); anything else is a backend
 *     bug (the emitter only routes `**` over map operands), so we throw a
 *     clear error rather than silently coercing.
 *
 * v0 cut-line (see `code/specs/sir-runtime.md`): this models keyword args as a
 * single trailing options map. A callee compiled from `def f(**opts)` receives
 * that map as its last positional parameter; mixing inline `key: value` pairs
 * with `**h` at one call site is a further documented cut-line.
 */
export function doubleSplatMerge(...maps: Val[]): Map<Val, Val> {
  const merged = new Map<Val, Val>();
  for (const m of maps) {
    if (!(m instanceof Map)) {
      throw new Error(
        `double-splat (\`**\`) expects a map operand, got ${toDisplay(m)}. ` +
          `The TypeScript backend only routes \`**\` over SIR maps; reaching ` +
          `this with a non-map is a backend coverage gap.`,
      );
    }
    for (const [k, v] of m) {
      merged.set(k, v);
    }
  }
  return merged;
}
