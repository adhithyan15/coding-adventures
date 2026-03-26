import { describe, it, expect } from "vitest";
import { parseExcelFormula } from "../src/parser.js";
import { isASTNode } from "@coding-adventures/parser";
import type { ASTNode } from "@coding-adventures/parser";
import type { Token } from "@coding-adventures/lexer";

function findNodes(node: ASTNode, ruleName: string): ASTNode[] {
  const results: ASTNode[] = [];
  if (node.ruleName === ruleName) {
    results.push(node);
  }
  for (const child of node.children) {
    if (isASTNode(child)) {
      results.push(...findNodes(child, ruleName));
    }
  }
  return results;
}

function findTokens(node: ASTNode): Token[] {
  const results: Token[] = [];
  for (const child of node.children) {
    if (isASTNode(child)) {
      results.push(...findTokens(child));
    } else {
      results.push(child as Token);
    }
  }
  return results;
}

function parseAndCollect(formula: string): { ast: ASTNode; tokens: Token[] } {
  const ast = parseExcelFormula(formula);
  return { ast, tokens: findTokens(ast) };
}

describe("Excel formula parser conformance matrix", () => {
  const parseableFormulas = [
    "1",
    "42",
    "\"hello\"",
    "TRUE",
    "FALSE",
    "#N/A",
    "=1+2*3",
    "=(1+2)*3",
    "-1",
    "+1",
    "--A1",
    "10%",
    "A1+10%",
    "2^3^4",
    "A1&B1",
    "A1=B1",
    "A1<>B1",
    "A1<=B1",
    "A1>=B1",
    "SUM(A1:B3)",
    "AVERAGE(A1,B1,C1)",
    "IF(TRUE,\"ok\",#N/A)",
    "IF(A1,,B1)",
    "SUM(A1:B2,C1:D2)",
    "SUM(A1:B2 C1:D2)",
    "SUM((A1:B2))",
    "{1,2;3,4}",
    "{TRUE,\"x\";#N/A,5}",
    "A1:B10",
    "A1:B2 C3:D4",
    "A1:B2,C3:D4",
    "A:C",
    "1:3",
    "SUM(A:C,1:3)",
    "'Sales Data'!A1",
    "Sheet1!A1",
    "[Book.xlsx]Sheet1!A1",
    "[Book.xlsx]Sheet1:Sheet3!A1",
    "Sales:Marketing!B3",
    "Sheet2:Sheet6!A2:A5",
    "!A1",
    "!MyName",
    "$A:$C",
    "$1:$3",
    "$A$1:$B$2",
    "'Bob''s Sales'!A1",
    "[Book.xlsx]'Sales Data'!$A$1",
    "MyName",
    "Table1[[#Headers],[Region]:[Sales]]",
    "DeptSales[Sales Amount]",
    "DeptSales[[#Totals],[Sales Amount]]",
    "DeptSales[[#Data],[Commission Amount]]",
    "Table1[#All]",
    "Table1[#Data]",
    "Table1[#Headers]",
    "Table1[#Totals]",
    "Table1[@[Sales Amount]]",
    "Table1[[#All],[Sales Amount]]",
    "Table1[[#Headers],[#Data],[Sales Amount]]",
    "[Sales Amount]",
    "[@[Sales Amount]]",
    "[@[% Commission]]",
    "=[Sales Amount]*[% Commission]",
    "=[@[Sales Amount]]*[@[% Commission]]",
    "=SUM(DeptSales[Sales Amount])",
    "=SUM(DeptSales[[#Totals],[Sales Amount]],DeptSales[[#Data],[Commission Amount]])",
    "=SUM(Sales:Marketing!B3)",
    "=SUM(Sheet2:Sheet6!A2:A5)",
    "=COLUMNS({1,2,3;4,5,6})",
    "=SUM(([Book.xlsx]Sheet1:Sheet3!A:C Table1[[#Data],[Region]:[Sales]]),IF(!Flag,10%,#N/A),{1,2;3,4})",
    "  =SUM(A1:B2)  ",
    "SUM( A1 , B1 )",
    "$A : $C",
    "$1 : $3",
    "[Book.xlsx]Sheet1!Table1[#All]",
    "'Sales Data'!Table1[[#Headers],[Sales Amount]]",
    "SUM(Table1[#All],DeptSales[Sales Amount],Sheet1!A1)",
    "SUM(IF(TRUE,A1,B1),AVERAGE(C1:C3),MAX(D1,D2,D3))",
    "SUM(,A1)",
    "SUM(A1,,)",
  ];

  it.each(parseableFormulas)("parses %s", (formula) => {
    expect(() => parseExcelFormula(formula)).not.toThrow();
  });

  const atomicExpressions = [
    "1",
    "A1",
    "$A:$C",
    "TRUE",
    "\"x\"",
    "#N/A",
    "SUM(A1:B2)",
    "DeptSales[Sales Amount]",
    "'Sales Data'!A1",
  ];

  const binaryOperators = [
    "+",
    "-",
    "*",
    "/",
    "^",
    "&",
    "=",
    "<>",
    "<=",
    ">=",
  ];

  const generatedBinaryFormulas = atomicExpressions.flatMap((left) =>
    atomicExpressions.map((right) => `${left}${binaryOperators[0]}${right}`),
  );

  const representativeBinaryFormulas = [
    ...atomicExpressions.flatMap((left) =>
      atomicExpressions.slice(0, 4).flatMap((right, index) => {
        const op = binaryOperators[index % binaryOperators.length];
        return `${left}${op}${right}`;
      }),
    ),
    ...generatedBinaryFormulas.slice(0, 8),
  ];

  it.each(representativeBinaryFormulas)("parses generated binary formula %s", (formula) => {
    expect(() => parseExcelFormula(formula)).not.toThrow();
  });

  const allOperatorBinaryFormulas = binaryOperators.flatMap((operator) =>
    atomicExpressions.slice(0, 6).flatMap((left) =>
      atomicExpressions.slice(0, 6).map((right) => `${left}${operator}${right}`),
    ),
  );

  it.each(allOperatorBinaryFormulas)("parses expanded generated binary formula %s", (formula) => {
    expect(() => parseExcelFormula(formula)).not.toThrow();
  });

  const unaryAtoms = [
    "1",
    "A1",
    "$A:$C",
    "\"x\"",
    "SUM(A1:B2)",
    "DeptSales[Sales Amount]",
  ];

  const generatedUnaryFormulas = unaryAtoms.flatMap((atom) => [
    `+${atom}`,
    `-${atom}`,
    `+${atom}%`,
    `-${atom}%`,
  ]);

  it.each(generatedUnaryFormulas)("parses generated unary formula %s", (formula) => {
    expect(() => parseExcelFormula(formula)).not.toThrow();
  });

  const generatedReferenceFormulas = [
    "A1:B2",
    "$A:$C",
    "$1:$3",
    "Sheet1!A1:B2",
    "'Sales Data'!A1:B2",
    "[Book.xlsx]Sheet1!A1:B2",
    "A1:B2 C3:D4",
    "A1:B2,C3:D4",
    "SUM(A1:B2 C3:D4)",
    "SUM($A:$C,$1:$3)",
    "$A : $C",
    "$1 : $3",
    "[Book.xlsx]Sheet1!Table1[#All]",
    "'Sales Data'!Table1[[#Headers],[Sales Amount]]",
  ];

  it.each(generatedReferenceFormulas)("parses generated reference formula %s", (formula) => {
    expect(() => parseExcelFormula(formula)).not.toThrow();
  });

  const nodeExpectationCases: Array<[string, string, number]> = [
    ["=1+2*3", "comparison_expr", 1],
    ["SUM(A1:B3)", "function_call", 1],
    ["SUM(A1:B3)", "reference_expression", 1],
    ["IF(A1,,B1)", "function_argument_list", 1],
    ["{1,2;3,4}", "array_constant", 1],
    ["Table1[[#Headers],[Region]:[Sales]]", "structure_reference", 1],
    ["DeptSales[Sales Amount]", "structure_reference", 1],
    ["[Sales Amount]", "structure_reference", 1],
    ["A1:B2,C3:D4", "union_reference", 1],
    ["A1:B2 C3:D4", "intersection_reference", 1],
    ["Sales:Marketing!B3", "prefixed_reference", 1],
    ["[Book.xlsx]Sheet1:Sheet3!A1", "prefixed_reference", 1],
    ["!A1", "bang_reference", 1],
    ["!MyName", "bang_name", 1],
    ["$A:$C", "range_reference", 1],
    ["$1:$3", "range_reference", 1],
    ["Table1[#All]", "structure_reference", 1],
    ["Table1[@[Sales Amount]]", "structure_reference", 1],
  ];

  it.each(nodeExpectationCases)(
    "finds %s node(s) in %s",
    (formula, ruleName, expectedCount) => {
      const ast = parseExcelFormula(formula);
      expect(findNodes(ast, ruleName)).toHaveLength(expectedCount);
    },
  );

  it("normalizes column and row range references", () => {
    const { tokens } = parseAndCollect("SUM(A:C,1:3)");
    expect(tokens.some((token) => token.type === "COLUMN_REF")).toBe(true);
    expect(tokens.some((token) => token.type === "ROW_REF")).toBe(true);
  });

  it("parses absolute A1 column and row ranges from the OpenXML productions", () => {
    const { tokens } = parseAndCollect("SUM($A:$C,$1:$3,$A$1:$B$2)");
    expect(tokens.some((token) => token.type === "DOLLAR")).toBe(true);
    expect(tokens.filter((token) => token.type === "CELL").length).toBeGreaterThanOrEqual(2);
  });

  it("preserves 3-D and workbook prefixes", () => {
    const { tokens } = parseAndCollect("=SUM([Book.xlsx]Sheet1:Sheet3!A1)");
    expect(tokens.some((token) => token.type === "REF_PREFIX")).toBe(true);
    expect(tokens.some((token) => token.type === "CELL")).toBe(true);
  });

  it("captures intersection spaces as real tokens", () => {
    const { tokens } = parseAndCollect("A1:B2 C3:D4");
    expect(tokens.some((token) => token.type === "SPACE")).toBe(true);
  });

  it("captures structured reference tokens in table formulas", () => {
    const { tokens } = parseAndCollect("=SUM(DeptSales[[#Totals],[Sales Amount]],DeptSales[[#Data],[Commission Amount]])");
    expect(tokens.some((token) => token.type === "TABLE_NAME")).toBe(true);
    expect(tokens.some((token) => token.type === "STRUCTURED_KEYWORD")).toBe(true);
    expect(tokens.some((token) => token.type === "STRUCTURED_COLUMN")).toBe(true);
  });

  it("captures bang references and names", () => {
    const { tokens } = parseAndCollect("!A1,!MyName");
    expect(tokens.filter((token) => token.type === "BANG")).toHaveLength(2);
    expect(tokens.some((token) => token.type === "CELL")).toBe(true);
    expect(tokens.some((token) => token.type === "NAME")).toBe(true);
  });

  it("covers dense mixed formulas across the grammar surface", () => {
    const { ast, tokens } = parseAndCollect("=SUM(([Book.xlsx]Sheet1:Sheet3!A:C Table1[[#Data],[Region]:[Sales]]),IF(!Flag,10%,#N/A),{1,2;3,4})");
    expect(findNodes(ast, "function_call").length).toBeGreaterThanOrEqual(2);
    expect(findNodes(ast, "structure_reference")).toHaveLength(1);
    expect(findNodes(ast, "array_constant")).toHaveLength(1);
    expect(tokens.some((token) => token.type === "REF_PREFIX")).toBe(true);
    expect(tokens.some((token) => token.type === "COLUMN_REF")).toBe(true);
    expect(tokens.some((token) => token.type === "PERCENT")).toBe(true);
    expect(tokens.some((token) => token.type === "ERROR_CONSTANT")).toBe(true);
  });

  const invalidFormulas = [
    " SUM ( A1 : B2 ) ",
    "SUM(A1:B2",
    "A1::B2",
    "A1:B2:C3",
    "Table1[",
    "Table1[]]",
    "A1 + * B2",
    "SUM())",
  ];

  it.each(invalidFormulas)("rejects invalid formula %s", (formula) => {
    expect(() => parseExcelFormula(formula)).toThrow();
  });
});

describe("official Microsoft examples", () => {
  // Sources:
  // - Microsoft Support: Using structured references with Excel tables
  // - Microsoft Support: Create a 3-D reference to the same cell range on multiple worksheets
  // - Microsoft Support: Use array constants in array formulas
  const officialExamples = [
    "=SUM(DeptSales[Sales Amount])",
    "=SUM(DeptSales[[#Totals],[Sales Amount]],DeptSales[[#Data],[Commission Amount]])",
    "=[Sales Amount]*[% Commission]",
    "=[@[Sales Amount]]*[@[% Commission]]",
    "=SUM(Sales:Marketing!B3)",
    "=SUM(Sheet2:Sheet6!A2:A5)",
    "=COLUMNS({1,2,3;4,5,6})",
  ];

  it.each(officialExamples)("parses official example %s", (formula) => {
    expect(() => parseExcelFormula(formula)).not.toThrow();
  });
});
