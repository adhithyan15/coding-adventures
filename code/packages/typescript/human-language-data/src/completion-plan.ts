// ---------------------------------------------------------------------------
// completion-plan.ts — the backlog, computed.
//
// WHY THIS MODULE EXISTS
//
// `BACKLOG.md` carried 148 hand-written entries and a prioritization header that
// was three days stale the day it was read: it ordered the work against a frame
// two later findings had already replaced. Nobody was careless. That is simply
// what happens when the ordering lives in prose that nothing recomputes.
//
// Meanwhile every number needed to order the work is computed on every run.
// `level-gate.ts` says which of four criteria each track fails and by how much.
// `script-closure.ts` says exactly how many glyphs a track shows its reader and
// never taught. `exam-inventory.ts` measures against an external published list.
//
// So the queue is derived rather than typed:
//
//     BACKLOG.md records what was LEARNED.  This module computes what is NEXT.
//
// It is the same argument `exam-inventory.ts` makes for probes over annotations,
// one level up. An annotation goes stale silently and in the flattering
// direction — and so does a hand-ordered backlog.
//
// WHAT IT IS NOT
//
// It is not a planner. It makes no estimate of effort in time, invents no
// dependency graph, and never decides that a deficit is acceptable. It reads
// measured shortfalls, groups them into tranches of a size that merged PRs have
// actually sustained, and sorts. Everything interesting is in the sort, and the
// sort is three mechanical keys with no judgement call at queue-build time.
//
// See `code/specs/HL15-the-completion-plan.md`.
// ---------------------------------------------------------------------------
import type { CefrLevel } from "./levels.js";
import { CEFR_LEVELS, levelRank } from "./levels.js";
import { LEVEL_VOCABULARY, type LevelGateReport } from "./level-gate.js";
import type { ScriptClosureReport } from "./script-closure.js";

/** The seven families of work. Every item belongs to exactly one. */
export type WorkKind =
  | "exam-inventory"
  | "script-closure"
  | "vocabulary"
  | "exam-point"
  | "reinforcement"
  | "atom-budget"
  | "spine-nodes";

/**
 * Sort key 2 — see HL15 §4.2. Lower goes first.
 *
 * The two entries worth defending:
 *
 * `exam-inventory` is 1 because until the external list exists, every other
 * number for that level is a proxy for something nobody is graded on. It is also
 * the cheapest family in the file: research and JSON, no content authoring.
 *
 * `script-closure` is 2 — ahead of the vocabulary grind — because it is the only
 * family with a TERMINAL state. Tamil has 247 glyphs and then it is finished
 * forever; vocabulary runs to 16,000 per track. Finite work that unblocks
 * infinite work goes first, and a vocabulary tranche authored into an unclosed
 * script is authored onto sand: the reader cannot decode the word it teaches.
 */
export const KIND_PRIORITY: Readonly<Record<WorkKind, number>> = Object.freeze({
  "exam-inventory": 1,
  "script-closure": 2,
  vocabulary: 3,
  "exam-point": 4,
  reinforcement: 5,
  "atom-budget": 6,
  "spine-nodes": 7,
});

/**
 * How much outstanding work one PR carries, per family.
 *
 * These are EMPIRICAL, not aesthetic. 35 headwords is the size HL-C198's six
 * merged tranches actually sustained at one PR each — not a round number
 * somebody liked. They live in one frozen constant so that a future measurement
 * can move one of them and re-shape the whole queue without anybody editing a
 * list of work items, which is the entire point of computing the queue.
 */
export const TRANCHE_SIZE: Readonly<Record<WorkKind, number>> = Object.freeze({
  "exam-inventory": 1,
  "script-closure": 10,
  vocabulary: 35,
  "exam-point": 5,
  reinforcement: 25,
  "atom-budget": 10,
  "spine-nodes": 3,
});

/**
 * CEFR levels an awarding body actually certifies.
 *
 * `pre-A1` is deliberately absent. `core/exam-levels.json` states plainly that
 * pre-A1 is NOT a CEFR level — it is this curriculum's own label for the ramp
 * below A1 — so no awarding body publishes an inventory for it, and queueing an
 * `exam-inventory` item at pre-A1 would be queueing work that cannot be sourced.
 * pre-A1 is measured against this project's own editorial floor instead, which
 * `level-gate.ts` already applies.
 */
export const CERTIFIABLE_LEVELS: readonly CefrLevel[] = CEFR_LEVELS.filter((level) => level !== "pre-A1");

/** One unit of pickup-able work. */
export interface WorkItem {
  /** Stable and derivable — the same corpus produces the same id on every run. */
  id: string;
  kind: WorkKind;
  language: string;
  /** The level this item serves. Sort key 1. */
  level: CefrLevel;
  /** What has to become true, in a sentence naming the measured number. */
  goal: string;
  /** Units still outstanding, in this family's own units. */
  outstanding: number;
  /** PRs this decomposes into at the family's tranche size. Sort key 3. */
  tranches: number;
}

/** Work counted rather than listed, because listing it would be a lie. */
export interface PlanProjection {
  kind: WorkKind;
  /** Items remaining to the ceiling, or `null` where the family is not projectable. */
  items: number | null;
  detail: string;
}

export interface CompletionPlan {
  ceiling: CefrLevel;
  /** The next items, fully enumerated and ready to pick up. */
  head: WorkItem[];
  /** Everything behind the head, counted per family. */
  projection: PlanProjection[];
  summary: {
    tracks: number;
    /** Tracks that have attained the ceiling and need nothing further. */
    tracksDone: number;
    itemsInHead: number;
    /** Total enumerable items today, of which `head` is the first slice. */
    itemsOutstanding: number;
    /** Head plus projected tail — the honest size of the thing. */
    projectedTotal: number | null;
  };
}

/** A (track, level) pair that already has an external inventory on disk. */
export interface InventoryPresence {
  language: string;
  level: CefrLevel;
}

export interface CompletionPlanInput {
  levelGate: LevelGateReport;
  scriptClosure: ScriptClosureReport;
  /** Which external inventories exist. Absent ones become `exam-inventory` items. */
  inventories: readonly InventoryPresence[];
  /** How far the plan runs. Defaults to C2 — the whole ladder. */
  ceiling?: CefrLevel;
  /** How many items to enumerate. Defaults to 25. */
  headSize?: number;
}

/** Ceil-divide, with the degenerate case that zero outstanding is zero tranches. */
function tranchesFor(outstanding: number, kind: WorkKind): number {
  if (outstanding <= 0) return 0;
  return Math.ceil(outstanding / TRANCHE_SIZE[kind]);
}

/**
 * The lowest certifiable level at or above `from` that has no inventory yet.
 *
 * Called with the level a track is IN PROGRESS at, so a track working pre-A1 is
 * asked for its A1 inventory. That is deliberate: the target for the next rung
 * has to be written down before the climb reaches it, or the climb is once again
 * aimed at a number this repository invented. HL-C184's Phase 0 made exactly
 * this point and it is the reason the item is ordered at the IN-PROGRESS level
 * rather than at the level it describes.
 */
function nextMissingInventory(
  language: string,
  from: CefrLevel,
  ceiling: CefrLevel,
  have: ReadonlySet<string>,
): CefrLevel | null {
  for (const level of CERTIFIABLE_LEVELS) {
    if (levelRank(level) < levelRank(from)) continue;
    if (levelRank(level) > levelRank(ceiling)) break;
    if (!have.has(`${language}/${level}`)) return level;
  }
  return null;
}

/**
 * Build the ordered queue from measured deficits.
 *
 * Pure over report data — it never touches the filesystem — so a test can hand
 * it a two-track fixture and assert on the ordering without building a corpus.
 */
export function buildCompletionPlan(input: CompletionPlanInput): CompletionPlan {
  const ceiling = input.ceiling ?? "C2";
  const headSize = input.headSize ?? 25;
  const have = new Set(input.inventories.map((entry) => `${entry.language}/${entry.level}`));
  const closureByLanguage = new Map(input.scriptClosure.tracks.map((track) => [track.language, track]));

  // Items are collected PER TRACK and interleaved at the end, never pooled and
  // sorted flat. The first version of this function pooled them, and the queue it
  // produced was 21 consecutive `exam-inventory` items — every track's research
  // task, ahead of every track's content. That is a correct reading of family
  // priority and a useless work queue: no language moves until all of them have
  // moved on one axis. See `interleave` below.
  const byTrack = new Map<string, WorkItem[]>();
  const items: WorkItem[] = [];
  let tracksDone = 0;

  for (const track of input.levelGate.tracks) {
    // `inProgressAt` is null exactly when the ladder is complete. A track that
    // has attained the ceiling needs nothing from this plan.
    const level = track.inProgressAt;
    if (level === null || levelRank(level) > levelRank(ceiling)) {
      tracksDone += 1;
      continue;
    }
    const mine: WorkItem[] = [];

    // 1. The external target for the next certifiable rung, if it is not written.
    const missing = nextMissingInventory(track.language, level, ceiling, have);
    if (missing !== null) {
      mine.push({
        id: `exam-inventory/${track.language}/${missing}`,
        kind: "exam-inventory",
        language: track.language,
        level,
        goal:
          `write the external ${missing} exam inventory for ${track.language}; ` +
          `until it exists, every ${track.language} number at ${missing} is a proxy`,
        outstanding: 1,
        tranches: 1,
      });
    }

    // 2. The script. `violations` is the symptom — lessons that ask for an
    // untaught glyph — but the WORK is teaching the glyphs, so the outstanding
    // count is `neverTaughtGlyphs`. Counting violations instead would make the
    // queue shrink by deleting a lesson, which is not progress.
    const closure = closureByLanguage.get(track.language);
    if (closure && closure.neverTaughtGlyphs > 0) {
      mine.push({
        id: `script-closure/${track.language}`,
        kind: "script-closure",
        language: track.language,
        level,
        goal:
          `teach ${closure.neverTaughtGlyphs} ${closure.script} glyph(s) the track shows but never taught ` +
          `(${closure.violations} lesson(s) currently ask the reader to decode one)`,
        outstanding: closure.neverTaughtGlyphs,
        tranches: tranchesFor(closure.neverTaughtGlyphs, "script-closure"),
      });
    }

    // 3..7. Whatever the level gate says is short, in its own units. The gate
    // already scopes each criterion to at-or-below the level, which is the bug
    // HL09 recorded and this module must not reintroduce by re-deriving them.
    for (const blocker of track.blockers) {
      const kind = blocker.criterion as WorkKind;
      mine.push({
        id: `${kind}/${track.language}/${level}`,
        kind,
        language: track.language,
        level,
        goal: blocker.detail,
        outstanding: blocker.shortfall,
        tranches: tranchesFor(blocker.shortfall, kind),
      });
    }

    // WITHIN one track the order is the spec's first two keys: the lowest rung
    // first, then family priority. This is the sequence a single language walks.
    mine.sort(
      (a, b) => levelRank(a.level) - levelRank(b.level) || effectivePriority(a) - effectivePriority(b),
    );
    byTrack.set(track.language, mine);
    items.push(...mine);
  }

  return {
    ceiling,
    head: interleave(byTrack, input.levelGate).slice(0, headSize),
    projection: project(input, ceiling, items),
    summary: {
      tracks: input.levelGate.tracks.length,
      tracksDone,
      itemsInHead: Math.min(headSize, items.length),
      itemsOutstanding: items.length,
      projectedTotal: projectedTotal(input, ceiling, items),
    },
  };
}

/**
 * Family priority, adjusted for whether the item serves the rung the track is
 * actually standing on.
 *
 * `exam-inventory` is family 1 because you cannot climb a rung whose target
 * nobody has written down. That is true — for the rung you are ON. Every track
 * in the corpus today sits at pre-A1, which `core/exam-levels.json` states
 * plainly is NOT a CEFR level and which no awarding body publishes an inventory
 * for, so the item those tracks generate is for A1: the rung ABOVE. That is
 * lookahead, and the first build of this module let lookahead outrank the floor
 * — producing a queue of twenty-two consecutive research PRs while eight tracks
 * taught no letters at all and one held three words.
 *
 * So the demotion: an inventory for a level the track has not reached yet sorts
 * LAST within that track. It is still queued, still counted in the projection,
 * and it jumps straight back to family 1 the moment the track's in-progress
 * level becomes the level the inventory describes. Writing the target down stays
 * a precondition for the climb; it stops being a precondition for the floor.
 */
function effectivePriority(item: WorkItem): number {
  if (item.kind === "exam-inventory" && !CERTIFIABLE_LEVELS.includes(item.level)) {
    return KIND_PRIORITY["spine-nodes"] + 1;
  }
  return KIND_PRIORITY[item.kind];
}

/**
 * Round-robin the per-track queues: every language moves once before any moves twice.
 *
 * WHY NOT A FLAT SORT
 *
 * The obvious implementation pools every item and sorts by (level, family,
 * cost). It was written that way first, and against the real corpus it produced
 * a head of twenty-one consecutive `exam-inventory` items — every track's
 * research task ahead of every track's content, because all 22 tracks sit at the
 * same rung and `exam-inventory` is family 1. Each of those items is correctly
 * placed and the queue as a whole is useless: no language moves at all until all
 * of them have moved on one axis, and the slowest tracks stay slowest.
 *
 * The owner's rule is the other one, and it is a rotation rather than a sort:
 * *"Please move all languages forward... one loop iteration at a time."* So the
 * queue takes each track's single most important next action, in track order,
 * before it comes back for anybody's second. Family priority still decides what
 * that action IS — it just no longer decides whose turn it is.
 *
 * TRACK ORDER IS FURTHEST-BEHIND-FIRST, which is the same rule the hand-run
 * backlog was already following when it wrote "cheapest first: tamil 84,
 * malayalam 84" and meant the two LOWEST counts. A track that is further from
 * its target has more claim on the next slot than one that is nearly there,
 * because the goal is a reader who can sit an exam in any of these languages —
 * not a leaderboard.
 */
function interleave(byTrack: ReadonlyMap<string, readonly WorkItem[]>, gate: LevelGateReport): WorkItem[] {
  const shortfall = new Map<string, number>();
  const rung = new Map<string, number>();
  for (const track of gate.tracks) {
    const vocabulary = track.blockers.find((blocker) => blocker.criterion === "vocabulary");
    shortfall.set(track.language, vocabulary?.shortfall ?? 0);
    rung.set(track.language, track.inProgressAt === null ? Number.MAX_SAFE_INTEGER : levelRank(track.inProgressAt));
  }

  const order = [...byTrack.keys()].sort(
    (a, b) =>
      // Lowest rung first — the floor is universal (HL15 4.1).
      (rung.get(a) ?? 0) - (rung.get(b) ?? 0) ||
      // Then furthest behind on that rung.
      (shortfall.get(b) ?? 0) - (shortfall.get(a) ?? 0) ||
      a.localeCompare(b),
  );

  const out: WorkItem[] = [];
  const deepest = Math.max(0, ...order.map((language) => byTrack.get(language)?.length ?? 0));
  for (let slot = 0; slot < deepest; slot += 1) {
    for (const language of order) {
      const item = byTrack.get(language)?.[slot];
      if (item) out.push(item);
    }
  }
  return out;
}

/**
 * The tail, counted per family.
 *
 * Only two families can honestly be projected to the ceiling. Vocabulary can,
 * because `LEVEL_VOCABULARY` states the cumulative target at every level and the
 * corpus states what a track holds today. Inventories can, because there are
 * exactly 22 tracks times six certifiable levels and that product does not move.
 *
 * The other five cannot, and are reported as `null` rather than as a number.
 * Reinforcement debt at B1 is a function of lessons nobody has written yet;
 * quoting a figure for it would be inventing one. `null` here means NOT
 * PROJECTABLE, which is a different fact from zero — the same distinction
 * `level-gate.ts` had to learn between "not measured" and "attained nothing".
 */
function project(input: CompletionPlanInput, ceiling: CefrLevel, items: readonly WorkItem[]): PlanProjection[] {
  const target = LEVEL_VOCABULARY[ceiling];
  let words = 0;
  let tracksShort = 0;
  for (const track of input.levelGate.tracks) {
    const short = Math.max(0, target - track.vocabulary);
    if (short > 0) tracksShort += 1;
    words += short;
  }
  const vocabularyItems = Math.ceil(words / TRANCHE_SIZE.vocabulary);

  const wanted = input.levelGate.tracks.length * CERTIFIABLE_LEVELS.filter((level) => levelRank(level) <= levelRank(ceiling)).length;
  const inventoryItems = Math.max(0, wanted - input.inventories.length);

  const glyphs = input.scriptClosure.tracks.reduce((sum, track) => sum + track.neverTaughtGlyphs, 0);

  const counted = (kind: WorkKind) => items.filter((item) => item.kind === kind).length;

  return [
    {
      kind: "vocabulary",
      items: vocabularyItems,
      detail:
        `${words.toLocaleString()} headword(s) short of ${target.toLocaleString()} across ` +
        `${tracksShort} track(s), at ${TRANCHE_SIZE.vocabulary} per tranche`,
    },
    {
      kind: "exam-inventory",
      items: inventoryItems,
      detail: `${input.inventories.length} of ${wanted} (track x certifiable level) inventories written`,
    },
    {
      kind: "script-closure",
      items: Math.ceil(glyphs / TRANCHE_SIZE["script-closure"]),
      detail: `${glyphs} glyph(s) shown but never taught, corpus-wide; finite and ends`,
    },
    {
      kind: "exam-point",
      items: null,
      detail: "not projectable — depends on inventories not yet written",
    },
    {
      kind: "reinforcement",
      items: null,
      detail: `not projectable — a function of lessons not yet authored (${counted("reinforcement")} open today)`,
    },
    {
      kind: "atom-budget",
      items: null,
      detail: `not projectable — a regression signal, not a deficit (${counted("atom-budget")} open today)`,
    },
    {
      kind: "spine-nodes",
      items: null,
      detail: `not projectable — depends on spine growth above B1 (${counted("spine-nodes")} open today)`,
    },
  ];
}

/** Head plus projectable tail, or `null` if nothing can be projected at all. */
function projectedTotal(input: CompletionPlanInput, ceiling: CefrLevel, items: readonly WorkItem[]): number | null {
  const projectable = project(input, ceiling, items)
    .map((entry) => entry.items)
    .filter((count): count is number => count !== null);
  if (projectable.length === 0) return null;
  return projectable.reduce((sum, count) => sum + count, 0);
}

/** Render the plan for a terminal. */
export function renderCompletionPlan(plan: CompletionPlan): string[] {
  const lines: string[] = [];
  lines.push(`Completion plan (HL15) — ceiling ${plan.ceiling}`);
  lines.push("=".repeat(`Completion plan (HL15) — ceiling ${plan.ceiling}`.length));
  lines.push(
    `${plan.summary.tracks} tracks, ${plan.summary.tracksDone} done; ` +
      `${plan.summary.itemsOutstanding} enumerable item(s) today, ` +
      `~${plan.summary.projectedTotal?.toLocaleString() ?? "?"} projected to ${plan.ceiling}`,
  );
  lines.push("");
  lines.push(`Next ${plan.summary.itemsInHead} item(s), in order:`);
  plan.head.forEach((item, index) => {
    lines.push(`  ${String(index + 1).padStart(3)}. [${item.level}] ${item.kind} — ${item.language}`);
    lines.push(`       ${item.goal}`);
    lines.push(`       ${item.outstanding} outstanding, ${item.tranches} tranche(s)`);
  });
  lines.push("");
  lines.push("Projection to the ceiling, per family:");
  for (const entry of plan.projection) {
    const count = entry.items === null ? "     —" : String(entry.items).padStart(6);
    lines.push(`  ${count}  ${entry.kind.padEnd(15)} ${entry.detail}`);
  }
  return lines;
}
