package fpga

import (
	"testing"
)

// =========================================================================
// FPGA Fabric Tests
// =========================================================================

func makeBitstream() *Bitstream {
	andTT := make([]int, 16)
	andTT[3] = 1
	zeroTT := make([]int, 16)

	return FromMap(
		map[string]CLBConfig{
			"clb_0": {
				Slice0: SliceConfig{LutA: andTT, LutB: zeroTT},
				Slice1: SliceConfig{LutA: zeroTT, LutB: zeroTT},
			},
		},
		map[string][]RouteConfig{
			"sw_0": {
				{Source: "clb_out", Destination: "east"},
				{Source: "north", Destination: "south"},
			},
		},
		map[string]IOConfig{
			"in_a":  {Mode: "input"},
			"in_b":  {Mode: "input"},
			"out":   {Mode: "output"},
			"tri":   {Mode: "tristate"},
		},
		4,
	)
}

func TestFPGA_EvaluateCLB(t *testing.T) {
	fpga := NewFPGA(makeBitstream())
	zeros := make([]int, 4)

	out := fpga.EvaluateCLB("clb_0",
		[]int{1, 1, 0, 0}, zeros, // slice 0
		zeros, zeros, // slice 1
		0, 0,
	)

	if out.Slice0.OutputA != 1 {
		t.Errorf("CLB slice0.OutputA = %d, want 1 (AND(1,1))", out.Slice0.OutputA)
	}
}

func TestFPGA_Route(t *testing.T) {
	fpga := NewFPGA(makeBitstream())

	outputs := fpga.Route("sw_0", map[string]int{"clb_out": 1, "north": 0})
	if outputs["east"] != 1 {
		t.Errorf("Route east = %d, want 1", outputs["east"])
	}
	if outputs["south"] != 0 {
		t.Errorf("Route south = %d, want 0", outputs["south"])
	}
}

func TestFPGA_IO(t *testing.T) {
	fpga := NewFPGA(makeBitstream())

	// Set input
	fpga.SetInput("in_a", 1)

	// Read input pad
	pad := fpga.ReadOutput("in_a")
	if pad == nil || *pad != 1 {
		t.Errorf("ReadOutput(in_a) = %v, want &1", pad)
	}

	// Drive output
	fpga.DriveOutput("out", 1)
	pad = fpga.ReadOutput("out")
	if pad == nil || *pad != 1 {
		t.Errorf("ReadOutput(out) = %v, want &1", pad)
	}

	// Tristate
	pad = fpga.ReadOutput("tri")
	if pad != nil {
		t.Errorf("ReadOutput(tri) = %v, want nil", pad)
	}
}

func TestFPGA_Properties(t *testing.T) {
	fpga := NewFPGA(makeBitstream())

	clbs := fpga.CLBs()
	if len(clbs) != 1 {
		t.Errorf("CLBs count = %d, want 1", len(clbs))
	}

	switches := fpga.Switches()
	if len(switches) != 1 {
		t.Errorf("Switches count = %d, want 1", len(switches))
	}

	ios := fpga.IOs()
	if len(ios) != 4 {
		t.Errorf("IOs count = %d, want 4", len(ios))
	}

	bs := fpga.GetBitstream()
	if bs == nil {
		t.Error("GetBitstream() is nil")
	}
}

func TestFPGA_CLBNotFound(t *testing.T) {
	fpga := NewFPGA(makeBitstream())
	zeros := make([]int, 4)

	assertPanics(t, "CLB not found", func() {
		fpga.EvaluateCLB("nonexistent", zeros, zeros, zeros, zeros, 0, 0)
	})
}

func TestFPGA_SwitchNotFound(t *testing.T) {
	fpga := NewFPGA(makeBitstream())
	assertPanics(t, "Switch not found", func() {
		fpga.Route("nonexistent", map[string]int{})
	})
}

func TestFPGA_IONotFound(t *testing.T) {
	fpga := NewFPGA(makeBitstream())
	assertPanics(t, "SetInput not found", func() { fpga.SetInput("x", 0) })
	assertPanics(t, "ReadOutput not found", func() { fpga.ReadOutput("x") })
	assertPanics(t, "DriveOutput not found", func() { fpga.DriveOutput("x", 0) })
}

func TestFPGA_FromJSON(t *testing.T) {
	jsonData := []byte(`{
		"lut_k": 4,
		"clbs": {
			"clb_0": {
				"slice0": {
					"lut_a": [0,0,0,1,0,0,0,0,0,0,0,0,0,0,0,0],
					"lut_b": [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0]
				}
			}
		},
		"io": {
			"in": {"mode": "input"},
			"out": {"mode": "output"}
		}
	}`)

	bs, err := FromJSONBytes(jsonData)
	if err != nil {
		t.Fatalf("FromJSONBytes error: %v", err)
	}

	fpga := NewFPGA(bs)
	zeros := make([]int, 4)

	out := fpga.EvaluateCLB("clb_0",
		[]int{1, 1, 0, 0}, zeros,
		zeros, zeros,
		0, 0,
	)

	if out.Slice0.OutputA != 1 {
		t.Errorf("JSON CLB AND gate = %d, want 1", out.Slice0.OutputA)
	}
}

func TestFPGA_EmptyBitstream(t *testing.T) {
	bs := FromMap(nil, nil, nil, 4)
	fpga := NewFPGA(bs)

	if len(fpga.CLBs()) != 0 {
		t.Errorf("Empty bitstream CLBs count = %d, want 0", len(fpga.CLBs()))
	}
}
