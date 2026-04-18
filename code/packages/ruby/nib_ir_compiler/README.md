# `coding_adventures_nib_ir_compiler`

`coding_adventures_nib_ir_compiler` lowers the typed AST produced by
`coding_adventures_nib_type_checker` into the generic register-based IR from
`coding_adventures_compiler_ir`.

This package is the Nib-specific frontend layer. It knows how to translate the
convergence-wave Nib subset into:

- `_start`
- `_fn_NAME`
- `LOAD_IMM`
- `ADD`
- `ADD_IMM`
- `SUB`
- `CALL`
- `BRANCH_Z`
- `JUMP`
- `RET`
- `HALT`

## Example

```ruby
require "coding_adventures_nib_ir_compiler"
require "coding_adventures_nib_type_checker"
require "coding_adventures_nib_parser"

ast = CodingAdventures::NibParser.parse_nib("fn main() -> u4 { return 7; }")
typed = CodingAdventures::NibTypeChecker.check(ast)
compiled = CodingAdventures::NibIrCompiler.compile_nib(typed.typed_ast)
puts compiled.program.instructions.length
```
