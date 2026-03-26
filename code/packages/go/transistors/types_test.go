package transistors

import "testing"

// === Tests for default parameter functions ===

func TestDefaultMOSFETParams(t *testing.T) {
	p := DefaultMOSFETParams()
	if p.Vth != 0.4 {
		t.Errorf("Vth = %v, want 0.4", p.Vth)
	}
	if p.K != 0.001 {
		t.Errorf("K = %v, want 0.001", p.K)
	}
	if p.W != 1e-6 {
		t.Errorf("W = %v, want 1e-6", p.W)
	}
	if p.L != 180e-9 {
		t.Errorf("L = %v, want 180e-9", p.L)
	}
	if p.CGate != 1e-15 {
		t.Errorf("CGate = %v, want 1e-15", p.CGate)
	}
	if p.CDrain != 0.5e-15 {
		t.Errorf("CDrain = %v, want 0.5e-15", p.CDrain)
	}
}

func TestDefaultBJTParams(t *testing.T) {
	p := DefaultBJTParams()
	if p.Beta != 100.0 {
		t.Errorf("Beta = %v, want 100.0", p.Beta)
	}
	if p.VbeOn != 0.7 {
		t.Errorf("VbeOn = %v, want 0.7", p.VbeOn)
	}
	if p.VceSat != 0.2 {
		t.Errorf("VceSat = %v, want 0.2", p.VceSat)
	}
	if p.Is != 1e-14 {
		t.Errorf("Is = %v, want 1e-14", p.Is)
	}
	if p.CBase != 5e-12 {
		t.Errorf("CBase = %v, want 5e-12", p.CBase)
	}
}

func TestDefaultCircuitParams(t *testing.T) {
	p := DefaultCircuitParams()
	if p.Vdd != 3.3 {
		t.Errorf("Vdd = %v, want 3.3", p.Vdd)
	}
	if p.Temperature != 300.0 {
		t.Errorf("Temperature = %v, want 300.0", p.Temperature)
	}
}

func TestMOSFETRegionConstants(t *testing.T) {
	if MOSFETCutoff != "cutoff" {
		t.Errorf("MOSFETCutoff = %q, want %q", MOSFETCutoff, "cutoff")
	}
	if MOSFETLinear != "linear" {
		t.Errorf("MOSFETLinear = %q, want %q", MOSFETLinear, "linear")
	}
	if MOSFETSaturation != "saturation" {
		t.Errorf("MOSFETSaturation = %q, want %q", MOSFETSaturation, "saturation")
	}
}

func TestBJTRegionConstants(t *testing.T) {
	if BJTCutoff != "cutoff" {
		t.Errorf("BJTCutoff = %q, want %q", BJTCutoff, "cutoff")
	}
	if BJTActive != "active" {
		t.Errorf("BJTActive = %q, want %q", BJTActive, "active")
	}
	if BJTSaturation != "saturation" {
		t.Errorf("BJTSaturation = %q, want %q", BJTSaturation, "saturation")
	}
}
