/**
 * HL10 section 2 -- the eight strands, measured.
 *
 * WHY THIS MODULE EXISTS
 *
 * HL09 proved that a course can be gentle on one ramp and brutal on another,
 * and that nobody notices, because only the gentle ramp was being counted.
 * Spanish measured 178 headwords with every lesson inside the atom budget --
 * a textbook-perfect vocabulary ramp -- while the learner still could not say
 * "no", could not say "I am", and met the entire past tense behind a single
 * spine node declaring one concept.
 *
 * The fix is not a bigger budget. It is more ladders. A strand is a
 * commitment that some dimension of the language keeps advancing, and the
 * point of writing it into the data is that a commitment can be checked while
 * an intention cannot.
 *
 * WHAT A ZERO MEANS
 *
 * `summarizeStrands` reports strands with no nodes as a first-class result
 * rather than an empty row to be skimmed past. At the time this module was
 * written three of the eight -- SOUND, ETYMOLOGY and IDIOM -- had zero nodes,
 * in a curriculum whose own specification calls etymology "the signature of
 * this curriculum". That is the gap this measurement exists to keep visible:
 * the etymology is genuinely there, in 708 lessons, but it is there as prose
 * an author chose to write rather than as a ladder anyone promised to climb.
 *
 * REPORT-ONLY, DELIBERATELY
 *
 * Every function here returns findings. None throws, and none fails a build.
 * The corpus predates the model, and per the HL05 precedent a gate that fails
 * on already-recorded debt teaches authors to route around it rather than to
 * pay it down.
 */

import {
  CURRICULUM_STRANDS,
  type CurriculumSpine,
  type CurriculumStage,
  type CurriculumStrand,
  type SpineNode,
} from "./types.js";
import { stripControlCharacters as clean } from "./constants.js";

/** One node that cannot be placed on a ladder. */
export interface StrandDefect {
  nodeId: string;
  kind: "missing-strand" | "unknown-strand";
  detail: string;
}

/** One node carrying more concepts than a chapter is allowed to introduce. */
export interface NodeSizeDefect {
  nodeId: string;
  stage: CurriculumStage;
  strand: CurriculumStrand | null;
  concepts: number;
  /** The design target from HL10 section 3.2. Exceeding it is a warning. */
  target: number;
  /** `maxNewAtomsPerChapter`. Exceeding it means no chapter can realize the node. */
  ceiling: number;
  severity: "over-target" | "over-ceiling";
}

export interface StrandCount {
  strand: CurriculumStrand;
  nodes: number;
  /** Node counts per stage, so a strand that stops advancing is visible. */
  byStage: Record<string, number>;
  /** Stages this strand never reaches. */
  missingStages: CurriculumStage[];
}

export interface StrandSummary {
  strands: StrandCount[];
  /** Strands with no nodes at all -- an aspiration, not a commitment. */
  emptyStrands: CurriculumStrand[];
  defects: StrandDefect[];
  nodeSizeDefects: NodeSizeDefect[];
  totalNodes: number;
  /** Largest concept count on any single node, the HL09 section 1 failure signal. */
  largestNode: { nodeId: string; concepts: number } | null;
}

/**
 * HL10 section 3.2: a node is realized by one to three chapters, so it may not
 * declare more concepts than a chapter may introduce. Six is the design target;
 * `maxNewAtomsPerChapter` (12) is the hard ceiling.
 */
export const NODE_CONCEPT_TARGET = 6;

/**
 * Which strand vocabulary to validate against.
 *
 * The spine file may declare its own `strands` list. That wins, so adding a
 * strand is a data edit rather than a code change -- the same rule HL01 uses
 * for scripts. `CURRICULUM_STRANDS` is the fallback for a spine written before
 * strands existed.
 */
export function declaredStrands(spine: CurriculumSpine): readonly string[] {
  // Array.isArray, not truthiness: a spine.json whose `strands` is an object or a
  // bare string is malformed data, and without this guard it reaches a `for...of`
  // or a `.map` and throws an uncaught TypeError out of the CLI. `loadChapterPolicy`
  // already shape-validates its budgets; this is the same contract.
  if (!Array.isArray(spine.strands) || spine.strands.length === 0) return CURRICULUM_STRANDS;
  return spine.strands;
}

/** Every node must name exactly one strand, and it must be one that was declared. */
export function strandDefects(spine: CurriculumSpine): StrandDefect[] {
  const allowed = new Set(declaredStrands(spine));
  const out: StrandDefect[] = [];
  for (const node of Array.isArray(spine.nodes) ? spine.nodes : []) {
    if (node === null || typeof node !== "object") continue;
    const strand = (node as SpineNode).strand as string | undefined;
    if (strand === undefined || strand === null || strand === "") {
      out.push({
        nodeId: node.id,
        kind: "missing-strand",
        detail: "declares no strand, so it sits on no ladder and cannot be ordered",
      });
      continue;
    }
    if (!allowed.has(strand)) {
      out.push({
        nodeId: node.id,
        kind: "unknown-strand",
        detail: `declares ${strand}, which is not in the spine's declared strand list`,
      });
    }
  }
  return out;
}

/**
 * Nodes too large to be realized gently.
 *
 * This is the HL09 section 1 defect made checkable. `SPINE-SAY-WHAT-I-DO`
 * declared 42 concepts while `SPINE-TALK-ABOUT-PAST` declared one and stood
 * for the entire past tense of the language. Both cannot be one rung of the
 * same ladder, and the asymmetry is what let a track claim A2 on fourteen
 * present-tense lessons.
 */
export function nodeSizeDefects(
  spine: CurriculumSpine,
  maxNewAtomsPerChapter: number,
): NodeSizeDefect[] {
  const out: NodeSizeDefect[] = [];
  for (const node of Array.isArray(spine.nodes) ? spine.nodes : []) {
    if (node === null || typeof node !== "object") continue;
    const concepts = node.concepts?.length ?? 0;
    if (concepts <= NODE_CONCEPT_TARGET) continue;
    out.push({
      nodeId: node.id,
      stage: node.stage,
      strand: (node as SpineNode).strand ?? null,
      concepts,
      target: NODE_CONCEPT_TARGET,
      ceiling: maxNewAtomsPerChapter,
      severity: concepts > maxNewAtomsPerChapter ? "over-ceiling" : "over-target",
    });
  }
  // Worst first: the node that most needs splitting should not be found by scrolling.
  return out.sort((a, b) => b.concepts - a.concepts);
}

/** Distribution of nodes across strands and stages, plus what is missing. */
export function summarizeStrands(
  spine: CurriculumSpine,
  maxNewAtomsPerChapter: number,
): StrandSummary {
  const stages = Array.isArray(spine.stages) ? spine.stages : [];
  const nodes = Array.isArray(spine.nodes) ? spine.nodes : [];
  const allowed = declaredStrands(spine);

  const counts = new Map<string, { nodes: number; byStage: Record<string, number> }>();
  // Seeded from the DECLARED list, not from the nodes present, or a strand with
  // zero nodes would simply not appear -- which is precisely the finding.
  for (const strand of allowed) {
    // Object.create(null), NOT Object.fromEntries. A plain object inherits from
    // Object.prototype, and the membership test below used to be `in`, which walks
    // the prototype chain: a node declaring `stage: "toString"` passed the check,
    // read the inherited FUNCTION, and `+= 1` wrote the string
    // "function toString() { [native code] }1" into the counts. That string then
    // failed the `=== 0` test in missingStages, so the stage was reported as
    // COVERED. A gate that reports clean because of a crafted stage name is worse
    // than no gate, which is why this is a null-prototype map and an own-property
    // check rather than a comment saying "stages are trusted".
    const byStage: Record<string, number> = Object.create(null) as Record<string, number>;
    for (const stage of stages) byStage[stage] = 0;
    counts.set(strand, { nodes: 0, byStage });
  }

  let largest: { nodeId: string; concepts: number } | null = null;
  for (const node of nodes) {
    if (node === null || typeof node !== "object") continue;
    const concepts = node.concepts?.length ?? 0;
    if (largest === null || concepts > largest.concepts) {
      largest = { nodeId: node.id, concepts };
    }
    const strand = (node as SpineNode).strand as string | undefined;
    if (strand === undefined) continue;
    const bucket = counts.get(strand);
    if (bucket === undefined) continue; // an unknown strand is a defect, not a count
    bucket.nodes += 1;
    if (Object.prototype.hasOwnProperty.call(bucket.byStage, node.stage)) {
      bucket.byStage[node.stage] += 1;
    }
  }

  const strandCounts: StrandCount[] = allowed.map((strand) => {
    const bucket = counts.get(strand)!;
    return {
      strand: strand as CurriculumStrand,
      nodes: bucket.nodes,
      byStage: bucket.byStage,
      missingStages: stages.filter((stage) => (bucket.byStage[stage] ?? 0) === 0),
    };
  });

  return {
    strands: strandCounts,
    emptyStrands: strandCounts.filter((s) => s.nodes === 0).map((s) => s.strand),
    defects: strandDefects(spine),
    nodeSizeDefects: nodeSizeDefects(spine, maxNewAtomsPerChapter),
    totalNodes: nodes.length,
    largestNode: largest,
  };
}

/** Human-readable lines for the gap report. */
export function renderStrandSummary(summary: StrandSummary): string[] {
  const lines: string[] = [];
  const spread = summary.strands
    .map((s) => `${clean(s.strand)} ${s.nodes}`)
    .join(", ");
  lines.push(`strands: ${summary.totalNodes} nodes across ${summary.strands.length} strands -- ${spread}`);

  if (summary.emptyStrands.length > 0) {
    lines.push(
      `  strands with no nodes: ${summary.emptyStrands.map(clean).join(", ")} ` +
        `(declared as ladders, not yet climbed)`,
    );
  }
  for (const defect of summary.defects) {
    lines.push(`  ${clean(defect.nodeId)}: ${clean(defect.detail)}`);
  }
  const overCeiling = summary.nodeSizeDefects.filter((d) => d.severity === "over-ceiling");
  if (overCeiling.length > 0) {
    lines.push(
      `  nodes above the chapter atom ceiling: ${overCeiling.length} ` +
        `(worst ${clean(overCeiling[0]!.nodeId)} at ${overCeiling[0]!.concepts} concepts) ` +
        `-- no single chapter can realize these`,
    );
  }
  const overTarget = summary.nodeSizeDefects.filter((d) => d.severity === "over-target");
  if (overTarget.length > 0) {
    lines.push(`  nodes above the ${NODE_CONCEPT_TARGET}-concept design target: ${overTarget.length}`);
  }
  return lines;
}
