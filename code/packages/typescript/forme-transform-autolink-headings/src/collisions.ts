/**
 * collisions.ts — deterministic resolution of repeated slug candidates.
 *
 * Two headings can produce the same slug — e.g. `## Setup` appears
 * twice in a doc, or `## Step 1` and `## Step.1!` both reduce to
 * `step-1`.  HTML `id`s must be unique within a document; the
 * algorithm here disambiguates by appending `-2`, `-3`, ... to
 * later occurrences:
 *
 *   Input:  ["setup", "setup", "intro", "setup"]
 *   Output: ["setup", "setup-2", "intro", "setup-3"]
 *
 * **First occurrence is unsuffixed** — preserves the most-likely
 * link target for external references that copy a heading's
 * slug from elsewhere in the docs.  Numbering matches GitHub's
 * behaviour exactly.
 *
 * **Reserved suffixes.**  If a heading naturally produces a slug
 * like `setup-2` and is followed by another `setup`, the resolver
 * skips taken suffixes:
 *
 *   Input:  ["setup", "setup-2", "setup"]
 *   Output: ["setup", "setup-2", "setup-3"]
 *                                   ^ not "setup-2" (taken)
 *                                   ^ jumps straight to -3
 *
 * The implementation maintains a `Set` of taken slugs and a
 * `Map<baseSlug, nextCandidate>` counter so the next-candidate
 * search is amortised O(1).
 *
 * @module collisions
 */

/**
 * Resolve a stream of slug candidates so every emitted slug is
 * unique.  Returns a new array of the same length; input order
 * preserved.
 *
 * Determinism: given the same input array, the output is byte-
 * identical.  Two callers running this in parallel with the same
 * input get the same result.  Input is never mutated.
 */
export function resolveCollisions(candidates: readonly string[]): string[] {
  const taken = new Set<string>();
  /**
   * For each base slug, the next integer suffix to *try*.  Starts
   * at 2 for the second occurrence and increments past any taken
   * suffixes so we never test an already-claimed slug twice.
   */
  const nextSuffix = new Map<string, number>();

  const out: string[] = new Array(candidates.length);
  for (let i = 0; i < candidates.length; i++) {
    const base = candidates[i]!;
    if (!taken.has(base)) {
      taken.add(base);
      out[i] = base;
      continue;
    }
    // Collision — pick the smallest available `base-N`.
    let n = nextSuffix.get(base) ?? 2;
    let candidate: string;
    // The `taken.has` check inside the loop also catches the case
    // where the *natural* slug of an earlier heading happened to
    // be e.g. `setup-2` already (see module docstring example).
    // Walk past those without redoing them.
    while (true) {
      candidate = `${base}-${n}`;
      if (!taken.has(candidate)) break;
      n++;
    }
    taken.add(candidate);
    nextSuffix.set(base, n + 1);
    out[i] = candidate;
  }
  return out;
}
