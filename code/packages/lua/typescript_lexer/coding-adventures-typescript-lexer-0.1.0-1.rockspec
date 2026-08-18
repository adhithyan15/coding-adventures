package = "coding-adventures-typescript-lexer"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "TypeScript lexer — tokenizes TypeScript source using the grammar-driven infrastructure",
    detailed = [[
        A thin wrapper around the grammar-driven GrammarLexer from the
        coding-adventures-lexer package.  Loads the shared typescript.tokens
        grammar file and delegates all tokenization to the GrammarLexer.
        TypeScript is a strict superset of JavaScript; this lexer recognizes
        all JavaScript tokens plus TypeScript-specific keywords (interface,
        type, enum, namespace, declare, readonly, public, private, protected,
        abstract, implements, extends, keyof, infer, never, unknown, any,
        void, boolean, object, symbol, bigint).
        Whitespace is consumed silently via typescript.tokens skip rules.
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
        ["coding_adventures.typescript_lexer"] = "src/coding_adventures/typescript_lexer/init.lua",
        ["coding_adventures.typescript_lexer._grammar_default"] = "src/coding_adventures/typescript_lexer/_grammar_default.lua",
        ["coding_adventures.typescript_lexer._grammar_ts1_0"] = "src/coding_adventures/typescript_lexer/_grammar_ts1_0.lua",
        ["coding_adventures.typescript_lexer._grammar_ts2_0"] = "src/coding_adventures/typescript_lexer/_grammar_ts2_0.lua",
        ["coding_adventures.typescript_lexer._grammar_ts3_0"] = "src/coding_adventures/typescript_lexer/_grammar_ts3_0.lua",
        ["coding_adventures.typescript_lexer._grammar_ts4_0"] = "src/coding_adventures/typescript_lexer/_grammar_ts4_0.lua",
        ["coding_adventures.typescript_lexer._grammar_ts5_0"] = "src/coding_adventures/typescript_lexer/_grammar_ts5_0.lua",
        ["coding_adventures.typescript_lexer._grammar_ts5_8"] = "src/coding_adventures/typescript_lexer/_grammar_ts5_8.lua",
    },
}
