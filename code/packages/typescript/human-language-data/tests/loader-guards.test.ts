// loader-guards.test.ts — every ledger read in `src/` goes through `shard.ts`.
//
// Issue #12564. `loader.ts` grew seventeen `JSON.parse(readFileSync(...))` call
// sites before `shard.ts` existed, and none of them picked up the guards it
// added afterwards. Three of those guards are defence in depth — symlink
// refusal, dangerous-key rejection, parse-error scrubbing. The fourth is not
// defensive at all, and it is why this file exists.
//
// Since HL21 landed (PR #12690), `<track>/chapters.d/`, `<track>/curriculum.d/`
// and `core/book-generation.d/` are the SOURCE OF TRUTH, and the `.json` beside
// each is a generated artifact kept only because a browser bundle cannot
// `readdirSync`. Between an edit to a shard and the next `--check`, that
// monolith holds stale bytes — which parse, validate, and look complete. A
// reader that opens it directly gets a plausible wrong answer and no error.
//
// The first test below is the whole point of the change: it builds a ledger
// whose monolith and shards genuinely DISAGREE and shows both halves — the old
// bare parse returning the stale answer silently, and the guarded path refusing.

import {
  mkdirSync,
  mkdtempSync,
  readFileSync,
  readdirSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { isAbsentErrno, isSharded, readLedgerFile } from "../src/shard.js";
import {
  loadLanguageRegistry,
  listExamInventories,
  loadLessons,
  loadScripts,
  loadTaxonomy,
  loadTrackChapters,
  loadTrackGrammarCells,
  loadTrackLessons,
  trackScript,
} from "../src/loader.js";

let root: string;

beforeEach(() => {
  root = mkdtempSync(join(tmpdir(), "hl-loader-guards-"));
});

afterEach(() => {
  rmSync(root, { recursive: true, force: true });
});

function writeJson(path: string, value: unknown): void {
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`, "utf8");
}

describe("a monolith that no longer agrees with its shards", () => {
  // The registry is the example because it is small enough to read at a glance
  // and because `loadLanguageRegistry` was one of the seventeen. The mechanism
  // is the ledger LAYOUT, not this particular file: any `X.json` with an `X.d/`
  // beside it behaves the same way.
  function writeDisagreeingRegistry(): void {
    // The monolith: what a generated artifact looks like after somebody edited
    // the shards and did not re-run `npm run unshard`. Perfectly well-formed.
    writeJson(join(root, "core", "languages.json"), {
      version: 1,
      languages: [{ id: "spanish" }],
    });
    // The shards: the source of truth, one language further along.
    writeJson(join(root, "core", "languages.d", "_meta.json"), { version: 1 });
    writeJson(join(root, "core", "languages.d", "0010-SPANISH.json"), { id: "spanish" });
    writeJson(join(root, "core", "languages.d", "0020-URDU.json"), { id: "urdu" });
  }

  it("was read, staleness and all, by the bare parse this change removed", () => {
    writeDisagreeingRegistry();

    // Verbatim the form that was on `loader.ts`'s `loadLanguageRegistry` before
    // this change. It is reproduced here rather than described, because the
    // claim being made is about what that code DID, and an inspection-only
    // claim about this file has been wrong before.
    const viaBareParse = JSON.parse(
      readFileSync(join(root, "core", "languages.json"), "utf8"),
    ) as { languages: { id: string }[] };

    // No throw. No warning. A registry that is simply a version behind, and
    // nothing anywhere in the process can tell that from a correct one.
    expect(viaBareParse.languages.map((l) => l.id)).toEqual(["spanish"]);
    expect(viaBareParse.languages).toHaveLength(1);
  });

  it("is refused by the guarded path, which names the directory that has the data", () => {
    writeDisagreeingRegistry();

    // Same bytes on disk, same function the rest of the package calls.
    expect(() => loadLanguageRegistry(root)).toThrow(/is sharded into/);
    // The message has to be actionable: whoever hits this needs to know where
    // the real data is, not merely that something was wrong.
    expect(() => loadLanguageRegistry(root)).toThrow(/languages\.d/);
    expect(() => loadLanguageRegistry(root)).toThrow(/may be stale/);
  });

  it("refuses even when the two happen to agree, because agreeing is not a property", () => {
    // The check is on LAYOUT, deliberately, not on a comparison of contents.
    // Comparing would mean reading and merging the shards to find out — which
    // is the work the caller was supposed to be doing — and would let the read
    // pass today and fail tomorrow for reasons no diff explains. A read that is
    // correct only while two files coincide is not a read anybody can rely on.
    writeJson(join(root, "core", "languages.json"), { version: 1, languages: [] });
    writeJson(join(root, "core", "languages.d", "_meta.json"), { version: 1 });

    expect(() => loadLanguageRegistry(root)).toThrow(/is sharded into/);
  });
});

describe("the fallback the partial migration depends on", () => {
  // HL21 is deliberately incomplete: `chapters` is sharded in 20 of 23 tracks
  // and `curriculum` in 22 of 23. A guard that assumed shards always exist
  // would take `french`, `japanese` and `marwadi` out of the corpus, and take
  // them out SILENTLY — `loadTrackChapters` treats an absent ledger as honest
  // un-authored debt. That is the exact shape of failure this whole area keeps
  // producing, so it gets a test rather than a comment.
  it("reads the monolith unchanged when there is no X.d beside it", () => {
    writeJson(join(root, "core", "languages.json"), {
      version: 1,
      languages: [{ id: "marwadi" }],
    });

    expect(loadLanguageRegistry(root).languages.map((l) => l.id)).toEqual(["marwadi"]);
  });

  it("still loads the three tracks whose chapters were never sharded", () => {
    // Against the REAL corpus, not a fixture, because the thing being checked
    // is a fact about the corpus as committed.
    const here = dirname(fileURLToPath(import.meta.url));
    const realRoot = join(here, "..", "..", "..", "..", "learning", "human-languages");
    const byLanguage = new Map(loadTrackChapters(realRoot).map((t) => [t.language, t]));

    for (const monolithOnly of ["french", "japanese", "marwadi"]) {
      const track = byLanguage.get(monolithOnly);
      expect(track, `${monolithOnly} fell out of the corpus`).toBeDefined();
      expect(track!.chapters.length).toBeGreaterThan(0);
    }
    // And the sharded majority still arrives too, so this is not passing by
    // having quietly stopped reading anything.
    expect(byLanguage.get("spanish")?.chapters.length).toBeGreaterThan(0);
    expect(byLanguage.size).toBeGreaterThanOrEqual(20);
  });
});

describe("the three guards the bare reads had been skipping", () => {
  it("refuses a ledger carrying __proto__", () => {
    mkdirSync(join(root, "data", "scripts"), { recursive: true });
    writeFileSync(
      join(root, "data", "scripts", "latin.json"),
      '{"script":"latin","__proto__":{"polluted":true}}',
      "utf8",
    );

    expect(() => loadScripts(root)).toThrow(/must not carry '__proto__'/);
    // And nothing leaked on the way to the throw.
    expect(({} as Record<string, unknown>).polluted).toBeUndefined();
  });

  it("holds back the file's bytes when it cannot parse it", () => {
    // V8 splices the offending content into its `SyntaxError` — parse a file
    // starting `AKIA…` and the message quotes it. These reads run in CI, and CI
    // logs are read far more widely than the repo.
    mkdirSync(join(root, "concepts"), { recursive: true });
    writeFileSync(
      join(root, "concepts", "taxonomy.json"),
      "AKIAIOSFODNN7EXAMPLE not json at all",
      "utf8",
    );

    let caught: unknown;
    try {
      loadTaxonomy(root);
    } catch (error) {
      caught = error;
    }
    expect(caught).toBeInstanceOf(Error);
    expect((caught as Error).message).toMatch(/malformed JSON/);
    // The secret is in neither the message nor the cause chain — the cause is
    // what Node's default handler and Vitest actually print.
    expect((caught as Error).message).not.toMatch(/AKIA/);
    expect(String((caught as Error).cause ?? "")).not.toMatch(/AKIA/);
  });

  it("refuses a symlinked ledger rather than following it", (ctx) => {
    // Creating a symlink needs Developer Mode or elevation on Windows. Where it
    // cannot be created the case skips; the guard it covers still runs on every
    // other platform.
    const secret = join(root, "secret.json");
    writeJson(secret, { script: "latin", secret: "s3cret" });
    mkdirSync(join(root, "data", "scripts"), { recursive: true });
    const link = join(root, "data", "scripts", "latin.json");
    try {
      symlinkSync(secret, link, "file");
    } catch {
      ctx.skip();
      return;
    }

    expect(() => loadScripts(root)).toThrow(/is a symbolic link/);
  });
});

describe("track ids that reach a path", () => {
  // `join` NORMALISES an embedded `..` rather than refusing it, so a trailing
  // filename is no protection: `join(root, "../../..", "grammar-cells.json")`
  // leaves the curriculum root entirely.
  const escapes = [
    "../../../etc",
    "a/b",
    "a\\b",
    "C:",
    "D:\\evil",
    "\\\\server\\share",
    "/absolute",
    "_fonts",
    // `$` is end-of-INPUT in JavaScript unless `m` is set, so this must not
    // match on the strength of its first line.
    "spanish\n../../etc",
  ];

  it("refuses a grammar-cells language id that would escape the root", () => {
    for (const bad of escapes) {
      expect(() => loadTrackGrammarCells(bad, root), bad).toThrow(/unsafe language id/);
    }
  });

  it("refuses a track-script id that would escape the root", () => {
    // `trackScript` had no guard at all, and is exported. It THROWS rather than
    // returning `undefined`: `undefined` is its answer for "no declaration,
    // use the built-in map", and folding a traversing id into that would let a
    // probe retry in silence. The guard sits outside the try/catch for the
    // same reason.
    for (const bad of escapes) {
      expect(() => trackScript(root, bad), bad).toThrow(/unsafe track id/);
    }
  });

  it("validates a lessons id before the id reaches a path", () => {
    // This check used to run AFTER `join` and after `existsSync`, which made
    // the return value an existence oracle: `[]` when `<target>/lessons` was
    // absent, a throw when it was present — for any path on the machine.
    for (const bad of escapes) {
      expect(() => loadTrackLessons(bad, root), bad).toThrow(/unsafe language id/);
    }
  });

  it("still walks the real corpus, whose root holds seven non-track directories", () => {
    // The reorder above has a trap, and this is the test for it. `loadLessons`
    // enumerates EVERY directory under the root — `_assets`, `_fonts`,
    // `_shared`, `concepts`, `core`, `data` and `progress` are all down there —
    // and used to reach the "not a track" answer by building `<dir>/lessons`
    // and finding nothing. Moving the id check to the front of
    // `loadTrackLessons` turns each of those into a throw unless the enumerator
    // skips them first.
    const here = dirname(fileURLToPath(import.meta.url));
    const realRoot = join(here, "..", "..", "..", "..", "learning", "human-languages");

    const lessons = loadLessons(realRoot);
    expect(lessons.length).toBeGreaterThan(1000);
    expect(new Set(lessons.map((l) => l.language)).size).toBeGreaterThanOrEqual(20);
  });
});

describe("'not sharded' must mean 'I looked, and it is not there'", () => {
  // Issue #12734. `isSharded` used to answer `false` for EVERY `lstat` failure,
  // which is not a conservative default: it is an assertion of a fact nobody
  // checked. Since #12690 it is also the dangerous direction — "not sharded"
  // routes every reader to a monolith that is now a generated artifact. The
  // sibling bug in `doc-shard.ts` printed "BACKLOG.d is missing" and exited 1
  // for a directory holding 109 shards.

  it("classifies only ENOENT and ENOTDIR as absent", () => {
    // The pure predicate, tested directly. `vi.spyOn` cannot patch a `node:fs`
    // export under ESM — the module namespace is not configurable — so a spy
    // cannot reach this classification at all. Extracting it is what makes the
    // errnos that matter most (EBUSY, EMFILE) testable without provoking them.
    expect(isAbsentErrno("ENOENT")).toBe(true);
    expect(isAbsentErrno("ENOTDIR")).toBe(true);

    for (const cannotTell of ["EACCES", "EPERM", "EBUSY", "EMFILE", "ENFILE", "EIO", "ELOOP"]) {
      expect(isAbsentErrno(cannotTell), `${cannotTell} is not absence`).toBe(false);
    }
    // An error with no `code` at all is the same "I could not tell".
    expect(isAbsentErrno(undefined)).toBe(false);
  });

  it("still reports a genuinely absent shard directory as absent", () => {
    // The HL21 §2.3 fallback has to keep working, or the 3 unmigrated tracks
    // and every unsharded ledger stop loading.
    writeJson(join(root, "core", "languages.json"), { version: 1, languages: [] });

    expect(isSharded(join(root, "core", "languages.json"))).toBe(false);
  });

  it("reports absent when a parent component is a file (ENOTDIR), not an error", () => {
    // Provoked for real rather than mocked. `core` is a FILE here, so
    // `core/languages.d` cannot exist and `lstat` says ENOTDIR.
    mkdirSync(root, { recursive: true });
    writeFileSync(join(root, "core"), "not a directory", "utf8");

    expect(isSharded(join(root, "core", "languages.json"))).toBe(false);
  });

  it("refuses a FILE squatting where the shard directory belongs", () => {
    // Also provoked for real. This used to return `false` — reporting the
    // ledger as unsharded and falling back to the monolith — which sends the
    // reader hunting for a directory whose name is already taken.
    writeJson(join(root, "core", "languages.json"), { version: 1, languages: [] });
    writeFileSync(join(root, "core", "languages.d"), "squatter", "utf8");

    expect(() => isSharded(join(root, "core", "languages.json"))).toThrow(
      /exists and is not a directory/,
    );
  });
});

describe("the two callers that tolerate a bad ledger tolerate only ONE thing", () => {
  // `trackScript` and `listExamInventories` both used a bare `catch {}`, which
  // was true to its comment when a parse was all that could go wrong. Widening
  // `readLedgerFile`'s throw surface silently widened what they absorbed.

  it("falls back to the script map for a malformed track.json, as documented", () => {
    mkdirSync(join(root, "spanish"), { recursive: true });
    writeFileSync(join(root, "spanish", "track.json"), "{ not json", "utf8");

    expect(trackScript(root, "spanish")).toBeUndefined();
  });

  it("does NOT swallow a refusal it cannot interpret as 'no declaration'", () => {
    // A file squatting where `track.d/` would go. Before the narrowing this
    // returned `undefined`, and `parse.ts` resolves an absent script to the
    // built-in map and ultimately to `latin` — so a track declaring its script
    // only here would have been re-parsed in the wrong script, silently.
    mkdirSync(join(root, "spanish"), { recursive: true });
    writeJson(join(root, "spanish", "track.json"), { script: "arabic" });
    writeFileSync(join(root, "spanish", "track.d"), "squatter", "utf8");

    expect(() => trackScript(root, "spanish")).toThrow(/exists and is not a directory/);
  });

  it("still skips one unparseable exam inventory without stopping the plan", () => {
    mkdirSync(join(root, "core"), { recursive: true });
    writeFileSync(join(root, "core", "exam-inventory-es-a1.json"), "{ not json", "utf8");
    writeJson(join(root, "core", "exam-inventory-fr-a1.json"), {
      language: "french",
      level: "a1",
    });

    const found = listExamInventories(root);
    expect(found.map((f) => f.language)).toEqual(["french"]);
  });

  it("does not let a poisoned exam inventory vanish from the plan in silence", () => {
    // Dropping this one would report the target as missing and queue somebody
    // to write an inventory that already exists.
    mkdirSync(join(root, "core"), { recursive: true });
    writeFileSync(
      join(root, "core", "exam-inventory-es-a1.json"),
      '{"language":"spanish","level":"a1","__proto__":{"x":1}}',
      "utf8",
    );

    expect(() => listExamInventories(root)).toThrow(/must not carry '__proto__'/);
  });
});

describe("a track that cannot be loaded must not simply not exist", () => {
  it("refuses a lessons-bearing directory whose name is not a usable track id", () => {
    // `loadLessons` skips non-track directories so the id check can sit at the
    // front of `loadTrackLessons`. `continue` alone would be fail-OPEN, and a
    // loader that drops a track leaves every gate green on a smaller corpus —
    // this package's recurring defect.
    mkdirSync(join(root, "_draft", "lessons"), { recursive: true });
    writeFileSync(join(root, "_draft", "lessons", "01-x.md"), "# x\n", "utf8");

    expect(() => loadLessons(root)).toThrow(/not a usable track id/);
  });

  it("still skips a non-track directory that holds no lessons", () => {
    mkdirSync(join(root, "_fonts"), { recursive: true });
    mkdirSync(join(root, "_assets"), { recursive: true });

    expect(loadLessons(root)).toEqual([]);
  });

  it("refuses a track that is a symlink rather than dropping it", (ctx) => {
    // `Dirent.isDirectory()` is FALSE for a symlink, so `spanish -> elsewhere`
    // was skipped before any id check could see it and the whole track left
    // the corpus without a word. Every other symlink encounter in this package
    // throws; this was the one that could delete a track from every gate.
    const elsewhere = join(root, "elsewhere");
    mkdirSync(join(elsewhere, "lessons"), { recursive: true });
    writeFileSync(join(elsewhere, "lessons", "01-x.md"), "# x\n", "utf8");
    try {
      symlinkSync(elsewhere, join(root, "spanish"), "dir");
    } catch {
      ctx.skip();
      return;
    }

    expect(() => loadLessons(root)).toThrow(/not a real directory but holds lessons/);
  });
});

describe("the convention, as an invariant over the source", () => {
  // A rule stated only in a comment is a rule that comes back. `loader.ts`
  // acquired seventeen of these one at a time, each perfectly reasonable on its
  // own. This is the check that makes the eighteenth fail in review.
  it("leaves no bare JSON.parse(readFileSync(...)) anywhere in src/", () => {
    const here = dirname(fileURLToPath(import.meta.url));
    const srcDir = join(here, "..", "src");
    const offenders: string[] = [];
    // A literal directory listing rather than a glob dependency: this test must
    // not be able to pass by matching nothing.
    const files = readdirSync(srcDir)
      .sort()
      .filter((name) => name.endsWith(".ts"));
    expect(files.length).toBeGreaterThan(40);

    for (const name of files) {
      const text = readFileSync(join(srcDir, name), "utf8");
      text.split("\n").forEach((line, index) => {
        // Skip prose. Every remaining occurrence in `src/` today is a comment
        // explaining why the form is banned.
        const code = line.trim();
        if (code.startsWith("//") || code.startsWith("*") || code.startsWith("/*")) return;
        if (/JSON\.parse\s*\(\s*readFileSync/.test(line)) {
          offenders.push(`${name}:${index + 1}: ${code}`);
        }
      });
    }

    expect(offenders).toEqual([]);
  });
});
