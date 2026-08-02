import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, normalize, relative as pathRelative, resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { renderBookChapter, type BookGenerationTarget } from "./book.js";
import { defaultCurriculumRoot, loadLessons } from "./loader.js";

interface BookGenerationConfig {
  version: 1;
  targets: BookGenerationTarget[];
}

interface GeneratedBookHashManifest {
  version: 1;
  algorithm: "fnv1a64";
  chapters: Array<{
    language: string;
    chapter: number;
    sourceHash: string;
    lessonIds: string[];
    tex: string;
  }>;
}

const MANIFEST_PATH = "core/generated-book-hashes.json";

function loadConfig(root: string): BookGenerationConfig {
  return JSON.parse(
    readFileSync(join(root, "core", "book-generation.json"), "utf8"),
  ) as BookGenerationConfig;
}

function safeOutput(root: string, relative: string): string {
  const output = resolve(root, relative);
  const fromRoot = normalize(pathRelative(resolve(root), output)).replaceAll("\\", "/");
  if (
    fromRoot === "" ||
    fromRoot === ".." ||
    fromRoot.startsWith("../") ||
    !fromRoot.endsWith(".tex")
  ) {
    throw new Error(`unsafe generated book output '${relative}'`);
  }
  return output;
}

export function generatedBookOutputs(root = defaultCurriculumRoot()): Map<string, string> {
  const config = loadConfig(root);
  if (config.version !== 1 || config.targets.length === 0) {
    throw new Error("book-generation.json must declare version 1 and at least one target");
  }
  const lessons = loadLessons(root);
  const outputs = new Map<string, string>();
  const manifest: GeneratedBookHashManifest = { version: 1, algorithm: "fnv1a64", chapters: [] };
  for (const target of config.targets) {
    const generated = renderBookChapter(target, lessons);
    safeOutput(root, target.output);
    outputs.set(target.output, generated.tex);
    manifest.chapters.push({
      language: target.language,
      chapter: target.chapter,
      sourceHash: generated.sourceHash,
      lessonIds: generated.lessonIds,
      tex: target.output,
    });
  }
  manifest.chapters.sort(
    (left, right) => left.language.localeCompare(right.language) || left.chapter - right.chapter,
  );
  outputs.set(MANIFEST_PATH, `${JSON.stringify(manifest, null, 2)}\n`);
  return outputs;
}

export function runBookGeneration(
  args = process.argv.slice(2),
  root = defaultCurriculumRoot(),
): number {
  const mode = args.length === 1 ? args[0] : undefined;
  if (mode !== "--check" && mode !== "--write") {
    process.stderr.write("usage: book-cli (--check | --write)\n");
    return 2;
  }
  let mismatch = false;
  for (const [relative, expected] of generatedBookOutputs(root)) {
    const output = relative === MANIFEST_PATH ? join(root, relative) : safeOutput(root, relative);
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
  process.exit(runBookGeneration());
}
