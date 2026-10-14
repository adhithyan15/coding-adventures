// Script-independent derivation, captioning and placement of filmstrip targets.
// Writing-system facts live in direct owners under figure-target-policies/.

import { basename } from "node:path";
import type { FigureTarget, ScriptFilmstripTarget } from "./figure.js";
import { LIST_SEPARATORS, listCaptionParts, listItemsOf } from "./filmstrip-caption.js";
import {
  APART_CLUSTER_SOURCES,
  DERIVED_FILMSTRIP_SCRIPTS,
  FUSED_LETTER_SEQUENCE_SOURCES,
  FUSED_SIGN_PAIRS,
  figureTargetPolicy,
  HEADLINE_WORD_SCRIPTS,
  SEPARATE_LETTER_SCRIPTS,
  WRITTEN_SIGN_SIDES,
} from "./figure-target-policy.js";
import type { ParsedLesson } from "./parse.js";
import {
  filmstripBlockIndex,
  letterBlockIndex,
  MODELLED_WRITING_STAGES,
  modelledPracticeBlockIndex,
  stripBlockIndex,
} from "./strip-placement.js";

export {
  filmstripBlockIndex,
  letterBlockIndex,
  MODELLED_WRITING_STAGES,
  modelledPracticeBlockIndex,
  stripBlockIndex,
};

const GRAPHEMES = new Intl.Segmenter("und", { granularity: "grapheme" });
const BASE_LETTER = /^(?!\p{Lm})\p{L}$/u;
const SIGNS_ONLY = /^\p{M}+$/u;
const DIGIT = /^\p{Nd}$/u;

/** A one-grapheme writing headword with a block where its strip can land. */
export function writingLetterOf(lesson: ParsedLesson): string | undefined {
  if (lesson.realization.type !== "writing") return undefined;
  const headword = (lesson.realization.headword ?? "").trim();
  if (headword === "" || [...GRAPHEMES.segment(headword)].length !== 1) return undefined;
  if (stripBlockIndex(lesson) === -1) return undefined;
  return headword;
}

/** Whether an NFD word contains a cited run that its font fuses. */
export function hasFusedLetterSequence(word: string, script: string): boolean {
  const nfd = word.normalize("NFD");
  return (FUSED_LETTER_SEQUENCE_SOURCES[script] ?? []).some((source) =>
    source.sequences.some((sequence) => nfd.includes(sequence)),
  );
}

function apartClusterOf(
  nfd: string,
  script: string,
): { heads: string[]; tail: string } | undefined {
  for (const source of APART_CLUSTER_SOURCES[script] ?? []) {
    for (const cluster of source.clusters) {
      if (!nfd.startsWith(cluster)) continue;
      const parts = [...cluster];
      return {
        heads: parts.slice(0, -1),
        tail: parts.at(-1)! + nfd.slice(cluster.length),
      };
    }
  }
  return undefined;
}

/**
 * The cited pieces of one grapheme in writing order. Missing policy, an
 * unlisted cluster, a fused pair, or ambiguous same-side marks all refuse.
 */
export function writtenPiecesOf(grapheme: string, script: string): string[] | undefined {
  if (BASE_LETTER.test(grapheme) || DIGIT.test(grapheme)) return [grapheme];
  const sides = WRITTEN_SIGN_SIDES[script];
  if (sides === undefined) return undefined;

  const cluster = apartClusterOf(grapheme.normalize("NFD"), script);
  if (cluster !== undefined) {
    const heads: string[] = [];
    for (let index = 0; index < cluster.heads.length; index += 2) {
      const pieces = writtenPiecesOf(cluster.heads.slice(index, index + 2).join(""), script);
      if (pieces === undefined) return undefined;
      heads.push(...pieces);
    }
    const tail = writtenPiecesOf(cluster.tail, script);
    if (tail === undefined || tail[0] !== [...cluster.tail][0]) return undefined;
    return [...heads, ...tail];
  }

  const [first, ...rest] = [...grapheme.normalize("NFD")];
  if (first === undefined) return undefined;
  const base = BASE_LETTER.test(first) ? first : undefined;
  const signs = base === undefined ? [first, ...rest] : rest;
  if (base === undefined && !SIGNS_ONLY.test(grapheme)) return undefined;
  if (
    signs.length === 0 ||
    signs.some((sign) => !Object.prototype.hasOwnProperty.call(sides, sign))
  ) {
    return undefined;
  }
  if (base !== undefined && signs.some((sign) => FUSED_SIGN_PAIRS[script]?.has(`${base}${sign}`))) {
    return undefined;
  }
  const before = signs.filter((sign) => sides[sign] === "before");
  const after = signs.filter((sign) => sides[sign] === "after");
  if (before.length > 1 || after.length > 1) return undefined;
  return [...before, ...(base === undefined ? [] : [base]), ...after];
}

/** A list or permitted stand-apart word, decomposed into cited-order pieces. */
export function writingSequenceOf(lesson: ParsedLesson, script: string): string[] | undefined {
  if (lesson.realization.type !== "writing") return undefined;
  if (stripBlockIndex(lesson) === -1) return undefined;
  const headword = (lesson.realization.headword ?? "").trim();
  const items = headword.split(LIST_SEPARATORS).filter((item) => item !== "");
  const graphemes = items.map((item) =>
    [...GRAPHEMES.segment(item)].map((part) => part.segment),
  );
  const letters = graphemes.flat();
  if (letters.length === 0) return undefined;
  const hasTable = WRITTEN_SIGN_SIDES[script] !== undefined;
  if (letters.length === 1) {
    if (!hasTable) return undefined;
    const pieces = writtenPiecesOf(letters[0]!, script);
    return pieces !== undefined && pieces.length >= 2 ? pieces : undefined;
  }
  if (listItemsOf(headword) !== undefined) {
    return hasTable
      ? letters.flatMap((letter) => writtenPiecesOf(letter, script) ?? [letter])
      : letters;
  }
  if (!SEPARATE_LETTER_SCRIPTS.has(script)) return undefined;
  if (items.some((item) => hasFusedLetterSequence(item, script))) return undefined;
  const pieces = letters.map((letter) => writtenPiecesOf(letter, script));
  if (pieces.some((piece) => piece === undefined)) return undefined;
  return pieces.flat() as string[];
}

function isHeadlineWord(word: string, script: string): boolean {
  const policy = figureTargetPolicy(script)?.headlineWord;
  if (policy === undefined) return false;
  const characters = [...word];
  if (characters.length < 2) return false;
  const split = new Set(
    (policy.splitLetterSources ?? []).flatMap((source) => source.letters),
  );
  return characters.every((character, index) => {
    if (policy.signs?.has(character)) {
      const before = characters[index - 1];
      return before !== undefined && policy.consonant?.test(before) === true;
    }
    return (
      BASE_LETTER.test(character) &&
      policy.letter.test(character) &&
      character.normalize("NFD") === character &&
      !split.has(character)
    );
  });
}

/** A writing headword eligible for one composed shared-headline strip. */
export function headlineWordOf(lesson: ParsedLesson, script: string): string | undefined {
  if (HEADLINE_WORD_SCRIPTS[script] === undefined) return undefined;
  if (lesson.realization.type !== "writing") return undefined;
  if (stripBlockIndex(lesson) === -1) return undefined;
  const word = (lesson.realization.headword ?? "").trim();
  if (word === "" || LIST_SEPARATORS.test(word)) return undefined;
  return isHeadlineWord(word, script) ? word : undefined;
}

/** A phrase eligible for word-by-word shared-headline composition. */
export function headlinePhraseOf(lesson: ParsedLesson, script: string): string[] | undefined {
  const letterScript = HEADLINE_WORD_SCRIPTS[script];
  if (letterScript === undefined) return undefined;
  if (lesson.realization.type !== "writing") return undefined;
  if (stripBlockIndex(lesson) === -1) return undefined;
  const phrase = (lesson.realization.headword ?? "").trim();
  if (!/^\S+(?: \S+)+$/u.test(phrase)) return undefined;
  const words = phrase.split(" ");
  const oneLetter = (word: string): boolean =>
    [...word].length === 1 &&
    BASE_LETTER.test(word) &&
    letterScript.test(word) &&
    word.normalize("NFD") === word;
  if (!words.every((word) => isHeadlineWord(word, script) || oneLetter(word))) return undefined;
  if (words.every(oneLetter)) return undefined;
  if (words.length > MAX_PHRASE_WORDS) return undefined;
  if ([...words.join("")].length > MAX_SEQUENCE_PIECES) return undefined;
  return words;
}

/** Safety bounds measured against the largest legible generated strips. */
export const MAX_PHRASE_WORDS = 3;
export const MAX_SEQUENCE_PIECES = 10;

/** Every writing lesson on an activated track that could carry a filmstrip. */
export function filmstripCandidates(
  lessons: readonly ParsedLesson[],
  scripts: Readonly<Record<string, string>> = DERIVED_FILMSTRIP_SCRIPTS,
): ScriptFilmstripTarget[] {
  const candidates: ScriptFilmstripTarget[] = [];
  for (const lesson of lessons) {
    const script = scripts[lesson.language];
    if (script === undefined) continue;
    const lessonId = lesson.realization.lessonId;
    const output = `${lesson.language}/book/figures/${lessonId}-filmstrip.svg`;
    const letters = writingSequenceOf(lesson, script);
    if (letters !== undefined && letters.length > MAX_SEQUENCE_PIECES) continue;
    if (letters === undefined) {
      const words = headlinePhraseOf(lesson, script);
      if (words !== undefined) {
        candidates.push({
          kind: "script-filmstrip",
          lessonId,
          script,
          glyph: (lesson.realization.headword ?? "").trim(),
          letters: words,
          composition: "shared-headline",
          output,
        });
        continue;
      }
      const word = headlineWordOf(lesson, script);
      if (word !== undefined) {
        candidates.push({
          kind: "script-filmstrip",
          lessonId,
          script,
          glyph: word,
          composition: "shared-headline",
          output,
        });
        continue;
      }
      const glyph = writingLetterOf(lesson);
      if (glyph !== undefined) {
        candidates.push({ kind: "script-filmstrip", lessonId, script, glyph, output });
      }
      continue;
    }
    candidates.push({
      kind: "script-filmstrip",
      lessonId,
      script,
      glyph: (lesson.realization.headword ?? "").trim(),
      letters,
      output,
    });
  }
  return candidates.sort((left, right) => left.lessonId.localeCompare(right.lessonId));
}

export function filmstripLetters(target: ScriptFilmstripTarget): readonly string[] {
  return target.letters ?? [target.glyph];
}

/** Merge only fully cited derived candidates; explicit targets win by lesson. */
export function withDerivedFilmstrips(
  explicit: readonly FigureTarget[],
  candidates: readonly ScriptFilmstripTarget[],
  hasDuctus: (script: string, glyph: string) => boolean,
): FigureTarget[] {
  const declared = new Set(
    explicit
      .filter((target) => target.kind === "script-filmstrip")
      .map((target) => target.lessonId),
  );
  return [
    ...explicit,
    ...candidates.filter(
      (candidate) =>
        !declared.has(candidate.lessonId) &&
        filmstripLetters(candidate).every((glyph) => hasDuctus(candidate.script, glyph)),
    ),
  ];
}

/** The accessible Markdown image caption for one filmstrip target. */
export function filmstripImageMarkdown(target: ScriptFilmstripTarget): string {
  const file = `figures/${basename(target.output)}`;
  if (target.composition === "shared-headline" && target.letters !== undefined) {
    return `![How ${target.glyph} is written, word by word, each with its own headline, stroke by stroke](${file})`;
  }
  if (target.composition === "shared-headline") {
    return `![How ${target.glyph} is written, letter by letter, then one headline, stroke by stroke](${file})`;
  }
  if (target.letters === undefined) {
    return `![How ${target.glyph} is written, stroke by stroke](${file})`;
  }
  const manner = target.letters.some((letter) => SIGNS_ONLY.test(letter))
    ? "part by part"
    : undefined;
  const items = listItemsOf(target.glyph);
  if (items !== undefined) {
    const list = listCaptionParts(items);
    return `![${list.subject}, ${manner ?? "one after another"}, stroke by stroke: ${list.items}](${file})`;
  }
  if (manner !== undefined) {
    return `![How ${target.glyph} is written, ${manner}, stroke by stroke](${file})`;
  }
  const what = target.letters.join("") === target.glyph
    ? `How ${target.glyph} is written`
    : `How the letters ${target.glyph} are written`;
  return `![${what}, letter by letter, stroke by stroke](${file})`;
}

/** Insert each filmstrip exactly once at the stable landing block. */
export function withFilmstripImages(
  lessons: readonly ParsedLesson[],
  targets: readonly FigureTarget[],
): ParsedLesson[] {
  const byLesson = new Map<string, ScriptFilmstripTarget>();
  for (const target of targets) {
    if (target.kind === "script-filmstrip") byLesson.set(target.lessonId, target);
  }
  return lessons.map((lesson) => {
    const target = byLesson.get(lesson.realization.lessonId);
    if (target === undefined) return lesson;
    const file = `figures/${basename(target.output)}`;
    if (lesson.blocks.some((block) => block.markdown.includes(file))) return lesson;
    const at = filmstripBlockIndex(lesson);
    if (at === -1) return lesson;
    const blocks = lesson.blocks.map((block, index) =>
      index === at
        ? {
            ...block,
            markdown: `${filmstripImageMarkdown(target)}\n\n${block.markdown.replace(/^\s+/, "")}`,
          }
        : block,
    );
    return { ...lesson, blocks };
  });
}
