package transistors

import (
	"math"
	"testing"
)

// ===========================================================================
// Common-Source Amplifier (MOSFET) Tests
// ===========================================================================

func TestCommonSourceAmp_InvertingGain(t *testing.T) {
	// Common-source amplifier should have negative voltage gain.
	tr := NewNMOS(nil)
	result := AnalyzeCommonSourceAmp(tr, 1.5, 3.3, 10_000, 1e-12)
	if result.VoltageGain >= 0 {
		t.Errorf("VoltageGain = %v, want < 0 (inverting)", result.VoltageGain)
	}
}

func TestCommonSourceAmp_HighInputImpedance(t *testing.T) {
	// MOSFET amplifiers should have very high input impedance.
	tr := NewNMOS(nil)
	result := AnalyzeCommonSourceAmp(tr, 1.5, 3.3, 10_000, 1e-12)
	if result.InputImpedance <= 1e9 {
		t.Errorf("InputImpedance = %v, want > 1e9", result.InputImpedance)
	}
}

func TestCommonSourceAmp_PositiveTransconductance(t *testing.T) {
	tr := NewNMOS(nil)
	result := AnalyzeCommonSourceAmp(tr, 1.5, 3.3, 10_000, 1e-12)
	if result.Transconductance <= 0 {
		t.Errorf("Transconductance = %v, want > 0", result.Transconductance)
	}
}

func TestCommonSourceAmp_PositiveBandwidth(t *testing.T) {
	tr := NewNMOS(nil)
	result := AnalyzeCommonSourceAmp(tr, 1.5, 3.3, 10_000, 1e-12)
	if result.Bandwidth <= 0 {
		t.Errorf("Bandwidth = %v, want > 0", result.Bandwidth)
	}
}

func TestCommonSourceAmp_OperatingPoint(t *testing.T) {
	// Operating point should contain required keys.
	tr := NewNMOS(nil)
	result := AnalyzeCommonSourceAmp(tr, 1.5, 3.3, 10_000, 1e-12)
	for _, key := range []string{"vgs", "vds", "ids", "gm"} {
		if _, ok := result.OperatingPoint[key]; !ok {
			t.Errorf("OperatingPoint missing key %q", key)
		}
	}
}

func TestCommonSourceAmp_HigherRdMoreGain(t *testing.T) {
	// Higher drain resistance should give more voltage gain.
	tr := NewNMOS(nil)
	r1 := AnalyzeCommonSourceAmp(tr, 1.5, 3.3, 5_000, 1e-12)
	r2 := AnalyzeCommonSourceAmp(tr, 1.5, 3.3, 20_000, 1e-12)
	if math.Abs(r2.VoltageGain) <= math.Abs(r1.VoltageGain) {
		t.Errorf("|gain at 20k| (%v) should exceed |gain at 5k| (%v)",
			math.Abs(r2.VoltageGain), math.Abs(r1.VoltageGain))
	}
}

// ===========================================================================
// Common-Emitter Amplifier (BJT) Tests
// ===========================================================================

func TestCommonEmitterAmp_InvertingGain(t *testing.T) {
	// Common-emitter amplifier should have negative voltage gain.
	tr := NewNPN(nil)
	result := AnalyzeCommonEmitterAmp(tr, 0.7, 5.0, 4700, 1e-12)
	if result.VoltageGain >= 0 {
		t.Errorf("VoltageGain = %v, want < 0 (inverting)", result.VoltageGain)
	}
}

func TestCommonEmitterAmp_ModerateInputImpedance(t *testing.T) {
	// BJT amplifiers have moderate input impedance (r_pi).
	tr := NewNPN(nil)
	result := AnalyzeCommonEmitterAmp(tr, 0.7, 5.0, 4700, 1e-12)
	if result.InputImpedance < 100 || result.InputImpedance > 1e6 {
		t.Errorf("InputImpedance = %v, want between 100 and 1e6", result.InputImpedance)
	}
}

func TestCommonEmitterAmp_PositiveTransconductance(t *testing.T) {
	tr := NewNPN(nil)
	result := AnalyzeCommonEmitterAmp(tr, 0.7, 5.0, 4700, 1e-12)
	if result.Transconductance <= 0 {
		t.Errorf("Transconductance = %v, want > 0", result.Transconductance)
	}
}

func TestCommonEmitterAmp_HigherBetaHigherImpedance(t *testing.T) {
	// Higher beta should give higher input impedance.
	pLow := BJTParams{Beta: 50, VbeOn: 0.7, VceSat: 0.2, Is: 1e-14, CBase: 5e-12}
	pHigh := BJTParams{Beta: 200, VbeOn: 0.7, VceSat: 0.2, Is: 1e-14, CBase: 5e-12}
	tLow := NewNPN(&pLow)
	tHigh := NewNPN(&pHigh)
	r1 := AnalyzeCommonEmitterAmp(tLow, 0.7, 5.0, 4700, 1e-12)
	r2 := AnalyzeCommonEmitterAmp(tHigh, 0.7, 5.0, 4700, 1e-12)
	if r2.InputImpedance <= r1.InputImpedance {
		t.Errorf("High-beta impedance (%v) should exceed low-beta (%v)",
			r2.InputImpedance, r1.InputImpedance)
	}
}

func TestCommonEmitterAmp_OperatingPoint(t *testing.T) {
	tr := NewNPN(nil)
	result := AnalyzeCommonEmitterAmp(tr, 0.7, 5.0, 4700, 1e-12)
	for _, key := range []string{"vbe", "vce", "ic", "ib"} {
		if _, ok := result.OperatingPoint[key]; !ok {
			t.Errorf("OperatingPoint missing key %q", key)
		}
	}
}
