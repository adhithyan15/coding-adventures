// HL10 section 5 -- the grammar cell inventory (HL-C82).
//
// Every gate gets a firing fixture AND a control. The corpus block pins the
// generated inventory and the pedagogical claims its prerequisite edges encode,
// because those edges are the actual curriculum design -- if one silently
// changes, the ramp changes.

import { describe, expect, it } from "vitest";
import {
  cellCoverage,
  cellGraphDefects,
  topologicalOrder,
} from "../src/grammar-cells.js";
import { loadGrammarSlots, loadTrackGrammarCells } from "../src/loader.js";
import { parseLesson } from "../src/parse.js";
import type { GrammarCell, GrammarSlotInventory, TrackGrammarCells } from "../src/types.js";

const SLOTS: GrammarSlotInventory = {
  version: 1,
  slots: [
    { id: "SLOT-A", kind: "finite", gloss: "a" },
    { id: "SLOT-B", kind: "finite", gloss: "b" },
    { id: "SLOT-C", kind: "finite", gloss: "c" },
  ],
};

function track(cells: GrammarCell[]): TrackGrammarCells {
  return { version: 1, language: "test", cells };
}

function cell(id: string, prerequisites: string[] = [], slot = "SLOT-A"): GrammarCell {
  return { id, slot, prerequisites };
}

function lesson(id: string, sequence: number, cells: string[]) {
  const teaches = cells.length > 0 ? `teaches_cells: [${cells.join(", ")}]\n` : "";
  return parseLesson(
    `---
schema_version: 2
id: ${id}
sequence: ${sequence}
chapter: 1
type: vocabulary
headword: x
gloss: x
${teaches}---

# x
`,
    "spanish",
  );
}

describe("topologicalOrder", () => {
  it("never places a cell before one it requires", () => {
    const cells = [cell("C", ["B"]), cell("A"), cell("B", ["A"])];
    const order = topologicalOrder(cells);
    expect(order.indexOf("A")).toBeLessThan(order.indexOf("B"));
    expect(order.indexOf("B")).toBeLessThan(order.indexOf("C"));
  });

  it("terminates on a cycle instead of looping forever", () => {
    // The graph is broken, and cellGraphDefects is what reports that. This
    // function's only job here is to come back.
    const cells = [cell("A", ["B"]), cell("B", ["A"])];
    expect(() => topologicalOrder(cells)).not.toThrow();
  });

  it("terminates on a dangling edge", () => {
    expect(() => topologicalOrder([cell("A", ["GHOST"])])).not.toThrow();
  });
});

describe("cellGraphDefects", () => {
  it("fires on a prerequisite no cell declares", () => {
    const found = cellGraphDefects(track([cell("A", ["GHOST"])]), SLOTS);
    expect(found.map((d) => d.kind)).toContain("dangling-prerequisite");
  });

  it("fires on a cell filling a slot the universal inventory does not declare", () => {
    const found = cellGraphDefects(track([cell("A", [], "SLOT-NOPE")]), SLOTS);
    expect(found.map((d) => d.kind)).toContain("unknown-slot");
  });

  it("fires on a duplicate id", () => {
    const found = cellGraphDefects(track([cell("A"), cell("A")]), SLOTS);
    expect(found.map((d) => d.kind)).toContain("duplicate-id");
  });

  it("fires on a cycle, which no ordering can reach", () => {
    const found = cellGraphDefects(track([cell("A", ["B"]), cell("B", ["A"])]), SLOTS);
    expect(found.map((d) => d.kind)).toContain("cycle");
  });

  it("control: a well-formed chain produces no defect", () => {
    const clean = track([cell("A"), cell("B", ["A"], "SLOT-B"), cell("C", ["B"], "SLOT-C")]);
    expect(cellGraphDefects(clean, SLOTS)).toEqual([]);
  });
});

describe("cellCoverage", () => {
  const chain = track([cell("A"), cell("B", ["A"], "SLOT-B"), cell("C", ["B"], "SLOT-C")]);

  it("counts nothing when no lesson declares a cell", () => {
    const c = cellCoverage(chain, [lesson("L1", 10, [])], 1);
    expect(c.taught).toEqual([]);
    expect(c.taughtPercent).toBe(0);
    expect(c.untaught).toHaveLength(3);
  });

  it("counts a declared cell and lists the rest in dependency order", () => {
    const c = cellCoverage(chain, [lesson("L1", 10, ["A"])], 1);
    expect(c.taught).toEqual(["A"]);
    expect(c.untaught).toEqual(["B", "C"]);
    expect(c.taughtPercent).toBe(33);
  });

  it("fires the budget when one lesson teaches two cells", () => {
    // The rule the whole model exists for: one cell per lesson, so the six-form
    // table can never arrive at once.
    const c = cellCoverage(chain, [lesson("L1", 10, ["A", "B"])], 1);
    expect(c.overBudget).toEqual([{ lessonId: "L1", cells: 2, budget: 1 }]);
  });

  it("control: one cell per lesson is within budget", () => {
    const c = cellCoverage(chain, [lesson("L1", 10, ["A"]), lesson("L2", 20, ["B"])], 1);
    expect(c.overBudget).toEqual([]);
  });

  it("fires when a cell is taught before its prerequisite", () => {
    const c = cellCoverage(chain, [lesson("L1", 10, ["B"]), lesson("L2", 20, ["A"])], 1);
    expect(c.outOfOrder).toEqual([
      { lessonId: "L1", cellId: "B", missingPrerequisite: "A" },
    ]);
  });

  it("reads reading order from `sequence`, not from array position", () => {
    // The lessons are passed in the wrong order deliberately: B's file comes
    // first but sequence says it is second, so this must NOT fire.
    const c = cellCoverage(chain, [lesson("L2", 20, ["B"]), lesson("L1", 10, ["A"])], 1);
    expect(c.outOfOrder).toEqual([]);
  });

  it("treats a prerequisite taught by the SAME lesson as out of order", () => {
    // Otherwise a two-cell lesson would launder itself past the ramp check by
    // teaching a cell and its prerequisite together. The budget catches the
    // count; this catches the ordering.
    const c = cellCoverage(chain, [lesson("L1", 10, ["A", "B"])], 1);
    expect(c.outOfOrder).toEqual([
      { lessonId: "L1", cellId: "B", missingPrerequisite: "A" },
    ]);
  });

  it("reports a declaration naming a cell that does not exist", () => {
    const c = cellCoverage(chain, [lesson("L1", 10, ["GHOST"])], 1);
    expect(c.unknownDeclarations).toEqual([{ lessonId: "L1", cellId: "GHOST" }]);
    expect(c.taught).toEqual([]);
  });

  it("coerces a string `sequence`, which is how the parser actually emits it", () => {
    // Regression. The frontmatter parser yields sequence as a STRING, so an
    // earlier `typeof raw === "number"` test sent every lesson to Infinity, the
    // sort became a no-op, and the ordering check graded file order instead of
    // reading order. It passed on every fixture that happened to be pre-sorted.
    const [first] = [lesson("L1", 10, ["A"])];
    expect(typeof (first!.frontmatter as Record<string, unknown>).sequence).toBe("string");

    // Out of order in the ARRAY, in order by sequence: must not fire.
    expect(cellCoverage(chain, [lesson("L2", 20, ["B"]), lesson("L1", 10, ["A"])], 1).outOfOrder)
      .toEqual([]);
    // In order in the array, OUT of order by sequence: must fire.
    expect(cellCoverage(chain, [lesson("L2", 5, ["B"]), lesson("L1", 90, ["A"])], 1).outOfOrder)
      .toEqual([{ lessonId: "L2", cellId: "B", missingPrerequisite: "A" }]);
  });

  it("skips the budget check when no budget is configured", () => {
    const c = cellCoverage(chain, [lesson("L1", 10, ["A", "B"])], undefined);
    expect(c.overBudget).toEqual([]);
  });
});

describe("hostile cell graphs (security review, HL-C82)", () => {
  it("does not hang when `prerequisites` is a length-carrying object", () => {
    // The finding. The walk is driven purely by .length, so this 77-byte
    // document used to pin a core at 99% indefinitely: every index is undefined,
    // so the loop continued without advancing the stack or popping. `?? []`
    // defends against null/undefined only. If this test ever regresses it HANGS
    // rather than failing, which is why the payload is small enough to reason
    // about and the guard is at the boundary every caller shares.
    const evil = {
      version: 1,
      language: "evil",
      cells: [{ id: "A", slot: "SLOT-A", prerequisites: { length: 1e15 } }],
    } as unknown as TrackGrammarCells;
    expect(topologicalOrder(evil.cells)).toEqual(["A"]);
    expect(() => cellGraphDefects(evil, SLOTS)).not.toThrow();
    expect(() => cellCoverage(evil, [], 1)).not.toThrow();
  });

  it.each([
    ["prerequisites is a string", { id: "A", slot: "SLOT-A", prerequisites: "B" }],
    ["prerequisites is a number", { id: "A", slot: "SLOT-A", prerequisites: 7 }],
    ["prerequisites is absent", { id: "A", slot: "SLOT-A" }],
  ])("survives malformed prerequisites: %s", (_label, cellLike) => {
    const t = { version: 1, language: "x", cells: [cellLike] } as unknown as TrackGrammarCells;
    expect(() => topologicalOrder(t.cells)).not.toThrow();
    expect(() => cellGraphDefects(t, SLOTS)).not.toThrow();
    expect(() => cellCoverage(t, [], 1)).not.toThrow();
  });

  it("survives a null element in cells, which used to throw on .map", () => {
    const t = { version: 1, language: "x", cells: [null, cell("A")] } as unknown as TrackGrammarCells;
    expect(() => cellGraphDefects(t, SLOTS)).not.toThrow();
    expect(topologicalOrder(t.cells)).toEqual(["A"]);
  });

  it("does not pollute Object.prototype from __proto__ cell ids", () => {
    const t = track([cell("__proto__"), cell("constructor", ["__proto__"])]);
    cellGraphDefects(t, SLOTS);
    cellCoverage(t, [], 1);
    expect(({} as Record<string, unknown>).polluted).toBeUndefined();
  });

  it("orders unsequenced lessons consistently rather than by a NaN comparator", () => {
    // Infinity - Infinity is NaN. An inconsistent comparator leaves the relative
    // order of unsequenced lessons arbitrary, in the one module whose job is
    // checking reading order.
    const chain2 = track([cell("A"), cell("B", ["A"], "SLOT-B")]);
    const noSeq = (id: string, cells: string[]) =>
      parseLesson(
        `---\nschema_version: 2\nid: ${id}\nchapter: 1\ntype: vocabulary\nheadword: x\ngloss: x\nteaches_cells: [${cells.join(", ")}]\n---\n\n# x\n`,
        "spanish",
      );
    expect(() => cellCoverage(chain2, [noSeq("L1", ["A"]), noSeq("L2", ["B"])], 1)).not.toThrow();
  });
});

describe("the committed inventory", () => {
  const slots = loadGrammarSlots();
  const spanish = loadTrackGrammarCells("spanish");

  it("has no structural defects", () => {
    expect(cellGraphDefects(spanish, slots)).toEqual([]);
  });

  it("pins the regular-cell arithmetic from HL10 section 5.1", () => {
    // 6 persons x 5 indicative tenses x 3 conjugations   =  90
    // 6 persons x 3 subjunctive tenses x 3 conjugations  =  54
    // 5 persons x 2 polarities x 3 conjugations          =  30
    // 6 persons x 8 compound tenses                      =  48
    // 3 non-finite forms x 3 conjugations                =   9
    expect(slots.counts).toMatchObject({
      finite: 144,
      imperative: 30,
      compound: 48,
      "non-finite": 9,
      total: 231,
    });
    expect(spanish.cells).toHaveLength(231);
  });

  it("keeps the universal inventory free of Spanish", () => {
    // HL10 section 4.4: the one property that cannot be retrofitted. A spine or
    // slot inventory with Spanish baked in is a Spanish syllabus, and the other
    // 21 tracks would have to start over.
    const spanishForms = /\b(hablo|hablar|comer|vivir|usted|vosotros|-ar|-er|-ir)\b/i;
    for (const slot of slots.slots) {
      expect(slot.id).not.toMatch(spanishForms);
      expect(slot.gloss).not.toMatch(spanishForms);
    }
  });

  it("starts the learner on the infinitives and the first present cell", () => {
    const roots = spanish.cells.filter((c) => c.prerequisites.length === 0).map((c) => c.id);
    expect(roots.sort()).toEqual([
      "ES-CELL-IND-PRES-1SG-CONJ1",
      "ES-CELL-INF-CONJ1",
      "ES-CELL-INF-CONJ2",
      "ES-CELL-INF-CONJ3",
    ]);
  });

  it("hangs the present subjunctive off the present indicative 1SG", () => {
    // Not decorative: the subjunctive stem IS the yo form (tengo -> tenga), and
    // this edge is why HL10 section 5.4 puts the -go verbs before the
    // subjunctive arc. If this edge moves, that ordering loses its reason.
    const sbj = spanish.cells.find((c) => c.id === "ES-CELL-SBJ-PRES-1SG-CONJ1")!;
    expect(sbj.prerequisites).toEqual(["ES-CELL-IND-PRES-1SG-CONJ1"]);
  });

  it("requires both the affirmative command and the subjunctive for a negative command", () => {
    const neg = spanish.cells.find((c) => c.id === "ES-CELL-IMP-NEG-2SG-CONJ1")!;
    expect(neg.prerequisites).toEqual([
      "ES-CELL-IMP-AFF-2SG-CONJ1",
      "ES-CELL-SBJ-PRES-2SG-CONJ1",
    ]);
  });

  it("requires the participle and the auxiliary's own cell for a compound", () => {
    // A learner cannot say "I have spoken" before they can say "I have".
    const perf = spanish.cells.find((c) => c.id === "ES-CELL-PERF-PRES-1SG")!;
    expect(perf.prerequisites).toEqual([
      "ES-CELL-PART-CONJ1",
      "ES-CELL-IND-PRES-1SG-CONJ2",
    ]);
  });

  it("walks persons one at a time rather than a row at a time", () => {
    // The rule that turns "the present tense" from one chapter into fourteen.
    const chain = ["1SG", "2SG", "3SG", "1PL", "2PL", "3PL"];
    for (let i = 1; i < chain.length; i += 1) {
      const c = spanish.cells.find((x) => x.id === `ES-CELL-IND-PRES-${chain[i]}-CONJ1`)!;
      expect(c.prerequisites).toContain(`ES-CELL-IND-PRES-${chain[i - 1]}-CONJ1`);
    }
  });

  it("marks the future subjunctive receptive-only rather than pretending it is dead", () => {
    const receptive = spanish.cells.filter((c) => c.productive === false);
    expect(receptive).toHaveLength(18); // 6 persons x 3 conjugations
    expect(receptive.every((c) => c.id.startsWith("ES-CELL-SBJ-FUT-"))).toBe(true);
    expect(receptive[0]?.receptiveOnlyBecause).toContain("legal");
  });

  it("pins the irregular overlays, and keeps the regular inventory untouched", () => {
    // HL10 section 5.1 sized the Spanish verb system at ~630 cells. 231 regular
    // plus 402 overlays is 633, so the original estimate holds.
    const overlays = spanish.overlays ?? [];
    expect(spanish.cells).toHaveLength(231);
    expect(overlays).toHaveLength(402);

    const byKind = overlays.reduce<Record<string, number>>((acc, o) => {
      acc[o.kind] = (acc[o.kind] ?? 0) + 1;
      return acc;
    }, {});
    expect(byKind["strong-preterite"]).toBe(90);
    expect(byKind["short-stem"]).toBe(144);
    expect(byKind["go-club"]).toBe(10);
    expect(byKind["irregular-imperfect"]).toBe(18);
  });

  it("hangs every overlay off a regular cell that exists", () => {
    const ids = new Set(spanish.cells.map((c) => c.id));
    for (const o of spanish.overlays ?? []) {
      expect(ids.has(o.deviatesFrom)).toBe(true);
      expect(o.prerequisites).toEqual([o.deviatesFrom]);
    }
  });

  it("keeps the plural persons out of a stem change, which is the boot", () => {
    // Singular plus third plural change; the two plural persons keep the regular
    // stem. If this ever covers six persons, the pattern has been flattened into
    // "poder is irregular", which is the thing the cell model exists to prevent.
    const poder = (spanish.overlays ?? []).filter(
      (o) => o.verb === "poder" && o.kind.startsWith("stem-change"),
    );
    expect(poder).toHaveLength(4);
    const persons = poder.map((o) => o.deviatesFrom.split("-")[4]).sort();
    expect(persons).toEqual(["1SG", "2SG", "3PL", "3SG"]);
  });

  it("gives a short-stem verb both the future and the conditional", () => {
    // One weld, twice (HL10 section 5.4 rung 22): a single shortened stem serves
    // both tenses, so the verb owns twelve cells but one thing to learn.
    const tener = (spanish.overlays ?? []).filter(
      (o) => o.verb === "tener" && o.kind === "short-stem",
    );
    expect(tener).toHaveLength(12);
    expect(new Set(tener.map((o) => o.deviatesFrom.split("-")[3]))).toEqual(
      new Set(["FUT", "COND"]),
    );
  });

  it("has unique overlay ids", () => {
    const overlays = spanish.overlays ?? [];
    expect(new Set(overlays.map((o) => o.id)).size).toBe(overlays.length);
  });

  it("measures the corpus at zero taught cells, which is the honest number", () => {
    // No lesson declares `teaches_cells` yet. Deliberately not inferred from
    // atom names: ES-GRAMMAR-AR-FUTURE-SINGULAR looks like three cells, but
    // calling it three would credit the corpus for teaching three at once --
    // exactly the info dump this model forbids. HL-C84 wires the declarations.
    const c = cellCoverage(spanish, [], 1);
    expect(c.taught).toEqual([]);
    expect(c.taughtPercent).toBe(0);
    expect(c.untaught).toHaveLength(231);
  });
});
