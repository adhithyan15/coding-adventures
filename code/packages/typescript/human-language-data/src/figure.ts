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

/** Turn an authored root id such as `qahwah-arabic` into a printable node. */
export function etymologyRootNode(root: string): FigureNode {
  const pieces = root.split("-").filter(Boolean);
  if (pieces.length < 2) {
    throw new Error(`etymology root '${root}' must end in a language tag`);
  }
  const language = pieces.pop() ?? "";
  return {
    term: pieces.join(" "),
    language: titleCase(language),
  };
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
export function renderEtymologyRouteFigure(lesson: ParsedLesson): GeneratedFigure {
  const { realization } = lesson;
  if (realization.lessonId === "" || realization.headword.trim() === "") {
    throw new Error("etymology figures require a lesson id and headword");
  }
  if (realization.roots.length < 2) {
    throw new Error(`${realization.lessonId}: etymology-route requires at least two roots`);
  }

  const nodes: FigureNode[] = [
    ...realization.roots.map(etymologyRootNode),
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
}

export function renderFigure(
  target: FigureTarget,
  lesson: ParsedLesson,
  sources: FigureSources = {},
): GeneratedFigure {
  if (target.kind === "etymology-route") return renderEtymologyRouteFigure(lesson);
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
