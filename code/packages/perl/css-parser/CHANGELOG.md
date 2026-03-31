# Changelog — CodingAdventures::CssParser (Perl)

## [0.01] — Initial release

### Added
- `parse($source)` — tokenizes and parses a CSS3 source string, returns root ASTNode
- `CodingAdventures::CssParser::ASTNode` submodule for AST nodes
- Hand-written recursive-descent parser (no grammar-driven GrammarParser in Perl layer)
- Full CSS3 grammar support:
  - `_parse_stylesheet` — entry point, { rule }
  - `_parse_rule` — at_rule | qualified_rule
  - `_parse_at_rule` — AT_KEYWORD prelude ( SEMICOLON | block )
  - `_parse_at_prelude` — flexible token sequence with paren-depth tracking
  - `_parse_qualified_rule` — selector_list block
  - `_parse_selector_list` — COMMA-separated complex selectors
  - `_parse_complex_selector` — compound selectors with combinators
  - `_parse_compound_selector` — chain of simple + subclass selectors
  - `_parse_simple_selector` — IDENT | STAR | AMPERSAND
  - `_parse_subclass_selector` — class, ID, attribute, pseudo-class, pseudo-element
  - `_parse_class_selector` — DOT IDENT
  - `_parse_id_selector` — HASH
  - `_parse_attribute_selector` — LBRACKET with optional matcher/value/flag
  - `_parse_pseudo_class` — COLON (FUNCTION args RPAREN | IDENT)
  - `_parse_pseudo_class_args` — flexible args with depth tracking
  - `_parse_pseudo_element` — COLON_COLON IDENT
  - `_parse_block` — LBRACE block_contents RBRACE
  - `_parse_block_contents` — { block_item }
  - `_parse_block_item` — at_rule | declaration_or_nested
  - `_parse_declaration_or_nested` — peek-ahead disambiguation
  - `_parse_declaration` — property COLON value_list [priority] SEMICOLON
  - `_parse_property` — IDENT | CUSTOM_PROPERTY
  - `_parse_priority` — BANG IDENT("important")
  - `_parse_value_list` — { value } until SEMICOLON/RBRACE/BANG/EOF
  - `_try_parse_value` — FUNCTION call | URL_TOKEN | simple value
  - `_parse_function_call` — FUNCTION function_args RPAREN
  - `_parse_function_args` — recursive function argument parser
- Declaration vs. nested rule disambiguation via 1-token peek-ahead
- Paren-depth tracking for at-rule preludes and pseudo-class args
- CSS nesting support (& selector, nested qualified rules in blocks)
- CSS custom properties as declaration properties and values
- `!important` priority annotation
- Comprehensive Test2::V0 test suite in `t/00-load.t` and `t/01-basic.t`
