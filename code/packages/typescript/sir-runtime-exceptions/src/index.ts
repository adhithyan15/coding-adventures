/**
 * Exception runtime primitives for Semantic-IR-emitted TypeScript/JavaScript.
 *
 * Most SIR constructs translate to *native* TypeScript (a sequence is an
 * `Array`, a loop is a `for`).  Exception handling translates *mostly*
 * natively — `Stmt::TryCatch` becomes a native `try { … } catch (e) { … }
 * finally { … }` — but two pieces have no faithful native equivalent and live
 * here:
 *
 *   1. **A SIR exception object.**  Ruby's `raise StandardError, "boom"` names a
 *      *class* and carries a message.  JavaScript's `throw` takes any value and
 *      its `Error` carries no Ruby class tag.  {@link SirError} is the base
 *      thrown object: a real `Error` (so stack traces work) that also records
 *      the SIR class name in `sirClass`.
 *
 *   2. **Rescue-clause type matching.**  A native `catch` binds *one* variable
 *      and catches *everything*; Ruby's `rescue TypeError, ArgumentError => e`
 *      matches a *set* of classes (and their subclasses) and falls through to
 *      the next clause otherwise.  {@link rescueMatches} answers "does this
 *      caught value match this clause's class list?" so the emitted `catch`
 *      body can dispatch to the right clause (or re-`throw` if none match).
 *
 * **Keyed to SIR, not Ruby.**  These helpers implement the SIR exception model,
 * so a future JavaScript→SIR→TS path reuses them unchanged.  See
 * `code/specs/sir-runtime.md`.
 *
 * **User-class ancestry (E2).**  The built-in table below is fixed, but SIR
 * *does* carry `class MyErr < StandardError` edges in `Stmt::ClassDef`.  The
 * backend threads them here with {@link registerAncestry} at program init, so a
 * `rescue StandardError` catches a raised `MyErr` even though `MyErr` is not in
 * the built-in table.  We keep this an **explicit string→string map** — no
 * `eval`/reflection, no walking real JS classes — because the SIR class names
 * are just tags, not live constructors.  User edges are *additive*: they extend
 * the chain up to a built-in root (`StandardError → Exception`) and never mutate
 * the built-in entries, so built-in matching is unchanged.  A user class with no
 * registered superclass still matches only by exact name (or via `Exception` / a
 * bare `rescue`), exactly as before.
 */

/** The SIR universal value type at this package's boundary. */
// eslint-disable-next-line @typescript-eslint/no-explicit-any
export type Val = any;

/**
 * Built-in Ruby exception ancestry: subclass name → immediate superclass name.
 *
 * Walked by {@link isAncestorOrSelf} so a `rescue StandardError` also catches
 * the everyday subclasses a program raises.  This is intentionally a small,
 * curated slice of Ruby's tree (the classes the frontend is likely to name),
 * not the whole standard library.  Every entry ultimately chains up to
 * `StandardError → Exception`.
 *
 * ```text
 * Exception
 * └─ StandardError
 *    ├─ RuntimeError ├─ ArgumentError ├─ TypeError
 *    ├─ NameError ─ NoMethodError      ├─ RangeError
 *    ├─ IndexError ─ KeyError          ├─ ZeroDivisionError
 *    ├─ IOError    ├─ StopIteration    └─ NotImplementedError
 * ```
 */
const BUILTIN_ANCESTRY: Readonly<Record<string, string>> = {
  RuntimeError: "StandardError",
  ArgumentError: "StandardError",
  TypeError: "StandardError",
  NameError: "StandardError",
  NoMethodError: "NameError",
  IndexError: "StandardError",
  KeyError: "IndexError",
  RangeError: "StandardError",
  ZeroDivisionError: "StandardError",
  IOError: "StandardError",
  StopIteration: "StandardError",
  NotImplementedError: "StandardError",
  StandardError: "Exception",
};

/**
 * The *live* ancestry the matcher walks.  Seeded from the built-in table and
 * then extended in place by {@link registerAncestry} with user
 * `child → superclass` edges.  We start from a spread copy so callers can never
 * mutate the frozen built-in reference.
 */
const ANCESTRY: Record<string, string> = { ...BUILTIN_ANCESTRY };

/**
 * Merge user `{childClassName: superclassName}` edges into the ancestry.
 *
 * Called once at program init with the module's `class Child < Parent` pairs,
 * *before* any `rescue` runs.  After this, {@link rescueMatches} walks a user
 * child up through its registered superclass and on into the built-in table —
 * so `rescue StandardError` catches a raised `MyErr extends StandardError`.
 *
 * The mapping is an **explicit string→string map**: keys and values are SIR
 * class-name tags, not JS constructors.  We deliberately do no reflection and
 * trust no live type — the frontend already knows the static superclass edge,
 * so threading it as data keeps the runtime free of `eval`/import magic.
 *
 * Idempotent and additive: re-registering the same edge is a no-op, and user
 * edges layer on top of the built-in table without replacing it (a chain like
 * `Grandchild → Child → StandardError → Exception` resolves by walking both
 * layers).  {@link isAncestorOrSelf} already guards against cycles, so a
 * malformed self-referential edge cannot loop forever.
 */
export function registerAncestry(mapping: Record<string, string>): void {
  for (const child of Object.keys(mapping)) {
    ANCESTRY[child] = mapping[child];
  }
}

/**
 * A SIR exception: a native `Error` tagged with its Ruby class name.
 *
 * `sirClass` is what {@link rescueMatches} dispatches on; `message` is the
 * human string Ruby's `raise Klass, "msg"` carries.  When no message is given
 * the class name itself is used (matching Ruby's default `exception.message`).
 */
export class SirError extends Error {
  /** The Ruby/SIR class name this exception was raised as. */
  readonly sirClass: string;

  constructor(sirClass: string, message?: Val) {
    const text =
      message === undefined || message === null ? sirClass : String(message);
    super(text);
    this.sirClass = sirClass;
    // `name` shows up in stack traces / `String(err)`; make it the Ruby class.
    this.name = sirClass;
    // Restore the prototype chain (TS-down-to-ES5 `extends Error` caveat) so
    // `err instanceof SirError` holds even under older targets.
    Object.setPrototypeOf(this, new.target.prototype);
  }
}

/**
 * Raise a SIR exception of class `className` with an optional `message`.
 *
 * Emitted for SIR `BuiltinCall("raise", …)`:
 *   - `raise Foo, "msg"` → `raiseError("Foo", "msg")`
 *   - `raise Foo`        → `raiseError("Foo")`
 *   - bare `raise`       → `raiseError()` → re-raises as a generic
 *     `RuntimeError` (SIR v0 does not thread the in-flight exception into a
 *     bare re-raise; documented limitation).
 *
 * Declared to return `never` so TypeScript's control-flow analysis knows code
 * after a `raise` is unreachable.
 */
export function raiseError(className?: string, message?: Val): never {
  throw new SirError(className ?? "RuntimeError", message);
}

/**
 * The SIR class name of a caught value.
 *
 * A {@link SirError} reports its tag; a plain native `Error` is treated as a
 * `StandardError` (so `rescue StandardError`/`rescue => e` catches JS runtime
 * errors too); anything else (a thrown non-error value) is also bucketed as
 * `StandardError`, the everyday rescuable root.
 */
export function classOfThrown(err: unknown): string {
  if (err instanceof SirError) return err.sirClass;
  return "StandardError";
}

/** `true` if `actual` is `target` or any of its registered ancestors. */
function isAncestorOrSelf(actual: string, target: string): boolean {
  let cur: string | undefined = actual;
  const seen = new Set<string>();
  while (cur !== undefined && !seen.has(cur)) {
    if (cur === target) return true;
    seen.add(cur);
    cur = ANCESTRY[cur];
  }
  return false;
}

/**
 * Does a caught value match a rescue clause that names `classNames`?
 *
 * - An **empty** `classNames` is a bare `rescue` (catch-all) → always `true`.
 * - `Exception` is Ruby's universal exception root → matches anything.
 * - Otherwise the value matches if its class equals, or descends from, any
 *   named class (per the built-in {@link ANCESTRY}; user classes match by exact
 *   name).
 *
 * The emitted `catch` block calls this once per rescue clause, in source order,
 * running the first matching clause's body and re-`throw`ing if none match.
 */
export function rescueMatches(err: unknown, classNames: string[]): boolean {
  if (classNames.length === 0) return true;
  const actual = classOfThrown(err);
  return classNames.some(
    (name) => name === "Exception" || isAncestorOrSelf(actual, name),
  );
}
