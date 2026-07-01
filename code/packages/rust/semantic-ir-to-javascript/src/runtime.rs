//! Inlined JavaScript runtime helpers.
//!
//! Unlike the TypeScript backend — which `import`s a published
//! `@coding-adventures/sir-runtime-core` package — the JavaScript
//! backend produces **fully self-contained** output: every emitted
//! `.js` file pastes this `__Sir` namespace verbatim at the top, so it
//! runs directly via `node file.js` with no `npm install`, no
//! `require()`, and no `import`.  (See `code/specs/SIR18-…md`, "Inlined
//! runtime".)
//!
//! ## Why an IIFE?
//!
//! `const __Sir = (() => { … })();` is the classic JavaScript *module
//! pattern*.  The arrow body is a private scope: the `Sym`/`Pair`/
//! `Closure` classes, the symbol table, and the dispatch helpers live
//! inside it and are invisible to user code.  Only the object returned
//! at the end — `{ Sym, Pair, Closure, intern, … }` — escapes, bound to
//! the single global name `__Sir`.  This mirrors the TypeScript
//! backend's `namespace __Sir { … }` exactly, minus the type
//! annotations.
//!
//! ## Value model (matches the spec's table)
//!
//! | SIR concept | JS representation                         |
//! |-------------|-------------------------------------------|
//! | `Int`/`Float` | native `number`                         |
//! | `Bool`      | native `boolean`                          |
//! | `Nil`       | `null`                                    |
//! | `Symbol`    | `__Sir.Sym` instance with interned `.name`|
//! | `Str`       | native `string`                           |
//! | `Pair`      | `__Sir.Pair` instance (`car`/`cdr`)       |
//! | `Closure`   | `__Sir.Closure` instance wrapping a `fn`  |
//!
//! ## Determinism
//!
//! The constant is fixed text — byte-identical in every artifact — so
//! two compilations of the same module produce the same output.

/// The full inlined runtime.  Always emitted verbatim, exactly once,
/// near the top of every artifact (after the banner, before the user's
/// function declarations).
///
/// Indentation is 2 spaces, every statement is terminated with a
/// semicolon, and the blob ends with a newline so the following
/// declarations start on their own line.
pub const RUNTIME: &str = r##"const __Sir = (() => {
  "use strict";
  // ── value model ────────────────────────────────────────────────
  // A symbol is an interned name; `===` on two interned symbols with
  // the same name is therefore identity-equal.
  class Sym {
    constructor(name) { this.name = name; }
  }
  // A cons pair: `(car . cdr)`.  Lisp's fundamental building block.
  class Pair {
    constructor(car, cdr) { this.car = car; this.cdr = cdr; }
  }
  // A first-class function value.  `fn` is the underlying JS function;
  // `applyClosure` invokes it.  Wrapping (rather than using a bare JS
  // function) keeps closures distinguishable from other callables.
  class Closure {
    constructor(fn) { this.fn = fn; }
  }

  // ── symbol interning ───────────────────────────────────────────
  // One table per program; `intern("x")` always returns the *same*
  // Sym object for a given name, so symbol equality is pointer equality.
  const symbolTable = new Map();
  function intern(name) {
    let s = symbolTable.get(name);
    if (s === undefined) { s = new Sym(name); symbolTable.set(name, s); }
    return s;
  }

  // ── closures ───────────────────────────────────────────────────
  function applyClosure(c, args) {
    if (!(c instanceof Closure)) {
      throw new TypeError("apply on non-closure");
    }
    return c.fn(...args);
  }

  // ── truthiness ─────────────────────────────────────────────────
  // SIR truthiness, NOT JavaScript's: only `false` and `nil` (null)
  // are falsy.  `0`, `""`, and `NaN` are all truthy — matching Lisp /
  // Ruby semantics rather than JS's surprising coercions.
  function truthy(v) {
    return v !== false && v !== null && v !== undefined;
  }

  // ── display / formatting ───────────────────────────────────────
  // `format` renders any SIR value to the string `print` writes.
  // Strings render WITHOUT surrounding quotes (so `print` of a string
  // shows its contents); everything else uses a Lisp-ish notation.
  function format(v) {
    if (v === null || v === undefined) { return "nil"; }
    if (v === true) { return "#t"; }
    if (v === false) { return "#f"; }
    if (typeof v === "string") { return v; }
    if (typeof v === "number") { return String(v); }
    if (v instanceof Sym) { return v.name; }
    if (v instanceof Pair) {
      return "(" + format(v.car) + " . " + format(v.cdr) + ")";
    }
    if (v instanceof Closure) { return "#<closure>"; }
    if (Array.isArray(v)) { return "[" + v.map(format).join(", ") + "]"; }
    return String(v);
  }

  // ── builtins dispatch table ────────────────────────────────────
  // Reached only for builtins the emitter did not specialise inline
  // (e.g. a variadic `+`, or a builtin referenced as a value via
  // `__Sir.builtins["name"]`).  Each entry is an ordinary JS function.
  function numFold(args, init, step) {
    let acc = init;
    for (const a of args) { acc = step(acc, a); }
    return acc;
  }
  const builtins = {
    "+": (...a) => a.length === 0 ? 0 : numFold(a.slice(1), a[0], (x, y) => x + y),
    "-": (...a) => a.length === 1 ? -a[0] : numFold(a.slice(1), a[0], (x, y) => x - y),
    "*": (...a) => numFold(a, 1, (x, y) => x * y),
    "/": (...a) => a.length === 1 ? 1 / a[0] : numFold(a.slice(1), a[0], (x, y) => x / y),
    "=": (x, y) => x === y,
    "<": (x, y) => x < y,
    ">": (x, y) => x > y,
    "<=": (x, y) => x <= y,
    ">=": (x, y) => x >= y,
    "not": (x) => !truthy(x),
    "neg": (x) => -x,
    "cons": (x, y) => new Pair(x, y),
    "car": (p) => p.car,
    "cdr": (p) => p.cdr,
    "pair?": (p) => p instanceof Pair,
    "null?": (x) => x === null || x === undefined,
    "number?": (x) => typeof x === "number",
    "symbol?": (x) => x instanceof Sym,
    "len": (x) => x.length,
    "print": (x) => { console.log(format(x)); return null; },
    "range": (start, stop, step) => {
      const out = [];
      const s = step === undefined || step === null ? 1 : step;
      if (s >= 0) { for (let i = start; i < stop; i += s) { out.push(i); } }
      else { for (let i = start; i > stop; i += s) { out.push(i); } }
      return out;
    },
  };
  // A builtin referenced as a value (e.g. passed to `map`) becomes a
  // Closure wrapping the table entry, so it round-trips through
  // `applyClosure` like any other first-class function.
  function builtinClosure(name) {
    const f = builtins[name];
    if (f === undefined) { throw new TypeError("unknown builtin: " + name); }
    return new Closure(f);
  }
  function callBuiltin(name, args) {
    const f = builtins[name];
    if (f === undefined) { throw new TypeError("unknown builtin: " + name); }
    return f(...args);
  }
  // `print` is promoted to a top-level member so emitted code can write
  // the readable `__Sir.print(x)` rather than `__Sir.builtins["print"](x)`.
  function print(x) { console.log(format(x)); return null; }

  // ── method dispatch (`__method__`) ─────────────────────────────
  // `recv.meth(args…)` reaches the backend as
  // `BuiltinCall("__method__", [recv, "meth", args…])`; the emitter routes
  // it here as `callMethod(recv, "meth", args…)`.  We dispatch to the
  // JS-native method on the receiver — arrays' `push`/`pop`/`map`/`filter`/
  // `forEach`/`includes`/`reduce`/…, strings' `toUpperCase`/… — so
  // frontend collection code runs end-to-end (C3/C4).
  //
  // A callback argument arrives as a `Closure` (the frontend lowers an
  // arrow / function argument to `MakeClosure`); `unwrapArg` turns it into
  // an ordinary JS function via `applyClosure`, so `arr.map(fn)` and
  // friends receive a callable the native method can invoke.  `length` is
  // accepted as a zero-arg method too (a property read spelled as a call),
  // though the frontend normally lowers bare `.length` to `SeqLen`.
  function unwrapArg(a) {
    if (a instanceof Closure) {
      // Native higher-order methods pass (element, index, array); the SIR
      // closure only binds the params it declared, so extra JS arguments
      // are harmlessly ignored by `applyClosure`'s spread.
      return (...xs) => applyClosure(a, xs);
    }
    return a;
  }
  // ── method-name allowlist (SECURITY, load-bearing) ─────────────
  // `name` here is ATTACKER-CONTROLLED: it originates as a source-level
  // method name in an untrusted input program and reaches us verbatim.
  // `recv[name]` is therefore an *unrestricted* dynamic property lookup, and
  // a handful of JavaScript member names are reflective gadgets that turn
  // that lookup into arbitrary-code execution.  The worst is `constructor`:
  // on any function it yields the `Function` constructor, so a translated
  // program can write
  //     id.constructor("return …payload…")
  // to synthesise and run evil code — and a native higher-order method
  // (Array.prototype.map/filter/…) will then invoke the result.  That is a
  // remote-code-execution hole.  `apply`/`call`/`bind` re-bind `this`, and
  // `__proto__`/`prototype`/`__define*etter__`/`__lookup*etter__`/`valueOf`/
  // `hasOwnProperty` are the other prototype-chain escape hatches.
  //
  // We therefore dispatch ONLY through this fixed allowlist of known-safe
  // collection / String / Number methods.  Anything not on the list — every
  // gadget above included, none of which appear here — is rejected with a
  // TypeError *before* any property is looked up or invoked.  This is the
  // primary gate: the emitted JS is what actually executes, so the allowlist
  // must live here (the frontend denylist is defense in depth in front of it).
  const METHOD_ALLOWLIST = new Set([
    // Array
    "push", "pop", "shift", "unshift", "slice", "splice", "concat",
    "map", "filter", "reduce", "reduceRight", "forEach", "includes",
    "indexOf", "lastIndexOf", "join", "sort", "reverse", "find",
    "findIndex", "some", "every", "flat", "flatMap", "fill", "at",
    "keys", "values", "entries",
    // String
    "toUpperCase", "toLowerCase", "trim", "trimStart", "trimEnd", "split",
    "charAt", "charCodeAt", "codePointAt", "substring", "repeat",
    "startsWith", "endsWith", "replace", "replaceAll", "padStart", "padEnd",
    // Number
    "toFixed", "toString",
  ]);
  function callMethod(recv, name, ...rawArgs) {
    const args = rawArgs.map(unwrapArg);
    // `length` as a nullary method mirrors the property.  Kept special-cased
    // ahead of the allowlist: it is a property read, not a method call.
    if (name === "length" && args.length === 0) { return recv.length; }
    // SECURITY gate: refuse any name outside the allowlist so reflective
    // gadgets (`constructor`, `__proto__`, `apply`, …) can never be reached.
    if (!METHOD_ALLOWLIST.has(name)) {
      throw new TypeError(
        "method `" + name + "` is not an allowed collection method");
    }
    const m = recv == null ? undefined : recv[name];
    if (typeof m !== "function") {
      throw new TypeError(
        "method `" + name + "` is not defined on " + format(recv));
    }
    return m.apply(recv, args);
  }

  // ── exceptions (SIR17 `Feature::Exceptions`) ───────────────────
  // Most SIR lowers to native JavaScript, and exception handling is
  // *mostly* native too: `Stmt::TryCatch` becomes a real
  // `try { … } catch (__exc) { … } finally { … }`.  Two pieces have no
  // faithful native equivalent and live here — mirroring the published
  // TypeScript `@coding-adventures/sir-runtime-exceptions` package
  // (ported to plain JS so the JavaScript backend stays self-contained,
  // no `import`/`require`):
  //
  //   1. A *class-tagged* thrown object.  Ruby's `raise ArgumentError,
  //      "boom"` names a **class** and carries a message; JavaScript's
  //      `throw` takes any value and its `Error` has no Ruby class tag.
  //      `SirError` is a real `Error` (so stack traces work) that also
  //      records the SIR class name in `sirClass`.
  //   2. Rescue-clause *type matching*.  A native `catch` binds one
  //      variable and catches everything; Ruby's ordered typed `rescue`
  //      clauses match a *set* of classes (and their subclasses) and
  //      fall through otherwise.  `rescueMatches` answers "does this
  //      caught value match this clause's class list?" so the emitted
  //      `catch` body dispatches to the right clause (or re-`throw`s).

  // Built-in Ruby exception ancestry: subclass name → immediate
  // superclass name.  Walked by `isAncestorOrSelf` so a
  // `rescue StandardError` also catches the everyday subclasses a program
  // raises.  A curated slice of Ruby's tree (the classes a frontend is
  // likely to name), each chaining up to `StandardError → Exception`.
  //
  //   Exception
  //   └─ StandardError
  //      ├─ RuntimeError ├─ ArgumentError ├─ TypeError
  //      ├─ NameError ─ NoMethodError      ├─ RangeError
  //      ├─ IndexError ─ KeyError          ├─ ZeroDivisionError
  //      ├─ IOError    ├─ StopIteration    └─ NotImplementedError
  //
  // `ancestry` starts as a copy of the built-in table; user-defined
  // classes are merged in at program init via `registerAncestry` (below).
  const BUILTIN_ANCESTRY = {
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
  // A *mutable* lookup seeded from the built-ins.  `Object.create(null)`
  // gives a prototype-less map so a user class literally named
  // `"constructor"`/`"__proto__"` cannot poison the lookup — dispatch is
  // pure DATA, never reflection.
  const ancestry = Object.assign(Object.create(null), BUILTIN_ANCESTRY);

  // Merge a user `{ childClass: superclassName }` map into `ancestry`
  // (E2, the JS half of user-defined class ancestry).  The emitter
  // collects the module's `class Child < Super` pairs and emits ONE
  // `__Sir.registerAncestry(...)` call at program init, so
  // `class MyErr < StandardError; raise MyErr; rescue StandardError`
  // matches through the merged chain.  Own-keys only (no inherited
  // prototype keys) keeps the merge to explicit source-declared pairs.
  function registerAncestry(map) {
    if (map == null) { return; }
    for (const child of Object.keys(map)) {
      ancestry[child] = map[child];
    }
  }

  // A SIR exception: a native `Error` tagged with its Ruby class name.
  // `sirClass` is what `rescueMatches` dispatches on; `message` is the
  // human string `raise Klass, "msg"` carries (defaulting to the class
  // name, matching Ruby's `exception.message`).
  class SirError extends Error {
    constructor(sirClass, message) {
      const text =
        message === undefined || message === null ? sirClass : String(message);
      super(text);
      this.sirClass = sirClass;
      this.name = sirClass;
      // Restore the prototype chain so `err instanceof SirError` holds.
      Object.setPrototypeOf(this, new.target.prototype);
    }
  }

  // Raise a SIR exception of class `className` with an optional message.
  // Emitted for `raise Foo, "msg"` → `raiseError("Foo", "msg")`,
  // `raise Foo` → `raiseError("Foo")`, and bare `raise` → `raiseError()`
  // → a generic `RuntimeError` (SIR v0 does not thread the in-flight
  // exception into a bare re-raise; documented limitation).
  function raiseError(className, message) {
    throw new SirError(className === undefined ? "RuntimeError" : className, message);
  }

  // The SIR class name of a caught value.  A `SirError` reports its tag;
  // any other thrown value (a native `Error`, a bare string, …) is
  // bucketed as `StandardError` — the everyday rescuable root — so a
  // `rescue StandardError` / bare `rescue` also catches JS runtime errors.
  function classOfThrown(err) {
    if (err instanceof SirError) { return err.sirClass; }
    return "StandardError";
  }

  // `true` if `actual` is `target` or any of its registered ancestors.
  // The `seen` guard makes a malformed (cyclic) user ancestry map
  // terminate rather than loop forever.  Lookup is by EXPLICIT table
  // (`ancestry[cur]`), never `eval`/reflection — class names are data.
  function isAncestorOrSelf(actual, target) {
    let cur = actual;
    const seen = new Set();
    while (cur !== undefined && cur !== null && !seen.has(cur)) {
      if (cur === target) { return true; }
      seen.add(cur);
      cur = ancestry[cur];
    }
    return false;
  }

  // Does a caught value match a rescue clause naming `classNames`?
  //   - empty `classNames` → a bare `rescue` (catch-all) → always true.
  //   - `Exception` → Ruby's universal root → matches anything.
  //   - otherwise: matches if the value's class equals or descends from
  //     any named class (per `ancestry`; user classes by exact name or
  //     registered chain).
  // The emitted `catch` calls this once per clause in source order,
  // running the first match's body and re-`throw`ing if none match.
  function rescueMatches(err, classNames) {
    if (classNames.length === 0) { return true; }
    const actual = classOfThrown(err);
    return classNames.some(
      (name) => name === "Exception" || isAncestorOrSelf(actual, name),
    );
  }

  return {
    Sym, Pair, Closure,
    intern, applyClosure, truthy, format, print,
    builtins, builtinClosure, callBuiltin, callMethod,
    SirError, raiseError, rescueMatches, registerAncestry,
  };
})();
"##;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_is_non_empty_and_terminates_newline() {
        assert!(!RUNTIME.is_empty());
        assert!(RUNTIME.ends_with('\n'));
    }

    #[test]
    fn runtime_defines_the_namespace_inline() {
        // Self-contained: the `__Sir` namespace is the IIFE itself, not
        // an import of an external package.
        assert!(RUNTIME.contains("const __Sir = (() => {"));
        assert!(!RUNTIME.contains("import "));
        assert!(!RUNTIME.contains("require("));
    }

    #[test]
    fn runtime_exports_the_helpers_the_emitter_calls() {
        for needed in [
            "intern", "applyClosure", "truthy", "format",
            "builtins", "builtinClosure", "callBuiltin", "callMethod",
            "class Sym", "class Pair", "class Closure",
            // Exception runtime (SIR17): the four helpers the emitter
            // references from its TryCatch / raise / ClassDef arms.
            "class SirError", "raiseError", "rescueMatches", "registerAncestry",
        ] {
            assert!(RUNTIME.contains(needed), "runtime missing `{needed}`");
        }
    }

    #[test]
    fn runtime_bakes_in_builtin_exception_ancestry() {
        // `rescue StandardError` must catch the everyday subclasses, so the
        // built-in ancestry table has to chain them up to StandardError.
        assert!(RUNTIME.contains("ArgumentError: \"StandardError\""));
        assert!(RUNTIME.contains("StandardError: \"Exception\""));
    }

    #[test]
    fn runtime_dispatches_ancestry_by_table_not_reflection() {
        // SECURITY: ancestry lookup is an explicit map read, never `eval`
        // or dynamic code synthesis.  A prototype-less map keeps a user
        // class named `constructor`/`__proto__` from poisoning the lookup.
        assert!(RUNTIME.contains("ancestry[cur]"));
        assert!(RUNTIME.contains("Object.create(null)"));
        assert!(!RUNTIME.contains("eval("));
    }
}
