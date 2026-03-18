package cpusimulator

import (
	"testing"
	"strings"
)

func TestFormatPipeline(t *testing.T) {
	trace := PipelineTrace{
		Cycle: 0,
		Fetch: FetchResult{PC: 4, RawInstruction: 0x93},
		Decode: DecodeResult{
			Mnemonic: "addi",
			Fields: map[string]int{"rd": 1},
		},
		Execute: ExecuteResult{
			Description: "x1 = 1",
			NextPC: 8,
		},
	}
	formatted := trace.FormatPipeline()
	if !strings.Contains(formatted, "--- Cycle 0 ---") {
		t.Errorf("Format missing cycle")
	}
	if !strings.Contains(formatted, "addi") {
		t.Errorf("Format missing decode mnemonic")
	}
}
