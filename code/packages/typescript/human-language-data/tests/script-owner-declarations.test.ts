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
  // 49 -> 51: HL-C360 adds ろ (U+308D) and ゅ (U+3085), the two hiragana the
  // Japanese cardinals one to ten needed — ろ for roku and ゅ for juu.
  // 59 -> 62: chapter 131 writes small ゃ (U+3083), small ょ (U+3087) and を
  // (U+3092), each after a word that already holds it (おちゃ, ちょっと,
  // おちゃを ください).
  // 62 -> 65: chapter 132 writes そ (U+305D), れ (U+308C) and る (U+308B),
  // each after a word that already holds it (そこ, これ, くるま).
  // 65 -> 69: chapter 133 writes the last four basic hiragana, き (U+304D),
  // け (U+3051), ぬ (U+306C) and へ (U+3078), each after a word that already
  // holds it (えき, いけ, いぬ, へや). 69 -> 70: ら (U+3089), written since
  // chapter 8 (JA-W08-ra) but never given an inventory row, gets its record,
  // its KanjiVG stroke-order source and its ductus in the same chapter.
  // 70 -> 78: chapters 134 and 135 write eight voiced kana, で (U+3067),
  // ば (U+3070), べ (U+3079), ぶ (U+3076), び (U+3073), ぐ (U+3050),
  // げ (U+3052) and ぎ (U+304E). Each row follows が, ご, ざ and ぼ: the base
  // sign plus the dakuten, with no stroke-order source of its own and so no
  // ductus.
  // 78 -> 85: chapters 136 and 137 write ぞ (U+305E), ず (U+305A), ぜ
  // (U+305C), ぱ (U+3071), ぴ (U+3074), ぷ (U+3077) and ぺ (U+307A). The
  // three z signs follow ざ (base plus the dakuten); the four p signs follow
  // ぽ (base plus the handakuten). None has a stroke-order source of its own,
  // and so none has a ductus.
  // 85 -> 87: だ (U+3060) and ど (U+3069), written since chapters 9 and 11
  // (JA-W09-do, JA-W11-da) but covered until now only through decomposition,
  // get rows of their own, because a cited stroke order and a ductus need a
  // row to belong to. Every voiced kana row and the three spacing marks now
  // cite KanjiVG.
  // 87 -> 90: 言 (U+8A00), 五 (U+4E94) and 口 (U+53E3), written on their own
  // since chapter 5 (JA-W05-gen-component, -five-component, -mouth-component),
  // get rows once each is read in a word headword (言う in JA-C28-iu, 五 in
  // JA-C14-go, 口 in JA-C11-kuchi), so each can carry its KanjiVG stroke order
  // and a ductus.
  { language: "japanese", script: "japanese", letters: 90, marks: 3, digits: 0 },
  // 24 -> 26: HL-C350 adds ج and ص as RECOGNITION-ONLY owners. panj (five) and
  // sad (a hundred) need them in a headword, and `uncoveredGlyphs` is a
  // headword check, so the numerals could not be taught without them. Both
  // carry an empty strokeOrder on purpose: this inventory records
  // Persian-scoped provenance and the only timestamped demonstrations on hand
  // are the separately sourced Arabic ones, which its own entries say must not
  // be borrowed. They enter closure when a Persian-scoped citation exists.
  // digits 0 -> 10: the Persian digits ۰-۹ get rows of their own (an optional
  // `digits/` section, HL25), each carrying its POH-Db stroke-order citation.
  { language: "persian", script: "perso-arabic", letters: 26, marks: 1, digits: 10 },
  // 29 -> 30: ஸ (U+0BB8), taught alone in TA-S129 and read inside நமஸ்காரம்,
  // gets its row once its order is cited to LipiTk's Tamil recognizer.
  { language: "tamil", script: "tamil", letters: 30, marks: 9, digits: 0 },
  // digits 0 -> 10: Urdu's ten digit rows; only ۰ ۱ ۲ ۳ carry a citation.
  { language: "urdu", script: "urdu-nastaliq", letters: 31, marks: 2, digits: 10 },
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
    "exactly matches $script's current $letters letter, $marks mark and $digits digit owners",
    ({ language, script, letters, marks, digits }) => {
      const declarations = readScriptOwnerDeclarations(corpus, {
        language,
        script,
      });
      const inventory = loadScripts(corpus)[script]!;
      expect(declarations.letters).toHaveLength(letters);
      expect(declarations.marks).toHaveLength(marks);
      expect(declarations.digits).toHaveLength(digits);
      expect(new Set(declarations.digits)).toEqual(
        new Set((inventory.digits ?? []).map((entry) => scriptEntryId(entry.glyph))),
      );
      // An inventory with no digit rows has no `digits` key at all.
      expect(Object.hasOwn(inventory, "digits")).toBe(digits > 0);
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

  it("treats digits/ as optional, and holds its owners to the inventory like the others", () => {
    withFixture((root) => {
      expect(
        readScriptOwnerDeclarations(root, { language: "japanese", script: "japanese" }).digits,
      ).toEqual([]);
      const digitDirectory = join(root, "data", "script-owner-declarations", "japanese", "digits");
      mkdirSync(digitDirectory);
      writeJson(join(digitDirectory, "U-6F0.json"), {
        language: "japanese",
        script: "japanese",
        kind: "digit",
        glyph: "۰",
      });
      expect(
        readScriptOwnerDeclarations(root, { language: "japanese", script: "japanese" }).digits,
      ).toEqual(["U-6F0"]);
      expect(() =>
        runShardCli(["--check", "data/scripts/japanese.json"], root),
      ).toThrow(/digits identity set differs:.*missing \[U-6F0\]/);
      const inventoryDigits = join(root, "data", "scripts", "japanese.d", "digits");
      mkdirSync(inventoryDigits);
      writeJson(join(inventoryDigits, "0010-U-6F0.json"), {
        glyph: "۰",
        sound: "0",
        role: "digit",
        components: ["۰  0 — fixture digit"],
        strokeOrder: [],
        strokeOrderNote: "",
      });
      expect(runShardCli(["--check", "data/scripts/japanese.json"], root)).toBe(0);
      // A digit declared with the wrong kind is refused.
      writeJson(join(digitDirectory, "U-6F0.json"), {
        language: "japanese",
        script: "japanese",
        kind: "letter",
        glyph: "۰",
      });
      expect(() =>
        readScriptOwnerDeclarations(root, { language: "japanese", script: "japanese" }),
      ).toThrow(/kind must be 'digit'/);
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
      ).toThrow(/must contain exactly: letters, marks \(and optionally digits\)/);
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
