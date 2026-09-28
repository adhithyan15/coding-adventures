// One lazy loader per committed stroke-order filmstrip. This module is itself
// dynamically imported by figures.ts: keeping the glob here keeps its hundreds
// of path strings and import functions out of the first-paint graph.
const FILMSTRIP_LOADERS = import.meta.glob(
  "../../../../learning/human-languages/*/book/figures/*-filmstrip.svg",
  { query: "?url", import: "default" },
) as Record<string, () => Promise<string>>;

/** Load one exact lesson-owned filmstrip, or null when its glyph has no cited ductus. */
export async function loadFilmstrip(
  language: string,
  lessonId: string,
): Promise<string | null> {
  const suffix =
    `/human-languages/${language}/book/figures/${lessonId}-filmstrip.svg`;
  const entry = Object.entries(FILMSTRIP_LOADERS).find(([path]) =>
    path.replaceAll("\\", "/").endsWith(suffix),
  );
  return entry === undefined ? null : entry[1]();
}
