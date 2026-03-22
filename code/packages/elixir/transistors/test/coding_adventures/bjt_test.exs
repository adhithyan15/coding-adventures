defmodule CodingAdventures.Transistors.BJTTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.Transistors.BJT
  alias CodingAdventures.Transistors.Types.BJTParams

  # ===========================================================================
  # NPN Tests
  # ===========================================================================

  describe "NPN region detection" do
    test "cutoff when Vbe below threshold" do
      assert BJT.npn_region(0.0, 5.0) == :cutoff
      assert BJT.npn_collector_current(0.0, 5.0) == 0.0
      assert BJT.npn_is_conducting?(0.0) == false
    end

    test "active region" do
      assert BJT.npn_region(0.7, 3.0) == :active
      ic = BJT.npn_collector_current(0.7, 3.0)
      assert ic > 0
    end

    test "saturation region" do
      assert BJT.npn_region(0.7, 0.1) == :saturation
    end

    test "saturation boundary at Vce_sat" do
      assert BJT.npn_region(0.7, 0.2) == :saturation
    end

    test "active boundary just above Vce_sat" do
      assert BJT.npn_region(0.7, 0.3) == :active
    end
  end

  describe "NPN is_conducting?" do
    test "below Vbe_on returns false" do
      assert BJT.npn_is_conducting?(0.5) == false
    end

    test "at Vbe_on returns true" do
      assert BJT.npn_is_conducting?(0.7) == true
    end

    test "above Vbe_on returns true" do
      assert BJT.npn_is_conducting?(1.0) == true
    end
  end

  describe "NPN current gain" do
    test "Ic/Ib approximately equals beta" do
      params = %BJTParams{beta: 100.0}
      ic = BJT.npn_collector_current(0.7, 3.0, params)
      ib = BJT.npn_base_current(0.7, 3.0, params)

      if ib > 0 do
        assert abs(ic / ib - 100.0) < 1.0
      end
    end
  end

  describe "NPN base current" do
    test "zero in cutoff" do
      assert BJT.npn_base_current(0.0, 5.0) == 0.0
    end
  end

  describe "NPN transconductance" do
    test "zero in cutoff" do
      assert BJT.npn_transconductance(0.0, 5.0) == 0.0
    end

    test "positive in active region" do
      gm = BJT.npn_transconductance(0.7, 3.0)
      assert gm > 0
    end
  end

  describe "NPN custom beta" do
    test "different beta affects base current" do
      params_low = %BJTParams{beta: 50.0}
      params_high = %BJTParams{beta: 200.0}
      ib_low = BJT.npn_base_current(0.7, 3.0, params_low)
      ib_high = BJT.npn_base_current(0.7, 3.0, params_high)
      # Lower beta = more base current needed
      assert ib_low > ib_high
    end
  end

  # ===========================================================================
  # PNP Tests
  # ===========================================================================

  describe "PNP region detection" do
    test "cutoff when |Vbe| is small" do
      assert BJT.pnp_region(0.0, 0.0) == :cutoff
      assert BJT.pnp_collector_current(0.0, 0.0) == 0.0
      assert BJT.pnp_is_conducting?(0.0) == false
    end

    test "conducts with negative Vbe" do
      assert BJT.pnp_is_conducting?(-0.7) == true
      assert BJT.pnp_region(-0.7, -3.0) == :active
    end

    test "saturation when |Vce| <= Vce_sat" do
      assert BJT.pnp_region(-0.7, -0.1) == :saturation
    end

    test "collector current positive when conducting" do
      ic = BJT.pnp_collector_current(-0.7, -3.0)
      assert ic > 0
    end

    test "base current positive when conducting" do
      ib = BJT.pnp_base_current(-0.7, -3.0)
      assert ib > 0
    end

    test "base current zero in cutoff" do
      assert BJT.pnp_base_current(0.0, 0.0) == 0.0
    end
  end

  describe "PNP transconductance" do
    test "positive when conducting" do
      gm = BJT.pnp_transconductance(-0.7, -3.0)
      assert gm > 0
    end

    test "zero in cutoff" do
      assert BJT.pnp_transconductance(0.0, 0.0) == 0.0
    end
  end
end
