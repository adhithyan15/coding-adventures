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
  clausesOf,
  narratedProseSpans,
  opensChainedOrFrontedWriting,
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

  // The two shapes a hand review of #16893/#16994 found the single regex
  // missing. Each is a sentence from a drivable lesson that the audio edition
  // read to a driver as written.
  it.each([
    ["say and write", "[PAUSE 8s] Say and write **the Chinese language** without a model."],
    ["a three-verb chain", "Hear all three in a mixed order. Say, read, and write each answer."],
    ["a chain with a wait in it", "Read **人**. Cover it, wait five seconds, and write it."],
    ["a chain after a bold step label", "4. **Writing:** hear all six and write them without a model."],
    ["a chain after a colon", "A date needs an ordinal: say *первый, второй, третий*, and write the first as a sign writes it, *1-й*."],
    ["a chain after a connective", "Say it; read **हां सा**; then cover and write it."],
    ["a comma-linked step", "Say *nānā*, write **बाप**, and say the known paternal-grandmother word."],
    ["a chain with a dashed aside", "Say *a doctor* — **R1**, one lesson back — and write the small sign in it."],
    ["a chain around a quoted cue word", "Then hear “younger sister,” say **いもうと**, read **ちち・はは**, and write all three."],
    ["a fronted phrase", "Without looking back, write **look**, **see**, and **good-looking** from mixed meaning cards."],
    ["a longer fronted phrase", "From sound and spoken names only, write *dhanyavād*, visarga, independent *ā*, and *bha*."],
    ["two fronted phrases", "Give the greeting. Then, with the new word covered, write **आ**, then **भ**."],
    ["a fronted phrase in a numbered step", "2. Beside each, write the number that needed it."],
    ["a chain after a fronted phrase", "With every model hidden, identify, say, read, and write **बस** in a shuffled order."],
    ["a chain into rewrite", "Repair only what was missed and rewrite that alone."],
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

  // Controls for the chained and fronted shapes. Each is a corpus sentence
  // that has a writing verb after "and", after a comma, or after a fronted
  // phrase, and asks nobody to write.
  it.each([
    ["the fixed form of a chain", "Read **人**. Cover it and wait five seconds. [YOU WRITE: the character]"],
    ["the fixed form of a fronted phrase", "[YOU WRITE: without looking back, **look**, **see**, and **good-looking**]"],
    ["a list of skills", "That closes the run: look, listen, speak, write."],
    ["a quotation being said", 'Say "I read and write Spanish". (*Leo y escribo español*.)'],
    ["a chain after a subject", "Water is what you have; wine is what you buy, ship, tax and write down."],
    ["a chain after an adverb", "Only then will you read and write the whole word."],
    ["a chain after a subordinate", "The writing track hasn't reached it, so read it for now and draw it later."],
    ["a notice quoted in a reading", "The notice tells you what to do: go to the desk, and write your name."],
    ["a subordinate clause, not a fronted phrase", "(It is not open yet; when it opens, write your name at reception; do not smoke.)"],
    ["a gloss pair", "*Bā*, come; *bare*, write. Both chapters used one more word in passing."],
    ["a list of phrases", "In Hindi, how do you say it, show me, I could not hear, write it down."],
    ["a description with a subject", "You can now name the telephone, write and send a letter, and say an address."],
    ["a chain that copies a sound", "Listen and copy the rhythm of the line."],
    ["a glossed chain", "Say the words for to weigh and to draw."],
    ["a chained recall question", "Loudly? (**ಜೋರಾಗಿ**.) Say it and write it down? (**ಬರೆದು ಕೊಡಿ**.)"],
  ])("%s", (_label, markdown) => {
    expect(bareWritingImperatives(markdown)).toEqual([]);
  });
});

describe("clausesOf", () => {
  // The chained and fronted tests judge one clause at a time, so where a
  // clause starts is what decides which verb "opens" it.
  it.each([
    ["sentences", "Say it. Write it.", ["Say it.", "Write it."]],
    ["a bold step label", "4. **Writing:** hear all six and write them.", ["4.", "**Writing:", "hear all six and write them."]],
    ["semicolons and a closing parenthesis", "(**ai**.) Say it; then cover and write it.", ["(**ai**.", "Say it;", "then cover and write it."]],
    ["a stop with no space after it", "1.5 metres", ["1.5 metres"]],
  ])("%s", (_label, span, expected) => {
    expect(clausesOf(span)).toEqual(expected);
  });

  it("stays linear on a long run of clauses and a long chain with no writing verb", () => {
    // No wall-clock bound (timing asserts flake on a loaded runner); the cases
    // are sized so a quadratic scan would stall the suite.
    expect(clausesOf("Say it. ".repeat(100_000))).toHaveLength(100_000);
    expect(opensChainedOrFrontedWriting(`Say ${"it, ".repeat(100_000)}and stop`)).toBe(false);
    expect(opensChainedOrFrontedWriting(`Without ${"looking ".repeat(100_000)}back, write it`)).toBe(false);
  });

  it("stays linear on the two inputs a security review found quadratic", () => {
    // Before the fix each took over a second at this size: an unanchored
    // `\s+and\s+` restarting at every space of a long run, and `“[^”]*”`
    // rescanning from every unclosed opener. Only the answers are asserted.
    const spaces = " ".repeat(40_000);
    expect(opensChainedOrFrontedWriting(`say${spaces}x`)).toBe(false);
    expect(opensChainedOrFrontedWriting(`Say it${spaces}and${spaces}write it`)).toBe(true);
    const openers = "“".repeat(40_000);
    expect(opensChainedOrFrontedWriting(`Say ${openers}`)).toBe(false);
    expect(opensChainedOrFrontedWriting(`Say ${openers} and write it`)).toBe(true);
    expect(opensChainedOrFrontedWriting(`Say “${"and write ".repeat(4_000)}” now`)).toBe(false);
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
