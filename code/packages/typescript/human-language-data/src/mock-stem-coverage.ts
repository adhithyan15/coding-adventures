// ---------------------------------------------------------------------------
// mock-stem-coverage.ts — what the QUESTION asks, against what the book taught.
//
// WHY THIS EXISTS
// ---------------
// `spanish-a1-mock-audit-cli.ts` decides whether a mock item is answerable by
// reading its `requires` row out of the answer key. Those rows are hand-authored,
// and HL-C421 found they list the words of the AUDIO PASSAGE and nothing else.
// So an item could pass while the option a candidate has to choose read
// `el nivel era demasiado alto` — with `nivel` taught nowhere.
//
// Repairing the rows was done by hand, three times, and the count went
// 6 → 15 → 17 → 32 → 48. Every one of the thirty-eight corrections ran in the
// same direction: the narrowing had been too generous. That is not a near miss,
// it is a BIAS, and a fourth hand pass would inherit it.
//
// This module is the repeatable version of that pass. It does not gate anything
// and it does not decide anything. It sorts the words of every stem and option
// into buckets and prints the two a reader has to look at.
//
// THE DESIGN RULE THAT MATTERS: NOTHING IS SILENTLY DROPPED
// ---------------------------------------------------------
// The hand passes failed by CLEARING words. A 3-character prefix matched
// `espacio` to the taught `esperar` — unrelated words — and `espacio` vanished
// from the working list, so nobody looked at it again until review found it in
// the stem of an item that was passing.
//
// So this reporter never clears a word on a morphological guess. A form with a
// plausible taught relative goes into `derivable`, which is PRINTED, beside the
// taught word it matched, so the reader can reject the match in a second. Only
// four things remove a word from the report entirely, and each is a list
// somebody wrote down in `core/spanish-mock-stem-vocabulary.json`: it is a
// taught form, a closed-class function word or auxiliary, exam apparatus, or a
// proper noun.
//
//     espacio → derivable, matched esperar        <- printed, obviously wrong
//     espacio → (cleared by prefix rule)          <- what the hand pass did
//
// The reporter is therefore allowed to be noisy. A false `derivable` costs a
// reader one glance; a false clear costs an exam item.
// ---------------------------------------------------------------------------
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { defaultCurriculumRoot } from "./loader.js";
import { readLedgerFile } from "./shard.js";
import { spanishTaughtForms, type MockAuditLevel } from "./spanish-a1-mock-audit-cli.js";

export const STEM_VOCABULARY_PATH = "core/spanish-mock-stem-vocabulary.json";

export interface StemVocabulary {
  readonly functionWords: readonly string[];
  readonly auxiliaryForms: readonly string[];
  readonly examApparatus: readonly string[];
  readonly properNouns: readonly string[];
  readonly derivation: {
    readonly minStemLength: number;
    readonly verbEndings: readonly string[];
    readonly nominalSuffixes: readonly string[];
    readonly stemChanges: readonly (readonly string[])[];
  };
}

/**
 * Load and SHAPE-CHECK the declared vocabulary.
 *
 * Every list is indexed into or iterated below, and a wrong type there fails in
 * the direction that hides words: a string where an array belongs turns
 * `includes` into a SUBSTRING test, so `a` would match every function word on
 * the list and the report would come back empty and look clean.
 * `root-slug-splits.ts` carries the same guard for the same reason, and learned
 * it the same way — from a security review.
 */
export function loadStemVocabulary(root = defaultCurriculumRoot()): StemVocabulary {
  const raw = readLedgerFile<unknown>(resolve(root, STEM_VOCABULARY_PATH));
  const asRecord = raw as Record<string, unknown>;
  const stringList = (key: string): readonly string[] => {
    const value = asRecord?.[key];
    if (!Array.isArray(value) || !value.every((entry) => typeof entry === "string")) {
      throw new Error(`${STEM_VOCABULARY_PATH}: '${key}' must be an array of strings`);
    }
    return value;
  };
  if (raw === null || typeof raw !== "object") {
    throw new Error(`${STEM_VOCABULARY_PATH}: expected an object`);
  }
  const derivation = asRecord.derivation as Record<string, unknown> | undefined;
  if (derivation === undefined || typeof derivation !== "object" || derivation === null) {
    throw new Error(`${STEM_VOCABULARY_PATH}: 'derivation' must be an object`);
  }
  const changes = derivation.stemChanges;
  if (
    !Array.isArray(changes) ||
    !changes.every(
      (pair) => Array.isArray(pair) && pair.length === 2 && pair.every((s) => typeof s === "string"),
    )
  ) {
    throw new Error(`${STEM_VOCABULARY_PATH}: 'derivation.stemChanges' must be [from, to] pairs`);
  }
  const minStemLength = derivation.minStemLength;
  if (typeof minStemLength !== "number" || !Number.isInteger(minStemLength) || minStemLength < 1) {
    throw new Error(`${STEM_VOCABULARY_PATH}: 'derivation.minStemLength' must be a positive integer`);
  }
  const endings = derivation.verbEndings;
  const suffixes = derivation.nominalSuffixes;
  for (const [name, list] of [["verbEndings", endings], ["nominalSuffixes", suffixes]] as const) {
    if (!Array.isArray(list) || !list.every((entry) => typeof entry === "string")) {
      throw new Error(`${STEM_VOCABULARY_PATH}: 'derivation.${name}' must be an array of strings`);
    }
  }
  return {
    functionWords: stringList("functionWords"),
    auxiliaryForms: stringList("auxiliaryForms"),
    examApparatus: stringList("examApparatus"),
    properNouns: stringList("properNouns"),
    derivation: {
      minStemLength,
      verbEndings: endings as readonly string[],
      nominalSuffixes: suffixes as readonly string[],
      stemChanges: changes as readonly (readonly string[])[],
    },
  };
}

/** One question, as the candidate meets it. */
export interface PaperItem {
  readonly item: number;
  readonly stem: string;
  readonly options: readonly string[];
}

/**
 * Pull the items out of a mock paper.
 *
 * The shape is `**12.** stem`, then `- a) option` lines. Tarea 4 items are
 * matched against a shared *Enunciados* block rather than carrying their own
 * options, so an item there legitimately has none; that is why `options` may be
 * empty and why the reporter reads the whole paper for those, not just the
 * per-item lines.
 */
export function parseMockPaper(text: string): PaperItem[] {
  const items: PaperItem[] = [];
  let current: { item: number; stem: string; options: string[] } | undefined;
  for (const line of text.split(/\r?\n/)) {
    const stem = /^\*\*(\d+)\.\*\*\s*(.*)$/.exec(line);
    if (stem) {
      current = { item: Number(stem[1]), stem: stem[2] ?? "", options: [] };
      items.push(current);
      continue;
    }
    const option = /^-\s*[a-z]\)\s*(.*)$/i.exec(line);
    if (option && current !== undefined) current.options.push(option[1] ?? "");
  }
  return items;
}

/**
 * Word forms, lower-cased, accents kept.
 *
 * Accents are KEPT because `título` and `titulo` are different strings to the
 * taught set, and folding here would quietly credit one for the other. The
 * character class is explicit rather than `\w`, which in JavaScript excludes
 * every accented letter and would split `mañana` into `ma` and `ana`.
 */
export function wordForms(text: string): string[] {
  return text.toLowerCase().normalize("NFC").match(/[a-záéíóúüñ]+/g) ?? [];
}

export type Bucket =
  | "taught"
  | "function-word"
  | "apparatus"
  | "proper-noun"
  | "in-requires"
  | "derivable"
  | "unaccounted";

export interface FormReport {
  readonly form: string;
  readonly bucket: Bucket;
  /** For `derivable`: the taught form it matched, so a reader can reject it. */
  readonly matched?: string;
  /** Where it appears: `12 stem` or `12 option`. */
  readonly places: readonly string[];
}

/** Every way the declared rules can cut a form down to a possible stem. */
function candidateStems(form: string, vocabulary: StemVocabulary): Set<string> {
  const { minStemLength, verbEndings, nominalSuffixes, stemChanges } = vocabulary.derivation;
  const out = new Set<string>([form]);
  for (const ending of [...verbEndings, ...nominalSuffixes]) {
    if (form.endsWith(ending) && form.length - ending.length >= minStemLength) {
      out.add(form.slice(0, form.length - ending.length));
    }
  }
  for (const stem of [...out]) {
    for (const [from, to] of stemChanges) {
      if (stem.includes(from)) out.add(stem.replace(from, to));
    }
  }
  return out;
}

/**
 * Sort every form of every stem and option into a bucket.
 *
 * `requiresByItem` is what the answer key already claims the item needs; a form
 * already listed there is `in-requires` and needs no further thought, which is
 * what makes this report shrink as the rows are repaired.
 */
export function reportStemCoverage(
  items: readonly PaperItem[],
  taught: ReadonlySet<string>,
  requiresByItem: ReadonlyMap<number, ReadonlySet<string>>,
  vocabulary: StemVocabulary,
): FormReport[] {
  const functionWords = new Set(vocabulary.functionWords);
  const auxiliaries = new Set(vocabulary.auxiliaryForms);
  const apparatus = new Set(vocabulary.examApparatus);
  const properNouns = new Set(vocabulary.properNouns);

  // Taught stems, so a `derivable` match can name the word it matched. Built
  // once: this is the only loop over the taught set, and it is the expensive one.
  const taughtStems = new Map<string, string>();
  for (const word of taught) {
    for (const stem of candidateStems(word, vocabulary)) {
      if (stem.length >= vocabulary.derivation.minStemLength && !taughtStems.has(stem)) {
        taughtStems.set(stem, word);
      }
    }
  }

  // A `requires` row is written in CITATION forms -- `alumno`, `utilizar` --
  // and the paper carries inflected ones -- `alumnos`, `utilizaba`. Comparing
  // the two as strings re-flags every word already accounted for, which is how
  // a first run of this reporter listed `alumnos` as unaccounted in the very
  // item whose row had just been repaired with `alumno`. The row's entries get
  // the same stem expansion the taught set does.
  const requiredStems = new Map<number, Map<string, string>>();
  for (const [item, entries] of requiresByItem) {
    const stems = new Map<string, string>();
    for (const entry of entries) {
      for (const stem of candidateStems(entry, vocabulary)) {
        if (stem.length >= vocabulary.derivation.minStemLength && !stems.has(stem)) {
          stems.set(stem, entry);
        }
      }
    }
    requiredStems.set(item, stems);
  }

  const places = new Map<string, Set<string>>();
  const seen = new Map<string, Bucket>();
  const matches = new Map<string, string>();
  for (const item of items) {
    const required = requiresByItem.get(item.item) ?? new Set<string>();
    const requiredByStem = requiredStems.get(item.item) ?? new Map<string, string>();
    const sources: [string, string][] = [
      [item.stem, "stem"],
      ...item.options.map((option): [string, string] => [option, "option"]),
    ];
    for (const [text, where] of sources) {
      for (const form of wordForms(text)) {
        places.set(form, (places.get(form) ?? new Set()).add(`${item.item} ${where}`));
        if (seen.has(form) && seen.get(form) !== "unaccounted") continue;
        let bucket: Bucket;
        if (taught.has(form)) bucket = "taught";
        else if (functionWords.has(form) || auxiliaries.has(form)) bucket = "function-word";
        else if (apparatus.has(form)) bucket = "apparatus";
        else if (properNouns.has(form)) bucket = "proper-noun";
        else if (required.has(form)) bucket = "in-requires";
        else {
          const stems = [...candidateStems(form, vocabulary)];
          const claimed = stems.map((stem) => requiredByStem.get(stem)).find((w) => w !== undefined);
          if (claimed !== undefined) {
            bucket = "in-requires";
            matches.set(form, claimed);
          } else {
            const match = stems.map((stem) => taughtStems.get(stem)).find((w) => w !== undefined);
            if (match !== undefined) {
              bucket = "derivable";
              matches.set(form, match);
            } else bucket = "unaccounted";
          }
        }
        seen.set(form, bucket);
      }
    }
  }

  return [...seen]
    .map(([form, bucket]): FormReport => ({
      form,
      bucket,
      ...(matches.has(form) ? { matched: matches.get(form)! } : {}),
      places: [...(places.get(form) ?? [])].sort(),
    }))
    .sort((left, right) => (left.form < right.form ? -1 : left.form > right.form ? 1 : 0));
}

/** Load the papers and their rows, and report. Convenience for the CLI and tests. */
export function reportSpanishMockStemCoverage(
  root = defaultCurriculumRoot(),
  level: MockAuditLevel = "A2",
): { mock: number; forms: FormReport[] }[] {
  const vocabulary = loadStemVocabulary(root);
  const { taught } = spanishTaughtForms(root, level);
  const dir = `spanish/mocks/${level.toLowerCase()}`;
  return [1, 2].map((mock) => {
    const paper = readFileSync(resolve(root, `${dir}/mock-${mock}-paper.md`), "utf8");
    const key = readFileSync(resolve(root, `${dir}/mock-${mock}-answer-key.md`), "utf8");
    const requiresByItem = new Map<number, ReadonlySet<string>>();
    for (const line of key.split(/\r?\n/)) {
      const row = /^\|\s*(\d+)\s*\|.*\|\s*([^|]+)\s*\|$/.exec(line);
      if (!row) continue;
      requiresByItem.set(
        Number(row[1]),
        new Set((row[2] ?? "").split(",").map((entry) => entry.trim().toLowerCase())),
      );
    }
    return {
      mock,
      forms: reportStemCoverage(parseMockPaper(paper), taught, requiresByItem, vocabulary),
    };
  });
}
