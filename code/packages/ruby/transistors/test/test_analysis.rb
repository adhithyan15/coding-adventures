# frozen_string_literal: true

require "test_helper"

module CodingAdventures
  module Transistors
    class TestNoiseMargins < Minitest::Test
      # Test noise margin computation.

      def test_cmos_positive_margins
        # CMOS noise margins should be positive.
        nm = Analysis.compute_noise_margins(CMOSInverter.new)
        assert nm.nml > 0
        assert nm.nmh > 0
      end

      def test_cmos_symmetric
        # CMOS noise margins should be roughly symmetric.
        nm = Analysis.compute_noise_margins(CMOSInverter.new)
        assert (nm.nml - nm.nmh).abs < nm.nml * 0.5
      end

      def test_ttl_positive_margins
        # TTL noise margins should be positive.
        nm = Analysis.compute_noise_margins(TTLNand.new)
        assert nm.nml > 0
        assert nm.nmh > 0
      end

      def test_cmos_vol_near_zero
        # CMOS output LOW should be near 0V.
        nm = Analysis.compute_noise_margins(CMOSInverter.new)
        assert nm.vol < 0.1
      end

      def test_ttl_vol_vce_sat
        # TTL output LOW should be near Vce_sat.
        nm = Analysis.compute_noise_margins(TTLNand.new)
        assert nm.vol < 0.5
      end
    end

    class TestPowerAnalysis < Minitest::Test
      # Test power consumption analysis.

      def test_cmos_zero_static_power
        # CMOS gates should have near-zero static power.
        power = Analysis.analyze_power(CMOSInverter.new)
        assert power.static_power < 1e-9
      end

      def test_ttl_significant_static_power
        # TTL gates should have milliwatt-level static power.
        power = Analysis.analyze_power(TTLNand.new)
        assert power.static_power > 1e-3
      end

      def test_positive_dynamic_power
        # Dynamic power should be positive at non-zero frequency.
        power = Analysis.analyze_power(CMOSInverter.new, frequency: 1e9)
        assert power.dynamic_power > 0
      end

      def test_total_power_sum
        # Total power should be static + dynamic.
        power = Analysis.analyze_power(CMOSInverter.new, frequency: 1e9)
        assert_in_delta power.total_power, power.static_power + power.dynamic_power, 1e-15
      end

      def test_energy_per_switch_positive
        # Energy per switch should be positive.
        power = Analysis.analyze_power(CMOSInverter.new)
        assert power.energy_per_switch > 0
      end

      def test_cmos_nand_power
        # CMOSNand should also work with analyze_power.
        power = Analysis.analyze_power(CMOSNand.new)
        assert_equal 0.0, power.static_power
      end

      def test_cmos_nor_power
        # CMOSNor should also work with analyze_power.
        power = Analysis.analyze_power(CMOSNor.new)
        assert_equal 0.0, power.static_power
      end
    end

    class TestTimingAnalysis < Minitest::Test
      # Test timing characteristic analysis.

      def test_cmos_positive_delays
        # CMOS propagation delays should be positive.
        timing = Analysis.analyze_timing(CMOSInverter.new)
        assert timing.tphl > 0
        assert timing.tplh > 0
        assert timing.tpd > 0
      end

      def test_tpd_is_average
        # tpd should be the average of tphl and tplh.
        timing = Analysis.analyze_timing(CMOSInverter.new)
        expected = (timing.tphl + timing.tplh) / 2.0
        assert_in_delta expected, timing.tpd, 1e-20
      end

      def test_cmos_faster_than_ttl
        # CMOS delay should be faster than TTL delay.
        cmos_timing = Analysis.analyze_timing(CMOSInverter.new)
        ttl_timing = Analysis.analyze_timing(TTLNand.new)
        assert cmos_timing.tpd < ttl_timing.tpd
      end

      def test_positive_rise_fall
        # Rise and fall times should be positive.
        timing = Analysis.analyze_timing(CMOSInverter.new)
        assert timing.rise_time > 0
        assert timing.fall_time > 0
      end

      def test_max_frequency_positive
        # Maximum frequency should be positive.
        timing = Analysis.analyze_timing(CMOSInverter.new)
        assert timing.max_frequency > 0
      end

      def test_cmos_nand_timing
        # CMOSNand should also work with analyze_timing.
        timing = Analysis.analyze_timing(CMOSNand.new)
        assert timing.tpd > 0
      end

      def test_cmos_nor_timing
        # CMOSNor should also work with analyze_timing.
        timing = Analysis.analyze_timing(CMOSNor.new)
        assert timing.tpd > 0
      end
    end

    class TestComparisonUtilities < Minitest::Test
      # Test CMOS vs TTL comparison and scaling functions.

      def test_compare_returns_both
        # compare_cmos_vs_ttl should return both CMOS and TTL data.
        result = Analysis.compare_cmos_vs_ttl
        assert result.key?("cmos")
        assert result.key?("ttl")
      end

      def test_cmos_less_static_power
        # CMOS should have much less static power than TTL.
        result = Analysis.compare_cmos_vs_ttl
        assert result["cmos"]["static_power_w"] < result["ttl"]["static_power_w"]
      end

      def test_scaling_returns_list
        # demonstrate_cmos_scaling should return an array of hashes.
        result = Analysis.demonstrate_cmos_scaling
        assert_instance_of Array, result
        assert result.length > 0
      end

      def test_scaling_default_nodes
        # Default should produce 6 technology nodes.
        result = Analysis.demonstrate_cmos_scaling
        assert_equal 6, result.length
      end

      def test_scaling_custom_nodes
        # Custom technology nodes should be respected.
        result = Analysis.demonstrate_cmos_scaling([180e-9, 45e-9])
        assert_equal 2, result.length
      end

      def test_scaling_vdd_decreases
        # Supply voltage should generally decrease with scaling.
        result = Analysis.demonstrate_cmos_scaling
        assert result[0]["vdd_v"] > result[-1]["vdd_v"]
      end

      def test_scaling_has_expected_keys
        # Each scaling result should have expected keys.
        result = Analysis.demonstrate_cmos_scaling([180e-9])
        entry = result[0]
        assert entry.key?("node_nm")
        assert entry.key?("vdd_v")
        assert entry.key?("vth_v")
        assert entry.key?("propagation_delay_s")
        assert entry.key?("dynamic_power_w")
        assert entry.key?("leakage_current_a")
      end
    end
  end
end
