# frozen_string_literal: true

# BJT Transistors — the original solid-state amplifier.
#
# === What is a BJT? ===
#
# BJT stands for Bipolar Junction Transistor. Invented in 1947 at Bell Labs
# by John Bardeen, Walter Brattain, and William Shockley, the BJT replaced
# vacuum tubes and launched the electronics revolution. Before the BJT,
# computers filled entire rooms with thousands of hot, unreliable vacuum tubes.
# After the BJT, they could be shrunk to a desk — and eventually to a pocket.
#
# A BJT has three terminals:
#     Base (B):      The control terminal. Current here controls the switch.
#     Collector (C): Current flows IN here (for NPN) or OUT here (for PNP).
#     Emitter (E):   Current flows OUT here (for NPN) or IN here (for PNP).
#
# The key difference from MOSFETs: a BJT is CURRENT-controlled. You must
# supply a continuous current to the base to keep it on. This means:
#     - Base current = wasted power (even in steady state)
#     - Lower input impedance than MOSFETs
#     - But historically faster switching (before CMOS caught up)
#
# === NPN vs PNP ===
#
#     NPN: Base current flows B->E. Collector current flows C->E.
#          "Current flows IN to the base to turn it ON."
#
#     PNP: Base current flows E->B. Collector current flows E->C.
#          "Current flows OUT of the base to turn it ON."
#
# === The Current Gain (beta) ===
#
# The magic of the BJT is current amplification:
#
#     Ic = beta * Ib
#
# A tiny base current (microamps) controls a much larger collector current
# (milliamps). Beta (also called hfe) is typically 50-300 for small-signal
# transistors.
#
# === Why CMOS Replaced BJT for Digital Logic ===
#
# In TTL (Transistor-Transistor Logic), the dominant BJT logic family:
#     - Static power: ~1-10 mW per gate (base current always flows)
#     - A chip with 1 million gates would consume 1-10 kW just sitting idle!
#
# In CMOS:
#     - Static power: ~nanowatts per gate (no DC current path)
#     - A chip with 1 billion gates consumes milliwatts in idle

module CodingAdventures
  module Transistors
    # NPN bipolar junction transistor.
    #
    # An NPN transistor turns ON when current flows into the base terminal
    # (Vbe > ~0.7V). A small base current controls a much larger collector
    # current through the current gain relationship: Ic = beta * Ib.
    #
    # === Operating regions ===
    #
    #     CUTOFF:      Vbe < 0.7V -> no base current -> no collector current.
    #
    #     ACTIVE:      Vbe ~ 0.7V, Vce > 0.2V -> Ic = beta * Ib.
    #                  The transistor is a LINEAR AMPLIFIER.
    #
    #     SATURATION:  Vbe ~ 0.7V, Vce ~ 0.2V -> transistor is fully ON.
    #                  Collector current is limited by the external circuit.
    class NPN
      # @return [BJTParams] the electrical parameters for this transistor
      attr_reader :params

      # Create a new NPN transistor with the given parameters.
      #
      # @param params [BJTParams, nil] electrical parameters. If nil,
      #   defaults to a typical 2N2222 small-signal NPN.
      def initialize(params = nil)
        @params = params || BJTParams.new
      end

      # Determine the operating region from terminal voltages.
      #
      # @param vbe [Float] Base-to-Emitter voltage (V). Must exceed ~0.7V to turn on.
      # @param vce [Float] Collector-to-Emitter voltage (V).
      # @return [String] one of BJTRegion::CUTOFF, ACTIVE, or SATURATION
      def region(vbe:, vce:)
        return BJTRegion::CUTOFF if vbe < @params.vbe_on
        return BJTRegion::SATURATION if vce <= @params.vce_sat

        BJTRegion::ACTIVE
      end

      # Calculate collector current (Ic) in amperes.
      #
      # The collector current depends on the operating region:
      #
      #     Cutoff:
      #         Ic = 0. No base current, no collector current.
      #
      #     Active/Saturation:
      #         Ic = Is * (exp(Vbe / Vt) - 1)
      #         where Vt = kT/q ~ 26mV at room temperature.
      #         The exponential is clamped to prevent overflow.
      #
      # @param vbe [Float] Base-to-Emitter voltage (V).
      # @param vce [Float] Collector-to-Emitter voltage (V).
      # @return [Float] Collector current in amperes.
      def collector_current(vbe:, vce:)
        r = region(vbe: vbe, vce: vce)
        return 0.0 if r == BJTRegion::CUTOFF

        # Thermal voltage: Vt = kT/q ~ 26mV at room temperature
        vt = 0.026

        # Ebers-Moll model (simplified):
        # Ic = Is * (exp(Vbe/Vt) - 1)
        exponent = [vbe / vt, 40.0].min # Clamp to prevent overflow
        @params.is_ * (Math.exp(exponent) - 1.0)
      end

      # Calculate base current (Ib) in amperes.
      #
      # Ib = Ic / beta in the active region.
      #
      # This is the "wasted" current that makes BJTs less efficient than
      # MOSFETs for digital logic.
      #
      # @param vbe [Float] Base-to-Emitter voltage (V).
      # @param vce [Float] Collector-to-Emitter voltage (V).
      # @return [Float] Base current in amperes.
      def base_current(vbe:, vce:)
        ic = collector_current(vbe: vbe, vce: vce)
        return 0.0 if ic == 0.0

        ic / @params.beta
      end

      # Digital abstraction: is this transistor ON?
      #
      # Returns true when Vbe >= Vbe_on (typically 0.7V).
      #
      # @param vbe [Float] Base-to-Emitter voltage (V).
      # @return [Boolean] true if conducting.
      def conducting?(vbe:)
        vbe >= @params.vbe_on
      end

      # Calculate small-signal transconductance gm.
      #
      # For a BJT in the active region:
      #     gm = Ic / Vt
      #
      # BJTs typically have higher gm than MOSFETs for the same current.
      #
      # @param vbe [Float] Base-to-Emitter voltage (V).
      # @param vce [Float] Collector-to-Emitter voltage (V).
      # @return [Float] Transconductance in Siemens (A/V).
      def transconductance(vbe:, vce:)
        ic = collector_current(vbe: vbe, vce: vce)
        return 0.0 if ic == 0.0

        vt = 0.026
        ic / vt
      end
    end

    # PNP bipolar junction transistor.
    #
    # The complement of NPN. A PNP transistor turns ON when the base is
    # pulled LOW relative to the emitter (Veb > 0.7V, equivalently
    # Vbe < -0.7V in our convention). Current flows from emitter to collector.
    #
    # === Voltage conventions ===
    #
    # For PNP, the "natural" voltages are reversed from NPN:
    # - Vbe is typically NEGATIVE (base below emitter)
    # - Vce is typically NEGATIVE (collector below emitter)
    #
    # We use absolute values internally, same as PMOS.
    class PNP
      # @return [BJTParams] the electrical parameters for this transistor
      attr_reader :params

      # Create a new PNP transistor with the given parameters.
      #
      # @param params [BJTParams, nil] electrical parameters.
      def initialize(params = nil)
        @params = params || BJTParams.new
      end

      # Determine operating region for PNP.
      #
      # Uses absolute values of Vbe and Vce since PNP operates with
      # reversed polarities.
      #
      # @param vbe [Float] Base-to-Emitter voltage (V). Typically negative.
      # @param vce [Float] Collector-to-Emitter voltage (V). Typically negative.
      # @return [String] one of BJTRegion::CUTOFF, ACTIVE, or SATURATION
      def region(vbe:, vce:)
        abs_vbe = vbe.abs
        abs_vce = vce.abs

        return BJTRegion::CUTOFF if abs_vbe < @params.vbe_on
        return BJTRegion::SATURATION if abs_vce <= @params.vce_sat

        BJTRegion::ACTIVE
      end

      # Calculate collector current magnitude for PNP.
      #
      # Same equations as NPN but using absolute values.
      # Returns current magnitude (always >= 0).
      #
      # @param vbe [Float] Base-to-Emitter voltage (V).
      # @param vce [Float] Collector-to-Emitter voltage (V).
      # @return [Float] Collector current magnitude in amperes.
      def collector_current(vbe:, vce:)
        r = region(vbe: vbe, vce: vce)
        return 0.0 if r == BJTRegion::CUTOFF

        abs_vbe = vbe.abs
        vt = 0.026

        exponent = [abs_vbe / vt, 40.0].min
        @params.is_ * (Math.exp(exponent) - 1.0)
      end

      # Calculate base current magnitude for PNP.
      #
      # @param vbe [Float] Base-to-Emitter voltage (V).
      # @param vce [Float] Collector-to-Emitter voltage (V).
      # @return [Float] Base current magnitude in amperes.
      def base_current(vbe:, vce:)
        ic = collector_current(vbe: vbe, vce: vce)
        return 0.0 if ic == 0.0

        ic / @params.beta
      end

      # Digital abstraction: is this PNP transistor ON?
      #
      # PNP turns ON when |Vbe| >= Vbe_on (base pulled below emitter).
      #
      # @param vbe [Float] Base-to-Emitter voltage (V). Typically negative.
      # @return [Boolean] true if |Vbe| >= Vbe_on.
      def conducting?(vbe:)
        vbe.abs >= @params.vbe_on
      end

      # Calculate small-signal transconductance gm for PNP.
      #
      # @param vbe [Float] Base-to-Emitter voltage (V).
      # @param vce [Float] Collector-to-Emitter voltage (V).
      # @return [Float] Transconductance in Siemens (A/V).
      def transconductance(vbe:, vce:)
        ic = collector_current(vbe: vbe, vce: vce)
        return 0.0 if ic == 0.0

        vt = 0.026
        ic / vt
      end
    end
  end
end
