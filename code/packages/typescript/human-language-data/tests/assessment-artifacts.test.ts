// ---------------------------------------------------------------------------
// A check that has only ever been observed PASSING has not been observed
// working. This suite is written the other way round: every assertion that the
// gate is quiet is preceded by an assertion that the same gate, on the same
// input minus one file, is loud.
//
// The failure class it guards against has appeared in this repository nine
// times in a week — a gate wired up, gone green, and later found to have been
// green because it measured nothing.
// ---------------------------------------------------------------------------
import {
  cpSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync, symlinkSync, writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { artifactExists } from "../src/artifact-presence.js";
import {
  ARTIFACT_CEILING_DIR,
  auditAssessmentArtifacts,
  auditTrackArtifacts,
  checkAssessmentArtifacts,
  collectArtifactReferences,
  serializeCeiling,
} from "../src/assessment-artifacts.js";
import { runAssessmentArtifactCli } from "../src/assessment-artifact-cli.js";
import { defaultCurriculumRoot, loadAssessmentContracts } from "../src/loader.js";

// --- fixture -----------------------------------------------------------------

const LEVELS = ["pre-A1", "A1", "A2", "B1", "B2", "C1", "C2"] as const;

/** Cumulative writing stages the real policy requires at each level. */
const STAGES: Record<(typeof LEVELS)[number], string[]> = {
  "pre-A1": ["observe-trace", "guided-copy", "delayed-copy", "dictation-transcription"],
  A1: [
    "observe-trace", "guided-copy", "delayed-copy", "dictation-transcription",
    "controlled-composition", "timed-assessment-production",
  ],
  A2: [
    "observe-trace", "guided-copy", "delayed-copy", "dictation-transcription",
    "controlled-composition", "connected-composition", "timed-assessment-production",
  ],
  B1: [], B2: [], C1: [], C2: [],
};
for (const level of ["B1", "B2", "C1", "C2"] as const) STAGES[level] = STAGES.A2;

function contract(language: string): unknown {
  return {
    version: 1,
    language,
    levels: LEVELS.map((level) => {
      const slug = level.toLowerCase();
      const skill = (name: string) => ({
        taskInventory: [`task-shapes/${slug}.json#${name}`],
        passThreshold: 0.6,
      });
      return {
        level,
        target: { name: `Fixture ${level}`, basis: "project-defined", source: "assessment-spec.md" },
        skills: {
          reading: skill("reading"),
          listening: skill("listening"),
          writing: skill("writing"),
          speaking: skill("speaking"),
        },
        writingStages: STAGES[level],
        fullMocks: [1, 2].map((n) => ({
          id: `${slug}-mock-${n}`,
          timed: true,
          rubric: `mocks/${slug}/rubric.md`,
          answerKey: `mocks/${slug}/mock-${n}-answer-key.md`,
        })),
      };
    }),
  };
}

/** Every distinct path the fixture contract promises: 7 inventories + 7 rubrics + 14 keys. */
function fixturePaths(): string[] {
  const out: string[] = [];
  for (const level of LEVELS) {
    const slug = level.toLowerCase();
    out.push(`task-shapes/${slug}.json`, `mocks/${slug}/rubric.md`);
    out.push(`mocks/${slug}/mock-1-answer-key.md`, `mocks/${slug}/mock-2-answer-key.md`);
  }
  return out.sort();
}

let root: string;

function write(relative: string, contents: string): void {
  const path = join(root, relative);
  mkdirSync(join(path, ".."), { recursive: true });
  writeFileSync(path, contents, "utf8");
}

/** Materialize every artifact the fixture contract promises. */
function satisfy(language: string, except: string[] = []): void {
  for (const path of fixturePaths()) {
    if (except.includes(path)) continue;
    write(`${language}/${path}`, path.endsWith(".json") ? "{}\n" : "# fixture\n");
  }
}

beforeEach(() => {
  root = mkdtempSync(join(tmpdir(), "hl-assessment-artifacts-"));
  mkdirSync(join(root, "core"), { recursive: true });
  // The real policy, copied rather than retyped: a fixture policy that drifts
  // from the shipped one would let this suite pass a contract CI would reject.
  cpSync(
    join(defaultCurriculumRoot(), "core", "assessment-policy.json"),
    join(root, "core", "assessment-policy.json"),
  );
  write("core/languages.json", `${JSON.stringify({
    version: 1,
    languages: [{ id: "fixtura", name: "Fixtura", family: "Test", script: "latin", status: "active", bridges: [] }],
  }, null, 2)}\n`);
  write("fixtura/assessment.json", `${JSON.stringify(contract("fixtura"), null, 2)}\n`);
});

afterEach(() => {
  rmSync(root, { recursive: true, force: true });
});

// --- the check bites ---------------------------------------------------------

describe("dangling assessment-contract references", () => {
  it("RED: reports a contract that promises a file nobody wrote", () => {
    // Nothing on disk but the contract. Every promise dangles.
    const report = auditAssessmentArtifacts(root);
    expect(report.summary.tracksAudited).toBe(1);
    expect(report.tracks[0]!.missing).toEqual(fixturePaths());
    expect(report.tracks[0]!.present).toEqual([]);

    // And the CHECK — not just the audit — fails, with one diagnostic per file
    // and the field that promised it named, because an author needs the list of
    // files rather than the news that a file is absent.
    const result = checkAssessmentArtifacts(root);
    expect(result.diagnostics).toHaveLength(fixturePaths().length);
    expect(new Set(result.diagnostics.map((d) => d.kind))).toEqual(new Set(["new-dangling-reference"]));
    expect(result.diagnostics.map((d) => d.message).join("\n")).toContain(
      "A1.fullMocks[a1-mock-1].answerKey promises 'mocks/a1/mock-1-answer-key.md'",
    );
    // The CLI must translate that into a non-zero exit. A checker whose failure
    // never reaches the process exit code is a checker CI cannot fail on.
    expect(runAssessmentArtifactCli(["--check"], root, () => {}, () => {}, () => {})).toBe(1);
  });

  it("GREEN: the counterfactual — a satisfied contract passes", () => {
    // The same contract, same checker, every promised file present. This is the
    // assertion that stops the RED case above from being satisfied by a checker
    // that simply always fails.
    satisfy("fixtura");
    const report = auditAssessmentArtifacts(root);
    expect(report.tracks[0]!.missing).toEqual([]);
    expect(report.tracks[0]!.present).toEqual(fixturePaths());
    const result = checkAssessmentArtifacts(root);
    expect(result.diagnostics).toEqual([]);
    expect(runAssessmentArtifactCli(["--check"], root, () => {}, () => {}, () => {})).toBe(0);
  });

  it("catches exactly the file that was removed, and no other", () => {
    // Sharper than "some diagnostic fired": one file deleted, one diagnostic,
    // naming that file. A checker that reported the whole track would pass a
    // coarser assertion and be useless to whoever has to fix it.
    satisfy("fixtura", ["mocks/b2/mock-2-answer-key.md"]);
    const result = checkAssessmentArtifacts(root);
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0]!.kind).toBe("new-dangling-reference");
    expect(result.diagnostics[0]!.message).toContain("mocks/b2/mock-2-answer-key.md");
    expect(result.diagnostics[0]!.message).toContain("B2.fullMocks[b2-mock-2].answerKey");
  });

  it("checks the fragment's FILE, not its section", () => {
    // `task-shapes/a1.json#reading` names a section inside a file. This module
    // asks only whether the file is there; whether the section is is
    // task-shapes.ts's question, and conflating them would make two different
    // repairs report identically.
    const references = collectArtifactReferences(loadAssessmentContracts(root)[0]!.contract);
    const inventory = references.find((r) => r.reference === "task-shapes/a1.json#reading")!;
    expect(inventory.reference).toBe("task-shapes/a1.json#reading");
    expect(inventory.path).toBe("task-shapes/a1.json");
  });
});

// --- the ratchet -------------------------------------------------------------

describe("the pinned ceiling", () => {
  function pin(language: string, paths: string[]): void {
    write(
      `${ARTIFACT_CEILING_DIR}/${language}.json`,
      serializeCeiling({ version: 1, language, unbuiltArtifacts: [...paths].sort() }),
    );
  }

  it("lets pinned debt through and stops the very next dangler", () => {
    // Two files unbuilt; only one pinned. The pinned one is silent, the other
    // fails. This is the whole contract of a ceiling in one assertion: it is a
    // record of debt already taken, never a licence to take more.
    satisfy("fixtura", ["mocks/c1/rubric.md", "mocks/c2/rubric.md"]);
    pin("fixtura", ["mocks/c1/rubric.md"]);
    const result = checkAssessmentArtifacts(root);
    expect(result.pinnedUnbuilt).toBe(1);
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0]!.kind).toBe("new-dangling-reference");
    expect(result.diagnostics[0]!.message).toContain("mocks/c2/rubric.md");
    expect(result.diagnostics[0]!.message).not.toContain("mocks/c1/rubric.md");
  });

  it("refuses a swap: one debt paid, one taken, same total", () => {
    // The reason the pin is a SET and not a count. Both corpora owe exactly one
    // artifact, so any count-based ceiling waves this through.
    satisfy("fixtura", ["mocks/c2/rubric.md"]);
    pin("fixtura", ["mocks/c1/rubric.md"]);
    const kinds = checkAssessmentArtifacts(root).diagnostics.map((d) => `${d.kind}`).sort();
    expect(kinds).toEqual(["ceiling-has-fallen", "new-dangling-reference"]);
  });

  it("fails when a pinned debt is paid, so the ceiling cannot rot into a floor", () => {
    // A ceiling nobody is forced to lower stops distinguishing "still owed" from
    // "paid two years ago". Note the message sends the author to the OPPOSITE
    // action from the dangling case — regenerate, do not delete the artifact —
    // which is why the two are separate diagnostic kinds.
    satisfy("fixtura");
    pin("fixtura", ["mocks/c1/rubric.md"]);
    const result = checkAssessmentArtifacts(root);
    expect(result.diagnostics).toHaveLength(1);
    expect(result.diagnostics[0]!.kind).toBe("ceiling-has-fallen");
    expect(result.diagnostics[0]!.message).toContain("now exists");
    expect(result.diagnostics[0]!.message).toContain("generate:assessment-artifacts");
  });

  it("fails when a pin survives the track paying every debt", () => {
    satisfy("fixtura");
    pin("fixtura", []);
    const result = checkAssessmentArtifacts(root);
    expect(result.diagnostics.map((d) => d.kind)).toEqual(["stale-ceiling-file"]);
  });

  it("--write lowers the ceiling to exactly what is still unbuilt", () => {
    satisfy("fixtura", ["mocks/c2/rubric.md"]);
    const written = new Map<string, string>();
    expect(runAssessmentArtifactCli(["--write"], root, (path, body) => written.set(path, body), () => {}, () => {}))
      .toBe(0);
    expect(written.size).toBe(1);
    const [[path, body]] = [...written];
    expect(path.replaceAll("\\", "/")).toContain(`${ARTIFACT_CEILING_DIR}/fixtura.json`);
    expect(JSON.parse(body).unbuiltArtifacts).toEqual(["mocks/c2/rubric.md"]);
    // And the generated file is one the checker then accepts — otherwise the
    // remedy the failure message prescribes would not actually work.
    write(`${ARTIFACT_CEILING_DIR}/fixtura.json`, body);
    expect(checkAssessmentArtifacts(root).diagnostics).toEqual([]);
  });

  it("rejects a hand-written pin whose shape is wrong", () => {
    write(`${ARTIFACT_CEILING_DIR}/fixtura.json`, `${JSON.stringify({ version: 2, language: "fixtura", unbuiltArtifacts: [] })}\n`);
    expect(() => checkAssessmentArtifacts(root)).toThrow(/ceiling version must be 1/);
    write(`${ARTIFACT_CEILING_DIR}/fixtura.json`, `${JSON.stringify({ version: 1, language: "other", unbuiltArtifacts: [] })}\n`);
    expect(() => checkAssessmentArtifacts(root)).toThrow(/declares language 'other'/);
    write(`${ARTIFACT_CEILING_DIR}/fixtura.json`, `${JSON.stringify({ version: 1, language: "fixtura", unbuiltArtifacts: [7] })}\n`);
    expect(() => checkAssessmentArtifacts(root)).toThrow(/must be a string array/);
  });
});

// --- absent versus could-not-determine ---------------------------------------

describe("artifactExists distinguishes absent from undetermined", () => {
  const errno = (code: string): Error => Object.assign(new Error(code), { code });
  const realFile = () => ({ isSymbolicLink: () => false });
  const link = () => ({ isSymbolicLink: () => true });

  it("treats ENOENT and ENOTDIR as absent", () => {
    expect(artifactExists("/nowhere", () => { throw errno("ENOENT"); })).toBe(false);
    expect(artifactExists("/nowhere", () => { throw errno("ENOTDIR"); })).toBe(false);
  });

  it("treats a successful stat as present", () => {
    expect(artifactExists("/somewhere", realFile)).toBe(true);
  });

  it("REFUSES a symlink rather than counting it as the artifact", () => {
    // The probe is `lstat`, not `stat`, so `mocks/a1/rubric.md -> ../../README.md`
    // cannot satisfy the presence gate and report the debt paid. It throws
    // instead of answering false, because "present but not admissible" is a
    // third answer and a boolean would collapse it the same way `existsSync`
    // collapses "I was not allowed to look".
    expect(() => artifactExists("/linked", link)).toThrow(/is a symbolic link/);
  });

  it("THROWS on any other errno, naming it", () => {
    // The bug fixed twice already (#12731, #12734): `catch { return false }`
    // reports "the file is not there" when the truthful answer is "I was not
    // allowed to look". Those need different responses from a human, so they
    // must not arrive as the same boolean.
    for (const code of ["EACCES", "EPERM", "EMFILE", "EIO", "ELOOP"]) {
      expect(() => artifactExists("/guarded", () => { throw errno(code); }))
        .toThrow(new RegExp(`could not determine.*${code}`, "s"));
    }
  });

  it("THROWS when the thrown value carries no errno at all", () => {
    expect(() => artifactExists("/odd", () => { throw new Error("something else"); }))
      .toThrow(/no errno on the thrown value/);
  });

  it("propagates that refusal through the audit rather than counting a missing file", () => {
    // The end-to-end shape of the same point: an I/O fault must stop the audit,
    // not silently inflate the debt it reports.
    expect(() => auditTrackArtifacts(root, "fixtura", loadAssessmentContracts(root)[0]!.contract, () => {
      throw errno("EACCES");
    })).toThrow(/EACCES/);
  });
});

// --- anti-vacuity ------------------------------------------------------------

describe("the gate cannot pass by measuring nothing", () => {
  it("fails when no contract is found", () => {
    write("core/languages.json", `${JSON.stringify({ version: 1, languages: [] }, null, 2)}\n`);
    const result = checkAssessmentArtifacts(root);
    expect(result.diagnostics.map((d) => d.kind)).toEqual(["audit-went-blind"]);
    expect(runAssessmentArtifactCli(["--check"], root, () => {}, () => {}, () => {})).toBe(1);
  });

  it("rejects an unsafe track id before joining it to a path", () => {
    const { contract: parsed } = loadAssessmentContracts(root)[0]!;
    expect(() => auditTrackArtifacts(root, "../escape", parsed)).toThrow(/unsafe track id/);
  });

  it("rejects an unsafe track id in the REGISTRY, before anything is read", () => {
    // `loadLanguageRegistry` is an unchecked cast over core/languages.json, so a
    // pull request could put `"id": "../../../../etc"` there and have CI stat
    // and read a path outside the tree. The guard in `auditTrackArtifacts` above
    // is too late — it runs on contracts the loader has already opened.
    write("core/languages.json", `${JSON.stringify({
      version: 1,
      languages: [{ id: "../../../../etc", name: "x", family: "x", script: "latin", status: "active", bridges: [] }],
    }, null, 2)}\n`);
    expect(() => loadAssessmentContracts(root)).toThrow(/unsafe track id/);
  });
});

describe("the generator refuses to write through a symlink", () => {
  // `resolve()` is lexical and `writeFileSync` follows a link, so a committed
  // `core/assessment-artifact-ceiling/fixtura.json -> ../../../.git/hooks/post-checkout`
  // would make `npm run generate:assessment-artifacts` an arbitrary write. Both
  // halves are guarded: the file and the directory it sits in.
  //
  // Symlink creation needs a privilege Windows does not always grant, and it
  // grants them SEPARATELY — a directory junction needs none, a file symlink
  // needs SeCreateSymbolicLinkPrivilege. So the probe must use the same link
  // type the case does; probing with a junction and then creating a file link
  // is how the first version of this test "skipped" by throwing EPERM.
  //
  // Where the privilege really is absent the case skips itself, and the skip is
  // visible — asserting the platform is the one where that is legitimate, so a
  // Linux runner that lost symlink support fails instead of going quiet. A
  // suite that stops exercising its security cases without saying so is the
  // failure mode this whole file is written against.
  const canSymlink = (type: "file" | "junction"): boolean => {
    const probe = join(root, `probe-${type}`);
    const target = type === "junction" ? join(root, "core") : join(root, "core", "languages.json");
    try {
      symlinkSync(target, probe, type);
      rmSync(probe, { force: true });
      return true;
    } catch {
      return false;
    }
  };

  it("refuses a symlinked ceiling FILE", () => {
    if (!canSymlink("file")) return void expect(process.platform).toBe("win32");
    const victim = join(root, "victim.txt");
    writeFileSync(victim, "original\n", "utf8");
    mkdirSync(join(root, ARTIFACT_CEILING_DIR), { recursive: true });
    symlinkSync(victim, join(root, ARTIFACT_CEILING_DIR, "fixtura.json"), "file");
    expect(() => runAssessmentArtifactCli(["--write"], root, undefined, () => {}, () => {}))
      .toThrow(/not a regular file — refusing to write through it/);
    expect(readFileSync(victim, "utf8")).toBe("original\n");
  });

  it("refuses a symlinked ceiling DIRECTORY", () => {
    if (!canSymlink("junction")) return void expect(process.platform).toBe("win32");
    const elsewhere = join(root, "elsewhere");
    mkdirSync(elsewhere, { recursive: true });
    mkdirSync(join(root, "core"), { recursive: true });
    symlinkSync(elsewhere, join(root, ARTIFACT_CEILING_DIR), "junction");
    expect(() => runAssessmentArtifactCli(["--write"], root, undefined, () => {}, () => {}))
      .toThrow(/not a real directory — refusing to write through it/);
    expect(readdirSync(elsewhere)).toEqual([]);
  });

  it("refuses an ANCESTOR symlink, which lstat alone cannot see", () => {
    // `lstat` vets only the LAST component. The case above links
    // `core/assessment-artifact-ceiling`; this one links `core` itself, so the
    // lstat lands on a real directory inside the linked-to tree, returns
    // isDirectory() === true, and every shard is written outside the root. Only
    // resolving the whole chain catches it — which is why `writeCeilingFile`
    // carries BOTH halves of book-cli's guard rather than just the cheap one.
    //
    // Self-consistent attack: with `core` linked, `loadLanguageRegistry` reads
    // `core/languages.json` through the link too, so the fixture is set up the
    // way a real one would be — the registry that drives the write lives in the
    // linked-to tree.
    if (!canSymlink("junction")) return void expect(process.platform).toBe("win32");
    const outside = mkdtempSync(join(tmpdir(), "hl-outside-"));
    try {
      cpSync(join(root, "core"), outside, { recursive: true });
      rmSync(join(root, "core"), { recursive: true, force: true });
      symlinkSync(outside, join(root, "core"), "junction");
      expect(() => runAssessmentArtifactCli(["--write"], root, undefined, () => {}, () => {}))
        .toThrow(/resolves outside the curriculum root/);
      expect(readdirSync(outside).sort()).toEqual(["assessment-policy.json", "languages.json"]);
    } finally {
      rmSync(join(root, "core"), { recursive: true, force: true });
      rmSync(outside, { recursive: true, force: true });
    }
  });
});

// --- the live corpus ---------------------------------------------------------

describe("the real corpus", () => {
  it("carries no unpinned dangling assessment-contract reference", { timeout: 60_000 }, () => {
    const result = checkAssessmentArtifacts(defaultCurriculumRoot());
    // Named, not counted. If this regresses, the failure should say which file
    // rather than making the next reader re-run the query by hand.
    expect(result.diagnostics.map((d) => `${d.kind}: ${d.message}`)).toEqual([]);

    // Anti-vacuity for the assertion above: it would also pass if the loader
    // found no contracts at all. These numbers are the corpus this gate was
    // built against and are expected to MOVE — they are floors, not pins, so
    // authoring a mock does not fail this line. The pins that must be exact
    // live in core/assessment-artifact-ceiling/.
    expect(result.report.summary.tracksAudited).toBeGreaterThanOrEqual(13);
    expect(result.report.summary.referencesChecked).toBeGreaterThanOrEqual(700);
  });
});
