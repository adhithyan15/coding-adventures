package transistors

// MOSFET Transistors — the building blocks of modern digital circuits.
//
// === What is a MOSFET? ===
//
// MOSFET stands for Metal-Oxide-Semiconductor Field-Effect Transistor. It is
// the most common type of transistor in the world — every CPU, GPU, and phone
// chip is built from billions of MOSFETs.
//
// A MOSFET has three terminals:
//
//	Gate (G):   The control terminal. Voltage here controls the switch.
//	Drain (D):  Current flows IN here (for NMOS) or OUT here (for PMOS).
//	Source (S): Current flows OUT here (for NMOS) or IN here (for PMOS).
//
// The key insight: a MOSFET is VOLTAGE-controlled. Applying a voltage to the
// gate creates an electric field that either allows or blocks current flow
// between drain and source. No current flows into the gate itself (it's
// insulated by a thin oxide layer), which means:
//   - Near-zero input power consumption
//   - Very high input impedance (good for amplifiers)
//   - Can be packed extremely densely on a chip
//
// === NMOS vs PMOS ===
//
//	NMOS: Gate HIGH -> ON  (conducts drain to source)
//	PMOS: Gate LOW  -> ON  (conducts source to drain)
//
// This complementary behavior is the foundation of CMOS (Complementary MOS)
// logic. By pairing NMOS and PMOS transistors, we can build gates that consume
// near-zero power in steady state.

import "math"

// NMOS represents an N-channel MOSFET transistor.
//
// An NMOS transistor conducts current from drain to source when the gate
// voltage exceeds the threshold voltage (Vgs > Vth). Think of it as a
// normally-OPEN switch that CLOSES when you apply voltage to the gate.
//
// In a digital circuit, NMOS connects the output to GROUND:
//
//	Output --|
//	         | NMOS (gate = input signal)
//	         |
//	        GND
//
//	Input HIGH -> NMOS ON  -> output pulled to GND (LOW)
//	Input LOW  -> NMOS OFF -> output disconnected from GND
type NMOS struct {
	Params MOSFETParams
}

// NewNMOS creates an NMOS transistor. If params is nil, default 180nm
// process parameters are used.
func NewNMOS(params *MOSFETParams) *NMOS {
	if params != nil {
		return &NMOS{Params: *params}
	}
	return &NMOS{Params: DefaultMOSFETParams()}
}

// Region determines the operating region given terminal voltages.
//
// The operating region determines which equations govern current flow:
//
//	Cutoff:     Vgs < Vth            (gate voltage below threshold)
//	Linear:     Vgs >= Vth AND Vds < Vgs - Vth
//	Saturation: Vgs >= Vth AND Vds >= Vgs - Vth
func (n *NMOS) Region(vgs, vds float64) string {
	vth := n.Params.Vth

	if vgs < vth {
		return MOSFETCutoff
	}

	// Overdrive voltage: how far above threshold the gate is driven.
	vov := vgs - vth
	if vds < vov {
		return MOSFETLinear
	}
	return MOSFETSaturation
}

// DrainCurrent calculates drain-to-source current (Ids) in amperes.
//
// Uses the simplified MOSFET current equations (Shockley model):
//
//	Cutoff:     Ids = 0 (no channel, no current)
//	Linear:     Ids = K * ((Vgs - Vth) * Vds - 0.5 * Vds^2)
//	Saturation: Ids = 0.5 * K * (Vgs - Vth)^2
func (n *NMOS) DrainCurrent(vgs, vds float64) float64 {
	region := n.Region(vgs, vds)
	k := n.Params.K
	vth := n.Params.Vth

	if region == MOSFETCutoff {
		return 0.0
	}

	vov := vgs - vth // Overdrive voltage

	if region == MOSFETLinear {
		// Linear/ohmic region: transistor acts like a voltage-controlled resistor.
		// Current increases with both Vgs and Vds.
		return k * (vov*vds - 0.5*vds*vds)
	}

	// Saturation region: channel is "pinched off" at the drain end.
	// Current depends only on Vgs, not Vds. This is why saturation
	// is used for amplifiers — output current is controlled solely
	// by input voltage.
	return 0.5 * k * vov * vov
}

// IsConducting returns true when the gate voltage exceeds the threshold.
// This is the simplified digital view: ON or OFF, no in-between.
func (n *NMOS) IsConducting(vgs float64) bool {
	return vgs >= n.Params.Vth
}

// OutputVoltage returns the output voltage when used as a pull-down switch.
//
// In a CMOS circuit, NMOS transistors form the pull-down network:
//
//	ON:  output ~ 0V   (pulled to ground through low-resistance channel)
//	OFF: output ~ Vdd  (pulled up by the PMOS network)
func (n *NMOS) OutputVoltage(vgs, vdd float64) float64 {
	if n.IsConducting(vgs) {
		return 0.0
	}
	return vdd
}

// Transconductance calculates small-signal transconductance gm = dIds/dVgs.
//
// This is the key parameter for amplifier design. In saturation:
//
//	gm = K * (Vgs - Vth)
//
// Higher gm = more gain, but also more power consumption.
func (n *NMOS) Transconductance(vgs, vds float64) float64 {
	region := n.Region(vgs, vds)
	if region == MOSFETCutoff {
		return 0.0
	}

	vov := vgs - n.Params.Vth
	return n.Params.K * vov
}

// PMOS represents a P-channel MOSFET transistor.
//
// A PMOS transistor is the complement of NMOS. It conducts current from
// source to drain when the gate voltage is LOW (below the source voltage
// by more than |Vth|). Think of it as a normally-CLOSED switch that OPENS
// when you apply voltage.
//
// PMOS transistors form the pull-UP network in CMOS gates:
//
//	Vdd
//	 |
//	 | PMOS (gate = input signal)
//	 |
//	Output
//
//	Input LOW  -> PMOS ON  -> output pulled to Vdd (HIGH)
//	Input HIGH -> PMOS OFF -> output disconnected from Vdd
//
// PMOS uses the same equations as NMOS, but with reversed voltage
// polarities. For PMOS, Vgs and Vds are typically negative.
type PMOS struct {
	Params MOSFETParams
}

// NewPMOS creates a PMOS transistor. If params is nil, default parameters are used.
func NewPMOS(params *MOSFETParams) *PMOS {
	if params != nil {
		return &PMOS{Params: *params}
	}
	return &PMOS{Params: DefaultMOSFETParams()}
}

// Region determines the operating region for PMOS using absolute values.
//
//	Cutoff:     |Vgs| < Vth
//	Linear:     |Vgs| >= Vth AND |Vds| < |Vgs| - Vth
//	Saturation: |Vgs| >= Vth AND |Vds| >= |Vgs| - Vth
func (p *PMOS) Region(vgs, vds float64) string {
	vth := p.Params.Vth
	absVgs := math.Abs(vgs)
	absVds := math.Abs(vds)

	if absVgs < vth {
		return MOSFETCutoff
	}

	vov := absVgs - vth
	if absVds < vov {
		return MOSFETLinear
	}
	return MOSFETSaturation
}

// DrainCurrent calculates source-to-drain current for PMOS.
// Same equations as NMOS but using absolute values of voltages.
// Current magnitude is returned (always >= 0).
func (p *PMOS) DrainCurrent(vgs, vds float64) float64 {
	region := p.Region(vgs, vds)
	k := p.Params.K
	vth := p.Params.Vth

	if region == MOSFETCutoff {
		return 0.0
	}

	absVgs := math.Abs(vgs)
	absVds := math.Abs(vds)
	vov := absVgs - vth

	if region == MOSFETLinear {
		return k * (vov*absVds - 0.5*absVds*absVds)
	}

	return 0.5 * k * vov * vov
}

// IsConducting returns true when |Vgs| >= Vth (PMOS turns ON when
// gate is pulled below the source).
func (p *PMOS) IsConducting(vgs float64) bool {
	return math.Abs(vgs) >= p.Params.Vth
}

// OutputVoltage returns the output voltage when used as a pull-up switch.
//
//	ON:  output ~ Vdd (pulled to supply through low-resistance channel)
//	OFF: output ~ 0V  (pulled down by NMOS network)
func (p *PMOS) OutputVoltage(vgs, vdd float64) float64 {
	if p.IsConducting(vgs) {
		return vdd
	}
	return 0.0
}

// Transconductance calculates small-signal transconductance gm for PMOS.
// Same formula as NMOS but using absolute values.
func (p *PMOS) Transconductance(vgs, vds float64) float64 {
	region := p.Region(vgs, vds)
	if region == MOSFETCutoff {
		return 0.0
	}

	vov := math.Abs(vgs) - p.Params.Vth
	return p.Params.K * vov
}
