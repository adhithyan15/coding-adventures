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
import { apply, Closure, eq, intern, isSymbol, truthy } from "@coding-adventures/sir-runtime-core";
import { raiseError } from "@coding-adventures/sir-runtime-exceptions";

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

// ── User class/instance method tables (O1) ───────────────────────────────────
//
// The Ruby→SIR frontend HOISTS every method to a detached top-level function,
// so nothing in the IR records that `speak` belongs to `Dog`.  We recover that
// association at *runtime* with two explicit tables, populated by emitted
// `__def_method__` / `__def_class_method__` registrations:
//
//     instanceMethods.get("Dog\x00speak") -> Closure   // def speak
//     classMethods.get("Counter\x00zero") -> Closure   // def self.zero
//
// **Model.**  A method value is a sir-runtime-core `Closure` (the hoisted
// top-level function captured by `MakeClosure`); it is invoked with `apply` —
// never property lookup / `eval` / reflection on the source-derived name (the
// C3 RCE lesson).  Dispatch is *always* an explicit `Map` lookup on the
// `(class, method)` key, walking the registered ancestry chain.
//
// **Self / receiver.**  Instance-method dispatch and `callNew` push the
// receiver onto the process-global self-stack (`pushSelf`) before invoking the
// body and pop after, so `@ivar` access reads the right object with no explicit
// `self` parameter.  `callSuper` runs in the *same* receiver (no push/pop) —
// `super` is a re-dispatch on the current self.  This mirrors the Python
// runtime's single-threaded v0 model; true per-object/per-thread binding is out
// of v0 scope.
//
// **Key encoding.**  Python keys the tables on a `(class, method)` tuple (dicts
// key tuples by value); a JS `Map` keys arrays by *identity*, so the mirror
// joins the pair with a NUL separator (`"class\x00method"`) — class and method
// names cannot contain a NUL, so the join is unambiguous.  Still a pure value
// lookup, never reflection.

const METHOD_KEY_SEP = "\x00";

function methodKey(className: string, methodName: string): string {
  return className + METHOD_KEY_SEP + methodName;
}

const instanceMethods = new Map<string, Closure>();
const classMethods = new Map<string, Closure>();

// ── Mixins: per-owner included-module list (MX3) ─────────────────────────────
//
// Ruby's `include M` weaves module `M` into an owner's ancestry *between the
// owner and its superclass*.  Because a module registers its `def`s exactly
// like a class (via `__def_method__` keyed by the MODULE name), the only new
// state the method-resolution walk needs is **which modules each owner
// includes, in include order**.  We keep that as an explicit list per owner —
// no reflection, no scanning the whole method table (the C3 RCE lesson: every
// dispatch decision is a plain `Map`/array lookup on a source-derived name we
// merely *store*, never *interpret*).
//
//     includedModules.get("Greeter") -> ["Loud", "Polite"]   // include order
//
// Ruby searches the *most recently included* module first, so the resolution
// walk iterates this list in **reverse** (`Polite` before `Loud`).  A diamond
// (a module reachable by two paths) is de-duplicated by the walk's `seen` set,
// so it is searched once — at its earliest (deepest-first) position.
const includedModules = new Map<string, string[]>();

/**
 * Register instance method `methodName` for `className` (`def m`).  Emitted by
 * the frontend as `__def_method__`; `fn` is the hoisted top-level function as a
 * `Closure`.  A *module* body's `def` registers here too — the frontend keys it
 * on the module name, so `owner` is simply a module rather than a class.
 */
export function defMethod(className: string, methodName: string, fn: Closure): void {
  instanceMethods.set(methodKey(className, methodName), fn);
}

/**
 * Record that `owner` `include`s module `moduleName` (`include M` in a
 * class/module body → `__include__("Owner", "M")`).  Appended in include order;
 * a repeated include of the same module is a no-op (Ruby re-orders rather than
 * duplicates, but the resolution walk's `seen` set already de-duplicates, so
 * keeping the first position is faithful for the mechanisms this cascade
 * covers).  The module's own `def`s must already be registered (the frontend
 * emits the module's `__def_method__`s before any `__include__` referencing it).
 */
export function includeModule(owner: string, moduleName: string): void {
  const list = includedModules.get(owner);
  if (list === undefined) {
    includedModules.set(owner, [moduleName]);
  } else if (!list.includes(moduleName)) {
    list.push(moduleName);
  }
}

/**
 * Mix module `moduleName`'s instance methods into `owner` as **class methods**
 * (`extend M` → `__extend__("Owner", "M")`).  Ruby's `extend` makes the
 * module's methods callable on the class/singleton itself (`Owner.method`), so
 * each `def` the module registered as an *instance* method is copied into
 * `owner`'s class-method table.  We copy the closures explicitly (a plain
 * table-to-table transfer, never reflection) at extend time; a module method
 * added *after* the `extend` is not retroactively picked up (v0 floor — real
 * programs `extend` after defining the module, which the frontend guarantees by
 * emitting the module's `__def_method__`s first).
 */
export function extendModule(owner: string, moduleName: string): void {
  const prefix = moduleName + METHOD_KEY_SEP;
  for (const [k, fn] of instanceMethods) {
    if (k.startsWith(prefix)) {
      const methodName = k.slice(prefix.length);
      classMethods.set(methodKey(owner, methodName), fn);
    }
  }
}

/**
 * Register class method `methodName` for `className` (`def self.m`).  Emitted by
 * the frontend as `__def_class_method__`.
 */
export function defClassMethod(className: string, methodName: string, fn: Closure): void {
  classMethods.set(methodKey(className, methodName), fn);
}

/**
 * Find `methodName` on `className` or any registered ancestor, walking Ruby's
 * **method resolution order** (MRO) — the mixin-aware generalisation of the
 * plain superclass chain (MX3):
 *
 *     class C  →  C's included modules (most-recent-first)  →
 *     C's superclass  →  its included modules  →  …  →  Object
 *
 * At each class in the ancestry we check the class's own table first (so a
 * method the **class defines itself shadows a module's** — class-first MRO),
 * then its included modules in **reverse** include order (Ruby searches the
 * last-included module first).  A module in turn can itself `include` further
 * modules, so the walk recurses into a module's own `includedModules` before
 * moving on — this is the depth-first, most-recent-first linearisation.
 *
 * **Cycle / diamond guard.**  A single `seen` set spans the *entire* walk
 * (classes and modules alike), so a module reachable by two paths (a diamond)
 * is searched exactly **once**, at its earliest (deepest-first) position, and a
 * module that (transitively) includes itself terminates rather than looping —
 * reusing the exception-ancestry `seen`-set discipline.
 *
 * Returns the first matching `Closure`, or `null` if unresolved.
 */
function resolveInstanceMethod(className: string, methodName: string): Closure | null {
  const seen = new Set<string>();

  // Search `owner`'s own table, then (depth-first, most-recent-first) the
  // modules it includes.  Returns the resolved closure or `null` for this
  // subtree; the shared `seen` set makes the whole search diamond-safe.
  function searchOwnerAndModules(owner: string): Closure | null {
    if (seen.has(owner)) return null;
    seen.add(owner);
    const own = instanceMethods.get(methodKey(owner, methodName));
    if (own !== undefined) return own;
    const mods = includedModules.get(owner);
    if (mods !== undefined) {
      // Reverse include order: the most recently included module wins.
      for (let i = mods.length - 1; i >= 0; i--) {
        const hit = searchOwnerAndModules(mods[i]!);
        if (hit !== null) return hit;
      }
    }
    return null;
  }

  // Ascend the superclass chain; at each class search it + its modules before
  // moving up.  `seen` also guards a cyclic superclass registration.
  let cur: string | null = className;
  const seenClasses = new Set<string>();
  while (cur !== null && !seenClasses.has(cur)) {
    seenClasses.add(cur);
    const hit = searchOwnerAndModules(cur);
    if (hit !== null) return hit;
    cur = superclassOf(cur);
  }
  return null;
}

/** Find class method `methodName` on `className` or an ancestor (cycle-guarded
 * ancestry walk); `null` if unresolved. */
function resolveClassMethod(className: string, methodName: string): Closure | null {
  let cur: string | null = className;
  const seen = new Set<string>();
  while (cur !== null && !seen.has(cur)) {
    const fn = classMethods.get(methodKey(cur, methodName));
    if (fn !== undefined) return fn;
    seen.add(cur);
    cur = superclassOf(cur);
  }
  return null;
}

/**
 * Allocate a `className` instance and run its `initialize` (`Foo.new`).
 *
 * Allocates via `newInstance`, pushes the new object as the current self, and —
 * if an `initialize` is registered for `className` or any ancestor — invokes it
 * with `args` (so `@ivar` assignments in the constructor land on the new
 * object).  Always pops self and returns the object, even with no `initialize`.
 */
export function callNew(className: string, ...args: Val[]): SirInstance {
  const obj = newInstance(className);
  pushSelf(obj);
  try {
    const initializer = resolveInstanceMethod(className, "initialize");
    if (initializer !== null) apply(initializer, args);
  } finally {
    popSelf();
  }
  return obj;
}

/**
 * Dispatch `super` — re-run `methodName` from `className`'s parent.
 *
 * Walks from `superclassOf(className)` upward and invokes the first ancestor
 * implementation of `methodName` with `args`, keeping the *current* self bound
 * (`super` runs in the same receiver, so no push/pop).  Returns `null` (Ruby
 * `nil`) if no ancestor defines the method — the honest floor, consistent with
 * `callMethod`.
 */
export function callSuper(methodName: string, className: string, ...args: Val[]): Val {
  const parent = superclassOf(className);
  if (parent === null) return null;
  const fn = resolveInstanceMethod(parent, methodName);
  if (fn === null) return null;
  return apply(fn, args);
}

/**
 * Dispatch a class method (`Foo.bar` for `def self.bar`).  Looks up `methodName`
 * in the class-method table walking `className`'s ancestry and applies it;
 * returns `null` if unresolved.  (`Foo.new` is the implicit class method but
 * routes to `callNew`, not here.)
 */
export function callClassMethod(className: string, methodName: string, ...args: Val[]): Val {
  const fn = resolveClassMethod(className, methodName);
  if (fn === null) return null;
  return apply(fn, args);
}

/**
 * The current receiver (top of the self-stack), or `null` if empty.  Backs the
 * `__self__` builtin — a bare `self` in a method body.  Returns `null` (Ruby
 * `nil`) at top level where no receiver is bound rather than the internal
 * default-self sentinel.
 */
export function currentSelfVal(): Val {
  return selfStack.length > 0 ? selfStack[selfStack.length - 1]! : null;
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

// `to_s`/`inspect` render Ruby display forms (see `rubyToS`/`rubyInspect`); they
// live here so `null`/`true`/`false` need no catalog of their own
// (`nil.to_s == ""`, `true.to_s == "true"`, `nil.inspect == "nil"`), with
// `nil.to_a == []` handled below.
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
  "to_s",
  "inspect",
  // Kernel flow-control methods (M6).  `send`/`__send__` re-enter dispatch with
  // a dynamic method name; `tap` and `then`/`yield_self` are the block-taking
  // pair (handled in `objectBlockMethod`), but they are listed here so
  // `respond_to?` reports them on *every* receiver — block-less and
  // block-bearing calls alike resolve.
  "send",
  "__send__",
  "public_send",
  "tap",
  "then",
  "yield_self",
]);

// Block-taking universal methods (M6): `tap` yields the receiver and returns it;
// `then`/`yield_self` yield the receiver and return the block's result.
// Dispatched in `objectBlockMethod` only when a trailing `Closure` is present
// (block-less `tap`/`then` fall through to the receiver-identity floor).
const OBJECT_BLOCK_METHODS = new Set<string>(["tap", "then", "yield_self"]);

// `Symbol`-routing methods (M6): the receiver's *first* argument names the
// method to dispatch. Listed for `respond_to?` honesty; split out in
// `callMethod` because they recurse through dispatch with a dynamic name.
const SEND_METHODS = new Set<string>(["send", "__send__", "public_send"]);

// `TrueClass`/`FalseClass` boolean logic (M6).  Ruby's `&` and `|` on a boolean
// are *non-short-circuiting* logical operators (`true & nil == false`,
// `false | 1 == true`), distinct from the lazy `&&`/`||` keywords; `^` is XOR.
// These resolve on a `boolean` receiver *before* the universal `Object` table.
const BOOL_METHODS = new Set<string>(["&", "|", "^"]);

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
  "join",
  "fetch",
  "take",
  "drop",
  "values_at",
  "rotate",
  "zip",
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
  "collect_concat",
  "any?",
  "all?",
  "none?",
  "sort_by",
  "min_by",
  "max_by",
  "group_by",
  "partition",
  "take_while",
  "drop_while",
  "count",
  "each_with_object",
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
  "ljust",
  "rjust",
  "center",
  "swapcase",
  "tr",
  "count",
  "delete",
  "squeeze",
]);

// Block-taking `String` methods (M1c); `each_char` yields one character.
const STRING_BLOCK_METHODS = new Set<string>(["each_char"]);

// Non-block `Integer`/`Float` methods (M1c).  Both are JS `number`; `boolean` is
// a *separate* `typeof`, so `callMethod` routes `true`/`false` to the universal
// `Object` methods (`true.to_s == "true"`) and never into this catalog.
const NUMERIC_METHODS = new Set<string>([
  "abs",
  "to_i",
  "to_f",
  "even?",
  "odd?",
  "zero?",
  "positive?",
  "negative?",
  "succ",
  "next",
  "pred",
  "floor",
  "ceil",
  "round",
  "divmod",
  "fdiv",
  "clamp",
  "between?",
  "gcd",
  "pow",
  "**",
  "digits",
]);

// Block-taking `Integer` methods (M1c): each invokes the block N times.
const NUMERIC_BLOCK_METHODS = new Set<string>(["times", "upto", "downto", "step"]);

// `Symbol` methods (M1c). A Ruby `Symbol` is a sir-runtime-core `Sym`;
// `upcase`/`downcase` return a *new* interned symbol.
const SYMBOL_METHODS = new Set<string>([
  "to_s",
  "to_sym",
  "length",
  "size",
  "upcase",
  "downcase",
  "inspect",
  "empty?",
]);

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
  if (isSymbol(recv) && SYMBOL_METHODS.has(name)) return true;
  // `boolean` is a distinct typeof — bools resolve the boolean operators (&/|/^)
  // plus the Object methods checked above.
  if (typeof recv === "boolean" && BOOL_METHODS.has(name)) return true;
  if (
    typeof recv === "number" &&
    (NUMERIC_METHODS.has(name) || NUMERIC_BLOCK_METHODS.has(name))
  ) {
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
    case "to_s":
      return rubyToS(recv);
    case "inspect":
      return rubyInspect(recv);
    case "tap":
      // Block-less `tap` (no `Closure` reached `objectBlockMethod`) still returns
      // the receiver — Ruby returns an Enumerator-less self in v0.
      return recv;
    case "then":
    case "yield_self":
      // Block-less `then`/`yield_self` returns the receiver (Ruby returns an
      // Enumerator; v0 floor — see the spec's "Out of scope" note).
      return recv;
    default:
      return MISS;
  }
}

/** Block-taking universal methods (Kernel).  `block` is applied via `apply` with
 * the receiver as its single argument.  `tap` yields the receiver and returns
 * **it**; `then`/`yield_self` yields the receiver and returns the **block's
 * result**.  Returns `MISS` if `name` is not a universal block method. */
function objectBlockMethod(recv: Val, name: string, block: Closure): Val | typeof MISS {
  switch (name) {
    case "tap":
      apply(block, [recv]);
      return recv;
    case "then":
    case "yield_self":
      return apply(block, [recv]);
    default:
      return MISS;
  }
}

/** `TrueClass`/`FalseClass` logical operators (`&`, `|`, `^`).  Ruby's *eager*
 * boolean operators (no short-circuit), coercing the argument by Ruby truthiness
 * (`null`/`false` falsy, everything else — `0`, `""` — truthy): `true & null`
 * is `false`, `false | 0` is `true`.  Returns `MISS` if `name` is not a boolean
 * operator. */
function boolMethod(recv: boolean, name: string, args: Val[]): Val | typeof MISS {
  // Not an operator (or called with no operand, e.g. `true.to_s`) — defer to the
  // universal `Object` table rather than coercing an absent argument.
  if (!BOOL_METHODS.has(name) || args.length === 0) return MISS;
  const other = truthy(args[0]);
  switch (name) {
    case "&":
      return recv && other;
    case "|":
      return recv || other;
    default: // "^"
      return recv !== other;
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
    case "join": {
      // Ruby `Array#join`: elements rendered with `to_s` (default sep "").
      const sep = args.length > 0 ? args[0] : "";
      return recv.map((item: Val) => rubyToS(item)).join(sep);
    }
    case "fetch": {
      // Ruby `Array#fetch(i)` — unlike `arr[i]` (which returns nil OOB), `fetch`
      // raises `IndexError` when the index is out of bounds AND no default was
      // supplied (T2). A negative index counts from the end (`fetch(-1)` is the
      // last element). With a second argument, that default is returned instead
      // of raising (Ruby's `fetch(i, default)`); a block form is out of scope.
      const raw = args[0] as number;
      const idx = raw < 0 ? recv.length + raw : raw;
      if (idx >= 0 && idx < recv.length) {
        return recv[idx];
      }
      if (args.length > 1) {
        return args[1]; // explicit default — no raise (Ruby semantics)
      }
      raiseError(
        "IndexError",
        `index ${raw} outside of array bounds: ${-recv.length}...${recv.length}`,
      );
      return MISS; // unreachable: raiseError returns `never`
    }
    case "take":
    case "drop": {
      // Ruby `Array#take(n)` / `#drop(n)`: the first `n` elements, or all
      // elements *after* the first `n`.  `n` is clamped to `[0, len]` — Ruby
      // raises `ArgumentError` on a negative `n`, but the never-raise floor
      // (mirroring the Go/Rust/Python runtimes) folds a negative count to 0,
      // and `slice` already saturates `n > len`.  A non-numeric argument
      // degrades to 0 rather than raising.
      let n = typeof args[0] === "number" ? Math.trunc(args[0] as number) : 0;
      if (n < 0) n = 0;
      if (n > recv.length) n = recv.length;
      return name === "take" ? recv.slice(0, n) : recv.slice(n);
    }
    case "values_at": {
      // Ruby `Array#values_at(*idxs)`: one element per index, with a negative
      // index folded from the end **once**.  An out-of-range index yields `nil`
      // (`null`) rather than raising — matching the sibling backends.
      const out: Val[] = [];
      for (const arg of args) {
        let idx = typeof arg === "number" ? Math.trunc(arg) : 0;
        if (idx < 0) idx += recv.length;
        out.push(idx >= 0 && idx < recv.length ? recv[idx] : null);
      }
      return out;
    }
    case "rotate": {
      // Ruby `Array#rotate(n=1)`: rotate left by `n` (a negative `n` rotates
      // right).  The modulo wraps so any magnitude terminates; an empty array is
      // `[]`.  No arg defaults to 1; a non-numeric arg degrades to 0 — matching
      // the Go/Rust runtimes.
      const length = recv.length;
      if (length === 0) return [];
      let n = args.length === 0
        ? 1
        : typeof args[0] === "number" ? Math.trunc(args[0] as number) : 0;
      // JS `%` keeps the sign of the dividend, so re-add `length` to fold right
      // rotations (negative `n`) back into `[0, length)`.
      const shift = ((n % length) + length) % length;
      return recv.slice(shift).concat(recv.slice(0, shift));
    }
    case "zip": {
      // Ruby `Array#zip(*others)`: an Array of tuples `[self[i], others..[i]]` of
      // length `recv.length`.  A shorter operand pads with `nil` (`null`); a
      // non-array operand is treated as empty (pad-only), never raising.
      const others: Val[][] = args.map((o) => (Array.isArray(o) ? o : []));
      const zipped: Val[] = [];
      for (let i = 0; i < recv.length; i++) {
        const row: Val[] = [recv[i]];
        for (const o of others) {
          row.push(i < o.length ? o[i] : null);
        }
        zipped.push(row);
      }
      return zipped;
    }
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
    case "flat_map":
    case "collect_concat": {
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
    case "sort_by": {
      // Sort by the block-computed key (Ruby `sort_by`); `<`/`>` keeps numbers
      // numeric, matching the plain `sort` arm. Keys computed once (Schwartzian).
      const keyed = recv.map((item: Val): [Val, Val] => [apply(block, [item]), item]);
      keyed.sort((a, b) => (a[0] < b[0] ? -1 : a[0] > b[0] ? 1 : 0));
      return keyed.map((pair) => pair[1]);
    }
    case "min_by":
    case "max_by": {
      if (recv.length === 0) return null;
      const wantMin = name === "min_by";
      let bestItem = recv[0];
      let bestKey = apply(block, [recv[0]]);
      for (let i = 1; i < recv.length; i++) {
        const key = apply(block, [recv[i]]);
        if (wantMin ? key < bestKey : key > bestKey) {
          bestItem = recv[i];
          bestKey = key;
        }
      }
      return bestItem;
    }
    case "group_by": {
      // A Hash (`Map`) of block key -> list of elements, in first-seen order.
      const groups = new Map<Val, Val>();
      for (const item of recv) {
        const key = apply(block, [item]);
        const bucket = groups.get(key);
        if (Array.isArray(bucket)) bucket.push(item);
        else groups.set(key, [item]);
      }
      return groups;
    }
    case "partition": {
      const yes: Val[] = [];
      const no: Val[] = [];
      for (const item of recv) {
        if (truthy(apply(block, [item]))) yes.push(item);
        else no.push(item);
      }
      return [yes, no];
    }
    case "take_while": {
      const out: Val[] = [];
      for (const item of recv) {
        if (truthy(apply(block, [item]))) out.push(item);
        else break;
      }
      return out;
    }
    case "drop_while": {
      const out: Val[] = [];
      let dropping = true;
      for (const item of recv) {
        if (dropping && truthy(apply(block, [item]))) continue;
        dropping = false;
        out.push(item);
      }
      return out;
    }
    case "count":
      // `count { |x| pred }` — number of truthy results (arg/bare forms are in
      // the non-block `arrayMethod`).
      return recv.reduce(
        (n: number, item: Val) => (truthy(apply(block, [item])) ? n + 1 : n),
        0,
      );
    case "each_with_object": {
      // `each_with_object(memo) { |x, memo| … }` — folds into and returns memo.
      if (args.length === 0) return recv;
      const memo = args[0];
      for (const item of recv) apply(block, [item, memo]);
      return memo;
    }
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
      // Ruby `Hash#fetch(key)` — unlike `hash[key]` (which returns nil on a
      // miss), `fetch` raises `KeyError` when the key is absent AND no default
      // was supplied (T2). With a second argument, that default is returned
      // (Ruby's `fetch(key, default)`); a block form is out of scope.
      if (recv.has(args[0])) return recv.get(args[0]);
      if (args.length > 1) return args[1]; // explicit default — no raise
      raiseError("KeyError", `key not found: ${rubyInspect(args[0])}`);
      return MISS; // unreachable: raiseError returns `never`
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
    case "ljust":
    case "rjust":
    case "center": {
      // Ruby `String#ljust`/`#rjust`/`#center(width, pad = " ")`: pad to `width`
      // CODE POINTS using `pad` cyclically; `width <= length` returns the string
      // unchanged; `center` puts an odd extra pad rune on the RIGHT (Ruby's rule).
      // An empty pad degrades to a single space (never-raise floor); the padding
      // length is clamped to `MAX_REPEAT_LEN` (like `strRepeat`) to bound a DoS.
      const width = typeof args[0] === "number" ? Math.trunc(args[0]) : 0;
      const pad = typeof args[1] === "string" && args[1] !== "" ? args[1] : " ";
      const cps = [...recv];
      const deficit = Math.min(width - cps.length, MAX_REPEAT_LEN);
      if (deficit <= 0) return recv;
      const pr = [...pad];
      const buildPad = (n: number): string => {
        let out = "";
        for (let i = 0; i < n; i++) out += pr[i % pr.length];
        return out;
      };
      if (name === "ljust") return recv + buildPad(deficit);
      if (name === "rjust") return buildPad(deficit) + recv;
      const left = Math.floor(deficit / 2);
      return buildPad(left) + recv + buildPad(deficit - left);
    }
    case "swapcase": {
      // Flip the case of each ASCII letter (non-letters / non-ASCII untouched),
      // iterating whole code points so astral runes are never split.
      let out = "";
      for (const ch of recv) {
        const c = ch.codePointAt(0) as number;
        if (c >= 65 && c <= 90) out += String.fromCodePoint(c + 32);
        else if (c >= 97 && c <= 122) out += String.fromCodePoint(c - 32);
        else out += ch;
      }
      return out;
    }
    case "tr": {
      // Ruby `String#tr(from, to)`: translate each char that appears in `from`
      // to the char at the same position in `to`.  A shorter `to` repeats its
      // LAST char; an empty `to` deletes matching chars; when `from` repeats a
      // char the last mapping wins.
      //
      //   "hello".tr("el", "ip") == "hippo"   (e→i, l→p)
      //   "hello".tr("l", "r")   == "herro"   (single mapping)
      //   "hello".tr("aeiou", "*") == "h*ll*" (shorter `to` repeats "*")
      //   "hello".tr("l", "")    == "heo"     (empty `to` deletes)
      //
      // NOTE: the char-RANGE (`"a-z"`) and NEGATION (`"^abc"`) forms are a
      // follow-up, matching the literal-only `sub`/`gsub` precedent here.  We
      // iterate whole code points (`[...s]` / `for…of`) so astral runes are
      // never split mid-surrogate.
      const from = typeof args[0] === "string" ? (args[0] as string) : null;
      const to = typeof args[1] === "string" ? (args[1] as string) : null;
      if (from === null || to === null) return recv;
      const toC = [...to];
      const fromC = [...from];
      // Map each `from` code point to its replacement (or `null` = delete).
      const table = new Map<string, string | null>();
      for (let i = 0; i < fromC.length; i++) {
        if (toC.length === 0) table.set(fromC[i], null);
        else table.set(fromC[i], i < toC.length ? toC[i] : toC[toC.length - 1]);
      }
      let out = "";
      for (const ch of recv) {
        if (table.has(ch)) {
          const r = table.get(ch) as string | null;
          if (r !== null) out += r;
        } else {
          out += ch;
        }
      }
      return out;
    }
    case "count":
    case "delete":
    case "squeeze": {
      // Char-set methods.  Each `string` argument is treated LITERALLY — the set
      // of characters it contains (ranges/negation are a follow-up).  `count`
      // returns how many chars of `recv` lie in the set; `delete` removes them;
      // `squeeze` collapses consecutive runs (of set chars, or of ALL chars when
      // no set is given).  Multiple set args INTERSECT (Ruby's rule).
      //
      //   "hello".count("l")        == 2
      //   "hello".delete("l")       == "heo"
      //   "mississippi".squeeze     == "misisipi"   (no set: all runs)
      //   "aaabbbccc".squeeze("a")  == "abbbccc"    (only "a" runs collapse)
      const sets: Array<Set<string>> = [];
      for (const a of args) if (typeof a === "string") sets.push(new Set([...a]));
      // A char is "in the set" only when it is present in EVERY set argument.
      const inAll = (ch: string): boolean => sets.length > 0 && sets.every((set) => set.has(ch));
      if (name === "squeeze" && sets.length === 0) {
        // Bare `squeeze` collapses every run of identical characters.
        let out = "";
        let last: string | null = null;
        for (const ch of recv) {
          if (ch !== last) {
            out += ch;
            last = ch;
          }
        }
        return out;
      }
      if (name === "count") {
        let n = 0;
        for (const ch of recv) if (inAll(ch)) n++;
        return n;
      }
      if (name === "delete") {
        let out = "";
        for (const ch of recv) if (!inAll(ch)) out += ch;
        return out;
      }
      // squeeze(set): collapse only runs of characters that lie in the set.
      let out = "";
      let last: string | null = null;
      for (const ch of recv) {
        if (ch === last && inAll(ch)) continue;
        out += ch;
        last = ch;
      }
      return out;
    }
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

// ── Ruby display forms (to_s / inspect) ──────────────────────────────────────
//
// sir-runtime-core's `toDisplay` renders *Lisp* forms (`nil`, `#t`, `#f`), so it
// is wrong for Ruby's `to_s`/`inspect`.  These helpers implement Ruby's surface:
// `nil.to_s == ""` but `nil.inspect == "nil"`; booleans print `true`/`false`; a
// symbol's `to_s` is its bare name and `inspect` prefixes `:`; an Array's `to_s`
// equals its `inspect` (`"[1, 2]"`); a Hash (Map) renders `{:k=>v}`.  String
// escaping in `inspect` is the v0 naive form (wrap in quotes; no escaping yet).
//
// NB: JS cannot distinguish `3.0` from `3` (both `number`, `Number.isInteger`),
// so a whole-valued Float prints as an integer — a documented v0 limitation.

function rubyToS(v: Val): string {
  if (v === null || v === undefined) return "";
  if (typeof v === "boolean") return v ? "true" : "false";
  if (isSymbol(v)) return v.name as string;
  if (typeof v === "string") return v;
  if (Array.isArray(v) || v instanceof Map) return rubyInspect(v);
  return String(v);
}

/** `seen` (a set of container references) and `depth` make this safe on
 * self-referential or deeply-nested structures: a cycle renders `[...]` /
 * `{...}` (matching Ruby) instead of recursing forever, and depth is capped at
 * `MAX_DISPLAY_DEPTH` so a deep acyclic structure cannot overflow the stack. */
function rubyInspect(v: Val, seen: Set<object> = new Set<object>(), depth = 0): string {
  if (v === null || v === undefined) return "nil";
  if (typeof v === "boolean") return v ? "true" : "false";
  if (isSymbol(v)) return ":" + (v.name as string);
  if (typeof v === "string") return '"' + v + '"';
  if (Array.isArray(v)) {
    if (seen.has(v) || depth > MAX_DISPLAY_DEPTH) return "[...]";
    seen.add(v);
    const body = v.map((item: Val) => rubyInspect(item, seen, depth + 1)).join(", ");
    seen.delete(v);
    return "[" + body + "]";
  }
  if (v instanceof Map) {
    if (seen.has(v) || depth > MAX_DISPLAY_DEPTH) return "{...}";
    seen.add(v);
    const body = [...v]
      .map(([k, val]: [Val, Val]) =>
        `${rubyInspect(k, seen, depth + 1)}=>${rubyInspect(val, seen, depth + 1)}`,
      )
      .join(", ");
    seen.delete(v);
    return "{" + body + "}";
  }
  return String(v);
}

// ── Numeric (Integer / Float) catalog ────────────────────────────────────────

function gcdInt(a: number, b: number): number {
  let x = Math.abs(Math.trunc(a));
  let y = Math.abs(Math.trunc(b));
  while (y !== 0) {
    [x, y] = [y, x % y];
  }
  return x;
}

// Recursion bound for `to_s`/`inspect`/`join` on nested containers (cycles are
// caught separately by identity tracking); past it, render a placeholder rather
// than overflowing the stack — honouring the never-crash invariant.
const MAX_DISPLAY_DEPTH = 100;

function digitsOf(n: number): number[] {
  // `2 ** 1e9` saturates to `Infinity` in IEEE-754; guard so the loop below
  // (which never reaches 0 for a non-finite value) cannot spin forever.
  if (!Number.isFinite(n)) return [0];
  let m = Math.abs(Math.trunc(n));
  if (m === 0) return [0];
  const out: number[] = [];
  while (m > 0) {
    out.push(m % 10);
    m = Math.floor(m / 10);
  }
  return out;
}

/** Ruby round: half **away from zero** (`2.5 -> 3`, `-2.5 -> -3`), unlike JS
 * `Math.round` which rounds half toward +Infinity. */
function rubyRound(x: number): number {
  return x >= 0 ? Math.floor(x + 0.5) : Math.ceil(x - 0.5);
}

/** Non-block `Integer`/`Float` methods.  Returns `MISS` if not catalogued. */
function numericMethod(recv: number, name: string, args: Val[]): Val | typeof MISS {
  switch (name) {
    case "abs":
      return Math.abs(recv);
    case "to_i":
      return Math.trunc(recv);
    case "to_f":
      return recv;
    case "even?":
      return recv % 2 === 0;
    case "odd?":
      return recv % 2 !== 0;
    case "zero?":
      return recv === 0;
    case "positive?":
      return recv > 0;
    case "negative?":
      return recv < 0;
    case "succ":
    case "next":
      return recv + 1;
    case "pred":
      return recv - 1;
    case "floor":
      return Math.floor(recv);
    case "ceil":
      return Math.ceil(recv);
    case "round": {
      // Ruby `round` / `round(ndigits)` — half AWAY from zero (via `rubyRound`,
      // NOT `Math.round` which is half-toward-+∞).  With no argument (or an
      // integer receiver and `ndigits >= 0`) the result is an integer; a
      // positive `ndigits` rounds to that many decimals; `ndigits <= 0` rounds
      // to a power of ten.  TS numbers are f64 — a hostile-magnitude `ndigits`
      // degrades naturally (the `factor` saturates to `Infinity` and
      // `recv / Infinity` is `0`), with no bignum and no allocation.  A
      // non-finite receiver returns unchanged.
      const nd = typeof args[0] === "number" ? Math.trunc(args[0]) : 0;
      if (!Number.isFinite(recv)) return recv;
      if (Number.isInteger(recv) && nd >= 0) return recv;
      const factor = Math.pow(10, nd);
      return rubyRound(recv * factor) / factor;
    }
    case "divmod": {
      // Ruby `divmod(n)` → `[quotient, remainder]` with a FLOORED quotient and
      // the divisor-signed remainder.  Division by zero raises a typed
      // `ZeroDivisionError` (so a translated `rescue` catches it).
      const d = typeof args[0] === "number" ? args[0] : 0;
      if (d === 0) raiseError("ZeroDivisionError", "divided by 0");
      const q = Math.floor(recv / d);
      const r = recv - q * d;
      return [q, r];
    }
    case "fdiv": {
      // Ruby `fdiv(n)` — floating-point division that NEVER raises: dividing by
      // zero yields `Infinity`/`-Infinity`/`NaN` (JS `/` already produces these),
      // honouring the never-raise floor.
      return recv / (typeof args[0] === "number" ? args[0] : 0);
    }
    case "clamp": {
      // Ruby `Comparable#clamp(min, max)`: `min` if recv < min, `max` if
      // recv > max, else recv.  (The Range form is a follow-up.)
      const lo = typeof args[0] === "number" ? args[0] : 0;
      const hi = typeof args[1] === "number" ? args[1] : 0;
      if (recv < lo) return lo;
      if (recv > hi) return hi;
      return recv;
    }
    case "between?": {
      // Ruby `Comparable#between?(min, max)`: `min <= recv <= max`.
      const lo = typeof args[0] === "number" ? args[0] : 0;
      const hi = typeof args[1] === "number" ? args[1] : 0;
      return recv >= lo && recv <= hi;
    }
    case "gcd":
      return gcdInt(recv, args[0]);
    case "pow":
    case "**":
      return recv ** args[0];
    case "digits":
      return digitsOf(recv);
    default:
      return MISS;
  }
}

/** Block-taking `Integer` methods (`times`/`upto`/`downto`/`step`); each returns
 * the receiver.  Returns `MISS` otherwise. */
function numericBlockMethod(
  recv: number,
  name: string,
  args: Val[],
  block: Closure,
): Val | typeof MISS {
  switch (name) {
    case "times":
      for (let i = 0; i < Math.trunc(recv); i++) apply(block, [i]);
      return recv;
    case "upto":
      for (let i = recv; i <= args[0]; i++) apply(block, [i]);
      return recv;
    case "downto":
      for (let i = recv; i >= args[0]; i--) apply(block, [i]);
      return recv;
    case "step": {
      const limit = args[0];
      const stride = args.length > 1 ? args[1] : 1;
      if (stride > 0) {
        for (let i = recv; i <= limit; i += stride) apply(block, [i]);
      } else if (stride < 0) {
        for (let i = recv; i >= limit; i += stride) apply(block, [i]);
      }
      return recv;
    }
    default:
      return MISS;
  }
}

// ── Symbol catalog ───────────────────────────────────────────────────────────

/** `Symbol` methods. `upcase`/`downcase` return a *new* interned symbol (Ruby
 * semantics). Returns `MISS` if not catalogued. */
function symbolMethod(recv: Val, name: string): Val | typeof MISS {
  const sym = recv.name as string;
  switch (name) {
    case "to_s":
      return sym;
    case "to_sym":
      return recv;
    case "length":
    case "size":
      return sym.length;
    case "upcase":
      return intern(sym.toUpperCase());
    case "downcase":
      return intern(sym.toLowerCase());
    case "inspect":
      return ":" + sym;
    case "empty?":
      return sym.length === 0;
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
  // User objects (O1): a method registered via `defMethod` (walking the class's
  // ancestry) is dispatched first — push the receiver as the current self, apply
  // the stored `Closure` with `args`, then pop self.  Only if no user method
  // resolves does dispatch fall through to the reflective built-ins
  // (`is_a?`/`class`/…) and the primitive catalog — so `obj.class` still works
  // while `obj.speak` runs the user body.
  if (recv instanceof SirInstance) {
    const userFn = resolveInstanceMethod(recv.sirClass, name);
    if (userFn !== null) {
      pushSelf(recv);
      try {
        return apply(userFn, args);
      } finally {
        popSelf();
      }
    }
  }

  switch (name) {
    case "is_a?":
    case "kind_of?":
      return isA(recv, classNameArg(args[0]));
    case "instance_of?":
      return classOf(recv) === classNameArg(args[0]);
    case "class":
      return classOf(recv);
  }

  // The user `defineMethod` table is consulted first (resolution order #2), so a
  // user-defined `send` override wins.
  const m = methods.get(name);
  if (m) return m(recv, args);

  // `send`/`__send__`/`public_send` re-enter dispatch with a *dynamic* method
  // name taken from the first argument (a Symbol or string), forwarding the rest
  // unchanged — so `x.send("upcase")` is exactly `x.upcase` and a trailing block
  // survives as a trailing arg.  An empty arg list bottoms out at the `null`
  // floor rather than throwing; routing recurses through `callMethod`.
  if (SEND_METHODS.has(name) && args.length > 0) {
    return callMethod(recv, methodNameArg(args[0]), ...args.slice(1));
  }

  if (typeof recv === "string") {
    // A block method (each_char) dispatches only with a trailing Closure.
    const last = args[args.length - 1];
    if (STRING_BLOCK_METHODS.has(name) && args.length > 0 && last instanceof Closure) {
      const blkResult = stringBlockMethod(recv, name, last);
      if (blkResult !== MISS) return blkResult;
    }
    const strResult = stringMethod(recv, name, args);
    if (strResult !== MISS) return strResult;
  } else if (isSymbol(recv)) {
    const symResult = symbolMethod(recv, name);
    if (symResult !== MISS) return symResult;
  } else if (typeof recv === "boolean") {
    // `boolean` is a distinct typeof — resolve the eager logical operators
    // (&/|/^) here, then fall through to the universal Object methods.
    const boolResult = boolMethod(recv, name, args);
    if (boolResult !== MISS) return boolResult;
  } else if (typeof recv === "number") {
    const last = args[args.length - 1];
    if (NUMERIC_BLOCK_METHODS.has(name) && args.length > 0 && last instanceof Closure) {
      const blkResult = numericBlockMethod(recv, name, args.slice(0, -1), last);
      if (blkResult !== MISS) return blkResult;
    }
    const numResult = numericMethod(recv, name, args);
    if (numResult !== MISS) return numResult;
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

  // Universal block-taking methods (`tap`/`then`/`yield_self`) apply to *every*
  // receiver, so they are dispatched here — after the type-specific catalogs —
  // only when an actual trailing `Closure` block is present.  A block-less
  // `tap`/`then` falls through to `objectMethod`, which returns the receiver
  // (the documented v0 Enumerator-less floor).
  const lastArg = args[args.length - 1];
  if (OBJECT_BLOCK_METHODS.has(name) && args.length > 0 && lastArg instanceof Closure) {
    const blkResult = objectBlockMethod(recv, name, lastArg);
    if (blkResult !== MISS) return blkResult;
  }

  const objResult = objectMethod(recv, name, args);
  if (objResult !== MISS) return objResult;

  // Nothing resolved `name` on `recv`. Ruby distinguishes two cases here, and
  // so must we (T2) — the difference is load-bearing so we do NOT over-raise:
  //
  //   • A method the receiver GENUINELY DOES NOT HAVE (`obj.undefined`,
  //     `nil.foo`, `5.bit_length`) → Ruby raises `NoMethodError`. We raise the
  //     typed `SirError` so a Ruby `rescue NoMethodError` catches it, replacing
  //     the previous silent `nil` floor.
  //
  //   • A method the receiver DOES have but was called in a shape v0 doesn't
  //     model — most notably a block-taking method invoked WITHOUT a block
  //     (`[1,2,3].map`, `5.times`) → Ruby returns an *Enumerator*, NOT a
  //     `NoMethodError`. We have no Enumerator in v0, so the honest floor stays
  //     `nil` (never a spurious raise). `respondsTo` reports catalog membership,
  //     so it cleanly separates the two: an unknown name is `false` (→ raise), a
  //     known-but-unsupported-shape name is `true` (→ nil floor).
  //
  // A method that legitimately RETURNS nil (`[].first`, `{}.fetch(k, nil)`)
  // never reaches here — it returns from its catalog arm above.
  if (!respondsTo(recv, name)) {
    raiseError("NoMethodError", `undefined method '${name}' for ${classOf(recv)}`);
  }
  return null;
}

function classNameArg(arg: Val): string {
  return typeof arg === "string" ? arg : classOf(arg);
}

// --- Symbol#to_proc (&:sym) ------------------------------------------------
//
// Ruby's `&:sym` block argument converts a `Symbol` into a block via
// `Symbol#to_proc`: the resulting proc calls the named method on its first
// argument, forwarding any remaining arguments. So `[1, 2, 3].map(&:to_s)` is
// `[1, 2, 3].map { |x| x.to_s }` and `[1, 2].inject(&:+)` is
// `inject { |acc, x| acc + x }`.
//
// The Ruby→SIR frontend lowers `&:sym` to `block_pass(SymLit("sym"))`; the
// backend emits the surviving `block_pass` envelope as a call to this helper
// (`__SirOop.symToProc(intern("sym"))`), which yields a `Closure` the
// block-taking catalog methods (`map`/`select`/…) drive through `apply`
// exactly like a `{ }` block. `apply` forwards arguments unadjusted, so the
// first becomes the receiver and the rest are forwarded as method arguments —
// faithful to `&:sym`'s arity (one required receiver plus a rest), correct for
// both the one-arg (`map`) and two-arg (`inject`) shapes.
export function symToProc(sym: Val): Closure {
  const method = isSymbol(sym) ? (sym as { name: string }).name : String(sym);
  return new Closure((recv: Val, ...rest: Val[]): Val => callMethod(recv, method, ...rest));
}

/**
 * Ruby case-equality (`pattern === value`), the test a `when` clause runs (M5).
 * Unlike `==`, the operation is keyed to the *pattern*'s type:
 *
 * | pattern kind | semantics                                    |
 * |--------------|----------------------------------------------|
 * | `RegExp`     | the regex matches `String(value)`            |
 * | `Range`      | membership — `value` falls in the range      |
 * | otherwise    | value equality (`==`)                        |
 *
 * The class case (`when Integer`) is handled at the *frontend* (it lowers to
 * `value.is_a?(Const)` via the `__method__` envelope) and never reaches here.
 * The else-branch floor is `eq`, so a literal `when 5` keeps plain equality.
 *
 * `Range` is detected structurally (by constructor name + an `includes`
 * method) rather than imported, so this package gains no dependency on
 * `sir-runtime-range`; `RegExp` is a native JS type.
 */
export function caseEq(pattern: Val, value: Val): boolean {
  if (pattern instanceof RegExp) {
    // Ruby `/re/ === x` is true when the regex matches x (a String); a
    // non-string scrutinee never matches.
    if (typeof value !== "string") {
      return false;
    }
    return pattern.test(value);
  }
  const p = pattern as { constructor?: { name?: string }; includes?: (v: Val) => boolean };
  if (
    p != null &&
    typeof p.includes === "function" &&
    p.constructor != null &&
    p.constructor.name === "Range"
  ) {
    return Boolean(p.includes(value));
  }
  return eq(pattern, value);
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
  instanceMethods.clear();
  classMethods.clear();
  includedModules.clear();
}
