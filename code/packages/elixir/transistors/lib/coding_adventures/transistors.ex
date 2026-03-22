defmodule CodingAdventures.Transistors do
  @moduledoc """
  Transistors — the electronic switches beneath logic gates.

  This package models transistors at the electrical level, showing how
  logic gates are physically constructed from MOSFET and BJT transistors.

  ## Package Organization

      Transistors.MOSFET      — NMOS and PMOS transistor functions
      Transistors.BJT         — NPN and PNP transistor functions
      Transistors.CMOSGates   — CMOS logic gates built from MOSFET pairs
      Transistors.TTLGates    — TTL logic gates built from BJTs (historical)
      Transistors.Amplifier   — Analog amplifier analysis
      Transistors.Analysis    — Noise margins, power, timing, technology comparison
      Transistors.Types       — Structs for parameters and results

  ## Design Philosophy

  Unlike the Python version which uses classes with state, this Elixir
  implementation uses pure functions with parameter structs passed in.
  This is idiomatic Elixir — functions are stateless, data flows through
  them, and structs carry configuration.
  """

  # Delegate MOSFET functions
  defdelegate nmos_region(vgs, vds, params \\ %CodingAdventures.Transistors.Types.MOSFETParams{}),
    to: CodingAdventures.Transistors.MOSFET

  defdelegate nmos_drain_current(vgs, vds, params \\ %CodingAdventures.Transistors.Types.MOSFETParams{}),
    to: CodingAdventures.Transistors.MOSFET

  defdelegate nmos_is_conducting?(vgs, params \\ %CodingAdventures.Transistors.Types.MOSFETParams{}),
    to: CodingAdventures.Transistors.MOSFET

  defdelegate pmos_region(vgs, vds, params \\ %CodingAdventures.Transistors.Types.MOSFETParams{}),
    to: CodingAdventures.Transistors.MOSFET

  defdelegate pmos_drain_current(vgs, vds, params \\ %CodingAdventures.Transistors.Types.MOSFETParams{}),
    to: CodingAdventures.Transistors.MOSFET

  defdelegate pmos_is_conducting?(vgs, params \\ %CodingAdventures.Transistors.Types.MOSFETParams{}),
    to: CodingAdventures.Transistors.MOSFET

  # Delegate BJT functions
  defdelegate npn_region(vbe, vce, params \\ %CodingAdventures.Transistors.Types.BJTParams{}),
    to: CodingAdventures.Transistors.BJT

  defdelegate npn_collector_current(vbe, vce, params \\ %CodingAdventures.Transistors.Types.BJTParams{}),
    to: CodingAdventures.Transistors.BJT

  defdelegate npn_is_conducting?(vbe, params \\ %CodingAdventures.Transistors.Types.BJTParams{}),
    to: CodingAdventures.Transistors.BJT

  defdelegate pnp_region(vbe, vce, params \\ %CodingAdventures.Transistors.Types.BJTParams{}),
    to: CodingAdventures.Transistors.BJT

  defdelegate pnp_collector_current(vbe, vce, params \\ %CodingAdventures.Transistors.Types.BJTParams{}),
    to: CodingAdventures.Transistors.BJT

  defdelegate pnp_is_conducting?(vbe, params \\ %CodingAdventures.Transistors.Types.BJTParams{}),
    to: CodingAdventures.Transistors.BJT
end
