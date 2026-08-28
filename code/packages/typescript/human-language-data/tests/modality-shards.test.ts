import {
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import {
  assertModalityManifestLanguages,
  modalityOwnerContents,
  readModalityManifestOwners,
} from "../src/modality-shards.js";
import {
  buildModalityManifest,
  type ModalityManifest,
} from "../src/modality-manifest.js";
import { parseLesson, type ParsedLesson } from "../src/parse.js";

const roots: string[] = [];

function lesson(options: {
  id: string;
  language: string;
  sequence?: number;
  modality?: string;
  body?: string;
}): ParsedLesson {
  return parseLesson(
    `---
schema_version: 2
id: ${options.id}
chapter: 1
sequence: ${options.sequence ?? 10}
type: word
headword: hello
gloss: hello
concept_tag: GREETING-HELLO
${options.modality === undefined ? "" : `modality: ${options.modality}`}
---

# ${options.id}

${options.body ?? "Say hello out loud."}
`,
    options.language,
  );
}

function manifest(): ModalityManifest {
  return buildModalityManifest([
    lesson({ id: "ES-C01-hola", language: "spanish", sequence: 10 }),
    lesson({ id: "ES-C01-adios", language: "spanish", sequence: 20 }),
    lesson({
      id: "TA-C01-vanakkam",
      language: "tamil",
      sequence: 10,
      modality: "sight",
    }),
  ]);
}

function temporaryRoot(): string {
  const root = mkdtempSync(join(tmpdir(), "hl-modality-shards-"));
  roots.push(root);
  mkdirSync(join(root, "core", "lesson-modality"), { recursive: true });
  return root;
}

function writeOwners(root: string, value: ModalityManifest = manifest()): void {
  for (const [relative, contents] of modalityOwnerContents(value)) {
    const path = join(root, "core", "lesson-modality", relative);
    mkdirSync(join(path, ".."), { recursive: true });
    writeFileSync(path, contents, "utf8");
  }
}

function expectedIds(
  value: ModalityManifest = manifest(),
): ReadonlyMap<string, readonly string[]> {
  const byLanguage = new Map<string, string[]>();
  for (const entry of value.lessons) {
    const ids = byLanguage.get(entry.language);
    if (ids) ids.push(entry.id);
    else byLanguage.set(entry.language, [entry.id]);
  }
  for (const ids of byLanguage.values()) ids.sort();
  return byLanguage;
}

function readOwners(root: string, value: ModalityManifest = manifest()): ModalityManifest {
  const ids = expectedIds(value);
  return readModalityManifestOwners(root, {
    expectedLanguages: [...ids.keys()].sort(),
    expectedLessonIds: ids,
    expectedNarrationLessonIds: ids,
  });
}

function owner(root: string, language: string, name: string): string {
  return join(root, "core", "lesson-modality", `${language}.d`, name);
}

afterEach(() => {
  for (const root of roots.splice(0)) {
    rmSync(root, { recursive: true, force: true });
  }
});

describe("modality direct owners", () => {
  it("folds direct lesson owners into the exact public aggregate", () => {
    const root = temporaryRoot();
    const value = manifest();
    writeOwners(root, value);

    expect([...modalityOwnerContents(value).keys()]).toEqual([
      "spanish.d/_meta.json",
      "spanish.d/ES-C01-hola.json",
      "spanish.d/ES-C01-adios.json",
      "tamil.d/_meta.json",
      "tamil.d/TA-C01-vanakkam.json",
    ]);
    expect(readOwners(root, value)).toEqual(value);
    expect(() =>
      assertModalityManifestLanguages(value, ["spanish", "tamil"]),
    ).not.toThrow();
    expect(() =>
      assertModalityManifestLanguages(value, ["spanish", "tamil", "urdu"]),
    ).toThrow(/missing.*urdu|urdu.*missing/i);
  });

  it("checks the exact language set before opening owner bytes", () => {
    const root = temporaryRoot();
    writeOwners(root);
    writeFileSync(owner(root, "spanish", "_meta.json"), "not json\n", "utf8");
    rmSync(join(root, "core", "lesson-modality", "tamil.d"), {
      recursive: true,
      force: true,
    });

    expect(() => readOwners(root)).toThrow(/missing.*tamil|tamil.*missing/i);
  });

  it("rejects an extra language owner", () => {
    const root = temporaryRoot();
    writeOwners(root);
    mkdirSync(join(root, "core", "lesson-modality", "ghost.d"));

    expect(() => readOwners(root)).toThrow(/extra.*ghost|ghost.*extra|unexpected.*ghost/i);
  });

  it("detects clean deletion and extra lesson owners from source identities", () => {
    const root = temporaryRoot();
    writeOwners(root);
    rmSync(owner(root, "spanish", "ES-C01-adios.json"));

    expect(() => readOwners(root)).toThrow(
      /missing.*ES-C01-adios|ES-C01-adios.*missing/i,
    );

    writeFileSync(owner(root, "spanish", "ES-C01-ghost.json"), "{}\n", "utf8");
    const sourceIds = new Map(expectedIds());
    sourceIds.set("spanish", ["ES-C01-hola"]);
    expect(() =>
      readModalityManifestOwners(root, {
        expectedLanguages: ["spanish", "tamil"],
        expectedLessonIds: sourceIds,
      }),
    ).toThrow(/extra.*ES-C01-ghost|ES-C01-ghost.*extra/i);
  });

  it("independently detects a lesson missing from narration identities", () => {
    const root = temporaryRoot();
    writeOwners(root);
    const sourceIds = expectedIds();
    const narrationIds = new Map(sourceIds);
    narrationIds.set("spanish", ["ES-C01-hola"]);

    expect(() =>
      readModalityManifestOwners(root, {
        expectedLanguages: ["spanish", "tamil"],
        expectedLessonIds: sourceIds,
        expectedNarrationLessonIds: narrationIds,
      }),
    ).toThrow(/narration.*ES-C01-adios|ES-C01-adios.*narration/i);
  });

  it("binds language directories, metadata, filenames, and lesson records", () => {
    const root = temporaryRoot();
    writeOwners(root);

    const lessonPath = owner(root, "spanish", "ES-C01-hola.json");
    const lessonOwner = JSON.parse(readFileSync(lessonPath, "utf8"));
    lessonOwner.lesson.id = "ES-C01-adios";
    writeFileSync(lessonPath, `${JSON.stringify(lessonOwner, null, 2)}\n`, "utf8");
    expect(() => readOwners(root)).toThrow(/ES-C01-hola.*ES-C01-adios|filename.*lesson/i);

    writeOwners(root);
    const wrongLanguage = JSON.parse(readFileSync(lessonPath, "utf8"));
    wrongLanguage.lesson.language = "tamil";
    writeFileSync(lessonPath, `${JSON.stringify(wrongLanguage, null, 2)}\n`, "utf8");
    expect(() => readOwners(root)).toThrow(/spanish.*tamil|language/i);

    writeOwners(root);
    const metaPath = owner(root, "spanish", "_meta.json");
    const meta = JSON.parse(readFileSync(metaPath, "utf8"));
    meta.language = "tamil";
    writeFileSync(metaPath, `${JSON.stringify(meta, null, 2)}\n`, "utf8");
    expect(() => readOwners(root)).toThrow(/spanish.*tamil|metadata.*language/i);
  });

  it("rejects noncanonical owner bytes", () => {
    const root = temporaryRoot();
    writeOwners(root);
    const path = owner(root, "spanish", "ES-C01-hola.json");
    writeFileSync(path, JSON.stringify(JSON.parse(readFileSync(path, "utf8"))), "utf8");

    expect(() => readOwners(root)).toThrow(/canonical/i);
  });

  it("rejects nesting, symlinks, non-regular owners, and reserved names", () => {
    const nested = temporaryRoot();
    writeOwners(nested);
    mkdirSync(owner(nested, "spanish", "nested"));
    expect(() => readOwners(nested)).toThrow(/direct|nested|unexpected|regular/i);

    const linked = temporaryRoot();
    writeOwners(linked);
    const linkedOwner = owner(linked, "spanish", "ES-C01-hola.json");
    rmSync(linkedOwner);
    symlinkSync(owner(linked, "spanish", "ES-C01-adios.json"), linkedOwner);
    expect(() => readOwners(linked)).toThrow(/symbolic link|regular|direct/i);

    const nonregular = temporaryRoot();
    writeOwners(nonregular);
    const nonregularOwner = owner(nonregular, "spanish", "ES-C01-hola.json");
    rmSync(nonregularOwner);
    mkdirSync(nonregularOwner);
    expect(() => readOwners(nonregular)).toThrow(/regular|direct/i);

    const reserved = temporaryRoot();
    writeOwners(reserved);
    writeFileSync(owner(reserved, "spanish", "con.json"), "{}\n", "utf8");
    expect(() => readOwners(reserved)).toThrow(/reserved|unexpected|unsafe|extra/i);
  });

  it("rejects case-fold collisions and dangerous identities", () => {
    const colliding = manifest();
    colliding.lessons[0]!.id = "ES-C01-Hola";
    colliding.lessons[1]!.id = "es-c01-hola";
    expect(() => modalityOwnerContents(colliding)).toThrow(/case|collision/i);

    const dangerous = manifest();
    dangerous.lessons[0]!.id = "__proto__";
    expect(() => modalityOwnerContents(dangerous)).toThrow(
      /__proto__|dangerous|unsafe|reserved/i,
    );
  });

  it("refuses to drop lessons or findings that do not belong to a track owner", () => {
    const orphanLesson = manifest();
    orphanLesson.lessons[0]!.language = "urdu";
    expect(() => modalityOwnerContents(orphanLesson)).toThrow(/lesson languages|urdu/i);

    const wrongFindingOwner = manifest();
    wrongFindingOwner.findings.push({
      code: "modality-unknown-value",
      lessonId: "ES-C01-hola",
      language: "tamil",
      message: "belongs to the wrong track",
    });
    expect(() => modalityOwnerContents(wrongFindingOwner)).toThrow(/does not belong/i);
  });

  it("rejects a case-fold duplicate owned by a different language", () => {
    const root = temporaryRoot();
    writeOwners(root);
    const original = owner(root, "tamil", "TA-C01-vanakkam.json");
    const parsed = JSON.parse(readFileSync(original, "utf8"));
    parsed.lesson.id = "es-c01-HOLA";
    for (const finding of parsed.findings) finding.lessonId = "es-c01-HOLA";
    rmSync(original);
    writeFileSync(
      owner(root, "tamil", "es-c01-HOLA.json"),
      `${JSON.stringify(parsed, null, 2)}\n`,
      "utf8",
    );
    const ids = new Map<string, readonly string[]>([
      ["spanish", ["ES-C01-adios", "ES-C01-hola"]],
      ["tamil", ["es-c01-HOLA"]],
    ]);

    expect(() =>
      readModalityManifestOwners(root, {
        expectedLanguages: ["spanish", "tamil"],
        expectedLessonIds: ids,
        expectedNarrationLessonIds: ids,
      }),
    ).toThrow(/case|collision|duplicate/i);
  });

  it("rejects dangerous keys in owner JSON", () => {
    const root = temporaryRoot();
    writeOwners(root);
    const path = owner(root, "spanish", "ES-C01-hola.json");
    const text = readFileSync(path, "utf8");
    writeFileSync(path, text.replace("{\n", '{\n  "__proto__": {},\n'), "utf8");

    expect(() => readOwners(root)).toThrow(/__proto__|dangerous/i);
  });

  it("rejects duplicate finding identities in otherwise canonical owner JSON", () => {
    const root = temporaryRoot();
    writeOwners(root);
    const path = owner(root, "spanish", "ES-C01-hola.json");
    const parsed = JSON.parse(readFileSync(path, "utf8"));
    const finding = {
      code: "modality-unknown-value",
      lessonId: "ES-C01-hola",
      language: "spanish",
      message: "duplicate gate identity",
    };
    parsed.findings = [finding, finding];
    writeFileSync(path, `${JSON.stringify(parsed, null, 2)}\n`, "utf8");

    expect(() => readOwners(root)).toThrow(/repeats identity|duplicate/i);
  });

  it("rejects resurrected per-language aggregates", () => {
    const root = temporaryRoot();
    const value = manifest();
    writeOwners(root, value);
    writeFileSync(
      join(root, "core", "lesson-modality", "spanish.json"),
      `${JSON.stringify(value, null, 2)}\n`,
      "utf8",
    );

    expect(() => readOwners(root, value)).toThrow(/aggregate|monolith|resurrected/i);
  });
});

describe("modality owner conflict surface", () => {
  it("changes exactly one owner when exactly one lesson changes", () => {
    const before = buildModalityManifest([
      lesson({ id: "ES-C01-hola", language: "spanish" }),
      lesson({ id: "TA-C01-vanakkam", language: "tamil" }),
    ]);
    const after = buildModalityManifest([
      lesson({
        id: "ES-C01-hola",
        language: "spanish",
        body: "Say hello twice out loud.",
      }),
      lesson({ id: "TA-C01-vanakkam", language: "tamil" }),
    ]);
    const beforeOwners = modalityOwnerContents(before);
    const afterOwners = modalityOwnerContents(after);

    expect([...beforeOwners.keys()]).toEqual([...afterOwners.keys()]);
    expect(
      [...beforeOwners.keys()].filter(
        (path) => beforeOwners.get(path) !== afterOwners.get(path),
      ),
    ).toEqual(["spanish.d/ES-C01-hola.json"]);
  });
});
