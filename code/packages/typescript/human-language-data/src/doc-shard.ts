// doc-shard.ts — the HL21 `X.d/` convention, applied to Markdown DOCUMENTS.
//
// ---------------------------------------------------------------------------
// Why a second sharder
// ---------------------------------------------------------------------------
//
// `shard.ts` shards JSON LEDGERS: parse, split an array, re-serialize. That is
// the right shape for `core/spine.json`, and the wrong shape for prose. A
// Markdown document has no array to split and no canonical serialization to
// round-trip through — `JSON.stringify` has one true output for a given value,
// but there is no such thing for a paragraph.
//
// So this module never re-serializes anything. It PARTITIONS the file's bytes
// at heading boundaries and writes each partition out verbatim. Rebuilding is
// string concatenation. The round trip is byte-exact by CONSTRUCTION rather
// than by verification: `preamble + sections.join("")` is the original file,
// for the same reason that cutting a rope and taping it back together gives you
// the rope.
//
// That is a genuinely stronger guarantee than the JSON path has, and it is the
// reason this is a sibling module rather than a generalisation of `shard.ts`.
// Trying to make one module serve both would mean a `serialize` hook that is
// the identity function on one side and `JSON.stringify` on the other, which is
// two modules wearing one name.
//
// ---------------------------------------------------------------------------
// The two files this exists for, and why they are the last two
// ---------------------------------------------------------------------------
//
// Of the last 200 human-languages commits on `main`:
//
//     code/learning/human-languages/BACKLOG.md                touched by 100
//     code/packages/typescript/human-language-data/CHANGELOG.md  touched by 75
//     <track>/CHANGELOG.md (per language)                     touched by 4-11
//
// The per-language changelogs are already partitioned BY TRACK and have never
// been a conflict point; they are deliberately left alone. These two are the
// files every human-languages author touches, whatever they are working on, and
// they are therefore the two that serialize 5-7 parallel level-authoring agents
// down to one.
//
// PR #12690 is the corroborating experiment. It went DIRTY three separate
// times, and every single time the conflict was ONLY `CHANGELOG.md`: not one of
// its ~4,000 shard files conflicted, across two concurrent Spanish tranches.
// The sharding pattern works. These two files are the part that had not had it
// applied yet.
//
// ---------------------------------------------------------------------------
// The hard part: these documents are NEWEST-FIRST
// ---------------------------------------------------------------------------
//
// HL21 §2.2 says order that carries meaning must live in the filename, and its
// worked examples all APPEND: a new spine node or a new chapter goes at the end,
// so it takes the next ordinal and nobody renames anything.
//
// A backlog and a changelog do the opposite. A new entry goes at the TOP. Under
// plain ascending filename order the newest entry would need the SMALLEST
// ordinal, so every author would be reaching downward into a shrinking gap —
// two agents would both compute "one less than the current minimum", and the
// numbering would run out at zero.
//
// The fix is to stop insisting the join is ascending. The rule that actually
// matters in HL21 §2.1 is that the order be DETERMINISTIC and locale-free, not
// that it be ascending; descending code-unit order is exactly as reproducible
// as ascending. So for a newest-first document the ordinal is a RECENCY RANK:
//
//     highest ordinal  = topmost section  = newest
//     lowest ordinal   = bottom section   = oldest
//
// and the join walks it downward. Adding an entry is then the ordinary append
// case again — `max + stride`, no neighbour renamed, no gap to exhaust.
//
// ---------------------------------------------------------------------------
// Why two parallel agents cannot pick the same filename
// ---------------------------------------------------------------------------
//
// A shard filename is three parts:
//
//     4520-ADDED-SOURCE-VERIFIED-TAMIL-3f9c2a1b.md
//     ^^^^ ^-------------------------^ ^------^
//     rank  human-readable slug         heading digest
//
// Each part earns its place:
//
//   * The RANK orders the join. Two agents appending at the same moment will
//     both compute the same rank, and that is fine — a tie is broken by the
//     rest of the name, and two entries authored the same day have no true
//     relative order to lose.
//
//   * The SLUG is for the human running `ls`. It is ASCII-folded, which means
//     it is NOT unique: `### Added - source-verified Tamil ர` and
//     `### Added - source-verified Tamil த` both fold to
//     `ADDED-SOURCE-VERIFIED-TAMIL`, because the only thing distinguishing them
//     is a character the filename cannot safely carry on every filesystem this
//     repo is cloned onto. That is why the slug alone is not the identity.
//
//   * The DIGEST is the identity: the first 8 hex of SHA-256 over the raw
//     heading line. Two different headings give two different filenames, full
//     stop — which is the property the whole exercise needs, and the one the
//     slug cannot provide. Two agents collide only if they independently write
//     the *identical heading text* at the *identical rank*, which is not a merge
//     problem at that point but a duplicated entry.
//
// The digest is over the HEADING, not the section body, and that is deliberate.
// Hashing the body would make every prose edit rename its own file, so the next
// `--shard` would emit a mass rename — and a mass rename is a mass merge
// conflict, which is the thing this work exists to remove. Headings change
// rarely; bodies change constantly.
//
// ---------------------------------------------------------------------------
// Fenced code blocks
// ---------------------------------------------------------------------------
//
// `^## ` inside a fenced code block is not a heading, it is a shell prompt or a
// diff hunk or a Markdown example. Splitting on it would cut a code block in
// half, and the two halves would still concatenate back to the original file —
// so the round-trip check would PASS while the shards were nonsense.
//
// That is the dangerous class of bug here: a partition is byte-exact no matter
// where you cut it. Byte-exactness cannot tell you the cut was in a sensible
// place. So the splitter tracks fence state, and it is the one piece of this
// module that a `--check` would never catch on its own.
//
// `BACKLOG.md` has 5 fenced blocks today and none of them contains a `## ` line;
// `CHANGELOG.md` has none at all. Both were measured, not assumed. The guard is
// here for the block somebody adds next week.

import { createHash } from "node:crypto";
import { lstatSync, readFileSync, readdirSync } from "node:fs";
import { basename, join } from "node:path";

/** The suffix that turns a document path into its shard directory. */
export const DOC_SHARD_DIR_SUFFIX = ".d";

/**
 * The filename that carries everything above the first section.
 *
 * `.md` rather than `.json` so an editor opens it as prose, and a leading
 * underscore for the HL21 §2.4 reason: `_` is 0x5F, above every digit and every
 * uppercase letter, so it sorts away from the section shards under code-unit
 * order and reads as "not one of the things" to anyone listing the directory.
 *
 * It is found BY NAME rather than by position, because this module joins
 * descending for newest-first documents and the preamble must lead either way.
 */
export const DOC_META_SHARD = "_meta.md";

/** How one Markdown document splits. */
export interface DocShardPlan {
  /** Document path relative to the repository root, POSIX-separated. */
  readonly path: string;
  /**
   * The ATX heading level that starts a section: 2 for `## `, 3 for `### `.
   *
   * Everything above the first heading at this level becomes `_meta.md`.
   * Headings at other levels are ordinary content inside whichever section they
   * fall in — which is what keeps `BACKLOG.md`'s five `###` sub-headings with
   * their parents, and what parks `CHANGELOG.md`'s frozen `## [0.2.0]` version
   * markers inside the entry above them rather than inventing a second axis.
   */
  readonly headingLevel: 2 | 3;
  /**
   * True when the document is written newest-at-the-top.
   *
   * Both documents here are. See the header: this is what turns the ordinal
   * into a recency rank and turns a prepend back into an append.
   */
  readonly newestFirst: boolean;
}

/** One section of a document, as an exact slice of the original bytes. */
export interface DocSection {
  /** The raw heading line, without its newline. The digest's input. */
  readonly heading: string;
  /**
   * Heading line through the byte before the next heading, verbatim.
   *
   * Includes its own trailing newline(s). Concatenating every section's `text`
   * after the preamble reproduces the file exactly — that is the invariant this
   * whole module rests on, and `splitDocument`'s caller asserts it.
   */
  readonly text: string;
}

/** A document split into the part nobody appends to and the parts they do. */
export interface SplitDocument {
  /** Everything above the first section heading, verbatim. May be empty. */
  readonly preamble: string;
  /** The sections, in DOCUMENT order — top first, whatever that means. */
  readonly sections: readonly DocSection[];
}

/**
 * `.../BACKLOG.md` -> `.../BACKLOG.d`.
 *
 * Throws on anything not ending in `.md`, for the reason `shardDirectoryFor`
 * gives: appending `.d` to whatever it was handed would happily produce
 * `book.tex.d` and then spend an afternoon of somebody's time explaining why it
 * is empty.
 */
export function docShardDirectoryFor(documentPath: string): string {
  if (!documentPath.endsWith(".md")) {
    throw new Error(`doc-shard: '${documentPath}' is not a .md document, so it has no .d directory`);
  }
  return `${documentPath.slice(0, -".md".length)}${DOC_SHARD_DIR_SUFFIX}`;
}

/**
 * True when the sharded form of this document is the one on disk.
 *
 * `lstatSync`, deliberately, and not `existsSync` + `statSync` — the same call
 * and the same reasoning as `shard.ts`'s `isSharded`, and it is repeated rather
 * than imported because the two modules disagree about the file extension and
 * sharing the function would mean threading the suffix through as a parameter
 * to save four lines.
 *
 * `statSync` FOLLOWS symlinks, and git tracks symlinks as first-class objects,
 * so a pull request can commit `BACKLOG.d` as a link to `~/.aws` and have this
 * loader merge whatever it finds there into a file the next `--unshard` writes
 * back into the tree. Reading past a symlink is a disclosure; writing past one
 * is worse. A link here is refused loudly rather than followed, and rather than
 * silently falling back to the monolith — a quiet fallback would hide it.
 */
export function isDocSharded(documentPath: string): boolean {
  const dir = docShardDirectoryFor(documentPath);
  let stat;
  try {
    stat = lstatSync(dir);
  } catch {
    return false;
  }
  if (stat.isSymbolicLink()) {
    throw new Error(
      `doc-shard: '${dir}' is a symbolic link — a shard directory must be a real ` +
        `directory beside its document, so that reads cannot leave the checkout`,
    );
  }
  return stat.isDirectory();
}

/**
 * The shard filenames of `X.d/`, in the one order every machine agrees on.
 *
 * Ascending code-unit order — `a < b` on the raw string, never `localeCompare`,
 * which consults the host's collation and would let `_meta.md` and `0010-A.md`
 * swap places between two developers' machines. The caller reverses this list
 * for a newest-first document; it does not ask the filesystem for a different
 * order, because `readdirSync` returns whatever NTFS or APFS or ext4 happens to
 * hand back and that is not an order at all.
 *
 * A `*.md` entry that is a SYMLINK is refused rather than skipped: `isFile()` is
 * false for a symlink, so such an entry would otherwise vanish from the merge in
 * silence, and a shard that disappears without a word is worse than one that
 * fails — the result still looks like a complete document.
 */
export function listDocShardNames(documentPath: string): string[] {
  const dir = docShardDirectoryFor(documentPath);
  const entries = readdirSync(dir, { withFileTypes: true });
  for (const entry of entries) {
    if (entry.name.endsWith(".md") && entry.isSymbolicLink()) {
      throw new Error(
        `doc shard '${entry.name}' in '${dir}': is a symbolic link — ` +
          `a shard must be a real file inside its shard directory`,
      );
    }
  }
  return entries
    .filter((entry) => entry.isFile() && entry.name.endsWith(".md"))
    .map((entry) => entry.name)
    .sort((a, b) => (a < b ? -1 : a > b ? 1 : 0));
}

/**
 * Refuse a path that is not a real file in the tree.
 *
 * The `.md` twin of `shard.ts`'s `assertRealFile`, and it exists for the lesson
 * recorded there: a guard that lives only in the reader is a guard the writer
 * forgets, and the writer is the dangerous one. `open(2)` with `O_WRONLY|O_TRUNC`
 * follows symlinks, so a `CHANGELOG.md` committed as a link would have its
 * TARGET truncated and overwritten by `--unshard`.
 */
export function assertRealDocFile(path: string, what = "document"): void {
  let stat;
  try {
    stat = lstatSync(path);
  } catch (cause) {
    throw new Error(`'${path}': cannot be read — ${describe(cause)}`, { cause });
  }
  if (stat.isSymbolicLink()) {
    throw new Error(`'${path}' is a symbolic link — a ${what} must be a real file in the tree`);
  }
  if (!stat.isFile()) {
    throw new Error(`'${path}' is not a regular file`);
  }
}

/** Read one document with every guard applied. The single door for a monolith. */
export function readDocumentFile(path: string): string {
  assertRealDocFile(path);
  try {
    return readFileSync(path, "utf8");
  } catch (cause) {
    throw new Error(`'${path}': cannot be read — ${describe(cause)}`, { cause });
  }
}

/** Read every shard body of `X.d/`, keyed by filename, or `null` if not sharded. */
export function readDocShards(documentPath: string): Map<string, string> | null {
  if (!isDocSharded(documentPath)) return null;
  const dir = docShardDirectoryFor(documentPath);
  const names = listDocShardNames(documentPath);
  if (names.length === 0) {
    // "No backlog on disk" and "a backlog with no entries" are opposite facts,
    // and a loader that returns the second when it means the first hands the
    // `--check` a clean bill of health for a document that is not there.
    // `loadModalityManifest` and `readShards` both already make this call.
    throw new Error(
      `doc-shard: '${dir}' exists but holds no *.md shards — ` +
        `an empty shard directory is a broken checkout, not an empty document. ` +
        `Delete the directory to fall back to '${basename(documentPath)}', or restore its shards.`,
    );
  }
  const out = new Map<string, string>();
  for (const name of names) {
    const path = join(dir, name);
    try {
      out.set(name, readFileSync(path, "utf8"));
    } catch (cause) {
      throw new Error(`doc shard '${name}' in '${dir}': cannot be read — ${describe(cause)}`, {
        cause,
      });
    }
  }
  return out;
}

/** The ATX heading matcher for one level: exactly N hashes, then a space. */
function headingPattern(level: 2 | 3): RegExp {
  return new RegExp(`^#{${level}} `);
}

/**
 * Split a document into its preamble and its sections.
 *
 * The invariant, asserted at the end rather than trusted: the pieces
 * concatenate back to the input. Everything downstream depends on it, and it is
 * one line to check.
 *
 * Fence tracking is the subtle part. A fence opens on ``` or ~~~ and closes on
 * the same character; a ``` inside a ~~~ block is content, not a close. Only the
 * opening character is tracked, not the run length, which is a simplification:
 * CommonMark also requires the closing run be at least as long as the opening
 * one. That difference can only ever cause this splitter to consider MORE of the
 * file to be code than a strict parser would, i.e. to split less eagerly, which
 * fails toward "a section is larger than it needed to be" rather than toward "a
 * code block was cut in half".
 */
export function splitDocument(text: string, level: 2 | 3): SplitDocument {
  const isHeading = headingPattern(level);
  const lines = text.split("\n");
  const starts: number[] = [];
  let fence: string | null = null;

  lines.forEach((line, index) => {
    const opener = line.match(/^\s{0,3}(`{3,}|~{3,})/);
    if (opener) {
      const char = opener[1][0];
      if (fence === null) fence = char;
      else if (fence === char) fence = null;
      return;
    }
    if (fence === null && isHeading.test(line)) starts.push(index);
  });

  // `split("\n")` drops the separators, so rebuilding a line range means
  // re-adding exactly one newline between lines and none after the last — which
  // `slice().join("\n")` does, provided the final chunk keeps whatever the file
  // ended with. Working in line indices and rejoining is why the trailing
  // newline, or its absence, survives untouched.
  const lineRange = (from: number, to: number): string =>
    lines.slice(from, to).join("\n") + (to < lines.length ? "\n" : "");

  const preamble = starts.length === 0 ? text : lineRange(0, starts[0]);
  const sections: DocSection[] = starts.map((start, i) => ({
    heading: lines[start],
    text: lineRange(start, i + 1 < starts.length ? starts[i + 1] : lines.length),
  }));

  const rebuilt = preamble + sections.map((section) => section.text).join("");
  if (rebuilt !== text) {
    // Unreachable by construction. It is asserted anyway because the cost of
    // being wrong is a silently mangled document, and the cost of the check is a
    // string comparison on a file measured in kilobytes.
    throw new Error(
      "doc-shard: internal error — splitting the document did not reproduce it. " +
        "Refusing to write shards that would lose bytes.",
    );
  }
  return { preamble, sections };
}

/**
 * The human-readable middle of a shard filename.
 *
 * ASCII letters and digits survive, upper-cased; every other run of characters
 * — spaces, em dashes, Tamil, Arabic, punctuation — collapses to a single
 * hyphen. Leading and trailing hyphens are trimmed, and the result is capped so
 * one long heading cannot push a path past the limits that still exist on
 * Windows.
 *
 * This is NOT the identity of the shard and must never be treated as one. See
 * the header: `Tamil ர` and `Tamil த` fold to the same slug, and the digest is
 * what tells them apart. A caller that deduplicates on the slug instead of the
 * filename would silently drop half this changelog.
 */
export function docSlug(heading: string): string {
  const folded = heading
    .replace(/[^A-Za-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "")
    .toUpperCase()
    .slice(0, SLUG_MAX);
  // Re-trim: the cap can land mid-separator and leave a trailing hyphen, which
  // would make `A-B-` and `A-B` two spellings of one idea.
  const trimmed = folded.replace(/-+$/g, "");
  // A heading of nothing but non-ASCII — a Chinese or Arabic section title — has
  // no slug at all. `SECTION` keeps the filename well-formed and lets the digest
  // carry the identity, which it was already doing.
  return trimmed === "" ? "SECTION" : trimmed;
}

const SLUG_MAX = 60;

/**
 * The eight hex digits that make a shard filename an identity.
 *
 * SHA-256 truncated to 32 bits. This is a DISAMBIGUATOR, not a security
 * control: nothing here is defended by the difficulty of finding a collision,
 * and `docShardContents` refuses a duplicate filename outright, so the worst a
 * collision can do is stop a `--shard` run with a message naming both headings.
 * Eight characters keeps the filename readable; a full digest would bury the
 * slug that makes the directory browsable in the first place.
 */
export function headingDigest(heading: string): string {
  return createHash("sha256").update(heading, "utf8").digest("hex").slice(0, 8);
}

/** Ordinal stride and pad width. Spaced by ten so an entry can be wedged in. */
const ORDINAL_STRIDE = 10;
const ORDINAL_WIDTH = 5;

/**
 * `4520-ADDED-SOURCE-VERIFIED-TAMIL-3f9c2a1b.md`
 *
 * Throws rather than overflowing the pad width, and this is the whole reason the
 * check exists: once the ordinal needs a sixth digit, `100000` sorts BEFORE
 * `10010`, so filename order silently stops reproducing document order.
 * `--check` cannot catch that — both directions use the same broken order, so
 * the round trip still closes — and the result is a re-ordered changelog nobody
 * sees.
 *
 * Five digits rather than the four `shard-cli` uses, because these documents are
 * bigger and grow faster: `CHANGELOG.md` is at 436 sections and took 75 of the
 * last 200 commits, so four digits would have run out around 999 entries, which
 * is close enough to be somebody's problem rather than nobody's.
 */
export function docShardFilename(ordinal: number, slug: string, digest: string): string {
  const padded = String(ordinal).padStart(ORDINAL_WIDTH, "0");
  if (padded.length > ORDINAL_WIDTH) {
    throw new Error(
      `doc-shard: ordinal ${ordinal} does not fit ${ORDINAL_WIDTH} digits — ` +
        `this document has outgrown the shard numbering. Widen ORDINAL_WIDTH and ` +
        `re-run --shard for every plan, in one commit, when no branch is in flight.`,
    );
  }
  return `${padded}-${slug}-${digest}.md`;
}

/**
 * Windows reserved device names, refused as the whole stem of a filename.
 *
 * `CON.md` cannot be checked out on Windows at all, so a shard set containing
 * one would silently fail on half the machines that use this repo. The slug is
 * only ever one part of a longer name here — `00010-CON-3f9c2a1b.md` is fine,
 * because the reservation applies to the stem before the first dot — but the
 * check is kept because the naming scheme is the kind of thing a later change
 * simplifies, and this is the failure it would reintroduce.
 */
const WINDOWS_RESERVED = new Set([
  "CON", "PRN", "AUX", "NUL",
  "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9",
  "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
]);

/** Filenames that are safe on every filesystem this repo is cloned onto. */
const SAFE_SHARD_NAME = /^[0-9]{5}-[A-Z0-9][A-Z0-9-]*-[0-9a-f]{8}\.md$/;

/**
 * The shard files a document would produce, as a filename -> contents map.
 *
 * Pure: it computes bytes and touches no disk, so `--shard` and `--check` share
 * one definition of what the shards ARE and cannot disagree about it.
 *
 * The ordinal is a RECENCY RANK for a newest-first document — the topmost
 * section gets the highest number — so that adding an entry at the top is an
 * append in ordinal space. See the header for why that is not a stylistic
 * choice.
 */
export function docShardContents(text: string, plan: DocShardPlan): Map<string, string> {
  const { preamble, sections } = splitDocument(text, plan.headingLevel);
  if (sections.length === 0) {
    throw new Error(
      `${plan.path}: no level-${plan.headingLevel} headings to shard on — ` +
        `the whole document would become '${DOC_META_SHARD}'`,
    );
  }
  const out = new Map<string, string>();
  out.set(DOC_META_SHARD, preamble);
  sections.forEach((section, index) => {
    const rank = plan.newestFirst ? sections.length - index : index + 1;
    const name = docShardFilename(rank * ORDINAL_STRIDE, docSlug(section.heading), headingDigest(section.heading));
    if (!SAFE_SHARD_NAME.test(name)) {
      throw new Error(`${plan.path}: section ${JSON.stringify(section.heading)} produced unsafe shard name '${name}'`);
    }
    if (WINDOWS_RESERVED.has(name.split(".")[0].toUpperCase())) {
      throw new Error(`${plan.path}: shard name '${name}' is a Windows reserved device name`);
    }
    if (out.has(name)) {
      // Two sections with one filename would produce one file, and the second
      // would overwrite the first — a silent loss of an entry, discovered later
      // by whoever notices the count is wrong. It takes two identical heading
      // lines at one rank to get here, which is a duplicated entry rather than a
      // naming problem, so the message names the heading rather than the file.
      throw new Error(
        `${plan.path}: duplicate section heading ${JSON.stringify(section.heading)} ` +
          `at the same rank — both would be written to '${name}'`,
      );
    }
    out.set(name, section.text);
  });
  return out;
}

/**
 * The document bytes that a set of shards currently means.
 *
 * `_meta.md` first, then the section shards. Ascending filename order for an
 * oldest-first document, DESCENDING for a newest-first one — the ordinal is a
 * recency rank, so walking it downward walks the document top to bottom.
 *
 * `_meta.md` is found by name and removed from the ordered list rather than
 * relying on where `_` happens to sort, because it must lead in both directions
 * and `_` (0x5F) sorts ABOVE every digit — so under ascending order it would
 * trail, and under descending order it would lead by luck rather than by rule.
 *
 * It is required, never defaulted to `""`. A rebase that dropped it would
 * otherwise read as a backlog that legitimately has no title.
 */
export function joinDocShards(shards: Map<string, string>, plan: DocShardPlan): string {
  const preamble = shards.get(DOC_META_SHARD);
  if (preamble === undefined) {
    throw new Error(
      `${plan.path}: no '${DOC_META_SHARD}' among ${shards.size} shard(s) — ` +
        `the document's preamble has no home`,
    );
  }
  const names = [...shards.keys()].filter((name) => name !== DOC_META_SHARD);
  names.sort((a, b) => (a < b ? -1 : a > b ? 1 : 0));
  if (plan.newestFirst) names.reverse();
  return preamble + names.map((name) => shards.get(name)!).join("");
}

/** `unknown` from a `catch` reduced to something printable. */
function describe(cause: unknown): string {
  return cause instanceof Error ? cause.message : String(cause);
}
