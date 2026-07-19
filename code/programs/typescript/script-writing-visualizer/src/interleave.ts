// ---------------------------------------------------------------------------
// interleave.ts — the pure glue for cross-script practice (HL02 interleaving).
//
// The scheduler (scheduler.ts) is generic: it schedules items keyed by a single
// numeric index. To drill *all scripts mixed together* — the interleaving that
// HL02 says "forces discrimination and transfers better" — we lay every letter
// of every script into ONE flat pool, hand that pool's length to the scheduler,
// and map the index it picks back to a (script, letter) pair.
//
// The pool is built ROUND-ROBIN across scripts (letter 0 of every script, then
// letter 1 of every script, …) so the very first pass already alternates
// scripts, not "all of Cyrillic, then all of Hebrew." Pure and deterministic.
// ---------------------------------------------------------------------------

/** One item in the combined pool: which script, and which letter within it. */
export interface PoolEntry {
  scriptIndex: number;
  letterIndex: number;
}

/**
 * Build the combined, round-robin-interleaved pool from each script's letter
 * count. `counts[s]` is the number of letters in script `s`.
 *
 *   buildPool([2, 3]) →
 *     [ {0,0}, {1,0}, {0,1}, {1,1}, {1,2} ]
 *     // letter 0 of both, letter 1 of both, then script 1's extra letter 2
 *
 * Every (script, letter) appears exactly once. Scripts with fewer letters simply
 * drop out of the later rounds (ragged is fine).
 */
export function buildPool(counts: number[]): PoolEntry[] {
  const max = counts.reduce((m, c) => Math.max(m, c), 0);
  const pool: PoolEntry[] = [];
  for (let letterIndex = 0; letterIndex < max; letterIndex++) {
    for (let scriptIndex = 0; scriptIndex < counts.length; scriptIndex++) {
      if (letterIndex < (counts[scriptIndex] ?? 0)) {
        pool.push({ scriptIndex, letterIndex });
      }
    }
  }
  return pool;
}

/** Total letters across all scripts (the combined pool size). */
export function poolSize(counts: number[]): number {
  return counts.reduce((sum, c) => sum + Math.max(0, c), 0);
}
