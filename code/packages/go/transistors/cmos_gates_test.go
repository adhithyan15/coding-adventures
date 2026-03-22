package transistors

import "testing"

// ===========================================================================
// CMOS Inverter Tests
// ===========================================================================

func TestCMOSInverter_TruthTable(t *testing.T) {
	// NOT gate: 0->1, 1->0.
	inv := NewCMOSInverter(nil, nil, nil)

	val, err := inv.EvaluateDigital(0)
	if err != nil {
		t.Fatal(err)
	}
	if val != 1 {
		t.Errorf("NOT(0) = %d, want 1", val)
	}

	val, err = inv.EvaluateDigital(1)
	if err != nil {
		t.Fatal(err)
	}
	if val != 0 {
		t.Errorf("NOT(1) = %d, want 0", val)
	}
}

func TestCMOSInverter_VoltageSwingHighInput(t *testing.T) {
	// Input HIGH -> output near GND.
	c := CircuitParams{Vdd: 3.3, Temperature: 300}
	inv := NewCMOSInverter(&c, nil, nil)
	result := inv.Evaluate(3.3)
	if result.Voltage >= 0.1 {
		t.Errorf("Evaluate(3.3).Voltage = %v, want < 0.1", result.Voltage)
	}
}

func TestCMOSInverter_VoltageSwingLowInput(t *testing.T) {
	// Input LOW -> output near Vdd.
	c := CircuitParams{Vdd: 3.3, Temperature: 300}
	inv := NewCMOSInverter(&c, nil, nil)
	result := inv.Evaluate(0.0)
	if result.Voltage <= 3.2 {
		t.Errorf("Evaluate(0.0).Voltage = %v, want > 3.2", result.Voltage)
	}
}

func TestCMOSInverter_StaticPowerZero(t *testing.T) {
	// CMOS should have near-zero static power.
	inv := NewCMOSInverter(nil, nil, nil)
	if sp := inv.StaticPower(); sp > 1e-9 {
		t.Errorf("StaticPower() = %v, want < 1e-9", sp)
	}
}

func TestCMOSInverter_DynamicPower(t *testing.T) {
	// Dynamic power should be positive and scale with V^2.
	c := CircuitParams{Vdd: 3.3, Temperature: 300}
	inv := NewCMOSInverter(&c, nil, nil)
	p := inv.DynamicPower(1e9, 1e-12)
	if p <= 0 {
		t.Errorf("DynamicPower(1e9, 1e-12) = %v, want > 0", p)
	}
}

func TestCMOSInverter_DynamicPowerScalesWithVSquared(t *testing.T) {
	// Halving Vdd should reduce dynamic power by ~4x.
	cHigh := CircuitParams{Vdd: 3.3, Temperature: 300}
	cLow := CircuitParams{Vdd: 1.65, Temperature: 300}
	invHigh := NewCMOSInverter(&cHigh, nil, nil)
	invLow := NewCMOSInverter(&cLow, nil, nil)
	pHigh := invHigh.DynamicPower(1e9, 1e-12)
	pLow := invLow.DynamicPower(1e9, 1e-12)
	ratio := pHigh / pLow
	if ratio < 3.5 || ratio > 4.5 {
		t.Errorf("Power ratio = %v, want between 3.5 and 4.5", ratio)
	}
}

func TestCMOSInverter_VTCHasSharpTransition(t *testing.T) {
	// VTC should show output snap from HIGH to LOW.
	c := CircuitParams{Vdd: 3.3, Temperature: 300}
	inv := NewCMOSInverter(&c, nil, nil)
	vtc := inv.VoltageTransferCharacteristic(10)
	if len(vtc) != 11 {
		t.Errorf("VTC length = %d, want 11", len(vtc))
	}
	// First point: input=0, output should be HIGH
	if vtc[0][1] <= 3.0 {
		t.Errorf("VTC[0] output = %v, want > 3.0", vtc[0][1])
	}
	// Last point: input=Vdd, output should be LOW
	if vtc[len(vtc)-1][1] >= 0.5 {
		t.Errorf("VTC[-1] output = %v, want < 0.5", vtc[len(vtc)-1][1])
	}
}

func TestCMOSInverter_RejectsInvalidInput(t *testing.T) {
	inv := NewCMOSInverter(nil, nil, nil)
	if _, err := inv.EvaluateDigital(2); err == nil {
		t.Error("EvaluateDigital(2) should return error")
	}
	if _, err := inv.EvaluateDigital(-1); err == nil {
		t.Error("EvaluateDigital(-1) should return error")
	}
}

func TestCMOSInverter_TransistorCount(t *testing.T) {
	// Inverter uses 2 transistors.
	inv := NewCMOSInverter(nil, nil, nil)
	result := inv.Evaluate(0.0)
	if result.TransistorCount != 2 {
		t.Errorf("TransistorCount = %d, want 2", result.TransistorCount)
	}
}

// ===========================================================================
// CMOS NAND Tests
// ===========================================================================

func TestCMOSNand_TruthTable(t *testing.T) {
	nand := NewCMOSNand(nil, nil, nil)
	cases := [][3]int{{0, 0, 1}, {0, 1, 1}, {1, 0, 1}, {1, 1, 0}}
	for _, c := range cases {
		val, err := nand.EvaluateDigital(c[0], c[1])
		if err != nil {
			t.Fatal(err)
		}
		if val != c[2] {
			t.Errorf("NAND(%d, %d) = %d, want %d", c[0], c[1], val, c[2])
		}
	}
}

func TestCMOSNand_TransistorCount(t *testing.T) {
	nand := NewCMOSNand(nil, nil, nil)
	if nand.TransistorCount() != 4 {
		t.Errorf("TransistorCount() = %d, want 4", nand.TransistorCount())
	}
}

func TestCMOSNand_VoltageOutputHigh(t *testing.T) {
	c := CircuitParams{Vdd: 3.3, Temperature: 300}
	nand := NewCMOSNand(&c, nil, nil)
	result := nand.Evaluate(0.0, 0.0)
	if result.Voltage <= 3.0 {
		t.Errorf("NAND(0,0) voltage = %v, want > 3.0", result.Voltage)
	}
}

func TestCMOSNand_VoltageOutputLow(t *testing.T) {
	c := CircuitParams{Vdd: 3.3, Temperature: 300}
	nand := NewCMOSNand(&c, nil, nil)
	result := nand.Evaluate(3.3, 3.3)
	if result.Voltage >= 0.5 {
		t.Errorf("NAND(Vdd,Vdd) voltage = %v, want < 0.5", result.Voltage)
	}
}

func TestCMOSNand_RejectsInvalidInput(t *testing.T) {
	nand := NewCMOSNand(nil, nil, nil)
	if _, err := nand.EvaluateDigital(2, 0); err == nil {
		t.Error("EvaluateDigital(2, 0) should return error")
	}
}

// ===========================================================================
// CMOS NOR Tests
// ===========================================================================

func TestCMOSNor_TruthTable(t *testing.T) {
	nor := NewCMOSNor(nil, nil, nil)
	cases := [][3]int{{0, 0, 1}, {0, 1, 0}, {1, 0, 0}, {1, 1, 0}}
	for _, c := range cases {
		val, err := nor.EvaluateDigital(c[0], c[1])
		if err != nil {
			t.Fatal(err)
		}
		if val != c[2] {
			t.Errorf("NOR(%d, %d) = %d, want %d", c[0], c[1], val, c[2])
		}
	}
}

func TestCMOSNor_RejectsInvalidInput(t *testing.T) {
	nor := NewCMOSNor(nil, nil, nil)
	if _, err := nor.EvaluateDigital(0, 2); err == nil {
		t.Error("EvaluateDigital(0, 2) should return error")
	}
}

// ===========================================================================
// CMOS AND Tests
// ===========================================================================

func TestCMOSAnd_TruthTable(t *testing.T) {
	andGate := NewCMOSAnd(nil)
	cases := [][3]int{{0, 0, 0}, {0, 1, 0}, {1, 0, 0}, {1, 1, 1}}
	for _, c := range cases {
		val, err := andGate.EvaluateDigital(c[0], c[1])
		if err != nil {
			t.Fatal(err)
		}
		if val != c[2] {
			t.Errorf("AND(%d, %d) = %d, want %d", c[0], c[1], val, c[2])
		}
	}
}

func TestCMOSAnd_RejectsInvalidInput(t *testing.T) {
	andGate := NewCMOSAnd(nil)
	if _, err := andGate.EvaluateDigital(2, 0); err == nil {
		t.Error("EvaluateDigital(2, 0) should return error")
	}
}

// ===========================================================================
// CMOS OR Tests
// ===========================================================================

func TestCMOSOr_TruthTable(t *testing.T) {
	orGate := NewCMOSOr(nil)
	cases := [][3]int{{0, 0, 0}, {0, 1, 1}, {1, 0, 1}, {1, 1, 1}}
	for _, c := range cases {
		val, err := orGate.EvaluateDigital(c[0], c[1])
		if err != nil {
			t.Fatal(err)
		}
		if val != c[2] {
			t.Errorf("OR(%d, %d) = %d, want %d", c[0], c[1], val, c[2])
		}
	}
}

func TestCMOSOr_RejectsInvalidInput(t *testing.T) {
	orGate := NewCMOSOr(nil)
	if _, err := orGate.EvaluateDigital(-1, 0); err == nil {
		t.Error("EvaluateDigital(-1, 0) should return error")
	}
}

// ===========================================================================
// CMOS XOR Tests
// ===========================================================================

func TestCMOSXor_TruthTable(t *testing.T) {
	xorGate := NewCMOSXor(nil)
	cases := [][3]int{{0, 0, 0}, {0, 1, 1}, {1, 0, 1}, {1, 1, 0}}
	for _, c := range cases {
		val, err := xorGate.EvaluateDigital(c[0], c[1])
		if err != nil {
			t.Fatal(err)
		}
		if val != c[2] {
			t.Errorf("XOR(%d, %d) = %d, want %d", c[0], c[1], val, c[2])
		}
	}
}

func TestCMOSXor_EvaluateFromNands(t *testing.T) {
	// NAND-based XOR should match direct XOR.
	xorGate := NewCMOSXor(nil)
	for _, a := range []int{0, 1} {
		for _, b := range []int{0, 1} {
			v1, err1 := xorGate.EvaluateFromNands(a, b)
			v2, err2 := xorGate.EvaluateDigital(a, b)
			if err1 != nil || err2 != nil {
				t.Fatalf("unexpected error: %v, %v", err1, err2)
			}
			if v1 != v2 {
				t.Errorf("EvaluateFromNands(%d,%d) = %d, EvaluateDigital = %d", a, b, v1, v2)
			}
		}
	}
}

func TestCMOSXor_RejectsInvalidInput(t *testing.T) {
	xorGate := NewCMOSXor(nil)
	if _, err := xorGate.EvaluateDigital(0, 2); err == nil {
		t.Error("EvaluateDigital(0, 2) should return error")
	}
}
