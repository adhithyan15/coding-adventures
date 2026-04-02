// Package cpusimulator provides pipeline tracing.
//
// === What is a pipeline? ===
//
// Every CPU operates by repeating three steps over and over:
// FETCH -> DECODE -> EXECUTE.
// To make parsing this clear visually, this file provides structs
// to capture the result of each stage and format it nicely.
package cpusimulator

import (
	"fmt"
	"strings"
)

// FetchResult captures raw instruction bytes from memory.
type FetchResult struct {
	PC             int
	RawInstruction uint32
}

// DecodeResult translates bits into ISA-specific operation commands.
type DecodeResult struct {
	Mnemonic       string
	Fields         map[string]int
	RawInstruction uint32
}

// ExecuteResult contains the changes made to registers and memory.
type ExecuteResult struct {
	Description      string
	RegistersChanged map[string]uint32
	MemoryChanged    map[int]byte
	NextPC           int
	Halted           bool
}

// PipelineTrace forms a historical log of what happened to an instruction.
type PipelineTrace struct {
	Cycle            int
	Fetch            FetchResult
	Decode           DecodeResult
	Execute          ExecuteResult
	RegisterSnapshot map[string]uint32
}

// FormatPipeline produces a multi-column visual map of the instruction trace.
func (p *PipelineTrace) FormatPipeline() string {
	result, _ := StartNew[string]("cpu-simulator.PipelineTrace.FormatPipeline", "",
		func(op *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
			op.AddProperty("cycle", p.Cycle)
			fetchLines := []string{
				"FETCH",
				fmt.Sprintf("PC: 0x%04X", p.Fetch.PC),
				fmt.Sprintf("-> 0x%08X", p.Fetch.RawInstruction),
			}

			decodeFields := []string{}
			for k, v := range p.Decode.Fields {
				decodeFields = append(decodeFields, fmt.Sprintf("%s=%d", k, v))
			}

			decodeLines := []string{
				"DECODE",
				p.Decode.Mnemonic,
				strings.Join(decodeFields, " "),
			}

			executeLines := []string{
				"EXECUTE",
				p.Execute.Description,
				fmt.Sprintf("PC -> %d", p.Execute.NextPC),
			}

			maxLines := len(fetchLines)
			if len(decodeLines) > maxLines {
				maxLines = len(decodeLines)
			}
			if len(executeLines) > maxLines {
				maxLines = len(executeLines)
			}

			for len(fetchLines) < maxLines {
				fetchLines = append(fetchLines, "")
			}
			for len(decodeLines) < maxLines {
				decodeLines = append(decodeLines, "")
			}
			for len(executeLines) < maxLines {
				executeLines = append(executeLines, "")
			}

			colWidth := 20
			// We use %-[width]s to pad the strings to the required column width explicitly.
			lines := []string{fmt.Sprintf("--- Cycle %d ---", p.Cycle)}
			for i := 0; i < maxLines; i++ {
				f := fmt.Sprintf("%-*s", colWidth, fetchLines[i])
				d := fmt.Sprintf("%-*s", colWidth, decodeLines[i])
				e := fmt.Sprintf("%-*s", colWidth, executeLines[i])
				lines = append(lines, fmt.Sprintf("  %s | %s | %s", f, d, e))
			}
			return rf.Generate(true, false, strings.Join(lines, "\n"))
		}).GetResult()
	return result
}
