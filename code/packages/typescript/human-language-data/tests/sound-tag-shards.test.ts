import {
  existsSync,
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
  SOUND_TAG_META_OWNER,
  readSoundTagRegistryOwners,
  serializeSoundTagRegistry,
  soundTagOwnerContents,
} from "../src/sound-tag-shards.js";
import {
  SOUND_TAG_PLAN,
  shardLedger,
  unshardContents,
  unshardLedger,
} from "../src/shard-cli.js";

const registry = {
  version: 1 as const,
  tracks: {
    spanish: ["silent-h", "vowel-a"],
    tamil: ["retroflex-l", "short-a"],
  },
};

const roots: string[] = [];

function temporaryRoot(): string {
  const root = mkdtempSync(join(tmpdir(), "hl-sound-tags-"));
  roots.push(root);
  mkdirSync(join(root, "core", "sound-tags.d"), { recursive: true });
  return root;
}

function writeOwners(root: string): void {
  for (const [name, body] of soundTagOwnerContents(registry)) {
    writeFileSync(join(root, "core", "sound-tags.d", name), body, "utf8");
  }
}

afterEach(() => {
  for (const root of roots.splice(0)) {
    rmSync(root, { recursive: true, force: true });
  }
});

describe("sound-tag direct owners", () => {
  it("folds one self-binding owner per language", () => {
    const root = temporaryRoot();
    writeOwners(root);

    expect([...soundTagOwnerContents(registry).keys()]).toEqual([
      SOUND_TAG_META_OWNER,
      "spanish.json",
      "tamil.json",
    ]);
    expect(
      readSoundTagRegistryOwners(root, {
        expectedLanguages: ["spanish", "tamil"],
      }),
    ).toEqual(registry);
  });

  it("checks missing and extra filenames before opening owner bytes", () => {
    const root = temporaryRoot();
    writeOwners(root);
    rmSync(join(root, "core", "sound-tags.d", "tamil.json"));
    writeFileSync(
      join(root, "core", "sound-tags.d", "spanish.json"),
      "not json\n",
      "utf8",
    );

    expect(() =>
      readSoundTagRegistryOwners(root, {
        expectedLanguages: ["spanish", "tamil"],
      }),
    ).toThrow(/missing: tamil/);

    writeFileSync(
      join(root, "core", "sound-tags.d", "ghost.json"),
      '{}\n',
      "utf8",
    );
    expect(() =>
      readSoundTagRegistryOwners(root, {
        expectedLanguages: ["spanish"],
      }),
    ).toThrow(/extra: ghost/);
  });

  it("rejects mismatches, noncanonical bytes, nesting, and reserved names", () => {
    const mismatch = temporaryRoot();
    writeOwners(mismatch);
    writeFileSync(
      join(mismatch, "core", "sound-tags.d", "spanish.json"),
      `${JSON.stringify({ language: "tamil", tags: ["short-a"] }, null, 2)}\n`,
      "utf8",
    );
    expect(() =>
      readSoundTagRegistryOwners(mismatch, {
        expectedLanguages: ["spanish", "tamil"],
      }),
    ).toThrow(/carries language 'tamil'/);

    const noncanonical = temporaryRoot();
    writeOwners(noncanonical);
    writeFileSync(
      join(noncanonical, "core", "sound-tags.d", "spanish.json"),
      JSON.stringify({ language: "spanish", tags: ["silent-h", "vowel-a"] }),
      "utf8",
    );
    expect(() =>
      readSoundTagRegistryOwners(noncanonical, {
        expectedLanguages: ["spanish", "tamil"],
      }),
    ).toThrow(/not canonical/);

    const nested = temporaryRoot();
    writeOwners(nested);
    mkdirSync(join(nested, "core", "sound-tags.d", "nested"));
    expect(() =>
      readSoundTagRegistryOwners(nested, {
        expectedLanguages: ["spanish", "tamil"],
      }),
    ).toThrow(/real direct-child/);

    const reserved = temporaryRoot();
    writeOwners(reserved);
    writeFileSync(join(reserved, "core", "sound-tags.d", "con.json"), "{}\n");
    expect(() =>
      readSoundTagRegistryOwners(reserved, {
        expectedLanguages: ["spanish", "tamil"],
      }),
    ).toThrow(/unexpected/);
  });

  it("rejects symlinked owners and a resurrected aggregate", () => {
    const linked = temporaryRoot();
    writeOwners(linked);
    rmSync(join(linked, "core", "sound-tags.d", "spanish.json"));
    symlinkSync(
      join(linked, "core", "sound-tags.d", "tamil.json"),
      join(linked, "core", "sound-tags.d", "spanish.json"),
    );
    expect(() =>
      readSoundTagRegistryOwners(linked, {
        expectedLanguages: ["spanish", "tamil"],
      }),
    ).toThrow(/real direct-child/);

    const resurrected = temporaryRoot();
    writeOwners(resurrected);
    writeFileSync(
      join(resurrected, "core", "sound-tags.json"),
      serializeSoundTagRegistry(registry),
      "utf8",
    );
    expect(() =>
      readSoundTagRegistryOwners(resurrected, {
        expectedLanguages: ["spanish", "tamil"],
      }),
    ).toThrow(/present beside canonical/);
  });
});

describe("sound-tag migration", () => {
  it("installs validated owners before removing the aggregate", () => {
    const root = mkdtempSync(join(tmpdir(), "hl-sound-tag-migration-"));
    roots.push(root);
    mkdirSync(join(root, "core"), { recursive: true });
    const before = serializeSoundTagRegistry(registry);
    writeFileSync(join(root, "core", "sound-tags.json"), before, "utf8");
    writeFileSync(
      join(root, "core", "languages.json"),
      `${JSON.stringify({
        version: 1,
        languages: [{ id: "spanish" }, { id: "tamil" }],
      }, null, 2)}\n`,
      "utf8",
    );

    expect(shardLedger(root, SOUND_TAG_PLAN)).toHaveLength(3);
    expect(existsSync(join(root, "core", "sound-tags.json"))).toBe(false);
    expect(unshardContents(root, SOUND_TAG_PLAN)).toBe(before);
    expect(() => unshardLedger(root, SOUND_TAG_PLAN)).toThrow(
      /monolith was removed/,
    );
  });

  it("preserves the aggregate when the destination is unsafe", () => {
    const root = mkdtempSync(join(tmpdir(), "hl-sound-tag-migration-"));
    roots.push(root);
    mkdirSync(join(root, "core"), { recursive: true });
    const aggregate = join(root, "core", "sound-tags.json");
    writeFileSync(aggregate, serializeSoundTagRegistry(registry), "utf8");
    symlinkSync(root, join(root, "core", "sound-tags.d"));

    expect(() => shardLedger(root, SOUND_TAG_PLAN)).toThrow(
      /symbolic link|real directory/,
    );
    expect(readFileSync(aggregate, "utf8")).toBe(
      serializeSoundTagRegistry(registry),
    );
  });

  it("does not let a resurrected aggregate overwrite canonical owners", () => {
    const root = temporaryRoot();
    writeOwners(root);
    const aggregate = join(root, "core", "sound-tags.json");
    writeFileSync(
      aggregate,
      serializeSoundTagRegistry({
        version: 1,
        tracks: { spanish: ["changed"], tamil: ["short-a"] },
      }),
      "utf8",
    );
    const before = readFileSync(
      join(root, "core", "sound-tags.d", "spanish.json"),
      "utf8",
    );

    expect(() => shardLedger(root, SOUND_TAG_PLAN)).toThrow(
      /resurrected aggregate cannot overwrite/,
    );
    expect(
      readFileSync(join(root, "core", "sound-tags.d", "spanish.json"), "utf8"),
    ).toBe(before);
  });

  it("preserves an aggregate that omits a registered language", () => {
    const root = mkdtempSync(join(tmpdir(), "hl-sound-tag-migration-"));
    roots.push(root);
    mkdirSync(join(root, "core"), { recursive: true });
    const aggregate = join(root, "core", "sound-tags.json");
    const incomplete = serializeSoundTagRegistry({
      version: 1,
      tracks: { spanish: ["silent-h"] },
    });
    writeFileSync(aggregate, incomplete, "utf8");
    writeFileSync(
      join(root, "core", "languages.json"),
      `${JSON.stringify({
        version: 1,
        languages: [{ id: "spanish" }, { id: "tamil" }],
      }, null, 2)}\n`,
      "utf8",
    );

    expect(() => shardLedger(root, SOUND_TAG_PLAN)).toThrow(/missing: tamil/);
    expect(readFileSync(aggregate, "utf8")).toBe(incomplete);
    expect(existsSync(join(root, "core", "sound-tags.d"))).toBe(false);
  });

  it("preserves a noncanonical source aggregate byte for byte", () => {
    const root = mkdtempSync(join(tmpdir(), "hl-sound-tag-migration-"));
    roots.push(root);
    mkdirSync(join(root, "core"), { recursive: true });
    const aggregate = join(root, "core", "sound-tags.json");
    const noncanonical = JSON.stringify(registry);
    writeFileSync(aggregate, noncanonical, "utf8");
    writeFileSync(
      join(root, "core", "languages.json"),
      `${JSON.stringify({
        version: 1,
        languages: [{ id: "spanish" }, { id: "tamil" }],
      }, null, 2)}\n`,
      "utf8",
    );

    expect(() => shardLedger(root, SOUND_TAG_PLAN)).toThrow(/not canonical/);
    expect(readFileSync(aggregate, "utf8")).toBe(noncanonical);
    expect(existsSync(join(root, "core", "sound-tags.d"))).toBe(false);
  });
});
