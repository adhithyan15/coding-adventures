import { existsSync, lstatSync, readFileSync, readdirSync } from "node:fs";
import { join, resolve } from "node:path";
import ts from "typescript";
import { describe, expect, it } from "vitest";

const repoRoot = resolve(import.meta.dirname, "../../../../..");
const inventoryRoot = join(
  repoRoot,
  "code/learning/human-languages/data/scripts/tamil.d",
);
const evidenceRoot = join(import.meta.dirname, "script-inventories/tamil");

function names(root: string, section: "letters" | "marks"): string[] {
  return readdirSync(join(root, section))
    .map((name) => {
      const path = join(root, section, name);
      const stat = lstatSync(path);
      expect(stat.isSymbolicLink(), `${path} must not be a symbolic link`).toBe(
        false,
      );
      expect(stat.isFile(), `${path} must be a regular file`).toBe(true);
      return name;
    })
    .sort();
}

function evidenceIds(filename: string, source: string): string[] {
  const ids: string[] = [];
  const sourceFile = ts.createSourceFile(
    filename,
    source,
    ts.ScriptTarget.Latest,
    true,
    ts.ScriptKind.TS,
  );
  const visit = (node: ts.Node): void => {
    if (
      ts.isPropertyAssignment(node) &&
      ts.isIdentifier(node.name) &&
      node.name.text === "id" &&
      ts.isStringLiteral(node.initializer)
    ) {
      ids.push(node.initializer.text);
    }
    ts.forEachChild(node, visit);
  };
  visit(sourceFile);
  return ids;
}

describe("Tamil canonical inventory ownership", () => {
  it("gives every glyph or mark one matching stable-code-point evidence owner", () => {
    expect(existsSync(join(inventoryRoot, "../tamil.json"))).toBe(false);
    expect(
      existsSync(
        join(import.meta.dirname, "script-inventories/tamil.evidence.ts"),
      ),
    ).toBe(false);

    const metadata = JSON.parse(
      readFileSync(join(inventoryRoot, "_meta.json"), "utf8"),
    );
    expect(metadata.script).toBe("tamil");
    expect(metadata.complete).toBe(false);
    expect(metadata).not.toHaveProperty("letters");
    expect(metadata).not.toHaveProperty("marks");

    for (const section of ["letters", "marks"] as const) {
      const inventory = names(inventoryRoot, section);
      const evidenceNames = names(evidenceRoot, section);
      const evidence = evidenceNames.map((name) =>
        name.replace(/\.evidence\.ts$/, ".json"),
      );
      expect(evidence).toEqual(inventory);
      expect(new Set(inventory).size).toBe(inventory.length);
      expect(
        inventory.every((name) => /^\d{4}-U-[0-9A-F]+\.json$/.test(name)),
      ).toBe(true);
      for (const name of evidenceNames) {
        const match = name.match(/^\d{4}-(U-[0-9A-F]+)\.evidence\.ts$/);
        expect(match, `${section}/${name} has a canonical owner name`).not.toBe(
          null,
        );
        const source = readFileSync(join(evidenceRoot, section, name), "utf8");
        expect(
          evidenceIds(name, source),
          `${section}/${name} id claims`,
        ).toEqual([match![1]]);
      }
    }
    expect(names(inventoryRoot, "letters")).toHaveLength(26);
    expect(names(inventoryRoot, "marks")).toHaveLength(9);
  });

  it("discovers nested evidence without a hand-maintained Tamil manifest", () => {
    const source = readFileSync(
      join(import.meta.dirname, "integration.test.ts"),
      "utf8",
    );
    expect(source).toContain('"./script-inventories/**/*.evidence.ts"');
    expect(source).not.toMatch(/import .*tamil\/letters|import .*tamil\/marks/);
  });
});
