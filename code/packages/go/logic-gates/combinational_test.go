package logicgates

import (
	"testing"
)

// =========================================================================
// Mux2 Tests
// =========================================================================

func TestMux2(t *testing.T) {
	// When sel=0, output should be d0
	// When sel=1, output should be d1
	tests := []struct {
		name     string
		d0, d1   int
		sel      int
		expected int
	}{
		{"sel=0, d0=0, d1=0", 0, 0, 0, 0},
		{"sel=0, d0=0, d1=1", 0, 1, 0, 0},
		{"sel=0, d0=1, d1=0", 1, 0, 0, 1},
		{"sel=0, d0=1, d1=1", 1, 1, 0, 1},
		{"sel=1, d0=0, d1=0", 0, 0, 1, 0},
		{"sel=1, d0=0, d1=1", 0, 1, 1, 1},
		{"sel=1, d0=1, d1=0", 1, 0, 1, 0},
		{"sel=1, d0=1, d1=1", 1, 1, 1, 1},
	}
	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			got := Mux2(tc.d0, tc.d1, tc.sel)
			if got != tc.expected {
				t.Errorf("Mux2(%d, %d, %d) = %d, want %d", tc.d0, tc.d1, tc.sel, got, tc.expected)
			}
		})
	}
}

func TestMux2_InvalidInput(t *testing.T) {
	assertPanics(t, "Mux2(2,0,0)", func() { Mux2(2, 0, 0) })
	assertPanics(t, "Mux2(0,2,0)", func() { Mux2(0, 2, 0) })
	assertPanics(t, "Mux2(0,0,2)", func() { Mux2(0, 0, 2) })
}

// =========================================================================
// Mux4 Tests
// =========================================================================

func TestMux4(t *testing.T) {
	tests := []struct {
		name               string
		d0, d1, d2, d3     int
		sel                []int
		expected           int
	}{
		{"sel=00 → d0=1", 1, 0, 0, 0, []int{0, 0}, 1},
		{"sel=01 → d1=1", 0, 1, 0, 0, []int{1, 0}, 1},
		{"sel=10 → d2=1", 0, 0, 1, 0, []int{0, 1}, 1},
		{"sel=11 → d3=1", 0, 0, 0, 1, []int{1, 1}, 1},
		{"all zero", 0, 0, 0, 0, []int{0, 0}, 0},
		{"all one sel=10", 1, 1, 1, 1, []int{0, 1}, 1},
	}
	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			got := Mux4(tc.d0, tc.d1, tc.d2, tc.d3, tc.sel)
			if got != tc.expected {
				t.Errorf("Mux4(%d, %d, %d, %d, %v) = %d, want %d",
					tc.d0, tc.d1, tc.d2, tc.d3, tc.sel, got, tc.expected)
			}
		})
	}
}

func TestMux4_InvalidSel(t *testing.T) {
	assertPanics(t, "Mux4 sel len=1", func() { Mux4(0, 0, 0, 0, []int{0}) })
	assertPanics(t, "Mux4 sel len=3", func() { Mux4(0, 0, 0, 0, []int{0, 0, 0}) })
}

// =========================================================================
// MuxN Tests
// =========================================================================

func TestMuxN_2Inputs(t *testing.T) {
	// 2:1 MUX (base case)
	if got := MuxN([]int{0, 1}, []int{0}); got != 0 {
		t.Errorf("MuxN 2:1 sel=0 = %d, want 0", got)
	}
	if got := MuxN([]int{0, 1}, []int{1}); got != 1 {
		t.Errorf("MuxN 2:1 sel=1 = %d, want 1", got)
	}
}

func TestMuxN_4Inputs(t *testing.T) {
	inputs := []int{0, 0, 0, 0}
	// Select each input
	for i := 0; i < 4; i++ {
		data := make([]int, 4)
		data[i] = 1
		sel := []int{i & 1, (i >> 1) & 1}
		got := MuxN(data, sel)
		if got != 1 {
			t.Errorf("MuxN 4:1 sel=%v data[%d]=1 = %d, want 1", sel, i, got)
		}
	}
	// All zeros
	got := MuxN(inputs, []int{1, 1})
	if got != 0 {
		t.Errorf("MuxN 4:1 all zeros = %d, want 0", got)
	}
}

func TestMuxN_8Inputs(t *testing.T) {
	// Select input 5 (binary 101)
	data := make([]int, 8)
	data[5] = 1
	got := MuxN(data, []int{1, 0, 1})
	if got != 1 {
		t.Errorf("MuxN 8:1 sel=101 = %d, want 1", got)
	}
}

func TestMuxN_16Inputs(t *testing.T) {
	// Select input 10 (binary 1010)
	data := make([]int, 16)
	data[10] = 1
	got := MuxN(data, []int{0, 1, 0, 1})
	if got != 1 {
		t.Errorf("MuxN 16:1 sel=1010 = %d, want 1", got)
	}
}

func TestMuxN_InvalidInputs(t *testing.T) {
	assertPanics(t, "MuxN 1 input", func() { MuxN([]int{0}, []int{}) })
	assertPanics(t, "MuxN 3 inputs", func() { MuxN([]int{0, 0, 0}, []int{0}) })
	assertPanics(t, "MuxN wrong sel count", func() { MuxN([]int{0, 0, 0, 0}, []int{0}) })
}

// =========================================================================
// Demux Tests
// =========================================================================

func TestDemux_4Outputs(t *testing.T) {
	tests := []struct {
		name     string
		data     int
		sel      []int
		expected []int
	}{
		{"data=1 sel=00 → output 0", 1, []int{0, 0}, []int{1, 0, 0, 0}},
		{"data=1 sel=01 → output 1", 1, []int{1, 0}, []int{0, 1, 0, 0}},
		{"data=1 sel=10 → output 2", 1, []int{0, 1}, []int{0, 0, 1, 0}},
		{"data=1 sel=11 → output 3", 1, []int{1, 1}, []int{0, 0, 0, 1}},
		{"data=0 sel=00 → all zero", 0, []int{0, 0}, []int{0, 0, 0, 0}},
		{"data=0 sel=11 → all zero", 0, []int{1, 1}, []int{0, 0, 0, 0}},
	}
	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			got := Demux(tc.data, tc.sel, 4)
			for i := range got {
				if got[i] != tc.expected[i] {
					t.Errorf("Demux(%d, %v, 4)[%d] = %d, want %d",
						tc.data, tc.sel, i, got[i], tc.expected[i])
				}
			}
		})
	}
}

func TestDemux_2Outputs(t *testing.T) {
	got := Demux(1, []int{0}, 2)
	if got[0] != 1 || got[1] != 0 {
		t.Errorf("Demux(1, [0], 2) = %v, want [1, 0]", got)
	}
	got = Demux(1, []int{1}, 2)
	if got[0] != 0 || got[1] != 1 {
		t.Errorf("Demux(1, [1], 2) = %v, want [0, 1]", got)
	}
}

func TestDemux_Invalid(t *testing.T) {
	assertPanics(t, "Demux nOutputs=3", func() { Demux(1, []int{0}, 3) })
	assertPanics(t, "Demux nOutputs=1", func() { Demux(1, []int{}, 1) })
	assertPanics(t, "Demux wrong sel", func() { Demux(1, []int{0, 0}, 2) })
}

// =========================================================================
// Decoder Tests
// =========================================================================

func TestDecoder_1Bit(t *testing.T) {
	// 1-to-2 decoder
	got := Decoder([]int{0})
	if got[0] != 1 || got[1] != 0 {
		t.Errorf("Decoder([0]) = %v, want [1, 0]", got)
	}
	got = Decoder([]int{1})
	if got[0] != 0 || got[1] != 1 {
		t.Errorf("Decoder([1]) = %v, want [0, 1]", got)
	}
}

func TestDecoder_2Bit(t *testing.T) {
	// 2-to-4 decoder: full truth table
	tests := []struct {
		input    []int
		expected []int
	}{
		{[]int{0, 0}, []int{1, 0, 0, 0}},
		{[]int{1, 0}, []int{0, 1, 0, 0}},
		{[]int{0, 1}, []int{0, 0, 1, 0}},
		{[]int{1, 1}, []int{0, 0, 0, 1}},
	}
	for _, tc := range tests {
		got := Decoder(tc.input)
		for i := range got {
			if got[i] != tc.expected[i] {
				t.Errorf("Decoder(%v)[%d] = %d, want %d", tc.input, i, got[i], tc.expected[i])
			}
		}
	}
}

func TestDecoder_3Bit(t *testing.T) {
	// 3-to-8 decoder: input 5 (binary 101) → output[5] = 1
	got := Decoder([]int{1, 0, 1})
	for i := 0; i < 8; i++ {
		expected := 0
		if i == 5 {
			expected = 1
		}
		if got[i] != expected {
			t.Errorf("Decoder([1,0,1])[%d] = %d, want %d", i, got[i], expected)
		}
	}
}

func TestDecoder_Invalid(t *testing.T) {
	assertPanics(t, "Decoder empty", func() { Decoder([]int{}) })
	assertPanics(t, "Decoder bad bit", func() { Decoder([]int{2}) })
}

// =========================================================================
// Encoder Tests
// =========================================================================

func TestEncoder_4to2(t *testing.T) {
	tests := []struct {
		input    []int
		expected []int
	}{
		{[]int{1, 0, 0, 0}, []int{0, 0}}, // index 0
		{[]int{0, 1, 0, 0}, []int{1, 0}}, // index 1
		{[]int{0, 0, 1, 0}, []int{0, 1}}, // index 2
		{[]int{0, 0, 0, 1}, []int{1, 1}}, // index 3
	}
	for _, tc := range tests {
		got := Encoder(tc.input)
		for i := range got {
			if got[i] != tc.expected[i] {
				t.Errorf("Encoder(%v)[%d] = %d, want %d", tc.input, i, got[i], tc.expected[i])
			}
		}
	}
}

func TestEncoder_8to3(t *testing.T) {
	// Input 5 active → binary 101 → [1, 0, 1]
	input := []int{0, 0, 0, 0, 0, 1, 0, 0}
	got := Encoder(input)
	expected := []int{1, 0, 1}
	for i := range got {
		if got[i] != expected[i] {
			t.Errorf("Encoder(bit5)[%d] = %d, want %d", i, got[i], expected[i])
		}
	}
}

func TestEncoder_Invalid(t *testing.T) {
	assertPanics(t, "Encoder 3 inputs", func() { Encoder([]int{1, 0, 0}) })
	assertPanics(t, "Encoder no active", func() { Encoder([]int{0, 0, 0, 0}) })
	assertPanics(t, "Encoder two active", func() { Encoder([]int{1, 1, 0, 0}) })
}

// =========================================================================
// PriorityEncoder Tests
// =========================================================================

func TestPriorityEncoder_4Input(t *testing.T) {
	tests := []struct {
		name        string
		input       []int
		expectedOut []int
		expectedV   int
	}{
		{"no active", []int{0, 0, 0, 0}, []int{0, 0}, 0},
		{"I0 only", []int{1, 0, 0, 0}, []int{0, 0}, 1},
		{"I1 only", []int{0, 1, 0, 0}, []int{1, 0}, 1},
		{"I2 only", []int{0, 0, 1, 0}, []int{0, 1}, 1},
		{"I3 only", []int{0, 0, 0, 1}, []int{1, 1}, 1},
		{"I0 and I2", []int{1, 0, 1, 0}, []int{0, 1}, 1},   // I2 wins
		{"I1 and I3", []int{0, 1, 0, 1}, []int{1, 1}, 1},   // I3 wins
		{"all active", []int{1, 1, 1, 1}, []int{1, 1}, 1},   // I3 wins
	}
	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			out, valid := PriorityEncoder(tc.input)
			if valid != tc.expectedV {
				t.Errorf("PriorityEncoder(%v) valid = %d, want %d", tc.input, valid, tc.expectedV)
			}
			for i := range out {
				if out[i] != tc.expectedOut[i] {
					t.Errorf("PriorityEncoder(%v)[%d] = %d, want %d",
						tc.input, i, out[i], tc.expectedOut[i])
				}
			}
		})
	}
}

func TestPriorityEncoder_Invalid(t *testing.T) {
	assertPanics(t, "PriorityEncoder 3 inputs", func() { PriorityEncoder([]int{0, 0, 0}) })
	assertPanics(t, "PriorityEncoder bad bit", func() { PriorityEncoder([]int{0, 2, 0, 0}) })
}

// =========================================================================
// TriState Tests
// =========================================================================

func TestTriState_Enabled(t *testing.T) {
	// When enabled, output should be a pointer to the data value
	result := TriState(1, 1)
	if result == nil || *result != 1 {
		t.Errorf("TriState(1, 1) = %v, want &1", result)
	}
	result = TriState(0, 1)
	if result == nil || *result != 0 {
		t.Errorf("TriState(0, 1) = %v, want &0", result)
	}
}

func TestTriState_Disabled(t *testing.T) {
	// When disabled, output should be nil (high-Z)
	result := TriState(0, 0)
	if result != nil {
		t.Errorf("TriState(0, 0) = %v, want nil", result)
	}
	result = TriState(1, 0)
	if result != nil {
		t.Errorf("TriState(1, 0) = %v, want nil", result)
	}
}

func TestTriState_Invalid(t *testing.T) {
	assertPanics(t, "TriState(2,0)", func() { TriState(2, 0) })
	assertPanics(t, "TriState(0,2)", func() { TriState(0, 2) })
}
