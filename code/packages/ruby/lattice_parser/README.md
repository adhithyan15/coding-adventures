# lattice-parser

Parser that turns Lattice source text into an AST. Uses `lattice_lexer` to
tokenize and `coding_adventures_parser`'s `GrammarDrivenParser` with the
`lattice.grammar` grammar file to produce a tree of `ASTNode` objects.

## Where It Fits

```
Lattice source text
        │
  lattice_lexer
        │ tokens
  lattice_parser          ← this package
        │ ASTNode tree
  lattice_ast_to_css
        │ CSS AST
  lattice_transpiler
        │ CSS text
```

## Usage

```ruby
require "coding_adventures_lattice_parser"

ast = CodingAdventures::LatticeParser.parse("$color: red; h1 { color: $color; }")
puts ast.rule_name  # => "stylesheet"
puts ast.children.size  # number of top-level rules
```

## AST Structure

The root node is always a `stylesheet` with `rule` children. Each `rule`
contains one of:

- `lattice_rule` — variable declaration, mixin/function definition, `@use`, or control flow
- `at_rule` — CSS at-rules (`@media`, `@import`, etc.)
- `qualified_rule` — CSS selector + block

Lattice-specific rule names: `variable_declaration`, `mixin_definition`,
`function_definition`, `use_directive`, `if_directive`, `for_directive`,
`each_directive`, `include_directive`, `return_directive`.

## Dependencies

- `coding_adventures_lattice_lexer` — tokenizer
- `coding_adventures_grammar_tools` — `.grammar` file parser
- `coding_adventures_parser` — `GrammarDrivenParser` engine

## Development

```bash
bundle exec rake test
```
