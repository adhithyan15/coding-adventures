defmodule CodingAdventures.Transistors.Analysis do
  @moduledoc """
  Electrical Analysis — noise margins, power, timing, and technology comparison.

  ## Why Electrical Analysis Matters

  Digital logic designers care about more than truth tables:

  1. **NOISE MARGINS:** Can the circuit tolerate voltage fluctuations?
  2. **POWER:** How much energy does the chip consume?
  3. **TIMING:** How fast can the circuit switch?
  4. **SCALING:** How do properties change as transistors shrink?
  """

  alias CodingAdventures.Transistors.TTLGates
  alias CodingAdventures.Transistors.Types.{
    MOSFETParams,
    NoiseMargins,
    PowerAnalysis,
    TimingAnalysis
  }

  @doc """
  Analyze noise margins for a gate.

  Noise margins tell you how much electrical noise a digital signal
  can tolerate before being misinterpreted.

  `gate_type` must be `:cmos_inverter` or `:ttl_nand`.

  Options:
    - `vdd` — supply voltage for CMOS (default 3.3)
    - `vcc` — supply voltage for TTL (default 5.0)
  """
  def compute_noise_margins(gate_type, opts \\ []) do
    case gate_type do
      :cmos_inverter ->
        vdd = Keyword.get(opts, :vdd, 3.3)
        # CMOS has nearly ideal rail-to-rail output
        vol = 0.0
        voh = vdd
        # Input thresholds at ~40% and ~60% of Vdd (symmetric CMOS)
        vil = 0.4 * vdd
        vih = 0.6 * vdd
        nml = vil - vol
        nmh = voh - vih

        %NoiseMargins{vol: vol, voh: voh, vil: vil, vih: vih, nml: nml, nmh: nmh}

      :ttl_nand ->
        vcc = Keyword.get(opts, :vcc, 5.0)
        # TTL specifications (standard 74xx series)
        vol = 0.2
        voh = vcc - 0.7
        vil = 0.8
        vih = 2.0
        nml = vil - vol
        nmh = voh - vih

        %NoiseMargins{vol: vol, voh: voh, vil: vil, vih: vih, nml: nml, nmh: nmh}

      other ->
        raise ArgumentError, "Unsupported gate type: #{inspect(other)}"
    end
  end

  @doc """
  Compute power consumption for a gate at a given operating frequency.

  `gate_type` must be `:cmos_inverter`, `:cmos_nand`, `:cmos_nor`, or `:ttl_nand`.

  Options:
    - `frequency`       — Operating frequency in Hz (default 1 GHz)
    - `c_load`          — Load capacitance in Farads (default 1 pF)
    - `activity_factor` — Fraction of cycles with transition (default 0.5)
    - `vdd`             — CMOS supply voltage (default 3.3)
    - `vcc`             — TTL supply voltage (default 5.0)
  """
  def analyze_power(gate_type, opts \\ []) do
    frequency = Keyword.get(opts, :frequency, 1.0e9)
    c_load = Keyword.get(opts, :c_load, 1.0e-12)
    activity_factor = Keyword.get(opts, :activity_factor, 0.5)

    {static, vdd} =
      case gate_type do
        :ttl_nand ->
          vcc = Keyword.get(opts, :vcc, 5.0)
          {TTLGates.ttl_nand_static_power(vcc), vcc}

        type when type in [:cmos_inverter, :cmos_nand, :cmos_nor] ->
          vdd = Keyword.get(opts, :vdd, 3.3)
          {0.0, vdd}

        other ->
          raise ArgumentError, "Unsupported gate type: #{inspect(other)}"
      end

    # Dynamic power: P = C * V^2 * f * alpha
    dynamic = c_load * vdd * vdd * frequency * activity_factor
    total = static + dynamic

    # Energy per switching event: E = C * V^2
    energy_per_switch = c_load * vdd * vdd

    %PowerAnalysis{
      static_power: static,
      dynamic_power: dynamic,
      total_power: total,
      energy_per_switch: energy_per_switch
    }
  end

  @doc """
  Compute timing characteristics for a gate.

  `gate_type` must be `:cmos_inverter`, `:cmos_nand`, `:cmos_nor`, or `:ttl_nand`.

  Options:
    - `c_load`      — Load capacitance in Farads (default 1 pF)
    - `vdd`         — CMOS supply voltage (default 3.3)
    - `nmos_params` — MOSFET params for NMOS (default %MOSFETParams{})
    - `pmos_params` — MOSFET params for PMOS (default %MOSFETParams{})
  """
  def analyze_timing(gate_type, opts \\ []) do
    c_load = Keyword.get(opts, :c_load, 1.0e-12)

    case gate_type do
      :ttl_nand ->
        # TTL has relatively fixed timing characteristics
        tphl = 7.0e-9
        tplh = 11.0e-9
        tpd = (tphl + tplh) / 2.0
        rise_time = 15.0e-9
        fall_time = 10.0e-9
        max_frequency = 1.0 / (2.0 * tpd)

        %TimingAnalysis{
          tphl: tphl,
          tplh: tplh,
          tpd: tpd,
          rise_time: rise_time,
          fall_time: fall_time,
          max_frequency: max_frequency
        }

      type when type in [:cmos_inverter, :cmos_nand, :cmos_nor] ->
        vdd = Keyword.get(opts, :vdd, 3.3)
        nmos_params = Keyword.get(opts, :nmos_params, %MOSFETParams{})
        pmos_params = Keyword.get(opts, :pmos_params, %MOSFETParams{})

        k_n = nmos_params.k
        vth_n = nmos_params.vth
        k_p = pmos_params.k
        vth_p = pmos_params.vth

        ids_sat_n =
          if vdd > vth_n, do: 0.5 * k_n * (vdd - vth_n) * (vdd - vth_n), else: 1.0e-12

        ids_sat_p =
          if vdd > vth_p, do: 0.5 * k_p * (vdd - vth_p) * (vdd - vth_p), else: 1.0e-12

        # Propagation delays
        tphl = c_load * vdd / (2.0 * ids_sat_n)
        tplh = c_load * vdd / (2.0 * ids_sat_p)
        tpd = (tphl + tplh) / 2.0

        # Rise and fall times (2.2 RC time constants)
        r_on_n = if ids_sat_n > 0, do: vdd / (2.0 * ids_sat_n), else: 1.0e6
        r_on_p = if ids_sat_p > 0, do: vdd / (2.0 * ids_sat_p), else: 1.0e6
        rise_time = 2.2 * r_on_p * c_load
        fall_time = 2.2 * r_on_n * c_load

        max_frequency = if tpd > 0, do: 1.0 / (2.0 * tpd), else: :infinity

        %TimingAnalysis{
          tphl: tphl,
          tplh: tplh,
          tpd: tpd,
          rise_time: rise_time,
          fall_time: fall_time,
          max_frequency: max_frequency
        }

      other ->
        raise ArgumentError, "Unsupported gate type: #{inspect(other)}"
    end
  end

  @doc """
  Compare CMOS and TTL NAND gates across all metrics.

  Demonstrates WHY CMOS replaced TTL:
    - ~1000x less static power
    - Better noise margins relative to Vdd
    - Can operate at lower voltages
  """
  def compare_cmos_vs_ttl(opts \\ []) do
    frequency = Keyword.get(opts, :frequency, 1.0e6)
    c_load = Keyword.get(opts, :c_load, 1.0e-12)

    cmos_power = analyze_power(:cmos_nand, frequency: frequency, c_load: c_load)
    ttl_power = analyze_power(:ttl_nand, frequency: frequency, c_load: c_load)

    cmos_timing = analyze_timing(:cmos_nand, c_load: c_load)
    ttl_timing = analyze_timing(:ttl_nand, c_load: c_load)

    cmos_nm = compute_noise_margins(:cmos_inverter)
    ttl_nm = compute_noise_margins(:ttl_nand)

    %{
      "cmos" => %{
        "transistor_count" => 4,
        "supply_voltage" => 3.3,
        "static_power_w" => cmos_power.static_power,
        "dynamic_power_w" => cmos_power.dynamic_power,
        "total_power_w" => cmos_power.total_power,
        "propagation_delay_s" => cmos_timing.tpd,
        "max_frequency_hz" => cmos_timing.max_frequency,
        "noise_margin_low_v" => cmos_nm.nml,
        "noise_margin_high_v" => cmos_nm.nmh
      },
      "ttl" => %{
        "transistor_count" => 3,
        "supply_voltage" => 5.0,
        "static_power_w" => ttl_power.static_power,
        "dynamic_power_w" => ttl_power.dynamic_power,
        "total_power_w" => ttl_power.total_power,
        "propagation_delay_s" => ttl_timing.tpd,
        "max_frequency_hz" => ttl_timing.max_frequency,
        "noise_margin_low_v" => ttl_nm.nml,
        "noise_margin_high_v" => ttl_nm.nmh
      }
    }
  end

  @doc """
  Show how CMOS performance changes with technology scaling (Moore's Law).

  As transistors shrink:
    - Gate length decreases -> faster switching
    - Supply voltage decreases -> less power per switch
    - Gate capacitance decreases -> less energy per transition
    - BUT leakage current INCREASES -> more static power (the "leakage wall")
  """
  def demonstrate_cmos_scaling(technology_nodes \\ nil) do
    nodes = technology_nodes || [180.0e-9, 90.0e-9, 45.0e-9, 22.0e-9, 7.0e-9, 3.0e-9]

    Enum.map(nodes, fn node ->
      # Empirical scaling relationships (simplified)
      scale = node / 180.0e-9

      vdd = max(0.7, 3.3 * :math.sqrt(scale))
      vth = max(0.15, 0.4 * :math.pow(scale, 0.3))
      c_gate = 1.0e-15 * scale
      k = 0.001 / :math.sqrt(scale)

      params = %MOSFETParams{vth: vth, k: k, l: node, c_gate: c_gate}

      timing =
        analyze_timing(:cmos_inverter,
          c_load: c_gate * 10,
          vdd: vdd,
          nmos_params: params,
          pmos_params: params
        )

      power =
        analyze_power(:cmos_inverter,
          frequency: 1.0e9,
          c_load: c_gate * 10,
          vdd: vdd
        )

      # Leakage current increases exponentially as Vth decreases
      leakage = 1.0e-12 * :math.exp((0.4 - vth) / 0.052)

      %{
        "node_nm" => node * 1.0e9,
        "vdd_v" => vdd,
        "vth_v" => vth,
        "c_gate_f" => c_gate,
        "propagation_delay_s" => timing.tpd,
        "dynamic_power_w" => power.dynamic_power,
        "leakage_current_a" => leakage,
        "max_frequency_hz" => timing.max_frequency
      }
    end)
  end
end
