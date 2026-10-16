import { readFileSync, readdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import ts from "typescript";
import { expect, it } from "vitest";

const caseDirectory = dirname(fileURLToPath(import.meta.url));
const thisCase = fileURLToPath(import.meta.url).split(/[\\/]/u).at(-1);
const censusOwner =
  "pins-sanskrit-a1-exam-coverage-which-nothing-read-before-c2d225a1.case.ts";
const aggregateFields = new Set([
  "enumerated",
  "covered",
  "unmapped",
  "partial",
  "byCategory",
]);

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

function isCoverageCall(expression: ts.Expression): boolean {
  const value = unwrap(expression);
  return (
    ts.isCallExpression(value) &&
    ts.isIdentifier(value.expression) &&
    value.expression.text === "measureExamCoverage"
  );
}

function aggregateCoverageEvidence(filename: string, source: string): string[] {
  const sourceFile = ts.createSourceFile(
    filename,
    source,
    ts.ScriptTarget.Latest,
    true,
    ts.ScriptKind.TS,
  );
  const coverageBindings = new Set<string>();

  // Follow direct aliases as well as the original binding. This makes a local
  // rename immaterial to the ownership rule.
  let changed = true;
  while (changed) {
    changed = false;
    const collect = (node: ts.Node): void => {
      if (
        ts.isVariableDeclaration(node) &&
        ts.isIdentifier(node.name) &&
        node.initializer
      ) {
        const initializer = unwrap(node.initializer);
        const isAlias =
          ts.isIdentifier(initializer) && coverageBindings.has(initializer.text);
        if (
          (isCoverageCall(initializer) || isAlias) &&
          !coverageBindings.has(node.name.text)
        ) {
          coverageBindings.add(node.name.text);
          changed = true;
        }
      }
      ts.forEachChild(node, collect);
    };
    collect(sourceFile);
  }

  const evidence: string[] = [];
  const isCoverageBinding = (expression: ts.Expression): boolean => {
    const value = unwrap(expression);
    return ts.isIdentifier(value) && coverageBindings.has(value.text);
  };
  const record = (node: ts.Node, reason: string): void => {
    const position = sourceFile.getLineAndCharacterOfPosition(node.getStart(sourceFile));
    evidence.push(`${position.line + 1}:${position.character + 1} ${reason}`);
  };
  const visit = (node: ts.Node): void => {
    if (
      ts.isPropertyAccessExpression(node) &&
      isCoverageBinding(node.expression) &&
      aggregateFields.has(node.name.text)
    ) {
      record(node, `reads aggregate field ${node.name.text}`);
    } else if (ts.isElementAccessExpression(node) && isCoverageBinding(node.expression)) {
      const key = node.argumentExpression && unwrap(node.argumentExpression);
      if (!key || !ts.isStringLiteralLike(key) || key.text !== "points") {
        record(node, "reads aggregate coverage through bracket access");
      }
    } else if (
      ts.isVariableDeclaration(node) &&
      ts.isObjectBindingPattern(node.name) &&
      node.initializer &&
      (isCoverageCall(node.initializer) || isCoverageBinding(node.initializer))
    ) {
      for (const element of node.name.elements) {
        const property = element.propertyName ?? element.name;
        const name =
          ts.isIdentifier(property) || ts.isStringLiteralLike(property)
            ? property.text
            : undefined;
        if (element.dotDotDotToken || name !== "points") {
          record(element, `destructures aggregate coverage field ${name ?? "<dynamic>"}`);
        }
      }
    } else if (ts.isSpreadAssignment(node) && isCoverageBinding(node.expression)) {
      record(node, "spreads the aggregate coverage result");
    } else if (
      ts.isCallExpression(node) &&
      node.arguments.some((argument) => isCoverageBinding(argument))
    ) {
      record(node, "passes the aggregate coverage result to a matcher or helper");
    } else if (
      ts.isReturnStatement(node) &&
      node.expression &&
      isCoverageBinding(node.expression)
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
