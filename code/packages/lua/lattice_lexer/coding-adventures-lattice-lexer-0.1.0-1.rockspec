package = "coding-adventures-lattice-lexer"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Lattice lexer — tokenizes Lattice (CSS superset) source using the grammar-driven infrastructure",
    detailed = [[
        A thin wrapper around the grammar-driven GrammarLexer from the
        coding-adventures-lexer package.  Loads the shared lattice.tokens
        grammar file and delegates all tokenization to the GrammarLexer,
        producing a flat stream of typed tokens covering all CSS tokens plus
        Lattice extensions: VARIABLE ($color), PLACEHOLDER (%selector),
        comparison operators (==, !=, >=, <=), and bang tokens (!default,
        !global).
        Whitespace and comments (// and /* */) are consumed silently via
        lattice.tokens skip rules.  String values include raw escape sequences
        as per escapes: none mode (CSS escape decoding is a semantic concern).
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
        ["coding_adventures.lattice_lexer"] = "src/coding_adventures/lattice_lexer/init.lua",
    },
}
