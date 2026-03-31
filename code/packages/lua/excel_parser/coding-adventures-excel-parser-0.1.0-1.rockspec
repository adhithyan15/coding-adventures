package = "coding-adventures-excel-parser"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Excel formula parser — hand-written recursive-descent parser producing an AST",
    detailed = [[
        A hand-written recursive-descent parser for Excel formula strings.
        Tokenizes the input using coding-adventures-excel-lexer, then
        constructs an Abstract Syntax Tree (AST) representing the formula.

        Supports the full Excel formula expression language:
          - Arithmetic operators: + - * / ^ & %
          - Comparison operators: = <> < <= > >=
          - Unary prefix: - +
          - Postfix: %
          - Cell references: A1, $B$2, A1:B10 (ranges)
          - Cross-sheet references: Sheet1!A1, 'My Sheet'!B2
          - Function calls with argument lists: SUM(A1:B10), IF(a>0,"y","n")
          - Array constants: {1,2;3,4}
          - Literal values: NUMBER, STRING, TRUE/FALSE, ERROR_CONSTANT

        Operator precedence (lowest to highest):
          comparison < concatenation(&) < addition < multiplication < power < unary < postfix

        Excel formulas are case-insensitive; input is normalized to lowercase
        by the underlying lexer.
    ]],
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
    "coding-adventures-excel-lexer >= 0.1.0",
    "coding-adventures-grammar-tools >= 0.1.0",
    "coding-adventures-lexer >= 0.1.0",
    "coding-adventures-directed-graph >= 0.1.0",
    "coding-adventures-state-machine >= 0.1.0",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.excel_parser"] = "src/coding_adventures/excel_parser/init.lua",
    },
}
