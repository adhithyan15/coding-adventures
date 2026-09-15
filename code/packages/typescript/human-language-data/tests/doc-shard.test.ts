// Tests for the Markdown document sharder (HL22/HL23).
//
// The load-bearing test in this file is `round-trips the REAL shards`. Every
// other test here is a fixture, and a fixture proves the code does what the
// fixture says — which is not the same as proving it does not lose a byte of a
// 6,200-line changelog. HL21 §8 step 5 says to assert the round trip against the
// real ledger, not only a fixture, and that instruction exists because a
// migration that silently drops content is the failure this whole convention is
// supposed to make impossible.

import {
  lstatSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  readdirSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { createHash } from "node:crypto";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { afterAll, describe, expect, it } from "vitest";
import {
  DOC_META_SHARD,
  type DocShardPlan,
  isAbsentErrno,
  isDocSharded,
  isValidDocShardName,
  docShardContents,
  docShardDirectoryFor,
  docSplitAt,
  docShardFilename,
  docSlug,
  headingDigest,
  joinDocShards,
  readDocShards,
  splitDocument,
} from "../src/doc-shard.js";
import {
  DOC_SHARD_PLANS,
  defaultRepoRoot,
  runDocShardCli,
  safeDocumentPath,
  shardDocument,
  unshardDocContents,
  unshardDocument,
} from "../src/doc-shard-cli.js";
import { LEGACY_DOC_SHARD_SHA256 } from "../src/doc-shard-legacy.js";

const PLAN: DocShardPlan = { path: "x/DOC.md", headingLevel: 2, newestFirst: true };
const OLDEST_FIRST: DocShardPlan = { ...PLAN, newestFirst: false };
const DUCTUS_CHANGELOG = "code/packages/typescript/script-ductus/CHANGELOG.md";
const HINDI_CHANGELOG = "code/learning/human-languages/hindi/CHANGELOG.md";
const HINDI_PLAN: DocShardPlan = {
  path: HINDI_CHANGELOG,
  headingLevel: 2,
  newestFirst: true,
};
const HINDI_FORWARD_FRAGMENT =
  "00250-UNRELEASED-HINDI-CHANGELOG-AUTHORING-IS-SHARDED-6788c56d.md";
const HINDI_MIGRATION_MAX_RANK = 240;
const DUCTUS_PLAN: DocShardPlan = {
  path: DUCTUS_CHANGELOG,
  headingLevel: 3,
  newestFirst: true,
};
const DUCTUS_FORWARD_FRAGMENT =
  "01625-CHANGED-SHARD-NATIVE-SCRIPT-INVENTORIES-9fa3a043.md";
const DUCTUS_MIGRATION_MAX_RANK = 1_630;

describe("Hindi changelog ownership", () => {
  it("registers the changelog as a fixed newest-first level-2 shard plan", () => {
    expect(DOC_SHARD_PLANS.find((plan) => plan.path === HINDI_CHANGELOG)).toEqual(
      HINDI_PLAN,
    );
  });

  it("keeps the generated monolith absent from a clean checkout", () => {
    const monolith = safeDocumentPath(defaultRepoRoot(), HINDI_CHANGELOG);
    let cause: unknown;
    try {
      lstatSync(monolith);
    } catch (error) {
      cause = error;
    }
    expect(isAbsentErrno((cause as NodeJS.ErrnoException | undefined)?.code)).toBe(true);
  });

  it("preserves the pre-migration Hindi history byte-for-byte", () => {
    const monolith = safeDocumentPath(defaultRepoRoot(), HINDI_CHANGELOG);
    const shards = readDocShards(monolith, HINDI_PLAN);
    expect(shards).not.toBeNull();

    const historical = new Map(
      [...shards!].filter(
        ([name]) =>
          name !== HINDI_FORWARD_FRAGMENT &&
          (name === DOC_META_SHARD || Number(name.slice(0, 5)) <= HINDI_MIGRATION_MAX_RANK),
      ),
    );
    const rendered = joinDocShards(historical, HINDI_PLAN);

    expect(Buffer.byteLength(rendered)).toBe(85_282);
    expect(splitDocument(rendered, 2).sections).toHaveLength(24);
    expect(createHash("sha256").update(rendered).digest("hex")).toBe(
      "ffb767831fd61e6e6d5ca7f79f61a06516b128e1169a08b8a7e95989b91b6590",
    );
  });
});

describe("HL26 Script Ductus changelog ownership", () => {
  it("registers the changelog as a fixed newest-first level-3 shard plan", () => {
    expect(DOC_SHARD_PLANS.find((plan) => plan.path === DUCTUS_CHANGELOG)).toEqual(DUCTUS_PLAN);
  });

  it("keeps the generated monolith absent from a clean checkout", () => {
    const monolith = safeDocumentPath(defaultRepoRoot(), DUCTUS_CHANGELOG);
    let cause: unknown;
    try {
      lstatSync(monolith);
    } catch (error) {
      cause = error;
    }
    expect(isAbsentErrno((cause as NodeJS.ErrnoException | undefined)?.code)).toBe(true);
  });

  it("preserves the fresh-main monolith byte-for-byte beside the forward fragment", () => {
    const monolith = safeDocumentPath(defaultRepoRoot(), DUCTUS_CHANGELOG);
    const shards = readDocShards(monolith, DUCTUS_PLAN);
    expect(shards).not.toBeNull();

    const historical = new Map(
      [...shards!].filter(
        ([name]) =>
          name !== DUCTUS_FORWARD_FRAGMENT &&
          (name === DOC_META_SHARD || Number(name.slice(0, 5)) <= DUCTUS_MIGRATION_MAX_RANK),
      ),
    );
    const rendered = joinDocShards(historical, DUCTUS_PLAN);

    expect(Buffer.byteLength(rendered)).toBe(59_966);
    expect(splitDocument(rendered, 3).sections).toHaveLength(163);
    expect(createHash("sha256").update(rendered).digest("hex")).toBe(
      "4e45c41b345252c2870c9704b194e66ed628ec1071d9c6d6228843b944ef2604",
    );
  });
});

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

describe("Markdown shard filename grammar", () => {
  it("rejects a document title copied into a numbered entry", () => {
    const root = mkdtempSync(join(tmpdir(), "doc-shard-title-"));
    const plan = DOC_SHARD_PLANS[0];
    const document = join(root, "CHANGELOG.md");
    const dir = docShardDirectoryFor(document);
    const heading = `${"#".repeat(plan.headingLevel)} Fixed entry`;
    const name = docShardFilename(10, docSlug(heading), headingDigest(heading));
    mkdirSync(dir, { recursive: true });
    writeFileSync(join(dir, DOC_META_SHARD), "# Changelog\n\n");
    try {
      writeFileSync(join(dir, name), `# Changelog\n\n${heading}\n\nDetails.\n`);
      expect(() => readDocShards(document, plan)).toThrow(/must start with its level-.*heading/);
      writeFileSync(join(dir, name), `${heading}\n\nDetails.\n`);
      expect(readDocShards(document, plan)?.get(name)).toBe(`${heading}\n\nDetails.\n`);
    } finally {
      rmSync(root, { recursive: true, force: true });
    }
  });

  it("accepts metadata and positive rank/slug/digest section names", () => {
    expect(isValidDocShardName(DOC_META_SHARD)).toBe(true);
    expect(isValidDocShardName("00010-A-0f0f0f0f.md")).toBe(true);
    expect(isValidDocShardName("00020-AGENT-A-11111111.md")).toBe(true);
    expect(isValidDocShardName("00020-AGENT-B-22222222.md")).toBe(true);
  });

  it.each([
    "A-0f0f0f0f.md", // missing rank
    "00000-A-0f0f0f0f.md", // zero is not an ordering rank
    "00010-A-0f0f0f0.md", // short digest
    "00010-A-0f0f0f0g.md", // non-hex digest
    "00010-A-0F0F0F0F.md", // digest must be lowercase
    "00010-AGENT--A-0f0f0f0f.md", // empty slug component
    "_notes.md", // _meta.md is the sole reserved metadata name
  ])("rejects malformed section name %s", (name) => {
    expect(isValidDocShardName(name)).toBe(false);
  });

  it("refuses a malformed file before reading or rendering any shard", () => {
    const root = mkdtempSync(join(tmpdir(), "doc-shard-malformed-"));
    const plan = DOC_SHARD_PLANS[0];
    const document = join(root, ...plan.path.split("/"));
    const dir = docShardDirectoryFor(document);
    mkdirSync(dir, { recursive: true });
    writeFileSync(join(dir, DOC_META_SHARD), "# T\n\n");
    writeFileSync(join(dir, "MISSING-RANK-deadbeef.md"), "## hidden\n");

    try {
      expect(() => runDocShardCli(["--check", plan.path], root)).toThrow(/malformed filename/);
    } finally {
      rmSync(root, { recursive: true, force: true });
    }
  });

  it("renders valid same-rank parallel fragments deterministically", () => {
    const root = mkdtempSync(join(tmpdir(), "doc-shard-parallel-"));
    const document = join(root, "x", "DOC.md");
    const dir = docShardDirectoryFor(document);
    const headingA = "## AGENT-A";
    const headingB = "## AGENT-B";
    const nameA = docShardFilename(20, docSlug(headingA), headingDigest(headingA));
    const nameB = docShardFilename(20, docSlug(headingB), headingDigest(headingB));
    mkdirSync(dir, { recursive: true });
    writeFileSync(join(dir, DOC_META_SHARD), "# T\n\n");
    writeFileSync(join(dir, nameA), `${headingA}\n\na\n\n`);
    writeFileSync(join(dir, nameB), `${headingB}\n\nb\n\n`);

    try {
      const shards = readDocShards(document, PLAN);
      expect(shards).not.toBeNull();
      expect(joinDocShards(shards!, PLAN)).toBe(
        "# T\n\n## AGENT-B\n\nb\n\n## AGENT-A\n\na\n\n",
      );
    } finally {
      rmSync(root, { recursive: true, force: true });
    }
  });

  it("accepts only the exact content-pinned legacy corpus", () => {
    const root = mkdtempSync(join(tmpdir(), "doc-shard-legacy-"));
    const plan = DOC_SHARD_PLANS[0];
    const legacyName = Object.keys(LEGACY_DOC_SHARD_SHA256[plan.path])[0];
    const document = join(root, ...plan.path.split("/"));
    const dir = docShardDirectoryFor(document);
    const source = join(
      defaultRepoRoot(),
      ...docShardDirectoryFor(plan.path).split("/"),
      legacyName,
    );
    const body = readFileSync(source, "utf8");
    mkdirSync(dir, { recursive: true });
    writeFileSync(join(dir, DOC_META_SHARD), "# T\n\n");
    writeFileSync(join(dir, legacyName), body);

    try {
      expect(readDocShards(document, plan)?.get(legacyName)).toBe(body);
      writeFileSync(join(dir, legacyName), `${body}\nchanged\n`);
      expect(() => readDocShards(document, plan)).toThrow(/grandfathered content changed/);
    } finally {
      rmSync(root, { recursive: true, force: true });
    }
  });

  it("rejects a new legacy-shaped name even when its body is old", () => {
    const root = mkdtempSync(join(tmpdir(), "doc-shard-new-legacy-"));
    const plan = DOC_SHARD_PLANS[0];
    const legacyName = Object.keys(LEGACY_DOC_SHARD_SHA256[plan.path])[0];
    const document = join(root, ...plan.path.split("/"));
    const dir = docShardDirectoryFor(document);
    const source = join(
      defaultRepoRoot(),
      ...docShardDirectoryFor(plan.path).split("/"),
      legacyName,
    );
    mkdirSync(dir, { recursive: true });
    writeFileSync(join(dir, DOC_META_SHARD), "# T\n\n");
    writeFileSync(join(dir, "00010-UNKNOWN-LEGACY.md"), readFileSync(source, "utf8"));

    try {
      expect(() => readDocShards(document, plan)).toThrow(/malformed filename/);
    } finally {
      rmSync(root, { recursive: true, force: true });
    }
  });

  it("refuses to reshard a grandfathered append-only document", () => {
    const root = mkdtempSync(join(tmpdir(), "doc-shard-legacy-write-"));
    const plan = DOC_SHARD_PLANS[0];
    const legacyName = Object.keys(LEGACY_DOC_SHARD_SHA256[plan.path])[0];
    const document = join(root, ...plan.path.split("/"));
    const dir = docShardDirectoryFor(document);
    const source = join(
      defaultRepoRoot(),
      ...docShardDirectoryFor(plan.path).split("/"),
      legacyName,
    );
    const body = readFileSync(source, "utf8");
    mkdirSync(dir, { recursive: true });
    mkdirSync(join(document, ".."), { recursive: true });
    writeFileSync(document, `# T\n\n${body}`);
    writeFileSync(join(dir, DOC_META_SHARD), "# T\n\n");
    writeFileSync(join(dir, legacyName), body);

    try {
      expect(() => shardDocument(root, plan)).toThrow(/cannot reshard.*append-only legacy/);
      expect(readFileSync(join(dir, legacyName), "utf8")).toBe(body);
    } finally {
      rmSync(root, { recursive: true, force: true });
    }
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
// Bullet entries: documents that are one heading over a long list.
// ---------------------------------------------------------------------------
describe("splitting on top-level bullets", () => {
  const BULLET_PLAN: DocShardPlan = {
    path: "fixture/BULLETS.md",
    headingLevel: 2,
    newestFirst: true,
    entryShape: "bullet",
  };

  it("takes everything above the first bullet as the preamble", () => {
    const text = "# Changelog\n\nBlurb.\n\n## Unreleased\n\n- first\n\n- second\n";
    const { preamble, sections } = splitDocument(text, "bullet");

    expect(preamble).toBe("# Changelog\n\nBlurb.\n\n## Unreleased\n\n");
    expect(sections.map((s) => s.heading)).toEqual(["- first", "- second"]);
  });

  it("round-trips byte-for-byte", () => {
    const text = "# T\n\n## Unreleased\n\n- one\n  continued\n\n- two\n";
    const { preamble, sections } = splitDocument(text, "bullet");
    expect(preamble + sections.map((s) => s.text).join("")).toBe(text);
  });

  it("keeps an INDENTED bullet with the entry above it", () => {
    // A nested list item is part of its parent entry. Splitting on it would cut
    // an entry in half and file the halves under different names.
    const text = "# T\n\n- parent\n  - nested\n  - also nested\n\n- next\n";
    const { sections } = splitDocument(text, "bullet");
    expect(sections.map((s) => s.heading)).toEqual(["- parent", "- next"]);
    expect(sections[0].text).toContain("  - nested");
  });

  it("ignores a bullet inside a fenced block", () => {
    // `- ` at column 0 inside ``` is shell/diff/YAML content, not an entry.
    const text = "# T\n\n- real\n\n```yaml\n- not an entry\n```\n\n- also real\n";
    const { sections } = splitDocument(text, "bullet");
    expect(sections.map((s) => s.heading)).toEqual(["- real", "- also real"]);
    expect(sections[0].text).toContain("- not an entry");
  });

  it("does not treat a heading as an entry in bullet mode", () => {
    const text = "# T\n\n## Unreleased\n\n- only entry\n\n## Older\n";
    const { sections } = splitDocument(text, "bullet");
    expect(sections).toHaveLength(1);
    expect(sections[0].text).toContain("## Older");
  });

  it("shards and rejoins through the plan API", () => {
    const text = "# T\n\n## Unreleased\n\n- alpha entry\n\n- beta entry\n";
    const shards = docShardContents(text, BULLET_PLAN);
    expect(shards.size).toBe(3); // two entries + _meta.md
    expect(joinDocShards(shards, BULLET_PLAN)).toBe(text);
  });

  it("refuses a document with no top-level bullets, naming bullets", () => {
    expect(() => docShardContents("# T\n\nprose only\n", BULLET_PLAN)).toThrow(
      /top-level bullets/,
    );
  });

  it("leaves heading-shaped plans splitting on headings", () => {
    // The default must not move. `entryShape` is optional, and every existing
    // plan omits it.
    const text = "# T\n\n## one\n\n- a bullet\n\n## two\n";
    const headingPlan: DocShardPlan = {
      path: "fixture/H.md",
      headingLevel: 2,
      newestFirst: true,
    };
    const shards = docShardContents(text, headingPlan);
    expect(shards.size).toBe(3);
    expect(joinDocShards(shards, headingPlan)).toBe(text);
  });
});

describe("the measured bullet-shaped document", () => {
  // Proves the tool handles the actual target before any migration commits to
  // it. `adj-facts-stdlib/CHANGELOG.md` is the repo's second-worst conflict
  // generator by time-clustered contention and cannot be split on headings: it
  // has exactly one `##` over 323 top-level entries.
  const TARGET = "code/specs/data/adj-facts-stdlib/CHANGELOG.md";

  // The document is read through `unshardDocContents`, NOT with `readFileSync`
  // on the path above.
  //
  // These two tests were written when that file was still tracked, and read it
  // straight off disk. This PR makes it a generated, gitignored aggregate — so
  // on a clean checkout it is not there, and both tests died with ENOENT in
  // CI. They passed locally only because an earlier `--unshard` had left a
  // rendered copy sitting in my working tree.
  //
  // `unshardDocContents` is what every other real-document test in this file
  // uses, and it reads the shards, which are the source of truth. It cannot go
  // stale against them and does not depend on whether anyone happens to have
  // rendered the aggregate.
  const plan = DOC_SHARD_PLANS.find((p) => p.path === TARGET);

  it("is registered as a bullet plan", () => {
    // Guards the two tests below from passing vacuously if the plan were
    // renamed or dropped: without this they would simply skip their bodies.
    expect(plan?.entryShape).toBe("bullet");
  });

  it("splits on headings into far too few sections to be useful", () => {
    // The reason the new mode exists, asserted rather than asserted-about.
    //
    // Deliberately MONOTONE, not `toHaveLength(1)`. That file is an active
    // keepachangelog owned by another team, and the moment someone cuts a
    // release it gains a `## [0.4.0]` and goes 1 -> 2. Worse, it lives outside
    // this package, so the build tool would not run this suite on the PR that
    // broke it -- the red would land later, in an unrelated change, pointing at
    // a changelog its author never touched. The claim that matters is "heading
    // splitting is useless here", which survives any number of release
    // headings.
    const text = unshardDocContents(defaultRepoRoot(), plan!);
    const byHeading = splitDocument(text, 2).sections.length;
    const byBullet = splitDocument(text, "bullet").sections.length;
    expect(byHeading).toBeLessThan(byBullet / 10);
  });

  it("splits on bullets into many, and rejoins byte-for-byte", () => {
    const text = unshardDocContents(defaultRepoRoot(), plan!);
    const shards = docShardContents(text, plan!);
    expect(shards.size).toBeGreaterThan(200);
    expect(joinDocShards(shards, plan!)).toBe(text);
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
      // Via `docSplitAt`, not `plan.headingLevel`. Under a bullet plan the
      // latter names a heading level the document barely has, so this filter
      // would collect one line and compare it against 323 shards. Same class of
      // bug as the failure message in doc-shard-cli.ts that still read
      // `headingLevel` — a call site that did not follow the split rule.
      const at = docSplitAt(plan);
      const level = at === "bullet" ? "- " : "#".repeat(at) + " ";

      const fromMonolith = rendered
        .split("\n")
        .filter(
          (line) =>
            line.startsWith(level) &&
            (at === "bullet" ? true : !line.startsWith(level + "#")),
        );

      const dir = docShardDirectoryFor(monolith);
      const names = readdirSync(dir)
        .filter((n) => n.endsWith(".md") && n !== DOC_META_SHARD)
        .sort((a, b) => (a < b ? -1 : a > b ? 1 : 0));
      if (plan.newestFirst) names.reverse();
      const fromShards = names.map(
        (n) => readFileSync(join(dir, n), "utf8").split("\n")[0],
      );

      expect(fromShards).toEqual(fromMonolith);
      if (plan.path === HINDI_CHANGELOG) {
        expect(fromShards.length).toBeGreaterThan(HINDI_MIGRATION_MAX_RANK / 10);
      } else {
        expect(fromShards.length).toBeGreaterThan(100);
      }
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
      // `docSplitAt`, for the same reason as the test above: under a bullet
      // plan `plan.headingLevel` splits the document into one section and this
      // would assert 324 === 2.
      const sections = splitDocument(rendered, docSplitAt(plan)).sections.length;
      expect(contents.size).toBe(sections + 1); // +1 for _meta.md
    });
  }
});

// ---------------------------------------------------------------------------
// The registry and the gate that guards it must not drift apart.
// ---------------------------------------------------------------------------
describe("the append-only deletion guard covers every plan", () => {
  // `--check` does NOT catch a deleted shard. Measured: delete one shard, run
  // `--check` with no local rendered monolith present (which is CI's state,
  // since the monolith is gitignored) and it exits 0. It only noticed locally
  // because a stale rendered file happened to be sitting there.
  //
  // What actually forbids a deletion is a `git diff --diff-filter=D` against a
  // HARDCODED glob list in the detect job of `human-languages-books.yml`. That
  // list is maintained by hand and had already drifted: Hindi has 72 committed
  // shards and appeared in DOC_SHARD_PLANS, yet no glob covered it, so a PR
  // deleting its entire history would have passed.
  //
  // This pins the two together, so adding a plan without extending the guard
  // fails here instead of silently shipping unguarded history.
  const WORKFLOW = ".github/workflows/human-languages-books.yml";

  /** The workflow with whole-line comments removed.
   *
   * A plain `includes()` over the raw file would be satisfied by a glob that
   * had been COMMENTED OUT but left as text — the exact state this test exists
   * to reject.
   *
   * Comments are stripped rather than the array being sliced out, because
   * `doc_shard_globs` is not one literal: Script Ductus is appended
   * conditionally with `+=` further down, and a test scoped to the literal
   * reported it missing when it is properly covered. Only whole-line comments
   * are removed, so a `#` inside a quoted glob could never be damaged.
   */
  function code(workflow: string): string {
    return workflow
      .split("\n")
      .filter((line) => !line.trimStart().startsWith("#"))
      .join("\n");
  }

  const workflow = () =>
    code(readFileSync(join(defaultRepoRoot(), WORKFLOW), "utf8"));

  it("names a shard glob for every document in DOC_SHARD_PLANS", () => {
    const text = workflow();
    const missing = DOC_SHARD_PLANS.map(
      (plan) => `${docShardDirectoryFor(plan.path)}/*.md`,
    ).filter((glob) => !text.includes(glob));

    expect(missing).toEqual([]);
  });

  it("forbids re-tracking the aggregate of every document in DOC_SHARD_PLANS", () => {
    // The second half of the same drift. `--check` does not test trackedness
    // either -- a resurrected aggregate that happens to be IN SYNC passes it --
    // so `tracked_doc_monoliths` is the only thing standing between a merge and
    // the restored hot spot. It had drifted too: Hindi was a plan with no entry.
    const text = workflow();
    const missing = DOC_SHARD_PLANS.map((plan) => plan.path).filter(
      (path) => !text.includes(path),
    );

    expect(missing).toEqual([]);
  });

  it("keeps every line-continuation in the trackedness gate intact", () => {
    // `includes()` proves a path is MENTIONED, not that it reaches `git
    // ls-files`. The paths are backslash-continued arguments, so dropping one
    // `\` truncates the argument list: every path below the break silently
    // stops being checked while the test above stays green, because the text is
    // still in the file.
    //
    // That is the failure this whole pin exists to prevent, one level down —
    // a gate that reads as covering more than it does.
    const lines = workflow().split("\n");
    const start = lines.findIndex((l) => l.includes("tracked_doc_monoliths=$("));
    expect(start).toBeGreaterThan(-1);

    const end = lines.findIndex((l, i) => i > start && l.trimEnd().endsWith(")"));
    expect(end).toBeGreaterThan(start);

    // Every line of the invocation except the last must end in a backslash,
    // with NO trailing whitespace after it — `\ ` is a line continuation that
    // bash does not honour.
    const broken = lines
      .slice(start, end)
      .filter((l) => !/\\$/.test(l))
      .map((l) => l.trim());

    expect(broken).toEqual([]);
    // And the block must actually carry every plan, not just end tidily.
    const block = lines.slice(start, end + 1).join("\n");
    expect(
      DOC_SHARD_PLANS.map((p) => p.path).filter((p) => !block.includes(p)),
    ).toEqual([]);
  });
});
