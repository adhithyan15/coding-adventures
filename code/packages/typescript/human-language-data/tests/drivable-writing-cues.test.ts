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
//
// When the last track's debt is paid the directory itself disappears: git does
// not keep an empty directory, so deleting the last `<track>.json` deletes
// `drivable-writing-debt/` too. A missing directory therefore means "no track
// has debt", read exactly like a missing file, and not a crash in `readdirSync`
// that would take the whole file's tests down with it. Reading absence as
// empty is safe HERE, unlike for a section directory a generator needs (see
// the lesson on "drive this to zero" programmes): the ledger is exact, so a
// directory lost by mistake while debt remains is not silent. Every lesson it
// listed is reported as NEW debt. The same is true of
// the anti-vacuity check: it can no longer ask for "some drivable lesson
// offends", because the goal is that none does. It asks instead that the
// detector still fires on the real corpus — on the lessons that are NOT
// drivable, where writing in prose is legitimate pen work.
//
// ---------------------------------------------------------------------------
// What counts as telling the listener to write
// ---------------------------------------------------------------------------
//
// Two things, both from `drivable-writing-imperatives.ts`: a bare writing
// imperative in prose, and a `[YOU RECALL: …]` cue whose content asks for
// writing ("[YOU RECALL: write **ば** — **R1**]"). The second is a cue, but
// RECALL is a spoken action, so the narration reads it out with no deferral.
//
// Three more checks at the end of this file read cues for the page or the body
// rather than the pen: pointing at a sign (inside a recall), reading printed
// script, and making a gesture — clapping, pointing, touching, showing fingers
// (both inside any cue the narration speaks unhedged — RECALL, RETURN TO, SAY,
// RUN, …). Each demands exactly zero in drivable lessons, with no ledger,
// because every such cue was fixed in the change that added (or widened) the
// check.

import { type Dirent, existsSync, readdirSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import { loadEverything, loadModalityManifest } from "../src/loader.js";
import {
  bareWritingImperatives,
  clausesOf,
  drivableWritingInstructions,
  gestureSpokenCues,
  narratedProseSpans,
  opensChainedOrFrontedWriting,
  pointingRecallCues,
  readingSpokenCues,
  recallCueAsksForWriting,
  recallCueAsksToPoint,
  spokenCueAsksForGesture,
  spokenCueAsksToReadScript,
  withoutHtmlComments,
  writingRecallCues,
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
// Inside a recall cue
// ---------------------------------------------------------------------------
//
// RECALL is a spoken cue action, so the narration reads "[YOU RECALL: write
// **ば** — **R1**]" to a driver as "recall: write ば". The positives are the
// four shapes the corpus had; the controls are corpus recall cues that mention
// writing and ask for none, and the cues the fix turned them into.

describe("writingRecallCues: what fires", () => {
  it.each([
    ["a spaced recall that opens with write", "- [YOU RECALL: write **ば** — **R1**, one lesson back]"],
    ["a qualified sign", "- [YOU RECALL: write small **ゃ** — **R4**, eighty lessons back]"],
    ["a recall that opens with draw", "- [YOU RECALL: draw the **।** and say what it marks — **R2**, five lessons back]"],
    ["write, then a spoken check", "- [YOU RECALL: write **ঞ**, and say where it is made — **R1**, one lesson back]"],
    ["a writing verb chained onto a spoken one", "- [YOU RECALL: ask *kitthe?* and write it — **R2**, five lessons back]"],
    [
      "a chain after a question mark inside the cue",
      "- [YOU RECALL: answer *kuṭhe?* with *ithe*, then *tithe*, and write **तिथे** once]",
    ],
    ["a fronted phrase inside the cue", "[YOU RECALL: the last sign; with the page covered, write it]"],
    ["a cue wrapped across two source lines", "- [YOU RECALL: write **पंदरा**,\n  five lessons back — **R2**]"],
    ["a cue after prose", "[PAUSE 2s] Four recalls, at four distances. [YOU RECALL: write **け** — **R4**]"],
  ])("%s", (_label, markdown) => {
    expect(writingRecallCues(markdown)).toHaveLength(1);
    expect(drivableWritingInstructions(markdown)).toHaveLength(1);
  });

  it("quotes the cue as authored, one entry per cue", () => {
    const markdown = "[PAUSE 22s]\n- [YOU RECALL: write **पंदरा** — **R1**]\n- [YOU RECALL: write **इ** — **R2**]\n- [YOU RECALL: say *do*]";
    expect(writingRecallCues(markdown)).toEqual(["[YOU RECALL: write **पंदरा** — **R1**]", "[YOU RECALL: write **इ** — **R2**]"]);
  });
});

describe("writingRecallCues: what does not fire", () => {
  it.each([
    ["the fixed form", "- [YOU WRITE: **ば** from memory — **R1**, one lesson back]"],
    ["the fixed form of a chain", "- [YOU RECALL: answer *kuṭhe?* with *ithe*, then *tithe*]\n- [YOU WRITE: **तिथे** once, from memory]"],
    ["a recall with no writing in it", "- [YOU RECALL: say *chauthā* — **R1**, one lesson back]"],
    ["the word for write, as a gloss", "[YOU RECALL: say the Japanese for to write, then the Japanese for to speak, then say *matsu* again]"],
    ["a gloss after a semicolon", "[YOU RECALL: say the Persian for to sew, then the Persian for to pull; to draw, then say *rikhtan* again]"],
    ["what the learner can already write", "[YOU RECALL: point to the sign in **これ** you can already write, and the one you cannot]"],
    ["what the learner cannot write yet", "[YOU RECALL: point to the sign in **くるま** you cannot write yet, and say which sign it looks like]"],
    ["a memory of having written", "[YOU RECALL: the first letters you wrote — **р с н б д е т** — and the one that looks like an English R]"],
    ["a memory of learning to write", "[YOU RECALL: *ek* and the letter **ए** you had to learn to write it]"],
    ["a past participle", "[YOU RECALL: say how much space a written answer needs on a form — **R4**, eighty lessons back]"],
    ["a quotation being recalled", '[YOU RECALL: say "I read and write Spanish" once more]'],
    // SAY is not read inside: its content is the material spoken, and in a
    // target language that material may well be about writing.
    ["a say cue about writing", "[YOU SAY: write it down, please]"],
    ["a bare word in prose", "Recall how you wrote it."],
  ])("%s", (_label, markdown) => {
    expect(writingRecallCues(markdown)).toEqual([]);
  });
});

describe("recallCueAsksForWriting stays linear", () => {
  // Each input is about 50,000 characters, sized so that a scan that restarts
  // at every candidate and rescans to the end would stall the suite. Only the
  // answers are asserted: timing bounds flake on a loaded runner.
  it.each([
    ["a long run of spaces before the verb", `ask it${" ".repeat(50_000)}and write it`, true],
    ["a long run of spaces with no verb", `ask it${" ".repeat(50_000)}x`, false],
    ["many unclosed quotation openers", `say ${"“".repeat(50_000)} and write it`, true],
    ["a quotation that hides every link", `say “${"and write ".repeat(5_000)}” now`, false],
    ["a long chain of links with no verb", `say ${"it, and ".repeat(6_000)}stop`, false],
    ["many clauses, the last one writing", `${"say it. ".repeat(6_000)}write it`, true],
    ["many question marks", `${"which? ".repeat(7_000)}none`, false],
    ["many stars", `${"*".repeat(50_000)} write`, false],
  ])("%s", (_label, content, expected) => {
    expect(content.length).toBeGreaterThan(40_000);
    expect(recallCueAsksForWriting(content)).toBe(expected);
  });

  it("walks a 50k-character paragraph of recall cues", () => {
    const markdown = "[YOU RECALL: write **ば**] ".repeat(2_000);
    expect(markdown.length).toBeGreaterThan(40_000);
    expect(writingRecallCues(markdown)).toHaveLength(2_000);
    // An unclosed cue swallows nothing: past MAX_CUE_LENGTH it is prose.
    expect(writingRecallCues(`[YOU RECALL: write ${"x ".repeat(25_000)}`)).toEqual([]);
  });
});

// ---------------------------------------------------------------------------
// Pointing at the page, inside a recall cue
// ---------------------------------------------------------------------------
//
// The positives are the shapes the corpus had before they were rewritten for
// the ear; the controls are corpus recall cues that mention pointing and ask
// for none, and the rewrites themselves.

describe("pointingRecallCues: what fires", () => {
  it.each([
    ["a recall that opens with point to", "- [YOU RECALL: point to the sign in **これ** you can already write, and the one you cannot]"],
    ["a mark to find", "- [YOU RECALL: point to the sign in **でんわ** that carries the two-stroke mark, and name the sign under it]"],
    ["point chained on with and", "- [YOU RECALL: say *mulāqāt*, then read **प्रणाम** and point to its **ण**]"],
    ["point at, after a comma", "[YOU RECALL: say *ek*, point at **ए**]"],
    ["point after then", "[YOU RECALL: say *do* then point at **द**]"],
    ["point after a semicolon", "[YOU RECALL: say *tīn*; then point to **त**]"],
    ["a capital at the start", "[YOU RECALL: Point to the small **っ** — **R2**]"],
    ["a cue wrapped across two source lines", "- [YOU RECALL: point to the sign in **くるま**\n  you cannot write yet]"],
  ])("%s", (_label, markdown) => {
    expect(pointingRecallCues(markdown)).toHaveLength(1);
  });
});

describe("pointingRecallCues: what does not fire", () => {
  it.each([
    ["the rewrite of a sign to name", "- [YOU RECALL: name the sign in **これ** you can already write, and the one you cannot]"],
    ["the rewrite of a mark to find", "- [YOU RECALL: say which sign in **でんわ** carries the two-stroke mark, and name the sign under it]"],
    ["the rewrite of a chained point", "- [YOU RECALL: say *mulāqāt*, then spell **प्रणाम** aloud letter by letter, naming its **ण** as you reach it]"],
    ["a word that points", "[YOU RECALL: two chapters back, the word that points at something near — *ei*]"],
    ["a pointing stem", "[YOU RECALL: *и*, the one-letter joiner, and its old pointing stem]"],
    ["point out, as a gloss", "[YOU RECALL: say the French for to imagine, then the French for to point out, then say *informer* again]"],
    ["point at, as a gloss", "[YOU RECALL: say the Persian for to point at, then say *nešân dâdan* again]"],
    ["a quotation being recalled", '[YOU RECALL: say "and point to it" once more]'],
    ["the POINT cue itself", "[YOU POINT: to **؟** at the end]"],
    ["a say cue", "[YOU SAY: then point to the door]"],
  ])("%s", (_label, markdown) => {
    expect(pointingRecallCues(markdown)).toEqual([]);
  });
});

describe("recallCueAsksToPoint stays linear", () => {
  // About 50,000 characters each; only the answers are asserted, because
  // timing bounds flake on a loaded runner.
  it.each([
    ["a long run of spaces before the verb", `say it${" ".repeat(50_000)}and point to it`, true],
    ["a long run of spaces with no verb", `say it${" ".repeat(50_000)}x`, false],
    ["many links with no verb", `say ${"it, and then ".repeat(4_000)}stop`, false],
    ["many links, the last one pointing", `say ${"it, and then ".repeat(4_000)}point at it`, true],
    ["many near misses", `${"and point out ".repeat(4_000)}`, false],
    ["many unclosed quotation openers", `say ${"“".repeat(50_000)} and point to it`, true],
    ["a quotation that hides every link", `say “${"and point to ".repeat(4_000)}” now`, false],
  ])("%s", (_label, content, expected) => {
    expect(content.length).toBeGreaterThan(40_000);
    expect(recallCueAsksToPoint(content)).toBe(expected);
  });
});

// ---------------------------------------------------------------------------
// Reading printed script, inside any spoken cue
// ---------------------------------------------------------------------------
//
// The positives are corpus cues as they were before the reading moved out
// into a `[YOU READ: …]` cue, one per shape the fixes met — recalls, the
// Tamil RETURN TO reviews, the Gujarati SAY prompts, the Urdu RUN — plus the
// step links and objects a security review named as gaps; the controls are
// corpus cues that say "read" and ask nobody to look (SAY material above
// all), and the fixed forms.

describe("readingSpokenCues: what fires", () => {
  it.each([
    ["a recall that is only a reading", "- [YOU RECALL: read **दाँत**]"],
    ["a reading after a spoken step", "- [YOU RECALL: say *dūdh*, then read **आँख**]"],
    ["a reading before a spoken step", "- [YOU RECALL: read **बच्चा**, then say *yahā̃*]"],
    ["a reading with its meaning to say", "- [YOU RECALL: say *ghās*, then read **कुआँ** and say what it means]"],
    ["two readings around a spoken step", "- [YOU RECALL: read **ತಿನ್ನು**, then say *nōḍu*, then read **ಗೊತ್ತು**]"],
    ["a spaced reading", "- [YOU RECALL: read **ऋ** — **R1**, one lesson back]"],
    ["a reading off the page", "- [YOU RECALL: read **さようなら** off the page — **R4**, eighty lessons back]"],
    ["a sign named by a noun", "- [YOU RECALL: read the sign **ೇ**, and say what it does to a letter — **R3**]"],
    ["a two-word noun phrase", "- [YOU RECALL: read the form label **आवडती कृती** and say why it ends in **-ती**]"],
    ["something printed", "- [YOU RECALL: read a printed ticket and say the figure on it aloud — **R3**]"],
    ["a reading after a semicolon", "[YOU RECALL: say *tīn*; read **त**]"],
    ["a reading after and", "[YOU RECALL: say *do* and then read **द**]"],
    ["reading aloud", "[YOU RECALL: say *ek*, then read aloud **एक**]"],
    ["a capital at the start", "[YOU RECALL: Read **ऋ** — **R1**]"],
    [
      "a cue wrapped across two source lines",
      "- [YOU RECALL: read **மற்றது**, then ask *vilai evvaḷavu?*, then say both forms of the\n  quotation line]",
    ],
    // Spoken verbs other than RECALL: the narration reads them all unhedged.
    [
      "a review that opens with a reading",
      "- [YOU RETURN TO: read **இன்று**, say *tuṭaippam* and say *kūrai* — three distances back — then join two of them with -உம்]",
    ],
    [
      "a review with a reading in the middle",
      "- [YOU RETURN TO: say *aṟuvaṭai*, read **பாலும்** and say *ēṉeṉṟāl* — three distances back — then say when one of them happens]",
    ],
    [
      "a review that ends its items with a reading",
      "- [YOU RETURN TO: say *mūḍu*, say *āṉāl* and read **எப்போது** — three distances back — then set two of them against each other, one and the other]",
    ],
    ["a say prompt with a reading after it", "- [YOU SAY: **છ** on its own, then read **છે** — same shape, different job]"],
    ["a say prompt with a reading after a dash", "- [YOU SAY: *chha, sāt* — then read **સાત** sign by sign]"],
    [
      "a run that ends off the script",
      "- [YOU RUN: both voices once from the romanization, then read the closing\n  line off the script alone]",
    ],
    ["a say drill reading digits", "- [YOU SAY: **এক**, then read **১**, **১০**, **১৩**]"],
    ["a say step reading a stem", "- [YOU SAY: strip **-ना** from **बोलना**, then read the stem **बोल** (*bol-*)]"],
    ["an answer with a reading after it", "[YOU ANSWER: *hā̃*, then read **हाँ**]"],
    // Step links and objects the first pass missed.
    ["a reading after an em dash", "[YOU SAY: *ek* — read **एक**]"],
    ["a reading after a colon", "[YOU SAY: *ek*: read **एक**]"],
    ["a reading after now", "[YOU RECALL: say *ek*, now read **एक**]"],
    ["a colon after the verb", "[YOU RECALL: say *ek*, then read: **एक**]"],
    ["read it, then a colon", "[YOU RECALL: say *ek*, then read it: **एक**]"],
    ["read it aloud, then a colon", "[YOU RECALL: say *ek*, then read it aloud: **एक**]"],
    ["a three-word noun phrase", "[YOU RECALL: read the very long sign **ೇ**]"],
    ["a four-word noun phrase", "[YOU RECALL: read the very long shop sign **ೇ**]"],
    ["a noun phrase closed by a colon", "[YOU RECALL: read the sign: **ೇ**]"],
  ])("%s", (_label, markdown) => {
    expect(readingSpokenCues(markdown)).toHaveLength(1);
  });
});

describe("readingSpokenCues: what does not fire", () => {
  it.each([
    ["the fixed form of a reading after a spoken step", "- [YOU RECALL: say *dīyā*]\n- [YOU READ: **कान**]"],
    ["the fixed form of a spaced reading", "- [YOU READ: **ऋ** — **R1**, one lesson back]"],
    ["the fixed form with its meaning to say", "- [YOU RECALL: say *ghās*]\n- [YOU READ: **कुआँ**, then say what it means]"],
    [
      "the fixed form of a review that opened with a reading",
      "- [YOU READ: **இன்று**]\n- [YOU RETURN TO: say *tuṭaippam* and say *kūrai* — three distances back — then join two of them with -உம்]",
    ],
    [
      "the fixed form of a review with a reading in the middle",
      "- [YOU RETURN TO: say *aṟuvaṭai*]\n- [YOU READ: **பாலும்**]\n" +
        "- [YOU RETURN TO: say *ēṉeṉṟāl* — three distances back — then say when one of them happens]",
    ],
    [
      "the fixed form of a review that ended its items with a reading",
      "- [YOU RETURN TO: say *mūḍu*, then say *āṉāl* — three distances back]\n- [YOU READ: **எப்போது**]\n" +
        "- [YOU RETURN TO: set two of them against each other, one and the other]",
    ],
    ["the fixed form of a say prompt", "- [YOU SAY: *chha, sāt*]\n- [YOU READ: **સાત** sign by sign]"],
    [
      "the fixed form of the run",
      "- [YOU RUN: both voices once from the romanization]\n- [YOU READ: the closing line off the script alone]",
    ],
    ["the word for read, as a gloss", "[YOU RECALL: say the Japanese for to read, then the Japanese for to write, then say *hanasu* again]"],
    ["a gloss later in the chain", "[YOU RECALL: say the Marwadi for to stay, then the Marwadi for to read, then say *jāṇṇo* again]"],
    ["material being recalled", "[YOU RECALL: say the line of your message that means *I read Marathi*]"],
    ["a meaning, recalled by ear", "[YOU RECALL: read *reception* on a sign — **R1**, one lesson back]"],
    ["two meanings, recalled by ear", "[YOU RECALL: read *open*, then *not yet open*, and say which one lets you in]"],
    ["reading out what was heard", "[YOU RECALL: read out the number you heard, then say it again]"],
    ["a phrase that stops before the script", "[YOU RECALL: read the whole line, then say **हाँ**]"],
    ["what the learner has read", "[YOU RECALL: say the first word you read in **देवनागरी**]"],
    ["a verb that ends in read", "[YOU RECALL: say *sūī*, then thread **धागा** through it]"],
    ["a quotation being recalled", '[YOU RECALL: say "then read **किताब**" once more]'],
    ["the READ cue itself", "[YOU READ: **किताब**, then say it without looking]"],
    ["another manual cue", "[YOU LOOK: at **क**, then read **ख**]"],
    // SAY and ANSWER material: what is spoken, not what is asked.
    ["a gloss after a dash", "[YOU SAY: **khândan** — to read]"],
    ["a quotation glossed after a dash", "[YOU SAY: \"legō\" — I read, hard g]"],
    ["a sentence glossed after a dash", "[YOU SAY: \"nēnu telugu caduvutānu\" — I read Telugu]"],
    ["a pronunciation after read it", "[YOU SAY: read it — AH-weh]"],
    ["read between two dashes", "[YOU SAY: \"pôṛā\" — read — then \"āmi poṛi,\" flapping the ড়]"],
    ["read it, glossing a sentence", "[YOU SAY: *maiṁ paṛhtā hūṁ*, then *maiṁ samajhtā hūṁ* — read it, then get it]"],
    ["a direction to read in", "[YOU SAY: read right to left — \"ism\"]"],
    ["a gloss with a colon", "[YOU SAY: the Japanese for to read: *yomu*]"],
    ["an answer that says I read", "[YOU ANSWER: *hā̃*, I read **हिंदी**]"],
    ["read it, then a word to say", "[YOU SAY: read it, then say **हाँ**]"],
    ["five plain words: past the bound", "[YOU RECALL: read the very long and winding sign **ೇ**]"],
  ])("%s", (_label, markdown) => {
    expect(readingSpokenCues(markdown)).toEqual([]);
  });
});

describe("spokenCueAsksToReadScript stays linear", () => {
  // About 50,000 characters each; only the answers are asserted, because
  // timing bounds flake on a loaded runner.
  it.each([
    ["a long run of spaces before the verb", `say it${" ".repeat(50_000)}then read **क**`, true],
    ["a long run of spaces with no verb", `say it${" ".repeat(50_000)}x`, false],
    ["a long run of spaces after the verb", `read${" ".repeat(50_000)}**क**`, true],
    ["many links with no verb", `say ${"it, and then ".repeat(4_000)}stop`, false],
    ["many links, the last one reading", `say ${"it, and then ".repeat(4_000)}read **क**`, true],
    ["many readings of nothing printed", `${"then read the whole line ".repeat(2_000)}`, false],
    ["many readings of meanings", `${"then read *a word* ".repeat(2_500)}`, false],
    ["many articles before a far script", `read ${"the ".repeat(12_500)}**क**`, false],
    ["many unclosed quotation openers", `say ${"“".repeat(50_000)} and read **क**`, true],
    ["a quotation that hides every link", `say “${"and read **क** ".repeat(4_000)}” now`, false],
    ["many stars", `read ${"*".repeat(50_000)}`, true],
    ["many em-dash links with no verb", `say ${"it — ".repeat(10_000)}stop`, false],
    ["many em-dash links, the last one reading", `say ${"it — ".repeat(10_000)}read **क**`, true],
    ["many colon links reading nothing", `${"say it: read ".repeat(4_000)}`, false],
    ["many now links, the last one reading", `say ${"it, now ".repeat(6_000)}read **क**`, true],
    ["many colons after the verb", `read${":".repeat(50_000)} **क**`, false],
    ["many particles before the script", `read ${"it ".repeat(16_000)}**क**`, false],
    ["a noun phrase far longer than the bound", `read the ${"very ".repeat(10_000)}long sign **क**`, false],
    ["many four-word phrases reaching nothing", `${"then read the very long sign ".repeat(1_500)}`, false],
    ["many read it: before a meaning", `${"then read it: *a* ".repeat(2_500)}`, false],
    ["many dashes after the verb", `read ${"— ".repeat(25_000)}**क**`, false],
  ])("%s", (_label, content, expected) => {
    expect(content.length).toBeGreaterThan(40_000);
    expect(spokenCueAsksToReadScript(content)).toBe(expected);
  });
});

// ---------------------------------------------------------------------------
// A gesture, inside any spoken cue
// ---------------------------------------------------------------------------
//
// The positives are corpus cues as they were before they were rewritten for
// the ear, one per shape the fix met, plus the non-drivable cues that keep
// their gestures; the controls are corpus cues that mention a hand, a clap or
// a point and ask for none, and the rewrites themselves.

describe("gestureSpokenCues: what fires", () => {
  it.each([
    // A gesture verb where a step starts.
    ["a demonstrative drill", '- [YOU SAY: "இங்கே" three times, pointing at something different each time]'],
    ["a mora drill", "- [YOU SAY: *denwa*, clapping three beats]"],
    ["a clap with a silent beat", "- [YOU SAY: *chotto*, clapping three beats — the middle clap is silent]"],
    ["a tap after and", "- [YOU SAY: *mo | o* and tap twice]"],
    ["finger counting after while", "- [YOU SAY: *onnŭ, raṇṭŭ, mūnnŭ, nālŭ, añcŭ* while raising one more finger]"],
    ["a hand raised after a dash", "- [YOU SAY: *valadu kai*, then *iḍadu kai* — and raise each hand as you say it]"],
    ["raising each hand", "- [YOU SAY: **வலது கை**, then **இடது கை**, raising each hand]"],
    ["a body part touched", "- [YOU SAY: *kandhā*, and touch it]"],
    ["each part touched", "- [YOU SAY: *uṅglī*, then *bāl*, and touch each as you name it]"],
    ["a pen held up", "- [YOU SAY: *kore*, holding up a pen, then *koko*, pointing at the floor where you stand]"],
    ["pointing at the start", "- [YOU SAY: point to one person, ask **¿Quién?**, and answer with a name]"],
    ["pointing after then", "- [YOU SAY: all five in order, then point at your మెడ and say *nāku noppi*]"],
    ["pointing after a semicolon and and", "- [YOU SAY: **voh** — that; and point at something across the room]"],
    ["pointing as you say it", "[YOU SAY: \"ondu\" as you point at ೧]"],
    ["a gesture named as a label", "- [YOU SAY: gesturing, then naming — *vahāṅ*, then *us kamre meṅ*]"],
    ["pointing named as a label", "- [YOU SAY: pointing at them — *ei bhāirā*]"],
    ["clapping inside a recall", "[YOU RECALL: say *a car*, and clap its beats — **R1**, one lesson back]"],
    // A hand or a bow as the manner of saying something.
    ["offered with both hands", "- [YOU SAY: *tohfā*, offered with both hands]"],
    ["placed with a hand", "- [YOU SAY: *kaḻuttŭ*, then *mutukŭ*, and place each one with a hand]"],
    ["a hand-wobble", '- [YOU SAY: "così così" — *koh-ZEE koh-ZEE*, with a hand-wobble]'],
    ["a small bow", '- [YOU SAY: "vaṇakkam" with a small bow]'],
    ["palms together", "- [YOU SAY: hello / goodbye, palms together — *nômoshkar*]"],
    ["a hand in front of the mouth", "[YOU SAY: ద then ಧ, and ಬ then ಭ, with a hand in front of your mouth]"],
    ["a remembered hand at the mouth", "- [YOU RECALL: *chār* from the last chapter, and the plain *ch* you tested with a hand at your mouth]"],
    // A nested cue whose verb is manual.
    ["a nested show", "- [YOU HEAR: *añcŭ*; YOU SHOW: 5]"],
    ["a nested write", "[YOU SAY: *ek*; YOU WRITE: **एक**]"],
    ["a cue wrapped across two source lines", "- [YOU SAY: *eki*,\n  clapping two beats]"],
  ])("%s", (_label, markdown) => {
    expect(gestureSpokenCues(markdown)).toHaveLength(1);
  });

  it("quotes the cue as authored, one entry per cue", () => {
    const markdown = "- [YOU SAY: *eki*, clapping two beats]\n- [YOU SAY: *eki*]\n- [YOU HEAR: *aintu*; YOU SHOW: 5]";
    expect(gestureSpokenCues(markdown)).toEqual(["[YOU SAY: *eki*, clapping two beats]", "[YOU HEAR: *aintu*; YOU SHOW: 5]"]);
  });
});

describe("gestureSpokenCues: what does not fire", () => {
  it.each([
    // The rewrites.
    ["the rewrite of a demonstrative drill", '- [YOU SAY: "இங்கே" three times, picturing something different each time]'],
    ["the rewrite of a mora drill", "- [YOU SAY: *denwa*, then count its beats aloud — three]"],
    ["the rewrite of finger counting", "- [YOU HEAR: *añcŭ*, then say the number — five]"],
    ["the rewrite of raising each hand", "- [YOU SAY: *valadu kai*, then *iḍadu kai* — and say *right* or *left* after each]"],
    ["the rewrite of a pen held up", "- [YOU SAY: *kore*, picturing a pen in your hand, then *koko*, picturing the spot where you stand]"],
    ["the rewrite of a bow", '- [YOU SAY: "vaṇakkam", the word that names the bow]'],
    ["the rewrite of palms together", "- [YOU SAY: hello / goodbye, the palms-together greeting — *nômoshkar*]"],
    ["the rewrite of a pointing label", "- [YOU SAY: these brothers here — *ei bhāirā*]"],
    ["a gesture the narration defers", "- [YOU POINT: **み** and **め** inside it]"],
    ["a reading the narration defers", "- [YOU READ: both printed lines, then point to the two words that mean *give* and *take*]"],
    // Vocabulary, glosses and material.
    ["a gloss after for to", "[YOU RECALL: say the Tamil for to touch, then the Tamil for to sneeze, then say *viḻuṅku* again]"],
    ["a gloss of to shake hands", "[YOU RECALL: say the Tamil for to shake hands, then the Tamil for to take a photo, then say *naṭi* again]"],
    ["a gloss of to clap", "[YOU RECALL: say the Kannada for to whisper, then the Kannada for to clap]"],
    ["a list of body words", "[YOU SAY: a heel, a fist, a palm, the liver, a lung]"],
    ["a list of verbs", "[YOU SAY: to cool, to tap, to pluck, to till, to grow]"],
    ["a word and its meaning", '[YOU SAY: "kai" — hand]'],
    ["material about hands", '[YOU SAY: "Ich mache die Hand auf. Ich mache die Hand zu."]'],
    ["material to translate", "[YOU SAY: you must show your card at the entrance]"],
    ["a speech act", "[YOU SAY: wave away an apology — *paravāgilla*]"],
    ["the tongue's tap", '[YOU SAY: "gracias" — *GRAH-syahs*, one soft tap on the *r*]'],
    ["a single consonant", "[YOU SAY: *kuṭi*, touching the sound once]"],
    ["a wobble in the voice", '[YOU SAY: "comme ci, comme ça" — with a little hand-wobble in the voice]'],
    ["the pointing words, named", "[YOU SAY: the three pointing and person words you now own — **maiṅ, yih, voh**]"],
    ["a word that points", "[YOU RECALL: two chapters back, the word that points at something near — *ei*]"],
    ["point out, as a gloss", "[YOU RECALL: say the French for to imagine, then the French for to point out, then say *informer* again]"],
    ["a thing to say about hands", "[YOU SAY: *koṭu*, then *vāṅgu* — and say which way each hand is moving]"],
    ["a noun point", "[YOU SAY: the honest point — Arabic noon carries a religious meaning]"],
    ["raising the pitch", '[YOU SAY: ask it — "¿Hablas español?" (raise the pitch; ¿ opens it)]'],
    ["a quotation being said", '[YOU SAY: "and clap twice" once more]'],
    ["a nested spoken cue", "[YOU HEAR: *ek*; YOU SAY: one]"],
    ["nested words in lower case", "[YOU SAY: you show: 5 of them]"],
  ])("%s", (_label, markdown) => {
    expect(gestureSpokenCues(markdown)).toEqual([]);
  });
});

describe("spokenCueAsksForGesture stays linear", () => {
  // About 50,000 characters each; only the answers are asserted, because
  // timing bounds flake on a loaded runner.
  it.each([
    ["a long run of spaces before the verb", `say it${" ".repeat(50_000)}and clap twice`, true],
    ["a long run of spaces with no verb", `say it${" ".repeat(50_000)}x`, false],
    ["a long run of spaces inside a manner phrase", `say it with${" ".repeat(50_000)}a hand`, true],
    ["many links with no verb", `say ${"it, and then ".repeat(4_000)}stop`, false],
    ["many links, the last one pointing", `say ${"it, and then ".repeat(4_000)}point at it`, true],
    ["many near misses on point", `${"and point out ".repeat(4_000)}`, false],
    ["many near misses on tap", `${"and tap the r ".repeat(4_000)}`, false],
    ["many near misses on raise", `${"and raise the pitch ".repeat(3_000)}`, false],
    ["many near misses on with", `${"with a little hand ".repeat(3_000)}`, false],
    ["many unclosed quotation openers", `say ${"“".repeat(50_000)} and touch it`, true],
    ["a quotation that hides every gesture", `say “${"and clap twice ".repeat(4_000)}” now`, false],
    ["many nested spoken cues", `${"YOU SAY: a; ".repeat(5_000)}`, false],
    ["many nested spoken cues, the last one manual", `${"YOU SAY: a; ".repeat(5_000)}YOU SHOW: 5`, true],
    ["a long run of capitals before a colon", `YOU ${"A".repeat(50_000)}: x`, false],
    ["many YOUs with no colon", `${"YOU SHOW ".repeat(6_000)}`, false],
  ])("%s", (_label, content, expected) => {
    expect(content.length).toBeGreaterThan(40_000);
    expect(spokenCueAsksForGesture(content)).toBe(expected);
  });
});

// ---------------------------------------------------------------------------
// The corpus
// ---------------------------------------------------------------------------

const { lessons, registry } = loadEverything();
const manifest = loadModalityManifest();
const drivableIds = new Set(manifest.lessons.filter((row) => row.drivable).map((row) => row.id));
const lessonById = new Map(lessons.map((lesson) => [String(lesson.frontmatter.id), lesson]));

/** The whole of a lesson as the narrator reads it: the preamble and every block. */
function lessonMarkdown(lesson: (typeof lessons)[number]): string {
  return [lesson.preamble, ...lesson.blocks.map((block) => block.markdown ?? "")].join("\n\n");
}

/** Every drivable lesson's instructions to write, grouped by track. Clean lessons are absent. */
function findOffenders(): Map<string, Map<string, string[]>> {
  const byTrack = new Map<string, Map<string, string[]>>();
  for (const lesson of lessons) {
    const id = String(lesson.frontmatter.id);
    if (!drivableIds.has(id)) continue;
    // A drivable lesson has no detachable block to set aside: the whole lesson
    // is read, so the preamble and every block are scanned.
    const markdown = lessonMarkdown(lesson);
    const hits = drivableWritingInstructions(markdown);
    if (hits.length === 0) continue;
    const track = byTrack.get(lesson.language) ?? new Map<string, string[]>();
    track.set(id, hits);
    byTrack.set(lesson.language, track);
  }
  return byTrack;
}

/**
 * The ledger files, or none when the directory is absent (every track's debt
 * paid; see the header).
 */
function ledgerEntries(): Dirent[] {
  return existsSync(DEBT_DIRECTORY) ? readdirSync(DEBT_DIRECTORY, { withFileTypes: true }) : [];
}

/** The committed ledger: track -> the lesson ids recorded as debt. */
function loadDebt(): Map<string, string[]> {
  const debt = new Map<string, string[]>();
  for (const entry of ledgerEntries()) {
    // Read only what the well-formed test below accepts: a regular file named
    // `<track>.json`. Anything else (a symlink, a stray file) is that test's
    // failure to report, not something to open and parse here.
    if (!entry.isFile() || !/^[a-z]+\.json$/.test(entry.name)) continue;
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
  });

  it("the detector still fires on the real corpus, where writing is legitimate", () => {
    // The other half of anti-vacuity. With every track's debt paid, "no
    // drivable lesson offends" is also what a detector that had stopped
    // matching anything would report. The lessons that are NOT drivable carry
    // real pen work in both shapes — prose ("Write all five from dictation.")
    // and recall cues ("[YOU RECALL: write **ば** — **R1**]") — and their
    // narration already opens with the hands-and-eyes notice, so they are
    // where the detector must keep firing.
    let prose = 0;
    let recall = 0;
    for (const lesson of lessons) {
      if (drivableIds.has(String(lesson.frontmatter.id))) continue;
      const markdown = lessonMarkdown(lesson);
      if (bareWritingImperatives(markdown).length > 0) prose += 1;
      if (writingRecallCues(markdown).length > 0) recall += 1;
    }
    // Floors set well below the counts measured when this was written (1,218
    // and 35 of 2,467 non-drivable lessons), but high enough that a detector
    // which lost most of its matches fails here rather than passing quietly.
    expect(prose, "pen lessons with a prose writing instruction").toBeGreaterThan(1000);
    expect(recall, "pen lessons with a writing recall cue").toBeGreaterThan(25);
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
    for (const entry of ledgerEntries()) {
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
          `NEW  ${id}: a drivable lesson tells the listener to write, in bare prose or a spoken cue. ` +
            `Author it as a [YOU WRITE: …] cue (or make the task spoken); a ` +
            `[YOU RECALL: write X — **R1**] becomes [YOU WRITE: X from memory — **R1**]:\n` +
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

describe("drivable lessons carry no recall cue that points at the page", () => {
  it("no drivable lesson asks a driver to point", () => {
    // No ledger: every such cue was rewritten for the ear when the check
    // arrived, so the corpus answer is exactly zero. The fixtures above keep
    // the detector honest, since a detector that matched nothing would also
    // report zero here.
    const problems: string[] = [];
    for (const lesson of lessons) {
      const id = String(lesson.frontmatter.id);
      if (!drivableIds.has(id)) continue;
      for (const cue of pointingRecallCues(lessonMarkdown(lesson))) {
        problems.push(
          `${id}: a drivable recall asks the listener to point at the page. Say the same ` +
            `thing for the ear ("name the sign in …", "say which sign in … carries …"), ` +
            `or make it a [YOU POINT: …] cue, which the narration defers:\n       ${cue.slice(0, 160)}`,
        );
      }
    }
    expect(problems, problems.join("\n")).toEqual([]);
  });
});

describe("drivable lessons carry no spoken cue that asks to read script", () => {
  it("no drivable lesson asks a driver to read the page inside a spoken cue", () => {
    // No ledger: every such cue was split into its spoken steps and a READ
    // cue when the check arrived (recalls first, then RETURN TO, SAY and RUN
    // when it was widened to every spoken verb), so the corpus answer is
    // exactly zero.
    const problems: string[] = [];
    for (const lesson of lessons) {
      const id = String(lesson.frontmatter.id);
      if (!drivableIds.has(id)) continue;
      for (const cue of readingSpokenCues(lessonMarkdown(lesson))) {
        problems.push(
          `${id}: a drivable spoken cue asks the listener to read printed script. Move the ` +
            `reading into its own [YOU READ: …] cue, which the narration defers, and keep ` +
            `the spoken steps in their own cue verb, in the authored order:\n       ${cue.slice(0, 160)}`,
        );
      }
    }
    expect(problems, problems.join("\n")).toEqual([]);
  });

  it("the reading check still fires on the real corpus, where reading is legitimate", () => {
    // Anti-vacuity, as for the writing check: a detector that matched nothing
    // would also report zero above. The lessons that are NOT drivable keep
    // their reading cues (their narration already opens with the
    // hands-and-eyes notice), so the detector must keep finding them there.
    let reading = 0;
    for (const lesson of lessons) {
      if (drivableIds.has(String(lesson.frontmatter.id))) continue;
      if (readingSpokenCues(lessonMarkdown(lesson)).length > 0) reading += 1;
    }
    // A floor set well below the count measured when this was written
    // (66 non-drivable lessons for recalls alone, 67 once every spoken verb was
    // read), and high enough that a detector which lost most of its matches
    // fails here rather than passing quietly.
    expect(reading, "non-drivable lessons with a reading cue").toBeGreaterThan(40);
  });
});

describe("drivable lessons carry no spoken cue that asks for a gesture", () => {
  it("no drivable lesson asks a driver to clap, point, touch or show fingers inside a spoken cue", () => {
    // No ledger: every such cue was rewritten for the ear when the check
    // arrived, so the corpus answer is exactly zero.
    const problems: string[] = [];
    for (const lesson of lessons) {
      const id = String(lesson.frontmatter.id);
      if (!drivableIds.has(id)) continue;
      for (const cue of gestureSpokenCues(lessonMarkdown(lesson))) {
        problems.push(
          `${id}: a drivable spoken cue asks the listener to make a gesture. Say the same thing ` +
            `for the ear ("then count its beats aloud — three", "picturing something different ` +
            `each time", "then say the number"), or move the gesture into a cue the narration ` +
            `defers ([YOU POINT: …]):\n       ${cue.slice(0, 160)}`,
        );
      }
    }
    expect(problems, problems.join("\n")).toEqual([]);
  });

  it("the gesture check still fires on the real corpus, where gestures are legitimate", () => {
    // Anti-vacuity, as for the other cue checks: a detector that matched
    // nothing would also report zero above. Lessons that are NOT drivable keep
    // their gesture work (digit lessons that point at a printed figure,
    // aspiration drills with a hand in front of the mouth), and their
    // narration already opens with the hands-and-eyes notice.
    let gesture = 0;
    for (const lesson of lessons) {
      if (drivableIds.has(String(lesson.frontmatter.id))) continue;
      if (gestureSpokenCues(lessonMarkdown(lesson)).length > 0) gesture += 1;
    }
    // A floor set below the count measured when this was written (29
    // non-drivable lessons), and high enough that a detector which lost most
    // of its matches fails here rather than passing quietly.
    expect(gesture, "non-drivable lessons with a gesture cue").toBeGreaterThan(20);
  });
});
