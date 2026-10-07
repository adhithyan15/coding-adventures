// Stroke-order filmstrips (`*-filmstrip.svg`, HL-C443) are excluded from THIS
// eager map. There are several hundred of them, and putting their URL loaders
// here pushed the first-paint chunk over the 500 kB `check:bundle` budget.
// `generatedFilmstripUrl` reaches a separate, dynamically imported source map,
// so opening an ordinary lesson still pays for none of those paths.
const GENERATED_FIGURES = import.meta.glob(
  [
    "../../../../learning/human-languages/*/book/figures/*.svg",
    "!../../../../learning/human-languages/*/book/figures/*-filmstrip.svg",
  ],
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

/** Resolve a writing lesson's canonical filmstrip only when the lesson opens. */
export async function generatedFilmstripUrl(
  language: string,
  lessonId: string,
): Promise<string | null> {
  if (!/^[a-z0-9-]+$/.test(language) || !/^[A-Za-z0-9._-]+$/.test(lessonId)) {
    throw new Error(`unsafe generated filmstrip '${language}:${lessonId}'`);
  }
  const { loadFilmstrip } = await import("./filmstrip-sources.ts");
  return loadFilmstrip(language, lessonId);
}

// The words under a writing lesson's filmstrip. The book captions the same
// strip from the same module, so "is this headword a LIST of letters?" has one
// answer in both: a list reads "How these letters are written, stroke by
// stroke: ক, ণ, শ" rather than "How ক — ণ — শ is written", and a single
// letter or a word reads exactly as before. The module has no imports, so it
// costs the first-paint chunk a few hundred bytes.
export { filmstripCaption } from "@coding-adventures/human-language-data/src/filmstrip-caption.ts";
