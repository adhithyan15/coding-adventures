import { readFileSync, readdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import ts from "typescript";
import { expect, it } from "vitest";

const caseDirectory = dirname(fileURLToPath(import.meta.url));
const thisCase = fileURLToPath(import.meta.url).split(/[\\/]/u).at(-1);
const censusOwner =
  "pins-sanskrit-a1-exam-coverage-which-nothing-read-before-c2d225a1.case.ts";

function unwrap(expression: ts.Expression): ts.Expression {
  if (
    ts.isParenthesizedExpression(expression) ||
    ts.isAsExpression(expression) ||
    ts.isTypeAssertionExpression(expression) ||
    ts.isNonNullExpression(expression)
  ) {
    return unwrap(expression.expression);
  }
  return expression;
}

function aggregateCoverageEvidence(filename: string, source: string): string[] {
  const sourceFile = ts.createSourceFile(
    filename,
    source,
    ts.ScriptTarget.Latest,
    true,
    ts.ScriptKind.TS,
  );
  const coverageCallees = new Set(["measureExamCoverage"]);
  const coverageBindings = new Set<string>();
  const collectImportAliases = (node: ts.Node): void => {
    if (
      ts.isImportSpecifier(node) &&
      (node.propertyName ?? node.name).text === "measureExamCoverage"
    ) {
      coverageCallees.add(node.name.text);
    }
    ts.forEachChild(node, collectImportAliases);
  };
  collectImportAliases(sourceFile);

  const isCoverageCall = (expression: ts.Expression): boolean => {
    const value = unwrap(expression);
    return (
      ts.isCallExpression(value) &&
      ts.isIdentifier(value.expression) &&
      coverageCallees.has(value.expression.text)
    );
  };
  const isCoverageValue = (expression: ts.Expression): boolean => {
    const value = unwrap(expression);
    return (
      isCoverageCall(value) ||
      (ts.isIdentifier(value) && coverageBindings.has(value.text))
    );
  };

  // Follow direct aliases as well as the original binding. This makes a local
  // rename immaterial to the ownership rule.
  let changed = true;
  while (changed) {
    changed = false;
    const collect = (node: ts.Node): void => {
      let binding: ts.Identifier | undefined;
      let initializer: ts.Expression | undefined;
      if (
        ts.isVariableDeclaration(node) &&
        ts.isIdentifier(node.name) &&
        node.initializer
      ) {
        binding = node.name;
        initializer = node.initializer;
      } else if (
        ts.isBinaryExpression(node) &&
        node.operatorToken.kind === ts.SyntaxKind.EqualsToken &&
        ts.isIdentifier(node.left)
      ) {
        binding = node.left;
        initializer = node.right;
      }
      if (
        binding &&
        initializer &&
        isCoverageValue(initializer) &&
        !coverageBindings.has(binding.text)
      ) {
        coverageBindings.add(binding.text);
        changed = true;
      }
      ts.forEachChild(node, collect);
    };
    collect(sourceFile);
  }

  const evidence: string[] = [];
  const record = (node: ts.Node, reason: string): void => {
    const position = sourceFile.getLineAndCharacterOfPosition(node.getStart(sourceFile));
    evidence.push(`${position.line + 1}:${position.character + 1} ${reason}`);
  };
  const isExactPointLookup = (points: ts.PropertyAccessExpression): boolean => {
    const findAccess = points.parent;
    return (
      ts.isPropertyAccessExpression(findAccess) &&
      findAccess.expression === points &&
      findAccess.name.text === "find" &&
      ts.isCallExpression(findAccess.parent) &&
      findAccess.parent.expression === findAccess
    );
  };
  const visit = (node: ts.Node): void => {
    if (
      ts.isPropertyAccessExpression(node) &&
      isCoverageValue(node.expression)
    ) {
      if (node.name.text !== "points" || !isExactPointLookup(node)) {
        record(node, `reads aggregate coverage through ${node.name.text}`);
      }
    } else if (ts.isElementAccessExpression(node) && isCoverageValue(node.expression)) {
      record(node, "reads aggregate coverage through bracket access");
    } else if (
      ts.isVariableDeclaration(node) &&
      ts.isObjectBindingPattern(node.name) &&
      node.initializer &&
      isCoverageValue(node.initializer)
    ) {
      for (const element of node.name.elements) {
        const property = element.propertyName ?? element.name;
        const name =
          ts.isIdentifier(property) || ts.isStringLiteralLike(property)
            ? property.text
            : undefined;
        record(element, `destructures aggregate coverage field ${name ?? "<dynamic>"}`);
      }
    } else if (ts.isSpreadAssignment(node) && isCoverageValue(node.expression)) {
      record(node, "spreads the aggregate coverage result");
    } else if (
      ts.isCallExpression(node) &&
      node.arguments.some((argument) => isCoverageValue(argument))
    ) {
      record(node, "passes the aggregate coverage result to a matcher or helper");
    } else if (
      ts.isReturnStatement(node) &&
      node.expression &&
      isCoverageValue(node.expression)
    ) {
      record(node, "returns the aggregate coverage result");
    }
    ts.forEachChild(node, visit);
  };
  visit(sourceFile);
  return evidence;
}

it("recognizes aggregate pins through ordinary TypeScript refactors", () => {
  const violatingSources = [
    "const result = measureExamCoverage(inventory, lessons); expect(result.covered).toBe(1);",
    "const result = measureExamCoverage(inventory, lessons); const { covered } = result;",
    'const result = measureExamCoverage(inventory, lessons); expect(result["covered"]).toBe(1);',
    "const result = measureExamCoverage(inventory, lessons); assert.deepEqual(result, totals);",
    "expect(measureExamCoverage(inventory, lessons).covered).toBe(1);",
    "expect(measureExamCoverage(inventory, lessons)).toEqual(totals);",
    "const result = measureExamCoverage(inventory, lessons); expect(result.points).toHaveLength(164);",
    "const result = measureExamCoverage(inventory, lessons); expect(result.points.filter((point) => point.covered)).toHaveLength(157);",
    'import { measureExamCoverage as measure } from "../../../src/exam-inventory.js"; const result = measure(inventory, lessons); expect(result.covered).toBe(1);',
    "let result; result = measureExamCoverage(inventory, lessons); expect(result.covered).toBe(1);",
  ];
  for (const [index, source] of violatingSources.entries()) {
    expect(aggregateCoverageEvidence(`violation-${index}.ts`, source)).not.toEqual([]);
  }
});

it("does not mistake point checks, comments, or strings for aggregate owners", () => {
  const source = `
    const coverage = measureExamCoverage(inventory, lessons);
    expect(coverage.points.find((point) => point.id === "SA-A1-M-07")).toMatchObject({
      covered: true,
      unmapped: false,
    });
    // expect(coverage.covered).toBe(157)
    const documentation = "coverage.byCategory is census-owned";
  `;
  expect(aggregateCoverageEvidence("point-only.ts", source)).toEqual([]);
});

it("keeps Sanskrit A1 aggregate coverage pins in one census owner", () => {
  const owners = readdirSync(caseDirectory)
    .filter((name) => name.endsWith(".case.ts") && name !== thisCase)
    .filter((name) => {
      const source = readFileSync(join(caseDirectory, name), "utf8");
      return aggregateCoverageEvidence(name, source).length > 0;
    })
    .sort();

  expect(owners).toEqual([censusOwner]);
});
