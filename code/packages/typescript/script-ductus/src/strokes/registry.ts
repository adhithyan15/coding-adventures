import type { LetterDuctus } from "../strokes.ts";

export type DuctusEntry = readonly [key: string, letter: LetterDuctus];

export interface DuctusOwner {
  readonly owner: string;
  readonly entries: readonly DuctusEntry[];
}

/** Assemble fixed owner sequences without allowing a later shard to win silently. */
export function assembleDuctusRegistry(
  owners: readonly DuctusOwner[],
): Record<string, LetterDuctus> {
  const registry: Record<string, LetterDuctus> = {};
  const claimedBy = new Map<string, string>();

  for (const { owner, entries } of owners) {
    for (const [key, letter] of entries) {
      const previousOwner = claimedBy.get(key);
      if (previousOwner === owner) {
        throw new Error(`Script Ductus owner ${owner} repeats key ${key}`);
      }
      if (previousOwner !== undefined) {
        throw new Error(
          `Script Ductus owners ${previousOwner} and ${owner} both claim key ${key}`,
        );
      }

      // Define rather than assign so even a hostile `__proto__` key remains an
      // ordinary own data property and cannot change the registry's prototype.
      Object.defineProperty(registry, key, {
        configurable: true,
        enumerable: true,
        value: letter,
        writable: true,
      });
      claimedBy.set(key, owner);
    }
  }

  return registry;
}
