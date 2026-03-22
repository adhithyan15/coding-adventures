defmodule CodingAdventures.Transistors.MOSFETTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.Transistors.MOSFET
  alias CodingAdventures.Transistors.Types.MOSFETParams

  # ===========================================================================
  # NMOS Tests
  # ===========================================================================

  describe "NMOS region detection" do
    test "cutoff when Vgs below threshold" do
      assert MOSFET.nmos_region(0.0, 1.0) == :cutoff
      assert MOSFET.nmos_drain_current(0.0, 1.0) == 0.0
      assert MOSFET.nmos_is_conducting?(0.0) == false
    end

    test "cutoff with negative Vgs" do
      assert MOSFET.nmos_region(-1.0, 0.0) == :cutoff
      assert MOSFET.nmos_drain_current(-1.0, 0.0) == 0.0
    end

    test "linear region with low Vds" do
      assert MOSFET.nmos_region(1.5, 0.1) == :linear
      ids = MOSFET.nmos_drain_current(1.5, 0.1)
      assert ids > 0
    end

    test "saturation region with high Vds" do
      assert MOSFET.nmos_region(1.0, 3.0) == :saturation
      ids = MOSFET.nmos_drain_current(1.0, 3.0)
      assert ids > 0
    end

    test "saturation current independent of Vds" do
      ids_1 = MOSFET.nmos_drain_current(1.5, 3.0)
      ids_2 = MOSFET.nmos_drain_current(1.5, 5.0)
      assert abs(ids_1 - ids_2) < 1.0e-10
    end

    test "linear current increases with Vds" do
      ids_low = MOSFET.nmos_drain_current(3.0, 0.1)
      ids_high = MOSFET.nmos_drain_current(3.0, 0.5)
      assert ids_high > ids_low
    end

    test "boundary: just above Vth with small Vds is linear" do
      assert MOSFET.nmos_region(0.5, 0.01) == :linear
    end

    test "boundary: at Vds = Vgs - Vth enters saturation" do
      vgs = 1.0
      vth = 0.4
      vds = vgs - vth
      assert MOSFET.nmos_region(vgs, vds) == :saturation
    end
  end

  describe "NMOS is_conducting?" do
    test "below Vth returns false" do
      assert MOSFET.nmos_is_conducting?(0.3) == false
    end

    test "at Vth returns true" do
      assert MOSFET.nmos_is_conducting?(0.4) == true
    end

    test "above Vth returns true" do
      assert MOSFET.nmos_is_conducting?(1.0) == true
    end
  end

  describe "NMOS output voltage" do
    test "ON pulls to GND" do
      assert MOSFET.nmos_output_voltage(3.3, 3.3) == 0.0
    end

    test "OFF stays at Vdd" do
      assert MOSFET.nmos_output_voltage(0.0, 3.3) == 3.3
    end
  end

  describe "NMOS custom params" do
    test "custom threshold is respected" do
      params = %MOSFETParams{vth: 0.7, k: 0.002}
      assert MOSFET.nmos_is_conducting?(0.5, params) == false
      assert MOSFET.nmos_is_conducting?(0.7, params) == true
    end
  end

  describe "NMOS transconductance" do
    test "zero in cutoff" do
      assert MOSFET.nmos_transconductance(0.0, 1.0) == 0.0
    end

    test "positive in saturation" do
      gm = MOSFET.nmos_transconductance(1.5, 3.0)
      assert gm > 0
    end
  end

  # ===========================================================================
  # PMOS Tests
  # ===========================================================================

  describe "PMOS region detection" do
    test "cutoff when Vgs is zero" do
      assert MOSFET.pmos_region(0.0, 0.0) == :cutoff
      assert MOSFET.pmos_is_conducting?(0.0) == false
    end

    test "conducts with negative Vgs" do
      assert MOSFET.pmos_is_conducting?(-1.5) == true
      assert MOSFET.pmos_region(-1.5, -3.0) == :saturation
    end

    test "linear region with small |Vds|" do
      assert MOSFET.pmos_region(-1.5, -0.1) == :linear
    end

    test "drain current positive when conducting" do
      ids = MOSFET.pmos_drain_current(-1.5, -3.0)
      assert ids > 0
    end

    test "cutoff has zero current" do
      assert MOSFET.pmos_drain_current(0.0, -1.0) == 0.0
    end
  end

  describe "PMOS output voltage" do
    test "ON pulls to Vdd" do
      assert MOSFET.pmos_output_voltage(-3.3, 3.3) == 3.3
    end

    test "OFF falls to GND" do
      assert MOSFET.pmos_output_voltage(0.0, 3.3) == 0.0
    end
  end

  describe "PMOS complementary to NMOS" do
    test "NMOS ON when PMOS OFF and vice versa" do
      vdd = 3.3

      # Input HIGH: NMOS ON, PMOS OFF
      assert MOSFET.nmos_is_conducting?(vdd) == true
      assert MOSFET.pmos_is_conducting?(0.0) == false

      # Input LOW: NMOS OFF, PMOS ON
      assert MOSFET.nmos_is_conducting?(0.0) == false
      assert MOSFET.pmos_is_conducting?(-vdd) == true
    end
  end

  describe "PMOS transconductance" do
    test "zero in cutoff" do
      assert MOSFET.pmos_transconductance(0.0, 0.0) == 0.0
    end

    test "positive when conducting" do
      gm = MOSFET.pmos_transconductance(-1.5, -3.0)
      assert gm > 0
    end
  end
end
