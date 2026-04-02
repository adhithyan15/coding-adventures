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
	result, _ := StartNew[*NPN]("transistors.NewNPN", nil,
		func(op *Operation[*NPN], rf *ResultFactory[*NPN]) *OperationResult[*NPN] {
			if params != nil {
				return rf.Generate(true, false, &NPN{Params: *params})
			}
			return rf.Generate(true, false, &NPN{Params: DefaultBJTParams()})
		}).GetResult()
	return result
}

// Region determines the operating region from terminal voltages.
//
//	Cutoff:     Vbe < VbeOn
//	Saturation: Vbe >= VbeOn AND Vce <= VceSat
//	Active:     Vbe >= VbeOn AND Vce > VceSat
func (n *NPN) Region(vbe, vce float64) string {
	result, _ := StartNew[string]("transistors.NPN.Region", BJTCutoff,
		func(op *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
			op.AddProperty("vbe", vbe)
			op.AddProperty("vce", vce)
			if vbe < n.Params.VbeOn {
				return rf.Generate(true, false, BJTCutoff)
			}
			if vce <= n.Params.VceSat {
				return rf.Generate(true, false, BJTSaturation)
			}
			return rf.Generate(true, false, BJTActive)
		}).GetResult()
	return result
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
	result, _ := StartNew[float64]("transistors.NPN.CollectorCurrent", 0.0,
		func(op *Operation[float64], rf *ResultFactory[float64]) *OperationResult[float64] {
			op.AddProperty("vbe", vbe)
			op.AddProperty("vce", vce)
			region := n.Region(vbe, vce)

			if region == BJTCutoff {
				return rf.Generate(true, false, 0.0)
			}

			vt := 0.026
			exponent := math.Min(vbe/vt, 40.0)
			return rf.Generate(true, false, n.Params.Is*(math.Exp(exponent)-1.0))
		}).GetResult()
	return result
}

// BaseCurrent calculates base current (Ib) in amperes.
//
// Ib = Ic / beta in the active region.
//
// This is the "wasted" current that makes BJTs less efficient than
// MOSFETs for digital logic. Every TTL gate has base current flowing
// continuously, consuming significant power.
func (n *NPN) BaseCurrent(vbe, vce float64) float64 {
	result, _ := StartNew[float64]("transistors.NPN.BaseCurrent", 0.0,
		func(op *Operation[float64], rf *ResultFactory[float64]) *OperationResult[float64] {
			op.AddProperty("vbe", vbe)
			op.AddProperty("vce", vce)
			ic := n.CollectorCurrent(vbe, vce)
			if ic == 0.0 {
				return rf.Generate(true, false, 0.0)
			}
			return rf.Generate(true, false, ic/n.Params.Beta)
		}).GetResult()
	return result
}

// IsConducting returns true when Vbe >= VbeOn (~0.7V for silicon).
func (n *NPN) IsConducting(vbe float64) bool {
	result, _ := StartNew[bool]("transistors.NPN.IsConducting", false,
		func(op *Operation[bool], rf *ResultFactory[bool]) *OperationResult[bool] {
			op.AddProperty("vbe", vbe)
			return rf.Generate(true, false, vbe >= n.Params.VbeOn)
		}).GetResult()
	return result
}

// Transconductance calculates small-signal transconductance gm = Ic / Vt.
//
// BJTs typically have higher gm than MOSFETs for the same current,
// which is why they're still preferred for some analog applications.
func (n *NPN) Transconductance(vbe, vce float64) float64 {
	result, _ := StartNew[float64]("transistors.NPN.Transconductance", 0.0,
		func(op *Operation[float64], rf *ResultFactory[float64]) *OperationResult[float64] {
			op.AddProperty("vbe", vbe)
			op.AddProperty("vce", vce)
			ic := n.CollectorCurrent(vbe, vce)
			if ic == 0.0 {
				return rf.Generate(true, false, 0.0)
			}
			vt := 0.026
			return rf.Generate(true, false, ic/vt)
		}).GetResult()
	return result
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
	result, _ := StartNew[*PNP]("transistors.NewPNP", nil,
		func(op *Operation[*PNP], rf *ResultFactory[*PNP]) *OperationResult[*PNP] {
			if params != nil {
				return rf.Generate(true, false, &PNP{Params: *params})
			}
			return rf.Generate(true, false, &PNP{Params: DefaultBJTParams()})
		}).GetResult()
	return result
}

// Region determines the operating region for PNP using absolute values.
func (p *PNP) Region(vbe, vce float64) string {
	result, _ := StartNew[string]("transistors.PNP.Region", BJTCutoff,
		func(op *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
			op.AddProperty("vbe", vbe)
			op.AddProperty("vce", vce)
			absVbe := math.Abs(vbe)
			absVce := math.Abs(vce)

			if absVbe < p.Params.VbeOn {
				return rf.Generate(true, false, BJTCutoff)
			}
			if absVce <= p.Params.VceSat {
				return rf.Generate(true, false, BJTSaturation)
			}
			return rf.Generate(true, false, BJTActive)
		}).GetResult()
	return result
}

// CollectorCurrent calculates collector current magnitude for PNP.
// Same equations as NPN but using absolute values. Returns >= 0.
func (p *PNP) CollectorCurrent(vbe, vce float64) float64 {
	result, _ := StartNew[float64]("transistors.PNP.CollectorCurrent", 0.0,
		func(op *Operation[float64], rf *ResultFactory[float64]) *OperationResult[float64] {
			op.AddProperty("vbe", vbe)
			op.AddProperty("vce", vce)
			region := p.Region(vbe, vce)

			if region == BJTCutoff {
				return rf.Generate(true, false, 0.0)
			}

			absVbe := math.Abs(vbe)
			vt := 0.026
			exponent := math.Min(absVbe/vt, 40.0)
			return rf.Generate(true, false, p.Params.Is*(math.Exp(exponent)-1.0))
		}).GetResult()
	return result
}

// BaseCurrent calculates base current magnitude for PNP.
func (p *PNP) BaseCurrent(vbe, vce float64) float64 {
	result, _ := StartNew[float64]("transistors.PNP.BaseCurrent", 0.0,
		func(op *Operation[float64], rf *ResultFactory[float64]) *OperationResult[float64] {
			op.AddProperty("vbe", vbe)
			op.AddProperty("vce", vce)
			ic := p.CollectorCurrent(vbe, vce)
			if ic == 0.0 {
				return rf.Generate(true, false, 0.0)
			}
			return rf.Generate(true, false, ic/p.Params.Beta)
		}).GetResult()
	return result
}

// IsConducting returns true when |Vbe| >= VbeOn (base pulled below emitter).
func (p *PNP) IsConducting(vbe float64) bool {
	result, _ := StartNew[bool]("transistors.PNP.IsConducting", false,
		func(op *Operation[bool], rf *ResultFactory[bool]) *OperationResult[bool] {
			op.AddProperty("vbe", vbe)
			return rf.Generate(true, false, math.Abs(vbe) >= p.Params.VbeOn)
		}).GetResult()
	return result
}

// Transconductance calculates small-signal transconductance gm for PNP.
func (p *PNP) Transconductance(vbe, vce float64) float64 {
	result, _ := StartNew[float64]("transistors.PNP.Transconductance", 0.0,
		func(op *Operation[float64], rf *ResultFactory[float64]) *OperationResult[float64] {
			op.AddProperty("vbe", vbe)
			op.AddProperty("vce", vce)
			ic := p.CollectorCurrent(vbe, vce)
			if ic == 0.0 {
				return rf.Generate(true, false, 0.0)
			}
			vt := 0.026
			return rf.Generate(true, false, ic/vt)
		}).GetResult()
	return result
}
