defmodule CodingAdventures.Transistors.CMOSGates do
  @moduledoc """
  CMOS Logic Gates — building digital logic from transistor pairs.

  ## What is CMOS?

  CMOS stands for Complementary Metal-Oxide-Semiconductor. It is the
  technology used in virtually every digital chip made since the 1980s.

  The "complementary" refers to pairing NMOS and PMOS transistors:
    - PMOS transistors form the PULL-UP network (connects output to Vdd)
    - NMOS transistors form the PULL-DOWN network (connects output to GND)

  For any valid input combination, exactly ONE network is active:
    - Pull-up ON -> output = Vdd (logic HIGH)
    - Pull-down ON -> output = GND (logic LOW)
    - Never both ON simultaneously -> near-zero static power

  ## Transistor Counts

      Gate    | NMOS | PMOS | Total
      --------|------|------|------
      NOT     |  1   |  1   |   2
      NAND    |  2   |  2   |   4
      NOR     |  2   |  2   |   4
      AND     |  3   |  3   |   6
      OR      |  3   |  3   |   6
      XOR     |  3   |  3   |   6
  """

  alias CodingAdventures.Transistors.MOSFET
  alias CodingAdventures.Transistors.Types.{CircuitParams, GateOutput, MOSFETParams}

  # ---------------------------------------------------------------------------
  # Input validation
  # ---------------------------------------------------------------------------
  # Every gate checks that its inputs are valid binary values (0 or 1).
  # Booleans are explicitly rejected — they must be integers.

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
  # CMOS INVERTER (NOT gate) — 2 transistors
  # ===========================================================================
  #
  #          Vdd
  #           |
  #      [ PMOS ] <-- gate = input
  #           |
  #        Output
  #           |
  #      [ NMOS ] <-- gate = input
  #           |
  #          GND
  #
  # Input HIGH: NMOS ON (pulls to GND), PMOS OFF -> Output LOW
  # Input LOW:  PMOS ON (pulls to Vdd), NMOS OFF -> Output HIGH

  @doc """
  Evaluate the CMOS inverter with an analog input voltage.

  Maps the input voltage through the CMOS transfer characteristic
  to produce an output voltage with full electrical detail.
  """
  def inverter_evaluate(input_voltage, circuit_params \\ %CircuitParams{},
        nmos_params \\ %MOSFETParams{}, pmos_params \\ %MOSFETParams{}) do
    vdd = circuit_params.vdd

    # NMOS: gate = input, source = GND -> Vgs_n = Vin
    vgs_n = input_voltage
    # PMOS: gate = input, source = Vdd -> Vgs_p = Vin - Vdd
    vgs_p = input_voltage - vdd

    nmos_on = MOSFET.nmos_is_conducting?(vgs_n, nmos_params)
    pmos_on = MOSFET.pmos_is_conducting?(vgs_p, pmos_params)

    # Determine output voltage
    output_v =
      cond do
        pmos_on and not nmos_on -> vdd
        nmos_on and not pmos_on -> 0.0
        true -> vdd / 2.0
      end

    logic_value = if output_v > vdd / 2.0, do: 1, else: 0

    # Current draw: only significant during transition (both on)
    current =
      if nmos_on and pmos_on do
        vds_n = vdd / 2.0
        MOSFET.nmos_drain_current(vgs_n, vds_n, nmos_params)
      else
        0.0
      end

    power = current * vdd

    # Propagation delay estimate
    c_load = nmos_params.c_drain + pmos_params.c_drain

    delay =
      if current > 0 do
        c_load * vdd / (2.0 * current)
      else
        ids_sat = MOSFET.nmos_drain_current(vdd, vdd, nmos_params)
        if ids_sat > 0, do: c_load * vdd / (2.0 * ids_sat), else: 1.0e-9
      end

    %GateOutput{
      logic_value: logic_value,
      voltage: output_v,
      current_draw: current,
      power_dissipation: power,
      propagation_delay: delay,
      transistor_count: 2
    }
  end

  @doc """
  Evaluate CMOS inverter with digital input (0 or 1).
  """
  def inverter_evaluate_digital(a, circuit_params \\ %CircuitParams{}) do
    validate_bit!(a, "a")
    vin = if a == 1, do: circuit_params.vdd, else: 0.0
    result = inverter_evaluate(vin, circuit_params)
    result.logic_value
  end

  @doc """
  Static power dissipation of a CMOS inverter (ideally ~0).

  In an ideal CMOS inverter, one transistor is always OFF, so no
  DC current flows from Vdd to GND.
  """
  def inverter_static_power, do: 0.0

  @doc """
  Dynamic power: P = C_load * Vdd^2 * f.

  Every time the output switches, the load capacitance must be
  charged or discharged. Energy per transition is C * Vdd^2.
  """
  def inverter_dynamic_power(frequency, c_load, circuit_params \\ %CircuitParams{}) do
    vdd = circuit_params.vdd
    c_load * vdd * vdd * frequency
  end

  @doc """
  Generate the voltage transfer characteristic (VTC) curve.

  Returns a list of `{vin, vout}` tuples showing the sharp switching
  threshold of CMOS.
  """
  def inverter_vtc(steps \\ 100, circuit_params \\ %CircuitParams{}) do
    vdd = circuit_params.vdd

    for i <- 0..steps do
      vin = vdd * i / steps
      result = inverter_evaluate(vin, circuit_params)
      {vin, result.voltage}
    end
  end

  # ===========================================================================
  # CMOS NAND gate — 4 transistors
  # ===========================================================================
  # Pull-up: PMOS in PARALLEL (either ON -> output HIGH)
  # Pull-down: NMOS in SERIES (BOTH must be ON -> output LOW)

  @doc """
  Evaluate CMOS NAND gate with analog input voltages.
  """
  def nand_evaluate(va, vb, circuit_params \\ %CircuitParams{},
        nmos_params \\ %MOSFETParams{}, pmos_params \\ %MOSFETParams{}) do
    vdd = circuit_params.vdd

    vgs_n1 = va
    vgs_n2 = vb
    vgs_p1 = va - vdd
    vgs_p2 = vb - vdd

    nmos1_on = MOSFET.nmos_is_conducting?(vgs_n1, nmos_params)
    nmos2_on = MOSFET.nmos_is_conducting?(vgs_n2, nmos_params)
    pmos1_on = MOSFET.pmos_is_conducting?(vgs_p1, pmos_params)
    pmos2_on = MOSFET.pmos_is_conducting?(vgs_p2, pmos_params)

    # Pull-down: NMOS in SERIES — BOTH must be ON
    pulldown_on = nmos1_on and nmos2_on
    # Pull-up: PMOS in PARALLEL — EITHER can pull up
    pullup_on = pmos1_on or pmos2_on

    output_v =
      cond do
        pullup_on and not pulldown_on -> vdd
        pulldown_on and not pullup_on -> 0.0
        true -> vdd / 2.0
      end

    logic_value = if output_v > vdd / 2.0, do: 1, else: 0
    current = if pulldown_on and pullup_on, do: 0.001, else: 0.0

    c_load = nmos_params.c_drain + pmos_params.c_drain
    ids_sat = MOSFET.nmos_drain_current(vdd, vdd, nmos_params)
    delay = if ids_sat > 0, do: c_load * vdd / (2.0 * ids_sat), else: 1.0e-9

    %GateOutput{
      logic_value: logic_value,
      voltage: output_v,
      current_draw: current,
      power_dissipation: current * vdd,
      propagation_delay: delay,
      transistor_count: 4
    }
  end

  @doc """
  Evaluate CMOS NAND with digital inputs (0 or 1).
  """
  def nand_evaluate_digital(a, b, circuit_params \\ %CircuitParams{}) do
    validate_bit!(a, "a")
    validate_bit!(b, "b")
    vdd = circuit_params.vdd
    va = if a == 1, do: vdd, else: 0.0
    vb = if b == 1, do: vdd, else: 0.0
    result = nand_evaluate(va, vb, circuit_params)
    result.logic_value
  end

  # ===========================================================================
  # CMOS NOR gate — 4 transistors
  # ===========================================================================
  # Pull-up: PMOS in SERIES (BOTH must be ON -> output HIGH)
  # Pull-down: NMOS in PARALLEL (EITHER ON -> output LOW)

  @doc """
  Evaluate CMOS NOR gate with analog input voltages.
  """
  def nor_evaluate(va, vb, circuit_params \\ %CircuitParams{},
        nmos_params \\ %MOSFETParams{}, pmos_params \\ %MOSFETParams{}) do
    vdd = circuit_params.vdd

    vgs_n1 = va
    vgs_n2 = vb
    vgs_p1 = va - vdd
    vgs_p2 = vb - vdd

    nmos1_on = MOSFET.nmos_is_conducting?(vgs_n1, nmos_params)
    nmos2_on = MOSFET.nmos_is_conducting?(vgs_n2, nmos_params)
    pmos1_on = MOSFET.pmos_is_conducting?(vgs_p1, pmos_params)
    pmos2_on = MOSFET.pmos_is_conducting?(vgs_p2, pmos_params)

    # Pull-down: NMOS in PARALLEL — EITHER ON pulls low
    pulldown_on = nmos1_on or nmos2_on
    # Pull-up: PMOS in SERIES — BOTH must be ON
    pullup_on = pmos1_on and pmos2_on

    output_v =
      cond do
        pullup_on and not pulldown_on -> vdd
        pulldown_on and not pullup_on -> 0.0
        true -> vdd / 2.0
      end

    logic_value = if output_v > vdd / 2.0, do: 1, else: 0
    current = if pulldown_on and pullup_on, do: 0.001, else: 0.0

    c_load = nmos_params.c_drain + pmos_params.c_drain
    ids_sat = MOSFET.nmos_drain_current(vdd, vdd, nmos_params)
    delay = if ids_sat > 0, do: c_load * vdd / (2.0 * ids_sat), else: 1.0e-9

    %GateOutput{
      logic_value: logic_value,
      voltage: output_v,
      current_draw: current,
      power_dissipation: current * vdd,
      propagation_delay: delay,
      transistor_count: 4
    }
  end

  @doc """
  Evaluate CMOS NOR with digital inputs (0 or 1).
  """
  def nor_evaluate_digital(a, b, circuit_params \\ %CircuitParams{}) do
    validate_bit!(a, "a")
    validate_bit!(b, "b")
    vdd = circuit_params.vdd
    va = if a == 1, do: vdd, else: 0.0
    vb = if b == 1, do: vdd, else: 0.0
    result = nor_evaluate(va, vb, circuit_params)
    result.logic_value
  end

  # ===========================================================================
  # CMOS AND gate — 6 transistors (NAND + inverter)
  # ===========================================================================

  @doc """
  Evaluate CMOS AND gate with analog input voltages.

  AND = NOT(NAND(A, B)). Requires 6 transistors because CMOS naturally
  produces inverted outputs.
  """
  def and_evaluate(va, vb, circuit_params \\ %CircuitParams{}) do
    nand_out = nand_evaluate(va, vb, circuit_params)
    inv_out = inverter_evaluate(nand_out.voltage, circuit_params)

    %GateOutput{
      logic_value: inv_out.logic_value,
      voltage: inv_out.voltage,
      current_draw: nand_out.current_draw + inv_out.current_draw,
      power_dissipation: nand_out.power_dissipation + inv_out.power_dissipation,
      propagation_delay: nand_out.propagation_delay + inv_out.propagation_delay,
      transistor_count: 6
    }
  end

  @doc """
  Evaluate CMOS AND with digital inputs (0 or 1).
  """
  def and_evaluate_digital(a, b, circuit_params \\ %CircuitParams{}) do
    validate_bit!(a, "a")
    validate_bit!(b, "b")
    vdd = circuit_params.vdd
    va = if a == 1, do: vdd, else: 0.0
    vb = if b == 1, do: vdd, else: 0.0
    result = and_evaluate(va, vb, circuit_params)
    result.logic_value
  end

  # ===========================================================================
  # CMOS OR gate — 6 transistors (NOR + inverter)
  # ===========================================================================

  @doc """
  Evaluate CMOS OR gate with analog input voltages.

  OR = NOT(NOR(A, B)). 6 transistors.
  """
  def or_evaluate(va, vb, circuit_params \\ %CircuitParams{}) do
    nor_out = nor_evaluate(va, vb, circuit_params)
    inv_out = inverter_evaluate(nor_out.voltage, circuit_params)

    %GateOutput{
      logic_value: inv_out.logic_value,
      voltage: inv_out.voltage,
      current_draw: nor_out.current_draw + inv_out.current_draw,
      power_dissipation: nor_out.power_dissipation + inv_out.power_dissipation,
      propagation_delay: nor_out.propagation_delay + inv_out.propagation_delay,
      transistor_count: 6
    }
  end

  @doc """
  Evaluate CMOS OR with digital inputs (0 or 1).
  """
  def or_evaluate_digital(a, b, circuit_params \\ %CircuitParams{}) do
    validate_bit!(a, "a")
    validate_bit!(b, "b")
    vdd = circuit_params.vdd
    va = if a == 1, do: vdd, else: 0.0
    vb = if b == 1, do: vdd, else: 0.0
    result = or_evaluate(va, vb, circuit_params)
    result.logic_value
  end

  # ===========================================================================
  # CMOS XOR gate — 6 transistors (4 NAND gates logically)
  # ===========================================================================
  # XOR(A, B) = NAND(NAND(A, NAND(A,B)), NAND(B, NAND(A,B)))

  @doc """
  Evaluate CMOS XOR gate with analog input voltages.

  Uses 4 NAND gates internally: XOR(A,B) = NAND(NAND(A,NAND(A,B)), NAND(B,NAND(A,B)))
  """
  def xor_evaluate(va, vb, circuit_params \\ %CircuitParams{}) do
    vdd = circuit_params.vdd

    # Step 1: NAND(A, B)
    nand_ab = nand_evaluate(va, vb, circuit_params)
    # Step 2: NAND(A, NAND(A,B))
    nand_a_nab = nand_evaluate(va, nand_ab.voltage, circuit_params)
    # Step 3: NAND(B, NAND(A,B))
    nand_b_nab = nand_evaluate(vb, nand_ab.voltage, circuit_params)
    # Step 4: NAND(step2, step3)
    result = nand_evaluate(nand_a_nab.voltage, nand_b_nab.voltage, circuit_params)

    total_current =
      nand_ab.current_draw + nand_a_nab.current_draw +
        nand_b_nab.current_draw + result.current_draw

    total_delay =
      nand_ab.propagation_delay +
        max(nand_a_nab.propagation_delay, nand_b_nab.propagation_delay) +
        result.propagation_delay

    %GateOutput{
      logic_value: result.logic_value,
      voltage: result.voltage,
      current_draw: total_current,
      power_dissipation: total_current * vdd,
      propagation_delay: total_delay,
      transistor_count: 6
    }
  end

  @doc """
  Evaluate CMOS XOR with digital inputs (0 or 1).
  """
  def xor_evaluate_digital(a, b, circuit_params \\ %CircuitParams{}) do
    validate_bit!(a, "a")
    validate_bit!(b, "b")
    vdd = circuit_params.vdd
    va = if a == 1, do: vdd, else: 0.0
    vb = if b == 1, do: vdd, else: 0.0
    result = xor_evaluate(va, vb, circuit_params)
    result.logic_value
  end

  @doc """
  Build XOR from 4 NAND gates to demonstrate universality.

  Same as `xor_evaluate_digital/2` but makes the NAND construction explicit.
  """
  def xor_evaluate_from_nands(a, b, circuit_params \\ %CircuitParams{}) do
    xor_evaluate_digital(a, b, circuit_params)
  end

  # ===========================================================================
  # CMOS XNOR gate — 8 transistors (XOR + Inverter)
  # ===========================================================================
  # XNOR(A, B) = NOT(XOR(A, B))
  # The "equivalence" gate: outputs 1 when A and B are equal.

  @doc """
  Evaluates a CMOS XNOR gate (XOR followed by an Inverter).

  XNOR(A, B) = NOT(XOR(A, B))

  Truth table:

  | A | B | XNOR |
  |---|---|------|
  | 0 | 0 |  1   |
  | 0 | 1 |  0   |
  | 1 | 0 |  0   |
  | 1 | 1 |  1   |

  Transistor count: xor_transistor_count + 2 (XOR + Inverter).
  XNOR is the "equivalence" gate — it answers "are A and B equal?"
  """
  def xnor_evaluate(va, vb, circuit_params \\ %CircuitParams{}) do
    xor_out = xor_evaluate(va, vb, circuit_params)
    inv_out = inverter_evaluate(xor_out.voltage, circuit_params)

    %GateOutput{
      logic_value: inv_out.logic_value,
      voltage: inv_out.voltage,
      current_draw: xor_out.current_draw + inv_out.current_draw,
      power_dissipation: xor_out.power_dissipation + inv_out.power_dissipation,
      propagation_delay: xor_out.propagation_delay + inv_out.propagation_delay,
      transistor_count: xor_out.transistor_count + 2
    }
  end

  @doc """
  Evaluates a CMOS XNOR gate with digital (0/1) inputs.
  """
  def xnor_evaluate_digital(a, b, circuit_params \\ %CircuitParams{}) do
    validate_bit!(a, "a")
    validate_bit!(b, "b")
    vdd = circuit_params.vdd
    va = if a == 1, do: vdd, else: 0.0
    vb = if b == 1, do: vdd, else: 0.0
    result = xnor_evaluate(va, vb, circuit_params)
    result.logic_value
  end
end
