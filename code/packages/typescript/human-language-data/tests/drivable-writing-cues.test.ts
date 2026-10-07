// A drivable lesson must not tell the listener to write (issue #12070).
//
// ---------------------------------------------------------------------------
// The rule
// ---------------------------------------------------------------------------
//
// A lesson the modality manifest marks `drivable: true` is narrated with no
// "you will need a pen" notice. Any writing task inside it therefore has to be
// authored as a `[YOU WRITE: …]` cue, which the narration defers ("once you
// have stopped driving — write: …"). A bare "Write 是." is read out as an
// instruction to a driver. `drivable-writing-imperatives.ts` explains how the
// bare form is recognised and why the pattern is as narrow as it is.
//
// ---------------------------------------------------------------------------
// Why there is a debt ledger, and why it can only shrink
// ---------------------------------------------------------------------------
//
// The issue named nine Chinese lessons. Running the detector over all 23 tracks
// found the same shape in about four hundred drivable lessons — Marwadi and
// Japanese dictation steps ("4. Write all five from dictation."), Chinese
// "Copy **学** once." practice lines, Hindi and Urdu warm-ups. Each of those
// needs an author's eye (some want a cue, some want a spoken warm-up instead),
// so rewriting them all inside the fix for nine lessons would be the wrong
// change. `toBe(0)` on day one would be a check that gets skipped.
//
// So this follows the ceiling precedent in `banned-words.test.ts`, made exact:
// `drivable-writing-debt/<track>.json` lists, per track, the drivable lessons
// that still carry a bare writing imperative. The test demands that the
// detector's findings and the ledger AGREE in both directions:
//
//   a drivable lesson offends, not in the ledger   -> fail: new debt. Use a
//                                                     [YOU WRITE: …] cue.
//   a ledger entry no longer offends               -> fail: the debt fell.
//                                                     Delete the entry.
//
// The second direction is what makes it a ratchet rather than an allowlist: a
// lesson that gets fixed cannot quietly keep its pass, so the list can only
// get shorter. A track with no debt has no file at all, so the absence of a
// file is itself a pin.
//
// The ledger is one file per track, not one list, because content PRs are
// per track. Two PRs paying down Japanese and Marwadi debt at the same time
// should not collide in a shared file.

import { readdirSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import { loadEverything, loadModalityManifest } from "../src/loader.js";
import {
  bareWritingImperatives,
  narratedProseSpans,
  withoutHtmlComments,
} from "./drivable-writing-imperatives.js";

const DEBT_DIRECTORY = join(dirname(fileURLToPath(import.meta.url)), "drivable-writing-debt");

/**
 * The lessons issue #12070 fixed. The issue listed nine; the same warm-up shape
 * sat in two more lessons of the same chapter (ZH-C03-er, ZH-C03-si), fixed
 * in the same change. Pinned by name so a regression points at the issue.
 */
const ISSUE_12070_LESSONS = [
  "ZH-C03-yi",
  "ZH-C03-er",
  "ZH-C03-san",
  "ZH-C03-si",
  "ZH-C03-wu",
  "ZH-C04-ri",
  "ZH-C04-bu",
  "ZH-C04-practice",
  "ZH-C05-shi",
  "ZH-C05-practice",
  "ZH-C06-zaijian",
] as const;

// ---------------------------------------------------------------------------
// The detector, on fixtures
// ---------------------------------------------------------------------------
//
// Every positive here is a sentence from the corpus, and every control is a
// real sentence the pattern had to learn to leave alone. The controls matter
// more than the positives: they are the reason the corpus run below can demand
// exact agreement instead of a fuzzy ceiling.

describe("bareWritingImperatives: what fires", () => {
  it.each([
    ["a warm-up after a pause cue", "[PAUSE 2s] Draw the shape you just learned. One stroke, left to right."],
    ["a second sentence", "Say **see** as *kànjiàn*. Write 看, then point to 见 in 再见."],
    ["a numbered dictation step", "4. Write all five from dictation."],
    ["a bold label", "**Write.** 的 — 白 first, then 勺, and the dot last."],
    ["a bold label with a colon", "4. **Write:** hear the word, wait ten seconds, and write all five signs."],
    ["a then-clause", "Say *desh*, then write **देश** from memory."],
    ["a connective", "Say **this shirt**. Then write **ও**."],
    ["copying a character", "Copy **学** once. Pause mentally after the top."],
    ["circling a letter", "Circle **ఝ**. Check its three rounded bowls."],
    ["a sentence after a closing parenthesis", "(**ai**.) Write the three forms."],
    ["prose after a cue", "[YOU SAY: hǎo] Write 好 once."],
  ])("%s", (_label, markdown) => {
    expect(bareWritingImperatives(markdown)).toHaveLength(1);
  });

  it("reads a paragraph across its wrapped lines", () => {
    // Lessons wrap at about eighty columns. Line by line, "what number you
    // expect it to be." would be its own span and the imperative on the line
    // above would be judged without its sentence.
    const markdown = "[PAUSE 2s] Draw the shape. Two bars, short over long. Count the strokes and say\nwhat number you expect it to be.";
    expect(narratedProseSpans(markdown)).toEqual([
      "Draw the shape. Two bars, short over long. Count the strokes and say what number you expect it to be.",
    ]);
    expect(bareWritingImperatives(markdown)).toHaveLength(1);
  });
});

describe("withoutHtmlComments", () => {
  // The narrator never reads a comment, so neither does the detector. An
  // unterminated opener hides the rest of the text, as it does in speech.ts.
  it.each([
    ["a closed comment", "a <!-- x --> b", "a  b"],
    ["two comments", "<!-- x -->a<!-- y -->b", "ab"],
    ["an unterminated opener", "a <!-- Draw it.", "a "],
    ["a nested opener", "a <!--<!-- x --> Draw", "a  Draw"],
    ["no comment at all", "Draw the shape.", "Draw the shape."],
  ])("%s", (_label, input, expected) => {
    expect(withoutHtmlComments(input)).toBe(expected);
  });

  it("empties a long run of unclosed openers", () => {
    // The lazy regex this replaces was quadratic here (800 KB took seconds).
    // No wall-clock bound: timing asserts flake on a loaded runner.
    expect(withoutHtmlComments("<!--".repeat(200_000))).toBe("");
  });
});

describe("bareWritingImperatives: what does not fire", () => {
  it.each([
    ["the cue form itself", "[PAUSE 2s] [YOU WRITE: 再, then 见 — six strokes, then four]"],
    ["a cue in a list", "- [YOU WRITE: 日 — and say both of its meanings]"],
    ["a cue whose content opens with a verb", "[YOU WRITE: copy **学** once from the model]"],
    ["the verb as a subject", "English refused it. **Write** is Old English *wrītan*."],
    ["an italic gloss", "**Hör!** — *Hear!* · **Schreib!** — *Write!*"],
    ["a vocabulary line", "Think: **विचार करणे**. Understand: **समजणे**. Write: **लिहिणे**."],
    ["a gloss after a dash", "*Apunta el número* — write the number down."],
    ["copying a sound", "Both are made further back than English *n*; copy the sound rather than the spelling."],
    ["a list of glosses", "*Contratar*, *firmar*, *fijar* — draw, harden, fasten."],
    ["a ring of cognates", "(A ring: circle, circuit, circa.)"],
    ["a recall question with its answer", "Loudly? (**ಜೋರಾಗಿ**.) Write it down? (**ಬರೆದು ಕೊಡಿ**.)"],
    ["etymological tracing", "Trace *nox* back far enough and you land on Proto-Indo-European."],
    ["a description mid-sentence", "Two characters, one word, and you can already\nwrite both of them."],
    ["a subject plus and-clause", "The older books attach it to a root, and draw in a wide family of words."],
    ["a written-form statement", "Say the last line twice. It **rises**. Written\ndown, it falls."],
    ["an HTML comment", "<!-- Write this later -->\nSay **hǎo**."],
  ])("%s", (_label, markdown) => {
    expect(bareWritingImperatives(markdown)).toEqual([]);
  });
});

// ---------------------------------------------------------------------------
// The corpus
// ---------------------------------------------------------------------------

const { lessons, registry } = loadEverything();
const manifest = loadModalityManifest();
const drivableIds = new Set(manifest.lessons.filter((row) => row.drivable).map((row) => row.id));
const lessonById = new Map(lessons.map((lesson) => [String(lesson.frontmatter.id), lesson]));

/** Every drivable lesson's bare writing imperatives, grouped by track. Clean lessons are absent. */
function findOffenders(): Map<string, Map<string, string[]>> {
  const byTrack = new Map<string, Map<string, string[]>>();
  for (const lesson of lessons) {
    const id = String(lesson.frontmatter.id);
    if (!drivableIds.has(id)) continue;
    // A drivable lesson has no detachable block to set aside: the whole lesson
    // is read, so the preamble and every block are scanned.
    const markdown = [lesson.preamble, ...lesson.blocks.map((block) => block.markdown ?? "")].join("\n\n");
    const hits = bareWritingImperatives(markdown);
    if (hits.length === 0) continue;
    const track = byTrack.get(lesson.language) ?? new Map<string, string[]>();
    track.set(id, hits);
    byTrack.set(lesson.language, track);
  }
  return byTrack;
}

/** The committed ledger: track -> the lesson ids recorded as debt. */
function loadDebt(): Map<string, string[]> {
  const debt = new Map<string, string[]>();
  for (const entry of readdirSync(DEBT_DIRECTORY, { withFileTypes: true })) {
    const track = entry.name.replace(/\.json$/, "");
    debt.set(track, JSON.parse(readFileSync(join(DEBT_DIRECTORY, entry.name), "utf8")) as string[]);
  }
  return debt;
}

const offenders = findOffenders();
const debt = loadDebt();

describe("drivable lessons carry no bare writing imperative", () => {
  it("scans a real corpus, not an empty one", () => {
    // Anti-vacuity. If the manifest stopped loading, or ids stopped matching,
    // every lesson would be skipped and the agreement below would pass with
    // two empty sides.
    const scanned = lessons.filter((lesson) => drivableIds.has(String(lesson.frontmatter.id)));
    expect(scanned.length).toBeGreaterThan(1000);
    expect(offenders.size).toBeGreaterThan(0);
  });

  it("the #12070 lessons are still drivable, and clean", () => {
    // Both halves matter. Clean but no longer drivable would mean the fix
    // "worked" by losing the lesson its place in the driving edition, which is
    // the one thing the cue form exists to avoid.
    for (const id of ISSUE_12070_LESSONS) {
      expect(lessonById.has(id), `${id} exists`).toBe(true);
      expect(drivableIds.has(id), `${id} is drivable`).toBe(true);
      expect(offenders.get("chinese")?.get(id), `${id} has no bare writing imperative`).toBeUndefined();
    }
  });

  it("the debt ledger is well formed", () => {
    const languages = new Set(registry.languages.map((language) => language.id));
    for (const entry of readdirSync(DEBT_DIRECTORY, { withFileTypes: true })) {
      expect(entry.isFile(), `${entry.name} is a plain file`).toBe(true);
      expect(entry.name, "ledger files are <track>.json").toMatch(/^[a-z]+\.json$/);
      expect(languages.has(entry.name.replace(/\.json$/, "")), `${entry.name} names a registered track`).toBe(true);
    }
    for (const [track, ids] of debt) {
      expect(Array.isArray(ids), `${track}.json holds an array`).toBe(true);
      // An empty file would be a second way of saying "no debt"; deleting the
      // file is the one way.
      expect(ids.length, `${track}.json is empty: delete the file instead`).toBeGreaterThan(0);
      expect(ids, `${track}.json is sorted and has no duplicates`).toEqual([...new Set(ids)].sort());
      for (const id of ids) {
        expect(lessonById.get(id)?.language, `${track}.json: ${id} is a ${track} lesson`).toBe(track);
      }
    }
  });

  it("every offending drivable lesson is recorded, and every record still offends", () => {
    const tracks = [...new Set([...offenders.keys(), ...debt.keys()])].sort();
    const problems: string[] = [];
    for (const track of tracks) {
      const found = offenders.get(track) ?? new Map<string, string[]>();
      const recorded = new Set(debt.get(track) ?? []);
      for (const [id, spans] of found) {
        if (recorded.has(id)) continue;
        problems.push(
          `NEW  ${id}: a drivable lesson tells the listener to write in bare prose. ` +
            `Author it as a [YOU WRITE: …] cue (or make the task spoken):\n` +
            spans.map((span) => `       ${span.slice(0, 160)}`).join("\n"),
        );
      }
      for (const id of recorded) {
        if (found.has(id)) continue;
        problems.push(
          `FIXED ${id}: no longer offends (or is no longer drivable). ` +
            `Remove it from tests/drivable-writing-debt/${track}.json — the ledger only shrinks.`,
        );
      }
    }
    expect(problems, problems.join("\n")).toEqual([]);
  });
});
