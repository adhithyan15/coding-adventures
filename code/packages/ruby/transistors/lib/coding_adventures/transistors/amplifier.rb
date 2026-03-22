# frozen_string_literal: true

# Analog Amplifier Analysis — transistors as signal amplifiers.
#
# === Beyond Digital: Transistors as Amplifiers ===
#
# A transistor used as a digital switch operates in only two states: ON and OFF.
# But transistors are fundamentally ANALOG devices. When biased in the right
# operating region (saturation for MOSFET, active for BJT), they can amplify
# small signals into larger ones.
#
# === Common-Source Amplifier (MOSFET) ===
#
# The most basic MOSFET amplifier. The input signal modulates Vgs, which
# modulates Ids (via transconductance gm), which creates a voltage drop
# across the drain resistor Rd:
#
#     Voltage gain: Av = -gm * Rd
#     The negative sign means it's an INVERTING amplifier.
#
# === Common-Emitter Amplifier (BJT) ===
#
# The BJT equivalent. Input signal modulates Vbe, which modulates Ic
# (via transconductance gm = Ic/Vt), which creates a voltage drop
# across the collector resistor Rc:
#
#     Voltage gain: Av = -gm * Rc = -(Ic/Vt) * Rc

module CodingAdventures
  module Transistors
    # Amplifier analysis module methods.
    #
    # These are module-level methods that analyze transistor amplifier
    # configurations and return AmplifierAnalysis results.
    module Amplifier
      module_function

      # Analyze an NMOS common-source amplifier configuration.
      #
      # The common-source amplifier is the most basic MOSFET amplifier topology.
      # The input signal is applied to the gate, and the output is taken from
      # the drain.
      #
      # @param transistor [NMOS] NMOS transistor instance
      # @param vgs [Float] DC gate-to-source bias voltage (V)
      # @param vdd [Float] Supply voltage (V)
      # @param r_drain [Float] Drain resistor value (ohms)
      # @param c_load [Float] Output load capacitance (F). Default 1 pF.
      # @return [AmplifierAnalysis] with gain, impedance, and bandwidth
      def analyze_common_source_amp(transistor, vgs:, vdd:, r_drain:, c_load: 1e-12)
        # Calculate DC operating point
        ids = transistor.drain_current(vgs: vgs, vds: vdd)
        vds = vdd - ids * r_drain

        # Recalculate with correct Vds
        ids = transistor.drain_current(vgs: vgs, vds: [vds, 0.0].max)
        vds = vdd - ids * r_drain

        # Transconductance
        gm = transistor.transconductance(vgs: vgs, vds: [vds, 0.0].max)

        # Voltage gain: Av = -gm * Rd (inverting amplifier)
        voltage_gain = -gm * r_drain

        # Input impedance: essentially infinite for MOSFET (gate is insulated)
        input_impedance = 1e12 # 1 Tohm

        # Output impedance: approximately Rd
        output_impedance = r_drain

        # Bandwidth: f_3dB = 1 / (2*PI*Rd*C_load)
        bandwidth = 1.0 / (2.0 * Math::PI * r_drain * c_load)

        operating_point = {
          "vgs" => vgs,
          "vds" => vds,
          "ids" => ids,
          "gm" => gm
        }

        AmplifierAnalysis.new(
          voltage_gain: voltage_gain,
          transconductance: gm,
          input_impedance: input_impedance,
          output_impedance: output_impedance,
          bandwidth: bandwidth,
          operating_point: operating_point
        )
      end

      # Analyze an NPN common-emitter amplifier configuration.
      #
      # The BJT equivalent of the common-source amplifier. Input is applied
      # to the base, output taken from the collector.
      #
      # BJT amplifiers typically have higher voltage gain than MOSFET amplifiers
      # at the same current, because BJT transconductance (gm = Ic/Vt) is
      # higher than MOSFET transconductance for the same bias current.
      #
      # However, BJT amplifiers have lower input impedance because base current
      # flows continuously.
      #
      # @param transistor [NPN] NPN transistor instance
      # @param vbe [Float] DC base-emitter bias voltage (V)
      # @param vcc [Float] Supply voltage (V)
      # @param r_collector [Float] Collector resistor value (ohms)
      # @param c_load [Float] Output load capacitance (F)
      # @return [AmplifierAnalysis] with gain, impedance, and bandwidth
      def analyze_common_emitter_amp(transistor, vbe:, vcc:, r_collector:, c_load: 1e-12)
        # Calculate DC operating point
        vce = vcc # Initial approximation
        ic = transistor.collector_current(vbe: vbe, vce: vce)
        vce = vcc - ic * r_collector
        vce = [vce, 0.0].max

        # Recalculate with correct Vce
        ic = transistor.collector_current(vbe: vbe, vce: vce)

        # Transconductance
        gm = transistor.transconductance(vbe: vbe, vce: vce)

        # Voltage gain: Av = -gm * Rc
        voltage_gain = -gm * r_collector

        # Input impedance: r_pi = beta / gm = beta * Vt / Ic
        beta = transistor.params.beta
        vt = 0.026
        r_pi = if ic > 0
                 beta * vt / ic
               else
                 1e12 # Very high when no current flows
               end

        input_impedance = r_pi
        output_impedance = r_collector

        # Bandwidth
        bandwidth = 1.0 / (2.0 * Math::PI * r_collector * c_load)

        operating_point = {
          "vbe" => vbe,
          "vce" => vce,
          "ic" => ic,
          "ib" => transistor.base_current(vbe: vbe, vce: vce),
          "gm" => gm
        }

        AmplifierAnalysis.new(
          voltage_gain: voltage_gain,
          transconductance: gm,
          input_impedance: input_impedance,
          output_impedance: output_impedance,
          bandwidth: bandwidth,
          operating_point: operating_point
        )
      end
    end
  end
end
