// Stroke-order filmstrips (`*-filmstrip.svg`, HL-C443) are excluded. The book
// places them from derived targets and no lesson's Markdown references one, so
// the app never asks for them -- and there are several hundred, which pushed
// this eager URL map over the 500 kB first-paint budget (`check:bundle`) the
// day they were rolled out. Showing filmstrips in the app is a lazy-loading
// feature of its own, tracked in HL-C443.
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
