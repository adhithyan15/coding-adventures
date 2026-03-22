# frozen_string_literal: true

require "test_helper"

module CodingAdventures
  module Transistors
    class TestNPN < Minitest::Test
      # Test NPN transistor operating regions and electrical behavior.

      def test_cutoff_region
        # Vbe below threshold -> no current.
        t = NPN.new
        assert_equal BJTRegion::CUTOFF, t.region(vbe: 0.0, vce: 5.0)
        assert_equal 0.0, t.collector_current(vbe: 0.0, vce: 5.0)
        refute t.conducting?(vbe: 0.0)
      end

      def test_active_region
        # Vbe at threshold, Vce > Vce_sat -> active (amplifier).
        t = NPN.new
        assert_equal BJTRegion::ACTIVE, t.region(vbe: 0.7, vce: 3.0)
        ic = t.collector_current(vbe: 0.7, vce: 3.0)
        assert ic > 0
      end

      def test_saturation_region
        # Vbe at threshold, Vce <= Vce_sat -> saturated (switch ON).
        t = NPN.new
        assert_equal BJTRegion::SATURATION, t.region(vbe: 0.7, vce: 0.1)
      end

      def test_is_conducting
        # conducting? should be true when Vbe >= Vbe_on.
        t = NPN.new
        refute t.conducting?(vbe: 0.5)
        assert t.conducting?(vbe: 0.7)
        assert t.conducting?(vbe: 1.0)
      end

      def test_current_gain
        # In active region, Ic should be approximately beta * Ib.
        t = NPN.new(BJTParams.new(beta: 100))
        ic = t.collector_current(vbe: 0.7, vce: 3.0)
        ib = t.base_current(vbe: 0.7, vce: 3.0)
        if ib > 0
          assert_in_delta 100.0, ic / ib, 1.0
        end
      end

      def test_base_current_cutoff
        # Base current should be 0 in cutoff.
        t = NPN.new
        assert_equal 0.0, t.base_current(vbe: 0.0, vce: 5.0)
      end

      def test_transconductance_cutoff
        # gm should be 0 in cutoff.
        t = NPN.new
        assert_equal 0.0, t.transconductance(vbe: 0.0, vce: 5.0)
      end

      def test_transconductance_active
        # gm should be positive in active region.
        t = NPN.new
        gm = t.transconductance(vbe: 0.7, vce: 3.0)
        assert gm > 0
      end

      def test_custom_beta
        # Custom beta should affect current gain.
        t_low = NPN.new(BJTParams.new(beta: 50))
        t_high = NPN.new(BJTParams.new(beta: 200))
        # Same Ic (determined by Is and Vbe), different Ib
        ib_low = t_low.base_current(vbe: 0.7, vce: 3.0)
        ib_high = t_high.base_current(vbe: 0.7, vce: 3.0)
        assert ib_low > ib_high # Lower beta = more base current
      end

      def test_saturation_boundary
        # At Vce = Vce_sat, transistor is in saturation.
        t = NPN.new
        assert_equal BJTRegion::SATURATION, t.region(vbe: 0.7, vce: 0.2)
      end

      def test_active_boundary
        # Just above Vce_sat, transistor is in active.
        t = NPN.new
        assert_equal BJTRegion::ACTIVE, t.region(vbe: 0.7, vce: 0.3)
      end
    end

    class TestPNP < Minitest::Test
      # Test PNP transistor operating regions and electrical behavior.

      def test_cutoff_region
        # PNP with small |Vbe| should be OFF.
        t = PNP.new
        assert_equal BJTRegion::CUTOFF, t.region(vbe: 0.0, vce: 0.0)
        assert_equal 0.0, t.collector_current(vbe: 0.0, vce: 0.0)
        refute t.conducting?(vbe: 0.0)
      end

      def test_conducts_with_negative_vbe
        # PNP conducts when |Vbe| >= Vbe_on (Vbe typically negative).
        t = PNP.new
        assert t.conducting?(vbe: -0.7)
        assert_equal BJTRegion::ACTIVE, t.region(vbe: -0.7, vce: -3.0)
      end

      def test_saturation
        # PNP in saturation when |Vce| <= Vce_sat.
        t = PNP.new
        assert_equal BJTRegion::SATURATION, t.region(vbe: -0.7, vce: -0.1)
      end

      def test_drain_current_positive
        # PNP collector current magnitude should be positive.
        t = PNP.new
        ic = t.collector_current(vbe: -0.7, vce: -3.0)
        assert ic > 0
      end

      def test_base_current
        # PNP should have non-zero base current when conducting.
        t = PNP.new
        ib = t.base_current(vbe: -0.7, vce: -3.0)
        assert ib > 0
      end

      def test_cutoff_no_base_current
        # PNP base current should be 0 in cutoff.
        t = PNP.new
        assert_equal 0.0, t.base_current(vbe: 0.0, vce: 0.0)
      end

      def test_transconductance
        # PNP gm should be positive when conducting.
        t = PNP.new
        gm = t.transconductance(vbe: -0.7, vce: -3.0)
        assert gm > 0
      end

      def test_transconductance_cutoff
        # PNP gm should be 0 in cutoff.
        t = PNP.new
        assert_equal 0.0, t.transconductance(vbe: 0.0, vce: 0.0)
      end
    end
  end
end
