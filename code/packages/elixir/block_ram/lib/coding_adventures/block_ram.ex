defmodule CodingAdventures.BlockRam do
  @moduledoc """
  Block RAM — memory building blocks for digital systems.

  ## What is Block RAM?

  Block RAM (BRAM) is a dedicated chunk of memory embedded in an FPGA or
  ASIC. Unlike distributed RAM (which is built from LUT resources), Block
  RAM is a purpose-built memory macro that provides dense, fast storage.

  In an FPGA, Block RAM is one of the three fundamental resource types:
    1. Logic (LUTs and flip-flops) — for computation
    2. Routing (switch matrices) — for connecting logic
    3. Block RAM — for storage

  ## The Memory Hierarchy in This Module

  We build memory from the ground up, starting with the smallest unit:

      SRAMCell     → 1-bit storage element (the atom of memory)
        │
      SRAMArray    → M x N grid of SRAM cells (a raw memory array)
        │
      SinglePortRAM → memory with one read/write port
        │
      DualPortRAM   → memory with two independent ports
        │
      ConfigurableBRAM → FPGA-style BRAM with configurable width/depth

  ## Functional Style

  All modules in this package use a functional approach: state is represented
  as structs, and operations return `{result, new_state}` tuples. This makes
  the code easy to test and reason about, even though it models inherently
  stateful hardware.
  """
end
