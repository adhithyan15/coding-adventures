// silicon_rust_go_test.go — integration tests for the silicon_rust_go CGo wrapper.
//
// These tests dlopen the silicon_rust_cgo shared library (built by
// `cargo build -p silicon-rust-cgo --release`) and verify that each Go
// wrapper function crosses the FFI boundary correctly and returns a
// physically reasonable value.
//
// They test "did the call cross the FFI and come back intact?", not
// "did the underlying Rust physics compute correctly?" — the Rust crate
// unit tests and the silicon-rust-python / silicon-rust-ruby sibling tests
// already cover the physics.
package silicon_rust_go_test

import (
	"math"
	"strings"
	"testing"

	srg "github.com/adhithyan15/coding-adventures/code/packages/go/silicon-rust-go"
)

// ─────────────────────────────────────────────────────────────────────────────
// Physical constants
// ─────────────────────────────────────────────────────────────────────────────

func TestKBoltzmann(t *testing.T) {
	v := srg.KBoltzmann()
	if math.Abs(v-1.380649e-23) > 1e-30 {
		t.Errorf("KBoltzmann = %e, want 1.380649e-23", v)
	}
}

func TestQElectron(t *testing.T) {
	v := srg.QElectron()
	if v <= 0 {
		t.Errorf("QElectron = %e, want positive", v)
	}
}

func TestEps0(t *testing.T) {
	v := srg.Eps0()
	if v <= 0 {
		t.Errorf("Eps0 = %e, want positive", v)
	}
}

func TestEpsSi(t *testing.T) {
	v := srg.EpsSi()
	if v <= srg.Eps0() {
		t.Errorf("EpsSi = %e, want > Eps0 (%e)", v, srg.Eps0())
	}
}

func TestEpsOx(t *testing.T) {
	v := srg.EpsOx()
	if v <= 0 || v >= srg.EpsSi() {
		t.Errorf("EpsOx = %e, want 0 < EpsOx < EpsSi", v)
	}
}

func TestNiAt300K(t *testing.T) {
	v := srg.NiAt300K()
	if math.Abs(v-1e16) > 1e9 {
		t.Errorf("NiAt300K = %e, want ~1e16 /m³", v)
	}
}

func TestEgSiAt300K(t *testing.T) {
	v := srg.EgSiAt300K()
	if math.Abs(v-1.12) > 0.01 {
		t.Errorf("EgSiAt300K = %f, want 1.12 eV", v)
	}
}

func TestMuN300K(t *testing.T) {
	v := srg.MuN300K()
	if v <= 0 {
		t.Errorf("MuN300K = %e, want positive", v)
	}
}

func TestMuP300K(t *testing.T) {
	v := srg.MuP300K()
	if v <= 0 || v >= srg.MuN300K() {
		t.Errorf("MuP300K = %e, want 0 < μp < μn", v)
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// device-physics
// ─────────────────────────────────────────────────────────────────────────────

func TestThermalVoltageAt300K(t *testing.T) {
	vt := srg.ThermalVoltage(300.0)
	// kT/q at 300 K ≈ 0.025852 V
	if math.Abs(vt-0.025852) > 1e-5 {
		t.Errorf("ThermalVoltage(300) = %f, want ~0.025852", vt)
	}
}

func TestIntrinsicConcentrationAt300K(t *testing.T) {
	ni, err := srg.IntrinsicConcentration(300.0)
	if err != nil {
		t.Fatalf("IntrinsicConcentration(300): unexpected error: %v", err)
	}
	if math.Abs(ni-1e16) > 1e9 {
		t.Errorf("IntrinsicConcentration(300) = %e, want ~1e16", ni)
	}
}

func TestIntrinsicConcentrationLowTempError(t *testing.T) {
	_, err := srg.IntrinsicConcentration(50.0)
	if err == nil {
		t.Error("IntrinsicConcentration(50): expected error, got nil")
	}
}

func TestFermiPotentialPType(t *testing.T) {
	phi, err := srg.FermiPotential(1e23, "p", 300.0)
	if err != nil {
		t.Fatalf("FermiPotential p-type: unexpected error: %v", err)
	}
	if phi <= 0 {
		t.Errorf("FermiPotential p-type = %f, want positive", phi)
	}
}

func TestFermiPotentialNType(t *testing.T) {
	phi, err := srg.FermiPotential(1e23, "n", 300.0)
	if err != nil {
		t.Fatalf("FermiPotential n-type: unexpected error: %v", err)
	}
	if phi >= 0 {
		t.Errorf("FermiPotential n-type = %f, want negative", phi)
	}
}

func TestPNJunctionBuiltInVoltage(t *testing.T) {
	vbi, err := srg.PNJunctionBuiltInVoltage(1e23, 1e22, 300.0)
	if err != nil {
		t.Fatalf("PNJunctionBuiltInVoltage: unexpected error: %v", err)
	}
	if vbi < 0.5 || vbi > 1.5 {
		t.Errorf("PNJunctionBuiltInVoltage = %f, want 0.5–1.5 V", vbi)
	}
}

func TestPNJunctionDepletionWidthPositive(t *testing.T) {
	w, err := srg.PNJunctionDepletionWidth(1e23, 1e22, 300.0, 0.0)
	if err != nil {
		t.Fatalf("PNJunctionDepletionWidth: unexpected error: %v", err)
	}
	if w <= 0 {
		t.Errorf("PNJunctionDepletionWidth = %e, want positive", w)
	}
}

func TestPNJunctionSaturationCurrentPositive(t *testing.T) {
	is_, err := srg.PNJunctionSaturationCurrent(1e23, 1e22, 1e-8, 300.0, 1e-6, 1e-6)
	if err != nil {
		t.Fatalf("PNJunctionSaturationCurrent: unexpected error: %v", err)
	}
	if is_ <= 0 {
		t.Errorf("PNJunctionSaturationCurrent = %e, want positive", is_)
	}
}

func TestPNJunctionCurrentForwardBias(t *testing.T) {
	i, err := srg.PNJunctionCurrent(1e23, 1e22, 1e-8, 300.0, 1e-6, 1e-6, 0.6)
	if err != nil {
		t.Fatalf("PNJunctionCurrent: unexpected error: %v", err)
	}
	if i <= 0 {
		t.Errorf("PNJunctionCurrent(V=0.6) = %e, want positive", i)
	}
}

func TestMosfetThresholdVoltageNMOS(t *testing.T) {
	vt, err := srg.MosfetThresholdVoltage("NMOS", 130e-9, 1e-6, 2e-9, 1e24, -0.05, 0.0, 300.0, 0.0)
	if err != nil {
		t.Fatalf("MosfetThresholdVoltage: unexpected error: %v", err)
	}
	// High body doping → V_T > 0.5 V
	if vt <= 0.5 {
		t.Errorf("MosfetThresholdVoltage NMOS = %f, want > 0.5 V", vt)
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// mosfet-models
// ─────────────────────────────────────────────────────────────────────────────

func TestEvaluateLevel1DefaultsSaturation(t *testing.T) {
	r, err := srg.EvaluateLevel1Defaults(1.8, 1.8, 0.0, 300.15)
	if err != nil {
		t.Fatalf("EvaluateLevel1Defaults: unexpected error: %v", err)
	}
	if r.Region != "saturation" {
		t.Errorf("EvaluateLevel1Defaults region = %q, want saturation", r.Region)
	}
	if r.Id <= 0 {
		t.Errorf("EvaluateLevel1Defaults Id = %e, want positive in saturation", r.Id)
	}
}

func TestEvaluateLevel1DefaultsAllKeys(t *testing.T) {
	r, err := srg.EvaluateLevel1Defaults(1.8, 1.8, 0.0, 300.15)
	if err != nil {
		t.Fatalf("EvaluateLevel1Defaults: unexpected error: %v", err)
	}
	// All capacitances should be finite non-negative values
	caps := []float64{r.Cgs, r.Cgd, r.Cgb, r.Cbs, r.Cbd}
	for _, c := range caps {
		if math.IsNaN(c) || math.IsInf(c, 0) || c < 0 {
			t.Errorf("capacitance field %e is invalid", c)
		}
	}
}

func TestEvaluateLevel1DefaultsCutoff(t *testing.T) {
	// V_GS = 0 V < V_T ≈ 0.42 V → cutoff or subthreshold
	r, err := srg.EvaluateLevel1Defaults(0.0, 1.8, 0.0, 300.15)
	if err != nil {
		t.Fatalf("EvaluateLevel1Defaults cutoff: unexpected error: %v", err)
	}
	if r.Region != "cutoff" && r.Region != "subthreshold" {
		t.Errorf("EvaluateLevel1Defaults(vGs=0) region = %q, want cutoff or subthreshold", r.Region)
	}
}

func TestEvaluateLevel1ExplicitSaturation(t *testing.T) {
	r, err := srg.EvaluateLevel1(
		0.42,   // vt0
		220e-6, // kp
		0.05,   // lambda
		0.27,   // gamma
		0.84,   // phi
		1e-6,   // w
		130e-9, // l
		1.4,    // nSub
		1.8,    // vGs
		1.8,    // vDs
		0.0,    // vBs
		300.15, // t
	)
	if err != nil {
		t.Fatalf("EvaluateLevel1: unexpected error: %v", err)
	}
	if r.Region != "saturation" {
		t.Errorf("EvaluateLevel1 explicit region = %q, want saturation", r.Region)
	}
	if r.Id <= 0 {
		t.Errorf("EvaluateLevel1 explicit Id = %e, want positive", r.Id)
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// fab-process-simulation — CrossSection wire format
// ─────────────────────────────────────────────────────────────────────────────

func TestDepositOnEmpty(t *testing.T) {
	cs, err := srg.Deposit("", "Si", 500.0)
	if err != nil {
		t.Fatalf("Deposit on empty: unexpected error: %v", err)
	}
	if !strings.HasPrefix(cs, "Si:") {
		t.Errorf("Deposit on empty = %q, want prefix Si:", cs)
	}
}

func TestDepositPrependsLayer(t *testing.T) {
	cs, err := srg.Deposit("Si:500.0", "Poly", 50.0)
	if err != nil {
		t.Fatalf("Deposit prepend: unexpected error: %v", err)
	}
	if !strings.HasPrefix(cs, "Poly:") {
		t.Errorf("Deposit prepend = %q, want Poly: on top", cs)
	}
	if !strings.Contains(cs, "|Si:500.0") {
		t.Errorf("Deposit prepend = %q, Si layer should remain", cs)
	}
}

func TestDealGroveOxidationProducesSiO2(t *testing.T) {
	cs, err := srg.DealGroveOxidation("Si:500.0", 5.0)
	if err != nil {
		t.Fatalf("DealGroveOxidation: unexpected error: %v", err)
	}
	if !strings.HasPrefix(cs, "SiO2:") {
		t.Errorf("DealGroveOxidation = %q, want SiO2: on top", cs)
	}
}

func TestDealGroveOxidationCustomCoeffs(t *testing.T) {
	cs, err := srg.DealGroveOxidationCustom("Si:500.0", 5.0, 0.165, 0.0117)
	if err != nil {
		t.Fatalf("DealGroveOxidationCustom: unexpected error: %v", err)
	}
	if !strings.HasPrefix(cs, "SiO2:") {
		t.Errorf("DealGroveOxidationCustom = %q, want SiO2: on top", cs)
	}
}

func TestEtchRemovesLayer(t *testing.T) {
	// Build "Poly:50.0|Si:500.0"
	cs, err := srg.Deposit("Si:500.0", "Poly", 50.0)
	if err != nil {
		t.Fatalf("Deposit before etch: %v", err)
	}
	// Etch all of Poly
	cs, err = srg.Etch(cs, "Poly", 50.0)
	if err != nil {
		t.Fatalf("Etch: unexpected error: %v", err)
	}
	if strings.Contains(cs, "Poly:") {
		t.Errorf("Etch = %q, Poly layer should be gone", cs)
	}
	if !strings.HasPrefix(cs, "Si:") {
		t.Errorf("Etch = %q, Si substrate should be exposed", cs)
	}
}

func TestImplantReturnsString(t *testing.T) {
	cs, err := srg.Deposit("", "Si", 500.0)
	if err != nil {
		t.Fatalf("Deposit before implant: %v", err)
	}
	cs2, err := srg.Implant(cs, "B", 30.0, 1e13)
	if err != nil {
		t.Fatalf("Implant: unexpected error: %v", err)
	}
	if cs2 == "" {
		t.Error("Implant returned empty string")
	}
}

func TestDiffuseReturnsString(t *testing.T) {
	cs, _ := srg.Deposit("", "Si", 500.0)
	cs, _ = srg.Implant(cs, "B", 30.0, 1e13)
	cs2, err := srg.Diffuse(cs, 30.0)
	if err != nil {
		t.Fatalf("Diffuse: unexpected error: %v", err)
	}
	if cs2 == "" {
		t.Error("Diffuse returned empty string")
	}
}

func TestDiffuseWithTempReturnsString(t *testing.T) {
	cs, _ := srg.Deposit("", "Si", 500.0)
	cs, _ = srg.Implant(cs, "B", 30.0, 1e13)
	cs2, err := srg.DiffuseWithTemp(cs, 30.0, 1000.0)
	if err != nil {
		t.Fatalf("DiffuseWithTemp: unexpected error: %v", err)
	}
	if cs2 == "" {
		t.Error("DiffuseWithTemp returned empty string")
	}
}

func TestImplantRangeBoron30KeV(t *testing.T) {
	rp, straggle, err := srg.ImplantRange("B", 30.0)
	if err != nil {
		t.Fatalf("ImplantRange: unexpected error: %v", err)
	}
	// SRIM reference: B at 30 keV → Rp ≈ 92 nm, straggle ≈ 38 nm
	if math.Abs(rp-92.0) > 5.0 {
		t.Errorf("ImplantRange B 30 keV Rp = %f nm, want ~92", rp)
	}
	if math.Abs(straggle-38.0) > 5.0 {
		t.Errorf("ImplantRange B 30 keV straggle = %f nm, want ~38", straggle)
	}
}

func TestImplantRangePositiveValues(t *testing.T) {
	rp, straggle, err := srg.ImplantRange("B", 30.0)
	if err != nil {
		t.Fatalf("ImplantRange: unexpected error: %v", err)
	}
	if rp <= 0 {
		t.Errorf("ImplantRange Rp = %f, want positive", rp)
	}
	if straggle <= 0 {
		t.Errorf("ImplantRange straggle = %f, want positive", straggle)
	}
}

func TestDiffusivityBoron1000C(t *testing.T) {
	d := srg.DiffusivityCm2PerS("B", 1000.0)
	// Reference: B in Si at 1000 °C ≈ 1e-14 cm²/s
	if math.Abs(d-1e-14) > 1e-16 {
		t.Errorf("DiffusivityCm2PerS(B, 1000) = %e, want ~1e-14", d)
	}
}

func TestDiffusivityPositive(t *testing.T) {
	d := srg.DiffusivityCm2PerS("B", 1000.0)
	if d <= 0 {
		t.Errorf("DiffusivityCm2PerS = %e, want positive", d)
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// Error cases / injection guard
// ─────────────────────────────────────────────────────────────────────────────

func TestDepositRejectsPipeInMaterialName(t *testing.T) {
	_, err := srg.Deposit("Si:500.0", "Bad|Material", 10.0)
	if err == nil {
		t.Error("Deposit with | in material name: expected error, got nil")
	}
	if !strings.Contains(err.Error(), "|") {
		t.Errorf("Deposit error message %q should mention the | character", err.Error())
	}
}

func TestDepositRejectsColonInMaterialName(t *testing.T) {
	_, err := srg.Deposit("Si:500.0", "Bad:Name", 10.0)
	if err == nil {
		t.Error("Deposit with : in material name: expected error, got nil")
	}
	if !strings.Contains(err.Error(), ":") {
		t.Errorf("Deposit error message %q should mention the : character", err.Error())
	}
}

func TestDealGroveOxidationNegativeTimeError(t *testing.T) {
	_, err := srg.DealGroveOxidation("Si:500.0", -1.0)
	if err == nil {
		t.Error("DealGroveOxidation(-1): expected error, got nil")
	}
}

func TestImplantRangeUnknownSpeciesError(t *testing.T) {
	_, _, err := srg.ImplantRange("UnknownElement", 30.0)
	if err == nil {
		t.Error("ImplantRange(UnknownElement): expected error, got nil")
	}
}

func TestIntrinsicConcentrationNegativeTempError(t *testing.T) {
	_, err := srg.IntrinsicConcentration(-10.0)
	if err == nil {
		t.Error("IntrinsicConcentration(-10): expected error, got nil")
	}
}
