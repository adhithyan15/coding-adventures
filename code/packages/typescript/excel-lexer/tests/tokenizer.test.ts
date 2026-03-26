import { describe, it, expect } from "vitest";
import { createExcelLexer, tokenizeExcelFormula } from "../src/tokenizer.js";

function tokenTypes(source: string): string[] {
  return tokenizeExcelFormula(source).map((t) => t.type);
}

function tokenValues(source: string): string[] {
  return tokenizeExcelFormula(source).map((t) => t.value);
}

describe("Excel formula lexer", () => {
  const tokenTypeCases: Array<[string, string[]]> = [
    ["=1+2", ["EQUALS", "NUMBER", "PLUS", "NUMBER", "EOF"]],
    ["1-2", ["NUMBER", "MINUS", "NUMBER", "EOF"]],
    ["1*2/3", ["NUMBER", "STAR", "NUMBER", "SLASH", "NUMBER", "EOF"]],
    ["2^3", ["NUMBER", "CARET", "NUMBER", "EOF"]],
    ["10%", ["NUMBER", "PERCENT", "EOF"]],
    ["A1>=10%", ["CELL", "GREATER_EQUALS", "NUMBER", "PERCENT", "EOF"]],
    ["A1<>B2", ["CELL", "NOT_EQUALS", "CELL", "EOF"]],
    ['"hello"&"world"', ["STRING", "AMP", "STRING", "EOF"]],
    ["TRUE", ["KEYWORD", "EOF"]],
    ["FALSE", ["KEYWORD", "EOF"]],
    ["#N/A", ["ERROR_CONSTANT", "EOF"]],
    ["SUM(A1:B3)", ["FUNCTION_NAME", "LPAREN", "CELL", "COLON", "CELL", "RPAREN", "EOF"]],
    ["IF(TRUE,\"ok\",#N/A)", ["FUNCTION_NAME", "LPAREN", "KEYWORD", "COMMA", "STRING", "COMMA", "ERROR_CONSTANT", "RPAREN", "EOF"]],
    ["IF(A1,,B1)", ["FUNCTION_NAME", "LPAREN", "CELL", "COMMA", "COMMA", "CELL", "RPAREN", "EOF"]],
    ["MAX(A1,B1,C1)", ["FUNCTION_NAME", "LPAREN", "CELL", "COMMA", "CELL", "COMMA", "CELL", "RPAREN", "EOF"]],
    ["A1:B2 C3:D4", ["CELL", "COLON", "CELL", "SPACE", "CELL", "COLON", "CELL", "EOF"]],
    ["A1:B2,C3:D4", ["CELL", "COLON", "CELL", "COMMA", "CELL", "COLON", "CELL", "EOF"]],
    ["(A1:B2)", ["LPAREN", "CELL", "COLON", "CELL", "RPAREN", "EOF"]],
    ["'Sales Data'!A1", ["REF_PREFIX", "CELL", "EOF"]],
    ["Sheet1!A1", ["REF_PREFIX", "CELL", "EOF"]],
    ["[Book.xlsx]Sheet1!A1", ["REF_PREFIX", "CELL", "EOF"]],
    ["[Book.xlsx]Sheet1:Sheet3!A1", ["REF_PREFIX", "CELL", "EOF"]],
    ["[Book.xlsx]'Sales Data'!$A$1", ["REF_PREFIX", "CELL", "EOF"]],
    ["'Bob''s Sales'!A1", ["REF_PREFIX", "CELL", "EOF"]],
    ["Sales:Marketing!B3", ["REF_PREFIX", "CELL", "EOF"]],
    ["!A1", ["BANG", "CELL", "EOF"]],
    ["!MyName", ["BANG", "NAME", "EOF"]],
    ["$A:$C", ["DOLLAR", "NAME", "COLON", "DOLLAR", "NAME", "EOF"]],
    ["$1:$3", ["DOLLAR", "NUMBER", "COLON", "DOLLAR", "NUMBER", "EOF"]],
    ["$A$1:$B$2", ["CELL", "COLON", "CELL", "EOF"]],
    ["A:C", ["NAME", "COLON", "NAME", "EOF"]],
    ["1:3", ["NUMBER", "COLON", "NUMBER", "EOF"]],
    ["Table1[[#Headers],[Region]:[Sales]]", ["TABLE_NAME", "LBRACKET", "STRUCTURED_KEYWORD", "COMMA", "STRUCTURED_COLUMN", "COLON", "STRUCTURED_COLUMN", "RBRACKET", "EOF"]],
    ["DeptSales[Sales Amount]", ["TABLE_NAME", "STRUCTURED_COLUMN", "EOF"]],
    ["[Sales Amount]", ["STRUCTURED_COLUMN", "EOF"]],
    ["[@[Sales Amount]]", ["LBRACKET", "AT", "STRUCTURED_COLUMN", "RBRACKET", "EOF"]],
    ["[@[% Commission]]", ["LBRACKET", "AT", "STRUCTURED_COLUMN", "RBRACKET", "EOF"]],
    ["Table1[#All]", ["TABLE_NAME", "STRUCTURED_KEYWORD", "EOF"]],
    ["Table1[#Data]", ["TABLE_NAME", "STRUCTURED_KEYWORD", "EOF"]],
    ["Table1[#Headers]", ["TABLE_NAME", "STRUCTURED_KEYWORD", "EOF"]],
    ["Table1[#Totals]", ["TABLE_NAME", "STRUCTURED_KEYWORD", "EOF"]],
    ["Table1[@[Sales Amount]]", ["TABLE_NAME", "LBRACKET", "AT", "STRUCTURED_COLUMN", "RBRACKET", "EOF"]],
    ["Table1[[#All],[Sales Amount]]", ["TABLE_NAME", "LBRACKET", "STRUCTURED_KEYWORD", "COMMA", "STRUCTURED_COLUMN", "RBRACKET", "EOF"]],
    ["Table1[[#Headers],[#Data],[Sales Amount]]", ["TABLE_NAME", "LBRACKET", "STRUCTURED_KEYWORD", "COMMA", "STRUCTURED_KEYWORD", "COMMA", "STRUCTURED_COLUMN", "RBRACKET", "EOF"]],
    ["{1,2;3,4}", ["LBRACE", "NUMBER", "COMMA", "NUMBER", "SEMICOLON", "NUMBER", "COMMA", "NUMBER", "RBRACE", "EOF"]],
    ["{TRUE,\"x\";#N/A,5}", ["LBRACE", "KEYWORD", "COMMA", "STRING", "SEMICOLON", "ERROR_CONSTANT", "COMMA", "NUMBER", "RBRACE", "EOF"]],
    [" SUM ( A1 : B2 ) ", ["SPACE", "FUNCTION_NAME", "SPACE", "LPAREN", "SPACE", "CELL", "SPACE", "COLON", "SPACE", "CELL", "SPACE", "RPAREN", "SPACE", "EOF"]],
  ];

  it.each(tokenTypeCases)("tokenizes %s", (source, expectedTypes) => {
    expect(tokenTypes(source)).toEqual(expectedTypes);
  });

  const tokenValueCases: Array<[string, string[]]> = [
    ["'Sales Data'!A1", ["'Sales Data'!", "A1", ""]],
    ["IF(TRUE,\"ok\",#N/A)", ["IF", "(", "TRUE", ",", "ok", ",", "#N/A", ")", ""]],
    ["DeptSales[Sales Amount]", ["DeptSales", "[Sales Amount]", ""]],
    ["[Sales Amount]", ["[Sales Amount]", ""]],
    ["Table1[#All]", ["Table1", "[#All]", ""]],
    ["'Bob''s Sales'!A1", ["'Bob''s Sales'!", "A1", ""]],
    ["SUM(A1:B3)", ["SUM", "(", "A1", ":", "B3", ")", ""]],
    ["{1,2;3,4}", ["{", "1", ",", "2", ";", "3", ",", "4", "}", ""]],
  ];

  it.each(tokenValueCases)("captures expected values for %s", (source, expectedValues) => {
    expect(tokenValues(source)).toEqual(expectedValues);
  });

  it("creates a GrammarLexer for advanced Excel handling", () => {
    const lexer = createExcelLexer("SUM(A1:A3)");
    expect(lexer.tokenize().map((token) => token.type)).toEqual([
      "FUNCTION_NAME",
      "LPAREN",
      "CELL",
      "COLON",
      "CELL",
      "RPAREN",
      "EOF",
    ]);
  });

  it("reclassifies function names context-sensitively", () => {
    expect(tokenTypes("SUM(A1)")).toEqual([
      "FUNCTION_NAME",
      "LPAREN",
      "CELL",
      "RPAREN",
      "EOF",
    ]);
    expect(tokenTypes("SUM + 1")).toEqual([
      "NAME",
      "SPACE",
      "PLUS",
      "SPACE",
      "NUMBER",
      "EOF",
    ]);
  });

  it("reclassifies table names context-sensitively", () => {
    expect(tokenTypes("Table1[Region]")).toEqual([
      "TABLE_NAME",
      "STRUCTURED_COLUMN",
      "EOF",
    ]);
    expect(tokenTypes("Table1 + 1")).toEqual([
      "NAME",
      "SPACE",
      "PLUS",
      "SPACE",
      "NUMBER",
      "EOF",
    ]);
  });
});
