/**
 * The generated narration, kept OUT of the eager chunk (HL-C87, HL10 §10.2).
 *
 * One JSON per chapter per track — 344 files for Spanish alone. Voice mode
 * needs exactly one of them at a time, so they load on demand, the same way
 * `lesson-sources.ts` handles lesson bodies and for the same reason: the map
 * itself is code, and a static import would put all of it on first paint.
 *
 * Nothing else may import this module statically.
 */
const NARRATION_LOADERS = import.meta.glob(
  "../../../../learning/human-languages/*/narration/*.json",
  { import: "default" },
) as Record<string, () => Promise<unknown>>;

/** Load one chapter's narration, or null when there is none for that chapter. */
export async function loadNarration(language: string, chapter: number): Promise<unknown | null> {
  // Chapter files are zero-padded to two digits and unpadded beyond that, so
  // try both rather than guessing which era a track is in.
  const padded = String(chapter).padStart(2, "0");
  const suffixes = [
    `/${language}/narration/ch${padded}.json`,
    `/${language}/narration/ch${chapter}.json`,
  ];
  for (const [path, load] of Object.entries(NARRATION_LOADERS)) {
    const normalized = path.replaceAll("\\", "/");
    if (suffixes.some((suffix) => normalized.endsWith(suffix))) {
      try {
        return await load();
      } catch {
        return null;
      }
    }
  }
  return null;
}
