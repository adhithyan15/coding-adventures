import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, normalize, relative as pathRelative, resolve } from "node:path";
import { assertRelativeManifestPath } from "./manifest-path.js";
import { readLedgerFile } from "./shard.js";
import { loadRootTagVocabulary } from "./root-slug-splits.js";
import { pathToFileURL } from "node:url";
import { defaultCurriculumRoot, loadLessons } from "./loader.js";
import { renderFigure, type FigureSources, type FigureTarget } from "./figure.js";
import {
  indexFilmstripLedger,
  FILMSTRIP_LEDGER_PATH,
  type FilmstripLedger,
} from "./figure-filmstrip.js";
import { filmstripCandidates, withDerivedFilmstrips } from "./figure-targets.js";
import type { ParsedLesson } from "./parse.js";

interface FigureGenerationConfig {
  version: 1;
  targets: FigureTarget[];
}

interface GeneratedFigureHashManifest {
  version: 1;
  algorithm: "fnv1a64";
  figures: Array<{
    kind: FigureTarget["kind"];
    lessonId: string;
    sourceHash: string;
    svgHash: string;
    svg: string;
  }>;
}

export const FIGURE_CONFIG_PATH = "core/figure-generation.json";
export const FIGURE_HASH_MANIFEST_PATH = "core/generated-figure-hashes.json";

function loadConfig(root: string): FigureGenerationConfig {
  // Through the guarded door, like `book-cli`'s sibling `loadConfig`.
  // `readLedgerFile` rather than `readMaybeSharded` because this ledger has no
  // `X.d/` today — and if it ever gets one, this read fails loudly instead of
  // quietly handing the generator a stale copy of its own target list. That is
  // exactly the failure `book-cli`'s comment describes, which is why it is
  // worth a line on a file nobody has sharded yet.
  return readLedgerFile<FigureGenerationConfig>(join(root, FIGURE_CONFIG_PATH));
}

/**
 * Reject a target the renderer could not honour, at the point the config is
 * read rather than deep inside a renderer. `figure-generation.json` is authored
 * by hand, so "kind" is checked against the union it claims to belong to and a
 * filmstrip target is required to name the letter it draws.
 */
export function assertKnownFigureTarget(target: FigureTarget): void {
  if (typeof target.lessonId !== "string" || target.lessonId === "") {
    throw new Error("every figure target needs a lessonId");
  }
  if (target.kind === "etymology-route") return;
  if (target.kind === "script-filmstrip") {
    if (typeof target.script !== "string" || !/^[a-z][a-z-]*$/.test(target.script)) {
      throw new Error(`${target.lessonId}: script-filmstrip needs a canonical script id`);
    }
    if (typeof target.glyph !== "string" || target.glyph === "") {
      throw new Error(`${target.lessonId}: script-filmstrip needs a glyph`);
    }
    return;
  }
  const exhaustive: never = target;
  throw new Error(`unknown figure kind in target ${JSON.stringify(exhaustive)}`);
}

/** Every generated SVG must remain under one track's book/figures directory. */
export function safeFigureOutput(root: string, relative: string): string {
  assertRelativeManifestPath(relative, `unsafe generated figure output '${relative}'`);
  const output = resolve(root, relative);
  const fromRoot = normalize(pathRelative(resolve(root), output)).replaceAll("\\", "/");
  if (
    fromRoot === "" ||
    fromRoot === ".." ||
    fromRoot.startsWith("../") ||
    !/^[a-z0-9-]+\/book\/figures\/[A-Za-z0-9._-]+\.svg$/.test(fromRoot)
  ) {
    throw new Error(`unsafe generated figure output '${relative}'`);
  }
  return output;
}

/**
 * Load whatever the declared targets actually need.
 *
 * The filmstrip ledger is only read when a `script-filmstrip` target exists, so
 * a curriculum that prints no filmstrips does not require the generated file to
 * be present at all — and one that does gets a named, actionable failure rather
 * than a missing-file stack trace.
 */
function figureSources(root: string, targets: FigureTarget[]): FigureSources {
  const sources: FigureSources = {};
  if (targets.some((target) => target.kind === "script-filmstrip")) {
    const ledger = readLedgerFile<FilmstripLedger>(join(root, FILMSTRIP_LEDGER_PATH));
    sources.filmstrips = indexFilmstripLedger(ledger);
  }
  // Read from THIS root, not the package's own, and only when a target needs
  // it -- the same conditional shape as the filmstrip ledger above, for the
  // same reason: a curriculum that prints no etymology routes should not have
  // to carry the file.
  if (targets.some((target) => target.kind === "etymology-route")) {
    sources.rootTags = loadRootTagVocabulary(root);
  }
  return sources;
}

/**
 * Every figure the book prints: the targets declared in `figure-generation.json`
 * plus, for switched-on tracks, a filmstrip for each single-letter writing
 * lesson whose letter has a cited ductus (HL-C443, `figure-targets.ts`). The
 * generated filmstrip ledger is the record of which letters are cited, so a
 * derived target can never outrun the stroke data.
 */
export function resolvedFigureTargets(
  root: string,
  lessons: readonly ParsedLesson[],
): FigureTarget[] {
  const config = loadConfig(root);
  if (config.version !== 1 || !Array.isArray(config.targets) || config.targets.length === 0) {
    throw new Error("figure-generation.json must declare version 1 and at least one target");
  }
  const candidates = filmstripCandidates(lessons);
  if (candidates.length === 0) return config.targets;
  const cited = indexFilmstripLedger(
    readLedgerFile<FilmstripLedger>(join(root, FILMSTRIP_LEDGER_PATH)),
  );
  return withDerivedFilmstrips(config.targets, candidates, (script, glyph) =>
    cited.has(`${script}:${glyph}`),
  );
}

export function generatedFigureOutputs(
  root = defaultCurriculumRoot(),
): Map<string, string> {
  const loaded = loadLessons(root);
  const targets = resolvedFigureTargets(root, loaded);
  const lessons = new Map(loaded.map((lesson) => [lesson.realization.lessonId, lesson]));
  const sources = figureSources(root, targets);
  const outputs = new Map<string, string>();
  const manifest: GeneratedFigureHashManifest = {
    version: 1,
    algorithm: "fnv1a64",
    figures: [],
  };
  for (const target of targets) {
    assertKnownFigureTarget(target);
    safeFigureOutput(root, target.output);
    if (outputs.has(target.output)) throw new Error(`${target.output}: duplicate figure output`);
    const lesson = lessons.get(target.lessonId);
    if (!lesson) throw new Error(`${target.lessonId}: figure target lesson is missing`);
    if (!target.output.startsWith(`${lesson.language}/book/figures/`)) {
      throw new Error(`${target.lessonId}: figure output must stay in the lesson's track book`);
    }
    const generated = renderFigure(target, lesson, sources);
    outputs.set(target.output, generated.svg);
    manifest.figures.push({
      kind: target.kind,
      lessonId: target.lessonId,
      sourceHash: generated.sourceHash,
      svgHash: generated.svgHash,
      svg: target.output,
    });
  }
  manifest.figures.sort((left, right) => left.svg.localeCompare(right.svg));
  outputs.set(FIGURE_HASH_MANIFEST_PATH, `${JSON.stringify(manifest, null, 2)}\n`);
  return outputs;
}

export function runFigureGeneration(
  args = process.argv.slice(2),
  root = defaultCurriculumRoot(),
): number {
  const mode = args.length === 1 ? args[0] : undefined;
  if (mode !== "--check" && mode !== "--write") {
    process.stderr.write("usage: figure-cli (--check | --write)\n");
    return 2;
  }
  let mismatch = false;
  for (const [relative, expected] of generatedFigureOutputs(root)) {
    const output =
      relative === FIGURE_HASH_MANIFEST_PATH
        ? join(root, relative)
        : safeFigureOutput(root, relative);
    if (mode === "--write") {
      mkdirSync(dirname(output), { recursive: true });
      writeFileSync(output, expected, "utf8");
      process.stdout.write(`generated ${relative}\n`);
      continue;
    }
    const actual = existsSync(output) ? readFileSync(output, "utf8") : undefined;
    if (actual !== expected) {
      process.stderr.write(`${relative}: generated output is missing or stale\n`);
      mismatch = true;
    }
  }
  return mismatch ? 1 : 0;
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  process.exit(runFigureGeneration());
}
