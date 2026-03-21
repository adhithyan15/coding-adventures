defmodule CodingAdventures.CpuPipeline do
  @moduledoc """
  A configurable N-stage CPU instruction pipeline simulator.

  ## The Pipeline: a CPU's Assembly Line

  A CPU pipeline is the central execution engine of a processor core. Instead
  of completing one instruction fully before starting the next (like a
  single-cycle CPU), a pipelined CPU overlaps instruction execution -- while
  one instruction is being executed, the next is being decoded, and the one
  after that is being fetched.

  This is the same principle as a factory assembly line:

      Single-cycle (no pipeline):
      Instr 1: [IF][ID][EX][MEM][WB]
      Instr 2:                       [IF][ID][EX][MEM][WB]
      Throughput: 1 instruction every 5 cycles

      Pipelined:
      Instr 1: [IF][ID][EX][MEM][WB]
      Instr 2:     [IF][ID][EX][MEM][WB]
      Instr 3:         [IF][ID][EX][MEM][WB]
      Throughput: 1 instruction every 1 cycle (after filling)

  ## What This Package Does

  This package manages the FLOW of instructions through pipeline stages. It
  does NOT interpret instructions -- that is the ISA decoder's job. The
  pipeline moves "tokens" (representing instructions) through stages, handling:

    - Normal advancement: tokens move one stage per clock cycle
    - Stalls: freeze earlier stages and insert a "bubble" (NOP)
    - Flushes: replace speculative instructions with bubbles
    - Statistics: track IPC, stall cycles, flush cycles

  The actual work of each stage (fetching, decoding, executing, etc.) is
  performed by callback functions injected from the CPU core. This makes the
  pipeline ISA-independent -- the same pipeline can run ARM, RISC-V, x86, or
  any other instruction set.

  ## The Classic 5-Stage Pipeline

      Stage 1: IF  (Instruction Fetch)  -- read instruction from memory at PC
      Stage 2: ID  (Instruction Decode) -- decode opcode, read registers
      Stage 3: EX  (Execute)            -- ALU operation, branch resolution
      Stage 4: MEM (Memory Access)      -- load/store data from/to memory
      Stage 5: WB  (Write Back)         -- write result to register file

  ## Dependency Injection

  The pipeline uses callback functions (lambdas) instead of importing other
  packages directly. This keeps the pipeline decoupled from specific
  implementations of caches, hazard detectors, and branch predictors.
  """

  alias CodingAdventures.CpuPipeline.{Pipeline, Token}

  defdelegate new_pipeline(config, fetch, decode, execute, memory, writeback), to: Pipeline, as: :new
  defdelegate step(pipeline), to: Pipeline
  defdelegate run(pipeline, max_cycles), to: Pipeline
  defdelegate set_hazard_func(pipeline, func), to: Pipeline
  defdelegate set_predict_func(pipeline, func), to: Pipeline
  defdelegate set_pc(pipeline, pc), to: Pipeline

  defdelegate new_token(), to: Token, as: :new
  defdelegate new_bubble(), to: Token, as: :new_bubble

  defdelegate classic_5_stage(), to: Pipeline
  defdelegate deep_13_stage(), to: Pipeline
end
