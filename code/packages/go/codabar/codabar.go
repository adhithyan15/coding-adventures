// Package codabar implements the Codabar barcode symbology and emits PaintScene.
package codabar

import (
	"fmt"
	"strings"

	barcodelayout1d "github.com/adhithyan15/coding-adventures/code/packages/go/barcode-layout-1d"
	paintinstructions "github.com/adhithyan15/coding-adventures/code/packages/go/paint-instructions"
)

const Version = "0.1.0"

type EncodedSymbol struct {
	Char        string
	Pattern     string
	SourceIndex int
	Role        string
}

type BarcodeRun = barcodelayout1d.Barcode1DRun
type RenderConfig = barcodelayout1d.LayoutConfig

var DefaultLayoutConfig = barcodelayout1d.DefaultLayoutConfig
var DefaultRenderConfig = DefaultLayoutConfig

var guards = map[string]struct{}{"A": {}, "B": {}, "C": {}, "D": {}}

var patterns = map[string]string{
	"0": "101010011",
	"1": "101011001",
	"2": "101001011",
	"3": "110010101",
	"4": "101101001",
	"5": "110101001",
	"6": "100101011",
	"7": "100101101",
	"8": "100110101",
	"9": "110100101",
	"-": "101001101",
	"$": "101100101",
	":": "1101011011",
	"/": "1101101011",
	".": "1101101101",
	"+": "1011011011",
	"A": "1011001001",
	"B": "1001001011",
	"C": "1010010011",
	"D": "1010011001",
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

func isGuard(char string) bool {
	_, ok := guards[char]
	return ok
}

func assertBodyChars(body string) error {
	for _, ch := range body {
		value := string(ch)
		_, ok := patterns[value]
		if !ok || isGuard(value) {
			return fmt.Errorf("invalid Codabar body character %q", value)
		}
	}
	return nil
}

// NormalizeCodabar normalizes payloads and adds default A/A guards when needed.
func NormalizeCodabar(data string) (string, error) {
	normalized := strings.ToUpper(data)
	if len(normalized) >= 2 && isGuard(string(normalized[0])) && isGuard(string(normalized[len(normalized)-1])) {
		if err := assertBodyChars(normalized[1 : len(normalized)-1]); err != nil {
			return "", err
		}
		return normalized, nil
	}
	if err := assertBodyChars(normalized); err != nil {
		return "", err
	}
	return "A" + normalized + "A", nil
}

// EncodeCodabar encodes a payload into logical symbols.
func EncodeCodabar(data string) ([]EncodedSymbol, error) {
	normalized, err := NormalizeCodabar(data)
	if err != nil {
		return nil, err
	}
	encoded := make([]EncodedSymbol, 0, len(normalized))
	for index, ch := range normalized {
		char := string(ch)
		role := "data"
		if index == 0 {
			role = "start"
		} else if index == len(normalized)-1 {
			role = "stop"
		}
		encoded = append(encoded, EncodedSymbol{
			Char:        char,
			Pattern:     patterns[char],
			SourceIndex: index,
			Role:        role,
		})
	}
	return encoded, nil
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

// ExpandCodabarRuns expands encoded symbols into barcode runs.
func ExpandCodabarRuns(data string) ([]BarcodeRun, error) {
	encoded, err := EncodeCodabar(data)
	if err != nil {
		return nil, err
	}
	runs := make([]BarcodeRun, 0)
	for index, symbol := range encoded {
		segmentRuns, err := barcodelayout1d.RunsFromBinaryPattern(
			symbol.Pattern,
			'1',
			'0',
			symbol.Char,
			symbol.SourceIndex,
			nil,
		)
		if err != nil {
			return nil, err
		}
		runs = append(runs, retagRuns(segmentRuns, symbol.Role)...)
		if index < len(encoded)-1 {
			runs = append(runs, BarcodeRun{
				Color:       "space",
				Modules:     1,
				SourceChar:  symbol.Char,
				SourceIndex: symbol.SourceIndex,
				Role:        "inter-character-gap",
				Metadata:    paintinstructions.Metadata{},
			})
		}
	}
	return runs, nil
}

// LayoutCodabar generates a paint scene for Codabar.
func LayoutCodabar(data string, config RenderConfig) (paintinstructions.PaintScene, error) {
	normalized, err := NormalizeCodabar(data)
	if err != nil {
		return paintinstructions.PaintScene{}, err
	}
	runs, err := ExpandCodabarRuns(normalized)
	if err != nil {
		return paintinstructions.PaintScene{}, err
	}
	return barcodelayout1d.DrawOneDimensionalBarcode(runs, config, barcodelayout1d.PaintOptions{
		Fill:       "#000000",
		Background: "#ffffff",
		Metadata: paintinstructions.Metadata{
			"symbology": "codabar",
			"start":     string(normalized[0]),
			"stop":      string(normalized[len(normalized)-1]),
		},
	})
}

// DrawCodabar is a compatibility alias for LayoutCodabar.
func DrawCodabar(data string, config RenderConfig) (paintinstructions.PaintScene, error) {
	return LayoutCodabar(data, config)
}
