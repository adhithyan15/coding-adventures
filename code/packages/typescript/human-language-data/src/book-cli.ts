import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, normalize, relative as pathRelative, resolve } from "node:path";
import { pathToFileURL } from "node:url";
import {
  renderBookChapter,
  type BookGenerationTarget,
  type InlineRenderOptions,
} from "./book.js";
import { defaultCurriculumRoot, loadLessons } from "./loader.js";

interface ConfiguredBookGenerationTarget extends BookGenerationTarget {
  /** Named reusable mapping from the config's scriptSets table. */
  scriptSet?: string;
}

/**
 * A chapter whose `.tex` a human wrote by hand, recorded so its title and label are
 * still checkable — and deliberately NOT generated.
 *
 * Why a second list instead of a `generated: false` flag on `targets[]`?  Because the
 * two options fail in opposite directions.  Everything in `targets[]` is rendered and
 * then *written over* the file at `output` by `--write`; a flag would put hand-authored
 * prose one forgotten `if` away from being overwritten by generated text.  A separate
 * array cannot suffer that: `generatedBookOutputs` only ever walks `config.targets`, so
 * the worst a mistake here can do is leave a chapter unchecked — the status quo — rather
 * than destroy it.  The safe failure mode is the whole point.
 *
 * These chapters predate the manifest and are mostly schema-v1, so there are no canonical
 * lessons to render them from anyway.  `title` and `label` are transcribed from what the
 * `.tex` actually declares, never invented, and the tests below re-read the files to prove
 * it.  Note that labels follow three different historical conventions (a bare `ch:greetings`
 * slug, an ISO-code `ch:fa-`/`ch:la-` prefix, and a language-name `ch:persian-` prefix);
 * they are recorded as-is, because rewriting a `\label` would break existing `\hyperref`
 * cross-references.
 */
export interface HandwrittenBookChapter {
  language: string;
  chapter: number;
  title: string;
  label: string;
  output: string;
}

interface BookGenerationConfig {
  version: 1;
  sourceBaseUrl: string;
  scriptSets?: Record<string, InlineRenderOptions[]>;
  targets: ConfiguredBookGenerationTarget[];
  /** Never rendered. See {@link HandwrittenBookChapter}. */
  handwritten?: HandwrittenBookChapter[];
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

/**
 * The hand-written chapters, for callers that need to know a chapter's printed title and
 * label without asking the generator to produce one.  Reading this never renders anything.
 */
export function handwrittenBookChapters(
  root = defaultCurriculumRoot(),
): HandwrittenBookChapter[] {
  return loadConfig(root).handwritten ?? [];
}

export function generatedBookOutputs(root = defaultCurriculumRoot()): Map<string, string> {
  const config = loadConfig(root);
  if (config.version !== 1 || config.targets.length === 0) {
    throw new Error("book-generation.json must declare version 1 and at least one target");
  }
  let sourceBaseUrl: string;
  try {
    const parsed = new URL(config.sourceBaseUrl);
    if (parsed.protocol !== "https:" && parsed.protocol !== "http:") throw new Error();
    parsed.search = "";
    parsed.hash = "";
    if (!parsed.pathname.endsWith("/")) parsed.pathname += "/";
    sourceBaseUrl = parsed.href;
  } catch {
    throw new Error("book-generation.json must declare an HTTP(S) sourceBaseUrl");
  }
  const lessons = loadLessons(root);
  const outputs = new Map<string, string>();
  const manifest: GeneratedBookHashManifest = { version: 1, algorithm: "fnv1a64", chapters: [] };
  for (const configuredTarget of config.targets) {
    const { scriptSet, ...plainTarget } = configuredTarget;
    let target: BookGenerationTarget = { ...plainTarget, sourceBaseUrl };
    if (scriptSet !== undefined) {
      if (
        target.inlineScripts !== undefined ||
        target.unicodeScript !== undefined ||
        target.scriptCommand !== undefined
      ) {
        throw new Error(
          `${target.language} chapter ${target.chapter}: scriptSet cannot be combined with inline script options`,
        );
      }
      const inlineScripts = config.scriptSets?.[scriptSet];
      if (!inlineScripts) {
        throw new Error(
          `${target.language} chapter ${target.chapter}: unknown scriptSet '${scriptSet}'`,
        );
      }
      target = { ...target, inlineScripts };
    }
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
