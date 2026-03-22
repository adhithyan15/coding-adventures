# frozen_string_literal: true

# MOSFET Transistors — the building blocks of modern digital circuits.
#
# === What is a MOSFET? ===
#
# MOSFET stands for Metal-Oxide-Semiconductor Field-Effect Transistor. It is
# the most common type of transistor in the world — every CPU, GPU, and phone
# chip is built from billions of MOSFETs.
#
# A MOSFET has three terminals:
#     Gate (G):   The control terminal. Voltage here controls the switch.
#     Drain (D):  Current flows IN here (for NMOS) or OUT here (for PMOS).
#     Source (S): Current flows OUT here (for NMOS) or IN here (for PMOS).
#
# The key insight: a MOSFET is VOLTAGE-controlled. Applying a voltage to the
# gate creates an electric field that either allows or blocks current flow
# between drain and source. No current flows into the gate itself (it's
# insulated by a thin oxide layer), which means:
#     - Near-zero input power consumption
#     - Very high input impedance (good for amplifiers)
#     - Can be packed extremely densely on a chip
#
# === NMOS vs PMOS ===
#
# The two MOSFET types are complementary — they turn on under opposite conditions:
#
#     NMOS: Gate HIGH -> ON  (conducts drain to source)
#     PMOS: Gate LOW  -> ON  (conducts source to drain)
#
# This complementary behavior is the foundation of CMOS (Complementary MOS)
# logic. By pairing NMOS and PMOS transistors, we can build gates that consume
# near-zero power in steady state — only burning energy during transitions.
#
# === The Three Operating Regions ===
#
# A MOSFET operates in one of three regions depending on the voltages at
# its terminals:
#
#     1. CUTOFF:     Vgs < Vth -> transistor is OFF, no current flows.
#                    Used as the OFF state in digital logic.
#
#     2. LINEAR:     Vgs > Vth, Vds < (Vgs - Vth) -> acts like a variable
#                    resistor. Current is proportional to both Vgs and Vds.
#                    Used as the ON state in digital logic (deep linear region).
#
#     3. SATURATION: Vgs > Vth, Vds >= (Vgs - Vth) -> current is roughly
#                    constant regardless of Vds. Used for analog amplifiers.

module CodingAdventures
  module Transistors
    # N-channel MOSFET transistor.
    #
    # An NMOS transistor conducts current from drain to source when the gate
    # voltage exceeds the threshold voltage (Vgs > Vth). Think of it as a
    # normally-OPEN switch that CLOSES when you apply voltage to the gate.
    #
    # === Water analogy ===
    #
    #     Imagine a water pipe with an electrically-controlled valve:
    #
    #         Water pressure (Vdd) --> [VALVE] --> Water out (Vss/ground)
    #                                    ^
    #                                Gate voltage
    #
    #     - Gate voltage HIGH: valve opens, water flows (current flows D->S)
    #     - Gate voltage LOW:  valve closed, water blocked (no current)
    #     - Gate voltage MEDIUM: valve partially open (analog amplifier mode)
    #
    # === In a digital circuit ===
    #
    #     When used as a digital switch, NMOS connects the output to GROUND:
    #
    #         Output --+
    #                  | NMOS (gate = input signal)
    #                  |
    #                 GND
    #
    #     Input HIGH -> NMOS ON -> output pulled to GND (LOW)
    #     Input LOW  -> NMOS OFF -> output disconnected from GND
    class NMOS
      # @return [MOSFETParams] the electrical parameters for this transistor
      attr_reader :params

      # Create a new NMOS transistor with the given parameters.
      #
      # @param params [MOSFETParams, nil] electrical parameters. If nil,
      #   defaults to a typical 180nm CMOS process.
      def initialize(params = nil)
        @params = params || MOSFETParams.new
      end

      # Determine the operating region given terminal voltages.
      #
      # The operating region determines which equations govern current flow.
      # For NMOS:
      #     Cutoff:     Vgs < Vth            (gate voltage below threshold)
      #     Linear:     Vgs >= Vth AND Vds < Vgs - Vth
      #     Saturation: Vgs >= Vth AND Vds >= Vgs - Vth
      #
      # @param vgs [Float] Gate-to-Source voltage (V). Positive turns NMOS on.
      # @param vds [Float] Drain-to-Source voltage (V). Positive for normal operation.
      # @return [String] one of MOSFETRegion::CUTOFF, LINEAR, or SATURATION
      #
      # @example
      #   t = NMOS.new
      #   t.region(vgs: 0.0, vds: 1.0)   # => MOSFETRegion::CUTOFF
      #   t.region(vgs: 1.5, vds: 0.1)   # => MOSFETRegion::LINEAR
      #   t.region(vgs: 1.0, vds: 3.0)   # => MOSFETRegion::SATURATION
      def region(vgs:, vds:)
        vth = @params.vth

        return MOSFETRegion::CUTOFF if vgs < vth

        vov = vgs - vth # Overdrive voltage
        return MOSFETRegion::LINEAR if vds < vov

        MOSFETRegion::SATURATION
      end

      # Calculate drain-to-source current (Ids) in amperes.
      #
      # Uses the simplified MOSFET current equations (Shockley model):
      #
      #     Cutoff (Vgs < Vth):
      #         Ids = 0
      #         No channel exists, no current flows.
      #
      #     Linear (Vgs >= Vth, Vds < Vgs - Vth):
      #         Ids = k * ((Vgs - Vth) * Vds - 0.5 * Vds^2)
      #         The transistor acts like a voltage-controlled resistor.
      #
      #     Saturation (Vgs >= Vth, Vds >= Vgs - Vth):
      #         Ids = 0.5 * k * (Vgs - Vth)^2
      #         The channel is "pinched off" at the drain end.
      #         Current depends only on Vgs, not Vds.
      #
      # @param vgs [Float] Gate-to-Source voltage (V).
      # @param vds [Float] Drain-to-Source voltage (V).
      # @return [Float] Drain current in amperes. Always >= 0 for NMOS.
      def drain_current(vgs:, vds:)
        r = region(vgs: vgs, vds: vds)
        k = @params.k
        vth = @params.vth

        return 0.0 if r == MOSFETRegion::CUTOFF

        vov = vgs - vth # Overdrive voltage

        if r == MOSFETRegion::LINEAR
          # Linear/ohmic region: Ids = k * ((Vgs-Vth)*Vds - 0.5*Vds^2)
          k * (vov * vds - 0.5 * vds * vds)
        else
          # Saturation region: Ids = 0.5 * k * (Vgs-Vth)^2
          0.5 * k * vov * vov
        end
      end

      # Digital abstraction: is this transistor ON?
      #
      # Returns true when the gate voltage exceeds the threshold voltage.
      # This is the simplified view used in digital circuit analysis —
      # the transistor is either fully ON or fully OFF, with no in-between.
      #
      # @param vgs [Float] Gate-to-Source voltage (V).
      # @return [Boolean] true if Vgs >= Vth (transistor is ON).
      def conducting?(vgs:)
        vgs >= @params.vth
      end

      # Output voltage when used as a pull-down switch.
      #
      # In a CMOS circuit, NMOS transistors form the pull-down network
      # (connecting output to ground). When the NMOS is ON, it pulls
      # the output to ~0V. When OFF, the output floats (determined by
      # the pull-up network).
      #
      # @param vgs [Float] Gate-to-Source voltage (V).
      # @param vdd [Float] Supply voltage (V).
      # @return [Float] Output voltage in volts.
      def output_voltage(vgs:, vdd:)
        conducting?(vgs: vgs) ? 0.0 : vdd
      end

      # Calculate small-signal transconductance gm.
      #
      # Transconductance is the key parameter for amplifier design.
      # It tells you how much the output current changes per unit
      # change in input voltage:
      #
      #     gm = dIds / dVgs
      #
      # In saturation:
      #     gm = k * (Vgs - Vth)
      #
      # Higher gm = more gain, but also more power consumption.
      #
      # @param vgs [Float] Gate-to-Source voltage (V).
      # @param vds [Float] Drain-to-Source voltage (V).
      # @return [Float] Transconductance in Siemens (A/V). Returns 0.0 in cutoff.
      def transconductance(vgs:, vds:)
        r = region(vgs: vgs, vds: vds)
        return 0.0 if r == MOSFETRegion::CUTOFF

        vov = vgs - @params.vth
        @params.k * vov
      end
    end

    # P-channel MOSFET transistor.
    #
    # A PMOS transistor is the complement of NMOS. It conducts current from
    # source to drain when the gate voltage is LOW (below the source voltage
    # by more than |Vth|). Think of it as a normally-CLOSED switch that OPENS
    # when you apply voltage.
    #
    # === Why PMOS matters ===
    #
    # PMOS transistors form the pull-UP network in CMOS gates. When we need
    # to connect the output to Vdd (logic HIGH), PMOS transistors do the job:
    #
    #     Vdd
    #      |
    #      | PMOS (gate = input signal)
    #      +
    #      |
    #     Output
    #
    #     Input LOW  -> PMOS ON -> output pulled to Vdd (HIGH)
    #     Input HIGH -> PMOS OFF -> output disconnected from Vdd
    #
    # === NMOS vs PMOS symmetry ===
    #
    # PMOS uses the same equations as NMOS, but with reversed voltage
    # polarities. For PMOS, Vgs and Vds are typically negative (because
    # the source is connected to Vdd, the highest voltage in the circuit).
    #
    # In this implementation, we handle the sign conventions internally
    # by using absolute values.
    class PMOS
      # @return [MOSFETParams] the electrical parameters for this transistor
      attr_reader :params

      # Create a new PMOS transistor with the given parameters.
      #
      # @param params [MOSFETParams, nil] electrical parameters.
      def initialize(params = nil)
        @params = params || MOSFETParams.new
      end

      # Determine operating region for PMOS.
      #
      # For PMOS, we use the magnitudes of Vgs and Vds (which are typically
      # negative in a circuit). The regions are:
      #
      #     Cutoff:     |Vgs| < Vth
      #     Linear:     |Vgs| >= Vth AND |Vds| < |Vgs| - Vth
      #     Saturation: |Vgs| >= Vth AND |Vds| >= |Vgs| - Vth
      #
      # @param vgs [Float] Gate-to-Source voltage (V). Typically negative for PMOS.
      # @param vds [Float] Drain-to-Source voltage (V). Typically negative for PMOS.
      # @return [String] one of MOSFETRegion::CUTOFF, LINEAR, or SATURATION
      def region(vgs:, vds:)
        vth = @params.vth
        abs_vgs = vgs.abs
        abs_vds = vds.abs

        return MOSFETRegion::CUTOFF if abs_vgs < vth

        vov = abs_vgs - vth
        return MOSFETRegion::LINEAR if abs_vds < vov

        MOSFETRegion::SATURATION
      end

      # Calculate source-to-drain current for PMOS.
      #
      # Same equations as NMOS but using absolute values of voltages.
      # Current magnitude is returned (always >= 0).
      #
      # @param vgs [Float] Gate-to-Source voltage (V).
      # @param vds [Float] Drain-to-Source voltage (V).
      # @return [Float] Current magnitude in amperes.
      def drain_current(vgs:, vds:)
        r = region(vgs: vgs, vds: vds)
        k = @params.k
        vth = @params.vth

        return 0.0 if r == MOSFETRegion::CUTOFF

        abs_vgs = vgs.abs
        abs_vds = vds.abs
        vov = abs_vgs - vth

        if r == MOSFETRegion::LINEAR
          k * (vov * abs_vds - 0.5 * abs_vds * abs_vds)
        else
          0.5 * k * vov * vov
        end
      end

      # Digital abstraction: is this PMOS transistor ON?
      #
      # PMOS turns ON when Vgs is sufficiently negative (gate pulled
      # below the source). Returns true when |Vgs| >= Vth.
      #
      # @param vgs [Float] Gate-to-Source voltage (V). Typically negative for PMOS.
      # @return [Boolean] true if |Vgs| >= Vth.
      def conducting?(vgs:)
        vgs.abs >= @params.vth
      end

      # Output voltage when used as a pull-up switch.
      #
      # PMOS forms the pull-up network in CMOS:
      #     ON:  output ~ Vdd
      #     OFF: output ~ 0V (pulled down by NMOS network)
      #
      # @param vgs [Float] Gate-to-Source voltage (V).
      # @param vdd [Float] Supply voltage (V).
      # @return [Float] Output voltage in volts.
      def output_voltage(vgs:, vdd:)
        conducting?(vgs: vgs) ? vdd : 0.0
      end

      # Calculate small-signal transconductance gm for PMOS.
      #
      # Same formula as NMOS but using absolute values.
      #
      # @param vgs [Float] Gate-to-Source voltage (V).
      # @param vds [Float] Drain-to-Source voltage (V).
      # @return [Float] Transconductance in Siemens (A/V).
      def transconductance(vgs:, vds:)
        r = region(vgs: vgs, vds: vds)
        return 0.0 if r == MOSFETRegion::CUTOFF

        vov = vgs.abs - @params.vth
        @params.k * vov
      end
    end
  end
end
