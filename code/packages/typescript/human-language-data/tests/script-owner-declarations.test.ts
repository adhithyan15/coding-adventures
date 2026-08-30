import {
  cpSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  readdirSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { defaultCurriculumRoot, loadScripts } from "../src/loader.js";
import {
  readScriptOwnerDeclarations,
  scriptOwnerDeclarationRelativePath,
} from "../src/script-owner-declarations.js";
import { runShardCli } from "../src/shard-cli.js";
import { scriptEntryId } from "../src/script-shards.js";

const corpus = defaultCurriculumRoot();

const CONFIGS = [
  { language: "japanese", script: "japanese", letters: 49, marks: 3 },
  { language: "persian", script: "perso-arabic", letters: 24, marks: 1 },
  { language: "tamil", script: "tamil", letters: 25, marks: 9 },
  { language: "urdu", script: "urdu-nastaliq", letters: 30, marks: 2 },
] as const;

function fixture(script = "japanese"): string {
  const root = mkdtempSync(join(tmpdir(), "hl-script-owner-declarations-"));
  const inventory = join("data", "scripts", `${script}.d`);
  const declarations = join("data", "script-owner-declarations", script);
  mkdirSync(join(root, "data", "scripts"), { recursive: true });
  mkdirSync(
    join(root, "data", "script-owner-declarations"),
    { recursive: true },
  );
  cpSync(join(corpus, inventory), join(root, inventory), { recursive: true });
  cpSync(join(corpus, declarations), join(root, declarations), {
    recursive: true,
  });
  return root;
}

function withFixture(run: (root: string) => void): void {
  const root = fixture();
  try {
    run(root);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
}

function declaration(
  root: string,
  section: "letters" | "marks",
  identity: string,
): string {
  return join(
    root,
    "data",
    "script-owner-declarations",
    "japanese",
    section,
    `${identity}.json`,
  );
}

function inventoryOwner(
  root: string,
  section: "letters" | "marks",
  identity: string,
): string {
  const directory = join(root, "data", "scripts", "japanese.d", section);
  const name = readdirSync(directory).find((candidate) =>
    candidate.endsWith(`-${identity}.json`),
  );
  if (name === undefined) throw new Error(`fixture has no ${section} ${identity}`);
  return join(directory, name);
}

function writeJson(path: string, value: unknown): void {
  writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`, "utf8");
}

describe("independent script owner declarations", () => {
  it.each(CONFIGS)(
    "exactly matches $script's current $letters letter and $marks mark owners",
    ({ language, script, letters, marks }) => {
      const declarations = readScriptOwnerDeclarations(corpus, {
        language,
        script,
      });
      const inventory = loadScripts(corpus)[script]!;
      expect(declarations.letters).toHaveLength(letters);
      expect(declarations.marks).toHaveLength(marks);
      expect(new Set(declarations.letters)).toEqual(
        new Set(inventory.letters.map((entry) => scriptEntryId(entry.glyph))),
      );
      expect(new Set(declarations.marks)).toEqual(
        new Set((inventory.marks ?? []).map((entry) => scriptEntryId(entry.mark))),
      );
      expect(runShardCli(["--check", `data/scripts/${script}.json`], corpus)).toBe(0);
    },
  );

  it("detects a clean inventory-owner deletion from declarations", () => {
    withFixture((root) => {
      rmSync(inventoryOwner(root, "letters", "U-3042"));
      expect(() =>
        runShardCli(["--check", "data/scripts/japanese.json"], root),
      ).toThrow(/letters identity set differs: missing \[U-3042\]/);
    });
  });

  it("detects a clean declaration deletion from the surviving inventory", () => {
    withFixture((root) => {
      rmSync(declaration(root, "letters", "U-3042"));
      expect(() =>
        runShardCli(["--check", "data/scripts/japanese.json"], root),
      ).toThrow(/letters identity set differs:.*unexpected \[U-3042\]/);
    });
  });

  it("rejects an independently declared owner absent from the inventory", () => {
    withFixture((root) => {
      writeJson(declaration(root, "letters", "U-20000"), {
        language: "japanese",
        script: "japanese",
        kind: "letter",
        glyph: "𠀀",
      });
      expect(() =>
        runShardCli(["--check", "data/scripts/japanese.json"], root),
      ).toThrow(/letters identity set differs:.*missing \[U-20000\]/);
    });
  });

  it("rejects one identity declared as both a letter and a mark", () => {
    withFixture((root) => {
      cpSync(
        declaration(root, "letters", "U-3042"),
        declaration(root, "marks", "U-3042"),
      );
      expect(() =>
        readScriptOwnerDeclarations(root, {
          language: "japanese",
          script: "japanese",
        }),
      ).toThrow(/repeats 'U-3042'/);
    });
  });

  it.skipIf(process.platform !== "linux")(
    "rejects declaration filenames that collide under case folding",
    () => {
      withFixture((root) => {
        cpSync(
          declaration(root, "letters", "U-3042"),
          declaration(root, "letters", "u-3042"),
        );
        expect(() =>
          readScriptOwnerDeclarations(root, {
            language: "japanese",
            script: "japanese",
          }),
        ).toThrow(/case-fold collision/);
      });
    },
  );

  it("binds filename, body identity, kind, language, and script", () => {
    withFixture((root) => {
      writeJson(declaration(root, "letters", "U-3042"), {
        language: "persian",
        script: "perso-arabic",
        kind: "mark",
        glyph: "い",
      });
      expect(() =>
        readScriptOwnerDeclarations(root, {
          language: "japanese",
          script: "japanese",
        }),
      ).toThrow(/\.language must be 'japanese'/);

      writeJson(declaration(root, "letters", "U-3042"), {
        language: "japanese",
        script: "japanese",
        kind: "letter",
        glyph: "い",
      });
      expect(() =>
        readScriptOwnerDeclarations(root, {
          language: "japanese",
          script: "japanese",
        }),
      ).toThrow(/claims 'U-3042'.*glyph is 'U-3044'/);
    });
  });

  it("rejects malformed JSON, dangerous keys, and non-canonical bytes", () => {
    withFixture((root) => {
      const path = declaration(root, "letters", "U-3042");
      writeFileSync(path, "{", "utf8");
      expect(() =>
        readScriptOwnerDeclarations(root, {
          language: "japanese",
          script: "japanese",
        }),
      ).toThrow(/malformed JSON/);

      writeFileSync(
        path,
        '{"language":"japanese","script":"japanese","kind":"letter","glyph":"あ","__proto__":{}}\n',
        "utf8",
      );
      expect(() =>
        readScriptOwnerDeclarations(root, {
          language: "japanese",
          script: "japanese",
        }),
      ).toThrow(/must not carry '__proto__'/);

      writeFileSync(
        path,
        '{"language":"japanese","script":"japanese","kind":"letter","glyph":"あ"}\n',
        "utf8",
      );
      expect(() =>
        readScriptOwnerDeclarations(root, {
          language: "japanese",
          script: "japanese",
        }),
      ).toThrow(/is not canonical/);
    });
  });

  it("rejects unsafe roots, malformed names, nesting, and unexpected sections", () => {
    expect(() =>
      scriptOwnerDeclarationRelativePath("../japanese", "letter", "あ"),
    ).toThrow(/unsafe or reserved/);
    expect(() =>
      scriptOwnerDeclarationRelativePath("con", "letter", "あ"),
    ).toThrow(/unsafe or reserved/);

    withFixture((root) => {
      mkdirSync(
        join(
          root,
          "data",
          "script-owner-declarations",
          "japanese",
          "letters",
          "nested",
        ),
      );
      expect(() =>
        readScriptOwnerDeclarations(root, {
          language: "japanese",
          script: "japanese",
        }),
      ).toThrow(/real direct-child regular file/);
    });

    withFixture((root) => {
      writeFileSync(
        join(
          root,
          "data",
          "script-owner-declarations",
          "japanese",
          "README.md",
        ),
        "unexpected",
      );
      expect(() =>
        readScriptOwnerDeclarations(root, {
          language: "japanese",
          script: "japanese",
        }),
      ).toThrow(/must contain exactly: letters, marks/);
    });
  });

  it.skipIf(process.platform === "win32")(
    "rejects a declaration file symlink without opening its target",
    () => {
      withFixture((root) => {
        const target = join(root, "outside.json");
        writeJson(target, {
          language: "japanese",
          script: "japanese",
          kind: "letter",
          glyph: "𠀀",
        });
        symlinkSync(target, declaration(root, "letters", "U-20000"));
        expect(() =>
          readScriptOwnerDeclarations(root, {
            language: "japanese",
            script: "japanese",
          }),
        ).toThrow(/real direct-child regular file/);
      });
    },
  );

  it("keeps two additions to one script on disjoint owner paths", () => {
    const first = scriptOwnerDeclarationRelativePath("tamil", "letter", "ஶ");
    const second = scriptOwnerDeclarationRelativePath("tamil", "letter", "ஜ");
    const inventoryFirst = `data/scripts/tamil.d/letters/0260-${scriptEntryId("ஶ")}.json`;
    const inventorySecond = `data/scripts/tamil.d/letters/0270-${scriptEntryId("ஜ")}.json`;
    expect(new Set([first, second, inventoryFirst, inventorySecond]).size).toBe(4);
  });
});
