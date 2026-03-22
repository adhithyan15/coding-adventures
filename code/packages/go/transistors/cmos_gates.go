package transistors

// CMOS Logic Gates — building digital logic from transistor pairs.
//
// === What is CMOS? ===
//
// CMOS stands for Complementary Metal-Oxide-Semiconductor. It is the
// technology used in virtually every digital chip made since the 1980s.
//
// The "complementary" refers to pairing NMOS and PMOS transistors:
//   - PMOS transistors form the PULL-UP network (connects output to Vdd)
//   - NMOS transistors form the PULL-DOWN network (connects output to GND)
//
// For any valid input combination, exactly ONE network is active:
//   - If pull-up is ON  -> output = Vdd (logic HIGH)
//   - If pull-down is ON -> output = GND (logic LOW)
//   - Never both ON simultaneously -> near-zero static power
//
// === Transistor Counts ===
//
//	Gate    | NMOS | PMOS | Total
//	--------|------|------|------
//	NOT     |  1   |  1   |   2
//	NAND    |  2   |  2   |   4
//	NOR     |  2   |  2   |   4
//	AND     |  3   |  3   |   6
//	OR      |  3   |  3   |   6
//	XOR     |  3   |  3   |   6

import "fmt"

// validateBit checks that a value is 0 or 1, returning an error if not.
// In Go we return errors rather than panicking for invalid inputs, giving
// callers the choice of how to handle bad data.
func validateBit(value int, name string) error {
	if value != 0 && value != 1 {
		return fmt.Errorf("%s must be 0 or 1, got %d", name, value)
	}
	return nil
}

// ===========================================================================
// CMOS INVERTER (NOT gate) — 2 transistors
// ===========================================================================

// CMOSInverter is a CMOS NOT gate: 1 PMOS + 1 NMOS = 2 transistors.
//
// The simplest and most important CMOS circuit. Every other CMOS gate
// is a variation of this fundamental pattern.
//
//	     Vdd
//	      |
//	 +----+----+
//	 |  PMOS   |--- Gate --- Input (A)
//	 +----+----+
//	      |
//	      +------------- Output (Y = NOT A)
//	      |
//	 +----+----+
//	 |  NMOS   |--- Gate --- Input (A)
//	 +----+----+
//	      |
//	     GND
//
// Input HIGH: NMOS ON, PMOS OFF -> output LOW.
// Input LOW:  NMOS OFF, PMOS ON -> output HIGH.
// Static power: ZERO (one transistor always OFF, breaking current path).
type CMOSInverter struct {
	Circuit CircuitParams
	Nmos    *NMOS
	Pmos    *PMOS
}

// NewCMOSInverter creates a CMOS inverter. Pass nil for any parameter
// to use defaults.
func NewCMOSInverter(circuit *CircuitParams, nmosParams, pmosParams *MOSFETParams) *CMOSInverter {
	var c CircuitParams
	if circuit != nil {
		c = *circuit
	} else {
		c = DefaultCircuitParams()
	}
	return &CMOSInverter{
		Circuit: c,
		Nmos:    NewNMOS(nmosParams),
		Pmos:    NewPMOS(pmosParams),
	}
}

// Evaluate runs the inverter with an analog input voltage and returns
// full electrical detail.
func (g *CMOSInverter) Evaluate(inputVoltage float64) GateOutput {
	vdd := g.Circuit.Vdd

	// NMOS: gate = input, source = GND -> Vgs_n = Vin
	vgsN := inputVoltage
	// PMOS: gate = input, source = Vdd -> Vgs_p = Vin - Vdd (negative when LOW)
	vgsP := inputVoltage - vdd

	nmosOn := g.Nmos.IsConducting(vgsN)
	pmosOn := g.Pmos.IsConducting(vgsP)

	// Determine output voltage
	var outputV float64
	switch {
	case pmosOn && !nmosOn:
		outputV = vdd // PMOS pulls to Vdd
	case nmosOn && !pmosOn:
		outputV = 0.0 // NMOS pulls to GND
	default:
		// Both on (transition region) or both off — approximate as Vdd/2
		outputV = vdd / 2.0
	}

	// Digital interpretation: above Vdd/2 is logic 1
	logicValue := 0
	if outputV > vdd/2.0 {
		logicValue = 1
	}

	// Current draw: only significant during transition (both on)
	var current float64
	if nmosOn && pmosOn {
		vdsN := vdd / 2.0
		current = g.Nmos.DrainCurrent(vgsN, vdsN)
	}

	power := current * vdd

	// Propagation delay estimate
	cLoad := g.Nmos.Params.CDrain + g.Pmos.Params.CDrain
	var delay float64
	if current > 0 {
		delay = cLoad * vdd / (2.0 * current)
	} else {
		idsSat := g.Nmos.DrainCurrent(vdd, vdd)
		if idsSat > 0 {
			delay = cLoad * vdd / (2.0 * idsSat)
		} else {
			delay = 1e-9
		}
	}

	return GateOutput{
		LogicValue:       logicValue,
		Voltage:          outputV,
		CurrentDraw:      current,
		PowerDissipation: power,
		PropagationDelay: delay,
		TransistorCount:  2,
	}
}

// EvaluateDigital evaluates with a digital input (0 or 1), returning 0 or 1.
func (g *CMOSInverter) EvaluateDigital(a int) (int, error) {
	if err := validateBit(a, "a"); err != nil {
		return 0, err
	}
	vin := 0.0
	if a == 1 {
		vin = g.Circuit.Vdd
	}
	return g.Evaluate(vin).LogicValue, nil
}

// StaticPower returns the static power dissipation (ideally ~0 for CMOS).
func (g *CMOSInverter) StaticPower() float64 {
	return 0.0
}

// DynamicPower returns P = C_load * Vdd^2 * frequency.
//
// This is the dominant power consumption mechanism in CMOS.
// Every time the output switches, the load capacitance must be
// charged or discharged. The energy per transition is C * Vdd^2.
func (g *CMOSInverter) DynamicPower(frequency, cLoad float64) float64 {
	vdd := g.Circuit.Vdd
	return cLoad * vdd * vdd * frequency
}

// VoltageTransferCharacteristic generates the VTC curve: a slice of
// [Vin, Vout] points showing the sharp switching threshold of CMOS.
func (g *CMOSInverter) VoltageTransferCharacteristic(steps int) [][2]float64 {
	vdd := g.Circuit.Vdd
	points := make([][2]float64, 0, steps+1)
	for i := 0; i <= steps; i++ {
		vin := vdd * float64(i) / float64(steps)
		result := g.Evaluate(vin)
		points = append(points, [2]float64{vin, result.Voltage})
	}
	return points
}

// ===========================================================================
// CMOS NAND — 4 transistors
// ===========================================================================

// CMOSNand is a CMOS NAND gate: 2 PMOS parallel + 2 NMOS series = 4 transistors.
//
// Pull-down: NMOS in SERIES  -> BOTH must be ON to pull output LOW.
// Pull-up:   PMOS in PARALLEL -> EITHER can pull output HIGH.
//
// This is why NAND is the "natural" CMOS gate — it requires only 4
// transistors. AND needs 6 (NAND + inverter).
type CMOSNand struct {
	Circuit CircuitParams
	Nmos1   *NMOS
	Nmos2   *NMOS
	Pmos1   *PMOS
	Pmos2   *PMOS
}

// NewCMOSNand creates a CMOS NAND gate.
func NewCMOSNand(circuit *CircuitParams, nmosParams, pmosParams *MOSFETParams) *CMOSNand {
	var c CircuitParams
	if circuit != nil {
		c = *circuit
	} else {
		c = DefaultCircuitParams()
	}
	return &CMOSNand{
		Circuit: c,
		Nmos1:   NewNMOS(nmosParams),
		Nmos2:   NewNMOS(nmosParams),
		Pmos1:   NewPMOS(pmosParams),
		Pmos2:   NewPMOS(pmosParams),
	}
}

// Evaluate evaluates the NAND gate with analog input voltages.
func (g *CMOSNand) Evaluate(va, vb float64) GateOutput {
	vdd := g.Circuit.Vdd

	vgsN1 := va
	vgsN2 := vb
	vgsP1 := va - vdd
	vgsP2 := vb - vdd

	nmos1On := g.Nmos1.IsConducting(vgsN1)
	nmos2On := g.Nmos2.IsConducting(vgsN2)
	pmos1On := g.Pmos1.IsConducting(vgsP1)
	pmos2On := g.Pmos2.IsConducting(vgsP2)

	// Pull-down: NMOS in SERIES — BOTH must be ON
	pulldownOn := nmos1On && nmos2On
	// Pull-up: PMOS in PARALLEL — EITHER can pull up
	pullupOn := pmos1On || pmos2On

	var outputV float64
	switch {
	case pullupOn && !pulldownOn:
		outputV = vdd
	case pulldownOn && !pullupOn:
		outputV = 0.0
	default:
		outputV = vdd / 2.0
	}

	logicValue := 0
	if outputV > vdd/2.0 {
		logicValue = 1
	}

	current := 0.0
	if pulldownOn && pullupOn {
		current = 0.001
	}

	cLoad := g.Nmos1.Params.CDrain + g.Pmos1.Params.CDrain
	idsSat := g.Nmos1.DrainCurrent(vdd, vdd)
	delay := 1e-9
	if idsSat > 0 {
		delay = cLoad * vdd / (2.0 * idsSat)
	}

	return GateOutput{
		LogicValue:       logicValue,
		Voltage:          outputV,
		CurrentDraw:      current,
		PowerDissipation: current * vdd,
		PropagationDelay: delay,
		TransistorCount:  4,
	}
}

// EvaluateDigital evaluates with digital inputs (0 or 1).
func (g *CMOSNand) EvaluateDigital(a, b int) (int, error) {
	if err := validateBit(a, "a"); err != nil {
		return 0, err
	}
	if err := validateBit(b, "b"); err != nil {
		return 0, err
	}
	vdd := g.Circuit.Vdd
	va := 0.0
	if a == 1 {
		va = vdd
	}
	vb := 0.0
	if b == 1 {
		vb = vdd
	}
	return g.Evaluate(va, vb).LogicValue, nil
}

// TransistorCount returns 4.
func (g *CMOSNand) TransistorCount() int {
	return 4
}

// ===========================================================================
// CMOS NOR — 4 transistors
// ===========================================================================

// CMOSNor is a CMOS NOR gate: 2 PMOS series + 2 NMOS parallel = 4 transistors.
//
// Pull-down: NMOS in PARALLEL -> EITHER ON pulls output LOW.
// Pull-up:   PMOS in SERIES   -> BOTH must be ON to pull output HIGH.
type CMOSNor struct {
	Circuit CircuitParams
	Nmos1   *NMOS
	Nmos2   *NMOS
	Pmos1   *PMOS
	Pmos2   *PMOS
}

// NewCMOSNor creates a CMOS NOR gate.
func NewCMOSNor(circuit *CircuitParams, nmosParams, pmosParams *MOSFETParams) *CMOSNor {
	var c CircuitParams
	if circuit != nil {
		c = *circuit
	} else {
		c = DefaultCircuitParams()
	}
	return &CMOSNor{
		Circuit: c,
		Nmos1:   NewNMOS(nmosParams),
		Nmos2:   NewNMOS(nmosParams),
		Pmos1:   NewPMOS(pmosParams),
		Pmos2:   NewPMOS(pmosParams),
	}
}

// Evaluate evaluates the NOR gate with analog input voltages.
func (g *CMOSNor) Evaluate(va, vb float64) GateOutput {
	vdd := g.Circuit.Vdd

	vgsN1 := va
	vgsN2 := vb
	vgsP1 := va - vdd
	vgsP2 := vb - vdd

	nmos1On := g.Nmos1.IsConducting(vgsN1)
	nmos2On := g.Nmos2.IsConducting(vgsN2)
	pmos1On := g.Pmos1.IsConducting(vgsP1)
	pmos2On := g.Pmos2.IsConducting(vgsP2)

	// Pull-down: NMOS in PARALLEL — EITHER ON pulls low
	pulldownOn := nmos1On || nmos2On
	// Pull-up: PMOS in SERIES — BOTH must be ON
	pullupOn := pmos1On && pmos2On

	var outputV float64
	switch {
	case pullupOn && !pulldownOn:
		outputV = vdd
	case pulldownOn && !pullupOn:
		outputV = 0.0
	default:
		outputV = vdd / 2.0
	}

	logicValue := 0
	if outputV > vdd/2.0 {
		logicValue = 1
	}

	current := 0.0
	if pulldownOn && pullupOn {
		current = 0.001
	}

	cLoad := g.Nmos1.Params.CDrain + g.Pmos1.Params.CDrain
	idsSat := g.Nmos1.DrainCurrent(vdd, vdd)
	delay := 1e-9
	if idsSat > 0 {
		delay = cLoad * vdd / (2.0 * idsSat)
	}

	return GateOutput{
		LogicValue:       logicValue,
		Voltage:          outputV,
		CurrentDraw:      current,
		PowerDissipation: current * vdd,
		PropagationDelay: delay,
		TransistorCount:  4,
	}
}

// EvaluateDigital evaluates with digital inputs (0 or 1).
func (g *CMOSNor) EvaluateDigital(a, b int) (int, error) {
	if err := validateBit(a, "a"); err != nil {
		return 0, err
	}
	if err := validateBit(b, "b"); err != nil {
		return 0, err
	}
	vdd := g.Circuit.Vdd
	va := 0.0
	if a == 1 {
		va = vdd
	}
	vb := 0.0
	if b == 1 {
		vb = vdd
	}
	return g.Evaluate(va, vb).LogicValue, nil
}

// ===========================================================================
// CMOS AND — 6 transistors (NAND + Inverter)
// ===========================================================================

// CMOSAnd is a CMOS AND gate: NAND + Inverter = 6 transistors.
//
// There is no "direct" CMOS AND gate. The CMOS topology naturally
// produces inverted outputs, so to get AND we must add an inverter.
type CMOSAnd struct {
	Circuit CircuitParams
	nand    *CMOSNand
	inv     *CMOSInverter
}

// NewCMOSAnd creates a CMOS AND gate.
func NewCMOSAnd(circuit *CircuitParams) *CMOSAnd {
	var c CircuitParams
	if circuit != nil {
		c = *circuit
	} else {
		c = DefaultCircuitParams()
	}
	return &CMOSAnd{
		Circuit: c,
		nand:    NewCMOSNand(&c, nil, nil),
		inv:     NewCMOSInverter(&c, nil, nil),
	}
}

// Evaluate evaluates AND = NOT(NAND(A, B)).
func (g *CMOSAnd) Evaluate(va, vb float64) GateOutput {
	nandOut := g.nand.Evaluate(va, vb)
	invOut := g.inv.Evaluate(nandOut.Voltage)
	return GateOutput{
		LogicValue:       invOut.LogicValue,
		Voltage:          invOut.Voltage,
		CurrentDraw:      nandOut.CurrentDraw + invOut.CurrentDraw,
		PowerDissipation: nandOut.PowerDissipation + invOut.PowerDissipation,
		PropagationDelay: nandOut.PropagationDelay + invOut.PropagationDelay,
		TransistorCount:  6,
	}
}

// EvaluateDigital evaluates with digital inputs.
func (g *CMOSAnd) EvaluateDigital(a, b int) (int, error) {
	if err := validateBit(a, "a"); err != nil {
		return 0, err
	}
	if err := validateBit(b, "b"); err != nil {
		return 0, err
	}
	vdd := g.Circuit.Vdd
	va := 0.0
	if a == 1 {
		va = vdd
	}
	vb := 0.0
	if b == 1 {
		vb = vdd
	}
	return g.Evaluate(va, vb).LogicValue, nil
}

// ===========================================================================
// CMOS OR — 6 transistors (NOR + Inverter)
// ===========================================================================

// CMOSOr is a CMOS OR gate: NOR + Inverter = 6 transistors.
type CMOSOr struct {
	Circuit CircuitParams
	nor     *CMOSNor
	inv     *CMOSInverter
}

// NewCMOSOr creates a CMOS OR gate.
func NewCMOSOr(circuit *CircuitParams) *CMOSOr {
	var c CircuitParams
	if circuit != nil {
		c = *circuit
	} else {
		c = DefaultCircuitParams()
	}
	return &CMOSOr{
		Circuit: c,
		nor:     NewCMOSNor(&c, nil, nil),
		inv:     NewCMOSInverter(&c, nil, nil),
	}
}

// Evaluate evaluates OR = NOT(NOR(A, B)).
func (g *CMOSOr) Evaluate(va, vb float64) GateOutput {
	norOut := g.nor.Evaluate(va, vb)
	invOut := g.inv.Evaluate(norOut.Voltage)
	return GateOutput{
		LogicValue:       invOut.LogicValue,
		Voltage:          invOut.Voltage,
		CurrentDraw:      norOut.CurrentDraw + invOut.CurrentDraw,
		PowerDissipation: norOut.PowerDissipation + invOut.PowerDissipation,
		PropagationDelay: norOut.PropagationDelay + invOut.PropagationDelay,
		TransistorCount:  6,
	}
}

// EvaluateDigital evaluates with digital inputs.
func (g *CMOSOr) EvaluateDigital(a, b int) (int, error) {
	if err := validateBit(a, "a"); err != nil {
		return 0, err
	}
	if err := validateBit(b, "b"); err != nil {
		return 0, err
	}
	vdd := g.Circuit.Vdd
	va := 0.0
	if a == 1 {
		va = vdd
	}
	vb := 0.0
	if b == 1 {
		vb = vdd
	}
	return g.Evaluate(va, vb).LogicValue, nil
}

// ===========================================================================
// CMOS XOR — 6 transistors (4 NANDs conceptually)
// ===========================================================================

// CMOSXor is a CMOS XOR gate using 4 NAND gates internally.
//
// XOR(A, B) = NAND(NAND(A, NAND(A,B)), NAND(B, NAND(A,B)))
//
// This construction proves that XOR can be built from the universal
// NAND gate alone.
type CMOSXor struct {
	Circuit CircuitParams
	nand1   *CMOSNand
	nand2   *CMOSNand
	nand3   *CMOSNand
	nand4   *CMOSNand
}

// NewCMOSXor creates a CMOS XOR gate.
func NewCMOSXor(circuit *CircuitParams) *CMOSXor {
	var c CircuitParams
	if circuit != nil {
		c = *circuit
	} else {
		c = DefaultCircuitParams()
	}
	return &CMOSXor{
		Circuit: c,
		nand1:   NewCMOSNand(&c, nil, nil),
		nand2:   NewCMOSNand(&c, nil, nil),
		nand3:   NewCMOSNand(&c, nil, nil),
		nand4:   NewCMOSNand(&c, nil, nil),
	}
}

// Evaluate evaluates XOR using 4 NAND gates.
func (g *CMOSXor) Evaluate(va, vb float64) GateOutput {
	vdd := g.Circuit.Vdd

	// Step 1: NAND(A, B)
	nandAB := g.nand1.Evaluate(va, vb)
	// Step 2: NAND(A, NAND(A,B))
	nandANab := g.nand2.Evaluate(va, nandAB.Voltage)
	// Step 3: NAND(B, NAND(A,B))
	nandBNab := g.nand3.Evaluate(vb, nandAB.Voltage)
	// Step 4: NAND(step2, step3)
	result := g.nand4.Evaluate(nandANab.Voltage, nandBNab.Voltage)

	totalCurrent := nandAB.CurrentDraw + nandANab.CurrentDraw +
		nandBNab.CurrentDraw + result.CurrentDraw

	// Critical path: nand1 -> max(nand2, nand3) -> nand4
	maxMiddle := nandANab.PropagationDelay
	if nandBNab.PropagationDelay > maxMiddle {
		maxMiddle = nandBNab.PropagationDelay
	}
	totalDelay := nandAB.PropagationDelay + maxMiddle + result.PropagationDelay

	return GateOutput{
		LogicValue:       result.LogicValue,
		Voltage:          result.Voltage,
		CurrentDraw:      totalCurrent,
		PowerDissipation: totalCurrent * vdd,
		PropagationDelay: totalDelay,
		TransistorCount:  6,
	}
}

// EvaluateDigital evaluates with digital inputs.
func (g *CMOSXor) EvaluateDigital(a, b int) (int, error) {
	if err := validateBit(a, "a"); err != nil {
		return 0, err
	}
	if err := validateBit(b, "b"); err != nil {
		return 0, err
	}
	vdd := g.Circuit.Vdd
	va := 0.0
	if a == 1 {
		va = vdd
	}
	vb := 0.0
	if b == 1 {
		vb = vdd
	}
	return g.Evaluate(va, vb).LogicValue, nil
}

// EvaluateFromNands builds XOR from 4 NAND gates to demonstrate universality.
// This is the same as EvaluateDigital but makes the NAND construction explicit.
func (g *CMOSXor) EvaluateFromNands(a, b int) (int, error) {
	return g.EvaluateDigital(a, b)
}
