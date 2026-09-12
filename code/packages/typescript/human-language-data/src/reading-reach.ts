// # Reading reach --- does the curriculum ever produce a text as long as the exam's?
//
// Every `task-shapes/<level>.json` states, part by part, how much text a
// candidate must read: DELE A1's `reading-personal-message` hands over 150--175
// words, and its `reading-everyday-information` 175--210. Those numbers were
// transcribed carefully and then never compared to anything. `task-shapes.ts`
// validates that a `stimulusLength` parses; nothing asks whether the track
// contains a passage that long.
//
// That is a gap you can pass straight through. A track can report full coverage
// of its A1 content inventory --- every grammar point, every notion, every
// function --- while the longest connected text it has ever shown a learner is
// a third of what the paper puts in front of them. Content coverage and exam
// readiness are different claims, and only one of them was measured.
//
// ## What counts as a passage
//
// A `comprehension` block's blockquote. That is the block type a reading lesson
// uses for its passage, and the blockquote is the passage itself rather than
// the instructions around it.
//
// A word is a whitespace-separated run containing a letter OR a digit. The
// digit half matters: a stimulus that says "el autobús 42 sale a las 9" is
// four content words plus two numerals to a candidate, and the boards count
// numerals when they publish a word count -- so dropping them would measure
// our passages by a stricter rule than the target they are compared against.
// A run with neither, such as an em-dash used as a bullet, is not a word.
//
// ## Three outcomes, and every inventory lands in exactly one
//
// * `measurable` --- the reading section publishes at least one stimulus
//   length, so the comparison is meaningful.
// * `length-not-published` --- there IS a reading section, but the board does
//   not publish word counts and the shape says so in `notPublished`. DELF,
//   Goethe and ALPT all do this. Such an inventory is excluded from the
//   comparison WITH ITS REASON, never dropped: a silent drop is how a
//   measurement quietly stops covering four of its twenty-seven cases.
// * `no-reading-section` --- the inventory declares no reading skill at all.
//
// The three are exhaustive by construction, and `measureReadingReach` returns
// one row per inventory the registry enumerates, so a track that gains a task
// shape tomorrow appears here tomorrow without anyone remembering to add it.

import type { CefrLevel } from "./levels.js";
import type { TaskShapeInventory } from "./task-shapes.js";
import type { ParsedLesson } from "./parse.js";

/** Why an inventory is or is not comparable against the corpus. */
export type ReadingReachStatus = "measurable" | "length-not-published" | "no-reading-section";

/** One reading part, and whether the track's longest passage reaches it. */
export interface ReadingPartReach {
  partId: string;
  /** Fewest words the board says the stimulus runs to, or `null` if unpublished. */
  minimum: number | null;
  /** True when the track's longest passage is at least `minimum`. */
  withinReach: boolean;
}

/** One task-shape inventory, measured against the track's longest passage. */
export interface TrackReadingReach {
  language: string;
  level: CefrLevel;
  status: ReadingReachStatus;
  /** Longest `comprehension` passage anywhere in the track, in words. */
  longestPassageWords: number;
  parts: ReadingPartReach[];
  /** Parts whose published minimum the track's longest passage reaches. */
  partsWithinReach: number;
  /** Parts publishing a minimum at all. Zero unless `status` is `measurable`. */
  partsMeasurable: number;
  /** For `length-not-published`, the board's own wording. Empty otherwise. */
  unpublishedReasons: string[];
}

export interface ReadingReachReport {
  rows: TrackReadingReach[];
  /** Longest passage per track, including tracks with no task shape at all. */
  longestByTrack: Record<string, number>;
}

/**
 * Words in a comprehension block's blockquote.
 *
 * Only the quoted lines count. A reading lesson's prose explains the passage
 * and would otherwise inflate it by exactly the amount of scaffolding the
 * author wrote, which would make a heavily-explained short passage outrank a
 * bare long one.
 */
/**
 * Scripts that write no spaces, so whitespace cannot delimit a unit.
 *
 * Han, hiragana, katakana and the CJK extensions. A passage in any of these is
 * counted by CHARACTER; everything else is counted by whitespace-separated word.
 */
const SPACELESS_SCRIPT = /[\u3040-\u30ff\u3400-\u4dbf\u4e00-\u9fff\uf900-\ufaff]/u;

/**
 * How much text a passage is, in the unit its own script makes available.
 *
 * The first version of this counted whitespace-separated runs everywhere, and
 * for Japanese that is not a unit at all: もうすこし、ゆっくりいってください is
 * fourteen signs and four words, and whitespace sees ONE token. The comma in it
 * is U+3001, which is not whitespace either. So a six-line Japanese passage
 * measured 6 and a reader could not tell that from six single words.
 *
 * A spaceless script is therefore counted by character, which is also the unit
 * Chinese's own task shape asks for -- `prea1-reading-pinyin-signs-and-doorway-words`
 * declares `unit: "items"`, and an item there is a character. Latin, Cyrillic,
 * Devanagari and the Arabic-script tracks keep the word count they always had,
 * because their shapes declare `unit: "words"` and they write the spaces to
 * support it.
 *
 * Mixed passages count both: a Han character is one, and a run of Latin letters
 * between spaces is one. That is the honest reading of a line like `コーヒー 100`.
 */
export function passageLength(markdown: string): number {
  const quoted = markdown
    .split("\n")
    .map((line) => line.trim())
    .filter((line) => line.startsWith(">"))
    .map((line) => line.replace(/^>\s?/, ""));
  const text = quoted.join(" ");
  if (!SPACELESS_SCRIPT.test(text)) {
    return text.split(/\s+/).filter((word) => /[\p{L}\p{N}]/u.test(word)).length;
  }
  let count = 0;
  let run = false;
  for (const char of text) {
    if (SPACELESS_SCRIPT.test(char)) {
      count += 1;
      run = false;
      continue;
    }
    if (/[\p{L}\p{N}]/u.test(char)) {
      if (!run) count += 1;
      run = true;
      continue;
    }
    run = false;
  }
  return count;
}

/**
 * Kept as the old name so a caller that genuinely wants words can say so.
 * Identical to `passageLength` for every space-writing script.
 */
export const passageWordCount = passageLength;

/** Longest comprehension passage in each track, keyed by language. */
export function longestPassageByTrack(lessons: readonly ParsedLesson[]): Record<string, number> {
  const longest: Record<string, number> = {};
  for (const lesson of lessons) {
    for (const block of lesson.blocks ?? []) {
      if (block.type !== "comprehension") continue;
      const words = passageLength(block.markdown ?? "");
      if (words > (longest[lesson.language] ?? 0)) longest[lesson.language] = words;
    }
  }
  return longest;
}

/**
 * Compare every task-shape inventory's reading section against the corpus.
 *
 * `inventories` is what the registry enumerates, not a hand-kept list, so the
 * report is co-total with the loader by construction.
 */
export function measureReadingReach(
  lessons: readonly ParsedLesson[],
  inventories: readonly TaskShapeInventory[],
): ReadingReachReport {
  const longestByTrack = longestPassageByTrack(lessons);
  const rows: TrackReadingReach[] = [];

  for (const inventory of inventories) {
    const longestPassageWords = longestByTrack[inventory.language] ?? 0;
    const section = inventory.sections.find((candidate) => candidate.skill === "reading");

    if (!section) {
      rows.push({
        language: inventory.language,
        level: inventory.level,
        status: "no-reading-section",
        longestPassageWords,
        parts: [],
        partsWithinReach: 0,
        partsMeasurable: 0,
        unpublishedReasons: [],
      });
      continue;
    }

    const parts: ReadingPartReach[] = section.parts.map((part) => {
      const minimum = part.stimulusLength?.minimum ?? null;
      return {
        partId: part.id,
        minimum,
        withinReach: minimum !== null && longestPassageWords >= minimum,
      };
    });
    const partsMeasurable = parts.filter((part) => part.minimum !== null).length;

    rows.push({
      language: inventory.language,
      level: inventory.level,
      status: partsMeasurable > 0 ? "measurable" : "length-not-published",
      longestPassageWords,
      parts,
      partsWithinReach: parts.filter((part) => part.withinReach).length,
      partsMeasurable,
      unpublishedReasons:
        partsMeasurable > 0
          ? []
          : [...new Set(section.parts.flatMap((part) => part.notPublished))].sort(),
    });
  }

  rows.sort(
    (left, right) =>
      left.language.localeCompare(right.language) || left.level.localeCompare(right.level),
  );
  return { rows, longestByTrack };
}

/** A human-readable table --- the thing somebody actually reads before authoring. */
export function formatReadingReach(report: ReadingReachReport): string {
  const lines: string[] = [];
  const measurable = report.rows.filter((row) => row.status === "measurable");
  const unpublished = report.rows.filter((row) => row.status === "length-not-published");
  const none = report.rows.filter((row) => row.status === "no-reading-section");

  lines.push("# Reading reach — longest passage against the exam's own stimulus length");
  lines.push("");
  lines.push(
    `${report.rows.length} task-shape inventories: ${measurable.length} measurable, ` +
      `${unpublished.length} with unpublished stimulus lengths, ` +
      `${none.length} with no reading section.`,
  );
  lines.push("");
  lines.push("| track | level | longest passage | shortest part | longest part | parts in reach |");
  lines.push("|---|---|---:|---:|---:|---|");
  for (const row of measurable) {
    const minima = row.parts
      .map((part) => part.minimum)
      .filter((minimum): minimum is number => minimum !== null);
    lines.push(
      `| ${row.language} | ${row.level} | ${row.longestPassageWords} | ` +
        `${Math.min(...minima)} | ${Math.max(...minima)} | ` +
        `${row.partsWithinReach}/${row.partsMeasurable} |`,
    );
  }
  if (unpublished.length > 0) {
    lines.push("");
    lines.push("Excluded — the board does not publish a stimulus word count:");
    for (const row of unpublished) {
      lines.push(`- ${row.language}/${row.level}: ${row.parts.length} reading part(s)`);
    }
  }
  return lines.join("\n");
}
