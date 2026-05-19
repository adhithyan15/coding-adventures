/**
 * typeset.test.ts — string-level substitution rules.
 */

import { describe, it, expect } from "vitest";
import { typeset } from "../src/index.js";

describe("typeset — smart quotes (double)", () => {
  it("paired straight quotes around a word", () => {
    expect(typeset(`He said "hello"`)).toBe(`He said “hello”`);
  });

  it("left quote at start of string", () => {
    expect(typeset(`"opening`)).toBe(`“opening`);
  });

  it("right quote at end of string (after letter)", () => {
    expect(typeset(`closing"`)).toBe(`closing”`);
  });

  it("after whitespace → left quote", () => {
    expect(typeset(`x "y"`)).toBe(`x “y”`);
  });

  it("after non-breaking space → left quote", () => {
    expect(typeset(`x "y"`)).toBe(`x “y”`);
  });
});

describe("typeset — smart quotes (single + apostrophes)", () => {
  it("apostrophe between letters → right single quote", () => {
    expect(typeset(`don't`)).toBe(`don’t`);
  });

  it("apostrophe after letter (possessive) → right single quote", () => {
    expect(typeset(`it's`)).toBe(`it’s`);
  });

  it("left single quote at start of string", () => {
    expect(typeset(`'opening`)).toBe(`‘opening`);
  });

  it("left single quote after whitespace", () => {
    expect(typeset(`said 'quote'`)).toBe(`said ‘quote’`);
  });

  it("apostrophe after digit → right single", () => {
    expect(typeset(`the '90s`)).toBe(`the ‘90s`);
  });

  it("right single after punctuation (defensive)", () => {
    // Quote after comma — treat as closing.
    expect(typeset(`x,'y`)).toBe(`x,’y`);
  });
});

describe("typeset — dashes", () => {
  it("`--` → en dash", () => {
    expect(typeset(`pages 5--7`)).toBe(`pages 5–7`);
  });

  it("`---` → em dash", () => {
    expect(typeset(`stop---wait`)).toBe(`stop—wait`);
  });

  it("longer pattern beats shorter (`---` doesn't become en dash + hyphen)", () => {
    expect(typeset(`---`)).toBe(`—`);
  });

  it("four hyphens → em dash + single hyphen", () => {
    expect(typeset(`----`)).toBe(`—-`);
  });

  it("single hyphen unchanged", () => {
    expect(typeset(`well-formed`)).toBe(`well-formed`);
  });
});

describe("typeset — ellipsis", () => {
  it("`...` → ellipsis", () => {
    expect(typeset(`wait...`)).toBe(`wait…`);
  });

  it("two dots → unchanged", () => {
    expect(typeset(`wait..`)).toBe(`wait..`);
  });

  it("one dot → unchanged", () => {
    expect(typeset(`end.`)).toBe(`end.`);
  });

  it("four dots → ellipsis + dot", () => {
    expect(typeset(`....`)).toBe(`….`);
  });
});

describe("typeset — ligatures (opt-in)", () => {
  it("default: ligatures off → `(c)` passthrough", () => {
    expect(typeset(`(c)`)).toBe(`(c)`);
  });

  it("ligatures: true → `(c)` → ©", () => {
    expect(typeset(`(c)`, { ligatures: true })).toBe(`©`);
  });

  it("ligatures: true → `(C)` → ©", () => {
    expect(typeset(`(C)`, { ligatures: true })).toBe(`©`);
  });

  it("ligatures: true → `(r)` → ®", () => {
    expect(typeset(`(r)`, { ligatures: true })).toBe(`®`);
  });

  it("ligatures: true → `(R)` → ®", () => {
    expect(typeset(`(R)`, { ligatures: true })).toBe(`®`);
  });

  it("ligatures: true → `(tm)` → ™", () => {
    expect(typeset(`(tm)`, { ligatures: true })).toBe(`™`);
  });

  it("ligatures: true → `(TM)` → ™", () => {
    expect(typeset(`(TM)`, { ligatures: true })).toBe(`™`);
  });

  it("ligatures: true → non-match `(xy)` → passthrough", () => {
    expect(typeset(`(xy)`, { ligatures: true })).toBe(`(xy)`);
  });

  it("ligatures: true → just `(` at end of string → passthrough", () => {
    expect(typeset(`x (`, { ligatures: true })).toBe(`x (`);
  });
});

describe("typeset — option toggles", () => {
  it("smartQuotes: false leaves quotes alone", () => {
    expect(typeset(`"x"`, { smartQuotes: false })).toBe(`"x"`);
  });

  it("dashes: false leaves dashes alone", () => {
    expect(typeset(`a--b`, { dashes: false })).toBe(`a--b`);
  });

  it("ellipsis: false leaves dots alone", () => {
    expect(typeset(`...`, { ellipsis: false })).toBe(`...`);
  });

  it("all disabled → identity fast path", () => {
    const input = `"hello"--don't...`;
    expect(typeset(input, { smartQuotes: false, dashes: false, ellipsis: false })).toBe(input);
  });

  it("only smartQuotes enabled, dashes off", () => {
    expect(typeset(`"x"--y`, { dashes: false })).toBe(`“x”--y`);
  });
});

describe("typeset — combinations", () => {
  it("mixed prose: quotes + dashes + apostrophes + ellipsis", () => {
    expect(typeset(`He said "wait" -- don't go...`))
      .toBe(`He said “wait” – don’t go…`);
  });

  it("opening + closing both correct in long quote", () => {
    expect(typeset(`"To be or not to be"`))
      .toBe(`“To be or not to be”`);
  });
});

describe("typeset — pure / deterministic", () => {
  it("does not mutate input (strings immutable but contract holds)", () => {
    const input = `"x"`;
    typeset(input);
    expect(input).toBe(`"x"`);
  });

  it("same input → byte-identical output", () => {
    const input = `He said "wait"...`;
    expect(typeset(input)).toBe(typeset(input));
  });

  it("non-string input coerced via String(...)", () => {
    // @ts-expect-error — defensive coercion
    expect(typeset(42)).toBe("42");
  });

  it("empty string → empty string", () => {
    expect(typeset(``)).toBe(``);
  });

  it("already-prettified text passes through unchanged for non-substitution chars", () => {
    expect(typeset(`“already”`)).toBe(`“already”`);
  });
});

describe("typeset — Unicode passthrough", () => {
  it("CJK chars pass through unchanged when no substitution applies", () => {
    expect(typeset(`日本語のテキスト`)).toBe(`日本語のテキスト`);
  });

  it("emoji passthrough (surrogate pairs handled as-is)", () => {
    expect(typeset(`hello 🎉 world`)).toBe(`hello 🎉 world`);
  });

  it("quote after CJK char is treated as closing (CJK not whitespace)", () => {
    // 語 (U+8A9E) is not in our whitespace set, so the " after
    // it is a closing right-DQ, then the closing " is after a
    // letter → also right-DQ.  Both produce the same character
    // here, which is the deterministic-rule guarantee we promise.
    expect(typeset(`語"x"`)).toBe(`語”x”`);
  });
});
