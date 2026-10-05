import { createHash } from "node:crypto";
import { lstatSync, readFileSync, readdirSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import ts from "typescript";
import { describe, expect, it } from "vitest";

import { DUCTUS } from "../src/strokes";

const sha256 = (value: string): string =>
  createHash("sha256").update(value).digest("hex");

const packageRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const ownerNames = [
  "arabic-family",
  "chinese",
  "cyrillic",
  "devanagari",
  "gujarati",
  "hebrew",
  "japanese",
  "kannada",
  "malayalam",
  "tamil",
  "telugu",
];

const glyphOwnerName = (glyph: string): string =>
  `U-${[...glyph]
    .map((character) => character.codePointAt(0)!.toString(16).toUpperCase())
    .join("-")}`;

const regularOwnerNames = (directory: string): string[] =>
  readdirSync(directory)
    .map((name) => {
      const stat = lstatSync(resolve(directory, name));
      expect(stat.isSymbolicLink(), `${name} must not be a symbolic link`).toBe(
        false,
      );
      expect(stat.isFile(), `${name} must be a regular file`).toBe(true);
      return name;
    })
    .sort();

const evidenceOwnerClaims = (
  filename: string,
  source: string,
): { imports: string[]; lookups: string[] } => {
  const imports: string[] = [];
  const lookups: string[] = [];
  const sourceFile = ts.createSourceFile(
    filename,
    source,
    ts.ScriptTarget.Latest,
    true,
    ts.ScriptKind.TS,
  );
  const visit = (node: ts.Node): void => {
    if (
      ts.isImportDeclaration(node) &&
      ts.isStringLiteral(node.moduleSpecifier)
    ) {
      const match = node.moduleSpecifier.text.match(
        /^\.\.\/\.\.\/\.\.\/src\/strokes\/tamil\/(U-[0-9A-F]+)\.ts$/,
      );
      if (match !== null) imports.push(match[1]);
    }
    if (
      ts.isElementAccessExpression(node) &&
      ts.isIdentifier(node.expression) &&
      node.expression.text === "DUCTUS" &&
      node.argumentExpression !== undefined &&
      ts.isStringLiteral(node.argumentExpression)
    ) {
      lookups.push(node.argumentExpression.text);
    }
    ts.forEachChild(node, visit);
  };
  visit(sourceFile);
  return { imports, lookups };
};

const sourceOwnerGlyphs = (filename: string, source: string): string[] => {
  const glyphs: string[] = [];
  const sourceFile = ts.createSourceFile(
    filename,
    source,
    ts.ScriptTarget.Latest,
    true,
    ts.ScriptKind.TS,
  );
  const visit = (node: ts.Node): void => {
    if (
      ts.isVariableDeclaration(node) &&
      ts.isIdentifier(node.name) &&
      node.name.text === "entry" &&
      node.initializer !== undefined &&
      ts.isArrayLiteralExpression(node.initializer)
    ) {
      const first = node.initializer.elements[0];
      if (first !== undefined && ts.isStringLiteral(first)) {
        glyphs.push(first.text);
      }
    }
    ts.forEachChild(node, visit);
  };
  visit(sourceFile);
  return glyphs;
};

const sharedObjectIdentityGroups = (
  registry: Record<string, object>,
): string[][] => {
  const seen = new Map<object, { path: string; root: string }>();
  const shared = new Map<object, string[]>();

  const visit = (value: unknown, path: string, root: string): void => {
    if (value === null || typeof value !== "object") return;
    const previous = seen.get(value);
    if (previous !== undefined) {
      if (previous.root !== root) {
        const paths = shared.get(value) ?? [previous.path];
        paths.push(path);
        shared.set(value, paths);
      }
      return;
    }
    seen.set(value, { path, root });
    for (const [key, child] of Object.entries(value)) {
      visit(child, `${path}.${key}`, root);
    }
  };

  for (const [key, value] of Object.entries(registry)) {
    visit(value, JSON.stringify(key), key);
  }
  return [...shared.values()].sort(([a], [b]) => a.localeCompare(b));
};

describe("stroke ownership migration baseline", () => {
  it("preserves the exact ordered registry and parsed data", () => {
    const counts = Object.values(DUCTUS).reduce<Record<string, number>>(
      (out, letter) => {
        out[letter.script] = (out[letter.script] ?? 0) + 1;
        return out;
      },
      {},
    );
    const nonTamilRegistry = Object.fromEntries(
      Object.entries(DUCTUS).filter(([, letter]) => letter.script !== "tamil"),
    );
    const identityGroups = sharedObjectIdentityGroups(DUCTUS);
    expect({
      keys: Object.keys(DUCTUS).length,
      keyHash: sha256(JSON.stringify(Object.keys(DUCTUS))),
      nonTamilDataHash: sha256(JSON.stringify(nonTamilRegistry)),
      sharedIdentityGroups: identityGroups.length,
      sharedIdentityHash: sha256(JSON.stringify(identityGroups)),
      counts: Object.fromEntries(
        Object.entries(counts).sort(([a], [b]) => a.localeCompare(b)),
      ),
      // Measured, not reasoned: the source-verified numeral tranche adds 六,
      // 七, 八, 九, 十 and 百, so it moves keys 353 -> 359, the ordered key hash,
      // the non-Tamil data hash, and Chinese 44 -> 50. Tamil is untouched,
      // which is why its own count and the shared-identity hashes do not move.
      //
      // Measured again for HL-C360, which adds exactly two glyphs — ろ (U+308D)
      // and ゅ (U+3085), the two hiragana the Japanese cardinals one to ten
      // needed. Keys 359 -> 361, japanese 15 -> 17, the ordered key hash and the
      // non-Tamil data hash. Tamil is untouched again, so its count and both
      // shared-identity values are unchanged.
      //
      // Measured for HL-C364: the native-count and counter lessons verify six
      // independently written hiragana — の, ひ, ふ, ほ, む and や. Keys move
      // 361 -> 367 and Japanese 17 -> 23; Tamil and both shared-identity values
      // remain unchanged.
      //
      // Measured for HL-C366: the Chinese particles and joining lessons add
      // ten source-verified glyphs. Keys move 367 -> 377 and Chinese 50 -> 60;
      // Tamil and both shared-identity values remain unchanged.
      //
      // Measured for the Malayalam chillu NN repair: the newly source-verified
      // ൺ adds its font-checked ductus. Keys move 377 -> 378 and Malayalam
      // 13 -> 14; Tamil and both shared-identity values remain unchanged.
      //
      // Telugu క starts the consonant pass with its five-movement, two-run
      // source-backed path. Keys move 378 -> 379 and Telugu 9 -> 10; Tamil and
      // both shared-identity values remain unchanged.
      //
      // Telugu ఖ continues that pass with its six-movement, two-run path.
      // Keys move 379 -> 380 and Telugu 10 -> 11; Tamil and both
      // shared-identity values remain unchanged.
      //
      // Telugu గ follows with its two-movement, two-run path. Keys move
      // 380 -> 381 and Telugu 11 -> 12; Tamil and both shared-identity
      // values remain unchanged.
      //
      // Telugu ఘ continues with its six-movement, four-run path. Keys move
      // 381 -> 382 and Telugu 12 -> 13; Tamil and both shared-identity
      // values remain unchanged.
      //
      // Telugu చ starts the next consonant row with a four-movement, two-run
      // path. Keys move 382 -> 383 and Telugu 13 -> 14; Tamil and both
      // shared-identity values remain unchanged.
      //
      // Telugu ఙ fills the word-first gap before చ with a five-movement,
      // three-run path. Keys move 383 -> 384 and Telugu 14 -> 15; Tamil and
      // both shared-identity values remain unchanged.
      //
      // Telugu జ is the next consonant with an existing vocabulary-first
      // lesson owner. Its four sourced movements remain four pen-down runs.
      // Keys move 384 -> 385 and Telugu 15 -> 16; Tamil and both
      // shared-identity values remain unchanged.
      //
      // Telugu ఞ already has a vocabulary-first lesson owner. Its eight
      // sourced movements remain eight pen-down runs. Keys move 385 -> 386
      // and Telugu 16 -> 17; Tamil and both shared-identity values remain
      // unchanged.
      //
      // Telugu ట already has a vocabulary-first lesson owner. Its six sourced
      // movements remain six pen-down runs. Keys move 386 -> 387 and Telugu
      // 17 -> 18; Tamil and both shared-identity values remain unchanged.
      //
      // Telugu ఠ already has a vocabulary-first lesson owner. Its three
      // sourced movements remain three pen-down runs. Keys move 387 -> 388
      // and Telugu 18 -> 19; shared-identity values remain unchanged.
      //
      // Telugu ఛ closes the preceding word-first gap with a five-movement,
      // two-run path. Keys move 388 -> 389 and Telugu 19 -> 20; Tamil and both
      // shared-identity values remain unchanged.
      //
      // Telugu డ already has a vocabulary-first lesson owner. Its five
      // sourced movements remain five pen-down runs. Keys move 389 -> 390
      // and Telugu 20 -> 21; shared-identity values remain unchanged.
      //
      // Telugu ఝ follows its newly added vocabulary-first lesson owner. Its
      // five sourced movements remain five pen-down runs. Keys move 390 -> 391
      // and Telugu 21 -> 22; shared-identity values remain unchanged.
      //
      // Telugu ఢ already has a vocabulary-first lesson owner. Its six
      // sourced movements remain six pen-down runs. Keys move 391 -> 392 and
      // Telugu 22 -> 23; shared-identity values remain unchanged.
      //
      // Telugu ణ already has a vocabulary-first lesson owner. Its five
      // sourced movements remain five pen-down runs. Keys move 392 -> 393 and
      // Telugu 23 -> 24; shared-identity values remain unchanged.
      //
      // Telugu త already anchors a complete word-first writing ladder. Its
      // seven sourced movements form two pen-down runs. Keys move 393 -> 394
      // and Telugu 24 -> 25; shared-identity values remain unchanged.
      //
      // Telugu థ follows its vocabulary-first lesson owner. Its seven
      // sourced movements remain seven pen-down runs. Keys move 394 -> 395
      // and Telugu 25 -> 26; shared-identity values remain unchanged.
      //
      // Telugu ద follows its familiar-word lesson owner. Its five sourced
      // movements remain five pen-down runs. Keys move 395 -> 396 and Telugu
      // 26 -> 27; shared-identity values remain unchanged.
      //
      // Telugu ధ opens the word-first writing ladder. Its six sourced
      // movements remain six pen-down runs. Keys move 396 -> 397 and Telugu
      // 27 -> 28; shared-identity values remain unchanged.
      //
      // Telugu న follows with its familiar-word lesson owner. Its three
      // sourced movements remain three pen-down runs. Keys move 397 -> 398
      // and Telugu 28 -> 29; shared-identity values remain unchanged.
      //
      // Telugu ప follows with another familiar-word lesson owner. Its four
      // sourced movements remain four pen-down runs. Keys move 398 -> 399
      // and Telugu 29 -> 30; shared-identity values remain unchanged.
      //
      // Telugu ఫ follows with the aspirated partner's five sourced movements,
      // including its separate short lower stem. Keys move 399 -> 400 and
      // Telugu 30 -> 31; shared-identity values remain unchanged.
      //
      // Telugu బ follows with four separately sourced bowl movements. Keys
      // move 400 -> 401 and Telugu 31 -> 32; shared-identity values remain
      // unchanged.
      //
      // Telugu భ follows with six sourced movements, including its separate
      // upper flourish and lower stem. Keys move 401 -> 402 and Telugu 32 ->
      // 33; shared-identity values remain unchanged.
      //
      // Telugu మ follows with seven separately sourced curves. Keys move
      // 402 -> 403 and Telugu 33 -> 34; shared-identity values remain
      // unchanged.
      //
      // Telugu య follows with four sourced paths. Keys move 403 -> 404 and
      // Telugu 34 -> 35; shared-identity values remain unchanged.
      //
      // Telugu ర follows with two sourced paths. Keys move 404 -> 405 and
      // Telugu 35 -> 36; shared-identity values remain unchanged.
      //
      // Telugu ల follows with two sourced paths. Keys move 405 -> 406 and
      // Telugu 36 -> 37; shared-identity values remain unchanged.
      //
      // Telugu వ follows with three sourced paths. Keys move 406 -> 407 and
      // Telugu 37 -> 38; shared-identity values remain unchanged.
      //
      // Telugu శ follows with three sourced paths. Keys move 407 -> 408 and
      // Telugu 38 -> 39; shared-identity values remain unchanged.
      //
      // Telugu ష follows with four sourced paths. Keys move 408 -> 409 and
      // Telugu 39 -> 40; shared-identity values remain unchanged.
      //
      // Telugu స follows with two sourced paths. Keys move 409 -> 410 and
      // Telugu 40 -> 41; shared-identity values remain unchanged.
      //
      // Telugu హ follows with four sourced paths. Keys move 410 -> 411 and
      // Telugu 41 -> 42; shared-identity values remain unchanged.
      //
      // Telugu ళ follows with four sourced paths. Keys move 411 -> 412 and
      // Telugu 42 -> 43; shared-identity values remain unchanged.
      //
      // Japanese chapter 131 writes small ゃ (U+3083), small ょ (U+3087) and
      // を (U+3092). Keys move 412 -> 415 and Japanese 23 -> 26, with the
      // ordered key hash and the non-Tamil data hash; Tamil and both
      // shared-identity values remain unchanged.
      //
      // Japanese chapter 132 writes そ (U+305D), れ (U+308C) and る (U+308B).
      // Keys move 415 -> 418 and Japanese 26 -> 29, with the ordered key hash
      // and the non-Tamil data hash; Tamil and both shared-identity values
      // remain unchanged.
      //
      // Japanese chapter 133 writes the last four basic hiragana: き (U+304D),
      // け (U+3051), ぬ (U+306C) and へ (U+3078). It also gives ら (U+3089),
      // written since chapter 8 but missing from the inventory, its record
      // and ductus. Keys move 418 -> 423 and Japanese 29 -> 34, with the
      // ordered key hash and the non-Tamil data hash; Tamil and both
      // shared-identity values remain unchanged.
      //
      // あ (U+3042), い (U+3044), う (U+3046), え (U+3048), お (U+304A) and
      // か (U+304B), written since chapters 1, 3 and 10 but never given a
      // cited stroke-order source, now cite KanjiVG and gain a ductus each.
      // Keys move 423 -> 429 and Japanese 34 -> 40, with the ordered key hash
      // and the non-Tamil data hash; Tamil and both shared-identity values
      // remain unchanged.
      //
      // こ (U+3053), さ (U+3055), す (U+3059), ち (U+3061) and と (U+3068),
      // written since chapters 2 to 4 but never given a cited stroke-order
      // source, now cite KanjiVG and gain a ductus each. Keys move 429 -> 434
      // and Japanese 40 -> 45, with the ordered key hash and the non-Tamil
      // data hash; Tamil and both shared-identity values remain unchanged.
      //
      // に (U+306B), は (U+306F), ま (U+307E), り (U+308A) and ん (U+3093),
      // written since chapters 1 to 4 but never given a cited stroke-order
      // source, now cite KanjiVG and gain a ductus each, so every one of the
      // 46 basic hiragana has one. Keys move 434 -> 439 and Japanese 45 -> 50,
      // with the ordered key hash and the non-Tamil data hash, measured after
      // the last caption was settled; Tamil and both shared-identity values
      // remain unchanged.
      //
      // ఞ, థ, మ, ట, ధ, భ and ఢ keep their source-numbered movements as
      // segments but now lift only where HP Labs India's native writers do
      // (2, 2, 1, 1, 1, 1 and 2 lifts, down from 7, 6, 6, 5, 5, 5 and 5).
      // Only the non-Tamil data hash moves, measured after the last caption
      // was settled; keys, the key hash, every count, Tamil and both
      // shared-identity values remain unchanged.
      //
      // ద, డ, ణ, బ, ఫ, ఐ, ఋ and ళ likewise keep their source movements as
      // segments but lift only where HP Labs India's native writers do (0, 1,
      // 0, 0, 2, 0, 2 and 0 lifts, down from 4, 4, 4, 3, 4, 4, 5 and 3), and
      // every Telugu caption now wraps to at most two lines. Only the
      // non-Tamil data hash moves, measured after the last caption was
      // settled; keys, the key hash, every count, Tamil and both
      // shared-identity values remain unchanged.
      //
      // A third batch does the same for త, న, ప, య, ర, ల, వ, శ, ష, హ, ఠ,
      // జ, చ, అ, ఎ and ఒ (0, 0, 1, 2, 0, 0, 0, 0, 2, 1, 1, 1, 0, 0, 0 and 0
      // lifts, down from 1, 2, 3, 3, 1, 1, 2, 2, 3, 3, 2, 3, 1, 1, 1 and 2).
      // Again only the non-Tamil data hash moves; keys, the key hash, every
      // count, Tamil and both shared-identity values remain unchanged.
      //
      // The 22 voiced kana the inventory holds, が to ぽ, and the three spacing
      // marks ゛, ゜ and ー now cite KanjiVG and gain a ductus each. Keys move
      // 439 -> 464 and Japanese 50 -> 75, with the ordered key hash and the
      // non-Tamil data hash, measured after the last caption was settled;
      // Tamil and both shared-identity values remain unchanged.
      //
      // The katakana コ and ヒ and the kanji 日, 語 and 本, taught by the
      // chapter 5 and 6 writing lessons, now cite KanjiVG and gain a ductus
      // each. Keys move 464 -> 469 and Japanese 75 -> 80, with
      // the ordered key hash and the non-Tamil data hash, measured after the
      // last caption was settled; Tamil and both shared-identity values
      // remain unchanged.
      // The non-Tamil hash was re-measured on top of the Telugu native-lift
      // batches.
      //
      // Devanagari क, य, र, प, ध, ल, द, ठ, घ, ष and औ now lift only where HP
      // Labs India's native writers do: their sourced runs become segments of
      // fewer strokes, with new captions and source notes. Only the non-Tamil
      // data hash moves, measured after the last caption was settled; keys,
      // the key hash, every script count, Tamil and both shared-identity
      // values remain unchanged.
      //
      // Devanagari अ, आ, ओ, झ, स, ब, च, थ, भ, म and व follow: each joins one
      // run into the next by climbing the stem it then descends, so each lifts
      // once less. Every Devanagari headline caption now reads "rightward" and
      // every long caption is shortened to fit two lines. Only the non-Tamil
      // data hash moves, measured after the last caption was settled.
    }).toEqual({
      keys: 469,
      keyHash:
        "91a70905663b348566b44730bf2706c94d9163cf2455f89f0aa1732039a31f73",
      nonTamilDataHash:
        "80e443a1efd9f8f26a42c2a6b939dd3300ad03748c2352fe6b133bf42e4e3502",
      //
      // Kannada base consonants ನ (U+0CA8), ತ (U+0CA4), ದ (U+0CA6), ರ (U+0CB0),
      // ಕ (U+0C95) and ಗ (U+0C97), taught since chapters 1 to 7 but never
      // given a cited stroke-order source, now cite Gopala Krishna A's Commons
      // animations and gain a ductus each. Keys move 439 -> 445 and Kannada
      // 13 -> 19, with the ordered key hash and the non-Tamil data hash,
      // measured after the last caption was settled; Tamil and both
      // shared-identity values remain unchanged.
      //
      // Kannada base consonants ಬ (U+0CAC), ಳ (U+0CB3), ಯ (U+0CAF), ಡ (U+0CA1),
      // ಹ (U+0CB9) and ಸ (U+0CB8), taught since chapters 4 to 12 but never
      // given a cited stroke-order source, now cite Gopala Krishna A's Commons
      // animations and gain a ductus each. Keys move
      // 445 -> 451 and Kannada 19 -> 25, with the ordered key hash and the
      // non-Tamil data hash, measured after the last caption was settled;
      // Tamil and both shared-identity values remain unchanged.
      //
      // Kannada base consonants ಚ (U+0C9A), ಪ (U+0CAA), ಝ (U+0C9D), ಥ (U+0CA5),
      // ಮ (U+0CAE), ಲ (U+0CB2), ವ (U+0CB5) and ಜ (U+0C9C), taught since
      // chapters 13 to 72 but never given a cited stroke-order source, now cite
      // Gopala Krishna A's Commons animations and gain a ductus each. Keys move
      // 451 -> 459 and Kannada 25 -> 33, with the ordered key hash and the
      // non-Tamil data hash, measured after the last caption was settled;
      // Tamil and both shared-identity values remain unchanged.
    }).toEqual({
      keys: 459,
      keyHash:
        "eee5d48210cc21343b65fe014f8a9bf47a584f39a027373d32488ff7317dbc30",
      nonTamilDataHash:
        "8eafae84f08917148b7d37ec2e54c9ad4b4d6b258c7ce7f90d6b6ac0b93dfc4b",
      sharedIdentityGroups: 17,
      sharedIdentityHash:
        "59b284847b09cda1297d9cabb3ba4886172bace6323dc93db8d58c9ee5bbf454",
      counts: {
        arabic: 32,
        chinese: 60,
        cyrillic: 33,
        devanagari: 44,
        gujarati: 44,
        hebrew: 22,
        japanese: 50,
        kannada: 33,
        malayalam: 14,
        "perso-arabic": 24,
        tamil: 29,
        telugu: 43,
        "urdu-nastaliq": 31,
      },
    });
  });

  it("discovers one source and evidence owner for every Tamil glyph", () => {
    const tamilGlyphs = Object.values(DUCTUS)
      .filter((letter) => letter.script === "tamil")
      .map((letter) => letter.glyph);
    const sourceNames = tamilGlyphs.map(
      (glyph) => `${glyphOwnerName(glyph)}.ts`,
    );
    const evidenceNames = tamilGlyphs.map(
      (glyph) => `${glyphOwnerName(glyph)}.test.ts`,
    );
    const sourceDir = resolve(packageRoot, "src/strokes/tamil");
    const evidenceDir = resolve(packageRoot, "tests/strokes/tamil");

    expect(regularOwnerNames(sourceDir)).toEqual(sourceNames.sort());
    expect(regularOwnerNames(evidenceDir)).toEqual(evidenceNames.sort());
    expect(
      readdirSync(resolve(packageRoot, "tests/strokes"))
        .filter((name) => name.startsWith("tamil"))
        .sort(),
    ).toEqual(["tamil"]);

    const assembly = readFileSync(
      resolve(packageRoot, "src/strokes/tamil.ts"),
      "utf8",
    );
    expect(assembly).not.toMatch(/[\u0b80-\u0bff]/u);
    expect(assembly).not.toMatch(
      /\b(?:script|glyph|strokes|segments|label|path|source|citation|url|variation|x|y)\s*:/,
    );

    for (const glyph of tamilGlyphs) {
      const owner = glyphOwnerName(glyph);
      const source = readFileSync(resolve(sourceDir, `${owner}.ts`), "utf8");
      const evidence = readFileSync(
        resolve(evidenceDir, `${owner}.test.ts`),
        "utf8",
      );
      const claims = evidenceOwnerClaims(`${owner}.test.ts`, evidence);
      expect(source.match(/\bexport\b/g)).toHaveLength(1);
      expect(
        source.match(/export const entry: DuctusEntry = \[/g),
      ).toHaveLength(1);
      expect(
        sourceOwnerGlyphs(`${owner}.ts`, source),
        `${owner} tuple glyph`,
      ).toEqual([glyph]);
      expect(claims.imports, `${owner} source-owner imports`).toEqual([owner]);
      expect(new Set(claims.lookups), `${owner} DUCTUS lookups`).toEqual(
        new Set([glyph]),
      );
      expect(evidence).toContain("preserves the exact glyph-owned data");
    }
  });

  it("keeps authored entries in stable owner modules", () => {
    const ownerDir = resolve(packageRoot, "src/strokes");
    expect(
      readdirSync(ownerDir)
        .filter((name) => name.endsWith(".ts") && name !== "registry.ts")
        .map((name) => name.replace(/\.ts$/, ""))
        .sort(),
    ).toEqual(ownerNames);

    const compatibilitySource = readFileSync(
      resolve(packageRoot, "src/strokes.ts"),
      "utf8",
    );
    expect(compatibilitySource).not.toMatch(/\[ductusKey\([^\n]+\)\]\s*:/);
    expect(compatibilitySource).not.toMatch(/^\s*["'][^"']+["']\s*:\s*\{\s*$/m);
  });

  it("keeps script-specific claims out of the two shared evidence roots", () => {
    for (const name of ["strokes.test.ts", "ductusview.test.ts"]) {
      const source = readFileSync(resolve(packageRoot, "tests", name), "utf8");
      expect(
        source,
        `${name} imports an owner-specific font fixture`,
      ).not.toMatch(/from\s+["']\.\/support\/font-fixtures["']/);
      expect(source, `${name} directly looks up an owner glyph`).not.toMatch(
        /DUCTUS\s*\[\s*["'][^"']+["']\s*\]/,
      );
      expect(source, `${name} names an owner script`).not.toMatch(
        /\b(?:arabic|chinese|cyrillic|devanagari|gujarati|hebrew|japanese|kannada|malayalam|tamil|telugu|urdu)\b/i,
      );
      expect(source, `${name} embeds a native owner-script glyph`).not.toMatch(
        /[\u0400-\u052f\u0590-\u06ff\u0900-\u097f\u0a80-\u0aff\u0b80-\u0cff\u0d00-\u0d7f\u3040-\u30ff\u3400-\u9fff]/u,
      );
    }
  });
});
