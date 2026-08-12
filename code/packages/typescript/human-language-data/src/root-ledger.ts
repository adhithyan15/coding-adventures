/**
 * HL10 section 6.2 -- the Root Ledger.
 *
 * WHY ETYMOLOGY NEEDS AN ACCOUNT
 *
 * HL00 calls the etymology "the heart of the lesson... the signature of this
 * curriculum", and it is genuinely the strongest thing in the corpus: 708
 * lessons carry an `etymology_hook`, and the chapter 1-8 review singled out
 * `hasta` and eight centuries of al-Andalus, `de nada` from *res nata*,
 * `trabajar` from *tripalium* and its road to English *travel*.
 *
 * But a root is only *useful* if it is spent again. HL10's rule:
 *
 *   rootLedgerMinReuse: 3 -- a root may be taught only if at least three LATER
 *   lessons draw on it. Roots that pay off fewer than three times are cut, or
 *   moved later to where the payoff lives.
 *
 * That turns etymology from a sequence of pleasant asides into a compounding
 * asset, and it is what makes the friends layer (HL10 section 6.7) generatable
 * rather than hand-typed: a root with recorded payoffs already knows which
 * later words it predicts.
 *
 * WHAT THE FIRST MEASUREMENT FOUND
 *
 * Across both namespaces: 2,717 roots, of which 2,624 pay off fewer than three
 * times (97%) and 1,807 are never spent at all -- named once and never returned
 * to. Spanish alone: 303 roots, 290 underspent, 190 never spent.
 *
 * The etymology is real, it is good, and almost none of it is being spent.
 * That is the difference between a curriculum whose vocabulary compounds and
 * one where every lesson starts over.
 *
 * TWO NAMESPACES, DELIBERATELY BOTH
 *
 * The corpus records etymology twice, and neither is redundant:
 *
 *   roots:            cross-language slugs -- `stare-latin`, `sanskrit-ratri`.
 *                     These are the join keys that let a Spanish root and an
 *                     Italian one be recognised as the same root.
 *   <LANG>-ETYMON-*   atoms in the knowledge graph, which participate in
 *                     prerequisites and the reinforcement windows.
 *
 * A ledger over only one of them would report a root as unspent while the
 * other namespace was quietly spending it, so this module reads both and says
 * which namespace each entry came from.
 *
 * REPORT-ONLY
 *
 * Nothing here throws or fails a build. The corpus predates the rule, and per
 * the HL05 precedent a gate that fails on already-recorded debt teaches authors
 * to route around it.
 */

import type { ParsedLesson } from "./parse.js";
import { frontmatterList } from "./ramp.js";
import { stripControlCharacters as clean } from "./constants.js";

export type RootNamespace = "roots" | "etymon-atom";

export interface RootEntry {
  /** The root's id: a cross-language slug, or a namespaced etymon atom. */
  root: string;
  namespace: RootNamespace;
  language: string;
  /** The earliest lesson, in reading order, that names this root. */
  introducedBy: string;
  /**
   * Lessons AFTER the introducing one that name it again. The count that
   * decides whether the root earned its place -- an introduction is not a
   * payoff, so a root named in exactly one lesson scores zero, not one.
   */
  payoffs: string[];
  payoffCount: number;
  /** True when payoffCount is below the configured minimum. */
  underspent: boolean;
}

export interface RootLedger {
  entries: RootEntry[];
  minReuse: number;
  summary: {
    roots: number;
    underspent: number;
    /** Roots named in exactly one lesson -- taught once, never spent. */
    neverSpent: number;
    /** payoffCount -> how many roots have it. */
    payoffDistribution: Record<string, number>;
    underspentPercent: number;
  };
}

function frontmatterValue(lesson: ParsedLesson, key: string): unknown {
  return (lesson.frontmatter as Record<string, unknown>)[key];
}

function lessonId(lesson: ParsedLesson): string {
  const raw = frontmatterValue(lesson, "id");
  return typeof raw === "string" ? raw : "<unidentified lesson>";
}

/**
 * Reading order.
 *
 * `sequence` arrives from the frontmatter parser as a STRING; see the identical
 * note in grammar-cells.ts, where testing `typeof raw === "number"` silently
 * turned the sort into a no-op. Compared rather than subtracted, because
 * Infinity - Infinity is NaN and an inconsistent comparator would leave the
 * introducing lesson of an unsequenced pair arbitrary.
 */
function sequenceOf(lesson: ParsedLesson): number {
  const raw = frontmatterValue(lesson, "sequence");
  if (raw === undefined || raw === null || String(raw).trim() === "") {
    return Number.POSITIVE_INFINITY;
  }
  const value = typeof raw === "number" ? raw : Number(raw);
  return Number.isFinite(value) ? value : Number.POSITIVE_INFINITY;
}

function declaredRootSlugs(lesson: ParsedLesson): string[] {
  return frontmatterList(lesson, "roots")
    .map((v) => v.trim())
    .filter((v) => v !== "");
}

/**
 * Etymon atoms this lesson names, from any knowledge list.
 *
 * The frontmatter keys are FLAT AND DOTTED -- `introduces.knowledge`, not a
 * nested `introduces` object. Reading them as nested returns undefined for
 * every lesson in the corpus; the first draft of this function did exactly
 * that and silently contributed ZERO etymon atoms to the ledger, which looked
 * like "the corpus has no etymon atoms" rather than "the reader is broken".
 * ramp.ts carries the same warning because the same mistake once made the
 * chapter gates report all 279 authored chapters as broken.
 *
 * Scanning the knowledge lists rather than the body is deliberate: an etymon
 * mentioned only in prose is not in the knowledge graph, so it cannot carry a
 * prerequisite or close a reinforcement window, and counting it would credit a
 * payoff the machinery cannot deliver.
 */
function declaredEtymonAtoms(lesson: ParsedLesson): string[] {
  const out = new Set<string>();
  for (const key of ["introduces.knowledge", "requires.knowledge", "practises.knowledge"]) {
    for (const atom of frontmatterList(lesson, key)) {
      if (/^[A-Z]{2}-ETYMON-/.test(atom)) out.add(atom);
    }
  }
  return [...out];
}

/** Build the ledger over one or more tracks. */
export function buildRootLedger(lessons: ParsedLesson[], minReuse: number): RootLedger {
  const ordered = [...lessons].sort((a, b) => {
    const left = sequenceOf(a);
    const right = sequenceOf(b);
    if (left < right) return -1;
    if (left > right) return 1;
    // Stable tie-break by id, so an unsequenced pair does not swap between runs
    // and change which lesson is reported as the introducer.
    return lessonId(a).localeCompare(lessonId(b));
  });

  // Keyed by language + namespace + root: the same slug in two tracks is two
  // ledger entries, because a Spanish root spent only in Italian has not been
  // spent for the Spanish reader.
  const seen = new Map<string, { entry: RootEntry }>();

  for (const lesson of ordered) {
    const id = lessonId(lesson);
    const language = lesson.language;
    const named: [RootNamespace, string][] = [
      ...declaredRootSlugs(lesson).map((r): [RootNamespace, string] => ["roots", r]),
      ...declaredEtymonAtoms(lesson).map((r): [RootNamespace, string] => ["etymon-atom", r]),
    ];
    for (const [namespace, root] of named) {
      // Length-prefixed, not separator-joined. A root slug is author-written and
      // may contain anything, so `${language} ${namespace} ${root}` would let
      // ("es", "roots", "a b") and ("es", "roots a", "b") collide and silently
      // merge two roots' payoff counts into one.
      const key = `${language.length}:${language}|${namespace.length}:${namespace}|${root}`;
      const existing = seen.get(key);
      if (existing === undefined) {
        seen.set(key, {
          entry: {
            root,
            namespace,
            language,
            introducedBy: id,
            payoffs: [],
            payoffCount: 0,
            underspent: true,
          },
        });
        continue;
      }
      // A lesson naming the same root twice is still one payoff.
      if (existing.entry.introducedBy === id || existing.entry.payoffs.includes(id)) continue;
      existing.entry.payoffs.push(id);
    }
  }

  const entries = [...seen.values()].map(({ entry }) => {
    entry.payoffCount = entry.payoffs.length;
    entry.underspent = entry.payoffCount < minReuse;
    return entry;
  });
  // Worst first: a root spent zero times is the one to cut or move.
  entries.sort((a, b) => a.payoffCount - b.payoffCount || a.root.localeCompare(b.root));

  const distribution: Record<string, number> = {};
  for (const entry of entries) {
    const bucket = String(entry.payoffCount);
    distribution[bucket] = (distribution[bucket] ?? 0) + 1;
  }

  const underspent = entries.filter((e) => e.underspent).length;
  return {
    entries,
    minReuse,
    summary: {
      roots: entries.length,
      underspent,
      neverSpent: entries.filter((e) => e.payoffCount === 0).length,
      payoffDistribution: distribution,
      underspentPercent: entries.length === 0 ? 0 : Math.round((underspent / entries.length) * 100),
    },
  };
}

/** Human-readable lines for the gap report. */
export function renderRootLedger(ledger: RootLedger): string[] {
  const { summary } = ledger;
  const lines = [
    `root ledger: ${summary.roots} roots, ${summary.underspent} spent fewer than ` +
      `${ledger.minReuse} times (${summary.underspentPercent}%), ${summary.neverSpent} never spent at all`,
  ];
  const worst = ledger.entries.filter((e) => e.payoffCount === 0).slice(0, 3);
  if (worst.length > 0) {
    lines.push(
      `  taught once and never returned to: ${worst.map((e) => `${clean(e.root)} (${clean(e.introducedBy)})`).join(", ")}`,
    );
  }
  const best = [...ledger.entries].sort((a, b) => b.payoffCount - a.payoffCount)[0];
  if (best !== undefined && best.payoffCount > 0) {
    lines.push(`  best-spent root: ${clean(best.root)} with ${best.payoffCount} payoffs`);
  }
  return lines;
}
