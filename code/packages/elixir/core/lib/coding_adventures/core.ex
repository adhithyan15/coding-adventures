defmodule CodingAdventures.Core do
  @moduledoc """
  Integrates all D-series micro-architectural components into a complete
  processor core.

  ## The Core: a Motherboard for Micro-Architecture

  A processor core is not a single piece of hardware. It is a composition of
  many sub-components, each independently designed and tested:

    - Pipeline (D04): moves instructions through stages (IF, ID, EX, MEM, WB)
    - Register File: fast storage for operands and results
    - Memory Controller: access to backing memory
    - Interrupt Controller: routes interrupts to cores in multi-core systems

  The Core itself defines no new micro-architectural behavior. It wires the
  parts together, like a motherboard connects CPU, RAM, and peripherals.
  The same Core can run ARM, RISC-V, or any custom ISA -- the ISA decoder
  is injected from outside via a behaviour.

  ## Configuration

  Every parameter that a real CPU architect would tune is exposed in
  CoreConfig. Change the pipeline depth, register count, memory size --
  all are configurable.

  ## Multi-Core

  MultiCoreCPU connects multiple cores to shared memory and an interrupt
  controller -- modeling a modern multi-core chip.

  ## Functional Design

  Since Elixir is functional, the Core takes state as input and returns new
  state. There is no mutation. Each call to `step/1` returns an updated Core
  struct along with a pipeline snapshot.
  """

  alias CodingAdventures.Core.{
    Config,
    Decoder,
    RegisterFile,
    MemoryController,
    InterruptController,
    MultiCore,
    Stats
  }
end
