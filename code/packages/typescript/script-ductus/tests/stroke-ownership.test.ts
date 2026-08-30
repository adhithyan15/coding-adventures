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
    }).toEqual({
      keys: 341,
      keyHash:
        "fb5711c158184044a6d0b090ae02ec1f6da171d98d6553e2ba35bacf0032a992",
      nonTamilDataHash:
        "a01795782a9d6ed521878796557c22fc2b829aca5b548f71b71826e45807488d",
      sharedIdentityGroups: 17,
      sharedIdentityHash:
        "59b284847b09cda1297d9cabb3ba4886172bace6323dc93db8d58c9ee5bbf454",
      counts: {
        arabic: 32,
        chinese: 43,
        cyrillic: 33,
        devanagari: 43,
        gujarati: 44,
        hebrew: 22,
        japanese: 15,
        kannada: 13,
        malayalam: 10,
        "perso-arabic": 24,
        tamil: 25,
        telugu: 6,
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
