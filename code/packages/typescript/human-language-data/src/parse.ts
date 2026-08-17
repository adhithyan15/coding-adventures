// Turn lesson frontmatter into realizations, and realizations into a dataset.
// Pure functions only — they take strings/objects in and return data out, with
// no filesystem access, so they're trivially unit-testable. The fs boundary
// lives in loader.ts.

import { splitFrontmatter, type Frontmatter } from "./frontmatter.js";
import { LANGUAGE_SCRIPT, CONTENT_TYPES, hasOwn } from "./constants.js";
import { canonicalLessonHash } from "./hash.js";
import { parseLessonActivityValue } from "./activity.js";
import type {
  Concept,
  Dataset,
  Gender,
  LessonBodyBlock,
  LessonBlockKnowledge,
  LessonBlockType,
  LessonPatternSlot,
  Realization,
  Script,
  Taxonomy,
} from "./types.js";

/** A lesson after parsing: its raw frontmatter kept alongside the derived row. */
export interface ParsedLesson {
  language: string;
  script: Script;
  frontmatter: Frontmatter;
  /** Lossless Markdown after the frontmatter, used by both books and apps. */
  body: string;
  /** Markdown before the first level-two teaching block, normally the title. */
  preamble: string;
  /** Typed, ordered body blocks for schema-v2 validation and rendering. */
  blocks: LessonBodyBlock[];
  /** Ordered substitution points declared by a productive `pattern` lesson. */
  patternSlots?: LessonPatternSlot[];
  /** Deterministic fingerprint of the canonical frontmatter and typed body AST. */
  sourceHash: string;
  realization: Realization;
}

function arrayify(value: Frontmatter[string] | undefined): string[] {
  if (Array.isArray(value)) return value;
  if (typeof value === "string" && value.trim() !== "") return [value];
  return [];
}

function str(value: Frontmatter[string] | undefined): string {
  return typeof value === "string" ? value : "";
}

/**
 * Parse the tiny-YAML representation of HL05's ordered pattern slots.
 *
 * The frontmatter reader deliberately supports only one nested-map level, so
 * authors write an indented `slots:` map and receive flattened
 * `slots.infinitive` keys here. Object insertion order preserves the authored
 * slot order for book and app renderers.
 */
function parsePatternSlots(frontmatter: Frontmatter): LessonPatternSlot[] {
  return Object.entries(frontmatter)
    .filter(([key]) => key.startsWith("slots."))
    .map(([key, value]) => ({
      name: key.slice("slots.".length),
      fillers: arrayify(value),
    }));
}

function classifyBlock(title: string): LessonBlockType {
  const normalized = title.toLowerCase();
  if (normalized === "warm-up" || normalized === "warmup") return "warmup";
  if (normalized.startsWith("you'll want to know")) return "input";
  if (normalized.startsWith("sounds you'll need")) return "pronunciation";
  if (normalized.startsWith("script")) return "script";
  // "The letters in this word" is HL00's inline-letters section: the place a word lesson
  // teaches the glyphs that word needs. It is a `script` block in everything but name —
  // 240 lessons across 12 tracks use this exact heading — and it mapped to `unknown`,
  // which schema v2 rejects. That single gap blocked the v2 migration for every Indic
  // track at once. Classifying it honestly costs drivability (a letter shape cannot be
  // read aloud) and buys a migration path; the driving edition is a filter over the
  // modality flag, not a quality bar, so the honest label is the right one.
  if (normalized.includes("letters in this word")) return "script";
  // "Writing:" opens a hand-formation section — the one detachable block type.
  // Checked before the looser prefixes below because a writing section's title
  // normally names the letter it teaches ("Writing: మ — the tick on top").
  if (normalized.startsWith("writing")) return "writing";
  if (normalized.includes("taken apart")) return "etymology";
  if (normalized.startsWith("why it's said this way")) return "culture-pragmatics";
  if (
    normalized.startsWith("grammar lens") ||
    normalized.startsWith("the adjective sibling") ||
    normalized.startsWith("its two tags")
  ) return "grammar";
  if (normalized.startsWith("guided practice") || normalized.startsWith("how to answer")) {
    return "guided-production";
  }
  if (normalized.startsWith("wrap-up recall")) return "recall";
  if (normalized.startsWith("what you've built")) return "notice";
  if (normalized.startsWith("the exchange") || normalized.startsWith("the two words")) return "input";
  return "unknown";
}

const KNOWLEDGE_DIRECTIVE =
  /^<!--\s*hl-knowledge:\s*introduces=\[([^\]]*)\];\s*assesses=\[([^\]]*)\]\s*-->$/;
const ACTIVITY_DIRECTIVE = /^<!--\s*hl-activity:\s*(\{.*\})\s*-->$/;

function directiveList(value: string): string[] {
  return value
    .split(",")
    .map((item) => item.trim())
    .filter((item) => item !== "");
}

function parseBlockMetadata(markdown: string): {
  markdown: string;
  knowledge?: LessonBlockKnowledge;
  knowledgeDirectiveError?: string;
  activities?: LessonBodyBlock["activities"];
  activityDirectiveErrors?: string[];
} {
  const lines = markdown.split("\n");
  const knowledgeIndexes = lines
    .map((line, index) => (line.includes("hl-knowledge") ? index : -1))
    .filter((index) => index >= 0);
  const activityIndexes = lines
    .map((line, index) => (line.includes("hl-activity") ? index : -1))
    .filter((index) => index >= 0);
  if (knowledgeIndexes.length === 0 && activityIndexes.length === 0) return { markdown };

  const firstContent = lines.findIndex((line) => line.trim() !== "");
  const knowledgeIndex = knowledgeIndexes[0] ?? -1;
  const knowledgeDirective = lines[knowledgeIndex]?.trim() ?? "";
  const knowledgeMatch = KNOWLEDGE_DIRECTIVE.exec(knowledgeDirective);
  const validKnowledge =
    knowledgeIndexes.length === 1 && knowledgeIndex === firstContent && knowledgeMatch !== null;
  const removeIndexes = new Set<number>();
  let knowledge: LessonBlockKnowledge | undefined;
  let knowledgeDirectiveError: string | undefined;
  if (knowledgeIndexes.length > 0) {
    if (!validKnowledge || !knowledgeMatch) {
      knowledgeDirectiveError =
        "expected one first-line '<!-- hl-knowledge: introduces=[...]; assesses=[...] -->' directive";
    } else {
      knowledge = {
        introduces: directiveList(knowledgeMatch[1] ?? ""),
        assesses: directiveList(knowledgeMatch[2] ?? ""),
      };
      removeIndexes.add(knowledgeIndex);
    }
  }

  const activities: NonNullable<LessonBodyBlock["activities"]> = [];
  const activityDirectiveErrors: string[] = [];
  if (activityIndexes.length > 0) {
    const firstDisplayIndex = lines.findIndex((line, index) =>
      line.trim() !== "" && index !== knowledgeIndex && !activityIndexes.includes(index),
    );
    for (const index of activityIndexes) {
      const positionIsValid =
        validKnowledge &&
        index > knowledgeIndex &&
        (firstDisplayIndex === -1 || index < firstDisplayIndex);
      if (!positionIsValid) {
        activityDirectiveErrors.push(
          "activity directives must follow the first-line hl-knowledge directive before learner copy",
        );
        continue;
      }
      const directive = lines[index]?.trim() ?? "";
      const match = ACTIVITY_DIRECTIVE.exec(directive);
      if (!match) {
        activityDirectiveErrors.push("expected '<!-- hl-activity: {...} -->'");
        continue;
      }
      let value: unknown;
      try {
        value = JSON.parse(match[1] ?? "");
      } catch {
        activityDirectiveErrors.push("contains invalid JSON");
        continue;
      }
      const parsed = parseLessonActivityValue(value);
      if (!parsed.activity) {
        activityDirectiveErrors.push(parsed.error ?? "contains an invalid activity object");
        continue;
      }
      activities.push(parsed.activity);
      removeIndexes.add(index);
    }
  }

  const result = {
    markdown: lines
      .filter((_line, index) => !removeIndexes.has(index))
      .join("\n")
      .replace(/^\n|\n$/g, ""),
    knowledge,
    knowledgeDirectiveError,
    activities: activities.length > 0 ? activities : undefined,
    activityDirectiveErrors:
      activityDirectiveErrors.length > 0 ? activityDirectiveErrors : undefined,
  };
  if (result.knowledge === undefined) delete result.knowledge;
  if (result.knowledgeDirectiveError === undefined) delete result.knowledgeDirectiveError;
  if (result.activities === undefined) delete result.activities;
  if (result.activityDirectiveErrors === undefined) delete result.activityDirectiveErrors;
  return result;
}

/** Parse level-two lesson sections into the stable HL04 body-block vocabulary. */
export function parseBodyBlocks(body: string): {
  preamble: string;
  blocks: LessonBodyBlock[];
} {
  const lines = body.split(/\r?\n/);
  const preamble: string[] = [];
  const blocks: LessonBodyBlock[] = [];
  let current: LessonBodyBlock | undefined;
  const blockLines: string[] = [];
  const finish = (): void => {
    if (!current) return;
    const parsedMetadata = parseBlockMetadata(
      blockLines.join("\n").replace(/^\n|\n$/g, ""),
    );
    current.markdown = parsedMetadata.markdown;
    current.knowledge = parsedMetadata.knowledge;
    current.knowledgeDirectiveError = parsedMetadata.knowledgeDirectiveError;
    current.activities = parsedMetadata.activities;
    current.activityDirectiveErrors = parsedMetadata.activityDirectiveErrors;
    if (current.knowledge === undefined) delete current.knowledge;
    if (current.knowledgeDirectiveError === undefined) delete current.knowledgeDirectiveError;
    if (current.activities === undefined) delete current.activities;
    if (current.activityDirectiveErrors === undefined) delete current.activityDirectiveErrors;
    blocks.push(current);
    blockLines.length = 0;
  };
  for (const line of lines) {
    const trimmed = line.trimStart();
    if (trimmed.startsWith("## ") && !trimmed.startsWith("### ")) {
      finish();
      const title = trimmed.slice(3).trim();
      current = { type: classifyBlock(title), title, markdown: "" };
    } else if (current) {
      blockLines.push(line);
    } else {
      preamble.push(line);
    }
  }
  finish();
  return {
    preamble: preamble.join("\n").replace(/^\n|\n$/g, ""),
    blocks,
  };
}

/** Gender: prefer an explicit field, else sniff the gloss, else none. */
function deriveGender(fm: Frontmatter, gloss: string): Gender {
  const explicit = str(fm.gender).toLowerCase();
  if (explicit === "masc" || explicit === "masculine") return "masc";
  if (explicit === "fem" || explicit === "feminine") return "fem";
  if (explicit === "neut" || explicit === "neuter") return "neut";
  if (/\bmasc/i.test(gloss)) return "masc";
  if (/\bfem/i.test(gloss)) return "fem";
  if (/\bneut/i.test(gloss)) return "neut";
  return null;
}

/**
 * Parse one lesson file's text into a ParsedLesson. `language` is the track slug
 * (the lessons/ directory's parent). `script` may be passed in (the loader
 * resolves it from the track's `track.json`); when omitted it falls back to the
 * built-in LANGUAGE_SCRIPT map, then to `latin`. Missing fields are left
 * empty/zero here and flagged later by the validator, so parsing never throws.
 */
export function parseLesson(
  source: string,
  language: string,
  script?: Script,
): ParsedLesson {
  const { frontmatter, body } = splitFrontmatter(source);
  const fm = frontmatter ?? {};
  const resolvedScript: Script =
    script ?? (hasOwn(LANGUAGE_SCRIPT, language) ? LANGUAGE_SCRIPT[language] : "latin");
  const headword = str(fm.headword);
  const gloss = str(fm.gloss);
  const chapterRaw = str(fm.chapter);
  const romanization =
    str(fm.romanization) || (resolvedScript === "latin" ? headword : "");

  // A lesson id is interpolated RAW into `\label{lesson:<id>}` and into the
  // `% canonical-lessons:` header of every generated .tex. It is the sibling of
  // the `chapters.json` label hole closed in HL-C209, found by the review of the
  // very next tranche: an id of `X}\write18{...}{` closes the brace and emits a
  // live control sequence into a file XeLaTeX compiles in CI. Builds run without
  // `--shell-escape` so `\write18` is refused, but `\input` and `\openout` are
  // not -- an arbitrary local file read into a published PDF.
  //
  // Same shape as `ACTIVITY_ID` in `activity.ts`, which has always been guarded.
  // Every id in the corpus already matches; this refuses the one that would not.
  const lessonId = str(fm.id);
  if (lessonId !== "" && !/^[A-Za-z0-9]+(?:-[A-Za-z0-9]+)*$/.test(lessonId)) {
    throw new Error(`lesson id must be a stable hyphenated token, got '${lessonId}'`);
  }

  const realization: Realization = {
    concept: str(fm.concept_tag),
    language,
    lessonId,
    chapter: chapterRaw === "" ? NaN : Number(chapterRaw),
    type: str(fm.type) || "word",
    headword,
    gloss,
    romanization,
    script: resolvedScript,
    gender: deriveGender(fm, gloss),
    sounds: arrayify(fm.sounds),
    roots: arrayify(fm.roots),
    etymologyHook: str(fm.etymology_hook),
  };
  const typedBody = parseBodyBlocks(body);
  const patternSlots = parsePatternSlots(fm);
  const parsed: ParsedLesson = {
    language,
    script: resolvedScript,
    frontmatter: fm,
    body,
    preamble: typedBody.preamble,
    blocks: typedBody.blocks,
    sourceHash: "",
    realization,
  };
  if (patternSlots.length > 0) parsed.patternSlots = patternSlots;
  parsed.sourceHash = canonicalLessonHash(parsed);
  return parsed;
}

/**
 * Assemble parsed lessons + the taxonomy into the queryable Dataset. Only
 * content lessons (word/phrase) become concepts/realizations — practice and
 * review lessons are session labels, not concepts.
 */
export function buildDataset(taxonomy: Taxonomy, lessons: ParsedLesson[]): Dataset {
  const content = lessons.filter((l) => CONTENT_TYPES.has(l.realization.type));

  // null-prototype maps so a stray `__proto__`/`constructor` key from a
  // filename-derived language or a concept tag can't collide with inherited members.
  const byConcept = new Map<string, Realization[]>();
  const byLanguage: Record<string, Realization[]> = Object.create(null);
  for (const { realization } of content) {
    if (realization.concept === "") continue;
    (byConcept.get(realization.concept) ?? setGet(byConcept, realization.concept)).push(
      realization,
    );
    (byLanguage[realization.language] ??= []).push(realization);
  }

  const concepts: Concept[] = [];
  for (const [id, realizations] of byConcept) {
    const canon = hasOwn(taxonomy.concepts, id) ? taxonomy.concepts[id] : undefined;
    concepts.push({
      id,
      family: canon?.family ?? "(namespaced)",
      gloss: canon?.gloss ?? realizations[0]?.gloss ?? "",
      core: canon?.core ?? false,
      namespaced: canon === undefined,
      realizations,
    });
  }
  concepts.sort((a, b) => a.id.localeCompare(b.id));

  return {
    taxonomy,
    concepts,
    byLanguage,
    languages: Object.keys(byLanguage).sort(),
  };
}

// Small helper: get-or-create a bucket in a Map.
function setGet<K, V>(map: Map<K, V[]>, key: K): V[] {
  const fresh: V[] = [];
  map.set(key, fresh);
  return fresh;
}
