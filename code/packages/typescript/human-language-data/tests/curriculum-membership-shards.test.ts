import { existsSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { afterEach, describe, expect, it } from "vitest";
import {
  attachCurriculumLessonMemberships,
  type AuthoredLanguageCurriculum,
} from "../src/curriculum-membership.js";
import {
  readCurriculumMembershipOwners,
} from "../src/curriculum-membership-shards.js";
import {
  defaultCurriculumRoot,
  loadAuthoredLanguageCurricula,
  loadLanguageCurricula,
} from "../src/loader.js";

const roots: string[] = [];
afterEach(() => {
  for (const root of roots.splice(0)) rmSync(root, { recursive: true, force: true });
});

function fixture(): string {
  const root = mkdtempSync(join(tmpdir(), "hl-curriculum-membership-"));
  roots.push(root);
  mkdirSync(join(root, "toy", "lessons"), { recursive: true });
  mkdirSync(join(root, "toy", "curriculum-membership.d"), { recursive: true });
  writeFileSync(join(root, "toy", "lessons", "TOY-L1.md"), "---\nid: TOY-L1\n---\n");
  writeFileSync(
    join(root, "toy", "curriculum-membership.d", "_meta.json"),
    `${JSON.stringify({ version: 1, language: "toy" }, null, 2)}\n`,
  );
  return root;
}

function authored(): AuthoredLanguageCurriculum {
  return {
    version: 1,
    language: "toy",
    path: [
      {
        id: "TOY-PATH-1",
        spine_node: "SPINE-1",
        before: [],
        inline: ["TOY-EXT-1"],
        after: [],
      },
    ],
    spine: {
      "SPINE-1": { segments: ["TOY-PATH-1"], omits: [], relocates: {} },
    },
    extensions: [
      {
        id: "TOY-EXT-1",
        stage: "A1",
        kind: "required",
        category: "grammar",
        canDo: "I can test the direct owner.",
        prerequisites: [],
      },
    ],
  };
}

describe("direct curriculum lesson owners", () => {
  // The canonical membership shards are already direct owners in git. A
  // tracked hash of their assembled graph is derived state, and every lesson
  // addition in one track used to rewrite that track's single aggregate pin.
  // Prove the useful property directly instead: repeated assembly is stable,
  // every loaded lesson is uniquely owned, and the loader's fail-closed checks
  // below remain the authority for malformed ownership.
  it("assembles every real track deterministically from unique direct owners", () => {
    const first = loadLanguageCurricula(defaultCurriculumRoot());
    const second = loadLanguageCurricula(defaultCurriculumRoot());

    expect(first.length).toBeGreaterThan(0);
    expect(second).toEqual(first);
    for (const curriculum of first) {
      const lessons = curriculum.path.flatMap((path) => path.lessons);
      expect(new Set(lessons).size, `${curriculum.language}: duplicate lesson owner`)
        .toBe(lessons.length);
    }
  });

  it("does not reintroduce mutable whole-track digest snapshots", () => {
    const retired = fileURLToPath(new URL("./curriculum-digests/", import.meta.url));
    expect(existsSync(retired)).toBe(false);
  });

  it("keeps the canonical curriculum shards free of derived lesson arrays", () => {
    for (const curriculum of loadAuthoredLanguageCurricula(defaultCurriculumRoot())) {
      expect(curriculum.path.every((path) => !Object.hasOwn(path, "lessons"))).toBe(true);
      expect(curriculum.extensions.every((extension) => !Object.hasOwn(extension, "lessons"))).toBe(true);
    }
  });

  it("requires one canonical owner for every lesson filename", () => {
    const root = fixture();
    expect(() => readCurriculumMembershipOwners(root, "toy")).toThrow(
      /missing: TOY-L1\.json/,
    );
    writeFileSync(
      join(root, "toy", "curriculum-membership.d", "TOY-L1.json"),
      `${JSON.stringify({
        id: "TOY-L1",
        pathSegment: "TOY-PATH-1",
        pathOrder: 0,
        extensions: [{ id: "TOY-EXT-1", order: 0 }],
      }, null, 2)}\n`,
    );
    expect(readCurriculumMembershipOwners(root, "toy").owners).toHaveLength(1);
  });

  it("rejects aggregate lesson-array resurrection", () => {
    const input = authored();
    (input.path[0] as unknown as { lessons: string[] }).lessons = ["TOY-L1"];
    expect(() => attachCurriculumLessonMemberships(input, [])).toThrow(
      /must not store derived 'lessons'/,
    );
  });

  it("rejects gaps in per-path order", () => {
    expect(() =>
      attachCurriculumLessonMemberships(authored(), [
        { id: "TOY-L1", pathSegment: "TOY-PATH-1", pathOrder: 1, extensions: [] },
      ]),
    ).toThrow(/expected 0/);
  });

  it("rejects extension membership outside the lesson's path", () => {
    const input = authored();
    input.path[0]!.inline = [];
    expect(() =>
      attachCurriculumLessonMemberships(input, [
        {
          id: "TOY-L1",
          pathSegment: "TOY-PATH-1",
          pathOrder: 0,
          extensions: [{ id: "TOY-EXT-1", order: 0 }],
        },
      ]),
    ).toThrow(/is not attached to path segment/);
  });
});
