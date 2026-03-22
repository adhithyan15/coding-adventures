# frozen_string_literal: true

# Electrical Analysis — noise margins, power, timing, and technology comparison.
#
# === Why Electrical Analysis Matters ===
#
# Digital logic designers don't just care about truth tables — they care about:
#
# 1. NOISE MARGINS: Can the circuit tolerate voltage fluctuations on the wires?
#    A chip has billions of wires running millimeters apart, each creating
#    electromagnetic interference on its neighbors.
#
# 2. POWER: How much energy does the chip consume? A modern CPU runs at
#    ~100 watts. Power = the #1 constraint in modern chip design.
#
# 3. TIMING: How fast can the circuit switch? The propagation delay through
#    a gate determines the maximum clock frequency.
#
# 4. SCALING: How do these properties change as we shrink transistors?

module CodingAdventures
  module Transistors
    # Analysis module methods for noise margins, power, timing, and comparison.
    module Analysis
      module_function

      # Analyze noise margins for a gate.
      #
      # Noise margins tell you how much electrical noise a digital signal
      # can tolerate before being misinterpreted by the next gate in the chain.
      #
      # For CMOS:
      #     VOL ~ 0V, VOH ~ Vdd -> large noise margins
      #     NML ~ NMH ~ 0.4 * Vdd (symmetric)
      #
      # For TTL:
      #     VOL ~ 0.2V, VOH ~ 3.5V -> smaller margins
      #     VIL = 0.8V, VIH = 2.0V (defined by spec)
      #
      # @param gate [CMOSInverter, TTLNand] the gate to analyze
      # @return [NoiseMargins] noise margin analysis results
      def compute_noise_margins(gate)
        if gate.is_a?(CMOSInverter)
          vdd = gate.circuit.vdd
          vol = 0.0
          voh = vdd
          vil = 0.4 * vdd
          vih = 0.6 * vdd
        elsif gate.is_a?(TTLNand)
          vol = 0.2
          voh = gate.vcc - 0.7
          vil = 0.8
          vih = 2.0
        else
          raise TypeError, "Unsupported gate type: #{gate.class}"
        end

        nml = vil - vol
        nmh = voh - vih

        NoiseMargins.new(
          vol: vol, voh: voh, vil: vil, vih: vih,
          nml: nml, nmh: nmh
        )
      end

      # Compute power consumption for a gate at a given operating frequency.
      #
      # === Power in CMOS ===
      #     P_total = P_static + P_dynamic
      #     P_static ~ 0 (negligible)
      #     P_dynamic = C_load * Vdd^2 * f * alpha
      #
      # === Power in TTL ===
      #     P_static ~ milliwatts (DOMINATES!)
      #     P_dynamic = similar formula
      #
      # @param gate [CMOSInverter, CMOSNand, CMOSNor, TTLNand] the gate
      # @param frequency [Float] Operating frequency in Hz (default 1 GHz)
      # @param c_load [Float] Load capacitance in Farads (default 1 pF)
      # @param activity_factor [Float] Fraction of cycles with transition (0-1)
      # @return [PowerAnalysis] power consumption breakdown
      def analyze_power(gate, frequency: 1e9, c_load: 1e-12, activity_factor: 0.5)
        if gate.is_a?(TTLNand)
          static = gate.static_power
          vdd = gate.vcc
        elsif gate.is_a?(CMOSInverter) || gate.is_a?(CMOSNand) || gate.is_a?(CMOSNor)
          static = 0.0
          vdd = gate.circuit.vdd
        else
          raise TypeError, "Unsupported gate type: #{gate.class}"
        end

        # Dynamic power: P = C * V^2 * f * alpha
        dynamic = c_load * vdd * vdd * frequency * activity_factor
        total = static + dynamic

        # Energy per switching event: E = C * V^2
        energy_per_switch = c_load * vdd * vdd

        PowerAnalysis.new(
          static_power: static,
          dynamic_power: dynamic,
          total_power: total,
          energy_per_switch: energy_per_switch
        )
      end

      # Compute timing characteristics for a gate.
      #
      # For CMOS:
      #     t_pd ~ (C_load * Vdd) / (2 * I_sat)
      #
      # For TTL:
      #     t_pd ~ 5-15 ns (fixed by transistor switching speed)
      #
      # @param gate [CMOSInverter, CMOSNand, CMOSNor, TTLNand] the gate
      # @param c_load [Float] Load capacitance in Farads (default 1 pF)
      # @return [TimingAnalysis] timing characteristics
      def analyze_timing(gate, c_load: 1e-12)
        if gate.is_a?(TTLNand)
          tphl = 7e-9
          tplh = 11e-9
          tpd = (tphl + tplh) / 2.0
          rise_time = 15e-9
          fall_time = 10e-9
        elsif gate.is_a?(CMOSInverter) || gate.is_a?(CMOSNand) || gate.is_a?(CMOSNor)
          vdd = gate.circuit.vdd

          # Get NMOS/PMOS parameters
          if gate.is_a?(CMOSInverter)
            nmos = gate.nmos
            pmos = gate.pmos
          else
            nmos = gate.nmos1
            pmos = gate.pmos1
          end

          # Saturation current approximation for timing
          k = nmos.params.k
          vth = nmos.params.vth
          ids_sat_n = vdd > vth ? 0.5 * k * (vdd - vth)**2 : 1e-12
          ids_sat_p = vdd > pmos.params.vth ? 0.5 * pmos.params.k * (vdd - pmos.params.vth)**2 : 1e-12

          # Propagation delays
          tphl = c_load * vdd / (2.0 * ids_sat_n) # Pull-down (NMOS)
          tplh = c_load * vdd / (2.0 * ids_sat_p) # Pull-up (PMOS)
          tpd = (tphl + tplh) / 2.0

          # Rise and fall times (2.2 RC time constants)
          r_on_n = ids_sat_n > 0 ? vdd / (2.0 * ids_sat_n) : 1e6
          r_on_p = ids_sat_p > 0 ? vdd / (2.0 * ids_sat_p) : 1e6
          rise_time = 2.2 * r_on_p * c_load
          fall_time = 2.2 * r_on_n * c_load
        else
          raise TypeError, "Unsupported gate type: #{gate.class}"
        end

        max_frequency = tpd > 0 ? 1.0 / (2.0 * tpd) : Float::INFINITY

        TimingAnalysis.new(
          tphl: tphl, tplh: tplh, tpd: tpd,
          rise_time: rise_time, fall_time: fall_time,
          max_frequency: max_frequency
        )
      end

      # Compare CMOS and TTL NAND gates across all metrics.
      #
      # This function demonstrates WHY CMOS replaced TTL:
      # - CMOS has ~1000x less static power
      # - CMOS has better noise margins (relative to Vdd)
      # - CMOS can operate at lower voltages
      # - CMOS gates use fewer transistors
      #
      # @param frequency [Float] operating frequency in Hz (default 1 MHz)
      # @param c_load [Float] load capacitance in Farads (default 1 pF)
      # @return [Hash] with "cmos" and "ttl" keys containing metrics
      def compare_cmos_vs_ttl(frequency: 1e6, c_load: 1e-12)
        cmos_nand = CMOSNand.new
        ttl_nand = TTLNand.new

        cmos_power = analyze_power(cmos_nand, frequency: frequency, c_load: c_load)
        ttl_power = analyze_power(ttl_nand, frequency: frequency, c_load: c_load)

        cmos_timing = analyze_timing(cmos_nand, c_load: c_load)
        ttl_timing = analyze_timing(ttl_nand, c_load: c_load)

        cmos_nm = compute_noise_margins(CMOSInverter.new)
        ttl_nm = compute_noise_margins(ttl_nand)

        {
          "cmos" => {
            "transistor_count" => 4,
            "supply_voltage" => cmos_nand.circuit.vdd,
            "static_power_w" => cmos_power.static_power,
            "dynamic_power_w" => cmos_power.dynamic_power,
            "total_power_w" => cmos_power.total_power,
            "propagation_delay_s" => cmos_timing.tpd,
            "max_frequency_hz" => cmos_timing.max_frequency,
            "noise_margin_low_v" => cmos_nm.nml,
            "noise_margin_high_v" => cmos_nm.nmh
          },
          "ttl" => {
            "transistor_count" => 3,
            "supply_voltage" => ttl_nand.vcc,
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

      # Show how CMOS performance changes with technology scaling.
      #
      # As transistors shrink (Moore's Law), several properties change:
      # - Gate length decreases -> faster switching
      # - Supply voltage decreases -> less power per switch
      # - Gate capacitance decreases -> less energy per transition
      # - BUT leakage current INCREASES -> more static power
      #
      # @param technology_nodes [Array<Float>, nil] process nodes in meters
      # @return [Array<Hash>] one hash per technology node with metrics
      def demonstrate_cmos_scaling(technology_nodes = nil)
        technology_nodes ||= [180e-9, 90e-9, 45e-9, 22e-9, 7e-9, 3e-9]

        technology_nodes.map do |node|
          # Empirical scaling relationships (simplified)
          scale = node / 180e-9

          vdd = [0.7, 3.3 * scale**0.5].max
          vth = [0.15, 0.4 * scale**0.3].max
          c_gate = 1e-15 * scale
          k = 0.001 / scale**0.5

          # Create transistor and circuit with scaled parameters
          params = MOSFETParams.new(vth: vth, k: k, l: node, c_gate: c_gate)
          circuit = CircuitParams.new(vdd: vdd)
          inv = CMOSInverter.new(circuit, params, params)

          timing = analyze_timing(inv, c_load: c_gate * 10)
          power = analyze_power(inv, frequency: 1e9, c_load: c_gate * 10)

          # Leakage current increases exponentially as Vth decreases
          leakage = 1e-12 * Math.exp((0.4 - vth) / 0.052)

          {
            "node_nm" => node * 1e9,
            "vdd_v" => vdd,
            "vth_v" => vth,
            "c_gate_f" => c_gate,
            "propagation_delay_s" => timing.tpd,
            "dynamic_power_w" => power.dynamic_power,
            "leakage_current_a" => leakage,
            "max_frequency_hz" => timing.max_frequency
          }
        end
      end
    end
  end
end
