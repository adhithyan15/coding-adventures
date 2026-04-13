defmodule CodingAdventures.CompilerSourceMap do
  @moduledoc """
  Elixir port of the source-mapping sidecar for the AOT compiler pipeline.

  The source map chain flows alongside the compiled program through every
  pipeline stage, recording which source positions produced which IR
  instructions, and which IR instructions produced which machine-code bytes.

  ## Why source maps?

  A flat table (machine-code offset → source position) serves the final
  consumer — a debugger, profiler, or error reporter. But it doesn't help
  when debugging the *compiler itself*:

  - "Why did the optimiser delete instruction #42?" → IrToIr segment
  - "Which AST node produced this IR instruction?" → AstToIr segment
  - "What IR produced this machine code?" → IrToMachineCode segment

  The chain makes the compiler pipeline **transparent and debuggable at
  every stage**.

  ## Segment overview

  ```
  Source text
      ↓ SourceToAst  (Segment 1)
  AST node IDs
      ↓ AstToIr  (Segment 2)
  IR instruction IDs
      ↓ IrToIr  (Segment 3, one per optimiser pass)
  Optimised IR instruction IDs
      ↓ IrToMachineCode  (Segment 4)
  Machine code byte offsets
  ```

  ## Modules

  - `CodingAdventures.CompilerSourceMap.SourcePosition` — a source span
  - `CodingAdventures.CompilerSourceMap.SourceToAst` — Segment 1
  - `CodingAdventures.CompilerSourceMap.AstToIr` — Segment 2
  - `CodingAdventures.CompilerSourceMap.IrToIr` — Segment 3
  - `CodingAdventures.CompilerSourceMap.IrToMachineCode` — Segment 4
  - `CodingAdventures.CompilerSourceMap.SourceMapChain` — all segments

  ## Usage

      alias CodingAdventures.CompilerSourceMap.{
        SourcePosition, SourceMapChain, SourceToAst, AstToIr, IrToMachineCode
      }

      # Create a chain
      chain = SourceMapChain.new()

      # Segment 1: record source → AST
      chain = %{chain |
        source_to_ast: SourceToAst.add(chain.source_to_ast,
          %SourcePosition{file: "hello.bf", line: 1, column: 1, length: 1}, 0)
      }

      # Segment 2: record AST → IR
      chain = %{chain |
        ast_to_ir: AstToIr.add(chain.ast_to_ir, 0, [7, 8, 9, 10])
      }

      # Segment 4: record IR → machine code
      mc = IrToMachineCode.new()
      mc = IrToMachineCode.add(mc, 7, 0x14, 4)
      chain = %{chain | ir_to_machine_code: mc}

      # Composite forward lookup
      results = SourceMapChain.source_to_mc(chain,
        %SourcePosition{file: "hello.bf", line: 1, column: 1, length: 1})

      # Composite reverse lookup
      source_pos = SourceMapChain.mc_to_source(chain, 0x14)
  """
end
