// Package code128 implements the Code 128 barcode symbology and emits PaintScene.
package code128

import (
	"fmt"

	barcodelayout1d "github.com/adhithyan15/coding-adventures/code/packages/go/barcode-layout-1d"
	paintinstructions "github.com/adhithyan15/coding-adventures/code/packages/go/paint-instructions"
)

const Version = "0.1.0"

const (
	startB = 104
	stop   = 106
)

type EncodedSymbol struct {
	Label       string
	Value       int
	Pattern     string
	SourceIndex int
	Role        string
}

type BarcodeRun = barcodelayout1d.Barcode1DRun
type RenderConfig = barcodelayout1d.LayoutConfig

var DefaultLayoutConfig = barcodelayout1d.DefaultLayoutConfig
var DefaultRenderConfig = DefaultLayoutConfig

var patterns = []string{
	"11011001100", "11001101100", "11001100110", "10010011000", "10010001100",
	"10001001100", "10011001000", "10011000100", "10001100100", "11001001000",
	"11001000100", "11000100100", "10110011100", "10011011100", "10011001110",
	"10111001100", "10011101100", "10011100110", "11001110010", "11001011100",
	"11001001110", "11011100100", "11001110100", "11101101110", "11101001100",
	"11100101100", "11100100110", "11101100100", "11100110100", "11100110010",
	"11011011000", "11011000110", "11000110110", "10100011000", "10001011000",
	"10001000110", "10110001000", "10001101000", "10001100010", "11010001000",
	"11000101000", "11000100010", "10110111000", "10110001110", "10001101110",
	"10111011000", "10111000110", "10001110110", "11101110110", "11010001110",
	"11000101110", "11011101000", "11011100010", "11011101110", "11101011000",
	"11101000110", "11100010110", "11101101000", "11101100010", "11100011010",
	"11101111010", "11001000010", "11110001010", "10100110000", "10100001100",
	"10010110000", "10010000110", "10000101100", "10000100110", "10110010000",
	"10110000100", "10011010000", "10011000010", "10000110100", "10000110010",
	"11000010010", "11001010000", "11110111010", "11000010100", "10001111010",
	"10100111100", "10010111100", "10010011110", "10111100100", "10011110100",
	"10011110010", "11110100100", "11110010100", "11110010010", "11011011110",
	"11011110110", "11110110110", "10101111000", "10100011110", "10001011110",
	"10111101000", "10111100010", "11110101000", "11110100010", "10111011110",
	"10111101110", "11101011110", "11110101110", "11010000100", "11010010000",
	"11010011100", "1100011101011",
}

func copyMetadata(metadata paintinstructions.Metadata) paintinstructions.Metadata {
	if metadata == nil {
		return paintinstructions.Metadata{}
	}
	result := make(paintinstructions.Metadata, len(metadata))
	for key, value := range metadata {
		result[key] = value
	}
	return result
}

// NormalizeCode128B validates printable ASCII input for Code Set B.
func NormalizeCode128B(data string) (string, error) {
	for _, ch := range data {
		if ch < 32 || ch > 126 {
			return "", fmt.Errorf("Code 128 Code Set B supports printable ASCII characters only")
		}
	}
	return data, nil
}

// ValueForCode128BChar converts a printable ASCII character into its Code 128 value.
func ValueForCode128BChar(char rune) int {
	return int(char) - 32
}

// ComputeCode128Checksum computes the weighted Code 128 checksum.
func ComputeCode128Checksum(values []int) int {
	sum := startB
	for index, value := range values {
		sum += value * (index + 1)
	}
	return sum % 103
}

func retagRuns(runs []BarcodeRun, role string) []BarcodeRun {
	result := make([]BarcodeRun, len(runs))
	for index, run := range runs {
		result[index] = run
		result[index].Role = role
		result[index].Metadata = copyMetadata(run.Metadata)
	}
	return result
}

// EncodeCode128B encodes a Code Set B payload into logical symbols.
func EncodeCode128B(data string) ([]EncodedSymbol, error) {
	normalized, err := NormalizeCode128B(data)
	if err != nil {
		return nil, err
	}
	dataSymbols := make([]EncodedSymbol, 0, len(normalized))
	values := make([]int, 0, len(normalized))
	for index, ch := range normalized {
		value := ValueForCode128BChar(ch)
		dataSymbols = append(dataSymbols, EncodedSymbol{
			Label:       string(ch),
			Value:       value,
			Pattern:     patterns[value],
			SourceIndex: index,
			Role:        "data",
		})
		values = append(values, value)
	}
	checksum := ComputeCode128Checksum(values)
	return append([]EncodedSymbol{{
		Label:       "Start B",
		Value:       startB,
		Pattern:     patterns[startB],
		SourceIndex: -1,
		Role:        "start",
	}}, append(dataSymbols, EncodedSymbol{
		Label:       fmt.Sprintf("Checksum %d", checksum),
		Value:       checksum,
		Pattern:     patterns[checksum],
		SourceIndex: len(normalized),
		Role:        "check",
	}, EncodedSymbol{
		Label:       "Stop",
		Value:       stop,
		Pattern:     patterns[stop],
		SourceIndex: len(normalized) + 1,
		Role:        "stop",
	})...), nil
}

// ExpandCode128Runs expands encoded symbols into barcode runs.
func ExpandCode128Runs(data string) ([]BarcodeRun, error) {
	encoded, err := EncodeCode128B(data)
	if err != nil {
		return nil, err
	}
	runs := make([]BarcodeRun, 0)
	for _, symbol := range encoded {
		segmentRuns, err := barcodelayout1d.RunsFromBinaryPattern(
			symbol.Pattern,
			'1',
			'0',
			symbol.Label,
			symbol.SourceIndex,
			nil,
		)
		if err != nil {
			return nil, err
		}
		runs = append(runs, retagRuns(segmentRuns, symbol.Role)...)
	}
	return runs, nil
}

// LayoutCode128 generates a paint scene for Code 128 Code Set B.
func LayoutCode128(data string, config RenderConfig) (paintinstructions.PaintScene, error) {
	normalized, err := NormalizeCode128B(data)
	if err != nil {
		return paintinstructions.PaintScene{}, err
	}
	runs, err := ExpandCode128Runs(normalized)
	if err != nil {
		return paintinstructions.PaintScene{}, err
	}
	encoded, err := EncodeCode128B(normalized)
	if err != nil {
		return paintinstructions.PaintScene{}, err
	}
	checksum := encoded[len(encoded)-2].Value
	return barcodelayout1d.DrawOneDimensionalBarcode(runs, config, barcodelayout1d.PaintOptions{
		Fill:       "#000000",
		Background: "#ffffff",
		Metadata: paintinstructions.Metadata{
			"symbology": "code128",
			"code_set":  "B",
			"checksum":  checksum,
		},
	})
}

// DrawCode128 is a compatibility alias for LayoutCode128.
func DrawCode128(data string, config RenderConfig) (paintinstructions.PaintScene, error) {
	return LayoutCode128(data, config)
}
