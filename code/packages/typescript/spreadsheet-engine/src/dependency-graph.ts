/**
 * # The dependency graph — the reusable heart of recalc
 *
 * When you type `=A1+B1` into cell C1, you create a *dependency*: C1 depends on
 * A1 and B1. Change A1, and C1 must be recomputed. Change C1, and anything that
 * referenced C1 must be recomputed. Tracking these relationships — and the order
 * to recompute them in — is the entire job of this file.
 *
 * We keep the graph in **two directions at once**, exactly as the spec (§5)
 * prescribes, because both queries are hot:
 *
 * ```text
 *   edgesOut[C1] = {A1, B1}   "the cells C1 reads"   ← used to rebuild edges
 *   edgesIn[A1]  = {C1}       "the cells that read A1" ← used to find the dirty set
 * ```
 *
 *   - After editing A1, "what must I recompute?" = transitive closure of edgesIn.
 *   - After re-parsing C1's formula, "register its deps" = push into both maps.
 *
 * Nodes are identified by the string key from `addressKey(addr)` so we can use
 * plain `Map`/`Set` without worrying about object identity.
 *
 * ## Why not reuse `@coding-adventures/directed-graph`?
 *
 * That package is excellent and we *do* depend on it transitively (excel-parser
 * pulls it in), but its `topologicalSort()` sorts the **whole** graph and throws
 * a `CycleError` the moment *any* cycle exists anywhere. Incremental recalc needs
 * two things it doesn't offer:
 *
 *   1. Topologically order only a **dirty subset** of cells (one edit shouldn't
 *      re-sort 10 000 untouched cells).
 *   2. Recover gracefully from cycles by marking *just* the cells in the cycle
 *      as `#CIRC!`, while still evaluating everything else — not aborting the
 *      whole recalc.
 *
 * Those are spreadsheet-specific recalc concerns, so we implement a small,
 * purpose-built graph here (adjacency maps + a subgraph Kahn topological sort
 * + cycle detection). It is a few dozen lines and keeps the recalc semantics
 * legible. (See the README "Design notes" section for the full rationale.)
 */

import type { CellAddress } from "./address.js";
import { addressKey } from "./address.js";

export class DependencyGraph {
  /** cell → the set of cells it depends on (reads). Keyed by addressKey. */
  private readonly edgesOut = new Map<string, Set<string>>();
  /** cell → the set of cells that depend on it (are read by). */
  private readonly edgesIn = new Map<string, Set<string>>();

  /** Replace the full out-edge set of `cell` with `deps`.
   *
   * Called every time a formula is (re)entered. We first tear down the old
   * edges (so a formula that used to read A1 but no longer does stops being
   * woken up by A1), then add the new ones. Both directions stay consistent. */
  setDependencies(cell: CellAddress, deps: CellAddress[]): void {
    const key = addressKey(cell);

    // 1. Drop existing out-edges and their mirror in-edges.
    const old = this.edgesOut.get(key);
    if (old) {
      for (const depKey of old) {
        this.edgesIn.get(depKey)?.delete(key);
      }
    }

    // 2. Install the new out-edges (deduped via a Set) and mirror them.
    const newOut = new Set<string>();
    for (const dep of deps) {
      const depKey = addressKey(dep);
      newOut.add(depKey);
      let inSet = this.edgesIn.get(depKey);
      if (!inSet) {
        inSet = new Set<string>();
        this.edgesIn.set(depKey, inSet);
      }
      inSet.add(key);
    }
    this.edgesOut.set(key, newOut);
  }

  /** Remove a cell entirely from the graph (used when a cell is cleared). */
  removeCell(cell: CellAddress): void {
    const key = addressKey(cell);
    const out = this.edgesOut.get(key);
    if (out) {
      for (const depKey of out) this.edgesIn.get(depKey)?.delete(key);
    }
    this.edgesOut.delete(key);
    // Note: we intentionally keep edgesIn[key] — other cells may still point at
    // this (now-empty) cell, and that reference should still wake them up.
  }

  /** The set of cells `cell` directly reads. */
  dependenciesOf(key: string): ReadonlySet<string> {
    return this.edgesOut.get(key) ?? EMPTY_SET;
  }

  /** The set of cells that directly read `cell`. */
  dependentsOf(key: string): ReadonlySet<string> {
    return this.edgesIn.get(key) ?? EMPTY_SET;
  }

  /**
   * Compute the **dirty set**: `seeds` plus every cell transitively downstream
   * of them (i.e. reachable by following edgesIn). This is everything that
   * *might* need recomputing after the seed cells changed.
   *
   * Implemented as a breadth-first walk over the reverse edges.
   */
  dirtySet(seeds: CellAddress[]): Set<string> {
    const dirty = new Set<string>();
    const queue: string[] = [];
    for (const s of seeds) {
      const k = addressKey(s);
      if (!dirty.has(k)) {
        dirty.add(k);
        queue.push(k);
      }
    }
    while (queue.length > 0) {
      const cur = queue.shift()!;
      for (const dependent of this.dependentsOf(cur)) {
        if (!dirty.has(dependent)) {
          dirty.add(dependent);
          queue.push(dependent);
        }
      }
    }
    return dirty;
  }

  /**
   * Topologically order the subgraph induced by `subset`, considering only
   * edges *within* the subset. Returns:
   *
   *   - `order`: the cells in a valid evaluation order (dependencies first),
   *     and
   *   - `cyclic`: the cells that could not be ordered because they take part in
   *     a cycle (or depend on one).
   *
   * This is **Kahn's algorithm** restricted to the subset. We count, for each
   * cell, how many of its dependencies are *also in the subset* (its in-degree
   * within the subgraph). Cells with in-degree 0 are ready to evaluate; as we
   * "remove" each one we decrement its dependents' counts. Whatever never
   * reaches in-degree 0 is exactly the set of cells tangled in a cycle — those
   * become `#CIRC!`.
   *
   * ### Worked example
   *
   * ```text
   *   subset = {A2, A3}      A2 = A3 + 1,  A3 = 5   (A3 has no in-subset deps)
   *   in-degree:  A3 → 0,  A2 → 1
   *   queue starts [A3] → emit A3, decrement A2 → 0 → queue [A2] → emit A2
   *   order = [A3, A2], cyclic = {}
   * ```
   */
  topoOrderSubset(subset: ReadonlySet<string>): { order: string[]; cyclic: Set<string> } {
    // in-degree restricted to edges whose *source dependency* is in the subset
    const inDegree = new Map<string, number>();
    for (const key of subset) {
      let deg = 0;
      for (const dep of this.dependenciesOf(key)) {
        if (subset.has(dep)) deg++;
      }
      inDegree.set(key, deg);
    }

    // Seed the queue with everything that has no in-subset dependency. Sorting
    // makes the output deterministic when several cells are simultaneously
    // ready — important for reproducible recalc (spec §6).
    const queue = [...inDegree.entries()]
      .filter(([, d]) => d === 0)
      .map(([k]) => k)
      .sort();

    const order: string[] = [];
    while (queue.length > 0) {
      const cur = queue.shift()!;
      order.push(cur);
      // "Remove" cur: any subset cell that depends on cur loses one in-edge.
      const readyNow: string[] = [];
      for (const dependent of this.dependentsOf(cur)) {
        if (!subset.has(dependent)) continue;
        const d = (inDegree.get(dependent) ?? 0) - 1;
        inDegree.set(dependent, d);
        if (d === 0) readyNow.push(dependent);
      }
      // keep determinism: merge the newly-ready nodes in sorted order
      if (readyNow.length > 0) {
        queue.push(...readyNow);
        queue.sort();
      }
    }

    // Anything not emitted is part of (or downstream of) a cycle.
    const cyclic = new Set<string>();
    for (const key of subset) {
      if (!order.includes(key)) cyclic.add(key);
    }
    return { order, cyclic };
  }
}

const EMPTY_SET: ReadonlySet<string> = new Set<string>();
