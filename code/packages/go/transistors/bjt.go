package transistors

// BJT Transistors — the original solid-state amplifier.
//
// === What is a BJT? ===
//
// BJT stands for Bipolar Junction Transistor. Invented in 1947 at Bell Labs
// by John Bardeen, Walter Brattain, and William Shockley, the BJT replaced
// vacuum tubes and launched the electronics revolution.
//
// A BJT has three terminals:
//
//	Base (B):      The control terminal. Current here controls the switch.
//	Collector (C): Current flows IN here (for NPN) or OUT here (for PNP).
//	Emitter (E):   Current flows OUT here (for NPN) or IN here (for PNP).
//
// The key difference from MOSFETs: a BJT is CURRENT-controlled. You must
// supply a continuous current to the base to keep it on. This means:
//   - Base current = wasted power (even in steady state)
//   - Lower input impedance than MOSFETs
//   - But historically faster switching (before CMOS caught up)
//
// === The Current Gain (beta) ===
//
//	Ic = beta * Ib
//
// A tiny base current (microamps) controls a much larger collector current
// (milliamps). This amplification property made radios, televisions, and
// early computers possible.

import "math"

// NPN represents an NPN bipolar junction transistor.
//
// An NPN transistor turns ON when current flows into the base terminal
// (Vbe > ~0.7V). A small base current controls a much larger collector
// current through the current gain: Ic = beta * Ib.
//
// Operating regions:
//
//	CUTOFF:      Vbe < 0.7V -> no current -> switch OFF.
//	ACTIVE:      Vbe >= 0.7V, Vce > 0.2V -> linear amplifier.
//	SATURATION:  Vbe >= 0.7V, Vce <= 0.2V -> fully ON (switch).
type NPN struct {
	Params BJTParams
}

// NewNPN creates an NPN transistor. If params is nil, default 2N2222-style
// parameters are used.
func NewNPN(params *BJTParams) *NPN {
	if params != nil {
		return &NPN{Params: *params}
	}
	return &NPN{Params: DefaultBJTParams()}
}

// Region determines the operating region from terminal voltages.
//
//	Cutoff:     Vbe < VbeOn
//	Saturation: Vbe >= VbeOn AND Vce <= VceSat
//	Active:     Vbe >= VbeOn AND Vce > VceSat
func (n *NPN) Region(vbe, vce float64) string {
	if vbe < n.Params.VbeOn {
		return BJTCutoff
	}
	if vce <= n.Params.VceSat {
		return BJTSaturation
	}
	return BJTActive
}

// CollectorCurrent calculates collector current (Ic) in amperes.
//
// Uses the simplified Ebers-Moll model:
//
//	Cutoff:          Ic = 0
//	Active/Saturation: Ic = Is * (exp(Vbe/Vt) - 1)
//
// where Vt = kT/q ~ 26mV at room temperature.
// The exponent is clamped to 40 to prevent floating-point overflow.
func (n *NPN) CollectorCurrent(vbe, vce float64) float64 {
	region := n.Region(vbe, vce)

	if region == BJTCutoff {
		return 0.0
	}

	// Thermal voltage: Vt = kT/q ~ 26mV at room temperature.
	vt := 0.026

	// Ebers-Moll: the exponential relationship is why BJTs are such
	// good amplifiers — a small change in Vbe causes a large change in Ic.
	exponent := math.Min(vbe/vt, 40.0) // Clamp to prevent overflow
	return n.Params.Is * (math.Exp(exponent) - 1.0)
}

// BaseCurrent calculates base current (Ib) in amperes.
//
// Ib = Ic / beta in the active region.
//
// This is the "wasted" current that makes BJTs less efficient than
// MOSFETs for digital logic. Every TTL gate has base current flowing
// continuously, consuming significant power.
func (n *NPN) BaseCurrent(vbe, vce float64) float64 {
	ic := n.CollectorCurrent(vbe, vce)
	if ic == 0.0 {
		return 0.0
	}
	return ic / n.Params.Beta
}

// IsConducting returns true when Vbe >= VbeOn (~0.7V for silicon).
func (n *NPN) IsConducting(vbe float64) bool {
	return vbe >= n.Params.VbeOn
}

// Transconductance calculates small-signal transconductance gm = Ic / Vt.
//
// BJTs typically have higher gm than MOSFETs for the same current,
// which is why they're still preferred for some analog applications.
func (n *NPN) Transconductance(vbe, vce float64) float64 {
	ic := n.CollectorCurrent(vbe, vce)
	if ic == 0.0 {
		return 0.0
	}
	vt := 0.026
	return ic / vt
}

// PNP represents a PNP bipolar junction transistor.
//
// The complement of NPN. A PNP transistor turns ON when the base is
// pulled LOW relative to the emitter (|Vbe| >= VbeOn). Current flows
// from emitter to collector.
//
// For PNP, the "natural" voltages are reversed from NPN:
//   - Vbe is typically NEGATIVE (base below emitter)
//   - Vce is typically NEGATIVE (collector below emitter)
//
// We use absolute values internally, same as PMOS.
type PNP struct {
	Params BJTParams
}

// NewPNP creates a PNP transistor. If params is nil, default parameters are used.
func NewPNP(params *BJTParams) *PNP {
	if params != nil {
		return &PNP{Params: *params}
	}
	return &PNP{Params: DefaultBJTParams()}
}

// Region determines the operating region for PNP using absolute values.
func (p *PNP) Region(vbe, vce float64) string {
	absVbe := math.Abs(vbe)
	absVce := math.Abs(vce)

	if absVbe < p.Params.VbeOn {
		return BJTCutoff
	}
	if absVce <= p.Params.VceSat {
		return BJTSaturation
	}
	return BJTActive
}

// CollectorCurrent calculates collector current magnitude for PNP.
// Same equations as NPN but using absolute values. Returns >= 0.
func (p *PNP) CollectorCurrent(vbe, vce float64) float64 {
	region := p.Region(vbe, vce)

	if region == BJTCutoff {
		return 0.0
	}

	absVbe := math.Abs(vbe)
	vt := 0.026

	exponent := math.Min(absVbe/vt, 40.0)
	return p.Params.Is * (math.Exp(exponent) - 1.0)
}

// BaseCurrent calculates base current magnitude for PNP.
func (p *PNP) BaseCurrent(vbe, vce float64) float64 {
	ic := p.CollectorCurrent(vbe, vce)
	if ic == 0.0 {
		return 0.0
	}
	return ic / p.Params.Beta
}

// IsConducting returns true when |Vbe| >= VbeOn (base pulled below emitter).
func (p *PNP) IsConducting(vbe float64) bool {
	return math.Abs(vbe) >= p.Params.VbeOn
}

// Transconductance calculates small-signal transconductance gm for PNP.
func (p *PNP) Transconductance(vbe, vce float64) float64 {
	ic := p.CollectorCurrent(vbe, vce)
	if ic == 0.0 {
		return 0.0
	}
	vt := 0.026
	return ic / vt
}
