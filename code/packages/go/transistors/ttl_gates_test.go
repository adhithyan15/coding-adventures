package transistors

import "testing"

// ===========================================================================
// TTL NAND Tests
// ===========================================================================

func TestTTLNand_TruthTable(t *testing.T) {
	nand := NewTTLNand(5.0, nil)
	cases := [][3]int{{0, 0, 1}, {0, 1, 1}, {1, 0, 1}, {1, 1, 0}}
	for _, c := range cases {
		val, err := nand.EvaluateDigital(c[0], c[1])
		if err != nil {
			t.Fatal(err)
		}
		if val != c[2] {
			t.Errorf("TTL NAND(%d, %d) = %d, want %d", c[0], c[1], val, c[2])
		}
	}
}

func TestTTLNand_StaticPowerMilliwatts(t *testing.T) {
	// TTL gates dissipate milliwatts even when idle.
	nand := NewTTLNand(5.0, nil)
	if sp := nand.StaticPower(); sp <= 1e-3 {
		t.Errorf("StaticPower() = %v, want > 1e-3 (1 mW)", sp)
	}
}

func TestTTLNand_OutputVoltageLow(t *testing.T) {
	// Output LOW should be near Vce_sat (~0.2V).
	nand := NewTTLNand(5.0, nil)
	result := nand.Evaluate(5.0, 5.0)
	if result.Voltage >= 0.5 {
		t.Errorf("Evaluate(5,5).Voltage = %v, want < 0.5", result.Voltage)
	}
	if result.LogicValue != 0 {
		t.Errorf("Evaluate(5,5).LogicValue = %d, want 0", result.LogicValue)
	}
}

func TestTTLNand_OutputVoltageHigh(t *testing.T) {
	// Output HIGH should be near Vcc - 0.7V.
	nand := NewTTLNand(5.0, nil)
	result := nand.Evaluate(0.0, 0.0)
	if result.Voltage <= 3.0 {
		t.Errorf("Evaluate(0,0).Voltage = %v, want > 3.0", result.Voltage)
	}
	if result.LogicValue != 1 {
		t.Errorf("Evaluate(0,0).LogicValue = %d, want 1", result.LogicValue)
	}
}

func TestTTLNand_PropagationDelay(t *testing.T) {
	// TTL should have propagation delay in nanosecond range.
	nand := NewTTLNand(5.0, nil)
	result := nand.Evaluate(5.0, 5.0)
	if result.PropagationDelay < 1e-9 || result.PropagationDelay > 100e-9 {
		t.Errorf("PropagationDelay = %v, want between 1ns and 100ns",
			result.PropagationDelay)
	}
}

func TestTTLNand_RejectsInvalidInput(t *testing.T) {
	nand := NewTTLNand(5.0, nil)
	if _, err := nand.EvaluateDigital(2, 0); err == nil {
		t.Error("EvaluateDigital(2, 0) should return error")
	}
}

func TestTTLNand_CustomVcc(t *testing.T) {
	// Custom Vcc should be respected.
	nand := NewTTLNand(3.3, nil)
	if nand.Vcc != 3.3 {
		t.Errorf("Vcc = %v, want 3.3", nand.Vcc)
	}
}

// ===========================================================================
// RTL Inverter Tests
// ===========================================================================

func TestRTLInverter_TruthTable(t *testing.T) {
	inv := NewRTLInverter(5.0, 10000.0, 1000.0, nil)
	val, err := inv.EvaluateDigital(0)
	if err != nil {
		t.Fatal(err)
	}
	if val != 1 {
		t.Errorf("RTL NOT(0) = %d, want 1", val)
	}
	val, err = inv.EvaluateDigital(1)
	if err != nil {
		t.Fatal(err)
	}
	if val != 0 {
		t.Errorf("RTL NOT(1) = %d, want 0", val)
	}
}

func TestRTLInverter_OutputVoltageHigh(t *testing.T) {
	// Input LOW -> output near Vcc.
	inv := NewRTLInverter(5.0, 10000.0, 1000.0, nil)
	result := inv.Evaluate(0.0)
	if result.Voltage <= 4.0 {
		t.Errorf("Evaluate(0.0).Voltage = %v, want > 4.0", result.Voltage)
	}
	if result.LogicValue != 1 {
		t.Errorf("Evaluate(0.0).LogicValue = %d, want 1", result.LogicValue)
	}
}

func TestRTLInverter_OutputVoltageLow(t *testing.T) {
	// Input HIGH -> output near GND.
	inv := NewRTLInverter(5.0, 10000.0, 1000.0, nil)
	result := inv.Evaluate(5.0)
	if result.Voltage >= 1.0 {
		t.Errorf("Evaluate(5.0).Voltage = %v, want < 1.0", result.Voltage)
	}
	if result.LogicValue != 0 {
		t.Errorf("Evaluate(5.0).LogicValue = %d, want 0", result.LogicValue)
	}
}

func TestRTLInverter_PropagationDelay(t *testing.T) {
	// RTL should be slower than TTL.
	inv := NewRTLInverter(5.0, 10000.0, 1000.0, nil)
	result := inv.Evaluate(5.0)
	if result.PropagationDelay <= 10e-9 {
		t.Errorf("PropagationDelay = %v, want > 10ns", result.PropagationDelay)
	}
}

func TestRTLInverter_RejectsInvalidInput(t *testing.T) {
	inv := NewRTLInverter(5.0, 10000.0, 1000.0, nil)
	if _, err := inv.EvaluateDigital(2); err == nil {
		t.Error("EvaluateDigital(2) should return error")
	}
}

func TestRTLInverter_CustomResistors(t *testing.T) {
	// Custom resistor values should be respected.
	inv := NewRTLInverter(5.0, 5000.0, 2000.0, nil)
	if inv.RBase != 5000.0 {
		t.Errorf("RBase = %v, want 5000", inv.RBase)
	}
	if inv.RCollector != 2000.0 {
		t.Errorf("RCollector = %v, want 2000", inv.RCollector)
	}
}
