defmodule CodingAdventures.Transistors.Types do
  @moduledoc """
  Shared types for the transistors package.

  ## Enums and Parameter Structs

  These types define the vocabulary of transistor simulation. Every transistor
  has an operating region (cutoff, linear, saturation), and every circuit has
  electrical parameters (voltage, capacitance, etc.).

  We use structs with `@enforce_keys` for parameters because transistor
  characteristics are fixed once manufactured — you cannot change a transistor's
  threshold voltage after fabrication.

  ## Operating Regions

  MOSFET regions are represented as atoms:
    - `:cutoff`     — transistor is OFF, no current flows
    - `:linear`     — acts like a variable resistor (digital ON state)
    - `:saturation` — current roughly constant (analog amplifier mode)

  BJT regions are also atoms:
    - `:cutoff`     — no base current, no collector current
    - `:active`     — linear amplifier region (Ic = beta * Ib)
    - `:saturation` — fully ON as a switch (both junctions forward-biased)

  **Confusing naming alert:** MOSFET "saturation" = constant current (amplifier).
  BJT "saturation" = fully ON (switch). These are DIFFERENT behaviors despite
  sharing a name.
  """

  # ===========================================================================
  # MOSFET PARAMETERS
  # ===========================================================================
  # Default values represent a typical 180nm CMOS process — the last
  # "large" process node that is still widely used in education and
  # analog/mixed-signal chips.

  defmodule MOSFETParams do
    @moduledoc """
    Electrical parameters for a MOSFET transistor.

    Key parameters:
      - `vth`     — Threshold voltage. Minimum Vgs to turn ON. Lower = faster but more leakage.
      - `k`       — Transconductance parameter. Controls current per Vgs. k = mu * Cox * (W/L).
      - `w`, `l`  — Channel width and length. W/L ratio tunes transistor strength.
      - `c_gate`  — Gate capacitance. Determines switching speed.
      - `c_drain` — Drain junction capacitance. Contributes to output load.
    """

    defstruct vth: 0.4,
              k: 0.001,
              w: 1.0e-6,
              l: 180.0e-9,
              c_gate: 1.0e-15,
              c_drain: 0.5e-15
  end

  # ===========================================================================
  # BJT PARAMETERS
  # ===========================================================================
  # Default values represent a typical small-signal NPN transistor
  # like the 2N2222 — one of the most common transistors ever made.

  defmodule BJTParams do
    @moduledoc """
    Electrical parameters for a BJT transistor.

    Key parameters:
      - `beta`   — Current gain (hfe). Ratio Ic/Ib. Typically 50-300.
      - `vbe_on` — Base-emitter voltage when conducting (~0.7V for silicon).
      - `vce_sat`— Collector-emitter voltage when fully saturated (~0.2V).
      - `i_s`    — Reverse saturation current. Tiny leakage when OFF.
      - `c_base` — Base capacitance. Limits switching speed.
    """

    defstruct beta: 100.0,
              vbe_on: 0.7,
              vce_sat: 0.2,
              i_s: 1.0e-14,
              c_base: 5.0e-12
  end

  # ===========================================================================
  # CIRCUIT PARAMETERS
  # ===========================================================================

  defmodule CircuitParams do
    @moduledoc """
    Parameters for a complete logic gate circuit.

      - `vdd`         — Supply voltage. Modern CMOS: 0.7-1.2V. Older: 3.3V or 5V.
      - `temperature` — Junction temperature in Kelvin. Room temp ~300K.
    """

    defstruct vdd: 3.3,
              temperature: 300.0
  end

  # ===========================================================================
  # RESULT TYPES
  # ===========================================================================

  defmodule GateOutput do
    @moduledoc """
    Result of evaluating a logic gate with voltage-level detail.

    Unlike the logic_gates package which only returns 0 or 1, this gives
    you the full electrical picture: voltage, current, power, and timing.
    """

    defstruct logic_value: 0,
              voltage: 0.0,
              current_draw: 0.0,
              power_dissipation: 0.0,
              propagation_delay: 0.0,
              transistor_count: 0
  end

  defmodule AmplifierAnalysis do
    @moduledoc """
    Results of analyzing a transistor as an amplifier.

      - `voltage_gain`     — Output voltage change per input voltage change. Negative = inverting.
      - `transconductance` — gm, ratio of output current to input voltage change (Siemens).
      - `input_impedance`  — How much the amplifier loads the source.
      - `output_impedance` — How stiff the output is. Lower = drives heavier loads.
      - `bandwidth`        — Frequency at which gain drops to -3dB.
      - `operating_point`  — Map of DC bias conditions.
    """

    defstruct voltage_gain: 0.0,
              transconductance: 0.0,
              input_impedance: 0.0,
              output_impedance: 0.0,
              bandwidth: 0.0,
              operating_point: %{}
  end

  defmodule NoiseMargins do
    @moduledoc """
    Noise margin analysis for a logic family.

      - `vol` — Output LOW voltage
      - `voh` — Output HIGH voltage
      - `vil` — Input LOW threshold (max voltage accepted as 0)
      - `vih` — Input HIGH threshold (min voltage accepted as 1)
      - `nml` — Noise Margin LOW = vil - vol
      - `nmh` — Noise Margin HIGH = voh - vih
    """

    defstruct vol: 0.0,
              voh: 0.0,
              vil: 0.0,
              vih: 0.0,
              nml: 0.0,
              nmh: 0.0
  end

  defmodule PowerAnalysis do
    @moduledoc """
    Power consumption breakdown for a gate or circuit.

      - `static_power`      — Power consumed even when not switching.
      - `dynamic_power`     — Power consumed during switching. P = C * Vdd^2 * f * alpha.
      - `total_power`       — static + dynamic.
      - `energy_per_switch` — Energy for one 0->1->0 transition. E = C * Vdd^2.
    """

    defstruct static_power: 0.0,
              dynamic_power: 0.0,
              total_power: 0.0,
              energy_per_switch: 0.0
  end

  defmodule TimingAnalysis do
    @moduledoc """
    Timing characteristics for a gate.

      - `tphl`          — Propagation delay HIGH to LOW.
      - `tplh`          — Propagation delay LOW to HIGH.
      - `tpd`           — Average propagation delay = (tphl + tplh) / 2.
      - `rise_time`     — Time from 10% to 90% of Vdd.
      - `fall_time`     — Time from 90% to 10% of Vdd.
      - `max_frequency` — Maximum clock frequency = 1 / (2 * tpd).
    """

    defstruct tphl: 0.0,
              tplh: 0.0,
              tpd: 0.0,
              rise_time: 0.0,
              fall_time: 0.0,
              max_frequency: 0.0
  end
end
