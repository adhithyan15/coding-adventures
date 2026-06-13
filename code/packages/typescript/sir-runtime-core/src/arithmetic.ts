/**
 * SIR arithmetic and comparison.
 *
 * These differ from JavaScript's native operators in two ways that justify
 * a runtime helper rather than a bare `a + b`:
 *
 * 1. **Variadic** — `add`/`sub`/`mul` fold over any number of arguments
 *    (`add()` is `0`, `mul()` is `1`), matching the SIR builtin contract.
 * 2. **Truncating integer division** — `div` truncates toward zero
 *    (`div(7, 2) === 3`, `div(-7, 2) === -3`), where JS `/` yields a float.
 */

import type { Val } from "./values.js";

const num = (v: Val): number => v as number;

/** Variadic sum; `add()` is `0`. */
export function add(...args: Val[]): Val {
  let total = 0;
  for (const a of args) {
    total += num(a);
  }
  return total;
}

/** Variadic difference; `sub(x)` negates, `sub()` is `0`. */
export function sub(...args: Val[]): Val {
  if (args.length === 0) {
    return 0;
  }
  if (args.length === 1) {
    return -num(args[0]!);
  }
  let acc = num(args[0]!);
  for (let i = 1; i < args.length; i++) {
    acc -= num(args[i]!);
  }
  return acc;
}

/** Variadic product; `mul()` is `1`. */
export function mul(...args: Val[]): Val {
  let acc = 1;
  for (const a of args) {
    acc *= num(a);
  }
  return acc;
}

/** Variadic quotient with truncating-integer division (toward zero). */
export function div(...args: Val[]): Val {
  if (args.length === 0) {
    return 0;
  }
  let acc = num(args[0]!);
  for (let i = 1; i < args.length; i++) {
    acc = Math.trunc(acc / num(args[i]!));
  }
  return acc;
}

/** Less-than. */
export function lt(a: Val, b: Val): boolean {
  return num(a) < num(b);
}

/** Greater-than. */
export function gt(a: Val, b: Val): boolean {
  return num(a) > num(b);
}
