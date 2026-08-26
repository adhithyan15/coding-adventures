// HL10 section 2 -- the strand dimension (HL-C80).
//
// Every gate gets a fixture that fires it AND a control that does not, because a
// rule asserted only in its failing direction cannot distinguish "the gate works"
// from "the gate always fires". The corpus block at the bottom pins the first
// published snapshot, including the three strands that measure ZERO -- that number
// is the finding, not an omission, so it is pinned rather than skipped.

import { describe, expect, it } from "vitest";
import {
  NODE_CONCEPT_TARGET,
  declaredStrands,
  nodeSizeDefects,
  renderStrandSummary,
  strandDefects,
  summarizeStrands,
} from "../src/strands.js";
import { loadChapterPolicy, loadCurriculumSpine } from "../src/loader.js";
import { CURRICULUM_STRANDS, type CurriculumSpine, type SpineNode } from "../src/types.js";

function node(over: Partial<SpineNode> & { id: string }): SpineNode {
  return {
    stage: "A1",
    strand: "FUNCTION",
    canDo: "I can do the thing.",
    prerequisites: [],
    core: true,
    concepts: [],
    ...over,
  } as SpineNode;
}

function spine(nodes: SpineNode[], over: Partial<CurriculumSpine> = {}): CurriculumSpine {
  return {
    version: 1,
    stages: ["pre-A1", "A1", "A2", "B1", "B2", "C1", "C2"],
    strands: [...CURRICULUM_STRANDS],
    nodes,
    ...over,
  };
}

describe("the declared strand vocabulary", () => {
  it("prefers the spine's own list, so adding a strand is a data edit", () => {
    const custom = spine([], { strands: ["FUNCTION", "GRAMMAR"] as never });
    expect(declaredStrands(custom)).toEqual(["FUNCTION", "GRAMMAR"]);
  });

  it("falls back to the built-in list for a spine written before strands existed", () => {
    const legacy = spine([], { strands: undefined });
    expect(declaredStrands(legacy)).toEqual(CURRICULUM_STRANDS);
  });

  it("treats an empty declared list as absent rather than as 'nothing is allowed'", () => {
    // An empty array would otherwise make EVERY node an unknown-strand defect, which
    // reads as a corpus catastrophe when the real fault is one empty key.
    expect(declaredStrands(spine([], { strands: [] }))).toEqual(CURRICULUM_STRANDS);
  });
});

describe("strandDefects", () => {
  it("fires on a node that declares no strand", () => {
    const bad = spine([node({ id: "SPINE-X", strand: undefined as never })]);
    const found = strandDefects(bad);
    expect(found).toHaveLength(1);
    expect(found[0]?.kind).toBe("missing-strand");
  });

  it("fires on a strand outside the declared list", () => {
    const bad = spine([node({ id: "SPINE-X", strand: "VIBES" as never })]);
    const found = strandDefects(bad);
    expect(found).toHaveLength(1);
    expect(found[0]?.kind).toBe("unknown-strand");
    expect(found[0]?.detail).toContain("VIBES");
  });

  it("reports an empty string as missing, not as unknown", () => {
    // "" is what a half-finished authoring pass leaves behind, and calling it
    // unknown would send the reader looking for a strand named "".
    const bad = spine([node({ id: "SPINE-X", strand: "" as never })]);
    expect(strandDefects(bad)[0]?.kind).toBe("missing-strand");
  });

  it("control: a well-formed node produces no defect", () => {
    expect(strandDefects(spine([node({ id: "SPINE-X", strand: "GRAMMAR" })]))).toEqual([]);
  });
});

describe("nodeSizeDefects", () => {
  const concepts = (n: number) => Array.from({ length: n }, (_, i) => `C-${i}`);

  it("says nothing about a node at the design target", () => {
    const ok = spine([node({ id: "SPINE-X", concepts: concepts(NODE_CONCEPT_TARGET) })]);
    expect(nodeSizeDefects(ok, 12)).toEqual([]);
  });

  it("warns above the target but below the ceiling", () => {
    const found = nodeSizeDefects(spine([node({ id: "SPINE-X", concepts: concepts(9) })]), 12);
    expect(found).toHaveLength(1);
    expect(found[0]?.severity).toBe("over-target");
  });

  it("escalates above the chapter ceiling, where no chapter can realize the node", () => {
    const found = nodeSizeDefects(spine([node({ id: "SPINE-X", concepts: concepts(42) })]), 12);
    expect(found[0]?.severity).toBe("over-ceiling");
    expect(found[0]?.concepts).toBe(42);
  });

  it("sorts worst first, so the node most needing a split is not found by scrolling", () => {
    const found = nodeSizeDefects(
      spine([
        node({ id: "SPINE-SMALL", concepts: concepts(8) }),
        node({ id: "SPINE-HUGE", concepts: concepts(42) }),
        node({ id: "SPINE-MID", concepts: concepts(20) }),
      ]),
      12,
    );
    expect(found.map((d) => d.nodeId)).toEqual(["SPINE-HUGE", "SPINE-MID", "SPINE-SMALL"]);
  });
});

describe("summarizeStrands", () => {
  it("reports a declared strand with no nodes rather than omitting it", () => {
    // The whole point: seeding from the node list would make an unclimbed ladder
    // invisible, which is exactly how ETYMOLOGY stayed 'the signature of this
    // curriculum' while being nobody's commitment.
    const s = summarizeStrands(spine([node({ id: "SPINE-X", strand: "FUNCTION" })]), 12);
    expect(s.strands).toHaveLength(CURRICULUM_STRANDS.length);
    expect(s.emptyStrands).toContain("IDIOM");
    expect(s.emptyStrands).not.toContain("FUNCTION");
  });

  it("does not count a node whose strand is unknown", () => {
    const s = summarizeStrands(spine([node({ id: "SPINE-X", strand: "VIBES" as never })]), 12);
    expect(s.strands.every((c) => c.nodes === 0)).toBe(true);
    expect(s.defects).toHaveLength(1);
  });

  it("tracks which stages a strand never reaches", () => {
    const s = summarizeStrands(
      spine([node({ id: "SPINE-X", strand: "GRAMMAR", stage: "A1" })]),
      12,
    );
    const grammar = s.strands.find((c) => c.strand === "GRAMMAR")!;
    expect(grammar.byStage.A1).toBe(1);
    expect(grammar.missingStages).toContain("C2");
    expect(grammar.missingStages).not.toContain("A1");
  });

  it("names the largest node, the HL09 section 1 failure signal", () => {
    const s = summarizeStrands(
      spine([
        node({ id: "SPINE-SMALL", concepts: ["a"] }),
        node({ id: "SPINE-BIG", concepts: Array.from({ length: 42 }, (_, i) => `c${i}`) }),
      ]),
      12,
    );
    expect(s.largestNode).toEqual({ nodeId: "SPINE-BIG", concepts: 42 });
  });
});

describe("renderStrandSummary", () => {
  it("surfaces empty strands in prose a reader will actually notice", () => {
    const lines = renderStrandSummary(
      summarizeStrands(spine([node({ id: "SPINE-X", strand: "FUNCTION" })]), 12),
    );
    expect(lines.join("\n")).toContain("strands with no nodes");
  });

  it("control: says nothing about empty strands when every strand is climbed", () => {
    const full = spine(CURRICULUM_STRANDS.map((strand) => node({ id: `SPINE-${strand}`, strand })));
    expect(renderStrandSummary(summarizeStrands(full, 12)).join("\n")).not.toContain(
      "strands with no nodes",
    );
  });
});

describe("hostile spine shapes (security review, HL-C80)", () => {
  // A gate whose job is making defects visible must not be silenceable by the data
  // it inspects. Each case below was found by adversarial review and confirmed to
  // misbehave before the fix.

  it("does not let a prototype-named stage forge a covered stage", () => {
    // `stage in byStage` walked the prototype chain, so "toString" passed the
    // membership test, read the inherited FUNCTION, and `+= 1` wrote
    // "function toString() { [native code] }1" into the count. That string is not
    // === 0, so missingStages reported the stage as COVERED. The gate reported
    // clean BECAUSE of the crafted name.
    const s = summarizeStrands(
      spine([node({ id: "SPINE-X", strand: "FUNCTION", stage: "toString" as never })]),
      12,
    );
    const fn = s.strands.find((c) => c.strand === "FUNCTION")!;
    for (const value of Object.values(fn.byStage)) {
      expect(typeof value).toBe("number");
    }
    // Every real stage is still correctly reported as unreached.
    expect(fn.missingStages).toContain("A1");
    expect(fn.missingStages).toContain("C2");
  });

  it("keeps buckets free of inherited keys entirely", () => {
    const s = summarizeStrands(spine([node({ id: "SPINE-X" })]), 12);
    const fn = s.strands.find((c) => c.strand === "FUNCTION")!;
    expect(Object.getPrototypeOf(fn.byStage)).toBeNull();
    expect(("toString" as string) in fn.byStage).toBe(false);
  });

  it("does not pollute Object.prototype from a __proto__ stage name", () => {
    summarizeStrands(
      spine([node({ id: "SPINE-X", stage: "__proto__" as never })]),
      12,
    );
    expect(({} as Record<string, unknown>).polluted).toBeUndefined();
    expect(Object.prototype.toString).toBe(Object.prototype.toString);
  });

  it.each([
    ["strands is an object", { strands: { length: 3 } as never }],
    ["strands is a string", { strands: "FUNCTION" as never }],
    ["stages is a string", { stages: "A1" as never }],
    ["nodes is absent", { nodes: undefined as never }],
    ["nodes holds null", { nodes: [null] as never }],
  ])("survives malformed JSON: %s", (_label, over) => {
    // These each threw an uncaught TypeError out of the CLI before the shape
    // guards, surfacing as a Node stack trace with absolute filesystem paths.
    const bad = spine([node({ id: "SPINE-X" })], over);
    expect(() => summarizeStrands(bad, 12)).not.toThrow();
    expect(() => strandDefects(bad)).not.toThrow();
    expect(() => nodeSizeDefects(bad, 12)).not.toThrow();
  });
});

describe("the committed corpus", () => {
  it("gives every spine node a declared strand", () => {
    expect(strandDefects(loadCurriculumSpine())).toEqual([]);
  });

  it("pins the first strand snapshot, including the three ladders nobody has climbed", () => {
    const summary = summarizeStrands(loadCurriculumSpine(), loadChapterPolicy().maxNewAtomsPerChapter);
    // 33 -> 34, and LEXICON 2 -> 3: HL23 adds `SPINE-NAME-EVERYDAY-ACTIONS`, an A1
    // LEXICON node holding the first six everyday-action concepts split off
    // `SPINE-SAY-WHAT-I-DO` — the first slice of HL09 §11 item 5. The pin moves
    // because the spine gained a commitment, which is exactly what it is here to
    // report; it is not loosened.
    expect(summary.totalNodes).toBe(34);

    const byStrand = Object.fromEntries(summary.strands.map((s) => [s.strand, s.nodes]));
    expect(byStrand).toEqual({
      FUNCTION: 14,
      GRAMMAR: 7,
      LEXICON: 3,
      SOUND: 0,
      ETYMOLOGY: 0,
      CULTURE: 3,
      IDIOM: 0,
      TEXT: 7,
    });

    // The measurement HL10 was written to make visible. SOUND, ETYMOLOGY and IDIOM
    // are declared ladders with nothing on them -- and ETYMOLOGY is the one HL00
    // calls "the signature of this curriculum", present in 708 lessons as authored
    // prose but promised by no node. When this list shrinks, the course has gained
    // a commitment, not just content.
    expect(summary.emptyStrands).toEqual(["SOUND", "ETYMOLOGY", "IDIOM"]);
  });

  it("still measures the node HL09 section 1 diagnosed, now six concepts lighter", () => {
    const summary = summarizeStrands(loadCurriculumSpine(), loadChapterPolicy().maxNewAtomsPerChapter);
    // 42 -> 39. HL23 moved VERB-DRINK, VERB-GIVE and VERB-PUT onto the new A1
    // `SPINE-NAME-EVERYDAY-ACTIONS`. This is the first slice of HL09 §11 item 5,
    // and a deliberately small one: a concept can only leave this node when every
    // lesson realizing it moves too, because `validateCurriculum` demotes a lesson
    // to "local support" the moment its concept's owner stops being its segment's
    // node. These three were the ones no other track had to be edited to release.
    // 39 is still 3x the chapter atom ceiling, so the node stays the corpus's
    // worst offender and stays pinned below.
    expect(summary.largestNode).toEqual({ nodeId: "SPINE-SAY-WHAT-I-DO", concepts: 39 });

    // Recorded debt, report-only: HL-C81 splits these. Until then the count is
    // pinned so it cannot grow quietly.
    const overCeiling = summary.nodeSizeDefects.filter((d) => d.severity === "over-ceiling");
    expect(overCeiling.map((d) => d.nodeId)).toEqual(["SPINE-SAY-WHAT-I-DO"]);
  });
});

describe("the committed policy", () => {
  it("carries the HL10 section 2.2 budgets", () => {
    const policy = loadChapterPolicy();
    expect(policy.maxNewGrammarCellsPerLesson).toBe(1);
    expect(policy.maxNewIdiomsPerLesson).toBe(1);
    expect(policy.maxNewSensesPerLesson).toBe(1);
    expect(policy.maxNewCultureClaimsPerLesson).toBe(2);
    expect(policy.maxRuleStatementsPerLesson).toBe(1);
    expect(policy.minDownstreamReach).toBe(1);
    expect(policy.rootLedgerMinReuse).toBe(3);
  });

  it("leaves the pre-existing HL08 budgets untouched", () => {
    // The strand budgets are additive. If one of these moves, something reformatted
    // the policy rather than extending it.
    const policy = loadChapterPolicy();
    expect(policy.maxNewAtomsPerLesson).toBe(3);
    expect(policy.maxNewAtomsPerChapter).toBe(12);
    expect(policy.payoffRepresentativeness).toBe(0.5);
    expect(policy.maxNewGlyphsPerLesson).toBe(3);
  });
});
