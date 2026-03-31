# Changelog — coding-adventures-css-parser (Lua)

## [0.1.0] — Initial release

### Added
- `parse(source)` — tokenizes and parses a CSS3 source string, returns root ASTNode
- `create_parser(source)` — tokenizes source and returns an initialized GrammarParser
- `get_grammar()` — returns the cached ParserGrammar for inspection or reuse
- Grammar loaded from `code/grammars/css.grammar` (6-level path navigation)
- Grammar caching — css.grammar is read and parsed only once per process
- Grammar-driven parsing via GrammarParser from coding-adventures-parser
- Full CSS3 grammar support (from css.grammar):
  - Qualified rules: `selector_list block`
  - All selector types: type, class, ID, attribute, pseudo-class,
    pseudo-element, combinators (`>`, `+`, `~`), CSS nesting (`&`)
  - At-rules: @media, @import, @charset, @keyframes, @font-face, @supports, etc.
  - Declarations with all value types: DIMENSION, PERCENTAGE, NUMBER, STRING,
    IDENT, HASH, CUSTOM_PROPERTY, function calls, URL tokens
  - `!important` priority annotation
  - Nested rules inside blocks (CSS Nesting Module)
- The root ASTNode always has `rule_name == "stylesheet"`
- Comprehensive busted test suite in `tests/test_css_parser.lua`
