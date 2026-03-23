defmodule CodingAdventures.Transistors.TTLGates do
  @moduledoc """
  TTL Logic Gates — historical BJT-based digital logic.

  ## What is TTL?

  TTL stands for Transistor-Transistor Logic. It was the dominant digital
  logic family from the mid-1960s through the 1980s, when CMOS replaced it.
  The "7400 series" defined the standard logic gates.

  ## Why TTL Lost to CMOS

  TTL's fatal flaw: STATIC POWER CONSUMPTION. In a TTL gate, current flows
  through resistors and transistors even when idle. A single gate dissipates
  ~1-10 mW at rest. For 1 million gates, that's 10,000 watts!

  ## RTL: The Predecessor to TTL

  Before TTL came RTL (Resistor-Transistor Logic), the simplest possible
  transistor logic. Used in the Apollo Guidance Computer (1969).
  """

  alias CodingAdventures.Transistors.Types.{BJTParams, GateOutput}

  defp validate_bit!(value, name) do
    cond do
      is_boolean(value) ->
        raise ArgumentError, "#{name} must be an integer, got boolean #{inspect(value)}"

      not is_integer(value) ->
        raise ArgumentError, "#{name} must be an integer, got #{inspect(value)}"

      value not in [0, 1] ->
        raise ArgumentError, "#{name} must be 0 or 1, got #{value}"

      true ->
        :ok
    end
  end

  # ===========================================================================
  # TTL NAND GATE
  # ===========================================================================
  # Uses NPN transistors in a simplified 7400-series topology.
  # Static power: ~1-10 mW per gate (base current always flows).

  @doc """
  Evaluate a TTL NAND gate with analog input voltages.

  TTL input thresholds: LOW < 0.8V, HIGH > 2.0V.

  When ALL inputs HIGH: output LOW (~0.2V, Vce_sat).
  When ANY input LOW: output HIGH (~Vcc - 0.7V).
  """
  def ttl_nand_evaluate(va, vb, vcc \\ 5.0, bjt_params \\ %BJTParams{}) do
    vbe_on = bjt_params.vbe_on
    r_pullup = 4000.0

    a_high = va > 2.0
    b_high = vb > 2.0

    {output_v, logic_value, current} =
      if a_high and b_high do
        # ALL inputs HIGH -> output LOW
        cur = (vcc - 2 * vbe_on - bjt_params.vce_sat) / r_pullup
        {bjt_params.vce_sat, 0, max(cur, 0.0)}
      else
        # At least one input LOW -> output HIGH
        out_v = vcc - vbe_on
        cur = (vcc - out_v) / r_pullup
        {out_v, 1, max(cur, 0.0)}
      end

    power = current * vcc
    delay = 10.0e-9

    %GateOutput{
      logic_value: logic_value,
      voltage: output_v,
      current_draw: current,
      power_dissipation: power,
      propagation_delay: delay,
      transistor_count: 3
    }
  end

  @doc """
  Evaluate TTL NAND with digital inputs (0 or 1).
  """
  def ttl_nand_evaluate_digital(a, b, vcc \\ 5.0) do
    validate_bit!(a, "a")
    validate_bit!(b, "b")
    va = if a == 1, do: vcc, else: 0.0
    vb = if b == 1, do: vcc, else: 0.0
    result = ttl_nand_evaluate(va, vb, vcc)
    result.logic_value
  end

  @doc """
  Static power dissipation of TTL NAND — significantly higher than CMOS.

  TTL gates consume power continuously due to resistor-based biasing.
  Returns static power in watts. Typically ~1-10 mW for a single gate.
  """
  def ttl_nand_static_power(vcc \\ 5.0, bjt_params \\ %BJTParams{}) do
    r_pullup = 4000.0
    current = (vcc - 2 * bjt_params.vbe_on - bjt_params.vce_sat) / r_pullup
    max(current, 0.0) * vcc
  end

  # ===========================================================================
  # RTL INVERTER
  # ===========================================================================
  # The simplest possible transistor logic: one NPN + two resistors.
  # Used in the Apollo Guidance Computer (1969).

  @doc """
  Evaluate an RTL inverter with an analog input voltage.

  Input HIGH: base current flows, Q1 saturates, output LOW (~Vce_sat).
  Input LOW: no base current, Q1 off, output HIGH (~Vcc).
  """
  def rtl_inverter_evaluate(v_input, vcc \\ 5.0, r_base \\ 10_000.0,
        r_collector \\ 1_000.0, bjt_params \\ %BJTParams{}) do
    vbe_on = bjt_params.vbe_on

    {output_v, logic_value, current} =
      if v_input > vbe_on do
        ib = (v_input - vbe_on) / r_base
        ic = min(ib * bjt_params.beta, (vcc - bjt_params.vce_sat) / r_collector)
        out_v = vcc - ic * r_collector
        out_v = max(out_v, bjt_params.vce_sat)
        lv = if out_v < vcc / 2.0, do: 0, else: 1
        {out_v, lv, ic + ib}
      else
        {vcc, 1, 0.0}
      end

    power = current * vcc
    delay = 50.0e-9

    %GateOutput{
      logic_value: logic_value,
      voltage: output_v,
      current_draw: current,
      power_dissipation: power,
      propagation_delay: delay,
      transistor_count: 1
    }
  end

  @doc """
  Evaluate RTL inverter with digital input (0 or 1).
  """
  def rtl_inverter_evaluate_digital(a, vcc \\ 5.0) do
    validate_bit!(a, "a")
    v_input = if a == 1, do: vcc, else: 0.0
    result = rtl_inverter_evaluate(v_input, vcc)
    result.logic_value
  end
end
