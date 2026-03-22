defmodule CodingAdventures.Transistors.AmplifierTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.Transistors.Amplifier
  alias CodingAdventures.Transistors.Types.BJTParams

  # ===========================================================================
  # Common-Source Amplifier (MOSFET) Tests
  # ===========================================================================

  describe "Common-source amplifier" do
    test "has negative (inverting) voltage gain" do
      result = Amplifier.analyze_common_source(1.5, 3.3, 10_000)
      assert result.voltage_gain < 0
    end

    test "has very high input impedance" do
      result = Amplifier.analyze_common_source(1.5, 3.3, 10_000)
      assert result.input_impedance > 1.0e9
    end

    test "has positive transconductance" do
      result = Amplifier.analyze_common_source(1.5, 3.3, 10_000)
      assert result.transconductance > 0
    end

    test "has positive bandwidth" do
      result = Amplifier.analyze_common_source(1.5, 3.3, 10_000)
      assert result.bandwidth > 0
    end

    test "operating point contains required keys" do
      result = Amplifier.analyze_common_source(1.5, 3.3, 10_000)
      assert Map.has_key?(result.operating_point, "vgs")
      assert Map.has_key?(result.operating_point, "vds")
      assert Map.has_key?(result.operating_point, "ids")
      assert Map.has_key?(result.operating_point, "gm")
    end

    test "higher Rd gives more voltage gain" do
      r1 = Amplifier.analyze_common_source(1.5, 3.3, 5_000)
      r2 = Amplifier.analyze_common_source(1.5, 3.3, 20_000)
      assert abs(r2.voltage_gain) > abs(r1.voltage_gain)
    end
  end

  # ===========================================================================
  # Common-Emitter Amplifier (BJT) Tests
  # ===========================================================================

  describe "Common-emitter amplifier" do
    test "has negative (inverting) voltage gain" do
      result = Amplifier.analyze_common_emitter(0.7, 5.0, 4700)
      assert result.voltage_gain < 0
    end

    test "has moderate input impedance" do
      result = Amplifier.analyze_common_emitter(0.7, 5.0, 4700)
      # r_pi should be in the hundreds to hundreds-of-thousands ohm range
      assert result.input_impedance > 100
      assert result.input_impedance < 1.0e6
    end

    test "has positive transconductance" do
      result = Amplifier.analyze_common_emitter(0.7, 5.0, 4700)
      assert result.transconductance > 0
    end

    test "higher beta gives higher input impedance" do
      params_low = %BJTParams{beta: 50.0}
      params_high = %BJTParams{beta: 200.0}
      r1 = Amplifier.analyze_common_emitter(0.7, 5.0, 4700, 1.0e-12, params_low)
      r2 = Amplifier.analyze_common_emitter(0.7, 5.0, 4700, 1.0e-12, params_high)
      assert r2.input_impedance > r1.input_impedance
    end

    test "operating point contains required keys" do
      result = Amplifier.analyze_common_emitter(0.7, 5.0, 4700)
      assert Map.has_key?(result.operating_point, "vbe")
      assert Map.has_key?(result.operating_point, "vce")
      assert Map.has_key?(result.operating_point, "ic")
      assert Map.has_key?(result.operating_point, "ib")
    end
  end
end
