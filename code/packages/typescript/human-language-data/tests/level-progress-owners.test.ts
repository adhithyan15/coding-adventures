import {
  mkdtempSync,
  mkdirSync,
  readFileSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import {
  assertLevelProgressOwners,
  loadLevelProgressOwners,
} from "./level-gate-attainment-pins.js";

const owner = (
  attained: string | null,
  reach: string | null,
  evidence = "Owned chapters establish the current measured level.",
) => JSON.stringify({ attained, reach, evidence });

const inTemporaryDirectory = (run: (root: string) => void): void => {
  const root = mkdtempSync(join(tmpdir(), "level-progress-owners-"));
  try {
    run(root);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
};

describe("per-track level-progress owners", () => {
  it("loads one exact attained, reach, and evidence record per expected track", () => {
    inTemporaryDirectory((root) => {
      writeFileSync(join(root, "chinese.json"), owner("A2", "A2"));
      writeFileSync(join(root, "spanish.json"), owner("B1", "C2"));

      expect(loadLevelProgressOwners(root, ["spanish", "chinese"])).toEqual({
        chinese: {
          attained: "A2",
          reach: "A2",
          evidence: "Owned chapters establish the current measured level.",
        },
        spanish: {
          attained: "B1",
          reach: "C2",
          evidence: "Owned chapters establish the current measured level.",
        },
      });
    });
  });

  it("establishes exact identity closure before opening owner bytes", () => {
    inTemporaryDirectory((root) => {
      writeFileSync(join(root, "spanish.json"), "not json");
      expect(() => loadLevelProgressOwners(root, ["spanish", "tamil"])).toThrow(
        /missing.*tamil\.json/i,
      );
    });
  });

  it("rejects empty, unsafe, nested, uppercase, and case-fold-colliding owner sets", () => {
    inTemporaryDirectory((root) => {
      expect(() => loadLevelProgressOwners(root, [])).toThrow(/must not be empty/i);
    });
    inTemporaryDirectory((root) => {
      writeFileSync(join(root, "_meta.json"), owner("A1", "A1"));
      expect(() => loadLevelProgressOwners(root, ["_meta"])).toThrow(/unsafe/i);
    });
    inTemporaryDirectory((root) => {
      mkdirSync(join(root, "spanish.json"));
      expect(() => loadLevelProgressOwners(root, ["spanish"])).toThrow(/real file/i);
    });
    inTemporaryDirectory((root) => {
      writeFileSync(join(root, "Spanish.json"), owner("A1", "A1"));
      expect(() => loadLevelProgressOwners(root, ["spanish"])).toThrow(/lowercase/i);
    });
    inTemporaryDirectory((root) => {
      writeFileSync(join(root, "spanish.json"), owner("A1", "A1"));
      writeFileSync(join(root, "SPANISH.json"), owner("A1", "A1"));
      expect(() => loadLevelProgressOwners(root, ["spanish"])).toThrow(/case-fold/i);
    });
  });

  it("rejects an owner symlink without opening its target", () => {
    if (process.platform === "win32") return;
    inTemporaryDirectory((root) => {
      const outside = join(tmpdir(), `level-progress-outside-${process.pid}.json`);
      writeFileSync(outside, owner("A1", "A1"));
      try {
        symlinkSync(outside, join(root, "spanish.json"));
        expect(() => loadLevelProgressOwners(root, ["spanish"])).toThrow(/real file/i);
      } finally {
        rmSync(outside, { force: true });
      }
    });
  });

  it("rejects unknown fields, unsafe levels, empty evidence, and oversized evidence", () => {
    inTemporaryDirectory((root) => {
      writeFileSync(
        join(root, "spanish.json"),
        JSON.stringify({ attained: "A1", reach: "A1", evidence: "ok", extra: true }),
      );
      expect(() => loadLevelProgressOwners(root, ["spanish"])).toThrow(/exactly/i);
    });
    inTemporaryDirectory((root) => {
      writeFileSync(join(root, "spanish.json"), owner("A9", "C2"));
      expect(() => loadLevelProgressOwners(root, ["spanish"])).toThrow(/unsafe attained/i);
    });
    inTemporaryDirectory((root) => {
      writeFileSync(join(root, "spanish.json"), owner("A1", "A9"));
      expect(() => loadLevelProgressOwners(root, ["spanish"])).toThrow(/unsafe reach/i);
    });
    inTemporaryDirectory((root) => {
      writeFileSync(join(root, "spanish.json"), owner("A1", "A1", "  "));
      expect(() => loadLevelProgressOwners(root, ["spanish"])).toThrow(/evidence/i);
    });
    inTemporaryDirectory((root) => {
      writeFileSync(join(root, "spanish.json"), owner("A1", "A1", "x".repeat(501)));
      expect(() => loadLevelProgressOwners(root, ["spanish"])).toThrow(/500/i);
    });
  });

  it("rejects an impossible owner whose reach ranks below attainment", () => {
    inTemporaryDirectory((root) => {
      writeFileSync(join(root, "spanish.json"), owner("B1", "A2"));
      expect(() => loadLevelProgressOwners(root, ["spanish"])).toThrow(/below attained/i);
    });
  });

  it("names the track, field, expected value, and actual value on a mismatch", () => {
    expect(() =>
      assertLevelProgressOwners(
        {
          spanish: {
            attained: "B1",
            reach: "C2",
            evidence: "Owned chapters establish the current measured level.",
          },
        },
        { spanish: { attained: "A2", reach: "C2" } },
      ),
    ).toThrow(/spanish.*attained.*B1.*A2/i);
  });
});

describe("shared progression tests remain generic", () => {
  it("does not restore track advancement diaries or exact track arrays", () => {
    const root = new URL(".", import.meta.url);
    for (const name of ["level-gate.test.ts", "levels.test.ts"]) {
      const source = readFileSync(new URL(name, root), "utf8");
      expect(source).not.toMatch(/\/\/.*\b(?:followed|joins? the list|moved it from)\b/i);
      expect(source).not.toMatch(/toEqual\(\[\s*"(?:arabic|bengali|chinese)"/s);
    }
  });
});
