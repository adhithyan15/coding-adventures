/**
 * Starlark Built-in Functions -- The standard library of Starlark.
 *
 * ==========================================================================
 * Chapter 1: What Are Built-in Functions?
 * ==========================================================================
 *
 * Built-in functions are functions that are always available in Starlark without
 * importing them. They're implemented in the host language (TypeScript) rather
 * than in Starlark bytecode. When the VM encounters a call to ``len(x)`` or
 * ``range(10)``, it dispatches to the TypeScript function registered here.
 *
 * The Starlark specification defines approximately 30 built-in functions. This
 * module implements the most commonly used ones. Each function takes a list of
 * arguments and returns a value, following the protocol defined by
 * ``GenericVM.registerBuiltin()``.
 *
 * ==========================================================================
 * Chapter 2: Starlark vs Python Built-ins
 * ==========================================================================
 *
 * Starlark's built-ins are a strict subset of Python's, with some restrictions:
 *
 * - ``sorted()`` always returns a new list (no in-place sort)
 * - ``range()`` returns a list, not a lazy range object
 * - ``type()`` returns a string, not a type object
 * - ``print()`` returns null (output is captured by the VM)
 * - No ``eval()``, ``exec()``, ``globals()``, ``locals()`` (security)
 *
 * @module
 */

import type { VMValue } from "@coding-adventures/virtual-machine";
import { VMTypeError } from "@coding-adventures/virtual-machine";

import { starlarkTypeName, isTruthy, starlarkValueRepr } from "./types.js";

// =========================================================================
// Type functions
// =========================================================================

/**
 * type(x) -- Return the type name as a string.
 *
 * Unlike Python's ``type()`` which returns a type object, Starlark's
 * ``type()`` returns a plain string. This is simpler and avoids
 * metaprogramming complexity.
 *
 *     type(42)       --> "int"
 *     type("hello")  --> "string"
 *     type([1, 2])   --> "list"
 */
export function builtinType(...args: VMValue[]): VMValue {
  if (args.length !== 1) {
    throw new VMTypeError(`type() takes exactly 1 argument (${args.length} given)`);
  }
  return starlarkTypeName(args[0]);
}

/**
 * bool(x) -- Convert to boolean.
 *
 * Follows Starlark truthiness rules (see {@link isTruthy}).
 */
export function builtinBool(...args: VMValue[]): VMValue {
  if (args.length !== 1) {
    throw new VMTypeError(`bool() takes exactly 1 argument (${args.length} given)`);
  }
  return isTruthy(args[0]);
}

/**
 * int(x[, base]) -- Convert to integer.
 *
 * Supports: int, float (truncates), string (parses), bool.
 * Optional second arg specifies base for string conversion.
 */
export function builtinInt(...args: VMValue[]): VMValue {
  if (args.length < 1 || args.length > 2) {
    throw new VMTypeError(`int() takes 1 or 2 arguments (${args.length} given)`);
  }
  const value = args[0];
  if (args.length === 2) {
    const base = args[1] as number;
    if (typeof value !== "string") {
      throw new VMTypeError("int() can't convert non-string with explicit base");
    }
    return parseInt(value, base);
  }
  if (typeof value === "boolean") return value ? 1 : 0;
  if (typeof value === "number") return Math.trunc(value);
  if (typeof value === "string") {
    const parsed = parseInt(value, 10);
    if (isNaN(parsed)) {
      throw new VMTypeError(`invalid literal for int(): '${value}'`);
    }
    return parsed;
  }
  throw new VMTypeError(
    `int() argument must be a string or a number, not '${typeof value}'`,
  );
}

/**
 * float(x) -- Convert to float.
 */
export function builtinFloat(...args: VMValue[]): VMValue {
  if (args.length !== 1) {
    throw new VMTypeError(`float() takes exactly 1 argument (${args.length} given)`);
  }
  const value = args[0];
  if (typeof value === "number") return value;
  if (typeof value === "string") {
    const parsed = parseFloat(value);
    if (isNaN(parsed)) {
      throw new VMTypeError(`could not convert string to float: '${value}'`);
    }
    return parsed;
  }
  throw new VMTypeError(
    `float() argument must be a string or a number, not '${typeof value}'`,
  );
}

/**
 * str(x) -- Convert to string representation.
 */
export function builtinStr(...args: VMValue[]): VMValue {
  if (args.length !== 1) {
    throw new VMTypeError(`str() takes exactly 1 argument (${args.length} given)`);
  }
  const value = args[0];
  if (value === null || value === undefined) return "None";
  if (typeof value === "boolean") return value ? "True" : "False";
  if (typeof value === "string") return value;
  if (typeof value === "number") return String(value);
  if (Array.isArray(value)) {
    return `[${value.map((v) => starlarkValueRepr(v)).join(", ")}]`;
  }
  return String(value);
}

// =========================================================================
// Collection functions
// =========================================================================

/**
 * len(x) -- Return the length of a collection or string.
 */
export function builtinLen(...args: VMValue[]): VMValue {
  if (args.length !== 1) {
    throw new VMTypeError(`len() takes exactly 1 argument (${args.length} given)`);
  }
  const value = args[0];
  if (typeof value === "string") return value.length;
  if (Array.isArray(value)) return value.length;
  if (typeof value === "object" && value !== null) {
    return Object.keys(value as Record<string, unknown>).length;
  }
  throw new VMTypeError(`object of type '${typeof value}' has no len()`);
}

/**
 * list(x) -- Convert an iterable to a list.
 */
export function builtinList(...args: VMValue[]): VMValue {
  if (args.length === 0) return [];
  if (args.length !== 1) {
    throw new VMTypeError(`list() takes at most 1 argument (${args.length} given)`);
  }
  const value = args[0];
  if (Array.isArray(value)) return [...value];
  if (typeof value === "string") return [...value];
  throw new VMTypeError(`cannot convert '${typeof value}' to list`);
}

/**
 * dict() -- Create a new dictionary.
 *
 * Called with no args: returns empty dict.
 * Called with an iterable of [key, value] pairs: creates dict from pairs.
 */
export function builtinDict(...args: VMValue[]): VMValue {
  if (args.length === 0) return {} as unknown as VMValue;
  if (args.length === 1) {
    const value = args[0];
    if (Array.isArray(value)) {
      const dict: Record<string, VMValue> = {};
      for (const pair of value) {
        const [k, v] = pair as [VMValue, VMValue];
        dict[String(k)] = v;
      }
      return dict as unknown as VMValue;
    }
    if (typeof value === "object" && value !== null) {
      return { ...(value as Record<string, VMValue>) } as unknown as VMValue;
    }
  }
  throw new VMTypeError(`dict() takes at most 1 argument (${args.length} given)`);
}

/**
 * tuple(x) -- Convert an iterable to a tuple.
 *
 * In JavaScript, we represent tuples as regular arrays (Starlark enforces
 * immutability at the language level).
 */
export function builtinTuple(...args: VMValue[]): VMValue {
  if (args.length === 0) return [];
  if (args.length !== 1) {
    throw new VMTypeError(`tuple() takes at most 1 argument (${args.length} given)`);
  }
  const value = args[0];
  if (Array.isArray(value)) return [...value];
  if (typeof value === "string") return [...value];
  throw new VMTypeError(`cannot convert '${typeof value}' to tuple`);
}

/**
 * range(stop) or range(start, stop[, step]) -- Return a list of integers.
 *
 * Unlike Python's lazy range(), Starlark's range() returns a concrete list.
 * This is because Starlark forbids lazy evaluation for determinism.
 */
export function builtinRange(...args: VMValue[]): VMValue {
  let start: number, stop: number, step: number;

  if (args.length === 1) {
    start = 0;
    stop = args[0] as number;
    step = 1;
  } else if (args.length === 2) {
    start = args[0] as number;
    stop = args[1] as number;
    step = 1;
  } else if (args.length === 3) {
    start = args[0] as number;
    stop = args[1] as number;
    step = args[2] as number;
  } else {
    throw new VMTypeError(`range() takes 1 to 3 arguments (${args.length} given)`);
  }

  if (step === 0) {
    throw new VMTypeError("range() arg 3 must not be zero");
  }

  const result: number[] = [];
  if (step > 0) {
    for (let i = start; i < stop; i += step) result.push(i);
  } else {
    for (let i = start; i > stop; i += step) result.push(i);
  }
  return result;
}

/**
 * sorted(x[, reverse]) -- Return a new sorted list.
 */
export function builtinSorted(...args: VMValue[]): VMValue {
  if (args.length < 1 || args.length > 2) {
    throw new VMTypeError(`sorted() takes 1 or 2 arguments (${args.length} given)`);
  }
  const iterable = args[0] as VMValue[];
  const reverse = args.length > 1 ? isTruthy(args[1]) : false;

  const result = [...iterable].sort((a, b) => {
    if (typeof a === "number" && typeof b === "number") return a - b;
    if (typeof a === "string" && typeof b === "string") return a.localeCompare(b);
    return 0;
  });
  if (reverse) result.reverse();
  return result;
}

/**
 * reversed(x) -- Return a reversed list.
 */
export function builtinReversed(...args: VMValue[]): VMValue {
  if (args.length !== 1) {
    throw new VMTypeError(`reversed() takes exactly 1 argument (${args.length} given)`);
  }
  const value = args[0];
  if (Array.isArray(value)) return [...value].reverse();
  if (typeof value === "string") return [...value].reverse().join("");
  throw new VMTypeError(`cannot reverse '${typeof value}'`);
}

/**
 * enumerate(x[, start]) -- Return list of [index, value] pairs.
 */
export function builtinEnumerate(...args: VMValue[]): VMValue {
  if (args.length < 1 || args.length > 2) {
    throw new VMTypeError(`enumerate() takes 1 or 2 arguments (${args.length} given)`);
  }
  const iterable = args[0] as VMValue[];
  const start = args.length > 1 ? (args[1] as number) : 0;
  return iterable.map((v, i) => [start + i, v]);
}

/**
 * zip(*iterables) -- Return list of tuples.
 */
export function builtinZip(...args: VMValue[]): VMValue {
  if (args.length === 0) return [];
  const iterables = args.map((a) => a as VMValue[]);
  const minLen = Math.min(...iterables.map((a) => a.length));
  const result: VMValue[][] = [];
  for (let i = 0; i < minLen; i++) {
    result.push(iterables.map((a) => a[i]));
  }
  return result;
}

// =========================================================================
// Logic and math functions
// =========================================================================

/**
 * min(x, y, ...) or min(iterable) -- Return the smallest element.
 */
export function builtinMin(...args: VMValue[]): VMValue {
  if (args.length === 1 && Array.isArray(args[0])) {
    const arr = args[0] as VMValue[];
    if (arr.length === 0) throw new VMTypeError("min() arg is an empty sequence");
    return arr.reduce((a, b) => ((a as number) < (b as number) ? a : b));
  }
  if (args.length === 0) throw new VMTypeError("min expected at least 1 argument, got 0");
  return args.reduce((a, b) => ((a as number) < (b as number) ? a : b));
}

/**
 * max(x, y, ...) or max(iterable) -- Return the largest element.
 */
export function builtinMax(...args: VMValue[]): VMValue {
  if (args.length === 1 && Array.isArray(args[0])) {
    const arr = args[0] as VMValue[];
    if (arr.length === 0) throw new VMTypeError("max() arg is an empty sequence");
    return arr.reduce((a, b) => ((a as number) > (b as number) ? a : b));
  }
  if (args.length === 0) throw new VMTypeError("max expected at least 1 argument, got 0");
  return args.reduce((a, b) => ((a as number) > (b as number) ? a : b));
}

/**
 * abs(x) -- Return the absolute value.
 */
export function builtinAbs(...args: VMValue[]): VMValue {
  if (args.length !== 1) {
    throw new VMTypeError(`abs() takes exactly 1 argument (${args.length} given)`);
  }
  return Math.abs(args[0] as number);
}

/**
 * all(iterable) -- Return True if all elements are truthy.
 */
export function builtinAll(...args: VMValue[]): VMValue {
  if (args.length !== 1) {
    throw new VMTypeError(`all() takes exactly 1 argument (${args.length} given)`);
  }
  const iterable = args[0] as VMValue[];
  return iterable.every((v) => isTruthy(v));
}

/**
 * any(iterable) -- Return True if any element is truthy.
 */
export function builtinAny(...args: VMValue[]): VMValue {
  if (args.length !== 1) {
    throw new VMTypeError(`any() takes exactly 1 argument (${args.length} given)`);
  }
  const iterable = args[0] as VMValue[];
  return iterable.some((v) => isTruthy(v));
}

// =========================================================================
// String/utility functions
// =========================================================================

/**
 * repr(x) -- Return a string representation (with quotes for strings).
 */
export function builtinRepr(...args: VMValue[]): VMValue {
  if (args.length !== 1) {
    throw new VMTypeError(`repr() takes exactly 1 argument (${args.length} given)`);
  }
  return starlarkValueRepr(args[0]);
}

/**
 * hasattr(x, name) -- Return True if x has the named attribute.
 */
export function builtinHasattr(...args: VMValue[]): VMValue {
  if (args.length !== 2) {
    throw new VMTypeError(`hasattr() takes exactly 2 arguments (${args.length} given)`);
  }
  const obj = args[0];
  const name = args[1] as string;
  if (typeof obj === "object" && obj !== null) {
    return name in (obj as Record<string, unknown>);
  }
  return false;
}

/**
 * getattr(x, name[, default]) -- Get a named attribute.
 */
export function builtinGetattr(...args: VMValue[]): VMValue {
  if (args.length < 2 || args.length > 3) {
    throw new VMTypeError(`getattr() takes 2 or 3 arguments (${args.length} given)`);
  }
  const obj = args[0];
  const name = args[1] as string;
  if (typeof obj === "object" && obj !== null) {
    const record = obj as Record<string, VMValue>;
    if (name in record) return record[name];
  }
  if (args.length === 3) return args[2];
  throw new VMTypeError(`object has no attribute '${name}'`);
}

/**
 * print(*args) -- Print arguments.
 *
 * In Starlark, print() always returns None. The output is captured
 * by the VM's output list rather than going to stdout.
 * The actual capture happens in the factory (createStarlarkVM) where
 * print is overridden with a closure that captures to vm.output.
 */
export function builtinPrint(..._args: VMValue[]): VMValue {
  return null;
}

// =========================================================================
// Registration helper
// =========================================================================

/**
 * Return a record mapping built-in function names to their implementations.
 *
 * This is used by ``createStarlarkVM()`` to register all built-ins
 * with the GenericVM.
 *
 * The 23 built-in functions are:
 *
 * | Category    | Functions                                         |
 * |-------------|---------------------------------------------------|
 * | Type        | type, bool, int, float, str                       |
 * | Collection  | len, list, dict, tuple, range, sorted, reversed,  |
 * |             | enumerate, zip                                    |
 * | Logic/Math  | min, max, abs, all, any                           |
 * | String/Util | repr, hasattr, getattr                            |
 * | I/O         | print                                             |
 */
export function getAllBuiltins(): Record<string, (...args: VMValue[]) => VMValue> {
  return {
    // Type functions
    type: builtinType,
    bool: builtinBool,
    int: builtinInt,
    float: builtinFloat,
    str: builtinStr,
    // Collection functions
    len: builtinLen,
    list: builtinList,
    dict: builtinDict,
    tuple: builtinTuple,
    range: builtinRange,
    sorted: builtinSorted,
    reversed: builtinReversed,
    enumerate: builtinEnumerate,
    zip: builtinZip,
    // Logic and math
    min: builtinMin,
    max: builtinMax,
    abs: builtinAbs,
    all: builtinAll,
    any: builtinAny,
    // String/utility
    repr: builtinRepr,
    hasattr: builtinHasattr,
    getattr: builtinGetattr,
    // I/O
    print: builtinPrint,
  };
}
