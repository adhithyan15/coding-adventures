import { existsSync, lstatSync, mkdirSync, readFileSync, realpathSync, writeFileSync } from "node:fs";
import { dirname, join, normalize, relative as pathRelative, resolve, sep } from "node:path";
import { assertRelativeManifestPath } from "./manifest-path.js";
import {
  BOOK_GENERATION_GROUPED_KEYS,
  mergeGroupedShards,
  readMaybeSharded,
} from "./shard.js";
import { pathToFileURL } from "node:url";
import {
  renderBookAnswerKey,
  renderBookChapter,
  renderBookChapterModalities,
  renderBookGlossary,
  renderBookIndex,
  renderReferenceAppendix,
  type BookAnswerKeyTarget,
  type BookGenerationTarget,
  type BookGlossaryTarget,
  type BookIndexTarget,
  type BookReferenceAppendixTarget,
  type InlineRenderOptions,
} from "./book.js";
import {
  defaultCurriculumRoot,
  loadChapterPolicy,
  loadLessons,
  loadTrackChapters,
} from "./loader.js";
import type { ParsedLesson } from "./parse.js";
import { summarizeModality } from "./modality.js";
import {
  BACKMATTER_TEX,
  FRONTMATTER_TEX,
  chapterInputsFor,
  renderBookTex,
} from "./book-tex.js";
import type { ChapterCapability } from "./types.js";

interface ConfiguredBookGenerationTarget extends Omit<BookGenerationTarget, "title" | "label"> {
  /** Named reusable mapping from the config's scriptSets table. */
  scriptSet?: string;
}

interface ConfiguredReferenceAppendixTarget extends BookReferenceAppendixTarget {
  /** Named reusable mapping from the config's scriptSets table. */
  scriptSet?: string;
}

interface ConfiguredBookGlossaryTarget extends BookGlossaryTarget {
  /** Named reusable mapping from the config's scriptSets table. */
  scriptSet?: string;
}

interface ConfiguredBookAnswerKeyTarget extends BookAnswerKeyTarget {
  /** Named reusable mapping from the config's scriptSets table. */
  scriptSet?: string;
}

interface ConfiguredBookIndexTarget extends BookIndexTarget {
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
 * array cannot suffer that: the renderer only sends `config.targets` through
 * `renderBookChapter`, so no handwritten output path can be overwritten. HL-C15 reads
 * this list only to put the chapter number in the book-wide generated modality projection;
 * the authored chapter body remains outside the output map. The safe failure mode is
 * the whole point.
 *
 * These chapters predate the manifest and are mostly schema-v1, so there are no canonical
 * lessons to render them from anyway. Their output path stays here, while `title` and
 * `label` resolve from `chapters.json`, the same authored capability ledger used by the
 * generated chapters. Tests re-read every committed `.tex` file to prove that canonical
 * metadata still agrees with the historical chapter command and label. Note that labels
 * follow three different historical conventions (a bare `ch:greetings` slug, an ISO-code
 * `ch:fa-`/`ch:la-` prefix, and a language-name `ch:persian-` prefix); the ledgers preserve
 * those values because rewriting a `\label` would break existing `\hyperref` references.
 */
interface ConfiguredHandwrittenBookChapter {
  language: string;
  chapter: number;
  output: string;
  /** Canonical schema-v2 lessons visibly inserted into this protected body. */
  embeddedLessonIds?: string[];
  /** Known canonical schema-v2 lessons still absent from this protected body. */
  omittedLessonIds?: string[];
  /** Dependency-linked GitHub issue that owns every `omittedLessonIds` entry. */
  omissionIssue?: number;
}

/** A handwritten declaration resolved against its canonical chapter capability. */
export interface HandwrittenBookChapter extends ConfiguredHandwrittenBookChapter {
  title: string;
  label: string;
}

interface BookGenerationConfig {
  version: 1;
  sourceBaseUrl: string;
  scriptSets?: Record<string, InlineRenderOptions[]>;
  targets: ConfiguredBookGenerationTarget[];
  /** Canonical Markdown references rendered into book back matter. */
  referenceAppendices?: ConfiguredReferenceAppendixTarget[];
  /** Canonical word and phrase lessons rendered into book glossaries. */
  glossaries?: ConfiguredBookGlossaryTarget[];
  /** Executable lesson activities rendered as review questions and answer keys. */
  answerKeys?: ConfiguredBookAnswerKeyTarget[];
  /** Canonical meanings, topic lessons, and chapter capabilities rendered as indexes. */
  indexes?: ConfiguredBookIndexTarget[];
  /** Chapter bodies are never rendered; coordinates feed shared derived book metadata. */
  handwritten?: ConfiguredHandwrittenBookChapter[];
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

export const BOOK_HASH_MANIFEST_DIR = "core/generated-book-hashes";

function manifestPath(language: string): string {
  if (!/^[a-z0-9-]+$/.test(language)) {
    throw new Error(`unsafe generated book manifest language '${language}'`);
  }
  return `${BOOK_HASH_MANIFEST_DIR}/${language}.json`;
}

export function loadBookGenerationConfig(root: string): BookGenerationConfig {
  // Through the shard reader, not a bare `readFileSync`. This was the only
  // non-loader read of any of the three HL21 ledgers left in `src/`, and once
  // `core/book-generation.d/` became the source of truth a direct read of the
  // monolith would have served this generator a file that is merely DERIVED
  // from it — correct while `--check` is green and quietly stale the moment it
  // is not, which is exactly the window `--check` exists to close.
  const config = readMaybeSharded<BookGenerationConfig>(
    join(root, "core", "book-generation.json"),
    (shards) =>
      mergeGroupedShards(shards, BOOK_GENERATION_GROUPED_KEYS) as unknown as BookGenerationConfig,
  );
  for (const [kind, entries] of [
    ["target", config.targets ?? []],
    ["handwritten chapter", config.handwritten ?? []],
  ] as const) {
    for (const entry of entries) {
      if ("title" in entry || "label" in entry) {
        throw new Error(
          `book-generation.json ${kind} ${entry.language} chapter ${entry.chapter} must derive title and label from chapters.json`,
        );
      }
    }
  }
  return config;
}

function chapterCapabilityIndex(root: string): Map<string, ChapterCapability> {
  const index = new Map<string, ChapterCapability>();
  for (const track of loadTrackChapters(root)) {
    for (const chapter of track.chapters) {
      index.set(`${track.language}#${chapter.chapter}`, chapter);
    }
  }
  return index;
}

function requireChapterCapability(
  index: ReadonlyMap<string, ChapterCapability>,
  language: string,
  chapter: number,
): ChapterCapability {
  const capability = index.get(`${language}#${chapter}`);
  if (!capability) {
    throw new Error(
      `${language} chapter ${chapter}: book-generation.json declaration has no chapters.json capability`,
    );
  }
  return capability;
}

function safeOutput(root: string, relative: string): string {
  assertRelativeManifestPath(relative, `unsafe generated book output '${relative}'`);
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

function safeMarkdownSource(root: string, relative: string): string {
  assertRelativeManifestPath(relative, `unsafe generated book source '${relative}'`);
  const source = resolve(root, relative);
  const fromRoot = normalize(pathRelative(resolve(root), source)).replaceAll("\\", "/");
  if (
    fromRoot === "" ||
    fromRoot === ".." ||
    fromRoot.startsWith("../") ||
    !fromRoot.endsWith(".md")
  ) {
    throw new Error(`unsafe generated book source '${relative}'`);
  }
  return source;
}

/**
 * The hand-written chapters, for callers that need to know a chapter's printed title and
 * label without asking the generator to produce one.  Reading this never renders anything.
 */
export function handwrittenBookChapters(
  root = defaultCurriculumRoot(),
): HandwrittenBookChapter[] {
  const handwritten = loadBookGenerationConfig(root).handwritten ?? [];
  const capabilities = chapterCapabilityIndex(root);
  // Nothing here is written today, only read — but `output` is still a path, and the
  // containment rule it has to satisfy is exactly the one `targets[]` already obeys.
  // Checking at the boundary means a later caller that *does* open one of these cannot
  // inherit a traversal hole from a malformed manifest, rather than each caller having
  // to remember the check for itself.
  for (const entry of handwritten) safeOutput(root, entry.output);
  return handwritten.map((entry) => {
    const capability = requireChapterCapability(
      capabilities,
      entry.language,
      entry.chapter,
    );
    return { ...entry, title: capability.title, label: capability.label };
  });
}

/**
 * Read one authored `book.tex` fragment, refusing a symlink.
 *
 * `existsSync` + `readFileSync` both follow symlinks, and `safeOutput` is purely
 * lexical so it cannot see one. A committed
 * `<track>/book/frontmatter.tex -> ~/.npmrc` would have that file's contents
 * copied verbatim into the generated `book.tex` and written into the working
 * tree — where the next `--write` commits it.
 */
/**
 * `lstat`, or undefined when the path does not exist.
 *
 * Deliberately NOT `existsSync`: that uses `stat`, so it follows symlinks and
 * reports a dangling one as absent — which silently skips any guard placed
 * behind it. `shard-cli.ts` carries the same helper for the same reason.
 */
function statIfPresent(path: string): ReturnType<typeof lstatSync> | undefined {
  try {
    return lstatSync(path);
  } catch {
    return undefined;
  }
}

function readAuthoredFragment(root: string, path: string): string {
  if (!lstatSync(path).isFile()) {
    throw new Error(`authored book fragment '${path}' is not a regular file`);
  }
  // `lstat` vets only the LAST component. `<track>` and `<track>/book` are still
  // followed, so a committed `spanish/book -> /somewhere/else` is read straight
  // through — and `safeOutput` cannot see it either, being purely lexical.
  // Resolving the whole chain and re-asserting containment is the only check
  // that covers every component at once.
  const real = realpathSync(path);
  const realRoot = realpathSync(root);
  // Compare by PREFIX, not by escape-shape.
  //
  // Inferring "escaped" from a leading `../` is the win32 hole this repo has
  // already fixed three times elsewhere: `path.relative()` cannot express a
  // journey between two roots as `..` steps, so it returns the target unchanged.
  // With root `C:\repo\curriculum`, measured on Node 24:
  //
  //     "D:\\evil.tex"                -> "D:/evil.tex"             NOT rejected
  //     "\\\\server\\share\\evil.tex"  -> "//server/share/evil.tex" NOT rejected
  //     "C:\\Windows\\evil.tex"       -> "../../Windows/evil.tex"   rejected
  //
  // `realpathSync` resolves NTFS junctions, so a committed `frontmatter.tex`
  // pointing at another volume — or at a UNC share, which additionally makes
  // this an outbound SMB fetch to an attacker-named host — would pass and have
  // its contents copied verbatim into the generated `book.tex`. Windows is this
  // repo's primary development platform, so that is the live case.
  if (real !== realRoot && !real.startsWith(realRoot + sep)) {
    throw new Error(`authored book fragment '${path}' resolves outside the curriculum root`);
  }
  return readFileSync(real, "utf8");
}

/**
 * Fail closed when schema-v2 curriculum and a protected handwritten chapter diverge.
 *
 * Generated chapters derive their complete lesson inventory from the canonical corpus.
 * Handwritten chapters cannot do that without overwriting authored prose, so their
 * manifest entry must instead partition every schema-v2 lesson into one of two explicit
 * states:
 *
 * - `embeddedLessonIds`: learner-visible in the body, proven by a canonical insertion
 *   marker and lesson label;
 * - `omittedLessonIds`: known migration debt owned by a dependency-linked issue.
 *
 * The partition is exact. A new lesson therefore cannot appear only in narration,
 * hashes, activities, or an answer key while `check:books` stays green.
 */
function assertHandwrittenLessonCoverageFrom(
  root: string,
  config: BookGenerationConfig,
  lessons: ParsedLesson[],
): void {
  const byChapter = new Map<string, ParsedLesson[]>();
  for (const lesson of lessons) {
    if (lesson.frontmatter.schema_version !== "2") continue;
    const chapter = Number(lesson.frontmatter.chapter);
    if (!Number.isInteger(chapter)) continue;
    const key = `${lesson.language}#${chapter}`;
    const entries = byChapter.get(key) ?? [];
    entries.push(lesson);
    byChapter.set(key, entries);
  }

  for (const entry of config.handwritten ?? []) {
    const key = `${entry.language}#${entry.chapter}`;
    const canonical = byChapter.get(key) ?? [];
    const canonicalIds = new Set(canonical.map((lesson) => lesson.realization.lessonId));
    const embedded = entry.embeddedLessonIds ?? [];
    const omitted = entry.omittedLessonIds ?? [];
    const declared = [...embedded, ...omitted];

    if (new Set(declared).size !== declared.length) {
      throw new Error(`${entry.language} chapter ${entry.chapter}: handwritten lesson declarations contain duplicates`);
    }
    for (const lessonId of declared) {
      if (!canonicalIds.has(lessonId)) {
        throw new Error(
          `${entry.language} chapter ${entry.chapter}: handwritten declaration '${lessonId}' is not a canonical schema-v2 lesson in this chapter`,
        );
      }
    }
    const undeclared = [...canonicalIds].filter((lessonId) => !declared.includes(lessonId));
    if (undeclared.length > 0) {
      throw new Error(
        `${entry.language} chapter ${entry.chapter}: canonical schema-v2 lesson(s) missing from handwritten coverage ledger: ${undeclared.sort().join(", ")}`,
      );
    }
    if (omitted.length > 0 && (!Number.isInteger(entry.omissionIssue) || entry.omissionIssue! < 1)) {
      throw new Error(
        `${entry.language} chapter ${entry.chapter}: omitted handwritten lessons require a positive omissionIssue`,
      );
    }
    if (omitted.length === 0 && entry.omissionIssue !== undefined) {
      throw new Error(
        `${entry.language} chapter ${entry.chapter}: omissionIssue is present but omittedLessonIds is empty`,
      );
    }
    if (embedded.length === 0) continue;

    const tex = readAuthoredFragment(root, safeOutput(root, entry.output));
    for (const lessonId of embedded) {
      if (!tex.includes(`% canonical-insertion: ${lessonId}`)) {
        throw new Error(
          `${entry.output}: embedded lesson '${lessonId}' lacks a canonical-insertion marker`,
        );
      }
      if (!tex.includes(`\\label{lesson:${lessonId}}`)) {
        throw new Error(
          `${entry.output}: embedded lesson '${lessonId}' lacks \\label{lesson:${lessonId}}`,
        );
      }
    }
  }
}

/** Public audit entry point used by focused tests and maintenance tooling. */
export function assertHandwrittenLessonCoverage(root = defaultCurriculumRoot()): void {
  assertHandwrittenLessonCoverageFrom(root, loadBookGenerationConfig(root), loadLessons(root));
}

export function generatedBookOutputs(root = defaultCurriculumRoot()): Map<string, string> {
  const capabilities = chapterCapabilityIndex(root);
  const config = loadBookGenerationConfig(root);
  if (config.version !== 1 || config.targets.length === 0) {
    throw new Error("book-generation.json must declare version 1 and at least one target");
  }
  // `sourceBaseUrl` names the canonical home of the lesson sources. It is still
  // required, and still validated, because it is the config's statement of
  // where this curriculum lives — but it no longer reaches the book renderer.
  // A printed book is a standalone artefact: a reader holding the PDF cannot
  // follow a link into a Git repository, so the book view resolves nothing
  // against this URL (see `absoluteBookLink` in book.ts). Other consumers of
  // the config keep the field.
  try {
    const parsed = new URL(config.sourceBaseUrl);
    if (parsed.protocol !== "https:" && parsed.protocol !== "http:") throw new Error();
  } catch {
    throw new Error("book-generation.json must declare an HTTP(S) sourceBaseUrl");
  }
  const lessons = loadLessons(root);
  assertHandwrittenLessonCoverageFrom(root, config, lessons);
  const policy = loadChapterPolicy(root);
  const modality = summarizeModality(lessons, {
    maxLinearisableTableColumns: policy.maxLinearisableTableColumns,
  });
  const outputs = new Map<string, string>();
  const manifests = new Map<string, GeneratedBookHashManifest>();
  for (const configuredTarget of config.targets) {
    const { scriptSet, ...plainTarget } = configuredTarget;
    const capability = requireChapterCapability(
      capabilities,
      plainTarget.language,
      plainTarget.chapter,
    );
    let target: BookGenerationTarget = {
      ...plainTarget,
      title: capability.title,
      label: capability.label,
    };
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
    // HL09 §8: a chapter opens by saying what the reader will be able to do. The
    // capability is looked up rather than authored, so the intro cannot drift from
    // the ledger the gap report measures.
    const generated = renderBookChapter(target, lessons, capability);
    safeOutput(root, target.output);
    outputs.set(target.output, generated.tex);
    let manifest = manifests.get(target.language);
    if (!manifest) {
      manifest = { version: 1, algorithm: "fnv1a64", chapters: [] };
      manifests.set(target.language, manifest);
    }
    manifest.chapters.push({
      language: target.language,
      chapter: target.chapter,
      sourceHash: generated.sourceHash,
      lessonIds: generated.lessonIds,
      tex: target.output,
    });
  }

  // HL-C15: one generated definition file per book projects the same modality model into
  // every numbered chapter, including protected handwritten chapters. Derive the
  // declared set from both config arrays, then fail closed if any declaration lacks a
  // modality rollup rather than printing a reassuring empty sign.
  const declaredByLanguage = new Map<string, Set<number>>();
  for (const entry of [...config.targets, ...(config.handwritten ?? [])]) {
    const chapters = declaredByLanguage.get(entry.language);
    if (chapters) chapters.add(entry.chapter);
    else declaredByLanguage.set(entry.language, new Set([entry.chapter]));
  }
  for (const [language, declared] of [...declaredByLanguage].sort(([left], [right]) =>
    left.localeCompare(right),
  )) {
    const track = modality.tracks.find((entry) => entry.language === language);
    if (!track) throw new Error(`${language}: declared book has no modality data`);
    const chapters = [...declared]
      .sort((left, right) => left - right)
      .map((chapter) => {
        const entry = track.chapters.find((candidate) => candidate.chapter === chapter);
        if (!entry) throw new Error(`${language} chapter ${chapter}: no modality data`);
        return entry;
      });
    const relative = `${language}/book/chapter-modalities.tex`;
    safeOutput(root, relative);
    outputs.set(relative, renderBookChapterModalities(language, chapters));
  }
  for (const configuredAppendix of config.referenceAppendices ?? []) {
    const { scriptSet, ...plainAppendix } = configuredAppendix;
    let appendix: BookReferenceAppendixTarget = { ...plainAppendix };
    if (scriptSet !== undefined) {
      if (
        appendix.inlineScripts !== undefined ||
        appendix.unicodeScript !== undefined ||
        appendix.scriptCommand !== undefined
      ) {
        throw new Error(
          `${appendix.language} reference appendix: scriptSet cannot be combined with inline script options`,
        );
      }
      const inlineScripts = config.scriptSets?.[scriptSet];
      if (!inlineScripts) {
        throw new Error(
          `${appendix.language} reference appendix: unknown scriptSet '${scriptSet}'`,
        );
      }
      appendix = { ...appendix, inlineScripts };
    }
    const source = safeMarkdownSource(root, appendix.source);
    safeOutput(root, appendix.output);
    if (!existsSync(source)) throw new Error(`${appendix.source}: reference source is missing`);
    if (outputs.has(appendix.output)) {
      throw new Error(`${appendix.output}: duplicate generated book output`);
    }
    outputs.set(
      appendix.output,
      renderReferenceAppendix(appendix, readFileSync(source, "utf8")),
    );
  }
  for (const configuredGlossary of config.glossaries ?? []) {
    const { scriptSet, ...plainGlossary } = configuredGlossary;
    let glossary: BookGlossaryTarget = { ...plainGlossary };
    if (scriptSet !== undefined) {
      if (
        glossary.inlineScripts !== undefined ||
        glossary.unicodeScript !== undefined ||
        glossary.scriptCommand !== undefined
      ) {
        throw new Error(
          `${glossary.language} glossary: scriptSet cannot be combined with inline script options`,
        );
      }
      const inlineScripts = config.scriptSets?.[scriptSet];
      if (!inlineScripts) {
        throw new Error(`${glossary.language} glossary: unknown scriptSet '${scriptSet}'`);
      }
      glossary = { ...glossary, inlineScripts };
    }
    safeOutput(root, glossary.output);
    if (outputs.has(glossary.output)) {
      throw new Error(`${glossary.output}: duplicate generated book output`);
    }
    outputs.set(glossary.output, renderBookGlossary(glossary, lessons));
  }
  for (const configuredAnswerKey of config.answerKeys ?? []) {
    const { scriptSet, ...plainAnswerKey } = configuredAnswerKey;
    let answerKey: BookAnswerKeyTarget = { ...plainAnswerKey };
    if (scriptSet !== undefined) {
      if (
        answerKey.inlineScripts !== undefined ||
        answerKey.unicodeScript !== undefined ||
        answerKey.scriptCommand !== undefined
      ) {
        throw new Error(
          `${answerKey.language} answer key: scriptSet cannot be combined with inline script options`,
        );
      }
      const inlineScripts = config.scriptSets?.[scriptSet];
      if (!inlineScripts) {
        throw new Error(`${answerKey.language} answer key: unknown scriptSet '${scriptSet}'`);
      }
      answerKey = { ...answerKey, inlineScripts };
    }
    safeOutput(root, answerKey.output);
    if (outputs.has(answerKey.output)) {
      throw new Error(`${answerKey.output}: duplicate generated book output`);
    }
    outputs.set(answerKey.output, renderBookAnswerKey(answerKey, lessons));
  }
  for (const configuredIndex of config.indexes ?? []) {
    const { scriptSet, ...plainIndex } = configuredIndex;
    let index: BookIndexTarget = { ...plainIndex };
    if (scriptSet !== undefined) {
      if (
        index.inlineScripts !== undefined ||
        index.unicodeScript !== undefined ||
        index.scriptCommand !== undefined
      ) {
        throw new Error(
          `${index.language} index: scriptSet cannot be combined with inline script options`,
        );
      }
      const inlineScripts = config.scriptSets?.[scriptSet];
      if (!inlineScripts) {
        throw new Error(`${index.language} index: unknown scriptSet '${scriptSet}'`);
      }
      index = { ...index, inlineScripts };
    }
    safeOutput(root, index.output);
    if (outputs.has(index.output)) {
      throw new Error(`${index.output}: duplicate generated book output`);
    }
    const chapters = [...config.targets, ...(config.handwritten ?? [])]
      .filter((chapter) => chapter.language === index.language)
      .map((chapter) => {
        const capability = requireChapterCapability(
          capabilities,
          chapter.language,
          chapter.chapter,
        );
        return {
          chapter: chapter.chapter,
          title: capability.title,
          label: capability.label,
        };
      });
    outputs.set(index.output, renderBookIndex(index, lessons, chapters));
  }
  for (const [language, manifest] of [...manifests].sort(([left], [right]) => left.localeCompare(right))) {
    manifest.chapters.sort((left, right) => left.chapter - right.chapter);
    outputs.set(manifestPath(language), `${JSON.stringify(manifest, null, 2)}\n`);
  }

  // HL21 section 6: `book.tex` is the last hand-maintained link in the
  // lesson -> chapter -> book chain, and the only one that could be forgotten
  // without anything failing. A chapter whose `\input` line is missing simply
  // does not appear in the book, while its `.tex` stays generated, committed and
  // hash-checked. That has already happened once.
  //
  // The authored halves live beside it as `frontmatter.tex` and
  // `backmatter.tex`; only the `\input` list in between is derived, and it is
  // derived from the ledger already in hand.
  for (const language of [...new Set(config.targets.map((target) => target.language))].sort()) {
    // Validated BEFORE the `join`, so the check covers the reads below and not
    // only the write. Checking it afterwards via `safeOutput` would still
    // contain the write, but a traversing `language` would reach `existsSync`
    // first, come back false, and `continue` — turning a manifest-integrity
    // violation into a silent no-op, which is the exact failure mode this
    // generator exists to remove.
    if (!/^[a-z][a-z0-9-]*$/.test(language)) {
      throw new Error(`unsafe track language '${language}' in book-generation.json`);
    }
    const frontPath = join(root, language, "book", FRONTMATTER_TEX);
    const backPath = join(root, language, "book", BACKMATTER_TEX);
    // A track that has not been migrated yet keeps its hand-written book.tex.
    // Same reasoning as HL21's loader fallback: this lands one track at a time
    // rather than needing a flag day.
    if (!existsSync(frontPath) || !existsSync(backPath)) continue;
    const relative = `${language}/book/book.tex`;
    safeOutput(root, relative);
    outputs.set(
      relative,
      renderBookTex(
        readAuthoredFragment(root, frontPath),
        chapterInputsFor(config, language),
        readAuthoredFragment(root, backPath),
      ),
    );
  }
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
    const output = relative.startsWith(`${BOOK_HASH_MANIFEST_DIR}/`)
      ? join(root, relative)
      : safeOutput(root, relative);
    if (mode === "--write") {
      mkdirSync(dirname(output), { recursive: true });
      // `writeFileSync` FOLLOWS a symlink, and `safeOutput` is purely lexical so
      // it cannot see one. This matters more than it used to: making
      // `<track>/book/book.tex` a generated output turns a committed symlink at
      // that path into an arbitrary-write primitive whose CONTENT is also
      // attacker-chosen (it is the authored `frontmatter.tex`, verbatim). Target
      // `~/.bashrc`, `.git/hooks/post-checkout`, a workflow file — and it fires
      // on an ordinary `npm run generate:books`.
      //
      // The read side of this feature already got `lstat` + `realpath`; leaving
      // the write side lexical would be a strange place to stop.
      const existing = statIfPresent(output);
      if (existing && !existing.isFile()) {
        throw new Error(
          `generated output '${relative}' is not a regular file — refusing to write through it`,
        );
      }
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
