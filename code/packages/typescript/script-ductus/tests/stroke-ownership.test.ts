// ---------------------------------------------------------------------------
// stroke-ownership.test.ts — the registry pin, and who may own which glyph
// ---------------------------------------------------------------------------
//
// ONE PIN PER SCRIPT, ONE FILE PER PIN (#12118, #13193).
//
// The first test pins the whole `DUCTUS` registry: every key, their order,
// every byte of non-Tamil data, every script count and the Arabic family's
// shared stroke objects. That pin used to be one literal here, and every
// filmstrip or stroke PR in EVERY script had to rewrite it — keys 636 -> 648,
// a new key hash, a new data hash — so a Kannada PR and a Malayalam PR in
// flight together conflicted on the same five lines, and each of them also
// appended a paragraph of provenance to the same comment above it.
//
// Now each script's numbers live in `tests/stroke-ownership/<script>.json`
// and the few facts that belong to no script — the order of the script
// blocks and the shared-identity values — in `tests/stroke-ownership/
// _registry.json`. `stroke-ownership-pins.ts` explains each field and why the
// split pins exactly what the single literal did, no less. The four-hundred-
// line history of every move (353 -> 648 keys) lives in git, in this file's
// log, and in the CHANGELOG.d fragments that made each move.
//
// After a deliberate change to stroke data, rewrite the pins with
//
//     npm run generate:stroke-ownership
//     (= vitest run tests/stroke-ownership.test.ts --mode write)
//
// review the diff — it should touch ONLY the scripts you meant to change —
// and say in your CHANGELOG.d fragment why each value moved. The write mode is
// the same switch `npm run generate:filmstrip-ledger` uses; without it the
// test only compares, and a missing pin for a new script, a stale pin for a
// removed one, or any moved value fails.
// ---------------------------------------------------------------------------

import {
  existsSync,
  lstatSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  readdirSync,
  rmSync,
  symlinkSync,
  unlinkSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import ts from "typescript";
import { describe, expect, it } from "vitest";

import { DUCTUS, type LetterDuctus } from "../src/strokes";
import {
  REGISTRY_PIN_FILE,
  interleaveScriptKeys,
  loadStrokeOwnershipPins,
  measureStrokeOwnershipPins,
  scriptLayout,
  strokeOwnershipPinFiles,
  type ScriptPin,
} from "./stroke-ownership-pins";

const packageRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const pinDirectory = resolve(packageRoot, "tests/stroke-ownership");
const ownerNames = [
  "arabic-family",
  // Bengali joins as one owner module, like Malayalam and Gujarati.
  "bengali",
  "chinese",
  "cyrillic",
  "devanagari",
  "gujarati",
  // Gurmukhi joins the same way, as one owner module after Bengali.
  "gurmukhi",
  "hebrew",
  "japanese",
  "kannada",
  // Latin joins as one owner module after Gurmukhi (print letters).
  "latin",
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

describe("stroke ownership migration baseline", () => {
  // The gate. In write mode it first rewrites every pin file from the live
  // registry and deletes the pin of any script that no longer exists; in the
  // normal mode it only reads. Either way the comparison below runs, so a
  // regeneration that produced something unreadable still fails here.
  it("pins every script's keys, order and data in its own shard", () => {
    const measured = measureStrokeOwnershipPins(DUCTUS);
    const files = strokeOwnershipPinFiles(measured);

    if (import.meta.env.MODE === "write") {
      mkdirSync(pinDirectory, { recursive: true });
      // Never write through a symlink: not the directory, not a pin file.
      if (!lstatSync(pinDirectory).isDirectory()) throw new Error(`${pinDirectory} is not a real directory`);
      for (const name of readdirSync(pinDirectory)) {
        const stale = join(pinDirectory, name);
        const stat = lstatSync(stale);
        if (!files.has(name) && /^[a-z][a-z0-9-]*\.json$/.test(name) && stat.isFile()) {
          unlinkSync(stale);
        }
      }
      for (const [name, bytes] of files) {
        const target = join(pinDirectory, name);
        if (existsSync(target) && !lstatSync(target).isFile()) throw new Error(`${target} is not a regular file`);
        writeFileSync(target, bytes, "utf8");
      }
    }

    const pinned = loadStrokeOwnershipPins(pinDirectory);
    const regenerate = "run `npm run generate:stroke-ownership` in script-ductus";

    // The SET of pin files must equal the set of scripts, in both directions:
    // a new script cannot slip past without a pin, and a removed script cannot
    // leave a pin behind that nothing checks.
    const live = Object.keys(measured.scripts).sort();
    const onDisk = Object.keys(pinned.scripts).sort();
    const problems: string[] = [];
    for (const script of live) {
      if (!onDisk.includes(script)) {
        problems.push(`NEW ${script}: no tests/stroke-ownership/${script}.json — ${regenerate}`);
      }
    }
    for (const script of onDisk) {
      if (!live.includes(script)) {
        problems.push(`STALE ${script}: no registry entry has this script — delete its pin`);
      }
    }

    // Then every value of every pin. Each moved field is named with its old
    // and new value, so the failure reads like the diff a regeneration makes.
    const fieldsOf = (pin: ScriptPin | undefined): (keyof ScriptPin)[] =>
      pin === undefined ? [] : (Object.keys(pin) as (keyof ScriptPin)[]);
    for (const script of live.filter((name) => onDisk.includes(name))) {
      const want = pinned.scripts[script]!;
      const got = measured.scripts[script]!;
      for (const field of new Set([...fieldsOf(want), ...fieldsOf(got)])) {
        const before = JSON.stringify(want[field]);
        const after = JSON.stringify(got[field]);
        if (before !== after) {
          problems.push(`MOVED ${script}.${field}: ${before} -> ${after} (tests/stroke-ownership/${script}.json)`);
        }
      }
    }
    for (const field of Object.keys(measured.registry) as (keyof typeof measured.registry)[]) {
      const before = JSON.stringify(pinned.registry[field]);
      const after = JSON.stringify(measured.registry[field]);
      if (before !== after) {
        problems.push(`MOVED ${field}: ${before} -> ${after} (tests/stroke-ownership/${REGISTRY_PIN_FILE})`);
      }
    }
    expect(problems, `${problems.join("\n")}\nIf the change is deliberate, ${regenerate}.`).toEqual([]);

    // Belt and braces: the pins as parsed must equal the measurement exactly,
    // so no field can exist on one side and be skipped by the loop above.
    expect(pinned).toEqual(measured);
  });

  // The split must lose nothing. The old literal hashed the ORDERED list of
  // every key; the shards keep only each script's own ordered keys plus the
  // block shape. This rebuilds the full order from exactly those pieces, so if
  // it ever stopped matching, the per-script pins would no longer determine
  // the registry and the gate would have quietly weakened.
  it("rebuilds the exact registry key order from the per-script layout", () => {
    const { scriptRuns, byScript } = scriptLayout(DUCTUS);
    expect(interleaveScriptKeys(scriptRuns, byScript)).toEqual(Object.keys(DUCTUS));

    // The pieces must fit exactly: a block that asks for more keys than its
    // script has, or a script with keys no block consumes, is refused.
    const tamil = byScript.get("tamil")!;
    expect(() =>
      interleaveScriptKeys(
        scriptRuns,
        new Map([...byScript, ["tamil", { ...tamil, runs: [...tamil.runs, 1] }]]),
      ),
    ).toThrow(/never consumes/);
    expect(() =>
      interleaveScriptKeys(
        [...scriptRuns, "tamil"],
        byScript,
      ),
    ).toThrow(/fewer blocks/);
    expect(() => interleaveScriptKeys(["klingon"], byScript)).toThrow(/unknown script/);
  });

  // The point of the split, checked on a copy of the real registry: a change
  // in one script moves that script's pin and nothing else, so two PRs in
  // different scripts can no longer meet in the same file. A brand-new script
  // is the one change that legitimately touches `_registry.json` too.
  it("moves only the changed script's pin when one script changes", () => {
    const baseline = measureStrokeOwnershipPins(DUCTUS);
    const differing = (registry: Record<string, LetterDuctus>): string[] => {
      const next = measureStrokeOwnershipPins(registry);
      const before = strokeOwnershipPinFiles(baseline);
      const after = strokeOwnershipPinFiles(next);
      return [...new Set([...before.keys(), ...after.keys()])]
        .filter((name) => before.get(name) !== after.get(name))
        .sort();
    };
    const lastKannada = Object.keys(DUCTUS).filter((key) => DUCTUS[key]!.script === "kannada").at(-1)!;
    const kannada = DUCTUS[lastKannada]!;

    // A changed caption, deep inside one Kannada glyph.
    const relabelled = structuredClone(kannada);
    relabelled.strokes[0]!.segments[0]!.label = `${relabelled.strokes[0]!.segments[0]!.label} (edited)`;
    expect(differing({ ...DUCTUS, [lastKannada]: relabelled })).toEqual(["kannada.json"]);

    // A glyph appended to the Kannada block, where new glyphs go. It is a deep
    // copy: sharing the original's stroke objects would (rightly) move the
    // shared-identity pin, which is a different change from adding a glyph.
    const keys = Object.keys(DUCTUS);
    const at = keys.indexOf(lastKannada) + 1;
    const withAdded = Object.fromEntries([
      ...keys.slice(0, at).map((key) => [key, DUCTUS[key]!]),
      ["kannada:test-only", { ...structuredClone(kannada), glyph: "test-only" }],
      ...keys.slice(at).map((key) => [key, DUCTUS[key]!]),
    ]);
    expect(differing(withAdded)).toEqual(["kannada.json"]);

    // A glyph removed from Kannada.
    const { [lastKannada]: _removed, ...withRemoved } = DUCTUS;
    expect(differing(withRemoved)).toEqual(["kannada.json"]);

    // Tamil data is pinned glyph by glyph, so editing it moves no shard at all
    // — exactly as the old `nonTamilDataHash` ignored it.
    const tamilKey = Object.keys(DUCTUS).find((key) => DUCTUS[key]!.script === "tamil")!;
    const tamilEdited = structuredClone(DUCTUS[tamilKey]!);
    tamilEdited.strokes[0]!.segments[0]!.label = "edited";
    expect(differing({ ...DUCTUS, [tamilKey]: tamilEdited })).toEqual([]);

    // A new script needs its own pin, and lengthens the block shape.
    expect(differing({ ...DUCTUS, "klingon:a": { ...structuredClone(kannada), script: "klingon" } })).toEqual([
      REGISTRY_PIN_FILE,
      "klingon.json",
    ]);
  });

  // The loader fails closed, like human-language-data's per-track pin loaders:
  // only real files with script-shaped names and exactly the expected fields.
  it("refuses pin files that are not exactly pins", () => {
    const root = mkdtempSync(join(tmpdir(), "script-ductus-ownership-pins-"));
    try {
      const write = (files: Map<string, string>): string => {
        const directory = mkdtempSync(join(root, "pins-"));
        for (const [name, bytes] of files) writeFileSync(join(directory, name), bytes);
        return directory;
      };
      const good = strokeOwnershipPinFiles(measureStrokeOwnershipPins(DUCTUS));
      expect(loadStrokeOwnershipPins(write(good))).toEqual(measureStrokeOwnershipPins(DUCTUS));

      const variant = (name: string, bytes: string | undefined): Map<string, string> => {
        const files = new Map(good);
        if (bytes === undefined) files.delete(name);
        else files.set(name, bytes);
        return files;
      };
      const kannada = JSON.parse(good.get("kannada.json")!) as Record<string, unknown>;
      const cases: [string, Map<string, string>, RegExp][] = [
        ["no registry pin", variant(REGISTRY_PIN_FILE, undefined), /_registry\.json is missing/],
        ["an uppercase name", variant("Kannada.json", good.get("kannada.json")), /unsafe/],
        ["a non-JSON file", variant("notes.txt", "hi"), /unsafe/],
        ["an extra field", variant("kannada.json", JSON.stringify({ ...kannada, note: "x" })), /kannada\.json: expected/],
        ["a missing field", variant("kannada.json", JSON.stringify({ ...kannada, keyHash: undefined })), /kannada\.json: expected/],
        ["a short hash", variant("kannada.json", JSON.stringify({ ...kannada, dataHash: "abc" })), /kannada\.json: expected/],
        ["a zero run", variant("kannada.json", JSON.stringify({ ...kannada, runs: [0] })), /kannada\.json: expected/],
        ["an empty block shape", variant(REGISTRY_PIN_FILE, JSON.stringify({ scriptRuns: [], sharedIdentityGroups: 0, sharedIdentityHash: "0".repeat(64) })), /_registry\.json: expected/],
      ];
      for (const [label, files, error] of cases) {
        expect(() => loadStrokeOwnershipPins(write(files)), label).toThrow(error);
      }

      const linked = write(variant("kannada.json", undefined));
      symlinkSync(join(write(good), "kannada.json"), join(linked, "kannada.json"));
      expect(() => loadStrokeOwnershipPins(linked), "a symlinked pin").toThrow(/unsafe/);
      expect(() => loadStrokeOwnershipPins(join(root, "absent"))).toThrow();
    } finally {
      rmSync(root, { recursive: true, force: true });
    }
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
