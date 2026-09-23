import { parseLesson } from "../src/parse.js";
import { describe, expect, it } from "vitest";
import {
  etymologyFigureSource,
  etymologyRootNode,
  renderEtymologyRouteFigure,
} from "../src/figure.js";

/**
 * A LITERAL vocabulary, not `loadRootTagVocabulary()`.
 *
 * `figure.ts` documents that it stays free of the filesystem so every figure is
 * a pure function of its inputs — which is what makes `check:figures` a byte
 * comparison rather than a re-run. A test that reached for the real
 * `core/root-tags.json` would quietly re-introduce the dependency the module
 * forbids, and would also couple these assertions to a corpus data file.
 */
const TAGS = {
  tags: [
    "arabic",
    "italian",
    "latin",
    "proto-dravidian",
    "proto-indo-european",
    "sanskrit",
    "tamil",
    "turkish",
  ],
  aliases: { pie: "proto-indo-european" },
};

function lesson(roots = "[qahwah-arabic, kahve-turkish, caffè-italian]") {
  return parseLesson(`---
schema_version: 2
id: ES-C06-cafe
spine_node: SPINE-POLITE-REQUEST-REPAIR
sequence: 490
chapter: 6
type: word
headword: café
gloss: coffee
concept_tag: ES-WORD-CAFE
roots: ${roots}
duration:
  max_seconds: 180
requires:
  knowledge: []
introduces:
  knowledge: [ES-LEX-CAFE]
practises:
  knowledge: [ES-LEX-CAFE]
skills: [reading]
modes: [interpretive]
strands: [meaning-input]
register: neutral
variety: general
---

# café

## The word, taken apart

Arabic qahwah became Turkish kahve, then Italian caffè, then Spanish café.
`, "spanish");
}

describe("canonical figure rendering", () => {
  it("parses printable root nodes without inventing data", () => {
    expect(etymologyRootNode("qahwah-arabic", TAGS)).toEqual({ term: "qahwah", language: "Arabic" });
    expect(etymologyRootNode("a-dios-latin", TAGS)).toEqual({ term: "a dios", language: "Latin" });
    expect(() => etymologyRootNode("orphan", TAGS)).toThrow(/language tag/);
  });

  it("finds the tag by vocabulary, not by position, so a PREFIX slug is not read backwards", () => {
    // This function used to be `pieces.pop()`, which is correct for exactly one
    // of the three slug shapes the corpus uses. HL-C419's normalisation flipped
    // `kahve-turkish` to `turkish-kahve` -- the shape the `turkish` tag's own
    // convention prefers -- and the published SVG for ES-C06-cafe went out
    // claiming Arabic qahwah became **Kahve "turkish"**. Nothing threw, because
    // the slug still had two pieces.
    expect(etymologyRootNode("turkish-kahve", TAGS)).toEqual({ term: "kahve", language: "Turkish" });
    expect(etymologyRootNode("sanskrit-kaala-time", TAGS)).toEqual({
      term: "kaala time",
      language: "Sanskrit",
    });
    // Positional reading was ALREADY wrong here before that pass; the
    // normalisation only made a prefix slug reachable from a figure. `pop()`
    // gave the term "proto indo european" in the language "Dwoh".
    expect(etymologyRootNode("proto-indo-european-dwoh", TAGS)).toEqual({
      term: "dwoh",
      language: "Proto-indo-european",
    });
  });

  it("prints the term as the author wrote it, not case-folded", () => {
    // `parseRootSlug` folds, because a join key must. A caption must not.
    expect(etymologyRootNode("SANSKRIT-PA-DRINK", TAGS)).toEqual({
      term: "PA DRINK",
      language: "Sanskrit",
    });
  });

  it("refuses a slug that is a bare language tag with no term", () => {
    expect(() => etymologyRootNode("latin-", TAGS)).toThrow(/no printable term/);
    expect(() => etymologyRootNode("-latin", TAGS)).toThrow(/no printable term/);
  });

  it("guards PRINTABILITY, not emptiness, so a blank caption cannot be published", () => {
    // Three drafts of this guard, each stopping one step short of where the
    // value actually goes, and each one publishing the same artifact: a box
    // with a language under it and no word in it, nothing thrown, hash ledger
    // regenerated to match.
    //
    //   `lemma === ""` let `latin--` through -- it slices to "-", then joins to ""
    //   `term === ""`  let `latin- -` through -- it joins to " "
    //
    // `latin- -` is PURE ASCII and survives the frontmatter list parser, which
    // trims only an item's outer edges. Both misses were found by security
    // review, on consecutive rounds.
    expect(() => etymologyRootNode("latin--", TAGS)).toThrow(/no printable term/);
    expect(() => etymologyRootNode("--latin", TAGS)).toThrow(/no printable term/);
    expect(() => etymologyRootNode("latin- -", TAGS)).toThrow(/no printable term/);
    expect(() => etymologyRootNode("latin-\u00a0", TAGS)).toThrow(/no printable term/);
    // U+200B is category Cf, not whitespace, so `.trim()` never touches it.
    expect(() => etymologyRootNode("latin-\u200b", TAGS)).toThrow(/no printable term/);
    expect(() => etymologyRootNode("\u200b-latin", TAGS)).toThrow(/no printable term/);
    // Control: a term that merely CONTAINS a space still renders.
    expect(etymologyRootNode("a-dios-latin", TAGS)).toEqual({ term: "a dios", language: "Latin" });
  });

  it("renders the ordered route through paint-vm-svg and hashes its canonical fields", () => {
    const parsed = lesson();
    const generated = renderEtymologyRouteFigure(parsed, TAGS);
    expect(generated.svg).toContain('<svg xmlns="http://www.w3.org/2000/svg"');
    expect(generated.svg).toContain("qahwah");
    expect(generated.svg).toContain("kahve");
    expect(generated.svg).toContain("caffè");
    expect(generated.svg).toContain("café");
    expect(generated.svg.indexOf("qahwah")).toBeLessThan(generated.svg.indexOf("kahve"));
    expect(generated.sourceHash).toMatch(/^fnv1a64:/);
    expect(generated.svgHash).toMatch(/^fnv1a64:/);
    expect(etymologyFigureSource(parsed)).not.toContain(parsed.body);
  });

  it("rejects a route that has no meaningful chain", () => {
    expect(() => renderEtymologyRouteFigure(lesson("[qahwah-arabic]"), TAGS)).toThrow(
      /at least two roots/,
    );
  });
});
