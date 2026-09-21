import { createHash } from "node:crypto";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
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
  it("reconstructs the exact pre-migration public graph", () => {
    const curricula = loadLanguageCurricula(defaultCurriculumRoot());
    const digest = createHash("sha256")
      .update(JSON.stringify(curricula))
      .digest("hex");
    // The digest and count below are of the LIVE curriculum graph, so they move
    // whenever any track gains a lesson -- not only when the membership
    // migration changes shape. 7200 -> 7204 and a new digest are Malayalam
    // chapter 112's four lessons; the guard still does its job, which is to make
    // any OTHER change to the public graph fail loudly rather than pass quietly.
    // Attribution was checked, not assumed: ML-PATH-112-PLACES holds exactly
    // ML-C112-bank, ML-C112-poleesu, ML-C112-aashupathri, ML-R112-places-recall
    // and nothing else, and the count moved by exactly four. (ML-S112-letter-pa
    // matches a naive /112/ search but is a SCRIPT lesson on ML-PATH-100, the
    // same false match ML-S111-letter-ca produced last chapter.)
    expect(digest).toBe("9f13e9a970c45f7b0c354dee7e1f0812d37d8834917fd9e935105280bdfc098c");
    expect(curricula.flatMap((curriculum) => curriculum.path).flatMap((path) => path.lessons))
      .toHaveLength(7204);
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
