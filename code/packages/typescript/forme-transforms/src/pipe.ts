/**
 * pipe.ts — left-to-right function composition over arrays.
 *
 * `pipe` is the readability win that justifies every other helper
 * having a uniform `(items, ...args) => result` shape: callers can
 * partially-apply each step and feed the chain into `pipe` to read
 * top-to-bottom rather than inside-out.
 *
 * ```ts
 * // Without pipe — inside-out, parens-heavy.
 * const recent = take(
 *   sortBy(
 *     filter(posts, (p) => !p.draft),
 *     (p) => p.pubDate,
 *     "desc",
 *   ),
 *   10,
 * );
 *
 * // With pipe — top-to-bottom narrative.
 * const recent = pipe(posts,
 *   (xs) => filter(xs, (p) => !p.draft),
 *   (xs) => sortBy(xs, (p) => p.pubDate, "desc"),
 *   (xs) => take(xs, 10),
 * );
 * ```
 *
 * The signature is intentionally untyped between steps (each step
 * sees `readonly unknown[]` internally), with overloads providing
 * type inference for the common 1-5 step cases.  Past five steps,
 * either break the pipeline into a named intermediate or accept
 * that the final type is `unknown[]` and assert at the boundary.
 *
 * @module pipe
 */

import type { PipeStep } from "./types.js";

// Overload set — TypeScript picks the most specific match based
// on arity, giving inference for chains up to length 5.

export function pipe<A>(items: readonly A[]): readonly A[];
export function pipe<A, B>(items: readonly A[], s1: PipeStep<A, B>): readonly B[];
export function pipe<A, B, C>(
  items: readonly A[],
  s1: PipeStep<A, B>,
  s2: PipeStep<B, C>,
): readonly C[];
export function pipe<A, B, C, D>(
  items: readonly A[],
  s1: PipeStep<A, B>,
  s2: PipeStep<B, C>,
  s3: PipeStep<C, D>,
): readonly D[];
export function pipe<A, B, C, D, E>(
  items: readonly A[],
  s1: PipeStep<A, B>,
  s2: PipeStep<B, C>,
  s3: PipeStep<C, D>,
  s4: PipeStep<D, E>,
): readonly E[];
export function pipe<A, B, C, D, E, F>(
  items: readonly A[],
  s1: PipeStep<A, B>,
  s2: PipeStep<B, C>,
  s3: PipeStep<C, D>,
  s4: PipeStep<D, E>,
  s5: PipeStep<E, F>,
): readonly F[];

/**
 * Implementation — applies each step in order; each step's output
 * becomes the next step's input.  Returns the original items
 * unchanged if no steps were supplied (useful in conditional
 * pipeline-building code where some steps are toggled off).
 */
export function pipe(
  items: readonly unknown[],
  ...steps: ReadonlyArray<PipeStep<unknown, unknown>>
): readonly unknown[] {
  let current: readonly unknown[] = items;
  for (let i = 0; i < steps.length; i++) {
    current = steps[i]!(current);
  }
  return current;
}
