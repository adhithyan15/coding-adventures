/**
 * @coding-adventures/forme-transforms
 *
 * Pure-data sequence helpers for the Forme pipeline (FM00 v0).
 *
 * Filter / map / sort / limit / partition / group / unique / pipe
 * over arrays of anything — `Document[]` (pages), `IndexItem[]`
 * (archive), `ContentNode[]` (AST), or your own types.  Every
 * helper is:
 *
 *   - **Pure** — input arrays are never mutated.
 *   - **Deterministic** — same input → byte-identical output
 *     (subject to deterministic `keyFn` / `predicate`).
 *   - **Capability-free** — no I/O, no network, no env, no shell.
 *
 * ```ts
 * import { pipe, filter, sortBy, take } from "@coding-adventures/forme-transforms";
 *
 * const recentPublished = pipe(posts,
 *   (xs) => filter(xs, (p) => !p.draft),
 *   (xs) => sortBy(xs, (p) => p.pubDate, "desc"),
 *   (xs) => take(xs, 10),
 * );
 * ```
 *
 * Sits alongside `forme-feeds` / `forme-opengraph` /
 * `forme-index-renderer` as the fourth FM00 v0 stage package.
 * Renderers and collectors compose these helpers; the pipeline
 * never has to reach for `Array.prototype` discipline directly.
 *
 * @module index
 */

export { filter, map, flatMap } from "./filter-map.js";
export { take, drop, slice, chunk } from "./slice.js";
export { sortBy, sortBy2 } from "./sort.js";
export { partition, groupBy, unique } from "./group.js";
export { pipe } from "./pipe.js";
export type {
  Predicate,
  Mapper,
  FlatMapper,
  KeyFn,
  SortDirection,
  Partition,
  PipeStep,
} from "./types.js";
