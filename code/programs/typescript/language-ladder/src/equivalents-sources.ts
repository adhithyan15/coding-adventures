// One lazy loader per HL41 equivalents owner file, and the table they fill.
// Like `filmstrip-sources.ts`, this module is itself reached only through a
// dynamic import (from main.ts, when a lesson opens), so the map of hundreds of
// paths, the comparison sets and the table builder all stay out of the
// first-paint chunk. vite.config.ts groups the owner files one chunk per track.
import comparisonSets from "../../../../learning/human-languages/core/comparison-sets.json";
import { equivalentsView, type EquivalentsView } from "./equivalents.ts";

const EQUIVALENT_LOADERS = import.meta.glob(
  "../../../../learning/human-languages/*/equivalents.d/*.json",
  { import: "default" },
) as Record<string, () => Promise<unknown>>;

interface ComparisonSetFile {
  sets: Record<string, { family?: string[]; neighbours?: string[] }>;
}

/** A lesson's owner file, as loaded, with its track's neighbour list. */
export interface LoadedEquivalents {
  raw: unknown;
  neighbours: readonly string[];
}

/**
 * Load one lesson's equivalents, or null when the lesson has none.
 *
 * Both ids are checked before they are joined into a path suffix, as
 * `generatedFilmstripUrl` checks them: the suffix only ever matches a key the
 * glob already produced, but an id with a slash in it is refused, not matched.
 */
export async function loadEquivalents(
  language: string,
  lessonId: string,
): Promise<LoadedEquivalents | null> {
  if (!/^[a-z][a-z0-9-]*$/.test(language) || !/^[A-Za-z0-9._-]+$/.test(lessonId)) return null;
  const suffix = `/human-languages/${language}/equivalents.d/${lessonId}.json`;
  const entry = Object.entries(EQUIVALENT_LOADERS).find(([path]) =>
    path.replaceAll("\\", "/").endsWith(suffix),
  );
  if (entry === undefined) return null;
  const neighbours = (comparisonSets as ComparisonSetFile).sets[language]?.neighbours ?? [];
  return { raw: await entry[1](), neighbours };
}

function node<Tag extends keyof HTMLElementTagNameMap>(
  tag: Tag,
  className: string,
  text?: string,
): HTMLElementTagNameMap[Tag] {
  const element = document.createElement(tag);
  if (className !== "") element.className = className;
  if (text !== undefined) element.textContent = text;
  return element;
}

/** The HL41 panel as a small table, built from text nodes only (no HTML strings). */
export function renderEquivalentsPanel(view: EquivalentsView): HTMLElement {
  const figure = node("figure", "lesson-body__equivalents");
  const caption = node("figcaption", "lesson-body__equivalents-title", "In the family, and next door");
  const table = node("table", "lesson-body__equivalents-table");
  for (const row of view.rows) {
    const tr = node("tr", "");
    const label = node("th", "", row.label);
    label.setAttribute("scope", "row");
    const word = node("td", "lesson-body__equivalents-word");
    word.appendChild(node("span", "lesson-body__equivalents-form", row.form));
    if (row.romanization !== "") {
      word.appendChild(node("span", "lesson-body__equivalents-said", row.romanization));
    }
    tr.append(label, word);
    if (view.marked) {
      tr.appendChild(node("td", "lesson-body__equivalents-root", row.sameRoot ? "same root" : ""));
    }
    table.appendChild(tr);
  }
  figure.append(caption, table);
  return figure;
}

/** Load, check and build one lesson's panel, or null when it has none. */
export async function equivalentsPanel(
  language: string,
  lessonId: string,
  nameOf: (languageId: string) => string,
): Promise<HTMLElement | null> {
  const loaded = await loadEquivalents(language, lessonId);
  if (loaded === null) return null;
  const view = equivalentsView(loaded.raw, loaded.neighbours, nameOf);
  return view === null ? null : renderEquivalentsPanel(view);
}
