-- Rockspec for coding-adventures-dartmouth-basic-lexer
-- =====================================================
--
-- This rockspec declares the Dartmouth BASIC 1964 lexer as a publishable
-- LuaRocks package. It lives in the coding-adventures monorepo at:
--   code/packages/lua/dartmouth_basic_lexer/
--
-- The package depends on:
--   - coding-adventures-grammar-tools — parses the .tokens specification
--   - coding-adventures-lexer         — provides the GrammarLexer engine
--   - coding-adventures-directed-graph — required internally by grammar_tools
--   - coding-adventures-state-machine  — required internally by lexer / grammar_tools

package = "coding-adventures-dartmouth-basic-lexer"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Dartmouth BASIC 1964 lexer — tokenizes original BASIC source text using the grammar-driven infrastructure",
    detailed = [[
        A thin wrapper around the grammar-driven GrammarLexer from the
        coding-adventures-lexer package.  Loads the shared dartmouth_basic.tokens
        grammar file and delegates all tokenization to the GrammarLexer,
        producing a flat stream of typed tokens.

        Post-tokenize processing (applied manually, since the Lua GrammarLexer
        has no hook API):

          1. relabel_line_numbers — promotes the first NUMBER token on each
             source line to LINE_NUM, implementing the positional rule that
             distinguishes line labels (10, 20, ...) from numeric literals.

          2. suppress_rem_content — removes all tokens between a REM keyword
             and the end of the source line, implementing BASIC's comment syntax.

        Token types produced: LINE_NUM, NUMBER, STRING, KEYWORD, BUILTIN_FN,
        USER_FN, NAME, LE, GE, NE, PLUS, MINUS, STAR, SLASH, CARET, EQ,
        LT, GT, LPAREN, RPAREN, COMMA, SEMICOLON, NEWLINE, UNKNOWN, EOF.

        Whitespace (spaces and tabs) is consumed silently. NEWLINE is kept
        because BASIC is line-oriented — newlines terminate statements.

        The grammar uses @case_insensitive true so print, Print, and PRINT
        all produce the same KEYWORD("PRINT") token.
    ]],
    homepage = "https://github.com/adhithyan15/coding-adventures",
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
        ["coding_adventures.dartmouth_basic_lexer"] = "src/coding_adventures/dartmouth_basic_lexer/init.lua",
    },
}
