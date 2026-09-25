import { createHash } from "node:crypto";
import { mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from "node:fs";
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

const DIGEST_DIR = fileURLToPath(new URL("./curriculum-digests/", import.meta.url));

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
  // ONE PIN PER TRACK, ONE FILE PER PIN (HL-C442).
  //
  // This used to pin a single SHA-256 and a single lesson count over the whole
  // live graph. Both moved whenever ANY track gained a lesson, so every content
  // PR on every track edited the same two lines and any two of them in flight
  // conflicted -- the shared-hot-file problem HL21-HL40 removed everywhere else,
  // still present in the test that guards that work. The history of every move
  // (7204 -> 7572) lives in git, in this file's log.
  //
  // Now each track's own graph is pinned in `curriculum-digests/<track>.json`.
  // A PR that adds Telugu lessons edits telugu.json and nothing else, and the
  // diff of that file IS the attribution: which track moved, and by how many
  // lessons. After a deliberate change, rewrite the pins with
  //
  //     UPDATE_CURRICULUM_DIGESTS=1 npx vitest run tests/curriculum-membership-shards.test.ts
  //
  // and say in the commit why the count moved. The set of pin files must equal
  // the set of tracks, so a new or removed track cannot slip past either.
  it("pins each track's curriculum graph in its own file", () => {
    const curricula = loadLanguageCurricula(defaultCurriculumRoot());
    const actual = new Map(
      curricula.map((curriculum) => [
        curriculum.language,
        {
          digest: createHash("sha256").update(JSON.stringify(curriculum)).digest("hex"),
          lessons: curriculum.path.flatMap((path) => path.lessons).length,
        },
      ]),
    );
    if (process.env.UPDATE_CURRICULUM_DIGESTS === "1") {
      mkdirSync(DIGEST_DIR, { recursive: true });
      for (const name of readdirSync(DIGEST_DIR)) {
        if (name.endsWith(".json") && !actual.has(name.slice(0, -".json".length))) {
          rmSync(join(DIGEST_DIR, name));
        }
      }
      for (const [language, pin] of actual) {
        writeFileSync(join(DIGEST_DIR, `${language}.json`), `${JSON.stringify(pin, null, 2)}\n`);
      }
    }
    const pinned = readdirSync(DIGEST_DIR)
      .filter((name) => name.endsWith(".json"))
      .map((name) => name.slice(0, -".json".length))
      .sort();
    expect(pinned).toEqual([...actual.keys()].sort());
    for (const [language, pin] of actual) {
      const expected = JSON.parse(readFileSync(join(DIGEST_DIR, `${language}.json`), "utf8"));
      expect(pin, `${language}: rewrite curriculum-digests/${language}.json and say why it moved`)
        .toEqual(expected);
    }
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
