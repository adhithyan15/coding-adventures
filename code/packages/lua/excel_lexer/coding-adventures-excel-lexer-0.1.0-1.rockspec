package = "coding-adventures-excel-lexer"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Excel formula lexer — tokenizes Excel formula text using the grammar-driven infrastructure",
    detailed = [[
        A thin wrapper around the grammar-driven GrammarLexer from the
        coding-adventures-lexer package.  Loads the shared excel.tokens
        grammar file and delegates all tokenization to the GrammarLexer,
        producing a flat stream of typed tokens (EQUALS, CELL, NAME, NUMBER,
        STRING, TRUE, FALSE, ERROR_CONSTANT, REF_PREFIX, STRUCTURED_KEYWORD,
        STRUCTURED_COLUMN, operator tokens, SPACE, EOF).

        Excel formulas are case-insensitive (historical reasons: early IBM PC
        users were not programmers; =SUM(a1:b10) and =SUM(A1:B10) are
        identical).  This lexer normalizes input to lowercase before
        tokenizing so that all returned token values are lowercase.

        Unlike JSON, Excel's space character is the range-intersection
        operator and is therefore preserved as a SPACE token rather than
        silently consumed.  Only tabs, carriage returns, and newlines are
        silently skipped.
    ]],
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
    "coding-adventures-grammar-tools >= 0.1.0",
    "coding-adventures-lexer >= 0.1.0",
    "coding-adventures-directed-graph >= 0.1.0",
    "coding-adventures-state-machine >= 0.1.0",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.excel_lexer"] = "src/coding_adventures/excel_lexer/init.lua",
    },
}
