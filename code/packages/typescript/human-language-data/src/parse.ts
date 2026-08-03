// Turn lesson frontmatter into realizations, and realizations into a dataset.
// Pure functions only — they take strings/objects in and return data out, with
// no filesystem access, so they're trivially unit-testable. The fs boundary
// lives in loader.ts.

import { splitFrontmatter, type Frontmatter } from "./frontmatter.js";
import { LANGUAGE_SCRIPT, CONTENT_TYPES, hasOwn } from "./constants.js";
import { canonicalLessonHash } from "./hash.js";
import type {
  Concept,
  Dataset,
  Gender,
  LessonBodyBlock,
  LessonBlockKnowledge,
  LessonBlockType,
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

function classifyBlock(title: string): LessonBlockType {
  const normalized = title.toLowerCase();
  if (normalized === "warm-up" || normalized === "warmup") return "warmup";
  if (normalized.startsWith("you'll want to know")) return "input";
  if (normalized.startsWith("sounds you'll need")) return "pronunciation";
  if (normalized.startsWith("script")) return "script";
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

function directiveList(value: string): string[] {
  return value
    .split(",")
    .map((item) => item.trim())
    .filter((item) => item !== "");
}

function parseBlockKnowledge(markdown: string): {
  markdown: string;
  knowledge?: LessonBlockKnowledge;
  knowledgeDirectiveError?: string;
} {
  const lines = markdown.split("\n");
  const directiveIndexes = lines
    .map((line, index) => (line.includes("hl-knowledge") ? index : -1))
    .filter((index) => index >= 0);
  if (directiveIndexes.length === 0) return { markdown };

  const firstContent = lines.findIndex((line) => line.trim() !== "");
  const directiveIndex = directiveIndexes[0] ?? -1;
  const directive = lines[directiveIndex]?.trim() ?? "";
  const match = KNOWLEDGE_DIRECTIVE.exec(directive);
  if (directiveIndexes.length !== 1 || directiveIndex !== firstContent || !match) {
    return {
      markdown,
      knowledgeDirectiveError:
        "expected one first-line '<!-- hl-knowledge: introduces=[...]; assesses=[...] -->' directive",
    };
  }

  lines.splice(directiveIndex, 1);
  return {
    markdown: lines.join("\n").replace(/^\n|\n$/g, ""),
    knowledge: {
      introduces: directiveList(match[1] ?? ""),
      assesses: directiveList(match[2] ?? ""),
    },
  };
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
    const parsedKnowledge = parseBlockKnowledge(
      blockLines.join("\n").replace(/^\n|\n$/g, ""),
    );
    current.markdown = parsedKnowledge.markdown;
    current.knowledge = parsedKnowledge.knowledge;
    current.knowledgeDirectiveError = parsedKnowledge.knowledgeDirectiveError;
    if (current.knowledge === undefined) delete current.knowledge;
    if (current.knowledgeDirectiveError === undefined) delete current.knowledgeDirectiveError;
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

  const realization: Realization = {
    concept: str(fm.concept_tag),
    language,
    lessonId: str(fm.id),
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
