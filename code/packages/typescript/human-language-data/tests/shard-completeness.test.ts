import {
  cpSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  renameSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { describe, expect, it } from "vitest";
import {
  SHARD_PLANS,
  assertNoCaseFoldCollisions,
  runShardCli,
  type ShardCompleteness,
} from "../src/shard-cli.js";
import { defaultCurriculumRoot } from "../src/loader.js";

const corpus = defaultCurriculumRoot();

function fixture(...paths: string[]): string {
  const root = mkdtempSync(join(tmpdir(), "hl-shard-completeness-"));
  for (const path of paths) {
    const target = join(root, path);
    mkdirSync(dirname(target), { recursive: true });
    cpSync(join(corpus, path), target, { recursive: true });
  }
  return root;
}

function checkThrows(root: string, path: string, message: RegExp): void {
  expect(() => runShardCli(["--check", path], root)).toThrow(message);
}

function withFixture(paths: string[], run: (root: string) => void): void {
  const root = fixture(...paths);
  try {
    run(root);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
}

describe("removed-monolith completeness declarations", () => {
  it("gives all 51 generic plans an exact independent source", () => {
    const exact = SHARD_PLANS.filter(
      (plan) =>
        plan.sections.length > 0 &&
        plan.completeness !== undefined,
    );
    expect(exact).toHaveLength(51);
    expect(
      exact.reduce<Record<ShardCompleteness["kind"], number>>(
        (counts, plan) => {
          const kind = plan.completeness!.kind;
          counts[kind] += 1;
          return counts;
        },
        {
          "curriculum-spine-union": 0,
          "generated-narration-chapters": 0,
          "curriculum-cross-references": 0,
          "script-owner-declarations": 0,
        },
      ),
    ).toEqual({
      "curriculum-spine-union": 1,
      "generated-narration-chapters": 23,
      "curriculum-cross-references": 23,
      "script-owner-declarations": 4,
    });
  });
});

describe("independent missing-owner detection", () => {
  it("detects a deleted core-spine owner from the curriculum-spine union", () => {
    withFixture(["core/spine.d", "spanish/curriculum.d"], (root) => {
      rmSync(join(root, "core/spine.d/0010-SPINE-MEET-GREET.json"));
      checkThrows(root, "core/spine.json", /missing \[SPINE-MEET-GREET\]/);
    });
  });

  it("detects a deleted chapter owner from generated narration identities", () => {
    withFixture(
      ["spanish/chapters.d", "core/generated-narration-hashes/spanish.d"],
      (root) => {
        rmSync(join(root, "spanish/chapters.d/0001.json"));
        checkThrows(root, "spanish/chapters.json", /missing \[1\]/);
      },
    );
  });

  it.each([
    ["spine", "spine/0010-SPINE-MEET-GREET.json", "SPINE-MEET-GREET"],
    ["path", "path/0010-ES-PATH-001.json", "ES-PATH-001"],
    [
      "extensions",
      "extensions/0010-ES-EXT-001-WRITING-RUNWAY.json",
      "ES-EXT-001-WRITING-RUNWAY",
    ],
  ])(
    "detects a deleted curriculum %s owner from cross-section references",
    (_section, owner, identity) => {
      withFixture(["core/spine.d", "spanish/curriculum.d"], (root) => {
        rmSync(join(root, "spanish/curriculum.d", owner));
        checkThrows(
          root,
          "spanish/curriculum.json",
          new RegExp(`missing \\[${identity}\\]`),
        );
      });
    },
  );
});

describe("logical identity and filename enforcement", () => {
  it("rejects an unexpected owner even when its filename and body agree", () => {
    withFixture(
      ["spanish/chapters.d", "core/generated-narration-hashes/spanish.d"],
      (root) => {
        const source = JSON.parse(
          readFileSync(join(root, "spanish/chapters.d/0001.json"), "utf8"),
        ) as Record<string, unknown>;
        source.chapter = 9999;
        writeFileSync(
          join(root, "spanish/chapters.d/9999.json"),
          `${JSON.stringify(source, null, 2)}\n`,
          "utf8",
        );
        checkThrows(root, "spanish/chapters.json", /unexpected \[9999\]/);
      },
    );
  });

  it("rejects two filenames that claim one logical identity", () => {
    withFixture(["core/spine.d", "spanish/curriculum.d"], (root) => {
      cpSync(
        join(root, "core/spine.d/0010-SPINE-MEET-GREET.json"),
        join(root, "core/spine.d/0015-SPINE-MEET-GREET.json"),
      );
      checkThrows(root, "core/spine.json", /duplicate logical identity/);
    });
  });

  it("rejects case-fold-colliding owner filenames", () => {
    expect(() =>
      assertNoCaseFoldCollisions(
        ["0010-SPINE-MEET-GREET.json", "0010-spine-meet-greet.json"],
        "test owners",
      ),
    ).toThrow(/collide when case-folded/);
  });

  it("binds an id-bearing filename to the identity inside its body", () => {
    withFixture(["core/spine.d", "spanish/curriculum.d"], (root) => {
      renameSync(
        join(root, "core/spine.d/0010-SPINE-MEET-GREET.json"),
        join(root, "core/spine.d/0010-SPINE-RENAMED.json"),
      );
      checkThrows(root, "core/spine.json", /bound to 'SPINE-MEET-GREET' by its body/);
    });
  });

  it("rejects a Windows-reserved identity read from an object-section filename", () => {
    withFixture(["core/spine.d", "spanish/curriculum.d"], (root) => {
      renameSync(
        join(
          root,
          "spanish/curriculum.d/spine/0010-SPINE-MEET-GREET.json",
        ),
        join(root, "spanish/curriculum.d/spine/0010-CON.json"),
      );
      checkThrows(root, "spanish/curriculum.json", /Windows reserved device name/);
    });
  });

  it("accepts a stable owner inserted at an intermediate ordinal", () => {
    withFixture(["core/spine.d", "spanish/curriculum.d"], (root) => {
      renameSync(
        join(root, "core/spine.d/0010-SPINE-MEET-GREET.json"),
        join(root, "core/spine.d/0015-SPINE-MEET-GREET.json"),
      );
      expect(runShardCli(["--check", "core/spine.json"], root)).toBe(0);
    });
  });
});
