defmodule CodingAdventures.Transistors.AnalysisTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.Transistors.Analysis

  # ===========================================================================
  # Noise Margin Tests
  # ===========================================================================

  describe "CMOS noise margins" do
    test "positive margins" do
      nm = Analysis.compute_noise_margins(:cmos_inverter)
      assert nm.nml > 0
      assert nm.nmh > 0
    end

    test "roughly symmetric" do
      nm = Analysis.compute_noise_margins(:cmos_inverter)
      assert abs(nm.nml - nm.nmh) < nm.nml * 0.5
    end

    test "VOL near zero" do
      nm = Analysis.compute_noise_margins(:cmos_inverter)
      assert nm.vol < 0.1
    end
  end

  describe "TTL noise margins" do
    test "positive margins" do
      nm = Analysis.compute_noise_margins(:ttl_nand)
      assert nm.nml > 0
      assert nm.nmh > 0
    end

    test "VOL near Vce_sat" do
      nm = Analysis.compute_noise_margins(:ttl_nand)
      assert nm.vol < 0.5
    end
  end

  # ===========================================================================
  # Power Analysis Tests
  # ===========================================================================

  describe "CMOS power" do
    test "near-zero static power" do
      power = Analysis.analyze_power(:cmos_inverter)
      assert power.static_power < 1.0e-9
    end

    test "positive dynamic power" do
      power = Analysis.analyze_power(:cmos_inverter, frequency: 1.0e9)
      assert power.dynamic_power > 0
    end

    test "total power is static + dynamic" do
      power = Analysis.analyze_power(:cmos_inverter, frequency: 1.0e9)
      assert abs(power.total_power - (power.static_power + power.dynamic_power)) < 1.0e-15
    end

    test "positive energy per switch" do
      power = Analysis.analyze_power(:cmos_inverter)
      assert power.energy_per_switch > 0
    end

    test "CMOS NAND has zero static power" do
      power = Analysis.analyze_power(:cmos_nand)
      assert power.static_power == 0.0
    end

    test "CMOS NOR has zero static power" do
      power = Analysis.analyze_power(:cmos_nor)
      assert power.static_power == 0.0
    end
  end

  describe "TTL power" do
    test "significant static power (milliwatts)" do
      power = Analysis.analyze_power(:ttl_nand)
      assert power.static_power > 1.0e-3
    end
  end

  # ===========================================================================
  # Timing Analysis Tests
  # ===========================================================================

  describe "CMOS timing" do
    test "positive propagation delays" do
      timing = Analysis.analyze_timing(:cmos_inverter)
      assert timing.tphl > 0
      assert timing.tplh > 0
      assert timing.tpd > 0
    end

    test "tpd is average of tphl and tplh" do
      timing = Analysis.analyze_timing(:cmos_inverter)
      expected = (timing.tphl + timing.tplh) / 2.0
      assert abs(timing.tpd - expected) < 1.0e-20
    end

    test "CMOS faster than TTL" do
      cmos_timing = Analysis.analyze_timing(:cmos_inverter)
      ttl_timing = Analysis.analyze_timing(:ttl_nand)
      assert cmos_timing.tpd < ttl_timing.tpd
    end

    test "positive rise and fall times" do
      timing = Analysis.analyze_timing(:cmos_inverter)
      assert timing.rise_time > 0
      assert timing.fall_time > 0
    end

    test "positive max frequency" do
      timing = Analysis.analyze_timing(:cmos_inverter)
      assert timing.max_frequency > 0
    end

    test "CMOS NAND timing works" do
      timing = Analysis.analyze_timing(:cmos_nand)
      assert timing.tpd > 0
    end

    test "CMOS NOR timing works" do
      timing = Analysis.analyze_timing(:cmos_nor)
      assert timing.tpd > 0
    end
  end

  # ===========================================================================
  # Comparison and Scaling Tests
  # ===========================================================================

  describe "CMOS vs TTL comparison" do
    test "returns both CMOS and TTL data" do
      result = Analysis.compare_cmos_vs_ttl()
      assert Map.has_key?(result, "cmos")
      assert Map.has_key?(result, "ttl")
    end

    test "CMOS has less static power than TTL" do
      result = Analysis.compare_cmos_vs_ttl()
      assert result["cmos"]["static_power_w"] < result["ttl"]["static_power_w"]
    end
  end

  describe "CMOS scaling" do
    test "returns a list of maps" do
      result = Analysis.demonstrate_cmos_scaling()
      assert is_list(result)
      assert length(result) > 0
    end

    test "default produces 6 technology nodes" do
      result = Analysis.demonstrate_cmos_scaling()
      assert length(result) == 6
    end

    test "custom nodes are respected" do
      result = Analysis.demonstrate_cmos_scaling([180.0e-9, 45.0e-9])
      assert length(result) == 2
    end

    test "Vdd decreases with scaling" do
      result = Analysis.demonstrate_cmos_scaling()
      first_vdd = hd(result)["vdd_v"]
      last_vdd = List.last(result)["vdd_v"]
      assert first_vdd > last_vdd
    end

    test "each entry has expected keys" do
      result = Analysis.demonstrate_cmos_scaling([180.0e-9])
      entry = hd(result)
      assert Map.has_key?(entry, "node_nm")
      assert Map.has_key?(entry, "vdd_v")
      assert Map.has_key?(entry, "vth_v")
      assert Map.has_key?(entry, "propagation_delay_s")
      assert Map.has_key?(entry, "dynamic_power_w")
      assert Map.has_key?(entry, "leakage_current_a")
    end
  end
end
