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
    // migration changes shape. 7204 -> 7246 was the first Spanish A2 vocabulary
    // tranche (chapters 431-436) and 7246 -> 7267 was the second (437-439) and
    // 7267 -> 7295 was the third (440-443) and
    // 7295 -> 7322 was the fourth (444-447) and
    // 7322 -> 7351 was the fifth (448-451) and
    // 7351 -> 7376 was the sixth (452-455) and
    // 7376 -> 7398 was the seventh (456-459) and
    // 7398 -> 7423 was the eighth (460-464) and
    // 7423 -> 7429 was the ninth (465, on its own) and
    // 7429 -> 7435 was the tenth (466) and
    // 7435 -> 7441 was the eleventh (467) and
    // 7441 -> 7447 was the twelfth (468) and
    // 7447 -> 7453 was the thirteenth (469) and
    // 7453 -> 7459 was the fourteenth (470) and
    // 7459 -> 7467 is the fifteenth (471), which is EIGHT rather than six because that chapter teaches six
    // headwords instead of four -- see the mock-audit test for why; the guard still does its job, which is to
    // make any OTHER change to the public graph fail loudly rather than pass quietly.
    //
    // Attribution was checked by RECONSTRUCTION, not assumed. A clean worktree
    // at the previous commit was loaded by this same function and reproduced the
    // previous digest and count byte for byte, so nothing outside the new path
    // segments moved. For 444-447 the control was origin/main, which reproduced
    // 1787ed7f... and 7295 exactly; ES-PATH-444-CASA, ES-PATH-445-PUESTO,
    // ES-PATH-446-MOVER and ES-PATH-447-JUICIO hold exactly 27 lessons between
    // them, and the corpus-wide count moved by exactly 27.
    //
    // Filtering the loaded object is NOT a substitute for that control: dropping
    // segments from an in-memory graph leaves the remaining records ordered and
    // shaped as the larger load produced them, so it reproduces neither digest.
    // Only re-loading a checkout that never had the files answers the question.
    // For 465 the control was origin/main at 35573f7d0a in a clean worktree,
    // which reproduced 06f2a8b6... and 7423 exactly; ES-PATH-465-APUNTE holds
    // exactly 6 lessons and the corpus-wide count moved by exactly 6. For 466
    // the control was origin/main at e34a62aa9f, which reproduced 4fdc1ab5...
    // and 7429 exactly; ES-PATH-466-INFORME holds exactly 6 lessons and the
    // count again moved by exactly 6. For 467 the control reproduced
    // 954755fd... and 7435 exactly, and ES-PATH-467-ALIMENTO holds 6. For 468
    // the control reproduced 5ef6e708... and 7441 exactly, and
    // ES-PATH-468-ABRIGO holds 6. For 469 the control was origin/main at
    // 4e5939c6db, which reproduced e1344996... and 7447 exactly;
    // ES-PATH-469-OFICINA holds exactly 6 lessons and the count moved by 6.
    // For 470 the control was origin/main at f3a7bed89d, which reproduced
    // 7248b982... and 7453 exactly; ES-PATH-470-PARAGUAS holds exactly 6
    // lessons and the count moved by 6. For 471 the control was origin/main at
    // 2eac85f55a, which reproduced ac4c49e1... and 7459 exactly;
    // ES-PATH-471-CURSO holds exactly 8 lessons and the count moved by 8.
    expect(digest).toBe("1218403f6db547752eea11233e9f0c185f671e858b28d8436b4cfa1156c41c64");
    expect(curricula.flatMap((curriculum) => curriculum.path).flatMap((path) => path.lessons))
      .toHaveLength(7467);
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
