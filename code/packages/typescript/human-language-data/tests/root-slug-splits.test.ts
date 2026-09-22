import { mkdirSync, mkdtempSync, rmSync, symlinkSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import {
  diffRootSlugSplits,
  findRootSlugSplits,
  loadRootSlugBaseline,
  loadRootTagVocabulary,
  parseRootSlug,
} from "../src/root-slug-splits.js";
import { runRootSlugSplitsCli } from "../src/root-slug-splits-cli.js";
import { defaultCurriculumRoot } from "../src/loader.js";

const roots: string[] = [];
afterEach(() => {
  for (const root of roots.splice(0)) rmSync(root, { recursive: true, force: true });
});

/** A two-lesson corpus whose roots are supplied by the caller. */
function fixture(rootsByLesson: Record<string, string[]>, tags?: unknown): string {
  const root = mkdtempSync(join(tmpdir(), "hl-root-slug-"));
  roots.push(root);
  mkdirSync(join(root, "core"), { recursive: true });
  mkdirSync(join(root, "toy", "lessons"), { recursive: true });
  writeFileSync(
    join(root, "core", "root-tags.json"),
    `${JSON.stringify(tags ?? { version: 1, tags: ["latin", "greek", "old-french"], aliases: {} }, null, 2)}\n`,
  );
  let index = 0;
  for (const [id, slugs] of Object.entries(rootsByLesson)) {
    index += 1;
    writeFileSync(
      join(root, "toy", "lessons", `${id}.md`),
      `---\nschema_version: 2\nid: ${id}\nchapter: ${index}\nheadword: w${index}\n` +
        `roots: [${slugs.join(", ")}]\n---\n\n# w${index}\n`,
    );
  }
  return root;
}

describe("root slug tag parsing", () => {
  const vocabulary = { tags: ["latin", "french", "old-french"], aliases: {} };

  it("matches the LONGEST tag, so old-french beats french", () => {
    // `corlieu-old-french` is a real slug. Matching `french` first would leave
    // the lemma as `corlieu-old`, which no other spelling could ever collide
    // with -- the split would become invisible rather than loud.
    expect(parseRootSlug("corlieu-old-french", vocabulary)).toMatchObject({
      lemma: "corlieu",
      tag: "old-french",
      shape: "suffix",
    });
  });

  it("reads a tag at either end", () => {
    expect(parseRootSlug("stare-latin", vocabulary)).toMatchObject({ lemma: "stare", shape: "suffix" });
    expect(parseRootSlug("latin-stare", vocabulary)).toMatchObject({ lemma: "stare", shape: "prefix" });
  });

  it("matches case-insensitively, and lowercases the lemma with it", () => {
    // The corpus carries exactly one case-only duplicate: `SANSKRIT-PA-DRINK`
    // in two Marwadi lessons against `sanskrit-pa-drink` in four Gujarati,
    // Punjabi and Marathi ones. One etymon, two spellings, six lessons --
    // exactly the defect this module exists for. A case-SENSITIVE match left
    // the upper-case form opaque and the split invisible.
    expect(parseRootSlug("SANSKRIT-PA-DRINK", { tags: ["sanskrit"], aliases: {} })).toMatchObject({
      slug: "SANSKRIT-PA-DRINK",
      lemma: "pa-drink",
      tag: "sanskrit",
    });
  });

  it("leaves an UNDECLARED tag opaque rather than guessing", () => {
    // `-heart`, `-speak` and `-see` are glosses, not languages, and nothing in
    // the slug says so. Treating an unknown trailing token as a tag would
    // split `bhaga-share` into lemma `bhaga`, and any other `bhaga-*` slug
    // would then be reported as a collision that is not one. Costing coverage
    // is the safe direction; inventing a split is not.
    expect(parseRootSlug("kerd-heart", vocabulary)).toMatchObject({
      lemma: "kerd-heart",
      tag: undefined,
      shape: "bare",
    });
  });
});

describe("root slug vocabulary", () => {
  it("rejects a BLANK declared tag", () => {
    // A blank tag matches at both ends of everything, collapsing every
    // `lemma|tag` key onto the `lemma|` form the bare-vs-tagged kind uses. A
    // freshly written baseline then fails its own --check and the gate is
    // stuck red with no repair available from the data.
    const root = fixture({ "TOY-1": ["a-latin"] }, { version: 1, tags: ["", "latin"], aliases: {} });
    expect(() => loadRootTagVocabulary(root)).toThrow(/a declared tag is blank/);
  });

  it("does not let an inherited property answer an alias lookup", () => {
    // `aliases` is built with Object.create(null). On a plain `{}` a tag named
    // `toString` or `constructor` would resolve to the inherited function and
    // silently produce a key like `x|function Object() { [native code] }`.
    const root = fixture(
      { "TOY-1": ["constructor-a"], "TOY-2": ["a-constructor"] },
      { version: 1, tags: ["constructor"], aliases: {} },
    );
    expect(findRootSlugSplits(root)).toEqual([
      { key: "a|constructor", slugs: ["a-constructor", "constructor-a"], kind: "shape" },
    ]);
  });

  it("rejects an alias pointing at a tag that is not declared", () => {
    // Such an alias would fold slugs onto a key nothing else can reach, so the
    // splits it was added to catch would silently stop being reported.
    const root = fixture(
      { "TOY-1": ["a-latin"] },
      { version: 1, tags: ["latin"], aliases: { pie: "proto-indo-european" } },
    );
    expect(() => loadRootTagVocabulary(root)).toThrow(/undeclared tag 'proto-indo-european'/);
  });
});

describe("root slug splits", () => {
  it("reports one etymon spelled two ways", () => {
    const root = fixture({ "TOY-1": ["stare-latin"], "TOY-2": ["latin-stare"] });
    expect(findRootSlugSplits(root)).toEqual([
      { key: "stare|latin", slugs: ["latin-stare", "stare-latin"], kind: "shape" },
    ]);
  });

  it("reports a bare lemma against a tagged one, as its own kind", () => {
    const root = fixture({ "TOY-1": ["bonus"], "TOY-2": ["bonus-latin"] });
    expect(findRootSlugSplits(root)).toEqual([
      { key: "bonus|", slugs: ["bonus", "bonus-latin"], kind: "bare-vs-tagged" },
    ]);
  });

  it("does NOT report one lemma under two different declared tags", () => {
    // `dravidian`/`proto-dravidian` co-occur on eight real lemmas and
    // `frankish`/`germanic` on three; in each the tags name different
    // languages and a lesson may legitimately cite either. Folding them would
    // ASSERT a shared etymology, which is the one thing cousins.ts exists to
    // prevent -- so this case is left alone and documented rather than guessed.
    const root = fixture({ "TOY-1": ["bursa-greek"], "TOY-2": ["bursa-latin"] });
    expect(findRootSlugSplits(root)).toEqual([]);
  });

  it("folds an ALIASED tag, so pie and proto-indo-european collide", () => {
    const root = fixture(
      { "TOY-1": ["pie-dwoh"], "TOY-2": ["proto-indo-european-dwoh"] },
      { version: 1, tags: ["pie", "proto-indo-european"], aliases: { pie: "proto-indo-european" } },
    );
    expect(findRootSlugSplits(root)).toEqual([
      {
        key: "dwoh|proto-indo-european",
        slugs: ["pie-dwoh", "proto-indo-european-dwoh"],
        kind: "shape",
      },
    ]);
  });

  it("says nothing when every etymon has one spelling", () => {
    const root = fixture({ "TOY-1": ["stare-latin"], "TOY-2": ["amare-latin"] });
    expect(findRootSlugSplits(root)).toEqual([]);
  });
});

describe("baseline diffing", () => {
  const baseline = { version: 1, splits: [{ key: "stare|latin", slugs: ["latin-stare", "stare-latin"] }] };

  it("passes when live matches baseline exactly", () => {
    const live = [{ key: "stare|latin", slugs: ["latin-stare", "stare-latin"], kind: "shape" as const }];
    expect(diffRootSlugSplits(live, baseline)).toEqual({ added: [], resolved: [] });
  });

  it("flags a NEW split", () => {
    const live = [
      { key: "stare|latin", slugs: ["latin-stare", "stare-latin"], kind: "shape" as const },
      { key: "amare|latin", slugs: ["amare-latin", "latin-amare"], kind: "shape" as const },
    ];
    expect(diffRootSlugSplits(live, baseline).added.map((split) => split.key)).toEqual(["amare|latin"]);
  });

  it("flags a split that GAINED a third spelling", () => {
    // The key is already in the baseline, so a key-only comparison would pass
    // this and the corpus would drift one spelling at a time without ever
    // failing. The slug list is compared, not just the key.
    const live = [
      { key: "stare|latin", slugs: ["latin-stare", "stare-latin", "stare-latin-2"], kind: "shape" as const },
    ];
    expect(diffRootSlugSplits(live, baseline).added).toHaveLength(1);
  });

  it("flags a STALE baseline entry, so the file may only shrink honestly", () => {
    expect(diffRootSlugSplits([], baseline).resolved).toEqual(["stare|latin"]);
  });

  it("compares slug lists element-wise, not space-joined", () => {
    // Six live slugs contain spaces (`ad de magis`, `qui sapit`, `sub ponere`,
    // ...). Joined on a space, ["a b","c"] and ["a","b c"] are the same string
    // and a real change between those shapes would pass.
    const joined = { version: 1, splits: [{ key: "k", slugs: ["a b", "c"] }] };
    const live = [{ key: "k", slugs: ["a", "b c"], kind: "shape" as const }];
    expect(diffRootSlugSplits(live, joined).added).toHaveLength(1);
  });
});

describe("the write side", () => {
  it("refuses to GROW the baseline without --allow-new", () => {
    // --write is the command a contributor is told to run, so without this it
    // is also the command that quietly launders a new split into the accepted
    // set. The baseline may only shrink.
    const root = fixture({ "TOY-1": ["stare-latin"], "TOY-2": ["latin-stare"] });
    writeFileSync(
      join(root, "core", "root-slug-split-baseline.json"),
      `${JSON.stringify({ version: 1, splits: [] }, null, 2)}\n`,
    );
    expect(() => runRootSlugSplitsCli(["--write"], root)).toThrow(/refusing to grow the baseline/);
    expect(runRootSlugSplitsCli(["--write", "--allow-new"], root)).toBe(0);
  });

  it("refuses to write through a SYMLINK, as the read side already refuses to read one", () => {
    // `readLedgerFile` refuses a symlinked ledger, so without this the two
    // halves of one CLI disagree about the same path and --write follows a link
    // the checker would have rejected. shard.ts: "a guard living only inside
    // the reader is a guard the writer forgets."
    const root = fixture({ "TOY-1": ["stare-latin"] });
    const elsewhere = join(root, "elsewhere.json");
    writeFileSync(elsewhere, "{}\n");
    try {
      symlinkSync(elsewhere, join(root, "core", "root-slug-split-baseline.json"));
    } catch {
      return; // A platform without symlink permission proves nothing either way.
    }
    expect(() => runRootSlugSplitsCli(["--write"], root)).toThrow(/must be a real regular file/);
  });
});

describe("the live corpus", () => {
  it("matches its committed baseline", () => {
    expect(runRootSlugSplitsCli(["--check"])).toBe(0);
  });

  it("pins the CURRENT DEBT, which may only fall", () => {
    // HL-C419. THE UNIT IS A BASELINE ENTRY, not an etymon, and the two differ:
    // 110 shape entries (stare-latin / latin-stare) plus 82 bare-vs-tagged
    // (bonus / bonus-latin) is 192 entries, but 16 bare-vs-tagged entries
    // strictly contain a shape entry for the same lemma, so the file covers
    // 175 DISTINCT ETYMON GROUPS over 372 slugs. Saying "192 splits" without
    // the unit would double-count those 16 -- which is the mistake the HL-C419
    // shard itself records as its own, three drafts running.
    //
    // This is the worklist for the normalisation PR the shard describes; the
    // guard exists so it cannot grow while that PR is waiting to be written.
    const baselineFile = loadRootSlugBaseline(defaultCurriculumRoot());
    expect(baselineFile.splits).toHaveLength(192);
    expect(findRootSlugSplits()).toHaveLength(192);
  });
});
