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
