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

import { add, div, gt, lt, mul, shiftLeft, sub, trueDiv, truncDiv } from "./arithmetic.js";
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

/**
 * Emit a single `per_value` argument under `__sys_write__` (SIR28 §2.1),
 * honouring Ruby `puts`'s per-value rules.
 *
 * Ruby `puts` is deceptively subtle. For one argument:
 * - **Array** → recurse over the *elements*, one per line, flattening
 *   nesting (`puts [1, [2, 3]]` → `1\n2\n3\n`). An **empty** array writes
 *   nothing here (the top-level `puts []` still emits one newline — see
 *   {@link write}).
 * - **anything else** → its display string then a newline.
 *
 * **Cycle safety.** An array is a shared, mutable reference, so a program can
 * build a *cyclic* array (`a = []; a << a`). The element-per-line flatten
 * recurses through nested arrays, so it MUST be cycle-guarded or a self-
 * referential array throws `RangeError: Maximum call stack size exceeded` (a
 * DoS: CWE-674, uncontrolled recursion). `seen` is a `Set` of the array
 * references currently on the active flatten path. An array ALREADY on the
 * path is a cycle: rather than recurse forever we write `[...]` then a newline
 * — matching real Ruby, where `puts a` on a self-referential array prints
 * `[...]` and terminates. An array removed from `seen` on exit still flattens
 * in full via a sibling path — only a true self-cycle is short-circuited, so
 * non-cyclic output is unchanged (`puts [1, [2, 3]]` still prints `1\n2\n3\n`).
 */
function writeOne(
  out: NodeJS.WriteStream,
  v: Val,
  unpackArrays: boolean,
  seen: Set<unknown>,
): void {
  if (unpackArrays && Array.isArray(v)) {
    if (seen.has(v)) {
      out.write("[...]\n");
      return;
    }
    seen.add(v);
    for (const item of v) {
      writeOne(out, item, unpackArrays, seen);
    }
    seen.delete(v);
    return;
  }
  out.write(toDisplay(v) + "\n");
}

/**
 * `__sys_write__` (SIR28 §2.1): the general console-output primitive every
 * frontend lowers `print`/`puts`/`console.log`/etc. to.
 *
 * It generalizes what used to be several backend-hardcoded newline policies
 * into ONE operation parameterized by policy flags carried as DATA — the
 * root cause SIR28 exists to fix: real Ruby's `print` never
 * newline-terminates, TypeScript's own `console.log` always does, but
 * before SIR28 both lowered to the identical `BuiltinCall("print", ...)`
 * this backend had no way to tell apart.
 *
 * `terminator`: `"none"` (write each value back to back, no newline —
 * matches Ruby's `print`) | `"per_value"` (one newline per value,
 * honouring `unpackArrays` — matches Ruby's `puts`) | `"once"`
 * (TypeScript's native `console.log(a, b)` — space-join every value, one
 * trailing newline). Deliberately does NOT replicate Ruby `puts`'s
 * trailing-newline-suppression nuance (`puts "x\n"` prints `x\n`, not
 * `x\n\n`) — that's a pre-existing, orthogonal divergence between
 * backends' own historical `puts` implementations that SIR28 does not fix
 * or replicate; `"per_value"` here always appends exactly one newline per
 * value, matching SIR28 §2.1's table and every other backend's
 * `__sys_write__` faithfully.
 */
export function write(
  stream: string,
  terminator: string,
  unpackArrays: boolean,
  ...values: Val[]
): null {
  const out = stream === "stderr" ? process.stderr : process.stdout;
  if (terminator === "per_value") {
    if (values.length === 0) {
      out.write("\n");
      return null;
    }
    const seen = new Set<unknown>();
    for (const v of values) {
      writeOne(out, v, unpackArrays, seen);
    }
    return null;
  }
  if (terminator === "once") {
    out.write(values.map((v) => toDisplay(v)).join(" ") + "\n");
    return null;
  }
  // "none"
  for (const v of values) {
    out.write(toDisplay(v));
  }
  return null;
}

// --- Builtin dispatch ------------------------------------------------------

// Built with a null prototype (matching the JS sibling backend's own
// `Object.assign(Object.create(null), {...})` table) — `callBuiltin`/
// `builtinClosure` index this by a SIR-NAME string, and a plain object
// literal would resolve an inherited `Object.prototype` member
// (`constructor`/`toString`/`hasOwnProperty`/`__defineGetter__`/…) for a
// lookup miss instead of the intended `undefined`, letting it slip past
// the `fn === undefined` guard and get INVOKED — a define-a-getter-on-
// global-style gadget (the [[dynamic-dispatch-rce]] hazard this repo's
// specs name explicitly). No call site here passes a non-literal name
// today, but both functions are public API of a published package.
const builtins: Record<string, (...args: Val[]) => Val> = Object.assign(Object.create(null), {
  "+": add,
  "<<": shiftLeft,
  "-": sub,
  "*": mul,
  "/": div,
  // SIR21 T3b-2: `div_floor` is a bare alias for `div` (a documented,
  // pre-existing limitation — see `div`'s own doc comment in
  // `arithmetic.ts` for why it is NOT fixed here). `div_trunc`/
  // `udiv_trunc` both route to `truncDiv` (identical in this runtime's
  // untagged numeric model — see `truncDiv`'s own doc comment).
  // `div_true` is genuinely new.
  div_floor: div,
  div_trunc: truncDiv,
  udiv_trunc: truncDiv,
  div_true: trueDiv,
  "=": (a: Val, b: Val) => eq(a!, b!),
  "<": (a: Val, b: Val) => lt(a!, b!),
  ">": (a: Val, b: Val) => gt(a!, b!),
  cons: (a: Val, b: Val) => cons(a!, b!),
  car: (p: Val) => car(p!),
  cdr: (p: Val) => cdr(p!),
  "null?": (v: Val) => isNull(v!),
  "pair?": (v: Val) => isPair(v!),
  "number?": (v: Val) => isNumber(v!),
  "symbol?": (v: Val) => isSymbol(v!),
  __sys_write__: (...args: Val[]) =>
    write(args[0] as string, args[1] as string, args[2] as boolean, ...args.slice(3)),
});

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
