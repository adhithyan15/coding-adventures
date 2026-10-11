// HL-C443, list captions — a strip that writes a LIST of letters ("ن، ت، ث",
// "ক — ণ — শ") is captioned as a list, not as one thing, and its letters are
// listed after the sentence with an English comma. Two things went wrong
// before: the sentence read "How ক — ণ — শ is written" in the app, and the
// Arabic comma ، (Script_Extensions=Arab) joined the letter before it inside
// the book's script macro, printing \ar{ن،}.
import { describe, expect, it } from "vitest";
import { renderInlineMarkdown } from "../../src/book.js";
import type { ScriptFilmstripTarget } from "../../src/figure.js";
import {
  filmstripCaption,
  LIST_SEPARATORS,
  listCaptionParts,
  listItemsOf,
  listNoun,
} from "../../src/filmstrip-caption.js";
import { filmstripImageMarkdown, writingSequenceOf } from "../../src/figure-targets.js";
import { lesson } from "./fixture.js";

function sequence(lessonId: string, script: string, glyph: string, letters: string[]): ScriptFilmstripTarget {
  return {
    kind: "script-filmstrip",
    lessonId,
    script,
    glyph,
    letters,
    output: `${script}/book/figures/${lessonId}-filmstrip.svg`,
  };
}

describe("which headwords are lists", () => {
  it("reads two or more one-grapheme items, whatever separates them", () => {
    expect(listItemsOf("ن، ت، ث")).toEqual(["ن", "ت", "ث"]);
    expect(listItemsOf("ক — ণ — শ")).toEqual(["ক", "ণ", "শ"]);
    expect(listItemsOf("வ, க")).toEqual(["வ", "க"]);
    expect(listItemsOf("ਸ · ਤ · ਕ")).toEqual(["ਸ", "ਤ", "ਕ"]);
    expect(listItemsOf("人 、口")).toEqual(["人", "口"]);
    expect(listItemsOf("  ¿ ¡  ")).toEqual(["¿", "¡"]);
    // A grapheme with its sign is ONE item.
    expect(listItemsOf("કે, ખ")).toEqual(["કે", "ખ"]);
  });

  it("is not a letter, a word, or several words", () => {
    expect(listItemsOf("ক")).toBeUndefined();
    expect(listItemsOf("はい")).toBeUndefined();
    expect(listItemsOf("buenos días")).toBeUndefined();
    expect(listItemsOf("मम नाम")).toBeUndefined();
    expect(listItemsOf("ி, நன்றி")).toBeUndefined();
    expect(listItemsOf("")).toBeUndefined();
  });

  it("is the same test the sequence strips draw a list by", () => {
    // writingSequenceOf still splits on the shared separators: a list in a
    // script with no written-order table comes back letter for letter.
    expect(writingSequenceOf(lesson("AR-W1", { headword: "ن، ت، ث" }), "arabic")).toEqual(
      listItemsOf("ن، ت، ث"),
    );
    expect(LIST_SEPARATORS.test("،")).toBe(true);
  });

  it("names signs as signs", () => {
    expect(listNoun(["ن", "ت"])).toBe("letters");
    expect(listNoun(["ુ", "ી"])).toBe("signs");
    expect(listNoun(["ૂ", "ટ"])).toBe("letters and signs");
    expect(listCaptionParts(["ك", "ر"])).toEqual({
      subject: "How these letters are written",
      items: "ك, ر",
    });
  });

  it("names digits as digits, and every kind in a mixed list", () => {
    // HL-C443 follow-up: Persian ۰ ۱ was captioned "How these letters are
    // written". A digit (\p{Nd}) is not a letter.
    expect(listNoun(["۰", "۱"])).toBe("digits");
    expect(listNoun(["൧", "൨", "൩"])).toBe("digits");
    expect(listNoun(["7", "3"])).toBe("digits");
    expect(listNoun(["ക", "൧"])).toBe("letters and digits");
    expect(listNoun(["ു", "൧"])).toBe("digits and signs");
    expect(listNoun(["ക", "൧", "ു"])).toBe("letters, digits and signs");
    // Malayalam ൰ (ten) is a number sign, \p{No}: not a decimal digit.
    expect(listNoun(["൯", "൰"])).toBe("letters and digits");
    expect(listCaptionParts(["۰", "۱"])).toEqual({ subject: "How these digits are written", items: "۰, ۱" });
  });

  it("keeps every noun it gave before digits were told apart", () => {
    expect(listNoun([])).toBe("letters");
    expect(listNoun(["¿", "¡"])).toBe("letters");
    expect(listNoun(["કે", "ખ"])).toBe("letters");
    expect(listNoun(["ુ", "ી"])).toBe("signs");
    expect(listNoun(["ૂ", "ટ", "ઈ", "ઢ"])).toBe("letters and signs");
  });
});

describe("the book's caption for a list strip", () => {
  it("lists Arabic letters after the sentence, with no Arabic comma in it", () => {
    const markdown = filmstripImageMarkdown(sequence("AR-W1", "arabic", "ن، ت، ث", ["ن", "ت", "ث"]));
    expect(markdown).toBe(
      "![How these letters are written, one after another, stroke by stroke: ن, ت, ث](figures/AR-W1-filmstrip.svg)",
    );
    expect(markdown).not.toContain("،");
  });

  it("puts each letter in its own script macro, the comma outside it", () => {
    const alt = /^!\[(.*)\]\(/u.exec(
      filmstripImageMarkdown(sequence("AR-W1", "arabic", "ن، ت، ث", ["ن", "ت", "ث"])),
    )![1]!;
    const tex = renderInlineMarkdown(alt, { unicodeScript: "Arabic", scriptCommand: "ar" });
    expect(tex).toBe(
      "How these letters are written, one after another, stroke by stroke: \\ar{ن}, \\ar{ت}, \\ar{ث}",
    );
    // The bug this closes: the lesson's own separator, rendered as it stood.
    expect(renderInlineMarkdown("ن، ت", { unicodeScript: "Arabic", scriptCommand: "ar" })).toBe(
      "\\ar{ن،} \\ar{ت}",
    );
  });

  it("replaces any separator with the English one", () => {
    expect(filmstripImageMarkdown(sequence("GU-R1", "gujarati", "ક — ણ — શ", ["ક", "ણ", "શ"]))).toBe(
      "![How these letters are written, one after another, stroke by stroke: ક, ણ, શ](figures/GU-R1-filmstrip.svg)",
    );
    expect(filmstripImageMarkdown(sequence("PA-S1", "gurmukhi", "ਸ · ਤ · ਕ", ["ਸ", "ਤ", "ਕ"]))).toBe(
      "![How these letters are written, one after another, stroke by stroke: ਸ, ਤ, ਕ](figures/PA-S1-filmstrip.svg)",
    );
  });

  it("says part by part, and names the signs, when a piece is a sign", () => {
    expect(
      filmstripImageMarkdown(sequence("GU-S1", "gujarati", "ુ ી", ["ુ", "ી"])),
    ).toBe(
      "![How these signs are written, part by part, stroke by stroke: ુ, ી](figures/GU-S1-filmstrip.svg)",
    );
    expect(
      filmstripImageMarkdown(
        sequence("GU-S2", "gujarati", "ૂ — ટ — ઈ — ઢ", ["ૂ", "ટ", "ઈ", "ઢ"]),
      ),
    ).toBe(
      "![How these letters and signs are written, part by part, stroke by stroke: ૂ, ટ, ઈ, ઢ](figures/GU-S2-filmstrip.svg)",
    );
  });

  it("names a list of digits as digits", () => {
    expect(filmstripImageMarkdown(sequence("FA-W19", "persian", "۰ ۱", ["۰", "۱"]))).toBe(
      "![How these digits are written, one after another, stroke by stroke: ۰, ۱](figures/FA-W19-filmstrip.svg)",
    );
    expect(filmstripImageMarkdown(sequence("ML-W07", "malayalam", "൧ ൨ ൩", ["൧", "൨", "൩"]))).toBe(
      "![How these digits are written, one after another, stroke by stroke: ൧, ൨, ൩](figures/ML-W07-filmstrip.svg)",
    );
  });

  it("leaves a single letter, a word and a sign-bearing word as they were", () => {
    expect(
      filmstripImageMarkdown({
        kind: "script-filmstrip",
        lessonId: "BN-W1",
        script: "bengali",
        glyph: "ক",
        output: "bengali/book/figures/BN-W1-filmstrip.svg",
      }),
    ).toBe("![How ক is written, stroke by stroke](figures/BN-W1-filmstrip.svg)");
    expect(filmstripImageMarkdown(sequence("JA-W1", "japanese", "はい", ["は", "い"]))).toBe(
      "![How はい is written, letter by letter, stroke by stroke](figures/JA-W1-filmstrip.svg)",
    );
    expect(filmstripImageMarkdown(sequence("TA-W1", "tamil", "மேசை", ["ே", "ம", "ை", "ச"]))).toBe(
      "![How மேசை is written, part by part, stroke by stroke](figures/TA-W1-filmstrip.svg)",
    );
  });
});

describe("the app's caption, from the headword alone", () => {
  it("keeps a single letter and a word as they were", () => {
    expect(filmstripCaption("ক")).toBe("How ক is written, stroke by stroke");
    expect(filmstripCaption("はい")).toBe("How はい is written, stroke by stroke");
    expect(filmstripCaption("मम नाम")).toBe("How मम नाम is written, stroke by stroke");
  });

  it("captions a list as a list", () => {
    expect(filmstripCaption("ক — ণ — শ")).toBe("How these letters are written, stroke by stroke: ক, ণ, শ");
    expect(filmstripCaption("ن، ت، ث")).toBe("How these letters are written, stroke by stroke: ن, ت, ث");
    expect(filmstripCaption("ુ ી")).toBe("How these signs are written, stroke by stroke: ુ, ી");
    expect(filmstripCaption("۰ ۱")).toBe("How these digits are written, stroke by stroke: ۰, ۱");
    expect(filmstripCaption("൧ ൨ ൩")).toBe("How these digits are written, stroke by stroke: ൧, ൨, ൩");
  });
});
