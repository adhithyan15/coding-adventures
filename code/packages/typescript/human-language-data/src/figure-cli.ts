import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, normalize, relative as pathRelative, resolve } from "node:path";
import { assertRelativeManifestPath } from "./manifest-path.js";
import { pathToFileURL } from "node:url";
import { defaultCurriculumRoot, loadLessons } from "./loader.js";
import { renderFigure, type FigureTarget } from "./figure.js";

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
  return JSON.parse(readFileSync(join(root, FIGURE_CONFIG_PATH), "utf8")) as FigureGenerationConfig;
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

export function generatedFigureOutputs(
  root = defaultCurriculumRoot(),
): Map<string, string> {
  const config = loadConfig(root);
  if (config.version !== 1 || !Array.isArray(config.targets) || config.targets.length === 0) {
    throw new Error("figure-generation.json must declare version 1 and at least one target");
  }
  const lessons = new Map(loadLessons(root).map((lesson) => [lesson.realization.lessonId, lesson]));
  const outputs = new Map<string, string>();
  const manifest: GeneratedFigureHashManifest = {
    version: 1,
    algorithm: "fnv1a64",
    figures: [],
  };
  for (const target of config.targets) {
    safeFigureOutput(root, target.output);
    if (outputs.has(target.output)) throw new Error(`${target.output}: duplicate figure output`);
    const lesson = lessons.get(target.lessonId);
    if (!lesson) throw new Error(`${target.lessonId}: figure target lesson is missing`);
    if (!target.output.startsWith(`${lesson.language}/book/figures/`)) {
      throw new Error(`${target.lessonId}: figure output must stay in the lesson's track book`);
    }
    const generated = renderFigure(target, lesson);
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
