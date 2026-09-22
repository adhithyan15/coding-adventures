import { parseLesson } from "../src/parse.js";
import { describe, expect, it } from "vitest";
import {
  etymologyFigureSource,
  etymologyRootNode,
  renderEtymologyRouteFigure,
} from "../src/figure.js";

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
    expect(etymologyRootNode("qahwah-arabic")).toEqual({ term: "qahwah", language: "Arabic" });
    expect(etymologyRootNode("a-dios-latin")).toEqual({ term: "a dios", language: "Latin" });
    expect(() => etymologyRootNode("orphan")).toThrow(/language tag/);
  });

  it("finds the tag by vocabulary, not by position, so a PREFIX slug is not read backwards", () => {
    // This function used to be `pieces.pop()`, which is correct for exactly one
    // of the three slug shapes the corpus uses. HL-C419's normalisation flipped
    // `kahve-turkish` to `turkish-kahve` -- the shape the `turkish` tag's own
    // convention prefers -- and the published SVG for ES-C06-cafe went out
    // claiming Arabic qahwah became **Kahve "turkish"**. Nothing threw, because
    // the slug still had two pieces.
    expect(etymologyRootNode("turkish-kahve")).toEqual({ term: "kahve", language: "Turkish" });
    expect(etymologyRootNode("sanskrit-kaala-time")).toEqual({
      term: "kaala time",
      language: "Sanskrit",
    });
    // Positional reading was ALREADY wrong here before that pass; the
    // normalisation only made a prefix slug reachable from a figure. `pop()`
    // gave the term "proto indo european" in the language "Dwoh".
    expect(etymologyRootNode("proto-indo-european-dwoh")).toEqual({
      term: "dwoh",
      language: "Proto-indo-european",
    });
  });

  it("prints the term as the author wrote it, not case-folded", () => {
    // `parseRootSlug` folds, because a join key must. A caption must not.
    expect(etymologyRootNode("SANSKRIT-PA-DRINK")).toEqual({
      term: "PA DRINK",
      language: "Sanskrit",
    });
  });

  it("refuses a slug that is a bare language tag with no term", () => {
    expect(() => etymologyRootNode("latin-")).toThrow(/no term/);
  });

  it("renders the ordered route through paint-vm-svg and hashes its canonical fields", () => {
    const parsed = lesson();
    const generated = renderEtymologyRouteFigure(parsed);
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
    expect(() => renderEtymologyRouteFigure(lesson("[qahwah-arabic]"))).toThrow(
      /at least two roots/,
    );
  });
});
