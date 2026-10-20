import { readFileSync, readdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import ts from "typescript";
import { expect, it } from "vitest";

const caseDirectory = dirname(fileURLToPath(import.meta.url));
const thisCase = fileURLToPath(import.meta.url).split(/[\\/]/u).at(-1);
const censusOwner =
  "pins-sanskrit-a1-exam-coverage-which-nothing-read-before-c2d225a1.case.ts";

function importsAggregateCoverageCapability(filename: string, source: string): boolean {
  const sourceFile = ts.createSourceFile(
    filename,
    source,
    ts.ScriptTarget.Latest,
    true,
    ts.ScriptKind.TS,
  );
  let found = false;
  const visit = (node: ts.Node): void => {
    if (ts.isImportDeclaration(node) && node.importClause) {
      const moduleName = ts.isStringLiteral(node.moduleSpecifier)
        ? node.moduleSpecifier.text
        : "";
      const bindings = node.importClause.namedBindings;
      const aggregateModule =
        moduleName.endsWith("src/exam-inventory.js") ||
        moduleName.endsWith("src/index.js");
      if (
        (aggregateModule && node.importClause.name) ||
        (aggregateModule && bindings && ts.isNamespaceImport(bindings)) ||
        (bindings &&
          ts.isNamedImports(bindings) &&
          bindings.elements.some(
            (element) =>
              (element.propertyName ?? element.name).text === "measureExamCoverage",
          ))
      ) {
        found = true;
      }
    } else if (
      ts.isCallExpression(node) &&
      ts.isIdentifier(node.expression) &&
      node.expression.text === "measureExamCoverage"
    ) {
      // This also makes standalone adversarial fixtures meaningful. A real
      // shard would additionally need the import above in order to compile.
      found = true;
    }
    ts.forEachChild(node, visit);
  };
  visit(sourceFile);
  return found;
}

it("recognizes renamed and namespace imports of aggregate coverage", () => {
  const violatingSources = [
    'import { measureExamCoverage } from "../../../src/exam-inventory.js";',
    'import { measureExamCoverage as measure } from "../../../src/exam-inventory.js";',
    'import * as inventoryCoverage from "../../../src/exam-inventory.js";',
    'import { measureExamCoverage } from "../../../src/index.js";',
    'import { measureExamCoverage as measure } from "../../../src/index.js";',
    'import * as humanLanguageData from "../../../src/index.js";',
    "measureExamCoverage(inventory, lessons);",
  ];
  for (const [index, source] of violatingSources.entries()) {
    expect(importsAggregateCoverageCapability(`violation-${index}.ts`, source)).toBe(true);
  }
});

it("does not mistake the point-only helper, comments, or strings for aggregate access", () => {
  const source = `
    import { sanskritA1PointCoverage } from "../sanskrit-a1-point-coverage.js";
    expect(sanskritA1PointCoverage("SA-A1-M-07")).toMatchObject({
      covered: true,
      unmapped: false,
    });
    // measureExamCoverage(inventory, lessons)
    const documentation = "measureExamCoverage remains census-owned";
  `;
  expect(importsAggregateCoverageCapability("point-only.ts", source)).toBe(false);
});

it("keeps Sanskrit A1 aggregate coverage capability in one census owner", () => {
  const owners = readdirSync(caseDirectory)
    .filter((name) => name.endsWith(".case.ts") && name !== thisCase)
    .filter((name) =>
      importsAggregateCoverageCapability(
        name,
        readFileSync(join(caseDirectory, name), "utf8"),
      ),
    )
    .sort();

  expect(owners).toEqual([censusOwner]);
});
