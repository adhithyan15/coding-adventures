defmodule CodingAdventures.FPGA do
  @moduledoc """
  FPGA — Field-Programmable Gate Array simulation in Elixir.

  ## What is an FPGA?

  An FPGA is a chip that can be programmed to implement any digital circuit
  AFTER manufacturing. Unlike a CPU (which executes instructions sequentially)
  or an ASIC (which is hardwired at the factory), an FPGA contains a grid
  of reconfigurable logic blocks connected by a programmable routing network.

  Think of it as a blank circuit board that you can rewire with software.

  ## FPGA Architecture

  A typical FPGA contains these components, all modeled in this package:

      ┌─────────────────────────────────────────────┐
      │  IO  IO  IO  IO  IO  IO  IO  IO  IO  IO     │
      │  ┌─────┐  ┌─────┐  ┌─────┐  ┌─────┐       │
      │  │ CLB │──│ SW  │──│ CLB │──│ SW  │        │
      │  └─────┘  └─────┘  └─────┘  └─────┘       │
      │     │        │        │        │            │
      │  ┌─────┐  ┌─────┐  ┌─────┐  ┌─────┐       │
      │  │ SW  │──│BRAM │──│ SW  │──│BRAM │        │
      │  └─────┘  └─────┘  └─────┘  └─────┘       │
      │     │        │        │        │            │
      │  ┌─────┐  ┌─────┐  ┌─────┐  ┌─────┐       │
      │  │ CLB │──│ SW  │──│ CLB │──│ SW  │        │
      │  └─────┘  └─────┘  └─────┘  └─────┘       │
      │  IO  IO  IO  IO  IO  IO  IO  IO  IO  IO     │
      └─────────────────────────────────────────────┘

  Where:
    - CLB = Configurable Logic Block (contains LUTs and flip-flops)
    - SW  = Switch Matrix (programmable routing)
    - BRAM = Block RAM (embedded memory)
    - IO  = I/O Block (interface to external pins)

  ## Module Hierarchy

      LUT           → Lookup Table (truth table in SRAM)
        │
      Slice         → 2 LUTs + 2 Flip-Flops + Carry Chain
        │
      CLB           → 2 Slices (Configurable Logic Block)
        │
      SwitchMatrix  → Programmable routing crossbar
        │
      IOBlock       → Input/Output interface
        │
      Bitstream     → Configuration data (from maps)
        │
      Fabric        → Complete FPGA with all components
  """
end
