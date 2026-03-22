package transistors

import (
	"math"
	"testing"
)

// ===========================================================================
// Noise Margin Tests
// ===========================================================================

func TestNoiseMargins_CMOSPositiveMargins(t *testing.T) {
	nm, err := ComputeNoiseMargins(NewCMOSInverter(nil, nil, nil))
	if err != nil {
		t.Fatal(err)
	}
	if nm.NML <= 0 {
		t.Errorf("NML = %v, want > 0", nm.NML)
	}
	if nm.NMH <= 0 {
		t.Errorf("NMH = %v, want > 0", nm.NMH)
	}
}

func TestNoiseMargins_CMOSSymmetric(t *testing.T) {
	// CMOS noise margins should be roughly symmetric.
	nm, err := ComputeNoiseMargins(NewCMOSInverter(nil, nil, nil))
	if err != nil {
		t.Fatal(err)
	}
	if math.Abs(nm.NML-nm.NMH) > nm.NML*0.5 {
		t.Errorf("NML=%v, NMH=%v: asymmetry too large", nm.NML, nm.NMH)
	}
}

func TestNoiseMargins_TTLPositiveMargins(t *testing.T) {
	nm, err := ComputeNoiseMargins(NewTTLNand(5.0, nil))
	if err != nil {
		t.Fatal(err)
	}
	if nm.NML <= 0 {
		t.Errorf("NML = %v, want > 0", nm.NML)
	}
	if nm.NMH <= 0 {
		t.Errorf("NMH = %v, want > 0", nm.NMH)
	}
}

func TestNoiseMargins_CMOSVolNearZero(t *testing.T) {
	nm, err := ComputeNoiseMargins(NewCMOSInverter(nil, nil, nil))
	if err != nil {
		t.Fatal(err)
	}
	if nm.VOL >= 0.1 {
		t.Errorf("VOL = %v, want < 0.1", nm.VOL)
	}
}

func TestNoiseMargins_TTLVolVceSat(t *testing.T) {
	nm, err := ComputeNoiseMargins(NewTTLNand(5.0, nil))
	if err != nil {
		t.Fatal(err)
	}
	if nm.VOL >= 0.5 {
		t.Errorf("VOL = %v, want < 0.5", nm.VOL)
	}
}

func TestNoiseMargins_UnsupportedType(t *testing.T) {
	_, err := ComputeNoiseMargins("invalid")
	if err == nil {
		t.Error("ComputeNoiseMargins on string should return error")
	}
}

// ===========================================================================
// Power Analysis Tests
// ===========================================================================

func TestPowerAnalysis_CMOSZeroStaticPower(t *testing.T) {
	power, err := AnalyzePower(NewCMOSInverter(nil, nil, nil), 1e9, 1e-12, 0.5)
	if err != nil {
		t.Fatal(err)
	}
	if power.StaticPower > 1e-9 {
		t.Errorf("StaticPower = %v, want < 1e-9", power.StaticPower)
	}
}

func TestPowerAnalysis_TTLSignificantStaticPower(t *testing.T) {
	power, err := AnalyzePower(NewTTLNand(5.0, nil), 1e9, 1e-12, 0.5)
	if err != nil {
		t.Fatal(err)
	}
	if power.StaticPower <= 1e-3 {
		t.Errorf("StaticPower = %v, want > 1e-3", power.StaticPower)
	}
}

func TestPowerAnalysis_PositiveDynamicPower(t *testing.T) {
	power, err := AnalyzePower(NewCMOSInverter(nil, nil, nil), 1e9, 1e-12, 0.5)
	if err != nil {
		t.Fatal(err)
	}
	if power.DynamicPower <= 0 {
		t.Errorf("DynamicPower = %v, want > 0", power.DynamicPower)
	}
}

func TestPowerAnalysis_TotalPowerSum(t *testing.T) {
	power, err := AnalyzePower(NewCMOSInverter(nil, nil, nil), 1e9, 1e-12, 0.5)
	if err != nil {
		t.Fatal(err)
	}
	expected := power.StaticPower + power.DynamicPower
	if math.Abs(power.TotalPower-expected) > 1e-15 {
		t.Errorf("TotalPower = %v, want %v (static+dynamic)", power.TotalPower, expected)
	}
}

func TestPowerAnalysis_EnergyPerSwitchPositive(t *testing.T) {
	power, err := AnalyzePower(NewCMOSInverter(nil, nil, nil), 1e9, 1e-12, 0.5)
	if err != nil {
		t.Fatal(err)
	}
	if power.EnergyPerSwitch <= 0 {
		t.Errorf("EnergyPerSwitch = %v, want > 0", power.EnergyPerSwitch)
	}
}

func TestPowerAnalysis_CMOSNandPower(t *testing.T) {
	power, err := AnalyzePower(NewCMOSNand(nil, nil, nil), 1e9, 1e-12, 0.5)
	if err != nil {
		t.Fatal(err)
	}
	if power.StaticPower != 0.0 {
		t.Errorf("StaticPower = %v, want 0.0", power.StaticPower)
	}
}

func TestPowerAnalysis_CMOSNorPower(t *testing.T) {
	power, err := AnalyzePower(NewCMOSNor(nil, nil, nil), 1e9, 1e-12, 0.5)
	if err != nil {
		t.Fatal(err)
	}
	if power.StaticPower != 0.0 {
		t.Errorf("StaticPower = %v, want 0.0", power.StaticPower)
	}
}

func TestPowerAnalysis_UnsupportedType(t *testing.T) {
	_, err := AnalyzePower("invalid", 1e9, 1e-12, 0.5)
	if err == nil {
		t.Error("AnalyzePower on string should return error")
	}
}

// ===========================================================================
// Timing Analysis Tests
// ===========================================================================

func TestTimingAnalysis_CMOSPositiveDelays(t *testing.T) {
	timing, err := AnalyzeTiming(NewCMOSInverter(nil, nil, nil), 1e-12)
	if err != nil {
		t.Fatal(err)
	}
	if timing.Tphl <= 0 {
		t.Errorf("Tphl = %v, want > 0", timing.Tphl)
	}
	if timing.Tplh <= 0 {
		t.Errorf("Tplh = %v, want > 0", timing.Tplh)
	}
	if timing.Tpd <= 0 {
		t.Errorf("Tpd = %v, want > 0", timing.Tpd)
	}
}

func TestTimingAnalysis_TpdIsAverage(t *testing.T) {
	timing, err := AnalyzeTiming(NewCMOSInverter(nil, nil, nil), 1e-12)
	if err != nil {
		t.Fatal(err)
	}
	expected := (timing.Tphl + timing.Tplh) / 2.0
	if math.Abs(timing.Tpd-expected) > 1e-20 {
		t.Errorf("Tpd = %v, want %v", timing.Tpd, expected)
	}
}

func TestTimingAnalysis_CMOSFasterThanTTL(t *testing.T) {
	cmosTiming, err := AnalyzeTiming(NewCMOSInverter(nil, nil, nil), 1e-12)
	if err != nil {
		t.Fatal(err)
	}
	ttlTiming, err := AnalyzeTiming(NewTTLNand(5.0, nil), 1e-12)
	if err != nil {
		t.Fatal(err)
	}
	if cmosTiming.Tpd >= ttlTiming.Tpd {
		t.Errorf("CMOS Tpd (%v) should be < TTL Tpd (%v)",
			cmosTiming.Tpd, ttlTiming.Tpd)
	}
}

func TestTimingAnalysis_PositiveRiseFall(t *testing.T) {
	timing, err := AnalyzeTiming(NewCMOSInverter(nil, nil, nil), 1e-12)
	if err != nil {
		t.Fatal(err)
	}
	if timing.RiseTime <= 0 {
		t.Errorf("RiseTime = %v, want > 0", timing.RiseTime)
	}
	if timing.FallTime <= 0 {
		t.Errorf("FallTime = %v, want > 0", timing.FallTime)
	}
}

func TestTimingAnalysis_MaxFrequencyPositive(t *testing.T) {
	timing, err := AnalyzeTiming(NewCMOSInverter(nil, nil, nil), 1e-12)
	if err != nil {
		t.Fatal(err)
	}
	if timing.MaxFrequency <= 0 {
		t.Errorf("MaxFrequency = %v, want > 0", timing.MaxFrequency)
	}
}

func TestTimingAnalysis_CMOSNandTiming(t *testing.T) {
	timing, err := AnalyzeTiming(NewCMOSNand(nil, nil, nil), 1e-12)
	if err != nil {
		t.Fatal(err)
	}
	if timing.Tpd <= 0 {
		t.Errorf("Tpd = %v, want > 0", timing.Tpd)
	}
}

func TestTimingAnalysis_CMOSNorTiming(t *testing.T) {
	timing, err := AnalyzeTiming(NewCMOSNor(nil, nil, nil), 1e-12)
	if err != nil {
		t.Fatal(err)
	}
	if timing.Tpd <= 0 {
		t.Errorf("Tpd = %v, want > 0", timing.Tpd)
	}
}

func TestTimingAnalysis_UnsupportedType(t *testing.T) {
	_, err := AnalyzeTiming("invalid", 1e-12)
	if err == nil {
		t.Error("AnalyzeTiming on string should return error")
	}
}

// ===========================================================================
// Comparison Utility Tests
// ===========================================================================

func TestCompareCMOSvsTTL_ReturnsBoth(t *testing.T) {
	result := CompareCMOSvsTTL(1e6, 1e-12)
	if _, ok := result["cmos"]; !ok {
		t.Error("missing 'cmos' key")
	}
	if _, ok := result["ttl"]; !ok {
		t.Error("missing 'ttl' key")
	}
}

func TestCompareCMOSvsTTL_LessStaticPower(t *testing.T) {
	result := CompareCMOSvsTTL(1e6, 1e-12)
	if result["cmos"]["static_power_w"] >= result["ttl"]["static_power_w"] {
		t.Errorf("CMOS static power (%v) should be < TTL (%v)",
			result["cmos"]["static_power_w"], result["ttl"]["static_power_w"])
	}
}

func TestDemonstrateCMOSScaling_ReturnsList(t *testing.T) {
	result := DemonstrateCMOSScaling(nil)
	if len(result) == 0 {
		t.Error("DemonstrateCMOSScaling returned empty slice")
	}
}

func TestDemonstrateCMOSScaling_DefaultNodes(t *testing.T) {
	// Default should produce 6 technology nodes.
	result := DemonstrateCMOSScaling(nil)
	if len(result) != 6 {
		t.Errorf("len = %d, want 6", len(result))
	}
}

func TestDemonstrateCMOSScaling_CustomNodes(t *testing.T) {
	result := DemonstrateCMOSScaling([]float64{180e-9, 45e-9})
	if len(result) != 2 {
		t.Errorf("len = %d, want 2", len(result))
	}
}

func TestDemonstrateCMOSScaling_VddDecreases(t *testing.T) {
	// Supply voltage should generally decrease with scaling.
	result := DemonstrateCMOSScaling(nil)
	if result[0]["vdd_v"] <= result[len(result)-1]["vdd_v"] {
		t.Errorf("180nm Vdd (%v) should exceed 3nm Vdd (%v)",
			result[0]["vdd_v"], result[len(result)-1]["vdd_v"])
	}
}

func TestDemonstrateCMOSScaling_HasExpectedKeys(t *testing.T) {
	result := DemonstrateCMOSScaling([]float64{180e-9})
	entry := result[0]
	for _, key := range []string{
		"node_nm", "vdd_v", "vth_v",
		"propagation_delay_s", "dynamic_power_w", "leakage_current_a",
	} {
		if _, ok := entry[key]; !ok {
			t.Errorf("missing key %q", key)
		}
	}
}
