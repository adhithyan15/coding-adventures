defmodule CodingAdventures.Transistors.MOSFET do
  @moduledoc """
  MOSFET Transistors — the building blocks of modern digital circuits.

  ## What is a MOSFET?

  MOSFET stands for Metal-Oxide-Semiconductor Field-Effect Transistor. It is
  the most common type of transistor in the world — every CPU, GPU, and phone
  chip is built from billions of MOSFETs.

  A MOSFET has three terminals:
    - **Gate (G):**   The control terminal. Voltage here controls the switch.
    - **Drain (D):**  Current flows IN here (for NMOS) or OUT here (for PMOS).
    - **Source (S):** Current flows OUT here (for NMOS) or IN here (for PMOS).

  The key insight: a MOSFET is VOLTAGE-controlled. Applying a voltage to the
  gate creates an electric field that either allows or blocks current flow
  between drain and source. No current flows into the gate itself (it's
  insulated by a thin oxide layer).

  ## NMOS vs PMOS

      NMOS: Gate HIGH -> ON  (conducts drain to source)
      PMOS: Gate LOW  -> ON  (conducts source to drain)

  This complementary behavior is the foundation of CMOS (Complementary MOS)
  logic. By pairing NMOS and PMOS transistors, we can build gates that consume
  near-zero power in steady state.

  ## The Three Operating Regions

      1. CUTOFF:     Vgs < Vth -> transistor is OFF, no current flows.
      2. LINEAR:     Vgs > Vth, Vds < (Vgs - Vth) -> acts like a variable resistor.
      3. SATURATION: Vgs > Vth, Vds >= (Vgs - Vth) -> current roughly constant.
  """

  alias CodingAdventures.Transistors.Types.MOSFETParams

  # ===========================================================================
  # NMOS FUNCTIONS
  # ===========================================================================
  # An NMOS transistor conducts current from drain to source when the gate
  # voltage exceeds the threshold voltage (Vgs > Vth). Think of it as a
  # normally-OPEN switch that CLOSES when you apply voltage to the gate.

  @doc """
  Determine the operating region of an NMOS transistor.

  The operating region determines which equations govern current flow:
    - `:cutoff`     — Vgs < Vth (gate voltage below threshold)
    - `:linear`     — Vgs >= Vth AND Vds < Vgs - Vth
    - `:saturation` — Vgs >= Vth AND Vds >= Vgs - Vth

  ## Examples

      iex> MOSFET.nmos_region(0.0, 1.0)
      :cutoff

      iex> MOSFET.nmos_region(1.5, 0.1)
      :linear

      iex> MOSFET.nmos_region(1.0, 3.0)
      :saturation
  """
  def nmos_region(vgs, vds, params \\ %MOSFETParams{}) do
    vth = params.vth

    if vgs < vth do
      :cutoff
    else
      vov = vgs - vth

      if vds < vov do
        :linear
      else
        :saturation
      end
    end
  end

  @doc """
  Calculate NMOS drain-to-source current (Ids) in amperes.

  Uses the simplified MOSFET current equations (Shockley model):

    - **Cutoff:** Ids = 0. No channel, no current.
    - **Linear:** Ids = k * ((Vgs - Vth) * Vds - 0.5 * Vds^2). Voltage-controlled resistor.
    - **Saturation:** Ids = 0.5 * k * (Vgs - Vth)^2. Current depends only on Vgs.
  """
  def nmos_drain_current(vgs, vds, params \\ %MOSFETParams{}) do
    region = nmos_region(vgs, vds, params)
    k = params.k
    vth = params.vth

    case region do
      :cutoff ->
        0.0

      :linear ->
        vov = vgs - vth
        k * (vov * vds - 0.5 * vds * vds)

      :saturation ->
        vov = vgs - vth
        0.5 * k * vov * vov
    end
  end

  @doc """
  Digital abstraction: is this NMOS transistor ON?

  Returns `true` when the gate voltage exceeds the threshold voltage (Vgs >= Vth).
  This is the simplified view used in digital circuit analysis.
  """
  def nmos_is_conducting?(vgs, params \\ %MOSFETParams{}) do
    vgs >= params.vth
  end

  @doc """
  Output voltage when NMOS is used as a pull-down switch.

  In a CMOS circuit, NMOS transistors form the pull-down network:
    - ON:  output ~ 0V (pulled to ground)
    - OFF: output ~ Vdd (pulled up by PMOS network)
  """
  def nmos_output_voltage(vgs, vdd, params \\ %MOSFETParams{}) do
    if nmos_is_conducting?(vgs, params) do
      0.0
    else
      vdd
    end
  end

  @doc """
  Calculate small-signal transconductance gm for NMOS.

  gm = dIds / dVgs = k * (Vgs - Vth) in saturation/linear.
  Returns 0.0 in cutoff. Higher gm = more gain but more power.
  """
  def nmos_transconductance(vgs, vds, params \\ %MOSFETParams{}) do
    region = nmos_region(vgs, vds, params)

    if region == :cutoff do
      0.0
    else
      vov = vgs - params.vth
      params.k * vov
    end
  end

  # ===========================================================================
  # PMOS FUNCTIONS
  # ===========================================================================
  # A PMOS transistor is the complement of NMOS. It conducts current from
  # source to drain when the gate voltage is LOW (below the source voltage
  # by more than |Vth|). PMOS transistors form the pull-UP network in CMOS gates.
  #
  # PMOS uses the same equations as NMOS but with reversed voltage polarities.
  # For PMOS, Vgs and Vds are typically negative.

  @doc """
  Determine the operating region of a PMOS transistor.

  Uses absolute values of Vgs and Vds since PMOS operates with
  reversed polarities:
    - `:cutoff`     — |Vgs| < Vth
    - `:linear`     — |Vgs| >= Vth AND |Vds| < |Vgs| - Vth
    - `:saturation` — |Vgs| >= Vth AND |Vds| >= |Vgs| - Vth
  """
  def pmos_region(vgs, vds, params \\ %MOSFETParams{}) do
    vth = params.vth
    abs_vgs = abs(vgs)
    abs_vds = abs(vds)

    if abs_vgs < vth do
      :cutoff
    else
      vov = abs_vgs - vth

      if abs_vds < vov do
        :linear
      else
        :saturation
      end
    end
  end

  @doc """
  Calculate PMOS source-to-drain current (magnitude).

  Same equations as NMOS but using absolute values of voltages.
  Current magnitude is returned (always >= 0).
  """
  def pmos_drain_current(vgs, vds, params \\ %MOSFETParams{}) do
    region = pmos_region(vgs, vds, params)
    k = params.k
    vth = params.vth

    case region do
      :cutoff ->
        0.0

      :linear ->
        abs_vgs = abs(vgs)
        abs_vds = abs(vds)
        vov = abs_vgs - vth
        k * (vov * abs_vds - 0.5 * abs_vds * abs_vds)

      :saturation ->
        abs_vgs = abs(vgs)
        vov = abs_vgs - vth
        0.5 * k * vov * vov
    end
  end

  @doc """
  Digital abstraction: is this PMOS transistor ON?

  PMOS turns ON when Vgs is sufficiently negative: |Vgs| >= Vth.
  """
  def pmos_is_conducting?(vgs, params \\ %MOSFETParams{}) do
    abs(vgs) >= params.vth
  end

  @doc """
  Output voltage when PMOS is used as a pull-up switch.

  PMOS forms the pull-up network in CMOS:
    - ON:  output ~ Vdd
    - OFF: output ~ 0V (pulled down by NMOS network)
  """
  def pmos_output_voltage(vgs, vdd, params \\ %MOSFETParams{}) do
    if pmos_is_conducting?(vgs, params) do
      vdd
    else
      0.0
    end
  end

  @doc """
  Calculate small-signal transconductance gm for PMOS.

  Same formula as NMOS but using absolute values.
  """
  def pmos_transconductance(vgs, vds, params \\ %MOSFETParams{}) do
    region = pmos_region(vgs, vds, params)

    if region == :cutoff do
      0.0
    else
      vov = abs(vgs) - params.vth
      params.k * vov
    end
  end
end
