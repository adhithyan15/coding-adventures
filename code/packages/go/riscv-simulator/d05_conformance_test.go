package riscvsimulator

import (
	"bytes"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"os"
	"path/filepath"
	"reflect"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/packages/go/core"
	cpupipeline "github.com/adhithyan15/coding-adventures/code/packages/go/cpu-pipeline"
)

const goD05FixtureDir = "../../../specs/fixtures/go-riscv-d05-v1"

type goD05Input struct {
	Schema             string            `json:"schema"`
	ID                 string            `json:"id"`
	RAMBytes           int               `json:"ram_bytes"`
	MaxCycles          int               `json:"max_cycles"`
	ProgramBytesHex    string            `json:"program_bytes_hex"`
	InitialMemoryWords []goD05MemoryWord `json:"initial_memory_words"`
}

type goD05MemoryWord struct {
	Address int    `json:"address"`
	Value   uint32 `json:"value"`
}

type goD05Forward struct {
	Operand     string `json:"operand"`
	Register    int    `json:"register"`
	Value       int    `json:"value"`
	SourceStage string `json:"source_stage"`
}

type goD05Stage struct {
	PC                int            `json:"pc"`
	Mnemonic          string         `json:"mnemonic"`
	BranchTaken       bool           `json:"branch_taken,omitempty"`
	ForwardedOperands []goD05Forward `json:"forwarded_operands,omitempty"`
}

type goD05Cycle struct {
	Cycle    int                    `json:"cycle"`
	Stages   map[string]*goD05Stage `json:"stages"`
	Stalled  bool                   `json:"stalled"`
	Flushing bool                   `json:"flushing"`
}

type goD05Stats struct {
	Cycles       int `json:"cycles"`
	Instructions int `json:"instructions"`
	Forwards     int `json:"forwards"`
	Stalls       int `json:"stalls"`
	Flushes      int `json:"flushes"`
}

type goD05Output struct {
	Schema      string       `json:"schema"`
	InputSHA256 string       `json:"input_sha256"`
	Trace       []goD05Cycle `json:"trace"`
	RetiredPCs  []int        `json:"retired_pcs"`
	Stats       goD05Stats   `json:"stats"`
	Registers   []uint32     `json:"registers"`
	RAMWords    []uint32     `json:"ram_words"`
}

func TestGoD05FixtureMatchesCapturedFiveStageTraceAndSequentialOracle(t *testing.T) {
	inputPath := filepath.Join(goD05FixtureDir, "input.json")
	inputBytes, err := os.ReadFile(inputPath)
	if err != nil {
		t.Fatalf("read D05 input fixture: %v", err)
	}
	var input goD05Input
	decoder := json.NewDecoder(bytes.NewReader(inputBytes))
	decoder.DisallowUnknownFields()
	if err := decoder.Decode(&input); err != nil {
		t.Fatalf("decode D05 input fixture: %v", err)
	}
	if input.Schema != "go-riscv-d05-pipeline-input-v1" || input.ID == "" || input.RAMBytes < 4 || input.RAMBytes%4 != 0 || input.MaxCycles <= 0 {
		t.Fatalf("invalid D05 input metadata: %#v", input)
	}
	program, err := hex.DecodeString(input.ProgramBytesHex)
	if err != nil || len(program) == 0 || len(program)%4 != 0 {
		t.Fatalf("program_bytes_hex must contain nonempty aligned bytes: %v", err)
	}
	if len(program) > input.RAMBytes {
		t.Fatalf("program of %d bytes exceeds RAM size %d", len(program), input.RAMBytes)
	}
	for _, word := range input.InitialMemoryWords {
		if word.Address < 0 || word.Address%4 != 0 || word.Address > input.RAMBytes-4 {
			t.Fatalf("unaligned or out-of-range initial word: %#v", word)
		}
	}

	actual := captureGoD05Output(t, input, inputBytes, program)
	expectedPath := filepath.Join(goD05FixtureDir, "expected_trace.json")
	if os.Getenv("UPDATE_GO_D05_TRACE") == "1" {
		encoded, err := json.MarshalIndent(actual, "", "  ")
		if err != nil {
			t.Fatalf("marshal captured trace: %v", err)
		}
		encoded = append(encoded, '\n')
		if err := os.WriteFile(expectedPath, encoded, 0o644); err != nil {
			t.Fatalf("write captured trace: %v", err)
		}
	}
	expectedBytes, err := os.ReadFile(expectedPath)
	if err != nil {
		t.Fatalf("read expected trace: %v (set UPDATE_GO_D05_TRACE=1 to recapture from the real Go core)", err)
	}
	var expected goD05Output
	expectedDecoder := json.NewDecoder(bytes.NewReader(expectedBytes))
	expectedDecoder.DisallowUnknownFields()
	if err := expectedDecoder.Decode(&expected); err != nil {
		t.Fatalf("decode expected trace: %v", err)
	}
	if !reflect.DeepEqual(actual, expected) {
		actualJSON, _ := json.MarshalIndent(actual, "", "  ")
		t.Fatalf("captured Go trace changed; run UPDATE_GO_D05_TRACE=1 go test ./... -run '^TestGoD05FixtureMatchesCapturedFiveStageTraceAndSequentialOracle$' -count=1 to review a recapture\nactual:\n%s", actualJSON)
	}
	assertGoD05Semantics(t, actual)
}

func captureGoD05Output(t *testing.T, input goD05Input, inputBytes, program []byte) goD05Output {
	t.Helper()
	config := core.SimpleConfig()
	staged, err := NewRiscVCore(config, input.RAMBytes)
	if err != nil {
		t.Fatalf("construct classic Go core: %v", err)
	}
	staged.LoadProgram(program, 0)
	for _, word := range input.InitialMemoryWords {
		staged.MemoryController().WriteWord(word.Address, int(word.Value))
	}

	sequential := NewRiscVSimulator(input.RAMBytes)
	for _, word := range input.InitialMemoryWords {
		sequential.CPU.Memory.WriteWord(word.Address, word.Value)
	}
	sequentialTrace := sequential.Run(program)

	trace := make([]goD05Cycle, 0, input.MaxCycles)
	retired := make([]int, 0, len(sequentialTrace))
	for cycle := 0; cycle < input.MaxCycles && !staged.IsHalted(); cycle++ {
		snapshot := staged.Step()
		entry := goD05Cycle{
			Cycle: snapshot.Cycle, Stages: make(map[string]*goD05Stage, 5),
			Stalled: snapshot.Stalled, Flushing: snapshot.Flushing,
		}
		for _, name := range []string{"IF", "ID", "EX", "MEM", "WB"} {
			token := snapshot.Stages[name]
			if token == nil || token.IsBubble {
				entry.Stages[name] = nil
				continue
			}
			stage := &goD05Stage{PC: token.PC, Mnemonic: token.Opcode}
			if token.IsBranch {
				stage.BranchTaken = token.BranchTaken
			}
			if name == "EX" {
				for _, forwarded := range token.ForwardedOperands {
					operand := "unknown"
					if forwarded.Operand == cpupipeline.SourceOperand1 {
						operand = "rs1"
					} else if forwarded.Operand == cpupipeline.SourceOperand2 {
						operand = "rs2"
					}
					stage.ForwardedOperands = append(stage.ForwardedOperands, goD05Forward{
						Operand: operand, Register: forwarded.Register,
						Value: forwarded.Value, SourceStage: forwarded.SourceStage,
					})
				}
			}
			entry.Stages[name] = stage
		}
		if wb := entry.Stages["WB"]; wb != nil {
			retired = append(retired, wb.PC)
		}
		trace = append(trace, entry)
	}
	if !staged.IsHalted() {
		t.Fatalf("Go core did not halt within %d cycles", input.MaxCycles)
	}

	sequentialPCs := make([]int, 0, len(sequentialTrace))
	for _, step := range sequentialTrace {
		sequentialPCs = append(sequentialPCs, step.Fetch.PC)
	}
	if !reflect.DeepEqual(retired, sequentialPCs) {
		t.Fatalf("retired PCs %v differ from sequential oracle %v", retired, sequentialPCs)
	}

	registers := make([]uint32, 32)
	for index := range registers {
		registers[index] = uint32(staged.ReadRegister(index))
		if registers[index] != sequential.CPU.Registers.Read(index) {
			t.Fatalf("x%d staged=%#x sequential=%#x", index, registers[index], sequential.CPU.Registers.Read(index))
		}
	}
	ramWords := make([]uint32, input.RAMBytes/4)
	for index := range ramWords {
		address := index * 4
		ramWords[index] = uint32(staged.MemoryController().ReadWord(address))
		if got := sequential.CPU.Memory.ReadWord(address); ramWords[index] != got {
			t.Fatalf("memory[%d] staged=%#x sequential=%#x", address, ramWords[index], got)
		}
	}
	stats := staged.Stats()
	inputHash := sha256.Sum256(inputBytes)
	return goD05Output{
		Schema: "go-riscv-d05-pipeline-trace-v1", InputSHA256: hex.EncodeToString(inputHash[:]),
		Trace: trace, RetiredPCs: retired,
		Stats: goD05Stats{
			Cycles: stats.TotalCycles, Instructions: stats.InstructionsCompleted,
			Forwards: stats.ForwardCount, Stalls: stats.StallCount, Flushes: stats.FlushCount,
		},
		Registers: registers, RAMWords: ramWords,
	}
}

func assertGoD05Semantics(t *testing.T, output goD05Output) {
	t.Helper()
	if output.Stats.Stalls != 1 || output.Stats.Flushes != 1 {
		t.Fatalf("expected exactly one stall and flush, got %#v", output.Stats)
	}
	if output.Stats.Instructions != len(output.RetiredPCs) {
		t.Fatalf("retirement count %d differs from retired PC count %d", output.Stats.Instructions, len(output.RetiredPCs))
	}
	if containsInt(output.RetiredPCs, 28) {
		t.Fatalf("wrong-path instruction PC 28 retired: %v", output.RetiredPCs)
	}
	var sawStorePair, sawLoadBranch, sawTakenBranch, sawIDStall, sawRedirectFetch bool
	for _, cycle := range output.Trace {
		if cycle.Flushing {
			if ifStage := cycle.Stages["IF"]; ifStage != nil && ifStage.PC == 32 {
				sawRedirectFetch = true
			}
		}
		if cycle.Stalled && cycle.Stages["ID"] != nil && cycle.Stages["ID"].PC == 24 {
			sawIDStall = true
		}
		if ex := cycle.Stages["EX"]; ex != nil {
			if ex.PC == 12 {
				gotRS1, gotRS2 := false, false
				for _, forward := range ex.ForwardedOperands {
					if forward.Operand == "rs1" && forward.Register == 2 && forward.Value == 128 && forward.SourceStage == "EX" {
						gotRS1 = true
					}
					if forward.Operand == "rs2" && forward.Register == 1 && forward.Value == 0 && forward.SourceStage == "MEM" {
						gotRS2 = true
					}
				}
				sawStorePair = gotRS1 && gotRS2
			}
			if ex.PC == 24 && ex.BranchTaken {
				sawTakenBranch = true
				for _, forward := range ex.ForwardedOperands {
					if forward.Operand == "rs1" && forward.Register == 3 && forward.Value == 0 && forward.SourceStage == "MEM" {
						sawLoadBranch = true
					}
				}
			}
		}
	}
	if !sawStorePair || !sawLoadBranch || !sawTakenBranch || !sawIDStall || !sawRedirectFetch {
		t.Fatalf("missing expected bypass/stall/branch evidence: store=%v load-branch=%v taken=%v id-stall=%v redirect-fetch=%v", sawStorePair, sawLoadBranch, sawTakenBranch, sawIDStall, sawRedirectFetch)
	}
	if output.Registers[0] != 0 || output.Registers[5] != 0 || output.Registers[4] != 1 || output.RAMWords[128/4] != 0 {
		t.Fatalf("unexpected final x0/x4/x5/memory values: x0=%d x4=%d x5=%d mem[128]=%d", output.Registers[0], output.Registers[4], output.Registers[5], output.RAMWords[128/4])
	}
}

func containsInt(values []int, value int) bool {
	for _, candidate := range values {
		if candidate == value {
			return true
		}
	}
	return false
}
