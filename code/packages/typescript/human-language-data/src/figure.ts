import {
  paintLine,
  paintPath,
  paintRect,
  paintScene,
  paintText,
  type PaintInstruction,
} from "@coding-adventures/paint-instructions";
import { renderToSvgString } from "@coding-adventures/paint-vm-svg";
import { fnv1a64 } from "./hash.js";
import {
  renderScriptFilmstripFigure,
  type FilmstripEntry,
} from "./figure-filmstrip.js";
import type { ParsedLesson } from "./parse.js";
import { stripControlCharacters } from "./constants.js";
import { parseRootSlug, type RootTagVocabulary } from "./root-slug-splits.js";

/**
 * The deterministic vector figures this curriculum generates.
 *
 * `etymology-route` was the first (HL06): the ordered roots a headword
 * travelled through. `script-filmstrip` is the second, and it exists because
 * writing cannot be taught in sentences — see `figure-filmstrip.ts`.
 */
export type FigureKind = "etymology-route" | "script-filmstrip";

interface FigureTargetBase {
  lessonId: string;
  output: string;
}

/** One lesson's etymology route: roots -> headword, left to right. */
export interface EtymologyRouteTarget extends FigureTargetBase {
  kind: "etymology-route";
}

/**
 * One letter's handwriting, frame by frame.
 *
 * `script` and `glyph` are named explicitly rather than inferred from the
 * lesson's headword: a lesson may print a letter it is not itself named after,
 * and a figure that silently followed a headword edit would redraw itself the
 * next time somebody fixed a typo.
 */
export interface ScriptFilmstripTarget extends FigureTargetBase {
  kind: "script-filmstrip";
  /** Canonical script id, e.g. `tamil`, `devanagari`, `perso-arabic`. */
  script: string;
  /** The character whose ductus is drawn. */
  glyph: string;
}

export type FigureTarget = EtymologyRouteTarget | ScriptFilmstripTarget;

export interface GeneratedFigure {
  svg: string;
  sourceHash: string;
  svgHash: string;
  labels: string[];
}

interface FigureNode {
  term: string;
  language: string;
}

function titleCase(value: string): string {
  return value.length === 0 ? value : `${value[0]?.toUpperCase()}${value.slice(1)}`;
}

/**
 * Turn an authored root id such as `qahwah-arabic` into a printable node.
 *
 * THE TAG IS FOUND BY VOCABULARY, NOT BY POSITION, and that is the whole point
 * of this function taking a `RootTagVocabulary`. It used to be
 * `pieces.pop()` — take the last hyphen-separated token and call it the
 * language — which is correct for exactly one of the three slug shapes the
 * corpus uses.
 *
 * HL-C419's normalisation is what exposed it. That pass established the
 * canonical shape is PER TAG: Latin and Greek write `lemma-latin`, but
 * Sanskrit, Dravidian, PIE, Persian and Arabic write `sanskrit-lemma`. When
 * `kahve-turkish` became `turkish-kahve`, this function read the term as
 * "turkish" and the language as "Kahve", and the published SVG for
 * `ES-C06-cafe` went out claiming Arabic *qahwah* became **Kahve "turkish"**.
 * Nothing threw: the slug still had two pieces, so the figure was silently
 * wrong and its hash ledger was regenerated to match.
 *
 * The positional reading was already wrong for a prefix slug before that pass;
 * the normalisation only made one reachable from a figure. `proto-indo-european-dwoh`
 * would have rendered the term "proto indo european" in the language "Dwoh".
 *
 * The lemma is sliced from the ORIGINAL string rather than taken from
 * `parseRootSlug`, which case-folds. A folded lemma is right for a join key and
 * wrong for a caption: the figure prints what the author wrote.
 */
export function etymologyRootNode(root: string, vocabulary: RootTagVocabulary): FigureNode {
  const parsed = parseRootSlug(root, vocabulary);
  const safe = stripControlCharacters(root);
  if (parsed.tag === undefined) {
    throw new Error(`etymology root '${safe}' must carry a language tag`);
  }
  // Sliced from the ORIGINAL by the FOLDED tag's length, which is sound only
  // because folding cannot change the length of the tag region. The one Unicode
  // lowercase mapping that expands is U+0130 -> `i` + U+0307, and no declared
  // tag contains U+0307. If a tag ever does, slice by a re-derived index
  // instead -- the failure would be a silently shifted caption, not a throw.
  const lemma =
    parsed.shape === "suffix"
      ? root.slice(0, -(parsed.tag.length + 1))
      : root.slice(parsed.tag.length + 1);
  // The guard is on PRINTABILITY, and it took three tries to get there, each
  // one stopping a step short of where the value actually goes:
  //
  //   `lemma === ""`      -- `latin--` slices to `-`, non-empty, then joins to ""
  //   `term === ""`       -- `latin- -` joins to " ", non-empty, prints nothing
  //   this one            -- follows the value to what a reader can SEE
  //
  // Every miss published the same artifact: a box with a language under it and
  // no word in it, nothing thrown, hash ledger regenerated to match. `latin- -`
  // is pure ASCII and survives the frontmatter list parser, which trims only an
  // item's outer edges. The `Cf` class is there because U+200B and friends are
  // format characters, not whitespace, so `.trim()` never touches them.
  const term = lemma.split("-").filter(Boolean).join(" ");
  if (term.replace(/[\p{White_Space}\p{Cf}]/gu, "") === "") {
    throw new Error(`etymology root '${safe}' has no printable term`);
  }
  return { term, language: titleCase(parsed.tag) };
}

/**
 * The canonical subset that is allowed to change an etymology-route figure.
 * Prose edits outside these fields do not churn an unrelated vector artifact.
 */
export function etymologyFigureSource(lesson: ParsedLesson): string {
  return JSON.stringify({
    kind: "etymology-route",
    lessonId: lesson.realization.lessonId,
    language: lesson.realization.language,
    headword: lesson.realization.headword,
    roots: lesson.realization.roots,
  });
}

function arrowInstructions(x1: number, x2: number, y: number): PaintInstruction[] {
  const tip = x2 - 8;
  return [
    paintLine(x1, y, tip, y, "#64748b", { stroke_width: 3, stroke_cap: "round" }),
    paintPath(
      [
        { kind: "move_to", x: tip, y: y - 7 },
        { kind: "line_to", x: x2, y },
        { kind: "line_to", x: tip, y: y + 7 },
        { kind: "close" },
      ],
      { fill: "#64748b" },
    ),
  ];
}

/** Render one lesson's ordered roots and headword through paint-vm-svg. */
export function renderEtymologyRouteFigure(
  lesson: ParsedLesson,
  vocabulary: RootTagVocabulary,
): GeneratedFigure {
  const { realization } = lesson;
  if (realization.lessonId === "" || realization.headword.trim() === "") {
    throw new Error("etymology figures require a lesson id and headword");
  }
  if (realization.roots.length < 2) {
    throw new Error(`${realization.lessonId}: etymology-route requires at least two roots`);
  }

  // The arrow is deliberate: `.map(etymologyRootNode)` hands the callback the
  // array INDEX as its second argument, which is the vocabulary parameter.
  const nodes: FigureNode[] = [
    ...realization.roots.map((root) => etymologyRootNode(root, vocabulary)),
    { term: realization.headword, language: titleCase(realization.language) },
  ];
  const nodeWidth = 170;
  const nodeHeight = 94;
  const gap = 50;
  const margin = 28;
  const width = margin * 2 + nodes.length * nodeWidth + (nodes.length - 1) * gap;
  const height = 180;
  const top = 38;
  const midline = top + nodeHeight / 2;
  const instructions: PaintInstruction[] = [];

  nodes.forEach((node, index) => {
    const x = margin + index * (nodeWidth + gap);
    const isDestination = index === nodes.length - 1;
    instructions.push(
      paintRect(x, top, nodeWidth, nodeHeight, {
        fill: isDestination ? "#eef2ff" : "#f8fafc",
        stroke: isDestination ? "#3b5bdb" : "#64748b",
        stroke_width: isDestination ? 3 : 2,
        corner_radius: 12,
      }),
      paintText(
        x + nodeWidth / 2,
        top + 41,
        node.term,
        "svg:Latin Modern Sans@22:700",
        22,
        "#172033",
        { text_align: "center" },
      ),
      paintText(
        x + nodeWidth / 2,
        top + 72,
        node.language,
        "svg:Latin Modern Sans@16",
        16,
        "#475569",
        { text_align: "center" },
      ),
    );
    if (index < nodes.length - 1) {
      instructions.push(...arrowInstructions(x + nodeWidth + 8, x + nodeWidth + gap - 8, midline));
    }
  });

  const sourceHash = fnv1a64(etymologyFigureSource(lesson));
  const svg = `${renderToSvgString(
    paintScene(width, height, "#ffffff", instructions, {
      id: realization.lessonId,
      metadata: { sourceHash, figureKind: "etymology-route" },
    }),
  )}\n`;
  return {
    svg,
    sourceHash,
    svgHash: fnv1a64(svg),
    labels: nodes.flatMap((node) => [node.term, node.language]),
  };
}

/**
 * Everything a figure kind may need that is not in the lesson.
 *
 * Today that is one thing: the filmstrip geometry `script-ductus` generates.
 * It is passed in rather than read here so this module stays free of the
 * filesystem and every figure remains a pure function of its inputs — which is
 * what makes `check:figures` a byte comparison rather than a re-run.
 */
export interface FigureSources {
  /** Filmstrip entries by `script:glyph`. */
  filmstrips?: Map<string, FilmstripEntry>;
  /**
   * The declared language tags, for reading an etymology slug's shape.
   *
   * Carried here for the reason in this block's header: read inside the
   * renderer, `core/root-tags.json` resolved from the PACKAGE's install
   * location rather than the caller's curriculum root, so a figure generated
   * for root R depended on a file that was not under R. Two `figure-cli`
   * fixtures were silently consuming the real repository's copy.
   *
   * It does NOT put the vocabulary inside `sourceHash`, and an earlier draft of
   * this comment claimed it did. `etymologyFigureSource` still covers lesson
   * fields only, so editing `root-tags.json` can still change an SVG without
   * moving its source hash. What catches that is `check:figures`, which
   * compares the rendered BYTES -- the byte comparison this module stays pure
   * in order to keep honest.
   */
  rootTags?: RootTagVocabulary;
}

export function renderFigure(
  target: FigureTarget,
  lesson: ParsedLesson,
  sources: FigureSources = {},
): GeneratedFigure {
  if (target.kind === "etymology-route") {
    if (sources.rootTags === undefined) {
      throw new Error(
        `${target.lessonId}: no root-tag vocabulary for etymology-route — ` +
          `pass \`rootTags\` in FigureSources`,
      );
    }
    return renderEtymologyRouteFigure(lesson, sources.rootTags);
  }
  if (target.kind === "script-filmstrip") {
    const key = `${target.script}:${target.glyph}`;
    const entry = sources.filmstrips?.get(key);
    if (entry === undefined) {
      throw new Error(
        `${target.lessonId}: no filmstrip geometry for ${key} — add the target, ` +
          `then run \`npm run generate:filmstrip-ledger\` in script-ductus`,
      );
    }
    return renderScriptFilmstripFigure(target.lessonId, entry);
  }
  const exhaustive: never = target;
  throw new Error(`unsupported figure kind '${JSON.stringify(exhaustive)}'`);
}
