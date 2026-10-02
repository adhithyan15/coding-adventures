import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, describe, expect, it, vi } from "vitest";
import {
  RETIRED_GENTLE_RAMP_SNAPSHOT_ENTRY,
  assertGentleRampSnapshotsRetired,
} from "../src/gentle-ramp-retirement.js";
import { runGentleRampSnapshots } from "../src/gentle-ramp-snapshot-cli.js";
import { defaultCurriculumRoot } from "../src/loader.js";

const roots: string[] = [];

function temporaryRoot(): string {
  const root = mkdtempSync(join(tmpdir(), "hl-gentle-retirement-"));
  roots.push(root);
  mkdirSync(join(root, "core"));
  return root;
}

afterEach(() => {
  for (const root of roots.splice(0)) rmSync(root, { recursive: true, force: true });
  vi.restoreAllMocks();
});

describe("retired gentle-ramp snapshots", () => {
  it("accepts an absent retired path", () => {
    expect(() => assertGentleRampSnapshotsRetired(temporaryRoot())).not.toThrow();
  });

  it("rejects file, directory, and case-fold resurrection", () => {
    const fileRoot = temporaryRoot();
    writeFileSync(join(fileRoot, "core", RETIRED_GENTLE_RAMP_SNAPSHOT_ENTRY), "stale\n");
    expect(() => assertGentleRampSnapshotsRetired(fileRoot)).toThrow(/retired.*resurrected/i);

    const directoryRoot = temporaryRoot();
    mkdirSync(join(directoryRoot, "core", RETIRED_GENTLE_RAMP_SNAPSHOT_ENTRY));
    expect(() => assertGentleRampSnapshotsRetired(directoryRoot)).toThrow(/retired.*resurrected/i);

    const foldedRoot = temporaryRoot();
    mkdirSync(join(foldedRoot, "core", "Gentle-Ramp-Snapshots"));
    expect(() => assertGentleRampSnapshotsRetired(foldedRoot)).toThrow(/Gentle-Ramp-Snapshots/);
  });

  it("keeps the compatibility checker read-only", () => {
    let error = "";
    vi.spyOn(process.stderr, "write").mockImplementation((chunk) => ((error += chunk), true));
    expect(runGentleRampSnapshots(["--write"], temporaryRoot())).toBe(2);
    expect(error).toMatch(/usage:.*--check/);
  });

  it("derives the complete real report with no generated snapshot tree", () => {
    expect(() => assertGentleRampSnapshotsRetired(defaultCurriculumRoot())).not.toThrow();
    expect(runGentleRampSnapshots(["--check"])).toBe(0);
    // `--check` loads the WHOLE corpus and derives the full gap report, so this
    // test's cost grows with every tranche. It is not a fixed-size unit test
    // wearing a generous budget. It runs ~17s on an idle laptop, and CI runners
    // under full-suite load are slower still. With Bengali chapters 237-262 that
    // put it past the 30s it carried, and a PR failed on a timeout rather than on
    // anything this test asserts. 180s matches `gentle-ramp.test.ts`, which builds
    // the same report and hit the same limit first. Nothing is skipped or
    // weakened: the derivation still runs in full and must still return 0.
  }, 180_000);
});
