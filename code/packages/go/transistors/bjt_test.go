package transistors

import (
	"math"
	"testing"
)

// ===========================================================================
// NPN Tests
// ===========================================================================

func TestNPN_CutoffRegion(t *testing.T) {
	// Vbe below threshold -> no current.
	tr := NewNPN(nil)
	if r := tr.Region(0.0, 5.0); r != BJTCutoff {
		t.Errorf("Region(0.0, 5.0) = %q, want %q", r, BJTCutoff)
	}
	if c := tr.CollectorCurrent(0.0, 5.0); c != 0.0 {
		t.Errorf("CollectorCurrent(0.0, 5.0) = %v, want 0.0", c)
	}
	if tr.IsConducting(0.0) {
		t.Error("IsConducting(0.0) = true, want false")
	}
}

func TestNPN_ActiveRegion(t *testing.T) {
	// Vbe at threshold, Vce > Vce_sat -> active (amplifier).
	tr := NewNPN(nil)
	if r := tr.Region(0.7, 3.0); r != BJTActive {
		t.Errorf("Region(0.7, 3.0) = %q, want %q", r, BJTActive)
	}
	ic := tr.CollectorCurrent(0.7, 3.0)
	if ic <= 0 {
		t.Errorf("CollectorCurrent(0.7, 3.0) = %v, want > 0", ic)
	}
}

func TestNPN_SaturationRegion(t *testing.T) {
	// Vbe at threshold, Vce <= Vce_sat -> saturated (switch ON).
	tr := NewNPN(nil)
	if r := tr.Region(0.7, 0.1); r != BJTSaturation {
		t.Errorf("Region(0.7, 0.1) = %q, want %q", r, BJTSaturation)
	}
}

func TestNPN_IsConducting(t *testing.T) {
	tr := NewNPN(nil)
	if tr.IsConducting(0.5) {
		t.Error("IsConducting(0.5) should be false")
	}
	if !tr.IsConducting(0.7) {
		t.Error("IsConducting(0.7) should be true")
	}
	if !tr.IsConducting(1.0) {
		t.Error("IsConducting(1.0) should be true")
	}
}

func TestNPN_CurrentGain(t *testing.T) {
	// In active region, Ic should be approximately beta * Ib.
	params := BJTParams{Beta: 100, VbeOn: 0.7, VceSat: 0.2, Is: 1e-14, CBase: 5e-12}
	tr := NewNPN(&params)
	ic := tr.CollectorCurrent(0.7, 3.0)
	ib := tr.BaseCurrent(0.7, 3.0)
	if ib > 0 {
		ratio := ic / ib
		if math.Abs(ratio-100.0) > 1.0 {
			t.Errorf("ic/ib = %v, want ~100.0", ratio)
		}
	}
}

func TestNPN_BaseCurrentCutoff(t *testing.T) {
	// Base current should be 0 in cutoff.
	tr := NewNPN(nil)
	if ib := tr.BaseCurrent(0.0, 5.0); ib != 0.0 {
		t.Errorf("BaseCurrent(0.0, 5.0) = %v, want 0.0", ib)
	}
}

func TestNPN_TransconductanceCutoff(t *testing.T) {
	// gm should be 0 in cutoff.
	tr := NewNPN(nil)
	if gm := tr.Transconductance(0.0, 5.0); gm != 0.0 {
		t.Errorf("Transconductance(0.0, 5.0) = %v, want 0.0", gm)
	}
}

func TestNPN_TransconductanceActive(t *testing.T) {
	// gm should be positive in active region.
	tr := NewNPN(nil)
	gm := tr.Transconductance(0.7, 3.0)
	if gm <= 0 {
		t.Errorf("Transconductance(0.7, 3.0) = %v, want > 0", gm)
	}
}

func TestNPN_CustomBeta(t *testing.T) {
	// Custom beta should affect current gain (base current changes).
	pLow := BJTParams{Beta: 50, VbeOn: 0.7, VceSat: 0.2, Is: 1e-14, CBase: 5e-12}
	pHigh := BJTParams{Beta: 200, VbeOn: 0.7, VceSat: 0.2, Is: 1e-14, CBase: 5e-12}
	tLow := NewNPN(&pLow)
	tHigh := NewNPN(&pHigh)
	ibLow := tLow.BaseCurrent(0.7, 3.0)
	ibHigh := tHigh.BaseCurrent(0.7, 3.0)
	// Lower beta = more base current for the same Ic
	if ibLow <= ibHigh {
		t.Errorf("ibLow (%v) should exceed ibHigh (%v)", ibLow, ibHigh)
	}
}

func TestNPN_SaturationBoundary(t *testing.T) {
	// At Vce = Vce_sat, transistor is in saturation.
	tr := NewNPN(nil)
	if r := tr.Region(0.7, 0.2); r != BJTSaturation {
		t.Errorf("Region(0.7, 0.2) = %q, want %q", r, BJTSaturation)
	}
}

func TestNPN_ActiveBoundary(t *testing.T) {
	// Just above Vce_sat, transistor is in active.
	tr := NewNPN(nil)
	if r := tr.Region(0.7, 0.3); r != BJTActive {
		t.Errorf("Region(0.7, 0.3) = %q, want %q", r, BJTActive)
	}
}

func TestNPN_NilParamsUsesDefaults(t *testing.T) {
	tr := NewNPN(nil)
	defaults := DefaultBJTParams()
	if tr.Params.Beta != defaults.Beta {
		t.Errorf("Beta = %v, want %v", tr.Params.Beta, defaults.Beta)
	}
}

// ===========================================================================
// PNP Tests
// ===========================================================================

func TestPNP_CutoffRegion(t *testing.T) {
	// PNP with small |Vbe| should be OFF.
	tr := NewPNP(nil)
	if r := tr.Region(0.0, 0.0); r != BJTCutoff {
		t.Errorf("Region(0.0, 0.0) = %q, want %q", r, BJTCutoff)
	}
	if c := tr.CollectorCurrent(0.0, 0.0); c != 0.0 {
		t.Errorf("CollectorCurrent(0.0, 0.0) = %v, want 0.0", c)
	}
	if tr.IsConducting(0.0) {
		t.Error("IsConducting(0.0) = true, want false")
	}
}

func TestPNP_ConductsWithNegativeVbe(t *testing.T) {
	// PNP conducts when |Vbe| >= Vbe_on.
	tr := NewPNP(nil)
	if !tr.IsConducting(-0.7) {
		t.Error("IsConducting(-0.7) should be true")
	}
	if r := tr.Region(-0.7, -3.0); r != BJTActive {
		t.Errorf("Region(-0.7, -3.0) = %q, want %q", r, BJTActive)
	}
}

func TestPNP_Saturation(t *testing.T) {
	// PNP in saturation when |Vce| <= Vce_sat.
	tr := NewPNP(nil)
	if r := tr.Region(-0.7, -0.1); r != BJTSaturation {
		t.Errorf("Region(-0.7, -0.1) = %q, want %q", r, BJTSaturation)
	}
}

func TestPNP_CollectorCurrentPositive(t *testing.T) {
	// PNP collector current magnitude should be positive.
	tr := NewPNP(nil)
	ic := tr.CollectorCurrent(-0.7, -3.0)
	if ic <= 0 {
		t.Errorf("CollectorCurrent(-0.7, -3.0) = %v, want > 0", ic)
	}
}

func TestPNP_BaseCurrent(t *testing.T) {
	// PNP should have non-zero base current when conducting.
	tr := NewPNP(nil)
	ib := tr.BaseCurrent(-0.7, -3.0)
	if ib <= 0 {
		t.Errorf("BaseCurrent(-0.7, -3.0) = %v, want > 0", ib)
	}
}

func TestPNP_CutoffNoBaseCurrent(t *testing.T) {
	// PNP base current should be 0 in cutoff.
	tr := NewPNP(nil)
	if ib := tr.BaseCurrent(0.0, 0.0); ib != 0.0 {
		t.Errorf("BaseCurrent(0.0, 0.0) = %v, want 0.0", ib)
	}
}

func TestPNP_Transconductance(t *testing.T) {
	// PNP gm should be positive when conducting.
	tr := NewPNP(nil)
	gm := tr.Transconductance(-0.7, -3.0)
	if gm <= 0 {
		t.Errorf("Transconductance(-0.7, -3.0) = %v, want > 0", gm)
	}
}

func TestPNP_TransconductanceCutoff(t *testing.T) {
	// PNP gm should be 0 in cutoff.
	tr := NewPNP(nil)
	if gm := tr.Transconductance(0.0, 0.0); gm != 0.0 {
		t.Errorf("Transconductance(0.0, 0.0) = %v, want 0.0", gm)
	}
}
