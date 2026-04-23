# `coding_adventures_nib_type_checker`

`coding_adventures_nib_type_checker` is the Ruby semantic-analysis stage for
Nib. It takes the generic grammar-driven AST from `coding_adventures_nib_parser`
and returns a typed wrapper that later stages can use for IR lowering.

This convergence-wave package intentionally focuses on the Nib subset already
used by the repo's WASM smoke tests:

- functions and parameters
- `let`
- assignment
- `return`
- `for`
- function calls
- integer and hex literals
- additive expressions

## Position In The Pipeline

```text
Nib source
  -> nib_parser
  -> nib_type_checker
  -> nib_ir_compiler
  -> nib_wasm_compiler
```

## Public API

```ruby
require "coding_adventures_nib_type_checker"
require "coding_adventures_nib_parser"

ast = CodingAdventures::NibParser.parse_nib("fn main() -> u4 { return 7; }")
result = CodingAdventures::NibTypeChecker.check(ast)

raise result.errors.first.message unless result.ok
typed_ast = result.typed_ast
```

`typed_ast` is a lightweight wrapper with:

- `root`: the original parser AST
- `types`: a map keyed by `ASTNode#object_id`
- `type_of(node)`: convenience lookup for later passes
