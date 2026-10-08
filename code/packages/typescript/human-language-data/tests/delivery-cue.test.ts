// Tests for the delivery-cue grammar shared by the book and the narration.
//
// The grammar decides only two things --- where a cue's bracket closes and what
// its words mean --- so these tests are about exactly that. How a cue LOOKS is
// tested where it is rendered: book.test.ts and narration.test.ts.

import { describe, expect, it } from "vitest";
import {
  closingBracket,
  joinQualifier,
  joinWrappedLines,
  MAX_CUE_LENGTH,
  opensDeliveryCue,
  parseDeliveryCue,
  readNumber,
} from "../src/delivery-cue.js";

describe("closingBracket", () => {
  it("tracks depth, so brackets nested inside a cue stay inside it", () => {
    const text = '[YOU SAY: "what [is] your name?"] after';
    expect(closingBracket(text, 0)).toBe(text.indexOf("] after"));
  });

  it("crosses newlines --- whether a cue may wrap is the caller's decision", () => {
    expect(closingBracket("[YOU SAY: a\nb]", 0)).toBe(13);
  });

  it("skips an escaped bracket", () => {
    expect(closingBracket("[a \\] b]", 0)).toBe(7);
  });

  it("returns -1 for an unclosed bracket and for one beyond the bound", () => {
    expect(closingBracket("[YOU SAY: never", 0)).toBe(-1);
    const far = `[YOU SAY: ${"x".repeat(MAX_CUE_LENGTH)}]`;
    expect(closingBracket(far, 0)).toBe(-1);
  });
});

describe("opensDeliveryCue", () => {
  it("accepts the three keywords followed by a blank, including a wrap", () => {
    expect(opensDeliveryCue("[YOU SAY: a]", 0)).toBe(true);
    expect(opensDeliveryCue("[PAUSE 2s]", 0)).toBe(true);
    expect(opensDeliveryCue("[REPEAT\nx2]", 0)).toBe(true);
    expect(opensDeliveryCue("x [YOU\nSAY: a]", 2)).toBe(true);
  });

  it("rejects glosses and words that merely start the same way", () => {
    expect(opensDeliveryCue("[I am your friend]", 0)).toBe(false);
    expect(opensDeliveryCue("[YOUR turn]", 0)).toBe(false);
    expect(opensDeliveryCue("[PAUSED]", 0)).toBe(false);
    expect(opensDeliveryCue("[you say: a]", 0)).toBe(false);
  });
});

describe("parseDeliveryCue", () => {
  it("parses pauses, per-item pauses and repeats", () => {
    expect(parseDeliveryCue("PAUSE 2s")).toEqual({ kind: "pause", seconds: 2, perItem: false });
    expect(parseDeliveryCue("PAUSE 1s each")).toEqual({ kind: "pause", seconds: 1, perItem: true });
    expect(parseDeliveryCue("PAUSE 1.5s")).toMatchObject({ seconds: 1.5 });
    expect(parseDeliveryCue("REPEAT x2")).toEqual({ kind: "repeat", times: 2 });
    expect(parseDeliveryCue("REPEAT 3")).toEqual({ kind: "repeat", times: 3 });
  });

  it("parses a prompt, its multi-word verb, and its content", () => {
    expect(parseDeliveryCue("YOU SAY: hola")).toEqual({
      kind: "prompt",
      action: "SAY",
      qualifier: "",
      content: "hola",
    });
    expect(parseDeliveryCue("YOU CHOOSE BY CONTEXT: clock → hour")).toMatchObject({
      action: "CHOOSE BY CONTEXT",
      qualifier: "",
    });
  });

  it("separates a qualifier from the verb (the 18 cues both views used to miss)", () => {
    expect(parseDeliveryCue("YOU SAY (m.): boltā · (f.): boltī")).toEqual({
      kind: "prompt",
      action: "SAY",
      qualifier: "(m.)",
      content: "boltā · (f.): boltī",
    });
    expect(parseDeliveryCue("YOU SWAP the middle: a · b")).toMatchObject({
      action: "SWAP",
      qualifier: "the middle",
    });
    expect(parseDeliveryCue("YOU READ ALOUD, one by one: x")).toMatchObject({
      action: "READ ALOUD",
      qualifier: ", one by one",
    });
  });

  it("re-joins a wrapped cue exactly as Markdown joins wrapped lines", () => {
    expect(parseDeliveryCue("YOU WRITE: the word,\n   then its\nmeaning")).toMatchObject({
      action: "WRITE",
      content: "the word, then its meaning",
    });
    expect(parseDeliveryCue("YOU\nSAY: hola")).toMatchObject({ action: "SAY" });
    expect(parseDeliveryCue("PAUSE\n2s")).toEqual({ kind: "pause", seconds: 2, perItem: false });
  });

  it("accepts any blank after the keyword, as the keyword filter does", () => {
    expect(parseDeliveryCue("YOU\tSAY: hi")).toMatchObject({ action: "SAY", content: "hi" });
    expect(parseDeliveryCue("PAUSE\t2s")).toEqual({ kind: "pause", seconds: 2, perItem: false });
    expect(parseDeliveryCue("REPEAT\tx2")).toEqual({ kind: "repeat", times: 2 });
    expect(parseDeliveryCue("PAUSE  2s")).toMatchObject({ seconds: 2 });
    for (const inner of ["YOU\tSAY: hi", "PAUSE\t2s", "REPEAT\tx2"]) {
      expect(opensDeliveryCue(`[${inner}]`, 0), inner).toBe(true);
    }
  });

  it("returns null for every bracket that is not a cue", () => {
    for (const inner of [
      "bonjour",
      "I am your friend",
      "PAUSE soon",
      "PAUSE 2s please",
      "REPEAT often",
      "REPEAT x2 more",
      "YOU there",
      "YOU think: lower case is prose",
      "YOU SAY-IT: glued",
      "YOU SAYx: glued",
      "You say: hola",
      "YOUSAY: glued",
      "PAUSE2s",
      "",
    ]) {
      expect(parseDeliveryCue(inner), inner).toBeNull();
    }
  });
});

describe("helpers", () => {
  it("reads a decimal number or nothing", () => {
    expect(readNumber("12s", 0)).toEqual({ value: 12, next: 2 });
    expect(readNumber("1.5s", 0)).toEqual({ value: 1.5, next: 3 });
    expect(readNumber("1.s", 0)).toEqual({ value: 1, next: 1 });
    expect(readNumber("s", 0)).toBeNull();
  });

  it("joins wrapped lines with one space and keeps a line's own inner spacing", () => {
    expect(joinWrappedLines("a  ·  b")).toBe("a  ·  b");
    expect(joinWrappedLines("  a\n   b  \nc ")).toBe("a b c");
  });

  it("joins a qualifier to its verb with a space, or flush before a comma", () => {
    expect(joinQualifier("Say it", "(m.)")).toBe("Say it (m.)");
    expect(joinQualifier("Read aloud", ", one by one")).toBe("Read aloud, one by one");
    expect(joinQualifier("Say it", "")).toBe("Say it");
  });
});
