// Package code39 implements the Code 39 barcode symbology and emits PaintScene.
package code39

import (
	"fmt"
	"strings"

	barcodelayout1d "github.com/adhithyan15/coding-adventures/code/packages/go/barcode-layout-1d"
	paintinstructions "github.com/adhithyan15/coding-adventures/code/packages/go/paint-instructions"
)

const Version = "0.1.0"

type EncodedCharacter struct {
	Char        string
	IsStartStop bool
	Pattern     string
}

type BarcodeRun = barcodelayout1d.Barcode1DRun
type RenderConfig = barcodelayout1d.LayoutConfig

var DefaultLayoutConfig = barcodelayout1d.DefaultLayoutConfig
var DefaultRenderConfig = DefaultLayoutConfig

var code39Patterns = map[string]string{
	"0": "bwbWBwBwb", "1": "BwbWbwbwB", "2": "bwBWbwbwB", "3": "BwBWbwbwb",
	"4": "bwbWBwbwB", "5": "BwbWBwbwb", "6": "bwBWBwbwb", "7": "bwbWbwBwB",
	"8": "BwbWbwBwb", "9": "bwBWbwBwb", "A": "BwbwbWbwB", "B": "bwBwbWbwB",
	"C": "BwBwbWbwb", "D": "bwbwBWbwB", "E": "BwbwBWbwb", "F": "bwBwBWbwb",
	"G": "bwbwbWBwB", "H": "BwbwbWBwb", "I": "bwBwbWBwb", "J": "bwbwBWBwb",
	"K": "BwbwbwbWB", "L": "bwBwbwbWB", "M": "BwBwbwbWb", "N": "bwbwBwbWB",
	"O": "BwbwBwbWb", "P": "bwBwBwbWb", "Q": "bwbwbwBWB", "R": "BwbwbwBWb",
	"S": "bwBwbwBWb", "T": "bwbwBwBWb", "U": "BWbwbwbwB", "V": "bWBwbwbwB",
	"W": "BWBwbwbwb", "X": "bWbwBwbwB", "Y": "BWbwBwbwb", "Z": "bWBwBwbwb",
	"-": "bWbwbwBwB", ".": "BWbwbwBwb", " ": "bWBwbwBwb", "$": "bWbWbWbwb",
	"/": "bWbWbwbWb", "+": "bWbwbWbWb", "%": "bwbWbWbWb", "*": "bWbwBwBwb",
}

var barSpaceColors = []string{"bar", "space", "bar", "space", "bar", "space", "bar", "space", "bar"}

func widthPattern(pattern string) string {
	var builder strings.Builder
	for _, ch := range pattern {
		if strings.ToUpper(string(ch)) == string(ch) {
			builder.WriteString("W")
		} else {
			builder.WriteString("N")
		}
	}
	return builder.String()
}

func normalizeCode39(data string) (string, error) {
	normalized := strings.ToUpper(data)
	for _, ch := range normalized {
		value := string(ch)
		if value == "*" {
			return "", fmt.Errorf(`input must not contain "*" because it is reserved for start/stop`)
		}
		if _, ok := code39Patterns[value]; !ok {
			return "", fmt.Errorf(`invalid character: %q is not supported by Code 39`, value)
		}
	}
	return normalized, nil
}

// EncodeCode39Char encodes a single character.
func EncodeCode39Char(char string) (EncodedCharacter, error) {
	pattern, ok := code39Patterns[char]
	if !ok {
		return EncodedCharacter{}, fmt.Errorf(`invalid character: %q is not supported by Code 39`, char)
	}
	return EncodedCharacter{Char: char, IsStartStop: char == "*", Pattern: widthPattern(pattern)}, nil
}

// EncodeCode39 encodes a data string into Code 39 characters.
func EncodeCode39(data string) ([]EncodedCharacter, error) {
	normalized, err := normalizeCode39(data)
	if err != nil {
		return nil, err
	}
	encoded := make([]EncodedCharacter, 0, len(normalized)+2)
	for _, ch := range "*" + normalized + "*" {
		item, err := EncodeCode39Char(string(ch))
		if err != nil {
			return nil, err
		}
		encoded = append(encoded, item)
	}
	return encoded, nil
}

// ExpandCode39Runs expands encoded characters into barcode runs.
func ExpandCode39Runs(data string) ([]BarcodeRun, error) {
	encoded, err := EncodeCode39(data)
	if err != nil {
		return nil, err
	}
	runs := make([]BarcodeRun, 0)
	for sourceIndex, encodedChar := range encoded {
		segmentRuns, err := barcodelayout1d.RunsFromWidthPattern(
			encodedChar.Pattern,
			barSpaceColors,
			encodedChar.Char,
			sourceIndex,
			1,
			3,
			"data",
			nil,
		)
		if err != nil {
			return nil, err
		}
		runs = append(runs, segmentRuns...)
		if sourceIndex < len(encoded)-1 {
			runs = append(runs, BarcodeRun{
				Color:       "space",
				Modules:     1,
				SourceChar:  encodedChar.Char,
				SourceIndex: sourceIndex,
				Role:        "inter-character-gap",
				Metadata:    paintinstructions.Metadata{},
			})
		}
	}
	return runs, nil
}

// DrawOneDimensionalBarcode lays out a generic 1D barcode from runs.
func DrawOneDimensionalBarcode(runs []BarcodeRun, config RenderConfig, metadata paintinstructions.Metadata) (paintinstructions.PaintScene, error) {
	return barcodelayout1d.DrawOneDimensionalBarcode(runs, config, barcodelayout1d.PaintOptions{
		Fill:       "#000000",
		Background: "#ffffff",
		Metadata:   metadata,
	})
}

// LayoutCode39 generates a paint scene for a Code 39 barcode.
func LayoutCode39(data string, config RenderConfig) (paintinstructions.PaintScene, error) {
	normalized, err := normalizeCode39(data)
	if err != nil {
		return paintinstructions.PaintScene{}, err
	}
	runs, err := ExpandCode39Runs(normalized)
	if err != nil {
		return paintinstructions.PaintScene{}, err
	}
	return DrawOneDimensionalBarcode(runs, config, paintinstructions.Metadata{
		"symbology": "code39",
		"data":      normalized,
	})
}

// DrawCode39 is kept as a compatibility alias for LayoutCode39.
func DrawCode39(data string, config RenderConfig) (paintinstructions.PaintScene, error) {
	return LayoutCode39(data, config)
}
