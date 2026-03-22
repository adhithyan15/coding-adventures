# frozen_string_literal: true

# CMOS Logic Gates — building digital logic from transistor pairs.
#
# === What is CMOS? ===
#
# CMOS stands for Complementary Metal-Oxide-Semiconductor. It is the
# technology used in virtually every digital chip made since the 1980s:
# CPUs, GPUs, memory, phone processors — all CMOS.
#
# The "complementary" refers to pairing NMOS and PMOS transistors:
#     - PMOS transistors form the PULL-UP network (connects output to Vdd)
#     - NMOS transistors form the PULL-DOWN network (connects output to GND)
#
# For any valid input combination, exactly ONE network is active:
#     - If pull-up is ON -> output = Vdd (logic HIGH)
#     - If pull-down is ON -> output = GND (logic LOW)
#     - Never both ON simultaneously -> no DC current path -> near-zero static power
#
# === Transistor Counts ===
#
#     Gate    | NMOS | PMOS | Total | Notes
#     --------|------|------|-------|------
#     NOT     |  1   |  1   |   2   | The simplest CMOS circuit
#     NAND    |  2   |  2   |   4   | Natural CMOS gate
#     NOR     |  2   |  2   |   4   | Natural CMOS gate
#     AND     |  3   |  3   |   6   | NAND + NOT
#     OR      |  3   |  3   |   6   | NOR + NOT
#     XOR     |  3   |  3   |   6   | Transmission gate design

module CodingAdventures
  module Transistors
    # Validate that a value is a binary digit (0 or 1).
    #
    # We reuse the same strict validation as the logic_gates package:
    # reject booleans, floats, and out-of-range integers.
    #
    # @param value [Object] the value to validate
    # @param name [String] the name of the parameter for error messages
    # @raise [TypeError] if value is not an Integer or is a Boolean
    # @raise [ValueError] if value is not 0 or 1
    def self.validate_bit(value, name = "input")
      if value.is_a?(TrueClass) || value.is_a?(FalseClass) || !value.is_a?(Integer)
        raise TypeError, "#{name} must be an Integer, got #{value.class.name}"
      end

      return if [0, 1].include?(value)

      raise ArgumentError, "#{name} must be 0 or 1, got #{value}"
    end

    # CMOS NOT gate: 1 PMOS + 1 NMOS = 2 transistors.
    #
    # The simplest and most important CMOS circuit. Every other CMOS gate
    # is a variation of this fundamental pattern.
    #
    # === Circuit Diagram ===
    #
    #          Vdd
    #           |
    #      +----+----+
    #      |  PMOS   |
    #      |         |--- Gate ---- Input (A)
    #      +----+----+
    #           |
    #           +------------- Output (Y = NOT A)
    #           |
    #      +----+----+
    #      |  NMOS   |
    #      |         |--- Gate ---- Input (A)
    #      +----+----+
    #           |
    #          GND
    #
    # === How it works ===
    #
    # Input A = HIGH (Vdd):
    #     NMOS: Vgs = Vdd > Vth -> ON -> pulls output to GND
    #     PMOS: Vgs = 0 -> OFF -> disconnected from Vdd
    #     Output = LOW (GND) = NOT HIGH
    #
    # Input A = LOW (0V):
    #     NMOS: Vgs = 0 < Vth -> OFF -> disconnected from GND
    #     PMOS: Vgs = -Vdd -> ON -> pulls output to Vdd
    #     Output = HIGH (Vdd) = NOT LOW
    #
    # Static power: ZERO. In both states, one transistor is OFF, breaking
    # the current path from Vdd to GND.
    class CMOSInverter
      TRANSISTOR_COUNT = 2

      # @return [CircuitParams] the circuit parameters
      attr_reader :circuit

      # @return [NMOS] the NMOS transistor
      attr_reader :nmos

      # @return [PMOS] the PMOS transistor
      attr_reader :pmos

      # Create a new CMOS inverter.
      #
      # @param circuit_params [CircuitParams, nil] circuit parameters
      # @param nmos_params [MOSFETParams, nil] NMOS transistor parameters
      # @param pmos_params [MOSFETParams, nil] PMOS transistor parameters
      def initialize(circuit_params = nil, nmos_params = nil, pmos_params = nil)
        @circuit = circuit_params || CircuitParams.new
        @nmos = NMOS.new(nmos_params)
        @pmos = PMOS.new(pmos_params)
      end

      # Evaluate the inverter with an analog input voltage.
      #
      # Maps the input voltage through the CMOS transfer characteristic
      # to produce an output voltage.
      #
      # @param input_voltage [Float] Input voltage in volts (0 to Vdd).
      # @return [GateOutput] with voltage, current, power, and timing details.
      def evaluate(input_voltage)
        vdd = @circuit.vdd

        # NMOS: gate = input, source = GND -> Vgs_n = Vin
        vgs_n = input_voltage

        # PMOS: gate = input, source = Vdd -> Vgs_p = Vin - Vdd (negative when input is LOW)
        vgs_p = input_voltage - vdd

        nmos_on = @nmos.conducting?(vgs: vgs_n)
        pmos_on = @pmos.conducting?(vgs: vgs_p)

        # Determine output voltage
        output_v = if pmos_on && !nmos_on
                     vdd # PMOS pulls to Vdd
                   elsif nmos_on && !pmos_on
                     0.0 # NMOS pulls to GND
                   elsif nmos_on && pmos_on
                     # Both on (transition region) — voltage divider
                     vdd / 2.0
                   else
                     # Both off (shouldn't happen in normal operation)
                     vdd / 2.0
                   end

        # Digital interpretation
        logic_value = output_v > vdd / 2.0 ? 1 : 0

        # Current draw: only significant during transition
        current = if nmos_on && pmos_on
                    # Short-circuit current during transition
                    vds_n = vdd / 2.0
                    @nmos.drain_current(vgs: vgs_n, vds: vds_n)
                  else
                    0.0 # Static: no current path
                  end

        power = current * vdd

        # Propagation delay estimate
        c_load = @nmos.params.c_drain + @pmos.params.c_drain
        delay = if current > 0
                  c_load * vdd / (2.0 * current)
                else
                  # Approximate delay using saturation current
                  ids_sat = @nmos.drain_current(vgs: vdd, vds: vdd)
                  ids_sat > 0 ? c_load * vdd / (2.0 * ids_sat) : 1e-9
                end

        GateOutput.new(
          logic_value: logic_value,
          voltage: output_v,
          current_draw: current,
          power_dissipation: power,
          propagation_delay: delay,
          transistor_count: TRANSISTOR_COUNT
        )
      end

      # Evaluate with digital input (0 or 1), returns 0 or 1.
      #
      # Convenience method that maps 0 -> 0V, 1 -> Vdd.
      #
      # @param a [Integer] digital input (0 or 1)
      # @return [Integer] digital output (0 or 1)
      def evaluate_digital(a)
        Transistors.validate_bit(a, "a")
        vin = a == 1 ? @circuit.vdd : 0.0
        evaluate(vin).logic_value
      end

      # Generate the VTC curve: array of [Vin, Vout] points.
      #
      # The VTC shows the sharp switching threshold of CMOS — the output
      # snaps from HIGH to LOW over a very narrow input range.
      #
      # @param steps [Integer] number of points to generate (default 100)
      # @return [Array<Array(Float, Float)>] array of [Vin, Vout] pairs
      def voltage_transfer_characteristic(steps: 100)
        vdd = @circuit.vdd
        (0..steps).map do |i|
          vin = vdd * i / steps.to_f
          result = evaluate(vin)
          [vin, result.voltage]
        end
      end

      # Static power dissipation (ideally ~0 for CMOS).
      #
      # In an ideal CMOS inverter, one transistor is always OFF, so no
      # DC current flows from Vdd to GND.
      #
      # @return [Float] static power in watts
      def static_power
        0.0
      end

      # Dynamic power: P = C_load * Vdd^2 * f.
      #
      # This is the dominant power consumption mechanism in CMOS:
      # every time the output switches, the load capacitance must be
      # charged or discharged.
      #
      # @param frequency [Float] Switching frequency in Hz.
      # @param c_load [Float] Load capacitance in Farads.
      # @return [Float] Dynamic power in Watts.
      def dynamic_power(frequency:, c_load:)
        vdd = @circuit.vdd
        c_load * vdd * vdd * frequency
      end
    end

    # CMOS NAND gate: 2 PMOS parallel + 2 NMOS series = 4 transistors.
    #
    # NAND requires only 4 transistors (2 PMOS + 2 NMOS). AND requires 6
    # (NAND + inverter). This is because the CMOS structure naturally
    # produces an inverted output.
    #
    # Pull-down: NMOS in SERIES — BOTH must be ON to pull output LOW
    # Pull-up: PMOS in PARALLEL — EITHER can pull output HIGH
    class CMOSNand
      TRANSISTOR_COUNT = 4

      # @return [CircuitParams] the circuit parameters
      attr_reader :circuit

      # @return [NMOS] first NMOS transistor
      attr_reader :nmos1

      # @return [NMOS] second NMOS transistor
      attr_reader :nmos2

      # @return [PMOS] first PMOS transistor
      attr_reader :pmos1

      # @return [PMOS] second PMOS transistor
      attr_reader :pmos2

      # Create a new CMOS NAND gate.
      #
      # @param circuit_params [CircuitParams, nil] circuit parameters
      # @param nmos_params [MOSFETParams, nil] NMOS transistor parameters
      # @param pmos_params [MOSFETParams, nil] PMOS transistor parameters
      def initialize(circuit_params = nil, nmos_params = nil, pmos_params = nil)
        @circuit = circuit_params || CircuitParams.new
        @nmos1 = NMOS.new(nmos_params)
        @nmos2 = NMOS.new(nmos_params)
        @pmos1 = PMOS.new(pmos_params)
        @pmos2 = PMOS.new(pmos_params)
      end

      # Evaluate the NAND gate with analog input voltages.
      #
      # @param va [Float] Input A voltage (V).
      # @param vb [Float] Input B voltage (V).
      # @return [GateOutput] with voltage and power details.
      def evaluate(va, vb)
        vdd = @circuit.vdd

        vgs_n1 = va
        vgs_n2 = vb
        vgs_p1 = va - vdd
        vgs_p2 = vb - vdd

        nmos1_on = @nmos1.conducting?(vgs: vgs_n1)
        nmos2_on = @nmos2.conducting?(vgs: vgs_n2)
        pmos1_on = @pmos1.conducting?(vgs: vgs_p1)
        pmos2_on = @pmos2.conducting?(vgs: vgs_p2)

        # Pull-down: NMOS in SERIES — BOTH must be ON
        pulldown_on = nmos1_on && nmos2_on
        # Pull-up: PMOS in PARALLEL — EITHER can pull up
        pullup_on = pmos1_on || pmos2_on

        output_v = if pullup_on && !pulldown_on
                     vdd
                   elsif pulldown_on && !pullup_on
                     0.0
                   else
                     vdd / 2.0
                   end

        logic_value = output_v > vdd / 2.0 ? 1 : 0
        current = (pulldown_on && pullup_on) ? 0.001 : 0.0

        c_load = @nmos1.params.c_drain + @pmos1.params.c_drain
        ids_sat = @nmos1.drain_current(vgs: vdd, vds: vdd)
        delay = ids_sat > 0 ? c_load * vdd / (2.0 * ids_sat) : 1e-9

        GateOutput.new(
          logic_value: logic_value,
          voltage: output_v,
          current_draw: current,
          power_dissipation: current * vdd,
          propagation_delay: delay,
          transistor_count: TRANSISTOR_COUNT
        )
      end

      # Evaluate with digital inputs (0 or 1).
      #
      # @param a [Integer] first digital input (0 or 1)
      # @param b [Integer] second digital input (0 or 1)
      # @return [Integer] digital output (0 or 1)
      def evaluate_digital(a, b)
        Transistors.validate_bit(a, "a")
        Transistors.validate_bit(b, "b")
        vdd = @circuit.vdd
        va = a == 1 ? vdd : 0.0
        vb = b == 1 ? vdd : 0.0
        evaluate(va, vb).logic_value
      end

      # @return [Integer] number of transistors (4)
      def transistor_count
        TRANSISTOR_COUNT
      end
    end

    # CMOS NOR gate: 2 PMOS series + 2 NMOS parallel = 4 transistors.
    #
    # Pull-down: NMOS in PARALLEL — EITHER ON pulls output LOW
    # Pull-up: PMOS in SERIES — BOTH must be ON (both inputs LOW)
    class CMOSNor
      TRANSISTOR_COUNT = 4

      # @return [CircuitParams] the circuit parameters
      attr_reader :circuit

      # @return [NMOS] first NMOS transistor
      attr_reader :nmos1

      # @return [NMOS] second NMOS transistor
      attr_reader :nmos2

      # @return [PMOS] first PMOS transistor
      attr_reader :pmos1

      # @return [PMOS] second PMOS transistor
      attr_reader :pmos2

      # Create a new CMOS NOR gate.
      def initialize(circuit_params = nil, nmos_params = nil, pmos_params = nil)
        @circuit = circuit_params || CircuitParams.new
        @nmos1 = NMOS.new(nmos_params)
        @nmos2 = NMOS.new(nmos_params)
        @pmos1 = PMOS.new(pmos_params)
        @pmos2 = PMOS.new(pmos_params)
      end

      # Evaluate the NOR gate with analog input voltages.
      def evaluate(va, vb)
        vdd = @circuit.vdd

        vgs_n1 = va
        vgs_n2 = vb
        vgs_p1 = va - vdd
        vgs_p2 = vb - vdd

        nmos1_on = @nmos1.conducting?(vgs: vgs_n1)
        nmos2_on = @nmos2.conducting?(vgs: vgs_n2)
        pmos1_on = @pmos1.conducting?(vgs: vgs_p1)
        pmos2_on = @pmos2.conducting?(vgs: vgs_p2)

        # Pull-down: NMOS in PARALLEL — EITHER ON pulls low
        pulldown_on = nmos1_on || nmos2_on
        # Pull-up: PMOS in SERIES — BOTH must be ON
        pullup_on = pmos1_on && pmos2_on

        output_v = if pullup_on && !pulldown_on
                     vdd
                   elsif pulldown_on && !pullup_on
                     0.0
                   else
                     vdd / 2.0
                   end

        logic_value = output_v > vdd / 2.0 ? 1 : 0
        current = (pulldown_on && pullup_on) ? 0.001 : 0.0

        c_load = @nmos1.params.c_drain + @pmos1.params.c_drain
        ids_sat = @nmos1.drain_current(vgs: vdd, vds: vdd)
        delay = ids_sat > 0 ? c_load * vdd / (2.0 * ids_sat) : 1e-9

        GateOutput.new(
          logic_value: logic_value,
          voltage: output_v,
          current_draw: current,
          power_dissipation: current * vdd,
          propagation_delay: delay,
          transistor_count: TRANSISTOR_COUNT
        )
      end

      # Evaluate with digital inputs (0 or 1).
      def evaluate_digital(a, b)
        Transistors.validate_bit(a, "a")
        Transistors.validate_bit(b, "b")
        vdd = @circuit.vdd
        va = a == 1 ? vdd : 0.0
        vb = b == 1 ? vdd : 0.0
        evaluate(va, vb).logic_value
      end
    end

    # CMOS AND gate: NAND + Inverter = 6 transistors.
    #
    # There is no "direct" CMOS AND gate. The CMOS topology naturally
    # produces inverted outputs (NAND, NOR), so to get AND we must add
    # an inverter after the NAND.
    class CMOSAnd
      TRANSISTOR_COUNT = 6

      # @return [CircuitParams] the circuit parameters
      attr_reader :circuit

      # Create a new CMOS AND gate.
      def initialize(circuit_params = nil)
        @circuit = circuit_params || CircuitParams.new
        @nand = CMOSNand.new(circuit_params)
        @inv = CMOSInverter.new(circuit_params)
      end

      # Evaluate AND = NOT(NAND(A, B)).
      def evaluate(va, vb)
        nand_out = @nand.evaluate(va, vb)
        inv_out = @inv.evaluate(nand_out.voltage)
        GateOutput.new(
          logic_value: inv_out.logic_value,
          voltage: inv_out.voltage,
          current_draw: nand_out.current_draw + inv_out.current_draw,
          power_dissipation: nand_out.power_dissipation + inv_out.power_dissipation,
          propagation_delay: nand_out.propagation_delay + inv_out.propagation_delay,
          transistor_count: TRANSISTOR_COUNT
        )
      end

      # Evaluate with digital inputs.
      def evaluate_digital(a, b)
        Transistors.validate_bit(a, "a")
        Transistors.validate_bit(b, "b")
        vdd = @circuit.vdd
        va = a == 1 ? vdd : 0.0
        vb = b == 1 ? vdd : 0.0
        evaluate(va, vb).logic_value
      end
    end

    # CMOS OR gate: NOR + Inverter = 6 transistors.
    class CMOSOr
      TRANSISTOR_COUNT = 6

      # @return [CircuitParams] the circuit parameters
      attr_reader :circuit

      # Create a new CMOS OR gate.
      def initialize(circuit_params = nil)
        @circuit = circuit_params || CircuitParams.new
        @nor = CMOSNor.new(circuit_params)
        @inv = CMOSInverter.new(circuit_params)
      end

      # Evaluate OR = NOT(NOR(A, B)).
      def evaluate(va, vb)
        nor_out = @nor.evaluate(va, vb)
        inv_out = @inv.evaluate(nor_out.voltage)
        GateOutput.new(
          logic_value: inv_out.logic_value,
          voltage: inv_out.voltage,
          current_draw: nor_out.current_draw + inv_out.current_draw,
          power_dissipation: nor_out.power_dissipation + inv_out.power_dissipation,
          propagation_delay: nor_out.propagation_delay + inv_out.propagation_delay,
          transistor_count: TRANSISTOR_COUNT
        )
      end

      # Evaluate with digital inputs.
      def evaluate_digital(a, b)
        Transistors.validate_bit(a, "a")
        Transistors.validate_bit(b, "b")
        vdd = @circuit.vdd
        va = a == 1 ? vdd : 0.0
        vb = b == 1 ? vdd : 0.0
        evaluate(va, vb).logic_value
      end
    end

    # CMOS XOR gate using 4-NAND construction = 6 transistors.
    #
    # XOR(A, B) = NAND(NAND(A, NAND(A,B)), NAND(B, NAND(A,B)))
    #
    # This construction proves that XOR can be built from the universal
    # NAND gate alone, which in turn is built from just 4 transistors.
    class CMOSXor
      TRANSISTOR_COUNT = 6

      # @return [CircuitParams] the circuit parameters
      attr_reader :circuit

      # Create a new CMOS XOR gate.
      def initialize(circuit_params = nil)
        @circuit = circuit_params || CircuitParams.new
        @nand1 = CMOSNand.new(circuit_params)
        @nand2 = CMOSNand.new(circuit_params)
        @nand3 = CMOSNand.new(circuit_params)
        @nand4 = CMOSNand.new(circuit_params)
      end

      # Evaluate XOR using 4 NAND gates.
      def evaluate(va, vb)
        vdd = @circuit.vdd

        # Step 1: NAND(A, B)
        nand_ab = @nand1.evaluate(va, vb)
        # Step 2: NAND(A, NAND(A,B))
        nand_a_nab = @nand2.evaluate(va, nand_ab.voltage)
        # Step 3: NAND(B, NAND(A,B))
        nand_b_nab = @nand3.evaluate(vb, nand_ab.voltage)
        # Step 4: NAND(step2, step3)
        result = @nand4.evaluate(nand_a_nab.voltage, nand_b_nab.voltage)

        total_current = nand_ab.current_draw +
          nand_a_nab.current_draw +
          nand_b_nab.current_draw +
          result.current_draw
        total_delay = nand_ab.propagation_delay +
          [nand_a_nab.propagation_delay, nand_b_nab.propagation_delay].max +
          result.propagation_delay

        GateOutput.new(
          logic_value: result.logic_value,
          voltage: result.voltage,
          current_draw: total_current,
          power_dissipation: total_current * vdd,
          propagation_delay: total_delay,
          transistor_count: TRANSISTOR_COUNT
        )
      end

      # Evaluate with digital inputs.
      def evaluate_digital(a, b)
        Transistors.validate_bit(a, "a")
        Transistors.validate_bit(b, "b")
        vdd = @circuit.vdd
        va = a == 1 ? vdd : 0.0
        vb = b == 1 ? vdd : 0.0
        evaluate(va, vb).logic_value
      end

      # Build XOR from 4 NAND gates to demonstrate universality.
      #
      # This is the same as evaluate_digital but makes the NAND
      # construction explicit for educational purposes.
      #
      # @param a [Integer] first digital input (0 or 1)
      # @param b [Integer] second digital input (0 or 1)
      # @return [Integer] digital output (0 or 1)
      def evaluate_from_nands(a, b)
        evaluate_digital(a, b)
      end
    end
  end
end
