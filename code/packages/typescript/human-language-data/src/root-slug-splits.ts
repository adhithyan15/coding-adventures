// ---------------------------------------------------------------------------
// ROOT-SLUG SPLIT GUARD  (HL-C419)
//
// WHAT IS WRONG
// -------------
// `roots:` is the cousins join key, and `cousins.ts` joins on EXACT STRING
// EQUALITY. The corpus spells one etymon three ways -- bare `stare`,
// `stare-latin`, and `latin-stare` -- so three lessons that share a Latin verb
// can produce three panels of one item each instead of one panel of three.
//
// The loss is entirely of OMISSION, which is why it survived so long: a
// missing cousin panel looks exactly like a word that has no cousins yet.
// Nothing fails, nothing prints anything false, and every etymology in every
// lesson body is still accurate.
//
// WHY A GUARD BEFORE THE FIX
// --------------------------
// Normalising the affected lessons is a separate change that needs its own
// review, because a wrong merge ASSERTS A SHARED ETYMOLOGY THAT IS NOT THERE
// -- the one failure mode `cousins.ts` exists to prevent. But without a guard
// the corpus drifts back the first time somebody types a slug from memory,
// and that is not hypothetical: writing the chapter that prompted this found
// three invented slugs in four tranches.
//
// So this module does not fix anything. It pins today's splits in a baseline
// and fails on a NEW one. The baseline may only shrink.
//
// WHY THE VOCABULARY IS DECLARED AND NOT INFERRED
// -----------------------------------------------
// A split cannot be detected without knowing which hyphen-separated token is a
// language tag. The corpus has both kinds: `-latin` (651 slugs) and
// `-dravidian` (63) are tags, while `-see` (9), `-heart` (7), `-speak` (7) and
// bare single letters are glosses. NOTHING IN A SLUG DISTINGUISHES THEM.
//
// An earlier census of this same defect was wrong three times running, and
// every one of the three was the same mistake: a category list that lived in
// the counting code rather than in the data, so the result silently depended
// on what the author happened to think of. `core/root-tags.json` is that list,
// written down, checked in, and read from here.
//
// A slug whose tag is not declared is treated as OPAQUE -- its whole text is
// the lemma. That direction is chosen on purpose: an undeclared tag costs
// coverage, never a false positive.
// ---------------------------------------------------------------------------

import { resolve } from "node:path";
import { rootSlugs } from "./cousins.js";
import { defaultCurriculumRoot, loadLessons } from "./loader.js";
import { readLedgerFile } from "./shard.js";

export interface RootTagVocabulary {
  /** Declared language tags, matched at either end of a slug. */
  readonly tags: readonly string[];
  /** Tags that name the SAME language, folded before grouping. */
  readonly aliases: Readonly<Record<string, string>>;
}

export interface SlugParse {
  readonly slug: string;
  readonly lemma: string;
  /** `undefined` when no declared tag matched -- the slug is opaque. */
  readonly tag: string | undefined;
  readonly shape: "prefix" | "suffix" | "bare";
}

export interface RootSlugSplit {
  /** The normalised key the spellings collide on. */
  readonly key: string;
  /** Every live slug that normalises to `key`, sorted. */
  readonly slugs: readonly string[];
  readonly kind: "shape" | "bare-vs-tagged";
}

export const ROOT_TAGS_PATH = "core/root-tags.json";
export const ROOT_SLUG_BASELINE_PATH = "core/root-slug-split-baseline.json";

export function loadRootTagVocabulary(root = defaultCurriculumRoot()): RootTagVocabulary {
  // `readLedgerFile`, never a bare `JSON.parse(readFileSync(...))`: it refuses
  // a symlinked ledger, rejects `__proto__`/`constructor`/`prototype` keys,
  // scrubs parse errors so file bytes cannot reach a CI log, and — the one that
  // matters most here — REFUSES if a `core/root-tags.d/` ever appears beside
  // this file, because a bare read would then take stale data that parses
  // cleanly and is wrong.
  const raw = readLedgerFile<{ tags?: unknown; aliases?: unknown }>(
    resolve(root, ROOT_TAGS_PATH),
  );
  const tags = Array.isArray(raw.tags) ? raw.tags.map((tag) => String(tag)) : [];
  if (tags.length === 0) throw new Error(`${ROOT_TAGS_PATH}: no tags declared`);
  // A BLANK tag is not merely useless: it matches at both ends of everything,
  // collapsing every `lemma|tag` key onto the `lemma|` shape the bare-vs-tagged
  // kind already uses. A freshly written baseline then fails its own `--check`
  // and the gate is stuck red with no way to repair it from the data.
  for (const tag of tags) {
    if (tag.trim() === "") throw new Error(`${ROOT_TAGS_PATH}: a declared tag is blank`);
  }
  // `Object.create(null)`, not `{}`: `canonicalTag` looks a tag up by name, and
  // on a normal object `toString`, `valueOf` and `constructor` all answer with
  // an inherited function rather than `undefined`. `readLedgerFile` already
  // refuses `__proto__`/`constructor`/`prototype` as KEYS, but a tag whose
  // declared VALUE is one of those names is an array element and slips through.
  const aliases: Record<string, string> = Object.create(null) as Record<string, string>;
  if (raw.aliases && typeof raw.aliases === "object") {
    for (const [from, to] of Object.entries(raw.aliases as Record<string, unknown>)) {
      aliases[from] = String(to);
    }
  }
  // An alias target that is not itself a declared tag would silently create a
  // key nothing else can reach, so it is rejected rather than tolerated.
  for (const [from, to] of Object.entries(aliases)) {
    if (!tags.includes(to)) {
      throw new Error(`${ROOT_TAGS_PATH}: alias '${from}' points at undeclared tag '${to}'`);
    }
  }
  return { tags, aliases };
}

/**
 * Split a slug into lemma and language tag.
 *
 * Tags are tried LONGEST FIRST at either end, so `old-french` wins over
 * `french` on `corlieu-old-french`. Suffix position is tried before prefix
 * because the suffix shape is the corpus majority (651 `-latin` slugs), which
 * keeps the common case one comparison shorter and, more importantly, makes
 * the outcome deterministic for a slug that could parse either way.
 */
export function parseRootSlug(slug: string, vocabulary: RootTagVocabulary): SlugParse {
  // MATCHED CASE-INSENSITIVELY, and the lemma is lowercased with it. The corpus
  // carries exactly one case-only duplicate -- `SANSKRIT-PA-DRINK` in two
  // Marwadi lessons against `sanskrit-pa-drink` in four Gujarati, Punjabi and
  // Marathi ones -- which is one etymon spelled two ways across six lessons,
  // precisely the defect this module exists for. Matching case-sensitively
  // left the upper-case form opaque and the split invisible.
  const folded = slug.toLowerCase();
  const ordered = [...vocabulary.tags]
    .map((tag) => tag.toLowerCase())
    .sort((left, right) => right.length - left.length);
  for (const tag of ordered) {
    if (folded.endsWith(`-${tag}`)) {
      return { slug, lemma: folded.slice(0, -tag.length - 1), tag, shape: "suffix" };
    }
  }
  for (const tag of ordered) {
    if (folded.startsWith(`${tag}-`)) {
      return { slug, lemma: folded.slice(tag.length + 1), tag, shape: "prefix" };
    }
  }
  return { slug, lemma: folded, tag: undefined, shape: "bare" };
}

/**
 * Compare by CODE UNIT, never `localeCompare`.
 *
 * `localeCompare` is ICU-dependent, so two contributors on different Node
 * builds can reorder the whole generated baseline and produce a diff of
 * hundreds of lines that changes nothing. `loader.ts` records the same trap.
 */
function compareCodeUnits(left: string, right: string): number {
  return left < right ? -1 : left > right ? 1 : 0;
}

function canonicalTag(tag: string, vocabulary: RootTagVocabulary): string {
  return vocabulary.aliases[tag] ?? tag;
}

/** Every distinct `roots:` slug in the corpus, sorted. */
export function liveRootSlugs(root = defaultCurriculumRoot()): string[] {
  const seen = new Set<string>();
  for (const lesson of loadLessons(root)) {
    for (const slug of rootSlugs(lesson)) seen.add(slug);
  }
  return [...seen].sort(compareCodeUnits);
}

/**
 * Find every etymon the corpus spells more than one way.
 *
 * TWO KINDS, and they are reported separately because they are not equally
 * certain:
 *
 *   * **shape** -- `stare-latin` against `latin-stare`. Same lemma, same tag,
 *     different arrangement. These are defects with no argument available.
 *   * **bare-vs-tagged** -- `bonus` against `bonus-latin`. Same lemma, one of
 *     them untagged. Almost always the same defect, but a bare slug could in
 *     principle be a different word that happens to share a spelling, so the
 *     kind is kept distinct rather than merged into the first.
 *
 * DELIBERATELY NOT REPORTED: the same lemma under two DIFFERENT declared tags.
 * `dravidian` and `proto-dravidian` co-occur on eight lemmas, `frankish` and
 * `germanic` on three, `latin` and `pie` on five, and in every one of those the
 * two tags name genuinely different languages. Folding them would assert a
 * shared etymology rather than reveal one. The one pair that IS the same
 * language, `pie` and `proto-indo-european`, is handled by the alias table and
 * so shows up as a `shape` split, which is what it is.
 */
export function findRootSlugSplits(root = defaultCurriculumRoot()): RootSlugSplit[] {
  const vocabulary = loadRootTagVocabulary(root);
  const slugs = liveRootSlugs(root);
  const parsed = slugs.map((slug) => parseRootSlug(slug, vocabulary));

  const byKey = new Map<string, string[]>();
  for (const entry of parsed) {
    if (entry.tag === undefined) continue;
    const key = `${entry.lemma}|${canonicalTag(entry.tag, vocabulary)}`;
    byKey.set(key, [...(byKey.get(key) ?? []), entry.slug]);
  }

  const splits: RootSlugSplit[] = [];
  for (const [key, group] of byKey) {
    if (group.length > 1) splits.push({ key, slugs: [...group].sort(), kind: "shape" });
  }

  const taggedByLemma = new Map<string, string[]>();
  for (const entry of parsed) {
    if (entry.tag === undefined) continue;
    taggedByLemma.set(entry.lemma, [...(taggedByLemma.get(entry.lemma) ?? []), entry.slug]);
  }
  for (const entry of parsed) {
    if (entry.tag !== undefined) continue;
    const tagged = taggedByLemma.get(entry.lemma);
    if (!tagged) continue;
    splits.push({
      key: `${entry.lemma}|`,
      slugs: [entry.slug, ...tagged].sort(),
      kind: "bare-vs-tagged",
    });
  }

  return splits.sort((left, right) => compareCodeUnits(left.key, right.key));
}

export interface RootSlugBaseline {
  readonly version: number;
  readonly splits: readonly { readonly key: string; readonly slugs: readonly string[] }[];
}

export function loadRootSlugBaseline(root = defaultCurriculumRoot()): RootSlugBaseline {
  const raw = readLedgerFile<unknown>(resolve(root, ROOT_SLUG_BASELINE_PATH));
  // Without this, a truncated or hand-edited baseline reaches `diff` and dies
  // on `Cannot read properties of undefined (reading 'map')`, which names
  // nothing a CI reader can act on. It fails closed either way; the point is
  // that it says which file.
  if (
    raw === null ||
    typeof raw !== "object" ||
    !Array.isArray((raw as { splits?: unknown }).splits)
  ) {
    throw new Error(`${ROOT_SLUG_BASELINE_PATH}: expected an object with a 'splits' array`);
  }
  return raw as RootSlugBaseline;
}

/**
 * Compare today's splits against the committed baseline.
 *
 * BOTH DIRECTIONS FAIL, and the second one is the point. A new split is a
 * regression. A baseline entry that no longer exists means somebody fixed a
 * split without pruning the baseline, and a baseline that is allowed to go
 * stale stops being evidence of anything -- the same reason the latex warning
 * baseline is exact rather than a ceiling.
 */
export function diffRootSlugSplits(
  live: readonly RootSlugSplit[],
  baseline: RootSlugBaseline,
): { added: RootSlugSplit[]; resolved: string[] } {
  // `JSON.stringify`, not `slugs.join(" ")`: six live slugs contain spaces
  // (`ad de magis`, `qui sapit`, `sub ponere`, ...), and a space-joined string
  // makes `["a b", "c"]` and `["a", "b c"]` compare equal.
  const fingerprint = (slugs: readonly string[]) => JSON.stringify(slugs);
  const baselineKeys = new Map(baseline.splits.map((split) => [split.key, fingerprint(split.slugs)]));
  const liveKeys = new Map(live.map((split) => [split.key, fingerprint(split.slugs)]));
  const added = live.filter((split) => baselineKeys.get(split.key) !== fingerprint(split.slugs));
  const resolved = [...baselineKeys.keys()]
    .filter((key) => !liveKeys.has(key))
    .sort(compareCodeUnits);
  return { added, resolved };
}

export function serialiseRootSlugBaseline(splits: readonly RootSlugSplit[]): string {
  const payload = {
    version: 1,
    note:
      "Generated by `npm run generate:root-slug-splits`. HL-C419: every etymon " +
      "the corpus spells more than one way. THIS FILE MAY ONLY SHRINK -- a new " +
      "entry is a regression and the guard fails on one. Pruning an entry is " +
      "how a normalisation PR proves it worked.",
    splits: splits.map((split) => ({ key: split.key, kind: split.kind, slugs: split.slugs })),
  };
  return `${JSON.stringify(payload, null, 2)}\n`;
}
