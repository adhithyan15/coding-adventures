package nibcompiler

import (
	"testing"

	intel4004packager "github.com/adhithyan15/coding-adventures/code/packages/go/intel-4004-packager"
	intel4004simulator "github.com/adhithyan15/coding-adventures/code/packages/go/intel4004-simulator"
)

func TestCompileSourceReturnsArtifacts(t *testing.T) {
	result, err := CompileSource("fn main() { let x: u4 = 5; }")
	if err != nil {
		t.Fatalf("compile failed: %v", err)
	}
	if result.RawIR == nil || len(result.Binary) == 0 || result.Assembly == "" {
		t.Fatalf("unexpected result: %#v", result)
	}
}

func TestCompiledProgramRunsInSimulator(t *testing.T) {
	result, err := CompileSource("fn main() { let x: u4 = 5; }")
	if err != nil {
		t.Fatalf("compile failed: %v", err)
	}
	decoded, err := intel4004packager.DecodeHex(result.HexText)
	if err != nil {
		t.Fatalf("decode failed: %v", err)
	}
	sim := intel4004simulator.NewIntel4004Simulator(4096)
	traces := sim.Run(decoded.Binary, 100)
	if len(traces) == 0 || !sim.Halted {
		t.Fatalf("expected simulator to halt, got %d traces", len(traces))
	}
	if sim.Registers[2] != 5 {
		t.Fatalf("expected R2 = 5, got %d", sim.Registers[2])
	}
}
