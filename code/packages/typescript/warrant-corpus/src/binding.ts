/**
 * # The binding table: Warrant operations -> this lane's callables
 *
 * `evidence.bindings` is *data*. It says which public export implements each
 * Warrant operation and how a thrown error becomes a declared `Err` -- and
 * nothing else. It cannot hold logic (a computed answer, a retry, a fallback):
 * that is precisely why it is a flat file of directives and not TypeScript. See
 * VER04i section 5.2 for the grammar:
 *
 *     lane typescript
 *     package typescript/ct-compare
 *     module ct_compare
 *     import src/index.ts
 *     source src
 *     seed 0x5741_5252_414e_5421
 *     values Bool=boolean Bytes=Uint8Array Nat=bigint U64=bigint
 *     op eq export=ctEq
 *     op select_bytes export=ctSelectBytes errors=CT_SELECT_LENGTH_MISMATCH:RangeError
 *
 * Two guards make the binding trustworthy at the boundary:
 *
 * - **Resolution at start-up.** Every export must exist and be callable
 *   *before* a single case runs (a lazily-missing export would otherwise surface
 *   as a pass on whichever suites never reached it).
 * - **Source binding.** The imported module's real path must live under the
 *   declared `source` root. The manifest's `source_sha256` digests that root; if
 *   the runtime had instead picked up a stale copy from `node_modules`, the
 *   digest would describe code that never ran. `import` is a *path*, never a
 *   bare package name, for the same reason.
 *
 * Error normalisation is closed (VER04i 5, rule 2): a *mapped* error class
 * becomes `Err(variant)`; anything else thrown is `unexpected-error`. The match
 * is on the exact prototype, so a subclass of `RangeError`, a look-alike object
 * and a thrown string are all unexpected. Class names resolve only among the
 * built-in error classes below, never to `Error` itself, so a binding cannot
 * swallow everything.
 *
 * ## Mutable bytes
 *
 * A `Uint8Array` is mutable, which Python's `bytes` is not. So every `Bytes`
 * argument is handed to the library as **its own fresh copy** (the enumerated
 * domain can never be corrupted by a library that writes to its input), and for
 * a `fresh` op the result must not share an `ArrayBuffer` with any argument
 * (rule 4).
 */

import { realpathSync } from "node:fs";
import { isAbsolute, join, relative } from "node:path";
import { pathToFileURL } from "node:url";
import { BindingError, CorpusError, UNEXPECTED_ERROR, TYPE_MISMATCH } from "./errors.ts";
import type { Fault } from "./faults.ts";
import { checkValue, lift, lower, type ErrorTypes, type OpSig } from "./values.ts";

export const MAX_BINDING_BYTES = 64 * 1024;

const NAME = /^[a-z][a-z0-9_]*$/;
const EXPORT = /^[A-Za-z_$][A-Za-z0-9_$]*$/;
const VARIANT = /^[A-Z][A-Z0-9_]*$/;
const SEED = /^(?:0x[0-9a-fA-F_]+|[0-9][0-9_]*)$/;
const IMPORT_PATH = /^[A-Za-z0-9_.@-]+(?:\/[A-Za-z0-9_.@-]+)*$/;

/** The only lane type each Warrant scalar may have (VER04i 5.2). */
const ALLOWED_VALUES: Readonly<Record<string, string>> = {
  Bool: "boolean",
  Bytes: "Uint8Array",
  U8: "number",
  U64: "bigint",
  Nat: "bigint",
};

/** The built-in error classes a binding may name. `Error` itself is deliberately absent. */
const ERROR_CLASSES: Readonly<Record<string, ErrorConstructor>> = {
  RangeError,
  TypeError,
  SyntaxError,
  ReferenceError,
  EvalError,
  URIError,
};

export type WidthMode = "none" | "implicit" | "runtime";

/** How one Warrant op maps to an export. */
export interface OpBinding {
  readonly name: string;
  readonly exportName: string;
  readonly width: WidthMode;
  readonly errors: readonly (readonly [string, ErrorConstructor])[];
}

/** A parsed `evidence.bindings`. */
export interface Binding {
  readonly lane: string;
  readonly package: string;
  readonly module: string;
  readonly importPath: string;
  readonly source: string;
  readonly seed: bigint;
  readonly values: ReadonlyMap<string, string>;
  readonly ops: ReadonlyMap<string, OpBinding>;
  readonly raw: Uint8Array;
}

function errorClass(name: string): ErrorConstructor {
  const found = Object.hasOwn(ERROR_CLASSES, name) ? ERROR_CLASSES[name] : undefined;
  if (found === undefined) throw new BindingError("bad-exception", `\`${name}\` is not an allowed error class`);
  return found;
}

function parseOp(parts: readonly string[]): OpBinding {
  const name = parts[0] as string;
  if (parts.length < 2 || !NAME.test(name)) throw new BindingError("syntax", "malformed op directive");
  const options = new Map<string, string>();
  for (const item of parts.slice(1)) {
    const at = item.indexOf("=");
    const key = at < 0 ? item : item.slice(0, at);
    const value = at < 0 ? "" : item.slice(at + 1);
    if (at < 0 || value === "" || options.has(key)) {
      throw new BindingError("syntax", `malformed or duplicate op option \`${key}\``);
    }
    options.set(key, value);
  }
  const unknown = [...options.keys()].filter((key) => !["export", "width", "errors"].includes(key));
  if (unknown.length > 0 || !options.has("export")) {
    throw new BindingError("syntax", `op \`${name}\` needs export= and only known options`);
  }
  const exportName = options.get("export") as string;
  if (!EXPORT.test(exportName)) throw new BindingError("syntax", "export must be an identifier");
  const width = options.get("width") ?? "none";
  if (width !== "none" && width !== "implicit" && width !== "runtime") {
    throw new BindingError("syntax", "width must be implicit or runtime");
  }
  const errors: (readonly [string, ErrorConstructor])[] = [];
  for (const row of (options.get("errors") ?? "").split(",").filter((item) => item !== "")) {
    const split = row.indexOf(":");
    const variant = split < 0 ? "" : row.slice(0, split);
    if (split < 0 || !VARIANT.test(variant)) {
      throw new BindingError("syntax", "errors rows are VARIANT:ErrorClassName");
    }
    errors.push([variant, errorClass(row.slice(split + 1))]);
  }
  return { name, exportName, width, errors };
}

function parseSeed(text: string): bigint {
  if (!SEED.test(text)) throw new BindingError("syntax", "seed must be a decimal or 0x hex u64");
  const digits = text.replaceAll("_", "");
  if (digits === "0x") throw new BindingError("syntax", "seed must be a decimal or 0x hex u64");
  const seed = BigInt(digits);
  if (seed >= 1n << 64n) throw new BindingError("syntax", "seed is outside u64");
  return seed;
}

/** Parse the directive file; any irregularity is a {@link BindingError}. */
export function parseBinding(data: Uint8Array): Binding {
  if (data.length > MAX_BINDING_BYTES) throw new BindingError("input-limit", "binding file is too large");
  let text: string;
  try {
    text = new TextDecoder("utf-8", { fatal: true }).decode(data);
  } catch {
    throw new BindingError("syntax", "binding is not UTF-8");
  }
  const single = new Map<string, string>();
  const values = new Map<string, string>();
  const ops = new Map<string, OpBinding>();
  for (const rawLine of text.split(/\r\n|\r|\n/)) {
    const line = rawLine.trim();
    if (line === "" || line.startsWith("#")) continue;
    const [directive, ...parts] = line.split(/\s+/) as [string, ...string[]];
    if (directive === "op") {
      const op = parseOp(parts);
      if (ops.has(op.name)) throw new BindingError("syntax", `duplicate op \`${op.name}\``);
      ops.set(op.name, op);
    } else if (directive === "values") {
      for (const item of parts) {
        const at = item.indexOf("=");
        const key = at < 0 ? item : item.slice(0, at);
        if (at < 0 || values.has(key)) throw new BindingError("syntax", "malformed or duplicate values row");
        values.set(key, item.slice(at + 1));
      }
    } else if (["lane", "package", "module", "import", "source", "seed"].includes(directive)) {
      if (parts.length !== 1 || single.has(directive)) {
        throw new BindingError("syntax", `\`${directive}\` takes one value, once`);
      }
      single.set(directive, parts[0] as string);
    } else {
      throw new BindingError("syntax", `unknown directive \`${directive}\``);
    }
  }
  const missing = ["lane", "package", "module", "import", "source", "seed"].filter((key) => !single.has(key));
  if (missing.length > 0 || ops.size === 0) {
    throw new BindingError("incomplete", `binding lacks ${missing.length > 0 ? missing.join(", ") : "ops"}`);
  }
  for (const key of ["import", "source"]) {
    const value = single.get(key) as string;
    if (!IMPORT_PATH.test(value) || value.split("/").some((part) => part === "." || part === "..")) {
      throw new BindingError("syntax", `${key} must be a normalised package-relative path`);
    }
  }
  const seed = parseSeed(single.get("seed") as string);
  // Rule 3: width representations are stated, and forbidden ones are refused.
  for (const [scalar, lanType] of values) {
    if (!Object.hasOwn(ALLOWED_VALUES, scalar) || ALLOWED_VALUES[scalar] !== lanType) {
      throw new BindingError("values", `\`${scalar}=${lanType}\` is not an allowed TypeScript representation`);
    }
  }
  return {
    lane: single.get("lane") as string,
    package: single.get("package") as string,
    module: single.get("module") as string,
    importPath: single.get("import") as string,
    source: single.get("source") as string,
    seed,
    values,
    ops,
    raw: data,
  };
}

// ---------------------------------------------------------------------------
// Resolution and the subject
// ---------------------------------------------------------------------------

/** The library misbehaved for one case: a verdict, not a lane error. */
export class SubjectFailure extends Error {
  readonly failureId: string;

  constructor(failureId: string) {
    super(failureId);
    this.failureId = failureId;
  }
}

export type Exports = ReadonlyMap<string, (...args: unknown[]) => unknown>;

/** Is `child` the same path as `root` or beneath it? */
function isUnder(root: string, child: string): boolean {
  const path = relative(root, child);
  return path === "" || (!path.startsWith("..") && !isAbsolute(path));
}

/** Import the library and resolve every export, or refuse to start. */
export async function resolveExports(binding: Binding, packageDir: string): Promise<Exports> {
  let root: string;
  let file: string;
  try {
    root = realpathSync(join(packageDir, ...binding.source.split("/")));
    file = realpathSync(join(packageDir, ...binding.importPath.split("/")));
  } catch {
    throw new BindingError("import", `cannot resolve \`${binding.importPath}\` under the package`);
  }
  if (!isUnder(root, file)) {
    throw new BindingError(
      "source-mismatch",
      `\`${binding.importPath}\` is not inside the declared source root`,
    );
  }
  let namespace: Record<string, unknown>;
  try {
    namespace = (await import(pathToFileURL(file).href)) as Record<string, unknown>;
  } catch {
    throw new BindingError("import", `cannot import \`${binding.importPath}\``);
  }
  const exports = new Map<string, (...args: unknown[]) => unknown>();
  for (const [name, op] of binding.ops) {
    const target = namespace[op.exportName];
    if (typeof target !== "function") {
      throw new BindingError("export", `op \`${name}\`: \`${op.exportName}\` is missing or not callable`);
    }
    exports.set(name, target as (...args: unknown[]) => unknown);
  }
  return exports;
}

/**
 * The real library, seen through the binding, with a call counter.
 *
 * `call` is the *only* path from a corpus case to the library. It counts
 * invocations (the runner compares the count with the corpus's
 * `expected_calls`, so a runner that never calls the binding is caught), applies
 * an optional fault, type-checks the result exactly, and returns it in canonical
 * form: a value, or `["err", VARIANT]`.
 */
export class Subject {
  readonly calls = new Map<string, number>();

  private readonly binding: Binding;
  private readonly exports: Exports;
  private readonly signatures: ReadonlyMap<string, OpSig>;
  private readonly errors: ErrorTypes;
  private readonly fault: Fault | null;
  private readonly faultOp: string | null;

  constructor(
    binding: Binding,
    exports: Exports,
    signatures: ReadonlyMap<string, OpSig>,
    errors: ErrorTypes,
    fault: Fault | null = null,
    faultOp: string | null = null,
  ) {
    this.binding = binding;
    this.exports = exports;
    this.signatures = signatures;
    this.errors = errors;
    this.fault = fault;
    this.faultOp = faultOp;
    for (const name of binding.ops.keys()) this.calls.set(name, 0);
  }

  /** Calls made so far to any op in `ops`. */
  totalCalls(ops: readonly string[]): number {
    return ops.reduce((sum, name) => sum + (this.calls.get(name) ?? 0), 0);
  }

  callsTo(op: string): number {
    return this.calls.get(op) ?? 0;
  }

  private faulted(op: string, signature: OpSig): Fault | null {
    const fault = this.fault;
    if (fault === null || (this.faultOp !== null && op !== this.faultOp)) return null;
    return fault.applies(signature) ? fault : null;
  }

  /** Invoke `op` on canonical `args`; return the exactly-typed, canonical result. */
  call(op: string, args: readonly unknown[], width: number | undefined): unknown {
    const signature = this.signatures.get(op) as OpSig;
    const binding = this.binding.ops.get(op) as OpBinding;
    this.calls.set(op, (this.calls.get(op) as number) + 1);
    if (width !== undefined) {
      for (const value of args) {
        if (value instanceof Uint8Array && value.length !== width) {
          throw new CorpusError("width", "a Bytes<N> argument does not have length N");
        }
      }
    }
    // Wherever the signature itself states a length (`Bytes<4>`), assert it.
    signature.params.forEach(([, ty], index) => {
      const value = args[index];
      if (
        ty.kind === "Bytes" &&
        typeof ty.width === "number" &&
        (!(value instanceof Uint8Array) || value.length !== ty.width)
      ) {
        throw new CorpusError("width", "an argument does not have its declared length");
      }
    });
    const given = args.map((value, index) => lower(value, (signature.params[index] as readonly [string, never])[1]));
    const fault = this.faulted(op, signature);
    let passed = fault ? fault.onInput([...given], signature) : given;
    if (binding.width === "runtime") passed = [...passed, width];
    let outcome: unknown;
    try {
      const returned = (this.exports.get(op) as (...values: unknown[]) => unknown)(...passed);
      // A library that hands back a promise has not returned a value; do not
      // let its rejection escape as an unhandled one while we fail the case.
      if (returned instanceof Promise) returned.catch(() => undefined);
      // A library signals success by returning the value itself; the model's
      // Result wraps it. Wrapping here (never trusting a returned pair) is what
      // makes a stray ["err", ...] from the library fail the type check instead
      // of passing as an error.
      outcome = signature.ret.kind === "Result" ? ["ok", returned] : returned;
    } catch (error) {
      outcome = this.mapped(binding, error);
    }
    if (signature.fresh) this.requireFresh(outcome, passed);
    const final = fault ? fault.onOutput(outcome, given, signature) : outcome;
    if (!checkValue(final, signature.ret, this.errors)) throw new SubjectFailure(TYPE_MISMATCH);
    return lift(final, signature.ret);
  }

  /** Exactly the mapped class becomes `Err(variant)`; anything else is a verdict. */
  private mapped(binding: OpBinding, error: unknown): unknown {
    if (typeof error === "object" && error !== null) {
      const prototype: unknown = Object.getPrototypeOf(error);
      for (const [variant, errorConstructor] of binding.errors) {
        if (prototype === errorConstructor.prototype) return ["err", variant];
      }
    }
    throw new SubjectFailure(UNEXPECTED_ERROR);
  }

  /** Rule 4: a `fresh` result may not share an `ArrayBuffer` with any argument it was given. */
  private requireFresh(outcome: unknown, passed: readonly unknown[]): void {
    const value = Array.isArray(outcome) && outcome[0] === "ok" ? outcome[1] : outcome;
    if (!(value instanceof Uint8Array)) return;
    for (const argument of passed) {
      if (argument instanceof Uint8Array && argument.buffer === value.buffer) {
        throw new SubjectFailure(TYPE_MISMATCH);
      }
    }
  }
}
