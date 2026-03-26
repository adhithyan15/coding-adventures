defmodule CodingAdventures.Transistors.BJT do
  @moduledoc """
  BJT Transistors — the original solid-state amplifier.

  ## What is a BJT?

  BJT stands for Bipolar Junction Transistor. Invented in 1947 at Bell Labs
  by Bardeen, Brattain, and Shockley, the BJT replaced vacuum tubes and
  launched the electronics revolution.

  A BJT has three terminals:
    - **Base (B):**      The control terminal. Current here controls the switch.
    - **Collector (C):** Current flows IN here (for NPN) or OUT here (for PNP).
    - **Emitter (E):**   Current flows OUT here (for NPN) or IN here (for PNP).

  The key difference from MOSFETs: a BJT is CURRENT-controlled. You must
  supply a continuous current to the base to keep it on.

  ## The Current Gain (beta)

      Ic = beta * Ib

  A tiny base current (microamps) controls a much larger collector current
  (milliamps). Beta is typically 50-300 for small-signal transistors.

  ## Why CMOS Replaced BJT for Digital Logic

  In TTL: ~1-10 mW per gate static power (base current always flows).
  In CMOS: ~nanowatts per gate static power (no DC current path).
  """

  alias CodingAdventures.Transistors.Types.BJTParams

  # ===========================================================================
  # NPN FUNCTIONS
  # ===========================================================================
  # An NPN transistor turns ON when current flows into the base terminal
  # (Vbe > ~0.7V). A small base current controls a much larger collector
  # current: Ic = beta * Ib.

  @doc """
  Determine the operating region of an NPN transistor.

    - `:cutoff`     — Vbe < Vbe_on (no base current, switch OFF)
    - `:active`     — Vbe >= Vbe_on AND Vce > Vce_sat (linear amplifier)
    - `:saturation` — Vbe >= Vbe_on AND Vce <= Vce_sat (fully ON switch)
  """
  def npn_region(vbe, vce, params \\ %BJTParams{}) do
    if vbe < params.vbe_on do
      :cutoff
    else
      if vce <= params.vce_sat do
        :saturation
      else
        :active
      end
    end
  end

  @doc """
  Calculate NPN collector current (Ic) in amperes.

  Uses the simplified Ebers-Moll model:
    - **Cutoff:** Ic = 0
    - **Active:** Ic = Is * (exp(Vbe/Vt) - 1), where Vt ~ 26mV at room temp
    - **Saturation:** Same equation (transistor at edge of saturation)

  The exponent is clamped to 40.0 to prevent floating-point overflow.
  """
  def npn_collector_current(vbe, vce, params \\ %BJTParams{}) do
    region = npn_region(vbe, vce, params)

    case region do
      :cutoff ->
        0.0

      _ ->
        # Thermal voltage: Vt = kT/q ~ 26mV at room temperature
        vt = 0.026
        exponent = min(vbe / vt, 40.0)
        params.i_s * (:math.exp(exponent) - 1.0)
    end
  end

  @doc """
  Calculate NPN base current (Ib) in amperes.

  Ib = Ic / beta. This is the "wasted" current that makes BJTs less
  efficient than MOSFETs for digital logic.
  """
  def npn_base_current(vbe, vce, params \\ %BJTParams{}) do
    ic = npn_collector_current(vbe, vce, params)

    if ic == 0.0 do
      0.0
    else
      ic / params.beta
    end
  end

  @doc """
  Digital abstraction: is this NPN transistor ON?

  Returns `true` when Vbe >= Vbe_on (typically 0.7V).
  """
  def npn_is_conducting?(vbe, params \\ %BJTParams{}) do
    vbe >= params.vbe_on
  end

  @doc """
  Calculate small-signal transconductance gm for NPN.

  For a BJT in the active region: gm = Ic / Vt.
  BJTs typically have higher gm than MOSFETs for the same current.
  """
  def npn_transconductance(vbe, vce, params \\ %BJTParams{}) do
    ic = npn_collector_current(vbe, vce, params)

    if ic == 0.0 do
      0.0
    else
      vt = 0.026
      ic / vt
    end
  end

  # ===========================================================================
  # PNP FUNCTIONS
  # ===========================================================================
  # The complement of NPN. A PNP transistor turns ON when the base is
  # pulled LOW relative to the emitter (|Vbe| > 0.7V).
  # We use absolute values internally, same as PMOS.

  @doc """
  Determine operating region for PNP.

  Uses absolute values of Vbe and Vce since PNP operates with
  reversed polarities.
  """
  def pnp_region(vbe, vce, params \\ %BJTParams{}) do
    abs_vbe = abs(vbe)
    abs_vce = abs(vce)

    if abs_vbe < params.vbe_on do
      :cutoff
    else
      if abs_vce <= params.vce_sat do
        :saturation
      else
        :active
      end
    end
  end

  @doc """
  Calculate PNP collector current magnitude.

  Same equations as NPN but using absolute values.
  Returns current magnitude (always >= 0).
  """
  def pnp_collector_current(vbe, vce, params \\ %BJTParams{}) do
    region = pnp_region(vbe, vce, params)

    case region do
      :cutoff ->
        0.0

      _ ->
        abs_vbe = abs(vbe)
        vt = 0.026
        exponent = min(abs_vbe / vt, 40.0)
        params.i_s * (:math.exp(exponent) - 1.0)
    end
  end

  @doc """
  Calculate PNP base current magnitude.
  """
  def pnp_base_current(vbe, vce, params \\ %BJTParams{}) do
    ic = pnp_collector_current(vbe, vce, params)

    if ic == 0.0 do
      0.0
    else
      ic / params.beta
    end
  end

  @doc """
  Digital abstraction: is this PNP transistor ON?

  PNP turns ON when |Vbe| >= Vbe_on (base pulled below emitter).
  """
  def pnp_is_conducting?(vbe, params \\ %BJTParams{}) do
    abs(vbe) >= params.vbe_on
  end

  @doc """
  Calculate small-signal transconductance gm for PNP.
  """
  def pnp_transconductance(vbe, vce, params \\ %BJTParams{}) do
    ic = pnp_collector_current(vbe, vce, params)

    if ic == 0.0 do
      0.0
    else
      vt = 0.026
      ic / vt
    end
  end
end
