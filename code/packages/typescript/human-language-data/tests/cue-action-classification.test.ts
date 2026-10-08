// Every cue verb in the corpus is classified as spoken or manual.
//
// ---------------------------------------------------------------------------
// The failure this prevents
// ---------------------------------------------------------------------------
//
// The narration decides whether a driver may do a `[YOU <VERB>: …]` cue by
// looking the verb up in `MANUAL_CUE_ACTIONS`. A verb that is not there is
// read out as an ordinary turn — "your turn — copy: राम-राम सा once" — with
// nothing to say "not now". That default was safe only while somebody kept
// the set complete, and nobody did: COPY, CIRCLE, LOOK, READ, FIND, TAP and
// COVER were all in use, several in drivable lessons, before any of them was
// listed.
//
// So a default is no longer enough. `SPOKEN_CUE_ACTIONS` names the verbs a
// driver CAN do, and this test walks every cue in every lesson and fails on a
// head verb in neither set. A new verb therefore arrives as a failing test
// that says "decide", not as a silent turn read to someone at speed.
//
// ---------------------------------------------------------------------------
// What counts as a cue, and as its verb
// ---------------------------------------------------------------------------
//
// Cues are found with the book's and the narration's own grammar:
// `closingBracket` to find the end of a bracket (nesting and wrapped lines
// included) and `parseDeliveryCue` to decide whether it is a cue at all. Both
// renderers use exactly these two, so this sees what they see — and a little
// more, because block titles and HTML comments are scanned too. Over-reading
// is the safe direction here: it can only ask for a verb to be classified
// that no renderer shows, never let one through that a renderer does show.
//
// The HEAD verb is the first word of the action: SAY in `SAY WHY`, READ in
// `READ ALOUD`, RETURN in `RETURN TO`. The later words are qualifiers or a
// second verb; `isManualCueAction` reads all of them, so `COVER AND WRITE` is
// manual twice over, and a manual word anywhere in an action defers it.

import { describe, expect, it } from "vitest";
import { closingBracket, parseDeliveryCue } from "../src/delivery-cue.js";
import { loadEverything, loadModalityManifest } from "../src/loader.js";
import {
  isManualCueAction,
  MANUAL_CUE_ACTIONS,
  parseNarrationCue,
  SPOKEN_CUE_ACTIONS,
} from "../src/narration.js";

/** One use of a verb: the lesson it is in and the cue as authored. */
interface CueUse {
  lessonId: string;
  drivable: boolean;
  /** The cue's words after the colon, still Markdown. */
  content: string;
  source: string;
}

/**
 * Every prompt cue in `text`, as `{ action, source }`.
 *
 * Walks `[` by `[` with `indexOf`, and lets `closingBracket` (whose lookahead
 * is bounded by MAX_CUE_LENGTH) find each end, so the scan is linear in the
 * text however many brackets it holds.
 */
function promptCues(text: string): Array<{ action: string; content: string; source: string }> {
  const cues: Array<{ action: string; content: string; source: string }> = [];
  for (let open = text.indexOf("["); open !== -1; open = text.indexOf("[", open + 1)) {
    const close = closingBracket(text, open);
    if (close < 0) continue;
    const cue = parseDeliveryCue(text.slice(open + 1, close));
    if (cue?.kind !== "prompt") continue;
    cues.push({
      action: cue.action,
      content: cue.content,
      source: text.slice(open, close + 1).replace(/\s+/g, " "),
    });
  }
  return cues;
}

const { lessons } = loadEverything();
const drivableIds = new Set(
  loadModalityManifest().lessons.filter((row) => row.drivable).map((row) => row.id),
);

/** Head verb -> every use of it in the corpus. */
const usesByVerb = new Map<string, CueUse[]>();
for (const lesson of lessons) {
  const lessonId = String(lesson.frontmatter.id);
  const texts = [lesson.preamble, ...lesson.blocks.flatMap((block) => [block.title ?? "", block.markdown ?? ""])];
  for (const text of texts) {
    for (const cue of promptCues(text)) {
      const head = cue.action.split(" ")[0] ?? cue.action;
      const uses = usesByVerb.get(head) ?? [];
      uses.push({ lessonId, drivable: drivableIds.has(lessonId), content: cue.content, source: cue.source });
      usesByVerb.set(head, uses);
    }
  }
}

describe("the two cue-verb sets", () => {
  it("do not overlap", () => {
    // A verb in both would be manual (the narration only reads the manual
    // set), so a SPOKEN entry for it would be a lie someone might believe.
    const both = [...MANUAL_CUE_ACTIONS].filter((verb) => SPOKEN_CUE_ACTIONS.has(verb));
    expect(both).toEqual([]);
  });

  it("hold single upper-case words, which is all a head verb can be", () => {
    for (const verb of [...MANUAL_CUE_ACTIONS, ...SPOKEN_CUE_ACTIONS]) {
      expect(verb, verb).toMatch(/^[A-Z]+$/);
    }
  });

  it("decide the narration's spoken flag", () => {
    // The sets are only worth guarding if the narration obeys them.
    for (const verb of MANUAL_CUE_ACTIONS) {
      expect(parseNarrationCue(`YOU ${verb}: x`), verb).toMatchObject({ spoken: false });
    }
    for (const verb of SPOKEN_CUE_ACTIONS) {
      expect(parseNarrationCue(`YOU ${verb}: x`), verb).toMatchObject({ spoken: true });
    }
  });

  it("read every word of a compound action", () => {
    expect(isManualCueAction("COVER AND WRITE")).toBe(true);
    expect(isManualCueAction("READ ALOUD")).toBe(true);
    expect(isManualCueAction("SAY AND WRITE")).toBe(true);
    expect(isManualCueAction("WRITE FROM THE HEARD OR ROMANIZED CUE")).toBe(true);
    expect(isManualCueAction("SAY WHY")).toBe(false);
    expect(isManualCueAction("CHOOSE BY CONTEXT")).toBe(false);
    expect(parseNarrationCue("YOU SAY AND WRITE: x")).toMatchObject({ spoken: false });
  });

  it("errs toward manual for an outside caller's lower case or odd spacing", () => {
    // The parser never produces these, but the function is exported, and "spoken"
    // is the unsafe wrong answer for a driver.
    expect(isManualCueAction("write")).toBe(true);
    expect(isManualCueAction("SAY\tWRITE")).toBe(true);
    expect(isManualCueAction("  say   and  write ")).toBe(true);
    expect(isManualCueAction("say why")).toBe(false);
    expect(isManualCueAction(" ".repeat(200_000))).toBe(false);
  });

  it("defer the hands-on cues that were read to a driver", () => {
    // The cues that prompted the guard, one per verb, as authored.
    for (const source of [
      "YOU COPY: **राम-राम सा** once, keeping the hyphen and space",
      "YOU CIRCLE: the first base letter **ఝ**",
      "YOU LOOK: at आओगे and put your finger on the letter **ओ**",
      "YOU READ: **でぐち** — an exit",
      "YOU FIND: the familiar **య** at the end",
      "YOU TAP: *yu | hold | ku | ri*",
      // Nested inside a HEAR cue in four drivable number drills until they were
      // rewritten; deferred now should it ever head a cue.
      "YOU SHOW: 5",
    ]) {
      expect(parseNarrationCue(source), source).toMatchObject({ kind: "prompt", spoken: false });
    }
  });
});

describe("every cue verb in the corpus is classified", () => {
  it("scans a real corpus, not an empty one", () => {
    // Anti-vacuity: an empty walk would classify nothing and pass.
    const total = [...usesByVerb.values()].reduce((sum, uses) => sum + uses.length, 0);
    expect(total).toBeGreaterThan(100_000);
    expect(usesByVerb.has("SAY")).toBe(true);
    expect(usesByVerb.has("WRITE")).toBe(true);
  });

  it("has no head verb outside SPOKEN_CUE_ACTIONS and MANUAL_CUE_ACTIONS", () => {
    const problems: string[] = [];
    for (const [verb, uses] of [...usesByVerb].sort(([left], [right]) => left.localeCompare(right))) {
      if (MANUAL_CUE_ACTIONS.has(verb) || SPOKEN_CUE_ACTIONS.has(verb)) continue;
      const drivable = uses.filter((use) => use.drivable).length;
      const example = uses.find((use) => use.drivable) ?? uses[0]!;
      problems.push(
        `[YOU ${verb}: …] is used ${uses.length} time(s), ${drivable} in drivable lessons, and is in ` +
          `neither set. If a driver can do it with the voice alone, add it to SPOKEN_CUE_ACTIONS ` +
          `in src/narration.ts; if it needs a hand or an eye, add it to MANUAL_CUE_ACTIONS so the ` +
          `narration defers it ("once you have stopped driving — …").\n` +
          `       e.g. ${example.lessonId}: ${example.source.slice(0, 160)}`,
      );
    }
    expect(problems, problems.join("\n")).toEqual([]);
  });
});

describe("a POINT cue names what to point at", () => {
  it("does not open with the preposition the book's label already says", () => {
    // The book prints `[YOU POINT: …]` as "*Point to:* …" and the narration as
    // "point: …", so the cue's own words must be the thing pointed at. Older
    // cues wrote "[YOU POINT: to the right edge …]", which printed "Point to:
    // to the right edge"; "at …" printed "Point to: at …". The fix is in the
    // lessons, not the renderer: stripping a leading "to" in the renderer
    // would also strip it from a cue that means it ("to and fro" is a thing a
    // learner could point at), and it would leave the narration's "point: to
    // …" to a second rule of its own.
    const uses = usesByVerb.get("POINT") ?? [];
    expect(uses.length).toBeGreaterThan(20);
    const leading = uses.filter((use) => /^(?:to|at|towards?|in|on)\b/i.test(use.content));
    expect(
      leading.map((use) => `${use.lessonId}: ${use.source}`),
      'drop the leading preposition: "[YOU POINT: the right edge]", not "[YOU POINT: to the right edge]"',
    ).toEqual([]);
  });
});

describe("the classification walk stays linear", () => {
  it("handles 50,000 characters of brackets and cues", () => {
    // Only answers are asserted: timing bounds flake on a loaded runner.
    const unclosed = "[".repeat(50_000);
    expect(promptCues(unclosed)).toEqual([]);
    const many = "[YOU SAY: a] ".repeat(4_000);
    expect(many.length).toBeGreaterThan(40_000);
    expect(promptCues(many)).toHaveLength(4_000);
  });
});
