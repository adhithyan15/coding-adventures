# frozen_string_literal: true

# Shared types for the transistors package.
#
# === Enums and Parameter Structs ===
#
# These types define the vocabulary of transistor simulation. Every transistor
# has an operating region (cutoff, linear, saturation), and every circuit has
# electrical parameters (voltage, capacitance, etc.).
#
# We use frozen Structs for parameters because transistor characteristics
# are fixed once manufactured — you can't change a transistor's threshold
# voltage after fabrication. Freezing enforces this immutability in code.

module CodingAdventures
  module Transistors
    # ===========================================================================
    # OPERATING REGION MODULES
    # ===========================================================================
    # A transistor is an analog device that operates differently depending on
    # the voltages applied to its terminals. The three "regions" describe these
    # different operating modes.

    # Operating region of a MOSFET transistor.
    #
    # Think of it like a water faucet with three positions:
    #
    #     CUTOFF:     Faucet is fully closed. No water flows.
    #                 (Vgs < Vth — gate voltage too low to turn on)
    #
    #     LINEAR:     Faucet is open, and water flow increases as you
    #                 turn the handle more. Flow is proportional to
    #                 both handle position AND water pressure.
    #                 (Vgs > Vth, Vds < Vgs - Vth — acts like a resistor)
    #
    #     SATURATION: Faucet is wide open, but the pipe is the bottleneck.
    #                 Adding more pressure doesn't increase flow much.
    #                 (Vgs > Vth, Vds >= Vgs - Vth — current is roughly constant)
    #
    # For digital circuits, we only use CUTOFF (OFF) and deep LINEAR (ON).
    # For analog amplifiers, we operate in SATURATION.
    module MOSFETRegion
      CUTOFF = "cutoff"
      LINEAR = "linear"
      SATURATION = "saturation"
    end

    # Operating region of a BJT transistor.
    #
    # Similar to MOSFET regions but with different names and physics:
    #
    #     CUTOFF:      No base current -> no collector current. Switch OFF.
    #                  (Vbe < ~0.7V)
    #
    #     ACTIVE:      Small base current, large collector current.
    #                  Ic = beta * Ib. This is the AMPLIFIER region.
    #                  (Vbe >= ~0.7V, Vce > ~0.2V)
    #
    #     SATURATION:  Both junctions forward-biased. Collector current
    #                  is maximum — transistor is fully ON as a switch.
    #                  (Vbe >= ~0.7V, Vce <= ~0.2V)
    #
    # Confusing naming alert: MOSFET "saturation" = constant current (amplifier).
    # BJT "saturation" = fully ON (switch). These are DIFFERENT behaviors despite
    # sharing a name. Hardware engineers have been confusing students with this
    # for decades.
    module BJTRegion
      CUTOFF = "cutoff"
      ACTIVE = "active"
      SATURATION = "saturation"
    end

    # Transistor polarity/type.
    module TransistorType
      NMOS = "nmos"
      PMOS = "pmos"
      NPN = "npn"
      PNP = "pnp"
    end

    # ===========================================================================
    # ELECTRICAL PARAMETERS
    # ===========================================================================
    # These Structs hold the physical characteristics of transistors.
    # Default values represent common, well-documented transistor types
    # so that users can start experimenting immediately without needing
    # to look up datasheets.

    # Electrical parameters for a MOSFET transistor.
    #
    # Default values represent a typical 180nm CMOS process — the last
    # "large" process node that is still widely used in education and
    # analog/mixed-signal chips.
    #
    # Key parameters:
    #     vth:     Threshold voltage — the minimum Vgs to turn the transistor ON.
    #              Lower Vth = faster switching but more leakage current.
    #              Modern CPUs use Vth around 0.2-0.4V.
    #
    #     k:       Transconductance parameter — controls how much current flows
    #              for a given Vgs. Higher k = more current = faster but more power.
    #              k = mu * Cox * (W/L) where mu is carrier mobility and Cox is
    #              oxide capacitance per unit area.
    #
    #     w, l:    Channel width and length. The W/L ratio is the main knob
    #              chip designers use to tune transistor strength. Wider = more
    #              current. Shorter = faster but harder to manufacture.
    #
    #     c_gate:  Gate capacitance — determines switching speed. The gate
    #              capacitor must charge/discharge to switch the transistor,
    #              so smaller C = faster switching.
    #
    #     c_drain: Drain junction capacitance — contributes to output load.
    MOSFETParams = Struct.new(:vth, :k, :w, :l, :c_gate, :c_drain, keyword_init: true) do
      # @param vth [Float] Threshold voltage (default 0.4V)
      # @param k [Float] Transconductance parameter (default 0.001)
      # @param w [Float] Channel width (default 1e-6 m)
      # @param l [Float] Channel length (default 180e-9 m)
      # @param c_gate [Float] Gate capacitance (default 1e-15 F)
      # @param c_drain [Float] Drain capacitance (default 0.5e-15 F)
      def initialize(vth: 0.4, k: 0.001, w: 1e-6, l: 180e-9, c_gate: 1e-15, c_drain: 0.5e-15)
        super(vth: vth, k: k, w: w, l: l, c_gate: c_gate, c_drain: c_drain)
        freeze
      end
    end

    # Electrical parameters for a BJT transistor.
    #
    # Default values represent a typical small-signal NPN transistor
    # like the 2N2222 — one of the most common transistors ever made,
    # used in everything from hobby projects to early spacecraft.
    #
    # Key parameters:
    #     beta:    Current gain (hfe) — the ratio Ic/Ib. A beta of 100
    #              means 1mA of base current controls 100mA of collector
    #              current. This amplification is what made transistors
    #              revolutionary.
    #
    #     vbe_on:  Base-emitter voltage when conducting. For silicon BJTs,
    #              this is always around 0.6-0.7V — it's a fundamental
    #              property of the silicon PN junction.
    #
    #     vce_sat: Collector-emitter voltage when fully saturated (switch ON).
    #              Ideally 0V, practically about 0.1-0.3V.
    #
    #     is_:     Reverse saturation current — the tiny leakage current
    #              that flows even when the transistor is OFF.
    #
    #     c_base:  Base capacitance — limits switching speed.
    BJTParams = Struct.new(:beta, :vbe_on, :vce_sat, :is_, :c_base, keyword_init: true) do
      # @param beta [Float] Current gain (default 100.0)
      # @param vbe_on [Float] Base-emitter on voltage (default 0.7V)
      # @param vce_sat [Float] Collector-emitter saturation voltage (default 0.2V)
      # @param is_ [Float] Reverse saturation current (default 1e-14 A)
      # @param c_base [Float] Base capacitance (default 5e-12 F)
      def initialize(beta: 100.0, vbe_on: 0.7, vce_sat: 0.2, is_: 1e-14, c_base: 5e-12)
        super(beta: beta, vbe_on: vbe_on, vce_sat: vce_sat, is_: is_, c_base: c_base)
        freeze
      end
    end

    # Parameters for a complete logic gate circuit.
    #
    # vdd:         Supply voltage. Modern CMOS uses 0.7-1.2V, older CMOS
    #              used 3.3V or 5V, TTL always uses 5V. Lower voltage
    #              means less power (P scales with V^2) but also less
    #              noise margin and slower switching.
    #
    # temperature: Junction temperature in Kelvin. Room temperature is
    #              ~300K (27C). Higher temperature increases leakage
    #              current and reduces carrier mobility.
    CircuitParams = Struct.new(:vdd, :temperature, keyword_init: true) do
      # @param vdd [Float] Supply voltage (default 3.3V)
      # @param temperature [Float] Junction temperature in Kelvin (default 300.0K)
      def initialize(vdd: 3.3, temperature: 300.0)
        super(vdd: vdd, temperature: temperature)
        freeze
      end
    end

    # ===========================================================================
    # RESULT TYPES
    # ===========================================================================
    # These Structs hold the results of transistor and circuit analysis.
    # Each one bundles together related measurements so callers don't need
    # to track multiple return values.

    # Result of evaluating a logic gate with voltage-level detail.
    #
    # Unlike the logic_gates package which only returns 0 or 1, this gives
    # you the full electrical picture: what voltage does the output actually
    # sit at? How much power is being consumed? How long did the signal
    # take to propagate?
    GateOutput = Struct.new(
      :logic_value,
      :voltage,
      :current_draw,
      :power_dissipation,
      :propagation_delay,
      :transistor_count,
      keyword_init: true
    )

    # Results of analyzing a transistor as an amplifier.
    #
    # voltage_gain:      How much the output voltage changes per unit change
    #                    in input voltage. Negative for inverting amplifiers.
    #
    # transconductance:  gm — the ratio of output current change to input
    #                    voltage change. Units: Siemens (A/V).
    #
    # input_impedance:   How much the amplifier "loads" the signal source.
    #                    MOSFET: very high (>1 Gohm). BJT: moderate (~1-10 kohm).
    #
    # output_impedance:  How "stiff" the output is. Lower = can drive heavier loads.
    #
    # bandwidth:         Frequency at which gain drops to 70.7% (-3dB).
    AmplifierAnalysis = Struct.new(
      :voltage_gain,
      :transconductance,
      :input_impedance,
      :output_impedance,
      :bandwidth,
      :operating_point,
      keyword_init: true
    )

    # Noise margin analysis for a logic family.
    #
    # Noise margins tell you how much electrical noise (voltage fluctuation)
    # a digital signal can tolerate before being misinterpreted.
    #
    #     vol: Output LOW voltage — what the gate actually outputs for logic 0
    #     voh: Output HIGH voltage — what the gate actually outputs for logic 1
    #     vil: Input LOW threshold — maximum voltage the next gate accepts as 0
    #     vih: Input HIGH threshold — minimum voltage the next gate accepts as 1
    #
    #     nml: Noise Margin LOW  = vil - vol
    #     nmh: Noise Margin HIGH = voh - vih
    NoiseMargins = Struct.new(
      :vol, :voh, :vil, :vih, :nml, :nmh,
      keyword_init: true
    )

    # Power consumption breakdown for a gate or circuit.
    #
    # static_power:      Power consumed even when the gate is not switching.
    #                    For CMOS: dominated by transistor leakage (~nW).
    #                    For TTL: dominated by resistor bias current (~mW).
    #
    # dynamic_power:     Power consumed during switching transitions.
    #                    P_dyn = C_load * Vdd^2 * f * alpha
    #
    # total_power:       static + dynamic.
    #
    # energy_per_switch: Energy for one complete 0->1->0 transition.
    #                    E = C_load * Vdd^2.
    PowerAnalysis = Struct.new(
      :static_power,
      :dynamic_power,
      :total_power,
      :energy_per_switch,
      keyword_init: true
    )

    # Timing characteristics for a gate.
    #
    # tphl:          Propagation delay from HIGH to LOW output.
    # tplh:          Propagation delay from LOW to HIGH output.
    # tpd:           Average propagation delay = (tphl + tplh) / 2.
    # rise_time:     Time for output to go from 10% to 90% of Vdd.
    # fall_time:     Time for output to go from 90% to 10% of Vdd.
    # max_frequency: Maximum clock frequency = 1 / (2 * tpd).
    TimingAnalysis = Struct.new(
      :tphl, :tplh, :tpd,
      :rise_time, :fall_time,
      :max_frequency,
      keyword_init: true
    )
  end
end
