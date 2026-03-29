package = "coding-adventures-css-parser"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "CSS parser — builds ASTs from CSS3 stylesheets",
    detailed = [[
        A grammar-driven CSS3 parser.  Tokenizes source text using
        coding-adventures-css-lexer and constructs an Abstract Syntax Tree
        using the grammar-driven GrammarParser (from coding-adventures-parser)
        driven by the css.grammar rule definitions.

        Supports the full CSS3 grammar subset:
          - Qualified rules (selectors + declaration blocks)
          - All selector types: type, class, ID, attribute, pseudo-class,
            pseudo-element, combinators, CSS nesting (&)
          - At-rules: @media, @import, @charset, @keyframes, @font-face, etc.
          - Declarations with diverse value types: DIMENSION, PERCENTAGE,
            NUMBER, STRING, IDENT, HASH, function calls, URL tokens
          - !important priority annotations
          - CSS custom properties (--variable-name)
          - Nested rules (CSS Nesting Module)

        The root ASTNode has rule_name == "stylesheet".
    ]],
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
    "coding-adventures-css-lexer >= 0.1.0",
    "coding-adventures-parser >= 0.1.0",
    "coding-adventures-grammar-tools >= 0.1.0",
    "coding-adventures-lexer >= 0.1.0",
    "coding-adventures-directed-graph >= 0.1.0",
    "coding-adventures-state-machine >= 0.1.0",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.css_parser"] = "src/coding_adventures/css_parser/init.lua",
    },
}
