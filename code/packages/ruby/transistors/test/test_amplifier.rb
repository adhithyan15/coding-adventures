# frozen_string_literal: true

require "test_helper"

module CodingAdventures
  module Transistors
    class TestCommonSourceAmplifier < Minitest::Test
      # Test NMOS common-source amplifier analysis.

      def test_inverting_gain
        # Common-source amplifier should have negative voltage gain.
        t = NMOS.new
        result = Amplifier.analyze_common_source_amp(t, vgs: 1.5, vdd: 3.3, r_drain: 10_000)
        assert result.voltage_gain < 0
      end

      def test_high_input_impedance
        # MOSFET amplifiers should have very high input impedance.
        t = NMOS.new
        result = Amplifier.analyze_common_source_amp(t, vgs: 1.5, vdd: 3.3, r_drain: 10_000)
        assert result.input_impedance > 1e9
      end

      def test_positive_transconductance
        # Transconductance should be positive.
        t = NMOS.new
        result = Amplifier.analyze_common_source_amp(t, vgs: 1.5, vdd: 3.3, r_drain: 10_000)
        assert result.transconductance > 0
      end

      def test_positive_bandwidth
        # Bandwidth should be positive.
        t = NMOS.new
        result = Amplifier.analyze_common_source_amp(t, vgs: 1.5, vdd: 3.3, r_drain: 10_000)
        assert result.bandwidth > 0
      end

      def test_operating_point
        # Operating point should contain required keys.
        t = NMOS.new
        result = Amplifier.analyze_common_source_amp(t, vgs: 1.5, vdd: 3.3, r_drain: 10_000)
        assert result.operating_point.key?("vgs")
        assert result.operating_point.key?("vds")
        assert result.operating_point.key?("ids")
        assert result.operating_point.key?("gm")
      end

      def test_higher_rd_more_gain
        # Higher drain resistance should give more voltage gain.
        t = NMOS.new
        r1 = Amplifier.analyze_common_source_amp(t, vgs: 1.5, vdd: 3.3, r_drain: 5_000)
        r2 = Amplifier.analyze_common_source_amp(t, vgs: 1.5, vdd: 3.3, r_drain: 20_000)
        assert r2.voltage_gain.abs > r1.voltage_gain.abs
      end
    end

    class TestCommonEmitterAmplifier < Minitest::Test
      # Test NPN common-emitter amplifier analysis.

      def test_inverting_gain
        # Common-emitter amplifier should have negative voltage gain.
        t = NPN.new
        result = Amplifier.analyze_common_emitter_amp(t, vbe: 0.7, vcc: 5.0, r_collector: 4700)
        assert result.voltage_gain < 0
      end

      def test_moderate_input_impedance
        # BJT amplifiers have moderate input impedance (r_pi).
        t = NPN.new
        result = Amplifier.analyze_common_emitter_amp(t, vbe: 0.7, vcc: 5.0, r_collector: 4700)
        assert result.input_impedance > 100
        assert result.input_impedance < 1e6
      end

      def test_positive_transconductance
        # Transconductance should be positive.
        t = NPN.new
        result = Amplifier.analyze_common_emitter_amp(t, vbe: 0.7, vcc: 5.0, r_collector: 4700)
        assert result.transconductance > 0
      end

      def test_higher_beta_higher_impedance
        # Higher beta should give higher input impedance.
        t_low = NPN.new(BJTParams.new(beta: 50))
        t_high = NPN.new(BJTParams.new(beta: 200))
        r1 = Amplifier.analyze_common_emitter_amp(t_low, vbe: 0.7, vcc: 5.0, r_collector: 4700)
        r2 = Amplifier.analyze_common_emitter_amp(t_high, vbe: 0.7, vcc: 5.0, r_collector: 4700)
        assert r2.input_impedance > r1.input_impedance
      end

      def test_operating_point
        # Operating point should contain required keys.
        t = NPN.new
        result = Amplifier.analyze_common_emitter_amp(t, vbe: 0.7, vcc: 5.0, r_collector: 4700)
        assert result.operating_point.key?("vbe")
        assert result.operating_point.key?("vce")
        assert result.operating_point.key?("ic")
        assert result.operating_point.key?("ib")
      end
    end
  end
end
