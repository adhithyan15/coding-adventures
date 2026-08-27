// Tests for the Markdown document sharder (HL22/HL23).
//
// The load-bearing test in this file is `round-trips the REAL shards`. Every
// other test here is a fixture, and a fixture proves the code does what the
// fixture says — which is not the same as proving it does not lose a byte of a
// 6,200-line changelog. HL21 §8 step 5 says to assert the round trip against the
// real ledger, not only a fixture, and that instruction exists because a
// migration that silently drops content is the failure this whole convention is
// supposed to make impossible.

import { mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { afterAll, describe, expect, it } from "vitest";
import {
  DOC_META_SHARD,
  type DocShardPlan,
  isAbsentErrno,
  isDocSharded,
  docShardContents,
  docShardDirectoryFor,
  docShardFilename,
  docSlug,
  headingDigest,
  joinDocShards,
  splitDocument,
} from "../src/doc-shard.js";
import {
  DOC_SHARD_PLANS,
  defaultRepoRoot,
  safeDocumentPath,
  unshardDocContents,
  unshardDocument,
} from "../src/doc-shard-cli.js";

const PLAN: DocShardPlan = { path: "x/DOC.md", headingLevel: 2, newestFirst: true };
const OLDEST_FIRST: DocShardPlan = { ...PLAN, newestFirst: false };

describe("docShardDirectoryFor", () => {
  it("maps X.md to X.d", () => {
    expect(docShardDirectoryFor("a/BACKLOG.md")).toBe("a/BACKLOG.d");
  });

  it("refuses a path that is not Markdown, rather than inventing book.tex.d", () => {
    expect(() => docShardDirectoryFor("a/book.tex")).toThrow(/not a .md document/);
  });
});

describe("splitDocument", () => {
  it("partitions the file exactly — preamble plus sections is the input", () => {
    const text = "# Title\n\nintro\n\n## A\n\nbody a\n\n## B\n\nbody b\n";
    const { preamble, sections } = splitDocument(text, 2);
    expect(preamble).toBe("# Title\n\nintro\n\n");
    expect(sections.map((s) => s.heading)).toEqual(["## A", "## B"]);
    expect(preamble + sections.map((s) => s.text).join("")).toBe(text);
  });

  it("preserves a file that does not end in a newline", () => {
    const text = "# T\n\n## A\n\nno trailing newline";
    const { preamble, sections } = splitDocument(text, 2);
    expect(preamble + sections.map((s) => s.text).join("")).toBe(text);
  });

  it("keeps sub-headings inside their parent section", () => {
    // BACKLOG.md has five `###` sub-headings living under `##` entries. Splitting
    // at level 2 must not notice them.
    const text = "# T\n\n## A\n\n### deeper\n\nx\n\n## B\n\ny\n";
    const { sections } = splitDocument(text, 2);
    expect(sections).toHaveLength(2);
    expect(sections[0].text).toContain("### deeper");
  });

  it("leaves a level-2 heading alone when splitting at level 3", () => {
    // How CHANGELOG.md's frozen `## [0.3.0]` version markers survive: they are
    // ordinary content of whichever entry precedes them.
    const text = "# C\n\n## Unreleased\n\n### one\n\na\n\n## [0.1.0]\n\n### two\n\nb\n";
    const { preamble, sections } = splitDocument(text, 3);
    expect(preamble).toBe("# C\n\n## Unreleased\n\n");
    expect(sections).toHaveLength(2);
    expect(sections[0].text).toContain("## [0.1.0]");
  });

  it("does NOT split on a heading inside a fenced code block", () => {
    // The one bug a byte-exact round trip cannot catch: a partition reassembles
    // no matter where it was cut, so cutting a code block in half still passes
    // `--check` while producing nonsense shards.
    const text = "# T\n\n## A\n\n```md\n## not a heading\n```\n\ntail\n\n## B\n\nb\n";
    const { sections } = splitDocument(text, 2);
    expect(sections.map((s) => s.heading)).toEqual(["## A", "## B"]);
    expect(sections[0].text).toContain("## not a heading");
  });

  it("treats a ``` inside a ~~~ block as content, not as a fence close", () => {
    const text = "# T\n\n## A\n\n~~~\n```\n## inner\n~~~\n\n## B\n\nb\n";
    const { sections } = splitDocument(text, 2);
    expect(sections.map((s) => s.heading)).toEqual(["## A", "## B"]);
  });

  it("handles a document whose FIRST line is a section heading", () => {
    // The empty-preamble case. Without the `to > from` guard in `lineRange`,
    // the empty range gained a newline and invented a blank line the file never
    // had — caught by the round-trip assertion, but as an unactionable
    // "internal error". Latent for both current plans, because both documents
    // open with an `#` title above their split level.
    const text = "## A\n\nx\n\n## B\n\ny\n";
    const { preamble, sections } = splitDocument(text, 2);
    expect(preamble).toBe("");
    expect(sections).toHaveLength(2);
    expect(preamble + sections.map((s) => s.text).join("")).toBe(text);
  });

  it("returns the whole document as preamble when there are no sections", () => {
    const { preamble, sections } = splitDocument("# T\n\njust prose\n", 2);
    expect(sections).toHaveLength(0);
    expect(preamble).toBe("# T\n\njust prose\n");
  });
});

describe("isDocSharded — absent versus UNKNOWN", () => {
  // A `--check` that says "missing" when it means "I could not read it" is a
  // gate that fails closed, intermittently, with a message that sends the reader
  // to look for a deleted directory. This block exists because the first version
  // did exactly that: `catch { return false }` collapsed every errno into
  // "absent", and a real run printed "BACKLOG.d is missing" and exited 1 with
  // 109 shards sitting in the directory.
  const tmp = mkdtempSync(join(tmpdir(), "doc-shard-"));

  afterAll(() => rmSync(tmp, { recursive: true, force: true }));

  it("returns false for a genuinely absent directory (the HL21 §2.3 fallback)", () => {
    expect(isDocSharded(join(tmp, "NOPE.md"))).toBe(false);
  });

  it("returns false for ENOTDIR — a parent component that is a file", () => {
    // `<file>/INNER.d` cannot exist, so "not sharded" is the correct answer
    // rather than a guess.
    const file = join(tmp, "plain.txt");
    writeFileSync(file, "x");
    expect(isDocSharded(join(file, "INNER.md"))).toBe(false);
  });

  it("REFUSES when something that is not a directory occupies the name", () => {
    // Previously returned false, so `--check` reported "missing" about a name
    // that was already taken — the reader would go and try to restore it.
    const doc = join(tmp, "SQUAT.md");
    writeFileSync(doc, "# x\n");
    writeFileSync(join(tmp, "SQUAT.d"), "not a directory");
    expect(() => isDocSharded(doc)).toThrow(/exists but is not a directory/);
  });

  it("CLASSIFIES every other errno as unknown, not as absent", () => {
    // The classification that caused the flaky gate, stated directly. EBUSY and
    // friends cannot be provoked portably, and `vi.spyOn` cannot patch a
    // `node:fs` export under ESM — the module namespace is not configurable. So
    // the decision is extracted as a pure predicate and pinned here, which is
    // also the honest thing to test: the bug was never in the syscall, it was in
    // what the code concluded from the syscall's failure.
    for (const absent of ["ENOENT", "ENOTDIR"]) {
      expect(isAbsentErrno(absent)).toBe(true);
    }
    for (const unknown of [
      "EBUSY",   // Windows: search indexer, antivirus, or a sync client holds it
      "EACCES",
      "EPERM",
      "EMFILE",  // a 102-file parallel test run genuinely reaches this
      "ENFILE",
      "EIO",
      "ELOOP",
      undefined, // an error with no `code` at all is still not "absent"
    ]) {
      expect(isAbsentErrno(unknown)).toBe(false);
    }
  });
});

describe("docSlug", () => {
  it("folds to uppercase ASCII with single hyphens", () => {
    expect(docSlug("## HL-C10E — Urdu closes the gap")).toBe("HL-C10E-URDU-CLOSES-THE-GAP");
  });

  it("drops non-ASCII, which is why it is NOT the shard's identity", () => {
    // Both real CHANGELOG headings. They fold to one slug; only the digest
    // separates them. A caller that deduplicated on the slug would lose one.
    expect(docSlug("### Added - source-verified Tamil ர")).toBe(
      docSlug("### Added - source-verified Tamil த"),
    );
    expect(headingDigest("### Added - source-verified Tamil ர")).not.toBe(
      headingDigest("### Added - source-verified Tamil த"),
    );
  });

  it("never emits a leading or trailing hyphen, even when the cap lands on one", () => {
    const slug = docSlug(`## ${"WORD ".repeat(40)}`);
    expect(slug.startsWith("-")).toBe(false);
    expect(slug.endsWith("-")).toBe(false);
    expect(slug.length).toBeLessThanOrEqual(60);
  });

  it("falls back to SECTION for a heading with no ASCII at all", () => {
    expect(docSlug("## 中文标题")).toBe("SECTION");
  });
});

describe("docShardFilename", () => {
  it("zero-pads so that string sort and numeric sort agree", () => {
    expect(docShardFilename(90, "A", "0f0f0f0f")).toBe("00090-A-0f0f0f0f.md");
    expect(docShardFilename(100, "A", "0f0f0f0f") > docShardFilename(90, "A", "0f0f0f0f")).toBe(true);
  });

  it("refuses to overflow the pad width rather than silently re-ordering", () => {
    // At six digits `100000` sorts before `10010`, so filename order stops
    // reproducing document order — and `--check` cannot see it, because both
    // directions use the same broken order.
    expect(() => docShardFilename(100000, "A", "0f0f0f0f")).toThrow(/outgrown the shard numbering/);
  });
});

describe("docShardContents", () => {
  const text = "# T\n\n## newest\n\na\n\n## middle\n\nb\n\n## oldest\n\nc\n";

  it("gives the TOP section the HIGHEST ordinal for a newest-first document", () => {
    // The whole reason this module exists. A prepend has to become an append in
    // ordinal space, or two agents both reach downward into a shrinking gap.
    const names = [...docShardContents(text, PLAN).keys()].filter((n) => n !== DOC_META_SHARD);
    expect(names[0].startsWith("00030-")).toBe(true); // "## newest"
    expect(names[2].startsWith("00010-")).toBe(true); // "## oldest"
  });

  it("numbers an oldest-first document the ordinary way round", () => {
    const names = [...docShardContents(text, OLDEST_FIRST).keys()].filter(
      (n) => n !== DOC_META_SHARD,
    );
    expect(names[0].startsWith("00010-")).toBe(true);
  });

  it("puts the preamble in _meta.md", () => {
    expect(docShardContents(text, PLAN).get(DOC_META_SHARD)).toBe("# T\n\n");
  });

  it("round-trips through joinDocShards", () => {
    expect(joinDocShards(docShardContents(text, PLAN), PLAN)).toBe(text);
    expect(joinDocShards(docShardContents(text, OLDEST_FIRST), OLDEST_FIRST)).toBe(text);
  });

  it("keeps two identical headings apart by ordinal", () => {
    // Real: CHANGELOG.md has two bare `### Added` headings. Same digest, same
    // slug, different rank — so different files, and neither overwrites the other.
    const twice = "# T\n\n### Added\n\na\n\n### Added\n\nb\n";
    const plan: DocShardPlan = { path: "x/DOC.md", headingLevel: 3, newestFirst: true };
    const contents = docShardContents(twice, plan);
    expect(contents.size).toBe(3);
    expect(joinDocShards(contents, plan)).toBe(twice);
  });

  it("refuses a document with no sections rather than making it all _meta", () => {
    expect(() => docShardContents("# T\n\nprose\n", PLAN)).toThrow(/no level-2 headings/);
  });
});

describe("joinDocShards", () => {
  it("requires _meta.md rather than defaulting the preamble to empty", () => {
    // A rebase that dropped it would otherwise read as a backlog that
    // legitimately has no title.
    const shards = new Map([["00010-A-0f0f0f0f.md", "## A\n"]]);
    expect(() => joinDocShards(shards, PLAN)).toThrow(/no '_meta\.md'/);
  });

  it("leads with _meta.md in BOTH directions, not by where '_' happens to sort", () => {
    // `_` is 0x5F, above every digit, so under ascending order it trails. Both
    // orders must still start with the preamble.
    const shards = new Map([
      [DOC_META_SHARD, "# T\n\n"],
      ["00010-OLD-0f0f0f0f.md", "## old\n"],
      ["00020-NEW-1f1f1f1f.md", "## new\n"],
    ]);
    expect(joinDocShards(shards, PLAN)).toBe("# T\n\n## new\n## old\n");
    expect(joinDocShards(shards, OLDEST_FIRST)).toBe("# T\n\n## old\n## new\n");
  });

  it("orders by code unit, not by locale", () => {
    // `localeCompare` under en-US folds case and ignores punctuation, so it can
    // reorder these between two developers' machines.
    const shards = new Map([
      [DOC_META_SHARD, ""],
      ["00010-A-00000000.md", "a"],
      ["00010-a-00000000.md", "b"],
    ]);
    // 'A' (0x41) < 'a' (0x61); newest-first reverses, so lowercase leads.
    expect(joinDocShards(shards, PLAN)).toBe("ba");
  });
});

describe("safeDocumentPath", () => {
  const root = defaultRepoRoot();

  it("accepts a plan path", () => {
    expect(() => safeDocumentPath(root, DOC_SHARD_PLANS[0].path)).not.toThrow();
  });

  it("refuses traversal that no leading '..' would reveal", () => {
    expect(() => safeDocumentPath(root, "code/a/../../../evil.md")).toThrow(/unsafe document path/);
  });

  it("refuses a drive-qualified path ON EVERY PLATFORM", () => {
    // `path.relative('C:/repo', 'D:/evil.md')` returns `'D:/evil.md'` on
    // Windows — not `..`-prefixed, so the lexical containment test passes it.
    // And `isAbsolute` alone cannot catch it, because on POSIX `D:\evil.md` is
    // an ordinary relative filename. `assertRelativeManifestPath` applies the
    // pattern everywhere, which is why this test can be unconditional.
    expect(() => safeDocumentPath(root, "D:\\evil.md")).toThrow(/must be relative/);
    expect(() => safeDocumentPath(root, "d:/evil.md")).toThrow(/must be relative/);
  });

  it("refuses a UNC path, which would turn a build step into an outbound write", () => {
    expect(() => safeDocumentPath(root, "\\\\server\\share\\evil.md")).toThrow(/must be relative/);
    expect(() => safeDocumentPath(root, "//server/share/evil.md")).toThrow(/must be relative/);
  });

  it("refuses an absolute path", () => {
    expect(() => safeDocumentPath(root, resolve(root, "a.md"))).toThrow(/must be relative/);
  });

  it("refuses a path that is not Markdown", () => {
    expect(() => safeDocumentPath(root, "code/a.json")).toThrow(/unsafe document path/);
  });
});

describe("unshardDocument — ignored local render", () => {
  const tmp = mkdtempSync(join(tmpdir(), "doc-shard-render-"));

  afterAll(() => rmSync(tmp, { recursive: true, force: true }));

  it("creates an absent rendered monolith from committed shards", () => {
    const dir = join(tmp, "x", "DOC.d");
    mkdirSync(dir, { recursive: true });
    writeFileSync(join(dir, DOC_META_SHARD), "# T\n\n");
    writeFileSync(join(dir, "00010-A-559aead0.md"), "## A\n\nbody\n");

    expect(unshardDocument(tmp, PLAN)).toBe("# T\n\n## A\n\nbody\n");
    expect(readFileSync(join(tmp, "x", "DOC.md"), "utf8")).toBe(
      "# T\n\n## A\n\nbody\n",
    );
  });
});

// ---------------------------------------------------------------------------
// ORDERING, verified independently of the round trip.
// ---------------------------------------------------------------------------
//
// A byte-identical round trip is NECESSARY BUT NOT SUFFICIENT, and HL21 §5.2 is
// the cautionary tale: it asserted `curriculum.json`'s `spine` needed no ordinal
// because "an object has no meaningful order". Bare-name shards would have
// re-sorted the shared ladder across 23 tracks — and a round-trip check run
// against a track whose keys happened to be ALREADY sorted would have passed
// while doing it.
//
// The trap is not that the round trip is a weak check. It is that verifying it
// on one instance generalises falsely. Both plans here are covered by the
// real-document tests below, which is 2 of 2 rather than 1 of 23 — but the
// numbering MECHANISM deserves its own tests, so the third plan somebody adds is
// covered before it exists.
describe("ordering", () => {
  /** A document of N sections, newest first, each heading distinct. */
  const doc = (n: number): string =>
    "# T\n\n" + Array.from({ length: n }, (_, i) => `## S${i}\n\nbody ${i}\n\n`).join("");

  const headingsOf = (text: string): string[] =>
    text.split("\n").filter((line) => line.startsWith("## "));

  it("zero-padding is LOAD-BEARING past ten sections", () => {
    // The "10 sorts before 2" bug. Eleven items is enough to expose it, which is
    // why this is not theoretical: both real documents are far past eleven, and
    // every one of the 20 chapter tracks hit exactly this.
    const text = doc(11);
    const names = [...docShardContents(text, PLAN).keys()].filter((n) => n !== DOC_META_SHARD);
    const asStrings = [...names].sort((a, b) => (a < b ? -1 : a > b ? 1 : 0));
    const asNumbers = [...names].sort((a, b) => Number(a.split("-")[0]) - Number(b.split("-")[0]));
    expect(asStrings).toEqual(asNumbers);
    expect(joinDocShards(docShardContents(text, PLAN), PLAN)).toBe(text);
  });

  it("proves the pad is doing the work — the same ranks UNPADDED mis-sort", () => {
    // The negative control. Without it, the test above also passes for a naming
    // scheme that never needed padding, and therefore proves nothing.
    const unpadded = Array.from({ length: 11 }, (_, i) => `${(i + 1) * 10}-S${i}.md`);
    const asStrings = [...unpadded].sort((a, b) => (a < b ? -1 : a > b ? 1 : 0));
    const asNumbers = [...unpadded].sort((a, b) => Number(a.split("-")[0]) - Number(b.split("-")[0]));
    expect(asStrings).not.toEqual(asNumbers);
    expect(asStrings[0]).toBe("10-S0.md");
    expect(asStrings[1]).toBe("100-S9.md"); // 100 before 20: the bug, live
  });

  it("a NEW TOP section lands at the top after regeneration", () => {
    // The operation every author performs, and the one the recency rank exists
    // for: add one file at max+stride, rename nothing.
    const text = doc(5);
    const shards = docShardContents(text, PLAN);
    const top = [...shards.keys()].filter((n) => n !== DOC_META_SHARD).sort().at(-1)!;
    const next = String(Number(top.split("-")[0]) + 10).padStart(5, "0");
    shards.set(`${next}-NEWEST-aaaaaaaa.md`, "## NEWEST\n\nbrand new\n\n");
    const rebuilt = joinDocShards(shards, PLAN);
    expect(headingsOf(rebuilt)[0]).toBe("## NEWEST");
    expect(headingsOf(rebuilt).slice(1)).toEqual(headingsOf(text));
  });

  it("a RANK COLLISION is deterministic and LOCALLY CONTAINED", () => {
    // Two parallel agents both compute max+stride and both write it. The pair's
    // relative order is then decided by the rest of the filename and is
    // arbitrary — two entries authored the same day have no true order to lose.
    //
    // What must NOT happen is a collision displacing a THIRD section. The padded
    // rank is a fixed-width prefix, so the colliding pair sorts as a block and
    // everything else keeps its place. That is the property that makes a tie
    // acceptable rather than a bug.
    const text = doc(4);
    const shards = docShardContents(text, PLAN);
    const top = [...shards.keys()].filter((n) => n !== DOC_META_SHARD).sort().at(-1)!;
    const next = String(Number(top.split("-")[0]) + 10).padStart(5, "0");
    shards.set(`${next}-AGENT-A-11111111.md`, "## AGENT-A\n\na\n\n");
    shards.set(`${next}-AGENT-B-22222222.md`, "## AGENT-B\n\nb\n\n");

    const headings = headingsOf(joinDocShards(shards, PLAN));
    expect(headings.slice(0, 2).sort()).toEqual(["## AGENT-A", "## AGENT-B"]);
    expect(headings.slice(2)).toEqual(headingsOf(text));

    // Deterministic: Map insertion order must not leak into the result.
    const reversed = new Map([...shards.entries()].reverse());
    expect(joinDocShards(reversed, PLAN)).toBe(joinDocShards(shards, PLAN));
  });

  it("REFUSES rather than silently re-ordering when the rank space is exhausted", () => {
    // 99999 is the last five-digit rank. At 100000 the string sorts BEFORE
    // 10010, so filename order stops reproducing document order — and `--check`
    // cannot see it, because both directions use the same broken order.
    expect(() => docShardFilename(99990, "A", "0f0f0f0f")).not.toThrow();
    expect(() => docShardFilename(100000, "A", "0f0f0f0f")).toThrow(/outgrown/);
    // The boundary in the terms an author meets it: 9,999 sections at stride 10.
    expect(() => docShardFilename(9999 * 10, "A", "0f0f0f0f")).not.toThrow();
    expect(() => docShardFilename(10000 * 10, "A", "0f0f0f0f")).toThrow(/outgrown/);
  });

  it("ORDER ORACLE — sorted shard order is the exact reverse of document order", () => {
    // Stated as a permutation rather than as a byte comparison, so it fails for
    // an ORDERING reason with an ordering message. This is the assertion HL21
    // §5.2 needed and did not have.
    const text = doc(30);
    const names = [...docShardContents(text, PLAN).keys()]
      .filter((n) => n !== DOC_META_SHARD)
      .sort((a, b) => (a < b ? -1 : a > b ? 1 : 0));
    const documentOrder = headingsOf(text).map((h) => h.slice(3));
    const shardOrder = names.map((n) => n.split("-")[1]);
    expect(shardOrder).toEqual([...documentOrder].reverse());
  });
});

// ---------------------------------------------------------------------------
// The test this file exists for.
// ---------------------------------------------------------------------------
describe("the real documents", () => {
  const root = defaultRepoRoot();

  for (const plan of DOC_SHARD_PLANS) {
    it(`round-trips the REAL shards for ${plan.path} byte-for-byte`, () => {
      const rendered = unshardDocContents(root, plan);
      expect(joinDocShards(docShardContents(rendered, plan), plan)).toBe(rendered);
    });

    it(`${plan.path}: shard order on disk reproduces rendered section order`, () => {
      // The ordering claim, asserted against the committed shard directory and
      // stated independently of the byte comparison above. Reading the headings
      // out of the shard FILES and out of the MONOLITH by two separate paths and
      // comparing the sequences fails with an ordering message when the ordering
      // is what broke — which is the diagnostic the byte comparison cannot give.
      const monolith = safeDocumentPath(root, plan.path);
      const rendered = unshardDocContents(root, plan);
      const level = "#".repeat(plan.headingLevel) + " ";

      const fromMonolith = rendered
        .split("\n")
        .filter((line) => line.startsWith(level) && !line.startsWith(level + "#"));

      const dir = docShardDirectoryFor(monolith);
      const names = readdirSync(dir)
        .filter((n) => n.endsWith(".md") && n !== DOC_META_SHARD)
        .sort((a, b) => (a < b ? -1 : a > b ? 1 : 0));
      if (plan.newestFirst) names.reverse();
      const fromShards = names.map(
        (n) => readFileSync(join(dir, n), "utf8").split("\n")[0],
      );

      expect(fromShards).toEqual(fromMonolith);
      expect(fromShards.length).toBeGreaterThan(100); // every document is well past the "10 < 2" threshold
    });

    it(`${plan.path}: every file in the shard directory is a *.md shard`, () => {
      // A `.md.orig` left by a botched merge would sit in the directory looking
      // like content and contribute nothing, and the document would still
      // rebuild cleanly — which is exactly why nobody would notice.
      const dir = docShardDirectoryFor(safeDocumentPath(root, plan.path));
      const stray = readdirSync(dir, { withFileTypes: true })
        .filter((entry) => !entry.isDirectory() && !entry.name.endsWith(".md"))
        .map((entry) => join(dir, entry.name));
      expect(stray).toEqual([]);
    });

    it(`${plan.path}: no two shards share a filename after a fresh --shard`, () => {
      // `docShardContents` throws on a collision, so reaching the assertion at
      // all is most of the proof; the count check catches a silent overwrite if
      // that guard were ever weakened.
      const rendered = unshardDocContents(root, plan);
      const contents = docShardContents(rendered, plan);
      const sections = splitDocument(rendered, plan.headingLevel).sections.length;
      expect(contents.size).toBe(sections + 1); // +1 for _meta.md
    });
  }
});
