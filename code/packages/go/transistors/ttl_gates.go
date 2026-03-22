package transistors

// TTL Logic Gates — historical BJT-based digital logic.
//
// === What is TTL? ===
//
// TTL stands for Transistor-Transistor Logic. It was the dominant digital
// logic family from the mid-1960s through the 1980s. The "7400 series" —
// a family of TTL chips — defined the standard logic gates.
//
// === Why TTL Lost to CMOS ===
//
// TTL's fatal flaw: STATIC POWER CONSUMPTION.
//
// In a TTL gate, current flows through resistors and transistors even when
// the gate is doing nothing. A single TTL NAND gate dissipates ~1-10 mW:
//
//	1 million gates x 10 mW/gate = 10,000 watts (a space heater!)
//
// CMOS gates consume near-zero power at rest, allowing chips to scale
// to billions of gates.
//
// === RTL: The Predecessor to TTL ===
//
// Before TTL came RTL (Resistor-Transistor Logic). An RTL inverter is just
// one transistor with two resistors. It was used in the Apollo Guidance
// Computer that landed humans on the moon in 1969.

import "fmt"

// TTLNand is a TTL NAND gate using NPN transistors (7400-series style).
//
// Simplified circuit:
//
//	     Vcc (+5V)
//	      |
//	      R1 (4k ohm)
//	      |
//	 +----+----+
//	 |  Q1     |  Multi-emitter input transistor
//	 |  (NPN)  |
//	 +-- E1 ---+-- Input A
//	 +-- E2 ---+-- Input B
//	 +----+----+
//	      |
//	 +----+----+
//	 |  Q2     |  Phase splitter
//	 +----+----+
//	      |
//	 +----+----+
//	 |  Q3     |  Output transistor
//	 +----+----+
//	      |
//	     GND
//
// Any input LOW -> output HIGH. ALL inputs HIGH -> output LOW (NAND).
type TTLNand struct {
	Vcc     float64
	Params  BJTParams
	RPullup float64
	Q1      *NPN
	Q2      *NPN
	Q3      *NPN
}

// NewTTLNand creates a TTL NAND gate. Default Vcc is 5V.
func NewTTLNand(vcc float64, params *BJTParams) *TTLNand {
	var p BJTParams
	if params != nil {
		p = *params
	} else {
		p = DefaultBJTParams()
	}
	return &TTLNand{
		Vcc:     vcc,
		Params:  p,
		RPullup: 4000.0, // 4k ohm pull-up resistor
		Q1:      NewNPN(&p),
		Q2:      NewNPN(&p),
		Q3:      NewNPN(&p),
	}
}

// Evaluate evaluates the TTL NAND gate with analog input voltages.
func (g *TTLNand) Evaluate(va, vb float64) GateOutput {
	vcc := g.Vcc
	vbeOn := g.Params.VbeOn

	// TTL input thresholds: LOW < 0.8V, HIGH > 2.0V
	aHigh := va > 2.0
	bHigh := vb > 2.0

	var outputV float64
	var logicValue int
	var current float64

	if aHigh && bHigh {
		// ALL inputs HIGH -> output LOW
		outputV = g.Params.VceSat // ~0.2V
		logicValue = 0

		// Static current: Vcc through resistor chain
		// I ~ (Vcc - Vbe_Q2 - Vbe_Q3 - Vce_sat_Q3) / R_pullup
		current = (vcc - 2*vbeOn - g.Params.VceSat) / g.RPullup
		if current < 0 {
			current = 0
		}
	} else {
		// At least one input LOW -> output HIGH
		outputV = vcc - vbeOn // ~4.3V (Vcc minus one diode drop)
		logicValue = 1

		// Small bias current through pull-up
		current = (vcc - outputV) / g.RPullup
		if current < 0 {
			current = 0
		}
	}

	power := current * vcc
	delay := 10e-9 // 10 ns typical for TTL

	return GateOutput{
		LogicValue:       logicValue,
		Voltage:          outputV,
		CurrentDraw:      current,
		PowerDissipation: power,
		PropagationDelay: delay,
		TransistorCount:  3, // Simplified: Q1 + Q2 + Q3
	}
}

// EvaluateDigital evaluates with digital inputs (0 or 1).
func (g *TTLNand) EvaluateDigital(a, b int) (int, error) {
	if err := validateBit(a, "a"); err != nil {
		return 0, err
	}
	if err := validateBit(b, "b"); err != nil {
		return 0, err
	}
	va := 0.0
	if a == 1 {
		va = g.Vcc
	}
	vb := 0.0
	if b == 1 {
		vb = g.Vcc
	}
	return g.Evaluate(va, vb).LogicValue, nil
}

// StaticPower returns the static power dissipation — significantly
// higher than CMOS. TTL gates consume power continuously due to
// resistor-based biasing. Worst case is when output is LOW.
func (g *TTLNand) StaticPower() float64 {
	// Worst case: output LOW, all inputs HIGH
	current := (g.Vcc - 2*g.Params.VbeOn - g.Params.VceSat) / g.RPullup
	if current < 0 {
		current = 0
	}
	return current * g.Vcc
}

// RTLInverter is a Resistor-Transistor Logic inverter — the earliest IC
// logic family.
//
// Circuit:
//
//	     Vcc
//	      |
//	     Rc (collector resistor, ~1k ohm)
//	      |
//	 +----+----+
//	 |  Q1     |  Single NPN transistor
//	 +----+----+
//	      |
//	     GND
//
//	Input -- Rb (base resistor, ~10k ohm) -- Base of Q1
//
// Input HIGH: Q1 saturates -> output LOW.
// Input LOW:  Q1 cutoff -> output pulled HIGH through Rc.
//
// RTL was used in the Apollo Guidance Computer (AGC) that navigated
// Apollo 11 to the moon in 1969.
type RTLInverter struct {
	Vcc        float64
	RBase      float64
	RCollector float64
	Params     BJTParams
	Q1         *NPN
}

// NewRTLInverter creates an RTL inverter.
func NewRTLInverter(vcc, rBase, rCollector float64, params *BJTParams) *RTLInverter {
	var p BJTParams
	if params != nil {
		p = *params
	} else {
		p = DefaultBJTParams()
	}
	return &RTLInverter{
		Vcc:        vcc,
		RBase:      rBase,
		RCollector: rCollector,
		Params:     p,
		Q1:         NewNPN(&p),
	}
}

// Evaluate evaluates the RTL inverter with an analog input voltage.
func (g *RTLInverter) Evaluate(vInput float64) GateOutput {
	vcc := g.Vcc
	vbeOn := g.Params.VbeOn

	var outputV float64
	var logicValue int
	var current float64

	if vInput > vbeOn {
		// Q1 is ON — calculate base current and check saturation
		ib := (vInput - vbeOn) / g.RBase

		// Collector current: min of beta*Ib and circuit-limited current
		icMax := (vcc - g.Params.VceSat) / g.RCollector
		ic := ib * g.Params.Beta
		if ic > icMax {
			ic = icMax
		}

		outputV = vcc - ic*g.RCollector
		if outputV < g.Params.VceSat {
			outputV = g.Params.VceSat
		}

		if outputV < vcc/2.0 {
			logicValue = 0
		} else {
			logicValue = 1
		}
		current = ic + ib
	} else {
		// Q1 is OFF — output pulled to Vcc through Rc
		outputV = vcc
		logicValue = 1
		current = 0.0
	}

	power := current * vcc
	delay := 50e-9 // RTL is slow: ~50 ns typical

	return GateOutput{
		LogicValue:       logicValue,
		Voltage:          outputV,
		CurrentDraw:      current,
		PowerDissipation: power,
		PropagationDelay: delay,
		TransistorCount:  1,
	}
}

// EvaluateDigital evaluates with a digital input (0 or 1).
func (g *RTLInverter) EvaluateDigital(a int) (int, error) {
	if a != 0 && a != 1 {
		return 0, fmt.Errorf("a must be 0 or 1, got %d", a)
	}
	vInput := 0.0
	if a == 1 {
		vInput = g.Vcc
	}
	return g.Evaluate(vInput).LogicValue, nil
}
