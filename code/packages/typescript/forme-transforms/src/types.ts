/**
 * types.ts — shared signatures for the sequence transforms.
 *
 * All helpers in this package are generic over `T` (the item type).
 * The Forme pipeline uses them for:
 *
 *   - lists of `Document` (page-level: filter drafts, sort by date)
 *   - lists of `IndexItem` (archive-level: limit to 100 newest)
 *   - lists of `ContentNode` (AST-level: extract headings, count tables)
 *
 * Keeping these signatures in one place gives every helper the same
 * shape so callers can reason about them as a single algebra rather
 * than 12 ad-hoc functions.
 *
 * @module types
 */

/** Standard predicate — true keeps the item, false drops it. */
export type Predicate<T> = (item: T, index: number) => boolean;

/** Standard projection — maps `T` to `U`. */
export type Mapper<T, U> = (item: T, index: number) => U;

/** Many-to-many projection — flatMap building block. */
export type FlatMapper<T, U> = (item: T, index: number) => readonly U[];

/**
 * Sort key extractor.  The returned key is compared with `<` / `>`
 * after coercion, so it should be a string, number, bigint, or
 * `null` (treated as "absent" — sorted to the end regardless of
 * direction).
 */
export type KeyFn<T, K> = (item: T) => K;

/** Sort direction — `"asc"` (default) or `"desc"`. */
export type SortDirection = "asc" | "desc";

/**
 * Result of `partition` — items where the predicate returned true
 * land in `yes`; the rest in `no`.  Both arrays preserve input
 * order.
 */
export interface Partition<T> {
  readonly yes: readonly T[];
  readonly no: readonly T[];
}

/**
 * A single step in a `pipe` chain.  Each step takes a readonly
 * input array and returns a (possibly differently-typed) readonly
 * output array.  Pipelines never mutate.
 */
export type PipeStep<T, U> = (items: readonly T[]) => readonly U[];
