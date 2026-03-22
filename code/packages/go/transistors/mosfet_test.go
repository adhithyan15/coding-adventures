package transistors

import (
	"math"
	"testing"
)

// ===========================================================================
// NMOS Tests
// ===========================================================================

func TestNMOS_CutoffRegion(t *testing.T) {
	// Vgs below threshold -> no current, switch OFF.
	tr := NewNMOS(nil)
	if r := tr.Region(0.0, 1.0); r != MOSFETCutoff {
		t.Errorf("Region(0.0, 1.0) = %q, want %q", r, MOSFETCutoff)
	}
	if c := tr.DrainCurrent(0.0, 1.0); c != 0.0 {
		t.Errorf("DrainCurrent(0.0, 1.0) = %v, want 0.0", c)
	}
	if tr.IsConducting(0.0) {
		t.Error("IsConducting(0.0) = true, want false")
	}
}

func TestNMOS_CutoffNegativeVgs(t *testing.T) {
	// Negative Vgs should also be cutoff.
	tr := NewNMOS(nil)
	if r := tr.Region(-1.0, 0.0); r != MOSFETCutoff {
		t.Errorf("Region(-1.0, 0.0) = %q, want %q", r, MOSFETCutoff)
	}
	if c := tr.DrainCurrent(-1.0, 0.0); c != 0.0 {
		t.Errorf("DrainCurrent(-1.0, 0.0) = %v, want 0.0", c)
	}
}

func TestNMOS_LinearRegion(t *testing.T) {
	// Vgs above threshold, low Vds -> linear region.
	tr := NewNMOS(nil)
	if r := tr.Region(1.5, 0.1); r != MOSFETLinear {
		t.Errorf("Region(1.5, 0.1) = %q, want %q", r, MOSFETLinear)
	}
	ids := tr.DrainCurrent(1.5, 0.1)
	if ids <= 0 {
		t.Errorf("DrainCurrent(1.5, 0.1) = %v, want > 0", ids)
	}
}

func TestNMOS_SaturationRegion(t *testing.T) {
	// Vgs above threshold, high Vds -> saturation.
	tr := NewNMOS(nil)
	if r := tr.Region(1.0, 3.0); r != MOSFETSaturation {
		t.Errorf("Region(1.0, 3.0) = %q, want %q", r, MOSFETSaturation)
	}
	ids := tr.DrainCurrent(1.0, 3.0)
	if ids <= 0 {
		t.Errorf("DrainCurrent(1.0, 3.0) = %v, want > 0", ids)
	}
}

func TestNMOS_SaturationCurrentIndependentOfVds(t *testing.T) {
	// In saturation, current depends only on Vgs, not Vds.
	tr := NewNMOS(nil)
	ids1 := tr.DrainCurrent(1.5, 3.0)
	ids2 := tr.DrainCurrent(1.5, 5.0)
	if math.Abs(ids1-ids2) > 1e-10 {
		t.Errorf("Saturation currents differ: %v vs %v", ids1, ids2)
	}
}

func TestNMOS_LinearCurrentIncreasesWithVds(t *testing.T) {
	// In linear region, current increases with Vds.
	tr := NewNMOS(nil)
	idsLow := tr.DrainCurrent(3.0, 0.1)
	idsHigh := tr.DrainCurrent(3.0, 0.5)
	if idsHigh <= idsLow {
		t.Errorf("ids at Vds=0.5 (%v) should exceed ids at Vds=0.1 (%v)", idsHigh, idsLow)
	}
}

func TestNMOS_IsConducting(t *testing.T) {
	// is_conducting should be true when Vgs >= Vth.
	tr := NewNMOS(nil)
	if tr.IsConducting(0.3) {
		t.Error("IsConducting(0.3) should be false (below Vth=0.4)")
	}
	if !tr.IsConducting(0.4) {
		t.Error("IsConducting(0.4) should be true (at Vth)")
	}
	if !tr.IsConducting(1.0) {
		t.Error("IsConducting(1.0) should be true (above Vth)")
	}
}

func TestNMOS_OutputVoltageOn(t *testing.T) {
	// When ON, output should be pulled to GND.
	tr := NewNMOS(nil)
	if v := tr.OutputVoltage(3.3, 3.3); v != 0.0 {
		t.Errorf("OutputVoltage(3.3, 3.3) = %v, want 0.0", v)
	}
}

func TestNMOS_OutputVoltageOff(t *testing.T) {
	// When OFF, output should be at Vdd.
	tr := NewNMOS(nil)
	if v := tr.OutputVoltage(0.0, 3.3); v != 3.3 {
		t.Errorf("OutputVoltage(0.0, 3.3) = %v, want 3.3", v)
	}
}

func TestNMOS_CustomParams(t *testing.T) {
	// Custom parameters should be respected.
	params := MOSFETParams{Vth: 0.7, K: 0.002, W: 1e-6, L: 180e-9, CGate: 1e-15, CDrain: 0.5e-15}
	tr := NewNMOS(&params)
	if tr.IsConducting(0.5) {
		t.Error("IsConducting(0.5) should be false with Vth=0.7")
	}
	if !tr.IsConducting(0.7) {
		t.Error("IsConducting(0.7) should be true with Vth=0.7")
	}
}

func TestNMOS_TransconductanceCutoff(t *testing.T) {
	// gm should be 0 in cutoff.
	tr := NewNMOS(nil)
	if gm := tr.Transconductance(0.0, 1.0); gm != 0.0 {
		t.Errorf("Transconductance(0.0, 1.0) = %v, want 0.0", gm)
	}
}

func TestNMOS_TransconductanceSaturation(t *testing.T) {
	// gm should be positive in saturation.
	tr := NewNMOS(nil)
	gm := tr.Transconductance(1.5, 3.0)
	if gm <= 0 {
		t.Errorf("Transconductance(1.5, 3.0) = %v, want > 0", gm)
	}
}

func TestNMOS_BoundaryCutoffLinear(t *testing.T) {
	// Just above Vth with small Vds -> linear.
	tr := NewNMOS(nil)
	if r := tr.Region(0.5, 0.01); r != MOSFETLinear {
		t.Errorf("Region(0.5, 0.01) = %q, want %q", r, MOSFETLinear)
	}
}

func TestNMOS_BoundaryLinearSaturation(t *testing.T) {
	// At Vds = Vgs - Vth, transistor enters saturation.
	tr := NewNMOS(nil)
	vgs := 1.0
	vds := vgs - 0.4 // exactly at boundary
	if r := tr.Region(vgs, vds); r != MOSFETSaturation {
		t.Errorf("Region(%v, %v) = %q, want %q", vgs, vds, r, MOSFETSaturation)
	}
}

func TestNMOS_NilParamsUsesDefaults(t *testing.T) {
	tr := NewNMOS(nil)
	defaults := DefaultMOSFETParams()
	if tr.Params.Vth != defaults.Vth {
		t.Errorf("Vth = %v, want %v", tr.Params.Vth, defaults.Vth)
	}
}

// ===========================================================================
// PMOS Tests
// ===========================================================================

func TestPMOS_CutoffWhenVgsZero(t *testing.T) {
	// PMOS with Vgs=0 should be OFF.
	tr := NewPMOS(nil)
	if r := tr.Region(0.0, 0.0); r != MOSFETCutoff {
		t.Errorf("Region(0.0, 0.0) = %q, want %q", r, MOSFETCutoff)
	}
	if tr.IsConducting(0.0) {
		t.Error("IsConducting(0.0) should be false")
	}
}

func TestPMOS_ConductsWhenVgsNegative(t *testing.T) {
	// PMOS conducts when Vgs is sufficiently negative.
	tr := NewPMOS(nil)
	if !tr.IsConducting(-1.5) {
		t.Error("IsConducting(-1.5) should be true")
	}
	if r := tr.Region(-1.5, -3.0); r != MOSFETSaturation {
		t.Errorf("Region(-1.5, -3.0) = %q, want %q", r, MOSFETSaturation)
	}
}

func TestPMOS_LinearRegion(t *testing.T) {
	// PMOS in linear region with small |Vds|.
	tr := NewPMOS(nil)
	if r := tr.Region(-1.5, -0.1); r != MOSFETLinear {
		t.Errorf("Region(-1.5, -0.1) = %q, want %q", r, MOSFETLinear)
	}
}

func TestPMOS_DrainCurrentPositive(t *testing.T) {
	// PMOS drain current magnitude should be positive.
	tr := NewPMOS(nil)
	ids := tr.DrainCurrent(-1.5, -3.0)
	if ids <= 0 {
		t.Errorf("DrainCurrent(-1.5, -3.0) = %v, want > 0", ids)
	}
}

func TestPMOS_CutoffNoCurrent(t *testing.T) {
	// PMOS in cutoff should have zero current.
	tr := NewPMOS(nil)
	if c := tr.DrainCurrent(0.0, -1.0); c != 0.0 {
		t.Errorf("DrainCurrent(0.0, -1.0) = %v, want 0.0", c)
	}
}

func TestPMOS_OutputVoltageOn(t *testing.T) {
	// When ON, PMOS pulls output to Vdd.
	tr := NewPMOS(nil)
	if v := tr.OutputVoltage(-3.3, 3.3); v != 3.3 {
		t.Errorf("OutputVoltage(-3.3, 3.3) = %v, want 3.3", v)
	}
}

func TestPMOS_OutputVoltageOff(t *testing.T) {
	// When OFF, PMOS output is at GND.
	tr := NewPMOS(nil)
	if v := tr.OutputVoltage(0.0, 3.3); v != 0.0 {
		t.Errorf("OutputVoltage(0.0, 3.3) = %v, want 0.0", v)
	}
}

func TestPMOS_ComplementaryToNMOS(t *testing.T) {
	// PMOS should be ON when NMOS is OFF and vice versa.
	nmos := NewNMOS(nil)
	pmos := NewPMOS(nil)
	vdd := 3.3

	// Input HIGH: NMOS ON, PMOS OFF
	if !nmos.IsConducting(vdd) {
		t.Error("NMOS should conduct at Vgs=Vdd")
	}
	if pmos.IsConducting(0.0) {
		t.Error("PMOS should be OFF at Vgs=0")
	}

	// Input LOW: NMOS OFF, PMOS ON
	if nmos.IsConducting(0.0) {
		t.Error("NMOS should be OFF at Vgs=0")
	}
	if !pmos.IsConducting(-vdd) {
		t.Error("PMOS should conduct at Vgs=-Vdd")
	}
}

func TestPMOS_TransconductanceCutoff(t *testing.T) {
	tr := NewPMOS(nil)
	if gm := tr.Transconductance(0.0, 0.0); gm != 0.0 {
		t.Errorf("Transconductance(0.0, 0.0) = %v, want 0.0", gm)
	}
}

func TestPMOS_TransconductanceOn(t *testing.T) {
	tr := NewPMOS(nil)
	gm := tr.Transconductance(-1.5, -3.0)
	if gm <= 0 {
		t.Errorf("Transconductance(-1.5, -3.0) = %v, want > 0", gm)
	}
}
