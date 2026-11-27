// Command main validates and records the external Go D-MEM fixture.
// It stays outside all publishable packages so simulator tests need no
// repository-input filesystem capability.
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
	"strings"

	"github.com/adhithyan15/coding-adventures/code/packages/go/core"
	cpupipeline "github.com/adhithyan15/coding-adventures/code/packages/go/cpu-pipeline"
	rv "github.com/adhithyan15/coding-adventures/code/packages/go/riscv-simulator"
)

const (
	inputSchema    = "go-riscv-dmem-ready-input-v1"
	oracleSchema   = "go-riscv-dmem-ready-oracle-v1"
	traceSchema    = "go-riscv-dmem-ready-trace-v1"
	inputSHA256    = "800486ce328d06263a6e8626964c45ec2b50aff1edbdb48c6a09b401c1213356"
	oracleSHA256   = "3b49a95bc30a02a95cebd64f811419229c6e69c23f314d751a1298d69751d700"
	d05InputSHA256 = "fcf69bf2c614f0119c10c2b5d8a1b742c2bf9c1e7d847650ee6c09e736930744"
	d05TraceSHA256 = "6183d3477816862d3dd504a12310b2e91ded137519485d740e0b5a57e68a93a7"
)

type fixtureInput struct {
	Schema string      `json:"schema"`
	Cases  []inputCase `json:"cases"`
}

type inputCase struct {
	ID                 string       `json:"id"`
	Mode               string       `json:"mode"`
	LatencyCycles      int          `json:"latency_cycles"`
	RAMBytes           int          `json:"ram_bytes"`
	MaxCycles          int          `json:"max_cycles"`
	ProgramBytesHex    string       `json:"program_bytes_hex"`
	InitialMemoryWords []memoryWord `json:"initial_memory_words"`
	Source             *sourceRef   `json:"source,omitempty"`
}

type sourceRef struct {
	Fixture     string `json:"fixture"`
	InputSHA256 string `json:"input_sha256"`
}

type memoryWord struct {
	Address int    `json:"address"`
	Value   uint32 `json:"value"`
}

type oracleFile struct {
	Schema      string       `json:"schema"`
	InputSHA256 string       `json:"input_sha256"`
	Cases       []oracleCase `json:"cases"`
}

type oracleCase struct {
	ID                  string         `json:"id"`
	RequestEvents       []requestEvent `json:"request_events"`
	StoreReceipts       []storeReceipt `json:"store_receipts"`
	RetiredPCs          []int          `json:"retired_pcs"`
	SelectedFault       *preciseFault  `json:"selected_fault"`
	FaultSelectionCycle *int           `json:"fault_selection_cycle"`
	TerminalCycle       *int           `json:"terminal_cycle"`
	Registers           []uint32       `json:"registers"`
	RAMWords            []uint32       `json:"ram_words"`
	Stats               oracleStats    `json:"stats"`
}

type requestEvent struct {
	Sequence   uint64 `json:"sequence"`
	PC         int    `json:"pc"`
	Access     string `json:"access"`
	Address    uint32 `json:"address"`
	IssueCycle int    `json:"issue_cycle"`
	WaitCycles []int  `json:"wait_cycles"`
	Completed  bool   `json:"completed"`
	ReadyCycle int    `json:"ready_cycle"`
}

type storeReceipt struct {
	PC          int    `json:"pc"`
	Address     uint32 `json:"address"`
	Value       uint32 `json:"value"`
	CommitCycle int    `json:"commit_cycle"`
}

type preciseFault struct {
	PC             int    `json:"pc"`
	RawInstruction uint32 `json:"raw_instruction"`
	Access         string `json:"access"`
	Cause          string `json:"cause"`
	Address        uint32 `json:"address"`
	Width          int    `json:"width"`
}

type oracleStats struct {
	Cycles         int `json:"cycles"`
	Instructions   int `json:"instructions"`
	Forwards       int `json:"forwards"`
	Stalls         int `json:"stalls"`
	Flushes        int `json:"flushes"`
	DMemWaitCycles int `json:"dmem_wait_cycles"`
}

type traceFile struct {
	Schema      string      `json:"schema"`
	InputSHA256 string      `json:"input_sha256"`
	Cases       []traceCase `json:"cases"`
}

type traceCase struct {
	ID          string     `json:"id"`
	LegacyTrace *legacyRef `json:"legacy_trace,omitempty"`
	Trace       []cycle    `json:"trace,omitempty"`
}

type legacyRef struct {
	Fixture string `json:"fixture"`
	SHA256  string `json:"sha256"`
}

type cycle struct {
	Cycle    int               `json:"cycle"`
	PC       int               `json:"pc"`
	Stages   map[string]*stage `json:"stages"`
	Stalled  bool              `json:"stalled"`
	Flushing bool              `json:"flushing"`
}

type stage struct {
	PC                int           `json:"pc"`
	Mnemonic          string        `json:"mnemonic"`
	BranchTaken       bool          `json:"branch_taken,omitempty"`
	ForwardedOperands []forward     `json:"forwarded_operands,omitempty"`
	LoadResult        *uint32       `json:"load_result,omitempty"`
	Fault             *preciseFault `json:"fault,omitempty"`
}

type forward struct {
	Operand     string `json:"operand"`
	Register    int    `json:"register"`
	Value       int    `json:"value"`
	SourceStage string `json:"source_stage"`
}

// These legacy types intentionally mirror the published D05 projection. That
// fixture is compared byte-semantically after JSON decoding, but never rewritten.
type legacyCycle struct {
	Cycle    int                     `json:"cycle"`
	Stages   map[string]*legacyStage `json:"stages"`
	Stalled  bool                    `json:"stalled"`
	Flushing bool                    `json:"flushing"`
}

type legacyStage struct {
	PC                int       `json:"pc"`
	Mnemonic          string    `json:"mnemonic"`
	BranchTaken       bool      `json:"branch_taken,omitempty"`
	ForwardedOperands []forward `json:"forwarded_operands,omitempty"`
}

type legacyStats struct {
	Cycles       int `json:"cycles"`
	Instructions int `json:"instructions"`
	Forwards     int `json:"forwards"`
	Stalls       int `json:"stalls"`
	Flushes      int `json:"flushes"`
}

type legacyTraceFile struct {
	Schema      string        `json:"schema"`
	InputSHA256 string        `json:"input_sha256"`
	Trace       []legacyCycle `json:"trace"`
	RetiredPCs  []int         `json:"retired_pcs"`
	Stats       legacyStats   `json:"stats"`
	Registers   []uint32      `json:"registers"`
	RAMWords    []uint32      `json:"ram_words"`
}

type accessObservation struct {
	PC      int
	Access  string
	Address uint32
	Value   uint32
}

type sequentialResult struct {
	Retired   []int
	Registers []uint32
	RAMWords  []uint32
	Accesses  []accessObservation
	Fault     *preciseFault
}

type pipelineResult struct {
	Trace               traceCase
	Legacy              legacyTraceFile
	Retired             []int
	Events              []requestEvent
	Receipts            []storeReceipt
	Registers           []uint32
	RAMWords            []uint32
	Fault               *preciseFault
	FaultSelectionCycle *int
	TerminalCycle       *int
	Stats               oracleStats
}

func main() {
	root := flag.String("repo-root", "../../../..", "repository root (default assumes cwd is the Go riscv-simulator package)")
	record := flag.Bool("record", false, "record expected_trace.json after all independent oracle checks pass")
	flag.Parse()
	if flag.NArg() != 0 {
		fatalf("unexpected arguments: %v", flag.Args())
	}
	if err := replay(*root, *record); err != nil {
		fatalf("%v", err)
	}
}

func replay(root string, record bool) error {
	dir := filepath.Join(root, "code", "specs", "fixtures", "go-riscv-dmem-ready-v1")
	inputPath := filepath.Join(dir, "input.json")
	oraclePath := filepath.Join(dir, "expected_oracle.json")
	tracePath := filepath.Join(dir, "expected_trace.json")
	inputBytes, err := os.ReadFile(inputPath)
	if err != nil {
		return fmt.Errorf("read input fixture: %w", err)
	}
	oracleBytes, err := os.ReadFile(oraclePath)
	if err != nil {
		return fmt.Errorf("read independent oracle: %w", err)
	}
	var in fixtureInput
	if err := decodeStrict(inputBytes, &in); err != nil {
		return fmt.Errorf("decode input fixture: %w", err)
	}
	var oracle oracleFile
	if err := decodeStrict(oracleBytes, &oracle); err != nil {
		return fmt.Errorf("decode independent oracle: %w", err)
	}
	inputHash := sha256.Sum256(inputBytes)
	inputHashText := hex.EncodeToString(inputHash[:])
	oracleHash := sha256.Sum256(oracleBytes)
	if inputHashText != inputSHA256 || hex.EncodeToString(oracleHash[:]) != oracleSHA256 {
		return fmt.Errorf("canonical input/oracle content identity mismatch: input=%s oracle=%s", inputHashText, hex.EncodeToString(oracleHash[:]))
	}
	if err := validateInput(in, inputHashText, root); err != nil {
		return err
	}
	if oracle.Schema != oracleSchema || oracle.InputSHA256 != inputHashText || len(oracle.Cases) != len(in.Cases) {
		return fmt.Errorf("oracle metadata mismatch: schema=%q hash=%s cases=%d", oracle.Schema, oracle.InputSHA256, len(oracle.Cases))
	}

	results := make([]pipelineResult, len(in.Cases))
	for index, current := range in.Cases {
		want := oracle.Cases[index]
		program, _ := hex.DecodeString(current.ProgramBytesHex)
		if err := validateOracleCase(current, want); err != nil {
			return fmt.Errorf("case %s: invalid independent oracle: %w", current.ID, err)
		}
		sequential, err := runSequentialOracle(current, program, want)
		if err != nil {
			return fmt.Errorf("case %s: independent sequential oracle: %w", current.ID, err)
		}
		if err := compareSequential(sequential, want); err != nil {
			return fmt.Errorf("case %s: independent sequential oracle mismatch: %w", current.ID, err)
		}
		if err := compareSemanticAccesses(sequential.Accesses, want); err != nil {
			return fmt.Errorf("case %s: request semantics mismatch: %w", current.ID, err)
		}
		pipeline, err := runPipeline(current, program)
		if err != nil {
			return fmt.Errorf("case %s: pipeline execution: %w", current.ID, err)
		}
		if err := comparePipeline(pipeline, want); err != nil {
			return fmt.Errorf("case %s: pipeline differs from independent oracle: %w", current.ID, err)
		}
		results[index] = pipeline
	}

	actual := traceFile{Schema: traceSchema, InputSHA256: inputHashText, Cases: make([]traceCase, len(results))}
	for index, result := range results {
		actual.Cases[index] = result.Trace
	}
	legacyReference, err := compareLegacyD05(filepath.Join(root, "code", "specs", "fixtures", "go-riscv-d05-v1"), results[0].Legacy)
	if err != nil {
		return err
	}
	actual.Cases[0].LegacyTrace = &legacyReference
	if record {
		encoded, err := json.MarshalIndent(actual, "", "  ")
		if err != nil {
			return fmt.Errorf("marshal normalized trace: %w", err)
		}
		encoded = append(encoded, '\n')
		if err := validateTraceShape(encoded, oracle); err != nil {
			return fmt.Errorf("refusing to record invalid normalized trace: %w", err)
		}
		if err := atomicWrite(tracePath, encoded); err != nil {
			return fmt.Errorf("record normalized trace: %w", err)
		}
	} else {
		expectedBytes, err := os.ReadFile(tracePath)
		if err != nil {
			return fmt.Errorf("read normalized trace: %w (run with explicit -record after reviewing the independent oracle)", err)
		}
		var expected traceFile
		if err := validateTraceShape(expectedBytes, oracle); err != nil {
			return fmt.Errorf("validate normalized trace shape: %w", err)
		}
		if err := decodeStrict(expectedBytes, &expected); err != nil {
			return fmt.Errorf("decode normalized trace: %w", err)
		}
		if !reflect.DeepEqual(actual, expected) {
			actualJSON, _ := json.MarshalIndent(actual, "", "  ")
			return fmt.Errorf("normalized trace differs; review explicit -record output\nactual:\n%s", actualJSON)
		}
	}

	traceBytes, err := os.ReadFile(tracePath)
	if err == nil {
		hash := sha256.Sum256(traceBytes)
		fmt.Printf("PASS D-MEM fixture: cases=%d trace_sha256=%s input_sha256=%s (independent oracle checked first)\n", len(results), hex.EncodeToString(hash[:]), inputHashText)
	}
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

func validateTraceShape(data []byte, oracle oracleFile) error {
	var root map[string]json.RawMessage
	if err := json.Unmarshal(data, &root); err != nil {
		return err
	}
	if err := exactKeys(root, []string{"schema", "input_sha256", "cases"}); err != nil {
		return fmt.Errorf("root: %w", err)
	}
	var schema, inputHash string
	if err := json.Unmarshal(root["schema"], &schema); err != nil || schema != traceSchema {
		return fmt.Errorf("unexpected schema %q", schema)
	}
	if err := json.Unmarshal(root["input_sha256"], &inputHash); err != nil || inputHash != inputSHA256 {
		return fmt.Errorf("unexpected input identity %q", inputHash)
	}
	var cases []json.RawMessage
	if err := json.Unmarshal(root["cases"], &cases); err != nil || len(cases) != len(oracle.Cases) {
		return fmt.Errorf("trace case count mismatch")
	}
	for index, rawCase := range cases {
		var object map[string]json.RawMessage
		if err := json.Unmarshal(rawCase, &object); err != nil {
			return fmt.Errorf("case %d: %w", index, err)
		}
		var id string
		if err := json.Unmarshal(object["id"], &id); err != nil || id != oracle.Cases[index].ID {
			return fmt.Errorf("case %d has unexpected id %q", index, id)
		}
		if index == 0 {
			if err := exactKeys(object, []string{"id", "legacy_trace"}); err != nil {
				return fmt.Errorf("D05 trace reference: %w", err)
			}
			var reference map[string]json.RawMessage
			if err := json.Unmarshal(object["legacy_trace"], &reference); err != nil {
				return err
			}
			if err := exactKeys(reference, []string{"fixture", "sha256"}); err != nil {
				return fmt.Errorf("D05 reference: %w", err)
			}
			continue
		}
		if err := exactKeys(object, []string{"id", "trace"}); err != nil {
			return fmt.Errorf("case %s: %w", id, err)
		}
		var cycles []json.RawMessage
		if err := json.Unmarshal(object["trace"], &cycles); err != nil || len(cycles) != oracle.Cases[index].Stats.Cycles {
			return fmt.Errorf("case %s trace length mismatch", id)
		}
		for cycleIndex, rawCycle := range cycles {
			var entry map[string]json.RawMessage
			if err := json.Unmarshal(rawCycle, &entry); err != nil {
				return fmt.Errorf("case %s cycle %d: %w", id, cycleIndex+1, err)
			}
			if err := exactKeys(entry, []string{"cycle", "pc", "stages", "stalled", "flushing"}); err != nil {
				return fmt.Errorf("case %s cycle %d: %w", id, cycleIndex+1, err)
			}
			var number int
			if err := json.Unmarshal(entry["cycle"], &number); err != nil || number != cycleIndex+1 {
				return fmt.Errorf("case %s cycles must be contiguous and one-based", id)
			}
			if err := json.Unmarshal(entry["pc"], &number); err != nil || number < 0 || uint64(number) > 0xffffffff {
				return fmt.Errorf("case %s cycle %d has invalid next-fetch PC", id, cycleIndex+1)
			}
			var stages map[string]json.RawMessage
			if err := json.Unmarshal(entry["stages"], &stages); err != nil || len(stages) != 5 {
				return fmt.Errorf("case %s cycle %d must have all five stage keys", id, cycleIndex+1)
			}
			for _, name := range []string{"IF", "ID", "EX", "MEM", "WB"} {
				rawStage, ok := stages[name]
				if !ok {
					return fmt.Errorf("case %s cycle %d missing stage %s", id, cycleIndex+1, name)
				}
				if bytes.Equal(bytes.TrimSpace(rawStage), []byte("null")) {
					continue
				}
				var stageObject map[string]json.RawMessage
				if err := json.Unmarshal(rawStage, &stageObject); err != nil {
					return fmt.Errorf("case %s cycle %d stage %s: %w", id, cycleIndex+1, name, err)
				}
				if err := exactKeys(stageObject, []string{"pc", "mnemonic"}, []string{"branch_taken", "forwarded_operands", "load_result", "fault"}); err != nil {
					return fmt.Errorf("case %s cycle %d stage %s: %w", id, cycleIndex+1, name, err)
				}
				if name != "EX" {
					if _, ok := stageObject["forwarded_operands"]; ok {
						return fmt.Errorf("forwarding may only be recorded at EX")
					}
				}
				if name != "MEM" {
					if _, ok := stageObject["load_result"]; ok {
						return fmt.Errorf("load result may only be recorded at MEM")
					}
				}
				if err := json.Unmarshal(stageObject["pc"], &number); err != nil || number < 0 || number%4 != 0 || uint64(number) > 0xfffffffc {
					return fmt.Errorf("case %s cycle %d stage %s has invalid PC", id, cycleIndex+1, name)
				}
				if rawForward, ok := stageObject["forwarded_operands"]; ok {
					var forwards []json.RawMessage
					if err := json.Unmarshal(rawForward, &forwards); err != nil {
						return fmt.Errorf("invalid forwarded operand list")
					}
					for _, rawForward := range forwards {
						var forwardObject map[string]json.RawMessage
						if err := json.Unmarshal(rawForward, &forwardObject); err != nil {
							return err
						}
						if err := exactKeys(forwardObject, []string{"operand", "register", "value", "source_stage"}); err != nil {
							return err
						}
					}
				}
				if rawFault, ok := stageObject["fault"]; ok {
					var faultObject map[string]json.RawMessage
					if err := json.Unmarshal(rawFault, &faultObject); err != nil {
						return err
					}
					if err := exactKeys(faultObject, []string{"pc", "raw_instruction", "access", "cause", "address", "width"}); err != nil {
						return err
					}
				}
			}
		}
	}
	return nil
}

func exactKeys(object map[string]json.RawMessage, required []string, optional ...[]string) error {
	allowed := make(map[string]bool, len(required)+len(optional))
	for _, key := range required {
		allowed[key] = true
		if _, ok := object[key]; !ok {
			return fmt.Errorf("missing required key %q", key)
		}
	}
	for _, group := range optional {
		for _, key := range group {
			allowed[key] = true
		}
	}
	for key := range object {
		if !allowed[key] {
			return fmt.Errorf("unknown key %q", key)
		}
	}
	return nil
}

func validateInput(in fixtureInput, inputHash, repoRoot string) error {
	ids := []string{"d05-precise-latency-one", "delayed-store-load", "older-store-before-younger-fault"}
	if in.Schema != inputSchema || len(in.Cases) != len(ids) {
		return fmt.Errorf("input schema/case count invalid: %q/%d", in.Schema, len(in.Cases))
	}
	canonicalPrograms := []string{
		"13003006930000001301000823201100930110008321010063841100930230061382110073000000",
		"1305000893007003232015000321050073000000",
		"9300a0022320100803210010930f3006",
	}
	for index, current := range in.Cases {
		if current.ID != ids[index] || current.Mode != "precise-flat-ram" || current.RAMBytes != 256 || current.MaxCycles != 100 {
			return fmt.Errorf("case %d has unsupported identity/config: %+v", index, current)
		}
		if current.LatencyCycles != []int{1, 3, 3}[index] || current.RAMBytes < 4 || current.RAMBytes > 65536 || current.RAMBytes%4 != 0 || current.MaxCycles < 1 || current.MaxCycles > 400 || current.ProgramBytesHex != canonicalPrograms[index] {
			return fmt.Errorf("case %s has invalid latency/RAM/budget", current.ID)
		}
		program, err := hex.DecodeString(current.ProgramBytesHex)
		if err != nil || len(program) == 0 || strings.ToLower(current.ProgramBytesHex) != current.ProgramBytesHex || len(program)%4 != 0 || len(program) > 128 || len(program) > current.RAMBytes {
			return fmt.Errorf("case %s has invalid canonical program bytes: %v", current.ID, err)
		}
		addresses := make(map[int]bool)
		for _, word := range current.InitialMemoryWords {
			if word.Address < 0 || word.Address%4 != 0 || word.Address > current.RAMBytes-4 || addresses[word.Address] {
				return fmt.Errorf("case %s has duplicate or out-of-range initial word %+v", current.ID, word)
			}
			addresses[word.Address] = true
		}
		if index == 0 {
			if current.Source == nil || current.Source.Fixture != "go-riscv-d05-v1/input.json" || current.Source.InputSHA256 != d05InputSHA256 {
				return fmt.Errorf("D05 compatibility source reference missing")
			}
			sourcePath := filepath.Join(repoRoot, "code", "specs", "fixtures", filepath.FromSlash(current.Source.Fixture))
			sourceBytes, err := os.ReadFile(sourcePath)
			if err != nil {
				return fmt.Errorf("read D05 source reference: %w", err)
			}
			hash := sha256.Sum256(sourceBytes)
			if hex.EncodeToString(hash[:]) != current.Source.InputSHA256 {
				return fmt.Errorf("D05 source hash mismatch")
			}
			var source struct {
				Program string       `json:"program_bytes_hex"`
				Words   []memoryWord `json:"initial_memory_words"`
			}
			if err := json.Unmarshal(sourceBytes, &source); err != nil {
				return fmt.Errorf("decode D05 source reference: %w", err)
			}
			if source.Program != current.ProgramBytesHex || !reflect.DeepEqual(source.Words, current.InitialMemoryWords) {
				return fmt.Errorf("D05 duplicated canonical guest differs from source")
			}
		} else if current.Source != nil {
			return fmt.Errorf("non-D05 case %s has unexpected source reference", current.ID)
		}
	}
	if inputHash != inputSHA256 {
		return fmt.Errorf("input SHA-256 changed: %s", inputHash)
	}
	return nil
}

func validateOracleCase(in inputCase, want oracleCase) error {
	if want.ID != in.ID || len(want.Registers) != 32 || len(want.RAMWords) != in.RAMBytes/4 || len(want.RetiredPCs) > 32 {
		return errors.New("oracle identity or state dimensions invalid")
	}
	if want.Stats.Cycles < 1 || want.Stats.Cycles > in.MaxCycles || want.Stats.Instructions != len(want.RetiredPCs) {
		return errors.New("oracle counters invalid")
	}
	expectedStats := map[string]oracleStats{
		"d05-precise-latency-one":          {Cycles: 16, Instructions: 9, Forwards: 2, Stalls: 1, Flushes: 1, DMemWaitCycles: 0},
		"delayed-store-load":               {Cycles: 13, Instructions: 5, Forwards: 1, Stalls: 4, Flushes: 0, DMemWaitCycles: 4},
		"older-store-before-younger-fault": {Cycles: 9, Instructions: 2, Forwards: 1, Stalls: 2, Flushes: 0, DMemWaitCycles: 2},
	}
	if expected, ok := expectedStats[in.ID]; !ok || want.Stats != expected {
		return fmt.Errorf("oracle stats do not match reviewed case arithmetic: got=%+v want=%+v", want.Stats, expected)
	}
	waitCount, previousSequence := 0, uint64(0)
	for _, event := range want.RequestEvents {
		if event.Sequence != previousSequence+1 || event.PC < 0 || event.PC%4 != 0 || event.Address%4 != 0 || event.IssueCycle < 1 || event.IssueCycle > in.MaxCycles || !event.Completed {
			return fmt.Errorf("invalid request event %+v", event)
		}
		previousSequence = event.Sequence
		expectedWaits := make([]int, 0, in.LatencyCycles-1)
		for cycle := event.IssueCycle + 1; cycle < event.IssueCycle+in.LatencyCycles; cycle++ {
			expectedWaits = append(expectedWaits, cycle)
		}
		ready := event.IssueCycle
		if in.LatencyCycles > 1 {
			ready = event.IssueCycle + in.LatencyCycles - 1
		}
		if !reflect.DeepEqual(event.WaitCycles, expectedWaits) || event.ReadyCycle != ready || event.ReadyCycle > want.Stats.Cycles {
			return fmt.Errorf("event timing mismatch at PC %d: waits=%v ready=%d want %v/%d", event.PC, event.WaitCycles, event.ReadyCycle, expectedWaits, ready)
		}
		waitCount += len(event.WaitCycles)
	}
	if waitCount != want.Stats.DMemWaitCycles || want.Stats.Stalls < waitCount {
		return fmt.Errorf("D-MEM wait counter mismatch: events=%d stats=%d stalls=%d", waitCount, want.Stats.DMemWaitCycles, want.Stats.Stalls)
	}
	if want.SelectedFault == nil {
		if want.FaultSelectionCycle != nil || want.TerminalCycle != nil {
			return errors.New("nonfault case has fault cycles")
		}
	} else if want.FaultSelectionCycle == nil || want.TerminalCycle == nil || *want.FaultSelectionCycle >= *want.TerminalCycle || *want.TerminalCycle > want.Stats.Cycles {
		return errors.New("fault selection/terminal timing invalid")
	}
	if in.ID == "older-store-before-younger-fault" {
		if want.SelectedFault == nil || want.SelectedFault.PC != 8 || want.SelectedFault.RawInstruction != 0x10002103 || want.SelectedFault.Access != "load" || want.SelectedFault.Cause != "load_access_fault" || want.SelectedFault.Address != 256 || want.SelectedFault.Width != 4 {
			return errors.New("fault oracle does not describe the canonical out-of-range load")
		}
	}
	for _, receipt := range want.StoreReceipts {
		if receipt.PC < 0 || receipt.PC%4 != 0 || receipt.Address%4 != 0 || receipt.CommitCycle < 1 || receipt.CommitCycle > want.Stats.Cycles {
			return fmt.Errorf("invalid store receipt %+v", receipt)
		}
	}
	return nil
}

func runSequentialOracle(in inputCase, program []byte, want oracleCase) (sequentialResult, error) {
	sequential := rv.NewRiscVSimulator(in.RAMBytes)
	for _, word := range in.InitialMemoryWords {
		sequential.CPU.Memory.WriteWord(word.Address, word.Value)
	}
	sequential.CPU.LoadProgram(program, 0)
	result := sequentialResult{Retired: make([]int, 0, len(want.RetiredPCs)), Accesses: make([]accessObservation, 0, len(want.RequestEvents))}
	for steps := 0; steps < in.MaxCycles; steps++ {
		pc := sequential.CPU.PC
		if pc < 0 || pc+4 > len(program) {
			return sequentialResult{}, fmt.Errorf("sequential PC %d left the canonical image before halt", pc)
		}
		raw := sequential.CPU.Memory.ReadWord(pc)
		access, address, value, memoryOp := sequentialMemoryAccess(raw, sequential)
		if memoryOp {
			if uint64(address)+4 > uint64(in.RAMBytes) {
				fault := &preciseFault{PC: pc, RawInstruction: raw, Access: access, Cause: access + "_access_fault", Address: address, Width: 4}
				result.Fault = fault
				break
			}
			result.Accesses = append(result.Accesses, accessObservation{PC: pc, Access: access, Address: address, Value: value})
		}
		step := sequential.CPU.Step()
		result.Retired = append(result.Retired, step.Fetch.PC)
		if sequential.CPU.Halted {
			break
		}
	}
	if result.Fault != nil {
		if result.Fault.PC != len(program)-8 || result.Fault.Access != "load" {
			return sequentialResult{}, fmt.Errorf("fault was not at expected final word: %+v", result.Fault)
		}
		// Restore all code bytes, retaining only successful-prefix state changes.
		sequential.CPU.LoadProgram(program, 0)
	} else if !sequential.CPU.Halted {
		return sequentialResult{}, errors.New("sequential program did not halt")
	}
	result.Registers = make([]uint32, 32)
	for register := range result.Registers {
		result.Registers[register] = sequential.CPU.Registers.Read(register)
	}
	result.RAMWords = make([]uint32, in.RAMBytes/4)
	for index := range result.RAMWords {
		result.RAMWords[index] = sequential.CPU.Memory.ReadWord(index * 4)
	}
	return result, nil
}

func sequentialMemoryAccess(raw uint32, sim *rv.RiscVSimulator) (access string, address uint32, value uint32, ok bool) {
	opcode := raw & 0x7f
	funct3 := (raw >> 12) & 7
	rs1 := int((raw >> 15) & 31)
	rs2 := int((raw >> 20) & 31)
	if opcode == 0x03 && funct3 == 2 {
		access = "load"
		imm := signExtend(raw>>20, 12)
		address = sim.CPU.Registers.Read(rs1) + uint32(imm)
		return access, address, 0, true
	}
	if opcode == 0x23 && funct3 == 2 {
		access = "store"
		imm := signExtend(((raw>>25)<<5)|((raw>>7)&31), 12)
		address = sim.CPU.Registers.Read(rs1) + uint32(imm)
		return access, address, sim.CPU.Registers.Read(rs2), true
	}
	return "", 0, 0, false
}

func signExtend(value uint32, width uint) int {
	shift := 32 - width
	return int(int32(value<<shift) >> shift)
}

func compareSequential(got sequentialResult, want oracleCase) error {
	if !reflect.DeepEqual(got.Retired, want.RetiredPCs) || !reflect.DeepEqual(got.Registers, want.Registers) || !reflect.DeepEqual(got.RAMWords, want.RAMWords) {
		return fmt.Errorf("retired or architectural state differs (retired=%v want=%v)", got.Retired, want.RetiredPCs)
	}
	if !reflect.DeepEqual(got.Fault, want.SelectedFault) {
		return fmt.Errorf("fault differs: got=%+v want=%+v", got.Fault, want.SelectedFault)
	}
	return nil
}

func compareSemanticAccesses(accesses []accessObservation, want oracleCase) error {
	if len(accesses) != len(want.RequestEvents) {
		return fmt.Errorf("native sequential access count=%d oracle request count=%d", len(accesses), len(want.RequestEvents))
	}
	for index, access := range accesses {
		event := want.RequestEvents[index]
		if access.PC != event.PC || access.Access != event.Access || access.Address != event.Address {
			return fmt.Errorf("access[%d]=%+v differs from request %+v", index, access, event)
		}
	}
	storeCount := 0
	for _, access := range accesses {
		if access.Access != "store" {
			continue
		}
		storeCount++
		var receipt *storeReceipt
		for index := range want.StoreReceipts {
			if want.StoreReceipts[index].PC == access.PC {
				receipt = &want.StoreReceipts[index]
				break
			}
		}
		if receipt == nil || receipt.Address != access.Address || receipt.Value != access.Value {
			return fmt.Errorf("store at PC %d has no matching sequentially derived receipt", access.PC)
		}
		var event *requestEvent
		for index := range want.RequestEvents {
			if want.RequestEvents[index].PC == access.PC {
				event = &want.RequestEvents[index]
				break
			}
		}
		if event == nil || receipt.CommitCycle != event.ReadyCycle+1 {
			return fmt.Errorf("store receipt at PC %d does not commit in the first WB cycle after ready", access.PC)
		}
	}
	if storeCount != len(want.StoreReceipts) {
		return fmt.Errorf("sequential store count=%d oracle receipt count=%d", storeCount, len(want.StoreReceipts))
	}
	return nil
}

func runPipeline(in inputCase, program []byte) (pipelineResult, error) {
	config := core.SimpleConfig()
	staged, err := rv.NewRiscVCoreWithPreciseMemoryLatency(config, in.RAMBytes, in.LatencyCycles)
	if err != nil {
		return pipelineResult{}, fmt.Errorf("construct precise core: %w", err)
	}
	staged.LoadProgram(program, 0)
	for _, word := range in.InitialMemoryWords {
		staged.MemoryController().WriteWord(word.Address, int(word.Value))
	}
	trace := make([]cycle, 0, in.MaxCycles)
	legacyTrace := make([]legacyCycle, 0, in.MaxCycles)
	retired := make([]int, 0, 32)
	terminalCycle, faultSelectionCycle := (*int)(nil), (*int)(nil)
	for count := 0; count < in.MaxCycles && !staged.IsHalted(); count++ {
		snapshot := staged.Step()
		entry := cycle{Cycle: snapshot.Cycle, PC: snapshot.PC, Stages: make(map[string]*stage, 5), Stalled: snapshot.Stalled, Flushing: snapshot.Flushing}
		legacy := legacyCycle{Cycle: snapshot.Cycle, Stages: make(map[string]*legacyStage, 5), Stalled: snapshot.Stalled, Flushing: snapshot.Flushing}
		for _, name := range []string{"IF", "ID", "EX", "MEM", "WB"} {
			token := snapshot.Stages[name]
			if token == nil || token.IsBubble {
				entry.Stages[name] = nil
				legacy.Stages[name] = nil
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
					current.ForwardedOperands = append(current.ForwardedOperands, forward{Operand: operand, Register: item.Register, Value: item.Value, SourceStage: item.SourceStage})
				}
			}
			if name == "MEM" && token.MemRead && !token.IsBubble {
				for _, event := range staged.PreciseMemoryRequestEvents() {
					if event.PC == token.PC && event.Completed && event.CompletionCycle <= snapshot.Cycle {
						value := uint32(token.MemData)
						current.LoadResult = &value
					}
				}
			}
			if token.PreciseFault != nil {
				current.Fault = &preciseFault{PC: token.PreciseFault.PC, RawInstruction: token.PreciseFault.RawInstruction, Access: token.PreciseFault.Access, Cause: token.PreciseFault.Cause, Address: token.PreciseFault.Address, Width: token.PreciseFault.Width}
				if name == "MEM" && faultSelectionCycle == nil {
					cycleNo := snapshot.Cycle
					faultSelectionCycle = &cycleNo
				}
				if name == "WB" {
					cycleNo := snapshot.Cycle
					terminalCycle = &cycleNo
				}
			}
			entry.Stages[name] = current
			legacyStageValue := &legacyStage{PC: current.PC, Mnemonic: current.Mnemonic, BranchTaken: current.BranchTaken, ForwardedOperands: current.ForwardedOperands}
			legacy.Stages[name] = legacyStageValue
		}
		if wb := entry.Stages["WB"]; wb != nil {
			if wb.Fault == nil {
				retired = append(retired, wb.PC)
			}
		}
		trace = append(trace, entry)
		legacyTrace = append(legacyTrace, legacy)
	}
	if !staged.IsHalted() {
		return pipelineResult{}, fmt.Errorf("precise core did not halt within %d cycles", in.MaxCycles)
	}
	stats := staged.Stats()
	registers := make([]uint32, 32)
	for index := range registers {
		registers[index] = uint32(staged.ReadRegister(index))
	}
	ramWords := make([]uint32, in.RAMBytes/4)
	for index := range ramWords {
		ramWords[index] = uint32(staged.MemoryController().ReadWord(index * 4))
	}
	events := make([]requestEvent, 0)
	for _, event := range staged.PreciseMemoryRequestEvents() {
		events = append(events, requestEvent{Sequence: event.Sequence, PC: event.PC, Access: event.Access, Address: event.Address, IssueCycle: event.IssueCycle, WaitCycles: append([]int{}, event.WaitCycles...), Completed: event.Completed, ReadyCycle: event.CompletionCycle})
	}
	receipts := make([]storeReceipt, 0)
	for _, receipt := range staged.PreciseStoreReceipts() {
		receipts = append(receipts, storeReceipt{PC: receipt.PC, Address: receipt.Address, Value: receipt.Value, CommitCycle: receipt.Cycle})
	}
	fault, hasFault := staged.PreciseFaultOutcome()
	var faultValue *preciseFault
	if hasFault {
		faultValue = &preciseFault{PC: fault.PC, RawInstruction: fault.RawInstruction, Access: fault.Access, Cause: fault.Cause, Address: fault.Address, Width: fault.Width}
	}
	if in.ID == "older-store-before-younger-fault" && faultSelectionCycle == nil {
		return pipelineResult{}, errors.New("fault never reached MEM selection boundary")
	}
	legacyInputHash := ""
	if in.Source != nil {
		legacyInputHash = in.Source.InputSHA256
	}
	legacy := legacyTraceFile{
		Schema: "go-riscv-d05-pipeline-trace-v1", InputSHA256: legacyInputHash,
		Trace: legacyTrace, RetiredPCs: retired,
		Stats:     legacyStats{Cycles: stats.TotalCycles, Instructions: stats.InstructionsCompleted, Forwards: stats.ForwardCount, Stalls: stats.StallCount, Flushes: stats.FlushCount},
		Registers: registers, RAMWords: ramWords,
	}
	resultTrace := traceCase{ID: in.ID, Trace: trace}
	if in.ID == "d05-precise-latency-one" {
		resultTrace.Trace = nil
	}
	return pipelineResult{
		Trace: resultTrace, Legacy: legacy, Retired: retired, Events: events, Receipts: receipts,
		Registers: registers, RAMWords: ramWords, Fault: faultValue,
		FaultSelectionCycle: faultSelectionCycle, TerminalCycle: terminalCycle,
		Stats: oracleStats{Cycles: stats.TotalCycles, Instructions: stats.InstructionsCompleted, Forwards: stats.ForwardCount, Stalls: stats.StallCount, Flushes: stats.FlushCount, DMemWaitCycles: sumWaits(events)},
	}, nil
}

func comparePipeline(got pipelineResult, want oracleCase) error {
	if !reflect.DeepEqual(got.Retired, want.RetiredPCs) || !reflect.DeepEqual(got.Events, want.RequestEvents) || !reflect.DeepEqual(got.Receipts, want.StoreReceipts) || !reflect.DeepEqual(got.Registers, want.Registers) || !reflect.DeepEqual(got.RAMWords, want.RAMWords) || !reflect.DeepEqual(got.Fault, want.SelectedFault) || !reflect.DeepEqual(got.FaultSelectionCycle, want.FaultSelectionCycle) || !reflect.DeepEqual(got.TerminalCycle, want.TerminalCycle) || !reflect.DeepEqual(got.Stats, want.Stats) {
		return fmt.Errorf("pipeline state/events differ from oracle (retired=%v; events=%+v; receipts=%+v; fault=%+v at %v/%v; stats=%+v)", got.Retired, got.Events, got.Receipts, got.Fault, got.FaultSelectionCycle, got.TerminalCycle, got.Stats)
	}
	return nil
}

func compareLegacyD05(dir string, actual legacyTraceFile) (legacyRef, error) {
	legacyInputPath := filepath.Join(dir, "input.json")
	legacyTracePath := filepath.Join(dir, "expected_trace.json")
	legacyInputBytes, err := os.ReadFile(legacyInputPath)
	if err != nil {
		return legacyRef{}, fmt.Errorf("read legacy D05 input: %w", err)
	}
	legacyInputHash := sha256.Sum256(legacyInputBytes)
	fixture := legacyRef{Fixture: "go-riscv-d05-v1/expected_trace.json"}
	traceBytes, err := os.ReadFile(legacyTracePath)
	if err != nil {
		return legacyRef{}, fmt.Errorf("read legacy D05 expected trace: %w", err)
	}
	legacyHash := sha256.Sum256(traceBytes)
	fixture.SHA256 = hex.EncodeToString(legacyHash[:])
	if actual.InputSHA256 != hex.EncodeToString(legacyInputHash[:]) || fixture.SHA256 != d05TraceSHA256 {
		return legacyRef{}, errors.New("legacy D05 identity invalid")
	}
	var expected legacyTraceFile
	if err := decodeStrict(traceBytes, &expected); err != nil {
		return legacyRef{}, fmt.Errorf("decode unchanged D05 trace: %w", err)
	}
	if !reflect.DeepEqual(actual, expected) {
		return legacyRef{}, errors.New("precise latency-one D05 projection differs from the unchanged legacy expected_trace.json")
	}
	return fixture, nil
}

func sumWaits(events []requestEvent) int {
	total := 0
	for _, event := range events {
		total += len(event.WaitCycles)
	}
	return total
}

func atomicWrite(path string, data []byte) error {
	temporary, err := os.CreateTemp(filepath.Dir(path), ".dmem-trace-*.tmp")
	if err != nil {
		return err
	}
	temporaryPath := temporary.Name()
	defer os.Remove(temporaryPath)
	if _, err := temporary.Write(data); err != nil {
		temporary.Close()
		return err
	}
	if err := temporary.Sync(); err != nil {
		temporary.Close()
		return err
	}
	if err := temporary.Close(); err != nil {
		return err
	}
	return os.Rename(temporaryPath, path)
}

func fatalf(format string, args ...any) {
	fmt.Fprintf(os.Stderr, format+"\n", args...)
	os.Exit(1)
}
