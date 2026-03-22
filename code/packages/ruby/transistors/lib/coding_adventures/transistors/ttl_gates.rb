# frozen_string_literal: true

# TTL Logic Gates — historical BJT-based digital logic.
#
# === What is TTL? ===
#
# TTL stands for Transistor-Transistor Logic. It was the dominant digital
# logic family from the mid-1960s through the 1980s, when CMOS replaced it.
# The "7400 series" — a family of TTL chips — defined the standard logic
# gates that every digital system used.
#
# === Why TTL Lost to CMOS ===
#
# TTL's fatal flaw: STATIC POWER CONSUMPTION.
#
# In a TTL gate, current flows through resistors and transistors even when
# the gate is doing nothing. A single TTL NAND gate dissipates ~1-10 mW
# at rest. That may sound small, but:
#
#     1 million gates * 10 mW/gate = 10,000 watts
#
# That's a space heater, not a computer chip!
#
# === RTL: The Predecessor to TTL ===
#
# Before TTL came RTL (Resistor-Transistor Logic), the simplest possible
# transistor logic. An RTL inverter is just one transistor with two resistors.
# It was slow and power-hungry, but it was used in the Apollo Guidance
# Computer that landed humans on the moon in 1969.

module CodingAdventures
  module Transistors
    # TTL NAND gate using NPN transistors (7400-series style).
    #
    # Uses 3 NPN transistors and a 4kohm pull-up resistor.
    #
    # === Operation ===
    #
    # Any input LOW:
    #     Q1's base-emitter junction forward-biases through the LOW input,
    #     stealing current from Q2's base -> Q2 and Q3 turn OFF ->
    #     output pulled HIGH through pull-up resistor.
    #
    # ALL inputs HIGH:
    #     Q1's base-collector junction forward-biases, driving current
    #     into Q2's base -> Q2 saturates -> Q3 saturates ->
    #     output pulled LOW (Vce_sat ~ 0.2V).
    class TTLNand
      # @return [Float] supply voltage
      attr_reader :vcc

      # @return [BJTParams] BJT parameters
      attr_reader :params

      # @return [Float] pull-up resistor value in ohms
      attr_reader :r_pullup

      # Create a new TTL NAND gate.
      #
      # @param vcc [Float] supply voltage (default 5.0V)
      # @param bjt_params [BJTParams, nil] BJT parameters
      def initialize(vcc: 5.0, bjt_params: nil)
        @vcc = vcc
        @params = bjt_params || BJTParams.new
        @r_pullup = 4000.0 # 4kohm pull-up resistor
        @q1 = NPN.new(@params)
        @q2 = NPN.new(@params)
        @q3 = NPN.new(@params)
      end

      # Evaluate the TTL NAND gate with analog input voltages.
      #
      # @param va [Float] Input A voltage (V). LOW < 0.8V, HIGH > 2.0V.
      # @param vb [Float] Input B voltage (V).
      # @return [GateOutput] with voltage and power details.
      def evaluate(va, vb)
        vbe_on = @params.vbe_on

        # TTL input thresholds
        a_high = va > 2.0
        b_high = vb > 2.0

        if a_high && b_high
          # ALL inputs HIGH -> output LOW
          output_v = @params.vce_sat # ~0.2V
          logic_value = 0

          # Static current through resistor chain
          current = (@vcc - 2 * vbe_on - @params.vce_sat) / @r_pullup
          current = [current, 0.0].max
        else
          # At least one input LOW -> output HIGH
          output_v = @vcc - vbe_on # ~4.3V
          logic_value = 1

          # Small bias current through pull-up
          current = (@vcc - output_v) / @r_pullup
          current = [current, 0.0].max
        end

        power = current * @vcc

        # TTL propagation delay: typically 5-15 ns
        delay = 10e-9 # 10 ns typical

        GateOutput.new(
          logic_value: logic_value,
          voltage: output_v,
          current_draw: current,
          power_dissipation: power,
          propagation_delay: delay,
          transistor_count: 3 # Q1 + Q2 + Q3
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
        va = a == 1 ? @vcc : 0.0
        vb = b == 1 ? @vcc : 0.0
        evaluate(va, vb).logic_value
      end

      # Static power dissipation — significantly higher than CMOS.
      #
      # TTL gates consume power continuously due to the resistor-based
      # biasing. The worst case is when the output is LOW (all inputs HIGH).
      #
      # @return [Float] Static power in watts. Typically ~1-10 mW.
      def static_power
        current = (@vcc - 2 * @params.vbe_on - @params.vce_sat) / @r_pullup
        [current, 0.0].max * @vcc
      end
    end

    # Resistor-Transistor Logic inverter — the earliest IC logic family.
    #
    # === Circuit Diagram ===
    #
    #         Vcc
    #          |
    #         Rc (collector resistor, ~1kohm)
    #          |
    #     +----+----+
    #     |  Q1     |     Single NPN transistor
    #     |  (NPN)  |
    #     +----+----+
    #          |
    #         GND
    #
    #     Input ---- Rb (base resistor, ~10kohm) ---- Base of Q1
    #
    # === Historical Note ===
    #
    # RTL was used in the Apollo Guidance Computer (AGC), which navigated
    # Apollo 11 to the moon in 1969.
    class RTLInverter
      # @return [Float] supply voltage
      attr_reader :vcc

      # @return [Float] base resistor value in ohms
      attr_reader :r_base

      # @return [Float] collector resistor value in ohms
      attr_reader :r_collector

      # @return [BJTParams] BJT parameters
      attr_reader :params

      # Create a new RTL inverter.
      #
      # @param vcc [Float] supply voltage (default 5.0V)
      # @param r_base [Float] base resistor in ohms (default 10_000)
      # @param r_collector [Float] collector resistor in ohms (default 1_000)
      # @param bjt_params [BJTParams, nil] BJT parameters
      def initialize(vcc: 5.0, r_base: 10_000.0, r_collector: 1_000.0, bjt_params: nil)
        @vcc = vcc
        @r_base = r_base
        @r_collector = r_collector
        @params = bjt_params || BJTParams.new
        @q1 = NPN.new(@params)
      end

      # Evaluate the RTL inverter with an analog input voltage.
      #
      # @param v_input [Float] Input voltage (V).
      # @return [GateOutput] with voltage and power details.
      def evaluate(v_input)
        vbe_on = @params.vbe_on

        if v_input > vbe_on
          ib = (v_input - vbe_on) / @r_base
          # Q1 is ON — check if saturated
          ic = [ib * @params.beta, (@vcc - @params.vce_sat) / @r_collector].min
          output_v = @vcc - ic * @r_collector
          output_v = [output_v, @params.vce_sat].max
          logic_value = output_v < @vcc / 2.0 ? 0 : 1
          current = ic + ib
        else
          # Q1 is OFF — output pulled to Vcc through Rc
          output_v = @vcc
          logic_value = 1
          current = 0.0
        end

        power = current * @vcc
        delay = 50e-9 # RTL is slow: ~50 ns typical

        GateOutput.new(
          logic_value: logic_value,
          voltage: output_v,
          current_draw: current,
          power_dissipation: power,
          propagation_delay: delay,
          transistor_count: 1
        )
      end

      # Evaluate with digital input (0 or 1).
      #
      # @param a [Integer] digital input (0 or 1)
      # @return [Integer] digital output (0 or 1)
      def evaluate_digital(a)
        Transistors.validate_bit(a, "a")
        v_input = a == 1 ? @vcc : 0.0
        evaluate(v_input).logic_value
      end
    end
  end
end
