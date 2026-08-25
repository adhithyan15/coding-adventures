// Tests for the Markdown document sharder (HL22).
//
// The load-bearing test in this file is `round-trips the REAL documents`. Every
// other test here is a fixture, and a fixture proves the code does what the
// fixture says — which is not the same as proving it does not lose a byte of a
// 6,200-line changelog. HL21 §8 step 5 says to assert the round trip against the
// real ledger, not only a fixture, and that instruction exists because a
// migration that silently drops content is the failure this whole convention is
// supposed to make impossible.

import { readFileSync, readdirSync } from "node:fs";
import { join, resolve } from "node:path";
import { describe, expect, it } from "vitest";
import {
  DOC_META_SHARD,
  type DocShardPlan,
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

// ---------------------------------------------------------------------------
// The test this file exists for.
// ---------------------------------------------------------------------------
describe("the real documents", () => {
  const root = defaultRepoRoot();

  for (const plan of DOC_SHARD_PLANS) {
    it(`round-trips the REAL ${plan.path} byte-for-byte`, () => {
      const committed = readFileSync(safeDocumentPath(root, plan.path), "utf8");
      // Both directions, against the file actually on disk:
      //   shards -> document   must reproduce the committed bytes
      //   document -> shards -> document  must be the identity
      expect(unshardDocContents(root, plan)).toBe(committed);
      expect(joinDocShards(docShardContents(committed, plan), plan)).toBe(committed);
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
      const committed = readFileSync(safeDocumentPath(root, plan.path), "utf8");
      const contents = docShardContents(committed, plan);
      const sections = splitDocument(committed, plan.headingLevel).sections.length;
      expect(contents.size).toBe(sections + 1); // +1 for _meta.md
    });
  }
});
