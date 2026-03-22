# frozen_string_literal: true

require "test_helper"

module CodingAdventures
  module Transistors
    class TestNMOS < Minitest::Test
      # Test NMOS transistor operating regions and electrical behavior.

      def test_cutoff_region
        # Vgs below threshold -> no current, switch OFF.
        t = NMOS.new
        assert_equal MOSFETRegion::CUTOFF, t.region(vgs: 0.0, vds: 1.0)
        assert_equal 0.0, t.drain_current(vgs: 0.0, vds: 1.0)
        refute t.conducting?(vgs: 0.0)
      end

      def test_cutoff_negative_vgs
        # Negative Vgs should also be cutoff.
        t = NMOS.new
        assert_equal MOSFETRegion::CUTOFF, t.region(vgs: -1.0, vds: 0.0)
        assert_equal 0.0, t.drain_current(vgs: -1.0, vds: 0.0)
      end

      def test_linear_region
        # Vgs above threshold, low Vds -> linear region.
        t = NMOS.new
        assert_equal MOSFETRegion::LINEAR, t.region(vgs: 1.5, vds: 0.1)
        ids = t.drain_current(vgs: 1.5, vds: 0.1)
        assert ids > 0
      end

      def test_saturation_region
        # Vgs above threshold, high Vds -> saturation.
        t = NMOS.new
        assert_equal MOSFETRegion::SATURATION, t.region(vgs: 1.0, vds: 3.0)
        ids = t.drain_current(vgs: 1.0, vds: 3.0)
        assert ids > 0
      end

      def test_saturation_current_independent_of_vds
        # In saturation, current depends only on Vgs, not Vds.
        t = NMOS.new
        ids_1 = t.drain_current(vgs: 1.5, vds: 3.0)
        ids_2 = t.drain_current(vgs: 1.5, vds: 5.0)
        assert_in_delta ids_1, ids_2, 1e-10
      end

      def test_linear_current_increases_with_vds
        # In linear region, current increases with Vds.
        t = NMOS.new
        ids_low = t.drain_current(vgs: 3.0, vds: 0.1)
        ids_high = t.drain_current(vgs: 3.0, vds: 0.5)
        assert ids_high > ids_low
      end

      def test_is_conducting
        # conducting? should be true when Vgs >= Vth.
        t = NMOS.new
        refute t.conducting?(vgs: 0.3) # Below default Vth=0.4
        assert t.conducting?(vgs: 0.4) # At Vth
        assert t.conducting?(vgs: 1.0) # Above Vth
      end

      def test_output_voltage_on
        # When ON, output should be pulled to GND.
        t = NMOS.new
        assert_equal 0.0, t.output_voltage(vgs: 3.3, vdd: 3.3)
      end

      def test_output_voltage_off
        # When OFF, output should be at Vdd.
        t = NMOS.new
        assert_equal 3.3, t.output_voltage(vgs: 0.0, vdd: 3.3)
      end

      def test_custom_params
        # Custom parameters should be respected.
        params = MOSFETParams.new(vth: 0.7, k: 0.002)
        t = NMOS.new(params)
        refute t.conducting?(vgs: 0.5) # Below custom Vth
        assert t.conducting?(vgs: 0.7) # At custom Vth
      end

      def test_transconductance_cutoff
        # gm should be 0 in cutoff.
        t = NMOS.new
        assert_equal 0.0, t.transconductance(vgs: 0.0, vds: 1.0)
      end

      def test_transconductance_saturation
        # gm should be positive in saturation.
        t = NMOS.new
        gm = t.transconductance(vgs: 1.5, vds: 3.0)
        assert gm > 0
      end

      def test_boundary_cutoff_linear
        # Just above Vth with small Vds, transistor is in linear.
        t = NMOS.new
        assert_equal MOSFETRegion::LINEAR, t.region(vgs: 0.5, vds: 0.01)
      end

      def test_boundary_linear_saturation
        # At Vds = Vgs - Vth, transistor enters saturation.
        t = NMOS.new
        vgs = 1.0
        vth = 0.4
        vds = vgs - vth # Exactly at boundary
        assert_equal MOSFETRegion::SATURATION, t.region(vgs: vgs, vds: vds)
      end
    end

    class TestPMOS < Minitest::Test
      # Test PMOS transistor operating regions and electrical behavior.

      def test_cutoff_when_vgs_zero
        # PMOS with Vgs=0 (gate at source level) should be OFF.
        t = PMOS.new
        assert_equal MOSFETRegion::CUTOFF, t.region(vgs: 0.0, vds: 0.0)
        refute t.conducting?(vgs: 0.0)
      end

      def test_conducts_when_vgs_negative
        # PMOS conducts when Vgs is sufficiently negative.
        t = PMOS.new
        assert t.conducting?(vgs: -1.5)
        assert_equal MOSFETRegion::SATURATION, t.region(vgs: -1.5, vds: -3.0)
      end

      def test_linear_region
        # PMOS in linear region with small |Vds|.
        t = PMOS.new
        assert_equal MOSFETRegion::LINEAR, t.region(vgs: -1.5, vds: -0.1)
      end

      def test_drain_current_positive
        # PMOS drain current magnitude should be positive.
        t = PMOS.new
        ids = t.drain_current(vgs: -1.5, vds: -3.0)
        assert ids > 0
      end

      def test_cutoff_no_current
        # PMOS in cutoff should have zero current.
        t = PMOS.new
        assert_equal 0.0, t.drain_current(vgs: 0.0, vds: -1.0)
      end

      def test_output_voltage_on
        # When ON, PMOS pulls output to Vdd.
        t = PMOS.new
        assert_equal 3.3, t.output_voltage(vgs: -3.3, vdd: 3.3)
      end

      def test_output_voltage_off
        # When OFF, PMOS output is at GND.
        t = PMOS.new
        assert_equal 0.0, t.output_voltage(vgs: 0.0, vdd: 3.3)
      end

      def test_complementary_to_nmos
        # PMOS should be ON when NMOS is OFF and vice versa.
        nmos = NMOS.new
        pmos = PMOS.new
        vdd = 3.3

        # Input HIGH: NMOS ON, PMOS OFF
        assert nmos.conducting?(vgs: vdd)
        refute pmos.conducting?(vgs: 0.0)

        # Input LOW: NMOS OFF, PMOS ON
        refute nmos.conducting?(vgs: 0.0)
        assert pmos.conducting?(vgs: -vdd)
      end

      def test_transconductance_cutoff
        # gm should be 0 in cutoff.
        t = PMOS.new
        assert_equal 0.0, t.transconductance(vgs: 0.0, vds: 0.0)
      end

      def test_transconductance_on
        # gm should be positive when conducting.
        t = PMOS.new
        gm = t.transconductance(vgs: -1.5, vds: -3.0)
        assert gm > 0
      end
    end
  end
end
