package = "coding-adventures-css-lexer"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "CSS lexer — tokenizes CSS3 source using the grammar-driven infrastructure",
    detailed = [[
        A thin wrapper around the grammar-driven GrammarLexer from the
        coding-adventures-lexer package.  Loads the shared css.tokens
        grammar file and delegates all tokenization to the GrammarLexer,
        producing a flat stream of typed tokens.

        CSS tokenization is harder than most languages due to compound tokens:
        10px is one DIMENSION token (not NUMBER + IDENT), 50% is one PERCENTAGE
        token, rgba( is one FUNCTION token, and url(./img) is one URL_TOKEN.

        The grammar uses first-match-wins priority ordering to handle this:
        DIMENSION before PERCENTAGE before NUMBER, URL_TOKEN before FUNCTION,
        FUNCTION before IDENT, COLON_COLON before COLON, etc.

        escapes: none mode is used because CSS hex escapes (\26, \000A9)
        differ from JSON-style \uXXXX and are handled post-parse.
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
        ["coding_adventures.css_lexer"] = "src/coding_adventures/css_lexer/init.lua",
    },
}
