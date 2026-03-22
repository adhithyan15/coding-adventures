defmodule CodingAdventures.LogicGates do
  @moduledoc """
  Logic Gates — the foundation of all digital computing.

  ## What is a logic gate?

  A logic gate is the simplest possible decision-making element. It takes
  one or two inputs, each either 0 or 1, and produces a single output
  that is also 0 or 1. The output is entirely determined by the inputs —
  there is no randomness, no hidden state, no memory.

  In physical hardware, gates are built from transistors — tiny electronic
  switches etched into silicon. A modern CPU contains billions of transistors
  organized into billions of gates. But conceptually, every computation a
  computer performs — from adding numbers to rendering video to running AI
  models — ultimately reduces to combinations of these simple 0-or-1 operations.

  This module re-exports all gate functions from the Gates and Sequential
  sub-modules for convenience.

  ## Why only 0 and 1?

  Computers use binary (base-2) because transistors are most reliable as
  on/off switches. A transistor that is "on" (conducting electricity)
  represents 1. A transistor that is "off" (blocking electricity) represents 0.
  You could theoretically build a computer using base-3 or base-10, but the
  error margins for distinguishing between voltage levels would make it
  unreliable. Binary gives us two clean, easily distinguishable states.
  """

  # Re-export all gate functions for convenience.
  defdelegate not_gate(a), to: CodingAdventures.LogicGates.Gates
  defdelegate and_gate(a, b), to: CodingAdventures.LogicGates.Gates
  defdelegate or_gate(a, b), to: CodingAdventures.LogicGates.Gates
  defdelegate xor_gate(a, b), to: CodingAdventures.LogicGates.Gates
  defdelegate nand_gate(a, b), to: CodingAdventures.LogicGates.Gates
  defdelegate nor_gate(a, b), to: CodingAdventures.LogicGates.Gates
  defdelegate xnor_gate(a, b), to: CodingAdventures.LogicGates.Gates

  defdelegate nand_not(a), to: CodingAdventures.LogicGates.Gates
  defdelegate nand_and(a, b), to: CodingAdventures.LogicGates.Gates
  defdelegate nand_or(a, b), to: CodingAdventures.LogicGates.Gates
  defdelegate nand_xor(a, b), to: CodingAdventures.LogicGates.Gates

  defdelegate and_n(inputs), to: CodingAdventures.LogicGates.Gates
  defdelegate or_n(inputs), to: CodingAdventures.LogicGates.Gates

  defdelegate sr_latch(set_, reset, q, q_bar), to: CodingAdventures.LogicGates.Sequential
  defdelegate d_latch(data, enable, q, q_bar), to: CodingAdventures.LogicGates.Sequential
  defdelegate d_flip_flop(data, clock, state), to: CodingAdventures.LogicGates.Sequential
  defdelegate register(data, clock, state), to: CodingAdventures.LogicGates.Sequential
  defdelegate shift_register(serial_in, clock, state, opts \\ []), to: CodingAdventures.LogicGates.Sequential
  defdelegate counter(clock, reset, state), to: CodingAdventures.LogicGates.Sequential
end
