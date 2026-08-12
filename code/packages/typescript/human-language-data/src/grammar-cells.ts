/**
 * HL10 section 5 -- the grammar ramp, measured in cells.
 *
 * A CELL IS NOT A PARADIGM
 *
 * A cell is one filled slot in one paradigm: `hablo` is a cell. The six-form
 * present-indicative table is not a teachable unit; it is six.
 *
 * Every language textbook opens a tense with its full grid, and that grid is
 * the steepest single step in language pedagogy -- six new forms, one new
 * concept, no retrieval, and an implicit claim that the learner absorbs them by
 * staring. HL10 forbids it twice over: `maxNewGrammarCellsPerLesson: 1`, and no
 * paradigm table may appear until every cell in it has been taught
 * individually, at which point the table is a recap rather than an
 * introduction.
 *
 * A rule like that is only enforceable if the cells are enumerated. They are,
 * in `core/grammar-slots.json` (universal, language-neutral) and
 * `spanish/grammar-cells.json` (Spanish's filling, with the ordering).
 *
 * WHAT THIS MODULE MEASURES, AND WHY IT STARTS AT ZERO
 *
 * `cellCoverage` reports how many cells the corpus actually teaches, via an
 * optional `teaches_cells:` frontmatter list. Today that answer is zero of 231,
 * because no lesson declares one yet.
 *
 * That zero is deliberate and is the honest number. The alternative was to
 * guess a mapping from existing atom names -- `ES-GRAMMAR-AR-FUTURE-SINGULAR`
 * looks like it covers three cells -- but "singular" is three cells only if you
 * assume the lesson taught all three, and the entire point of this model is
 * that teaching three cells at once is the thing being forbidden. A fuzzy
 * mapping would have reported partial coverage the corpus has not earned and
 * quietly legitimised exactly the info dump the model exists to prevent.
 *
 * So coverage starts at zero and becomes a burn-down. See HL-C84, which wires
 * the declarations onto the existing lessons.
 */

import type { ParsedLesson } from "./parse.js";
import type { GrammarCell, GrammarSlotInventory, TrackGrammarCells } from "./types.js";
import { stripControlCharacters as clean } from "./constants.js";

export interface CellGraphDefect {
  cellId: string;
  kind: "dangling-prerequisite" | "unknown-slot" | "duplicate-id" | "cycle";
  detail: string;
}

export interface CellCoverage {
  /** Cells some lesson declares it teaches. */
  taught: string[];
  /** Cells nothing teaches yet -- the authoring list, in dependency order. */
  untaught: string[];
  /** Declarations naming a cell that does not exist. */
  unknownDeclarations: { lessonId: string; cellId: string }[];
  /** Lessons declaring more cells than the budget allows. */
  overBudget: { lessonId: string; cells: number; budget: number }[];
  /**
   * Lessons teaching a cell before one of its prerequisites is taught anywhere
   * earlier in reading order. This is the ramp check -- the DAG says what must
   * come first, `sequence` says what actually does.
   */
  outOfOrder: { lessonId: string; cellId: string; missingPrerequisite: string }[];
  taughtPercent: number;
}

/**
 * Drop anything that is not a usable cell.
 *
 * `cells.map((c) => [c.id, c])` throws on a null array element, so one `null` in
 * the JSON turned every function here into an uncaught TypeError. Filtering once
 * at the boundary makes the module total over malformed input rather than merely
 * non-hanging.
 */
function usableCells(cells: unknown): GrammarCell[] {
  if (!Array.isArray(cells)) return [];
  return cells.filter(
    (c): c is GrammarCell => c !== null && typeof c === "object" && typeof (c as GrammarCell).id === "string",
  );
}

/** Cells in an order that always satisfies prerequisites first. */
export function topologicalOrder(input: GrammarCell[]): string[] {
  const cells = usableCells(input);
  const byId = new Map(cells.map((c) => [c.id, c]));
  const out: string[] = [];
  const state = new Map<string, "visiting" | "done">();

  // Iterative DFS: 231 cells is shallow today (max depth 15) but a recursive
  // walk over a future irregular overlay of ~400 more is not worth the risk.
  const visit = (start: string): void => {
    const stack: { id: string; index: number }[] = [{ id: start, index: 0 }];
    while (stack.length > 0) {
      const frame = stack[stack.length - 1]!;
      if (state.get(frame.id) === "done") {
        stack.pop();
        continue;
      }
      state.set(frame.id, "visiting");
      // Array.isArray, not `?? []`. The loop is driven purely by `.length`, so a
      // `prerequisites` of {"length": 1e15} -- 77 bytes of JSON -- spins here
      // forever: every index is undefined, so it continues without advancing the
      // stack or popping. `?? []` only defends against null and undefined.
      const rawPrereqs = byId.get(frame.id)?.prerequisites;
      const prereqs = Array.isArray(rawPrereqs) ? rawPrereqs : [];
      if (frame.index < prereqs.length) {
        const next = prereqs[frame.index]!;
        frame.index += 1;
        // A cycle or a dangling edge is reported by cellGraphDefects; here we
        // simply refuse to loop forever.
        if (state.get(next) === "visiting" || !byId.has(next)) continue;
        if (state.get(next) !== "done") stack.push({ id: next, index: 0 });
        continue;
      }
      state.set(frame.id, "done");
      out.push(frame.id);
      stack.pop();
    }
  };

  for (const cell of cells) if (state.get(cell.id) !== "done") visit(cell.id);
  return out;
}

/** Structural defects in the committed cell graph. */
export function cellGraphDefects(
  track: TrackGrammarCells,
  slots: GrammarSlotInventory,
): CellGraphDefect[] {
  const out: CellGraphDefect[] = [];
  const cells = usableCells(track.cells);
  const ids = new Set<string>();
  const slotIds = new Set((Array.isArray(slots.slots) ? slots.slots : []).map((s) => s.id));

  for (const cell of cells) {
    if (ids.has(cell.id)) {
      out.push({ cellId: cell.id, kind: "duplicate-id", detail: "declared more than once" });
    }
    ids.add(cell.id);
    if (cell.slot !== undefined && !slotIds.has(cell.slot)) {
      out.push({
        cellId: cell.id,
        kind: "unknown-slot",
        detail: `fills ${cell.slot}, which the universal inventory does not declare`,
      });
    }
  }
  for (const cell of cells) {
    for (const prereq of Array.isArray(cell.prerequisites) ? cell.prerequisites : []) {
      if (!ids.has(prereq)) {
        out.push({
          cellId: cell.id,
          kind: "dangling-prerequisite",
          detail: `requires ${prereq}, which no cell declares`,
        });
      }
    }
  }

  // A cycle means some cell can never be reached in any order -- a curriculum
  // that cannot be taught at all, not merely one taught badly.
  const ordered = new Set(topologicalOrder(cells));
  const reachable = new Set<string>();
  const byId = new Map(cells.map((c) => [c.id, c]));
  for (const id of topologicalOrder(cells)) {
    const rawPrereqs = byId.get(id)?.prerequisites;
    const prereqs = Array.isArray(rawPrereqs) ? rawPrereqs : [];
    if (prereqs.every((p) => reachable.has(p) || !ids.has(p))) reachable.add(id);
  }
  for (const cell of cells) {
    if (!ordered.has(cell.id) || !reachable.has(cell.id)) {
      out.push({
        cellId: cell.id,
        kind: "cycle",
        detail: "sits in or behind a prerequisite cycle, so no ordering can reach it",
      });
    }
  }
  return out;
}

function lessonId(lesson: ParsedLesson): string {
  const raw = (lesson.frontmatter as Record<string, unknown>).id;
  return typeof raw === "string" ? raw : "<unidentified lesson>";
}

function declaredCells(lesson: ParsedLesson): string[] {
  const raw = (lesson.frontmatter as Record<string, unknown>).teaches_cells;
  if (!Array.isArray(raw)) return [];
  return raw.filter((v): v is string => typeof v === "string" && v.length > 0);
}

/**
 * Reading order.
 *
 * `sequence` arrives from the frontmatter parser as a STRING, not a number --
 * an earlier draft of this module tested `typeof raw === "number"`, so every
 * lesson fell through to Infinity, the sort became a no-op, and the ordering
 * check silently graded the corpus in file order instead of reading order. It
 * would have passed on any fixture that happened to already be sorted. This
 * mirrors `declaredSequence` in continuity.ts, which is the module that got it
 * right first.
 */
function sequenceOf(lesson: ParsedLesson): number {
  const raw = (lesson.frontmatter as Record<string, unknown>).sequence;
  if (raw === undefined || raw === null || String(raw).trim() === "") {
    return Number.POSITIVE_INFINITY;
  }
  const value = typeof raw === "number" ? raw : Number(raw);
  return Number.isFinite(value) ? value : Number.POSITIVE_INFINITY;
}

/** How much of the cell inventory the corpus actually teaches, and in what order. */
export function cellCoverage(
  track: TrackGrammarCells,
  lessons: ParsedLesson[],
  maxNewGrammarCellsPerLesson: number | undefined,
): CellCoverage {
  const cells = usableCells(track.cells);
  const ids = new Set(cells.map((c) => c.id));
  const byId = new Map(cells.map((c) => [c.id, c]));

  // Compare, do not subtract. Two unsequenced lessons both yield Infinity, and
  // Infinity - Infinity is NaN -- an inconsistent comparator, which leaves their
  // relative order arbitrary in a module whose whole job is checking order.
  const ordered = [...lessons].sort((a, b) => {
    const left = sequenceOf(a);
    const right = sequenceOf(b);
    if (left < right) return -1;
    if (left > right) return 1;
    return 0;
  });
  const taughtAt = new Map<string, number>();
  const unknownDeclarations: { lessonId: string; cellId: string }[] = [];
  const overBudget: { lessonId: string; cells: number; budget: number }[] = [];
  const outOfOrder: { lessonId: string; cellId: string; missingPrerequisite: string }[] = [];

  ordered.forEach((lesson, index) => {
    const declared = declaredCells(lesson);
    if (declared.length === 0) return;
    if (maxNewGrammarCellsPerLesson !== undefined && declared.length > maxNewGrammarCellsPerLesson) {
      overBudget.push({ lessonId: lessonId(lesson), cells: declared.length, budget: maxNewGrammarCellsPerLesson });
    }
    for (const cellId of declared) {
      if (!ids.has(cellId)) {
        unknownDeclarations.push({ lessonId: lessonId(lesson), cellId });
        continue;
      }
      const cellPrereqs = byId.get(cellId)?.prerequisites;
      for (const prereq of Array.isArray(cellPrereqs) ? cellPrereqs : []) {
        // Strictly earlier: a prerequisite taught by the SAME lesson is still a
        // two-cell lesson, which the budget above is what catches.
        const at = taughtAt.get(prereq);
        if (at === undefined || at >= index) {
          outOfOrder.push({ lessonId: lessonId(lesson), cellId, missingPrerequisite: prereq });
        }
      }
      if (!taughtAt.has(cellId)) taughtAt.set(cellId, index);
    }
  });

  const taught = [...taughtAt.keys()];
  const taughtSet = new Set(taught);
  const untaught = topologicalOrder(cells).filter((id) => !taughtSet.has(id));

  return {
    taught,
    untaught,
    unknownDeclarations,
    overBudget,
    outOfOrder,
    taughtPercent: cells.length === 0 ? 0 : Math.round((taught.length / cells.length) * 100),
  };
}

/** Human-readable lines for the gap report. */
export function renderCellCoverage(
  track: TrackGrammarCells,
  slots: GrammarSlotInventory,
  coverage: CellCoverage,
): string[] {
  const total = (Array.isArray(track.cells) ? track.cells : []).length;
  const slotTotal = (Array.isArray(slots.slots) ? slots.slots : []).length;
  const lines = [
    `grammar cells (${clean(track.language)}): ${coverage.taught.length} of ${total} regular cells taught ` +
      `(${coverage.taughtPercent}%), against ${slotTotal} universal slots`,
  ];
  if (coverage.untaught.length > 0) {
    lines.push(
      `  next unstarted cells in dependency order: ${coverage.untaught.slice(0, 3).map(clean).join(", ")}`,
    );
  }
  for (const item of coverage.overBudget) {
    lines.push(`  ${clean(item.lessonId)}: teaches ${item.cells} cells, budget is ${item.budget}`);
  }
  for (const item of coverage.outOfOrder.slice(0, 5)) {
    lines.push(`  ${clean(item.lessonId)}: teaches ${clean(item.cellId)} before ${clean(item.missingPrerequisite)}`);
  }
  for (const item of coverage.unknownDeclarations.slice(0, 5)) {
    lines.push(`  ${clean(item.lessonId)}: declares unknown cell ${clean(item.cellId)}`);
  }
  return lines;
}
