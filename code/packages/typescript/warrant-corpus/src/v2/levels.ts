/**
 * # Feature levels and the tag registry (VER04r section 4.1, Appendix A)
 *
 * The model IR grows in closed *levels*.  A level is a set of tags, built-ins
 * and types; a lane declares the levels it implements and everything else is
 * `level-missing` (a refusal scoped by what an expression *reaches*, [D64]).
 *
 * This runner implements three levels: `ir.core`, `ir.fixed` (S10f) and
 * `ir.tuple` (S10f).  It must still *know the names* of every defined level,
 * because the spec asks a runner to tell two refusals apart for something it
 * cannot run:
 *
 *   * a tag of a defined level this runner lacks        -> `level-missing`
 *   * a tag no level defines                            -> `unknown-tag`
 *
 * so `some` (a tag of `ir.data`) is `level-missing` while `frobnicate` is
 * `unknown-tag`.  (A *program* configured with fewer levels, as every kit
 * case's own `levels` field does, still refuses `bit_xor` as `level-missing`.)  The tables below are the closed tag sets of Appendix A, A.1
 * and A.2, plus the reserved levels' names.
 */

/**
 * The levels this lane implements.  Reserved levels are never in this list ([D66]).  `ir.data` is not here either:
 * `match` on anything but a `Result`, the data patterns, `Option` as a real type and records and variants are all
 * `level-missing` (R76).
 */
export const IMPLEMENTED_LEVELS: readonly string[] = ["ir.core", "ir.fixed", "ir.tuple"];

/** Every level name this document defines, implemented or reserved. */
export const DEFINED_LEVELS: ReadonlySet<string> = new Set([
  "ir.core",
  "ir.fixed",
  "ir.tuple",
  "ir.data",
  "ir.list",
  "ir.seq",
  "ir.inout.records",
]);

/** `ir.core`: the 21 tags v1 emits plus `neg concat take drop is_err` (Appendix A). */
const CORE_TAGS = [
  "lit", "var", "subject_op", "model_call", "len", "ok", "err",
  "eq", "ne", "lt", "gt", "le", "ge", "add", "sub", "mul",
  "and", "or", "not", "implies", "if",
  "concat", "take", "drop", "is_err", "neg",
]; // prettier-ignore

/** `ir.fixed`: exactly the 35 names of Appendix A.1 [D24]. */
const FIXED_TAGS = [
  "bit_and", "bit_or", "bit_xor", "bit_not", "shl", "shr", "sar", "rotl", "rotr",
  "popcount", "clz", "ctz", "bit", "widen", "wrap", "try", "to_int", "to_nat",
  "max_value", "min_value", "checked_add", "checked_sub", "checked_mul",
  "div", "rem", "div_trunc", "rem_trunc", "imin", "imax",
  "be_bytes", "le_bytes", "from_be", "from_le", "at", "unwrap_or",
]; // prettier-ignore

/**
 * `ir.tuple`: the tuple literal, `project`, and `match`.
 *
 * `match` is `ir.tuple` when its scrutinee is a `Result` and `ir.data`
 * otherwise (R76); deciding which needs a type this lane cannot compute for a
 * level it lacks, so the registry names the lower of the two.  Either way a
 * lane without both levels refuses it as `level-missing`.
 */
const TUPLE_TAGS = ["tuple", "project", "match"];

/** `ir.data`: the four constructors of Appendix A.2. */
const DATA_TAGS = ["some", "none", "ctor", "field"];

/** `ir.list`: reserved ([D63]); the tag `list` is a known tag of a reserved level (R57). */
const LIST_TAGS = ["list"];

/** `ir.seq`: reserved, no content in this document; the WX-D names it will use. */
const SEQ_TAGS = ["fold", "map", "all", "any", "range", "count"];

const TAG_LEVEL = new Map<string, string>();
for (const [level, tags] of [
  ["ir.core", CORE_TAGS],
  ["ir.fixed", FIXED_TAGS],
  ["ir.tuple", TUPLE_TAGS],
  ["ir.data", DATA_TAGS],
  ["ir.list", LIST_TAGS],
  ["ir.seq", SEQ_TAGS],
] as const) {
  for (const tag of tags) TAG_LEVEL.set(tag, level);
}

/** The level a tag belongs to, or `undefined` for a tag of no level (an unknown tag). */
export function levelOfTag(tag: string): string | undefined {
  return TAG_LEVEL.get(tag);
}

/** The `ir.core` tag set, for the tests that check the table against Appendix A. */
export const CORE_TAG_LIST: readonly string[] = CORE_TAGS;
/** The `ir.fixed` tag set, likewise. */
export const FIXED_TAG_LIST: readonly string[] = FIXED_TAGS;

/**
 * The order in which a missing level is *named* when several are missing:
 * the reserved and highest-content levels first, because they imply the others
 * (`ir.list` implies `ir.data`, which implies `ir.tuple`).  A refusal of a
 * `List<T>` therefore names `ir.list` ([D63]), never `ir.data`.
 */
const NAMING_ORDER = ["ir.list", "ir.seq", "ir.inout.records", "ir.data", "ir.tuple", "ir.fixed", "ir.core"];

/** The level to name among those of `needed` that `have` lacks, or `undefined` when none is lacking. */
export function firstMissing(needed: Iterable<string>, have: ReadonlySet<string>): string | undefined {
  const lacking = new Set([...needed].filter((level) => !have.has(level)));
  for (const level of NAMING_ORDER) if (lacking.has(level)) return level;
  return [...lacking].sort()[0];
}
