# coding_adventures_compiler_pipeline

Compiler pipeline orchestrator that chains lexer, parser, compiler, and VM into a single execution flow, capturing traces at every stage for visualization.

## What It Does

The Pipeline class wires the full computing stack together:

```
Source code -> Lexer -> Parser -> Compiler -> VM
```

At each stage, it captures the output into a dedicated data structure so the HTML visualizer can show exactly what happened.

## How It Fits in the Stack

This is the integration layer (Layer 0) that ties all other layers together. It depends on the lexer, parser, bytecode compiler, and virtual machine gems.

## Usage

```ruby
require "coding_adventures_compiler_pipeline"

pipeline = CodingAdventures::CompilerPipeline::Orchestrator.new
result = pipeline.run("x = 1 + 2")

# Inspect each stage:
puts result.lexer_stage.token_count       # Number of tokens
puts result.parser_stage.ast_dict         # JSON-serializable AST
puts result.compiler_stage.instructions_text  # Human-readable bytecode
puts result.vm_stage.final_variables      # {"x" => 3}
```

## Dependencies

- `coding_adventures_lexer` -- Tokenizes source code
- `coding_adventures_parser` -- Parses tokens into AST
- `coding_adventures_bytecode_compiler` -- Compiles AST to bytecode
- `coding_adventures_virtual_machine` -- Executes bytecode
