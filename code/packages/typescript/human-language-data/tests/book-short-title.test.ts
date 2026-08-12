// ---------------------------------------------------------------------------
// Section short titles (HL-C109d).
//
// `\section[short]{full}` sends `short` to two places that are exactly one line
// wide: the table of contents and the running head. An entry that overflows
// there is not merely ugly. `\@dottedtocline` sets `\parfillskip -\rightskip`,
// which cancels the ragged-right stretch on the entry's LAST line -- so a
// wrapped entry has to be justified, and a script with no hyphenation patterns
// cannot justify without a badly stretched line. kannada and telugu each
// carried one such box, from a headword that is legitimately seven weekday
// words long.
//
// These tests pin the SHAPE of the cut: where it falls, what it marks, what it
// leaves alone. Whether the cut actually fixed the page is a different
// question, and only a XeLaTeX run answers it -- see build-books-locally.sh.
// ---------------------------------------------------------------------------
import { describe, expect, it } from "vitest";
import { renderBookChapter } from "../src/book.js";
import { parseLesson } from "../src/parse.js";

function source(headword: string, romanization = ""): string {
  return `---
schema_version: 2
id: T1
spine_node: HELLO
sequence: 10
chapter: 1
type: word
headword: ${headword}
${romanization === "" ? "" : `romanization: ${romanization}\n`}gloss: a test
concept_tag: GREETING-HELLO
prerequisites: []
duration:
  max_seconds: 120
requires:
  knowledge: []
introduces:
  knowledge: []
practises:
  knowledge: []
skills: [reading]
modes: [interpretive]
strands: [meaning-input]
register: neutral
variety: general
---

# ${headword} — lesson

## Warm-up

[PAUSE 2s] Recall it.
`;
}

const target = {
  language: "test",
  chapter: 1,
  title: "Short titles",
  label: "ch:short",
  output: "test/book/chapters/ch01-short.tex",
};

/** The `[...]` of the generated `\section[...]{...}`. */
function shortTitle(headword: string, romanization = ""): string {
  const tex = renderBookChapter(target, [parseLesson(source(headword, romanization), "test")]).tex;
  const match = /\\section\[(.*?)\]\{/.exec(tex);
  if (match === null) throw new Error("no \\section[...] in the generated chapter");
  return match[1]!;
}

describe("section short titles", () => {
  it("leaves an ordinary headword completely alone", () => {
    expect(shortTitle("hola")).toBe("hola");
    // Four words is not by itself too long: the median short title in the
    // corpus is 7 columns wide and this is 19, well inside the budget. A word
    // COUNT would have cut this sentence in half, which is why the rule counts
    // columns instead.
    expect(shortTitle("mucho gusto en conocerte")).toBe("mucho gusto en conocerte");
  });

  it("cuts a headword that would overflow the line, at a word boundary", () => {
    const weekdays = "Montag Dienstag Mittwoch Donnerstag Freitag Samstag Sonntag";
    const cut = shortTitle(weekdays);
    expect(cut).not.toBe(weekdays);
    expect(cut.endsWith("…")).toBe(true);
    // The cut falls between words, never inside one.
    expect(weekdays.startsWith(cut.replace(" …", ""))).toBe(true);
    expect(cut.replace(" …", "").split(" ").every((w) => weekdays.split(" ").includes(w))).toBe(
      true,
    );
  });

  it("does not hand the ellipsis a dangling separator", () => {
    // Cutting a separated list between items leaves behind the separator that
    // was joining them to the item now gone. `sal · sé · …` reads as though
    // something were missing from the middle rather than trimmed from the end.
    expect(shortTitle("di · haz · ve · pon · ten · sal · sé · ven")).toBe(
      "di · haz · ve · pon · ten · sal · sé …",
    );
    // The same for a trailing comma.
    expect(shortTitle("diēs Lūnae, Mārtis, Mercuriī, Iovis, Veneris")).toBe(
      "diēs Lūnae, Mārtis, Mercuriī, Iovis …",
    );
  });

  it("keeps the whole of a single word that is wider than the budget", () => {
    // A truncated word is unreadable, and one long word is a narrower defect
    // than a wrapped list, so it is kept intact rather than cut mid-word.
    const long = "Donaudampfschifffahrtselektrizitaetenhauptbetriebswerkbauunterbeamtengesellschaft";
    expect(shortTitle(long)).toBe(long);
  });

  it("never leaves emphasis unpaired when it cuts", () => {
    // Cutting happens on the authored text, so it can land inside authored
    // `**...**`. When it would, one more word is dropped rather than emitting
    // markup the renderer would mis-pair.
    const split = shortTitle("alpha bravo charlie delta **echo foxtrot** golf hotel india juliet");
    expect(split).toBe("alpha bravo charlie delta …");
    expect(split).not.toContain("**");
    expect(split).not.toContain("\\textbf");

    // When the emphasis fits whole it survives, rendered and brace-balanced.
    const whole = shortTitle("alpha bravo charlie **delta echo** foxtrot golf hotel india juliet");
    expect(whole).toBe("alpha bravo charlie \\textbf{delta echo} …");
    expect((whole.match(/\{/g) ?? []).length).toBe((whole.match(/\}/g) ?? []).length);
  });

  it("measures columns, not characters, so a combining mark is free", () => {
    // Devanagari vowel signs stack on the consonant before them rather than
    // occupying a column of their own. Counting code points would make this
    // headword look far wider than it sets.
    const marked = "कि खी गु घू ङे";
    expect(shortTitle(marked)).toBe(marked);
  });

  it("still says Practice for a practice lesson", () => {
    const practice = source("anything at all here").replace("type: word", "type: practice");
    const tex = renderBookChapter(target, [parseLesson(practice, "test")]).tex;
    expect(tex).toContain("\\section[Practice]{");
  });
});
