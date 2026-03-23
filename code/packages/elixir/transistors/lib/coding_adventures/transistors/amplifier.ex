defmodule CodingAdventures.Transistors.Amplifier do
  @moduledoc """
  Analog Amplifier Analysis — transistors as signal amplifiers.

  ## Beyond Digital: Transistors as Amplifiers

  When biased in the right operating region (saturation for MOSFET, active
  for BJT), transistors can amplify small signals into larger ones.

  ## Common-Source Amplifier (MOSFET)

  The input signal modulates Vgs, which modulates Ids (via gm), creating
  a voltage drop across drain resistor Rd:

      Voltage gain: Av = -gm * Rd  (inverting amplifier)

  ## Common-Emitter Amplifier (BJT)

  Input modulates Vbe, which modulates Ic (via gm = Ic/Vt), creating a
  voltage drop across collector resistor Rc:

      Voltage gain: Av = -gm * Rc
  """

  alias CodingAdventures.Transistors.MOSFET
  alias CodingAdventures.Transistors.BJT
  alias CodingAdventures.Transistors.Types.{AmplifierAnalysis, BJTParams, MOSFETParams}

  @doc """
  Analyze an NMOS common-source amplifier configuration.

  The MOSFET must be biased in SATURATION for amplification:
  Vgs > Vth AND Vds >= Vgs - Vth.

  Returns an `AmplifierAnalysis` struct with gain, impedance, and bandwidth.
  """
  def analyze_common_source(vgs, vdd, r_drain, c_load \\ 1.0e-12,
        nmos_params \\ %MOSFETParams{}) do
    # Calculate DC operating point
    ids = MOSFET.nmos_drain_current(vgs, vdd, nmos_params)
    vds = vdd - ids * r_drain

    # Recalculate with correct Vds
    ids = MOSFET.nmos_drain_current(vgs, max(vds, 0.0), nmos_params)
    vds = vdd - ids * r_drain

    # Transconductance
    gm = MOSFET.nmos_transconductance(vgs, max(vds, 0.0), nmos_params)

    # Voltage gain: Av = -gm * Rd (inverting amplifier)
    voltage_gain = -gm * r_drain

    # Input impedance: essentially infinite for MOSFET (gate is insulated)
    input_impedance = 1.0e12

    # Output impedance: approximately Rd
    output_impedance = r_drain

    # Bandwidth: f_3dB = 1 / (2*pi * Rd * C_load)
    bandwidth = 1.0 / (2.0 * :math.pi() * r_drain * c_load)

    operating_point = %{
      "vgs" => vgs,
      "vds" => vds,
      "ids" => ids,
      "gm" => gm
    }

    %AmplifierAnalysis{
      voltage_gain: voltage_gain,
      transconductance: gm,
      input_impedance: input_impedance,
      output_impedance: output_impedance,
      bandwidth: bandwidth,
      operating_point: operating_point
    }
  end

  @doc """
  Analyze an NPN common-emitter amplifier configuration.

  BJT amplifiers typically have higher voltage gain than MOSFET amplifiers
  at the same current, but lower input impedance (base current flows).

  Returns an `AmplifierAnalysis` struct with gain, impedance, and bandwidth.
  """
  def analyze_common_emitter(vbe, vcc, r_collector, c_load \\ 1.0e-12,
        bjt_params \\ %BJTParams{}) do
    # Calculate DC operating point
    vce = vcc
    ic = BJT.npn_collector_current(vbe, vce, bjt_params)
    vce = vcc - ic * r_collector
    vce = max(vce, 0.0)

    # Recalculate with correct Vce
    ic = BJT.npn_collector_current(vbe, vce, bjt_params)

    # Transconductance
    gm = BJT.npn_transconductance(vbe, vce, bjt_params)

    # Voltage gain: Av = -gm * Rc
    voltage_gain = -gm * r_collector

    # Input impedance: r_pi = beta * Vt / Ic
    beta = bjt_params.beta
    vt = 0.026

    r_pi =
      if ic > 0 do
        beta * vt / ic
      else
        1.0e12
      end

    input_impedance = r_pi
    output_impedance = r_collector

    # Bandwidth
    bandwidth = 1.0 / (2.0 * :math.pi() * r_collector * c_load)

    ib = BJT.npn_base_current(vbe, vce, bjt_params)

    operating_point = %{
      "vbe" => vbe,
      "vce" => vce,
      "ic" => ic,
      "ib" => ib,
      "gm" => gm
    }

    %AmplifierAnalysis{
      voltage_gain: voltage_gain,
      transconductance: gm,
      input_impedance: input_impedance,
      output_impedance: output_impedance,
      bandwidth: bandwidth,
      operating_point: operating_point
    }
  end
end
