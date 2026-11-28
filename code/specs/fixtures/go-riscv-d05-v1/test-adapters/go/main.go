// Command main replays or deliberately recaptures the external Go D05 fixture.
// It imports only public simulator APIs and stays outside the publishable Go
// package so package tests need no filesystem capability.
package main

import (
	"bytes"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"errors"
	"flag"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"reflect"

	"github.com/adhithyan15/coding-adventures/code/packages/go/core"
	cpupipeline "github.com/adhithyan15/coding-adventures/code/packages/go/cpu-pipeline"
	rv "github.com/adhithyan15/coding-adventures/code/packages/go/riscv-simulator"
)

type input struct {
	Schema             string       `json:"schema"`
	ID                 string       `json:"id"`
	RAMBytes           int          `json:"ram_bytes"`
	MaxCycles          int          `json:"max_cycles"`
	ProgramBytesHex    string       `json:"program_bytes_hex"`
	InitialMemoryWords []memoryWord `json:"initial_memory_words"`
}

type memoryWord struct {
	Address int    `json:"address"`
	Value   uint32 `json:"value"`
}

type forward struct {
	Operand     string `json:"operand"`
	Register    int    `json:"register"`
	Value       int    `json:"value"`
	SourceStage string `json:"source_stage"`
}

type stage struct {
	PC                int       `json:"pc"`
	Mnemonic          string    `json:"mnemonic"`
	BranchTaken       bool      `json:"branch_taken,omitempty"`
	ForwardedOperands []forward `json:"forwarded_operands,omitempty"`
}

type cycle struct {
	Cycle    int               `json:"cycle"`
	Stages   map[string]*stage `json:"stages"`
	Stalled  bool              `json:"stalled"`
	Flushing bool              `json:"flushing"`
}

type goD05Stats struct {
	Cycles       int `json:"cycles"`
	Instructions int `json:"instructions"`
	Forwards     int `json:"forwards"`
	Stalls       int `json:"stalls"`
	Flushes      int `json:"flushes"`
}

type output struct {
	Schema      string     `json:"schema"`
	InputSHA256 string     `json:"input_sha256"`
	Trace       []cycle    `json:"trace"`
	RetiredPCs  []int      `json:"retired_pcs"`
	Stats       goD05Stats `json:"stats"`
	Registers   []uint32   `json:"registers"`
	RAMWords    []uint32   `json:"ram_words"`
}

func main() {
	root := flag.String("repo-root", "../../../..", "repository root (default assumes cwd is the Go riscv-simulator package)")
	update := flag.Bool("update", false, "recapture expected_trace.json from the actual Go pipeline")
	flag.Parse()
	if flag.NArg() != 0 {
		fatalf("unexpected arguments: %v", flag.Args())
	}
	if err := replay(*root, *update); err != nil {
		fatalf("%v", err)
	}
}

func replay(root string, update bool) error {
	dir := filepath.Join(root, "code", "specs", "fixtures", "go-riscv-d05-v1")
	inputPath := filepath.Join(dir, "input.json")
	expectedPath := filepath.Join(dir, "expected_trace.json")
	inputBytes, err := os.ReadFile(inputPath)
	if err != nil {
		return fmt.Errorf("read input fixture: %w", err)
	}
	var in input
	if err := decodeStrict(inputBytes, &in); err != nil {
		return fmt.Errorf("decode input fixture: %w", err)
	}
	if in.Schema != "go-riscv-d05-pipeline-input-v1" || in.ID == "" || in.RAMBytes < 4 || in.RAMBytes%4 != 0 || in.MaxCycles <= 0 {
		return fmt.Errorf("invalid fixture metadata: %#v", in)
	}
	program, err := hex.DecodeString(in.ProgramBytesHex)
	if err != nil || len(program) == 0 || len(program)%4 != 0 {
		return fmt.Errorf("program_bytes_hex must contain nonempty aligned bytes: %v", err)
	}
	if len(program) > in.RAMBytes {
		return fmt.Errorf("program of %d bytes exceeds RAM size %d", len(program), in.RAMBytes)
	}
	for _, word := range in.InitialMemoryWords {
		if word.Address < 0 || word.Address%4 != 0 || word.Address > in.RAMBytes-4 {
			return fmt.Errorf("unaligned or out-of-range initial word: %#v", word)
		}
	}

	actual, err := capture(in, inputBytes, program)
	if err != nil {
		return err
	}
	if err := assertSemantics(actual); err != nil {
		return err
	}
	if update {
		encoded, err := json.MarshalIndent(actual, "", "  ")
		if err != nil {
			return fmt.Errorf("marshal captured trace: %w", err)
		}
		encoded = append(encoded, '\n')
		if err := os.WriteFile(expectedPath, encoded, 0o644); err != nil {
			return fmt.Errorf("write captured trace: %w", err)
		}
	}
	expectedBytes, err := os.ReadFile(expectedPath)
	if err != nil {
		return fmt.Errorf("read expected trace: %w (pass -update to capture it)", err)
	}
	var expected output
	if err := decodeStrict(expectedBytes, &expected); err != nil {
		return fmt.Errorf("decode expected trace: %w", err)
	}
	if !reflect.DeepEqual(actual, expected) {
		actualJSON, _ := json.MarshalIndent(actual, "", "  ")
		return fmt.Errorf("captured trace differs; review a recapture with -update\nactual:\n%s", actualJSON)
	}
	fmt.Printf("PASS %s: cycles=%d retired=%d forwards=%d stalls=%d flushes=%d input_sha256=%s\n",
		in.ID, actual.Stats.Cycles, len(actual.RetiredPCs), actual.Stats.Forwards,
		actual.Stats.Stalls, actual.Stats.Flushes, actual.InputSHA256)
	return nil
}

func decodeStrict(data []byte, destination any) error {
	decoder := json.NewDecoder(bytes.NewReader(data))
	decoder.DisallowUnknownFields()
	if err := decoder.Decode(destination); err != nil {
		return err
	}
	var trailing any
	if err := decoder.Decode(&trailing); !errors.Is(err, io.EOF) {
		if err == nil {
			return errors.New("multiple JSON values in fixture")
		}
		return err
	}
	return nil
}

func capture(in input, inputBytes, program []byte) (output, error) {
	config := core.SimpleConfig()
	staged, err := rv.NewRiscVCore(config, in.RAMBytes)
	if err != nil {
		return output{}, fmt.Errorf("construct classic Go core: %w", err)
	}
	staged.LoadProgram(program, 0)
	for _, word := range in.InitialMemoryWords {
		staged.MemoryController().WriteWord(word.Address, int(word.Value))
	}
	sequential := rv.NewRiscVSimulator(in.RAMBytes)
	for _, word := range in.InitialMemoryWords {
		sequential.CPU.Memory.WriteWord(word.Address, word.Value)
	}
	sequentialTrace := sequential.Run(program)

	trace := make([]cycle, 0, in.MaxCycles)
	retired := make([]int, 0, len(sequentialTrace))
	for count := 0; count < in.MaxCycles && !staged.IsHalted(); count++ {
		snapshot := staged.Step()
		entry := cycle{Cycle: snapshot.Cycle, Stages: make(map[string]*stage, 5), Stalled: snapshot.Stalled, Flushing: snapshot.Flushing}
		for _, name := range []string{"IF", "ID", "EX", "MEM", "WB"} {
			token := snapshot.Stages[name]
			if token == nil || token.IsBubble {
				entry.Stages[name] = nil
				continue
			}
			current := &stage{PC: token.PC, Mnemonic: token.Opcode}
			if token.IsBranch {
				current.BranchTaken = token.BranchTaken
			}
			if name == "EX" {
				for _, item := range token.ForwardedOperands {
					operand := "unknown"
					if item.Operand == cpupipeline.SourceOperand1 {
						operand = "rs1"
					} else if item.Operand == cpupipeline.SourceOperand2 {
						operand = "rs2"
					}
					current.ForwardedOperands = append(current.ForwardedOperands, forward{
						Operand: operand, Register: item.Register, Value: item.Value, SourceStage: item.SourceStage,
					})
				}
			}
			entry.Stages[name] = current
		}
		if wb := entry.Stages["WB"]; wb != nil {
			retired = append(retired, wb.PC)
		}
		trace = append(trace, entry)
	}
	if !staged.IsHalted() {
		return output{}, fmt.Errorf("Go core did not halt within %d cycles", in.MaxCycles)
	}
	sequentialPCs := make([]int, 0, len(sequentialTrace))
	for _, step := range sequentialTrace {
		sequentialPCs = append(sequentialPCs, step.Fetch.PC)
	}
	if !reflect.DeepEqual(retired, sequentialPCs) {
		return output{}, fmt.Errorf("retired PCs %v differ from sequential oracle %v", retired, sequentialPCs)
	}
	registers := make([]uint32, 32)
	for index := range registers {
		registers[index] = uint32(staged.ReadRegister(index))
		if registers[index] != sequential.CPU.Registers.Read(index) {
			return output{}, fmt.Errorf("x%d staged=%#x sequential=%#x", index, registers[index], sequential.CPU.Registers.Read(index))
		}
	}
	ramWords := make([]uint32, in.RAMBytes/4)
	for index := range ramWords {
		address := index * 4
		ramWords[index] = uint32(staged.MemoryController().ReadWord(address))
		if got := sequential.CPU.Memory.ReadWord(address); ramWords[index] != got {
			return output{}, fmt.Errorf("memory[%d] staged=%#x sequential=%#x", address, ramWords[index], got)
		}
	}
	coreStats := staged.Stats()
	inputHash := sha256.Sum256(inputBytes)
	return output{
		Schema: "go-riscv-d05-pipeline-trace-v1", InputSHA256: hex.EncodeToString(inputHash[:]),
		Trace: trace, RetiredPCs: retired,
		Stats: goD05Stats{Cycles: coreStats.TotalCycles, Instructions: coreStats.InstructionsCompleted,
			Forwards: coreStats.ForwardCount, Stalls: coreStats.StallCount, Flushes: coreStats.FlushCount},
		Registers: registers, RAMWords: ramWords,
	}, nil
}

func assertSemantics(result output) error {
	if result.Stats.Stalls != 1 || result.Stats.Flushes != 1 {
		return fmt.Errorf("expected exactly one stall and flush, got %#v", result.Stats)
	}
	if result.Stats.Instructions != len(result.RetiredPCs) {
		return fmt.Errorf("retirement count %d differs from retired PC count %d", result.Stats.Instructions, len(result.RetiredPCs))
	}
	if contains(result.RetiredPCs, 28) {
		return fmt.Errorf("wrong-path instruction PC 28 retired: %v", result.RetiredPCs)
	}
	var sawStorePair, sawLoadBranch, sawTakenBranch, sawIDStall, sawRedirectFetch bool
	for _, item := range result.Trace {
		if item.Flushing {
			if fetch := item.Stages["IF"]; fetch != nil && fetch.PC == 32 {
				sawRedirectFetch = true
			}
		}
		if item.Stalled && item.Stages["ID"] != nil && item.Stages["ID"].PC == 24 {
			sawIDStall = true
		}
		if ex := item.Stages["EX"]; ex != nil {
			if ex.PC == 12 {
				gotRS1, gotRS2 := false, false
				for _, f := range ex.ForwardedOperands {
					gotRS1 = gotRS1 || (f.Operand == "rs1" && f.Register == 2 && f.Value == 128 && f.SourceStage == "EX")
					gotRS2 = gotRS2 || (f.Operand == "rs2" && f.Register == 1 && f.Value == 0 && f.SourceStage == "MEM")
				}
				sawStorePair = gotRS1 && gotRS2
			}
			if ex.PC == 24 && ex.BranchTaken {
				sawTakenBranch = true
				for _, f := range ex.ForwardedOperands {
					if f.Operand == "rs1" && f.Register == 3 && f.Value == 0 && f.SourceStage == "MEM" {
						sawLoadBranch = true
					}
				}
			}
		}
	}
	if !sawStorePair || !sawLoadBranch || !sawTakenBranch || !sawIDStall || !sawRedirectFetch {
		return fmt.Errorf("missing bypass/stall/branch evidence: store=%v load-branch=%v taken=%v id-stall=%v redirect-fetch=%v", sawStorePair, sawLoadBranch, sawTakenBranch, sawIDStall, sawRedirectFetch)
	}
	if result.Registers[0] != 0 || result.Registers[5] != 0 || result.Registers[4] != 1 || result.RAMWords[128/4] != 0 {
		return fmt.Errorf("unexpected final x0/x4/x5/memory values: x0=%d x4=%d x5=%d mem[128]=%d", result.Registers[0], result.Registers[4], result.Registers[5], result.RAMWords[128/4])
	}
	return nil
}

func contains(values []int, value int) bool {
	for _, candidate := range values {
		if candidate == value {
			return true
		}
	}
	return false
}

func fatalf(format string, values ...any) {
	fmt.Fprintf(os.Stderr, format+"\n", values...)
	os.Exit(1)
}
