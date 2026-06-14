// Package silicon_rust_go exposes the Rust silicon simulation stack to Go
// via CGo.
//
// The package wraps silicon-rust-cgo, a Rust cdylib that exports a plain C
// ABI.  CGo translates each C function call into a direct Rust invocation
// with no intermediate interpreter overhead.
//
// # Architecture
//
//	Go caller
//	  ↓ import "silicon_rust_go"
//	silicon_rust_go  (this package, CGo)
//	  ↓ import "C" → silicon_cgo.h
//	silicon-rust-cgo  (Rust cdylib, plain C ABI)
//	  ↓ Rust function calls
//	device-physics   mosfet-models   fab-process-simulation
//
// # Quick start
//
//	cs, err := silicon_rust_go.Deposit("", "Si", 500.0)
//	cs, err  = silicon_rust_go.DealGroveOxidation(cs, 5.0)
//	r, err  := silicon_rust_go.EvaluateLevel1Defaults(1.8, 1.8, 0.0, 300.15)
//	fmt.Println(r.Region)  // "saturation"
//
// # Wire format
//
// Cross-sections travel across the FFI boundary as pipe-separated strings:
//
//	""                              empty cross-section
//	"Si:500.0"                      bare silicon substrate
//	"SiO2:4.8|Si:500.0"            gate oxide on silicon
//	"Poly:50.0|SiO2:4.8|Si:500.0" poly gate on gate oxide on silicon
//
// Material names containing '|' or ':' are rejected to prevent wire-format
// injection.
//
// # Build
//
// Before running go test, build the Rust cdylib:
//
//	cargo build -p silicon-rust-cgo --release
//
// Then from this directory:
//
//	go test ./...
package silicon_rust_go

// CGo directives — must appear immediately before `import "C"` with no blank
// lines between them.
//
// CFLAGS: add the include/ directory so the C compiler finds silicon_cgo.h.
// LDFLAGS: link the pre-built Rust cdylib from the workspace target directory.
//
// ${SRCDIR} expands to the absolute path of this source file's directory at
// go build time, making the path relocatable.
//
// Platform-specific LDFLAGS add the runtime library search path so the OS
// dynamic linker can find libsilicon_rust_cgo.so at test execution time
// (Linux rpath / macOS @rpath).

// #cgo CFLAGS: -I${SRCDIR}/../../rust/silicon-rust-cgo/include
// #cgo linux   LDFLAGS: -L${SRCDIR}/../../rust/target/release -lsilicon_rust_cgo -Wl,-rpath,${SRCDIR}/../../rust/target/release
// #cgo darwin  LDFLAGS: -L${SRCDIR}/../../rust/target/release -lsilicon_rust_cgo -Wl,-rpath,${SRCDIR}/../../rust/target/release
// #cgo windows LDFLAGS: -L${SRCDIR}/../../rust/target/release -lsilicon_rust_cgo
// #include "silicon_cgo.h"
// #include <stdlib.h>
import "C"
import (
	"fmt"
	"unsafe"
)

// Buffer sizes for C ↔ Go string passing.
// 4096 bytes is sufficient for any realistic CMOS process flow (< 20 layers).
const (
	outBufSize = 4096
	errBufSize = 1024
)

// MosResult holds the Level-1 MOSFET DC operating point returned by
// EvaluateLevel1 and EvaluateLevel1Defaults.
//
// Fields:
//   - Id  — drain current [A]
//   - Gm  — transconductance [S]
//   - Gds — drain-source conductance [S]
//   - Gmb — body transconductance [S]
//   - Cgs, Cgd, Cgb, Cbs, Cbd — small-signal capacitances [F]
//   - Region — operating region: "cutoff", "subthreshold", "triode", or "saturation"
type MosResult struct {
	Id, Gm, Gds, Gmb         float64
	Cgs, Cgd, Cgb, Cbs, Cbd float64
	Region                   string
}

// ---------------------------------------------------------------------------
// Physical constants — all infallible, return float64 directly
// ---------------------------------------------------------------------------

// KBoltzmann returns Boltzmann's constant: 1.380649×10⁻²³ J/K.
func KBoltzmann() float64 { return float64(C.silicon_k_boltzmann()) }

// QElectron returns the elementary charge: 1.602176634×10⁻¹⁹ C.
func QElectron() float64 { return float64(C.silicon_q_electron()) }

// Eps0 returns the permittivity of free space: 8.8541878×10⁻¹² F/m.
func Eps0() float64 { return float64(C.silicon_eps0()) }

// EpsSi returns the permittivity of silicon: 11.7 × ε₀ ≈ 1.0359×10⁻¹⁰ F/m.
func EpsSi() float64 { return float64(C.silicon_eps_si()) }

// EpsOx returns the permittivity of SiO₂: 3.9 × ε₀ ≈ 3.4531×10⁻¹¹ F/m.
func EpsOx() float64 { return float64(C.silicon_eps_ox()) }

// NiAt300K returns the intrinsic carrier concentration of Si at 300 K: 1×10¹⁶ /m³.
func NiAt300K() float64 { return float64(C.silicon_ni_at_300k()) }

// EgSiAt300K returns the silicon bandgap at 300 K: 1.12 eV.
func EgSiAt300K() float64 { return float64(C.silicon_eg_si_at_300k()) }

// MuN300K returns the electron mobility in Si at 300 K: 0.1350 m²/V·s.
func MuN300K() float64 { return float64(C.silicon_mu_n_300k()) }

// MuP300K returns the hole mobility in Si at 300 K: 0.0480 m²/V·s.
func MuP300K() float64 { return float64(C.silicon_mu_p_300k()) }

// ---------------------------------------------------------------------------
// device-physics
// ---------------------------------------------------------------------------

// ThermalVoltage returns kT/q [V] at the given temperature.
//
//	ThermalVoltage(300) ≈ 0.025852 V
func ThermalVoltage(tKelvin float64) float64 {
	return float64(C.silicon_thermal_voltage(C.double(tKelvin)))
}

// IntrinsicConcentration returns nᵢ(T) [/m³] using the Sze–Ng model.
// Returns an error below ~100 K where the model degrades.
func IntrinsicConcentration(tKelvin float64) (float64, error) {
	var out C.double
	var errBuf [errBufSize]C.char
	rc := C.silicon_intrinsic_concentration(C.double(tKelvin), &out, &errBuf[0], errBufSize)
	if rc != 0 {
		return 0, fmt.Errorf("IntrinsicConcentration: %s", C.GoString(&errBuf[0]))
	}
	return float64(out), nil
}

// FermiPotential returns φ_F [V] for a doped semiconductor.
// kind must be "p" (p-type, φ_F > 0) or "n" (n-type, φ_F < 0).
func FermiPotential(nDoping float64, kind string, tKelvin float64) (float64, error) {
	ckind := C.CString(kind)
	defer C.free(unsafe.Pointer(ckind))
	var out C.double
	var errBuf [errBufSize]C.char
	rc := C.silicon_fermi_potential(C.double(nDoping), ckind, C.double(tKelvin), &out, &errBuf[0], errBufSize)
	if rc != 0 {
		return 0, fmt.Errorf("FermiPotential: %s", C.GoString(&errBuf[0]))
	}
	return float64(out), nil
}

// PNJunctionBuiltInVoltage returns the built-in potential V_bi [V].
// na, nd [/m³] are the acceptor and donor concentrations; t [K] is temperature.
func PNJunctionBuiltInVoltage(na, nd, t float64) (float64, error) {
	var out C.double
	var errBuf [errBufSize]C.char
	rc := C.silicon_pn_junction_built_in_voltage(
		C.double(na), C.double(nd), C.double(t), &out, &errBuf[0], errBufSize)
	if rc != 0 {
		return 0, fmt.Errorf("PNJunctionBuiltInVoltage: %s", C.GoString(&errBuf[0]))
	}
	return float64(out), nil
}

// PNJunctionDepletionWidth returns the depletion width W [m] at applied bias vApplied [V].
func PNJunctionDepletionWidth(na, nd, t, vApplied float64) (float64, error) {
	var out C.double
	var errBuf [errBufSize]C.char
	rc := C.silicon_pn_junction_depletion_width(
		C.double(na), C.double(nd), C.double(t), C.double(vApplied), &out, &errBuf[0], errBufSize)
	if rc != 0 {
		return 0, fmt.Errorf("PNJunctionDepletionWidth: %s", C.GoString(&errBuf[0]))
	}
	return float64(out), nil
}

// PNJunctionSaturationCurrent returns I₀ [A], the reverse saturation current.
// a [m²] is junction area; tauN, tauP [s] are minority-carrier lifetimes.
func PNJunctionSaturationCurrent(na, nd, a, t, tauN, tauP float64) (float64, error) {
	var out C.double
	var errBuf [errBufSize]C.char
	rc := C.silicon_pn_junction_saturation_current(
		C.double(na), C.double(nd), C.double(a), C.double(t),
		C.double(tauN), C.double(tauP), &out, &errBuf[0], errBufSize)
	if rc != 0 {
		return 0, fmt.Errorf("PNJunctionSaturationCurrent: %s", C.GoString(&errBuf[0]))
	}
	return float64(out), nil
}

// PNJunctionCurrent returns I(V) [A] using the ideal diode equation.
// v [V] is the applied forward voltage.
func PNJunctionCurrent(na, nd, a, t, tauN, tauP, v float64) (float64, error) {
	var out C.double
	var errBuf [errBufSize]C.char
	rc := C.silicon_pn_junction_current(
		C.double(na), C.double(nd), C.double(a), C.double(t),
		C.double(tauN), C.double(tauP), C.double(v), &out, &errBuf[0], errBufSize)
	if rc != 0 {
		return 0, fmt.Errorf("PNJunctionCurrent: %s", C.GoString(&errBuf[0]))
	}
	return float64(out), nil
}

// MosfetThresholdVoltage returns V_T [V] for a long-channel MOSFET.
//
//   - deviceType: "NMOS" or "PMOS"
//   - l, w [m]: gate length and width
//   - tOx [m]: gate oxide thickness
//   - nBody [/m³]: body doping
//   - phiMs [V]: metal–semiconductor work-function difference
//   - qOx [C/m²]: oxide fixed charge density
//   - t [K]: temperature
//   - vSb [V]: source-body reverse bias
func MosfetThresholdVoltage(deviceType string, l, w, tOx, nBody, phiMs, qOx, t, vSb float64) (float64, error) {
	cdev := C.CString(deviceType)
	defer C.free(unsafe.Pointer(cdev))
	var out C.double
	var errBuf [errBufSize]C.char
	rc := C.silicon_mosfet_threshold_voltage(
		cdev, C.double(l), C.double(w), C.double(tOx), C.double(nBody),
		C.double(phiMs), C.double(qOx), C.double(t), C.double(vSb),
		&out, &errBuf[0], errBufSize)
	if rc != 0 {
		return 0, fmt.Errorf("MosfetThresholdVoltage: %s", C.GoString(&errBuf[0]))
	}
	return float64(out), nil
}

// ---------------------------------------------------------------------------
// mosfet-models
// ---------------------------------------------------------------------------

// EvaluateLevel1 computes the Level-1 MOSFET DC operating point for explicit
// device parameters.
//
// Parameters:
//   - vt0 [V]: threshold voltage at zero bias
//   - kp [A/V²]: process transconductance parameter
//   - lambda [1/V]: channel-length modulation
//   - gamma [√V]: body-effect coefficient
//   - phi [V]: surface potential at strong inversion
//   - w, l [m]: gate width and length
//   - nSub: subthreshold slope factor
//   - vGs, vDs, vBs [V]: terminal voltages
//   - t [K]: temperature
func EvaluateLevel1(vt0, kp, lambda, gamma, phi, w, l, nSub, vGs, vDs, vBs, t float64) (MosResult, error) {
	var r C.SiliconMosResult
	var errBuf [errBufSize]C.char
	rc := C.silicon_evaluate_level1(
		C.double(vt0), C.double(kp), C.double(lambda), C.double(gamma),
		C.double(phi), C.double(w), C.double(l), C.double(nSub),
		C.double(vGs), C.double(vDs), C.double(vBs), C.double(t),
		&r, &errBuf[0], errBufSize)
	if rc != 0 {
		return MosResult{}, fmt.Errorf("EvaluateLevel1: %s", C.GoString(&errBuf[0]))
	}
	return mosResultFromC(&r), nil
}

// EvaluateLevel1Defaults computes the Level-1 MOSFET DC operating point using
// default 130 nm NMOS parameters (Level1Params::default() in Rust).
func EvaluateLevel1Defaults(vGs, vDs, vBs, t float64) (MosResult, error) {
	var r C.SiliconMosResult
	var errBuf [errBufSize]C.char
	rc := C.silicon_evaluate_level1_defaults(
		C.double(vGs), C.double(vDs), C.double(vBs), C.double(t),
		&r, &errBuf[0], errBufSize)
	if rc != 0 {
		return MosResult{}, fmt.Errorf("EvaluateLevel1Defaults: %s", C.GoString(&errBuf[0]))
	}
	return mosResultFromC(&r), nil
}

// mosResultFromC converts a C SiliconMosResult into a Go MosResult.
func mosResultFromC(r *C.SiliconMosResult) MosResult {
	return MosResult{
		Id:     float64(r.id),
		Gm:     float64(r.gm),
		Gds:    float64(r.gds),
		Gmb:    float64(r.gmb),
		Cgs:    float64(r.cgs),
		Cgd:    float64(r.cgd),
		Cgb:    float64(r.cgb),
		Cbs:    float64(r.cbs),
		Cbd:    float64(r.cbd),
		Region: C.GoString(&r.region[0]),
	}
}

// ---------------------------------------------------------------------------
// fab-process-simulation
// ---------------------------------------------------------------------------

// Deposit adds a layer of material with the given thickness (nm) on top of
// the cross-section cs.  Returns the new wire-format cross-section string.
//
// Returns an error if material contains '|' or ':' (injection guard).
func Deposit(cs, material string, thicknessNm float64) (string, error) {
	ccs := C.CString(cs)
	cmat := C.CString(material)
	defer C.free(unsafe.Pointer(ccs))
	defer C.free(unsafe.Pointer(cmat))
	var outBuf [outBufSize]C.char
	var errBuf [errBufSize]C.char
	rc := C.silicon_deposit(ccs, cmat, C.double(thicknessNm),
		&outBuf[0], outBufSize, &errBuf[0], errBufSize)
	if rc != 0 {
		return "", fmt.Errorf("Deposit: %s", C.GoString(&errBuf[0]))
	}
	return C.GoString(&outBuf[0]), nil
}

// Etch removes depth_nm of target_material from the top of the cross-section.
// Returns the new wire-format cross-section string.
func Etch(cs, target string, depthNm float64) (string, error) {
	ccs := C.CString(cs)
	ctarget := C.CString(target)
	defer C.free(unsafe.Pointer(ccs))
	defer C.free(unsafe.Pointer(ctarget))
	var outBuf [outBufSize]C.char
	var errBuf [errBufSize]C.char
	rc := C.silicon_etch(ccs, ctarget, C.double(depthNm),
		&outBuf[0], outBufSize, &errBuf[0], errBufSize)
	if rc != 0 {
		return "", fmt.Errorf("Etch: %s", C.GoString(&errBuf[0]))
	}
	return C.GoString(&outBuf[0]), nil
}

// Implant performs an ion implant of species at energy_kev [keV] and
// dose_cm2 [cm⁻²].  Returns the new wire-format cross-section string.
func Implant(cs, species string, energyKev, doseCm2 float64) (string, error) {
	ccs := C.CString(cs)
	cspe := C.CString(species)
	defer C.free(unsafe.Pointer(ccs))
	defer C.free(unsafe.Pointer(cspe))
	var outBuf [outBufSize]C.char
	var errBuf [errBufSize]C.char
	rc := C.silicon_implant(ccs, cspe, C.double(energyKev), C.double(doseCm2),
		&outBuf[0], outBufSize, &errBuf[0], errBufSize)
	if rc != 0 {
		return "", fmt.Errorf("Implant: %s", C.GoString(&errBuf[0]))
	}
	return C.GoString(&outBuf[0]), nil
}

// Diffuse anneals the cross-section for time_min minutes at the default
// temperature (1000 °C).  Returns the new wire-format cross-section string.
func Diffuse(cs string, timeMin float64) (string, error) {
	ccs := C.CString(cs)
	defer C.free(unsafe.Pointer(ccs))
	var outBuf [outBufSize]C.char
	var errBuf [errBufSize]C.char
	rc := C.silicon_diffuse(ccs, C.double(timeMin),
		&outBuf[0], outBufSize, &errBuf[0], errBufSize)
	if rc != 0 {
		return "", fmt.Errorf("Diffuse: %s", C.GoString(&errBuf[0]))
	}
	return C.GoString(&outBuf[0]), nil
}

// DiffuseWithTemp anneals the cross-section for time_min minutes at the
// explicit temperature_c [°C].  Returns the new wire-format cross-section string.
func DiffuseWithTemp(cs string, timeMin, temperatureC float64) (string, error) {
	ccs := C.CString(cs)
	defer C.free(unsafe.Pointer(ccs))
	var outBuf [outBufSize]C.char
	var errBuf [errBufSize]C.char
	rc := C.silicon_diffuse_with_temp(ccs, C.double(timeMin), C.double(temperatureC),
		&outBuf[0], outBufSize, &errBuf[0], errBufSize)
	if rc != 0 {
		return "", fmt.Errorf("DiffuseWithTemp: %s", C.GoString(&errBuf[0]))
	}
	return C.GoString(&outBuf[0]), nil
}

// DealGroveOxidation grows a thermal oxide on the cross-section for time_min
// minutes using the default Deal-Grove coefficients for dry oxidation.
// Returns the new wire-format cross-section string.
func DealGroveOxidation(cs string, timeMin float64) (string, error) {
	ccs := C.CString(cs)
	defer C.free(unsafe.Pointer(ccs))
	var outBuf [outBufSize]C.char
	var errBuf [errBufSize]C.char
	rc := C.silicon_deal_grove_oxidation(ccs, C.double(timeMin),
		&outBuf[0], outBufSize, &errBuf[0], errBufSize)
	if rc != 0 {
		return "", fmt.Errorf("DealGroveOxidation: %s", C.GoString(&errBuf[0]))
	}
	return C.GoString(&outBuf[0]), nil
}

// DealGroveOxidationCustom grows a thermal oxide using explicit Deal-Grove
// coefficients: aUm [µm] and bUm2PerHr [µm²/hr].
// Returns the new wire-format cross-section string.
func DealGroveOxidationCustom(cs string, timeMin, aUm, bUm2PerHr float64) (string, error) {
	ccs := C.CString(cs)
	defer C.free(unsafe.Pointer(ccs))
	var outBuf [outBufSize]C.char
	var errBuf [errBufSize]C.char
	rc := C.silicon_deal_grove_oxidation_custom(ccs, C.double(timeMin),
		C.double(aUm), C.double(bUm2PerHr),
		&outBuf[0], outBufSize, &errBuf[0], errBufSize)
	if rc != 0 {
		return "", fmt.Errorf("DealGroveOxidationCustom: %s", C.GoString(&errBuf[0]))
	}
	return C.GoString(&outBuf[0]), nil
}

// ImplantRange returns the projected range rp [nm] and straggle [nm] for
// an ion implant of species at energy_kev [keV], from SRIM look-up tables.
// Returns an error for unknown species.
func ImplantRange(species string, energyKev float64) (rp, straggle float64, err error) {
	cspe := C.CString(species)
	defer C.free(unsafe.Pointer(cspe))
	var crp, cst C.double
	var errBuf [errBufSize]C.char
	rc := C.silicon_implant_range(cspe, C.double(energyKev), &crp, &cst, &errBuf[0], errBufSize)
	if rc != 0 {
		return 0, 0, fmt.Errorf("ImplantRange: %s", C.GoString(&errBuf[0]))
	}
	return float64(crp), float64(cst), nil
}

// DiffusivityCm2PerS returns the diffusivity D [cm²/s] of species in silicon
// at the given temperature in °C.  Returns 0 for unknown species.
func DiffusivityCm2PerS(species string, temperatureC float64) float64 {
	cspe := C.CString(species)
	defer C.free(unsafe.Pointer(cspe))
	return float64(C.silicon_diffusivity_cm2_per_s(cspe, C.double(temperatureC)))
}
