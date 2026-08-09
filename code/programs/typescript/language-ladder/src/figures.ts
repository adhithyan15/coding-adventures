const GENERATED_FIGURES = import.meta.glob(
  "../../../../learning/human-languages/*/book/figures/*.svg",
  { eager: true, query: "?url", import: "default" },
) as Record<string, string>;

/** Resolve the same committed SVG that the book build converts to PDF. */
export function generatedFigureUrl(language: string, source: string): string {
  const match = /^figures\/([A-Za-z0-9._-]+\.svg)$/.exec(source);
  if (!/^[a-z0-9-]+$/.test(language) || !match) {
    throw new Error(`unsafe generated lesson figure '${language}:${source}'`);
  }
  const suffix = `/human-languages/${language}/book/figures/${match[1]}`;
  const entry = Object.entries(GENERATED_FIGURES).find(([path]) =>
    path.replaceAll("\\", "/").endsWith(suffix),
  );
  if (!entry) throw new Error(`missing generated lesson figure '${language}:${source}'`);
  return entry[1];
}
