// Package code39 implements the Code 39 barcode symbology and emits generic draw scenes.
package code39

import (
	"fmt"
	"strings"

	drawinstructions "github.com/adhithyan15/coding-adventures/code/packages/go/draw-instructions"
)

const Version = "0.1.0"

type EncodedCharacter struct {
	Char        string
	IsStartStop bool
	Pattern     string
}

type BarcodeRun struct {
	Color               string
	Width               string
	SourceChar          string
	SourceIndex         int
	IsInterCharacterGap bool
}

type RenderConfig struct {
	NarrowUnit               int
	WideUnit                 int
	BarHeight                int
	QuietZoneUnits           int
	IncludeHumanReadableText bool
}

var DefaultRenderConfig = RenderConfig{4, 12, 120, 10, true}

const textMargin = 8
const textFontSize = 16
const textBlockHeight = textMargin + textFontSize + 4

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
	return StartNew[EncodedCharacter]("code39.EncodeCode39Char", EncodedCharacter{},
		func(op *Operation[EncodedCharacter], rf *ResultFactory[EncodedCharacter]) *OperationResult[EncodedCharacter] {
			op.AddProperty("char", char)
			pattern, ok := code39Patterns[char]
			if !ok {
				return rf.Fail(EncodedCharacter{}, fmt.Errorf(`invalid character: %q is not supported by Code 39`, char))
			}
			return rf.Generate(true, false, EncodedCharacter{Char: char, IsStartStop: char == "*", Pattern: widthPattern(pattern)})
		}).GetResult()
}

// EncodeCode39 encodes a data string into Code 39 characters.
func EncodeCode39(data string) ([]EncodedCharacter, error) {
	return StartNew[[]EncodedCharacter]("code39.EncodeCode39", nil,
		func(op *Operation[[]EncodedCharacter], rf *ResultFactory[[]EncodedCharacter]) *OperationResult[[]EncodedCharacter] {
			op.AddProperty("data", data)
			normalized, err := normalizeCode39(data)
			if err != nil {
				return rf.Fail(nil, err)
			}
			encoded := make([]EncodedCharacter, 0, len(normalized)+2)
			for _, ch := range "*" + normalized + "*" {
				item, err := EncodeCode39Char(string(ch))
				if err != nil {
					return rf.Fail(nil, err)
				}
				encoded = append(encoded, item)
			}
			return rf.Generate(true, false, encoded)
		}).GetResult()
}

// ExpandCode39Runs expands encoded characters into barcode runs.
func ExpandCode39Runs(data string) ([]BarcodeRun, error) {
	return StartNew[[]BarcodeRun]("code39.ExpandCode39Runs", nil,
		func(op *Operation[[]BarcodeRun], rf *ResultFactory[[]BarcodeRun]) *OperationResult[[]BarcodeRun] {
			op.AddProperty("data", data)
			encoded, err := EncodeCode39(data)
			if err != nil {
				return rf.Fail(nil, err)
			}
			colors := []string{"bar", "space", "bar", "space", "bar", "space", "bar", "space", "bar"}
			runs := make([]BarcodeRun, 0)
			for sourceIndex, encodedChar := range encoded {
				for elementIndex, element := range encodedChar.Pattern {
					width := "narrow"
					if string(element) == "W" {
						width = "wide"
					}
					runs = append(runs, BarcodeRun{
						Color: colors[elementIndex], Width: width, SourceChar: encodedChar.Char, SourceIndex: sourceIndex,
					})
				}
				if sourceIndex < len(encoded)-1 {
					runs = append(runs, BarcodeRun{
						Color: "space", Width: "narrow", SourceChar: encodedChar.Char, SourceIndex: sourceIndex, IsInterCharacterGap: true,
					})
				}
			}
			return rf.Generate(true, false, runs)
		}).GetResult()
}

func unitWidth(width string, config RenderConfig) int {
	if width == "wide" {
		return config.WideUnit
	}
	return config.NarrowUnit
}

// DrawOneDimensionalBarcode draws a generic 1D barcode from runs.
func DrawOneDimensionalBarcode(runs []BarcodeRun, textValue string, config RenderConfig) (drawinstructions.DrawScene, error) {
	return StartNew[drawinstructions.DrawScene]("code39.DrawOneDimensionalBarcode", drawinstructions.DrawScene{},
		func(op *Operation[drawinstructions.DrawScene], rf *ResultFactory[drawinstructions.DrawScene]) *OperationResult[drawinstructions.DrawScene] {
			op.AddProperty("textValue", textValue)
			if config.WideUnit <= config.NarrowUnit || config.NarrowUnit <= 0 || config.BarHeight <= 0 || config.QuietZoneUnits <= 0 {
				return rf.Fail(drawinstructions.DrawScene{}, fmt.Errorf("invalid render config"))
			}
			quietZoneWidth := config.QuietZoneUnits * config.NarrowUnit
			instructions := make([]drawinstructions.DrawInstruction, 0)
			cursorX := quietZoneWidth
			for _, run := range runs {
				width := unitWidth(run.Width, config)
				if run.Color == "bar" {
					instructions = append(instructions, drawinstructions.DrawRect(cursorX, 0, width, config.BarHeight, "#000000",
						drawinstructions.Metadata{"char": run.SourceChar, "index": run.SourceIndex}))
				}
				cursorX += width
			}
			if config.IncludeHumanReadableText && textValue != "" {
				instructions = append(instructions, drawinstructions.DrawText(
					(cursorX+quietZoneWidth)/2, config.BarHeight+textMargin+textFontSize-2, textValue,
					drawinstructions.Metadata{"role": "label"},
				))
			}
			height := config.BarHeight
			if config.IncludeHumanReadableText {
				height += textBlockHeight
			}
			scene := drawinstructions.CreateScene(cursorX+quietZoneWidth, height, instructions, "", drawinstructions.Metadata{
				"label": fmt.Sprintf("Code 39 barcode for %s", textValue), "symbology": "code39",
			})
			return rf.Generate(true, false, scene)
		}).GetResult()
}

// DrawCode39 generates a draw scene for a Code 39 barcode.
func DrawCode39(data string, config RenderConfig) (drawinstructions.DrawScene, error) {
	return StartNew[drawinstructions.DrawScene]("code39.DrawCode39", drawinstructions.DrawScene{},
		func(op *Operation[drawinstructions.DrawScene], rf *ResultFactory[drawinstructions.DrawScene]) *OperationResult[drawinstructions.DrawScene] {
			op.AddProperty("data", data)
			normalized, err := normalizeCode39(data)
			if err != nil {
				return rf.Fail(drawinstructions.DrawScene{}, err)
			}
			runs, err := ExpandCode39Runs(normalized)
			if err != nil {
				return rf.Fail(drawinstructions.DrawScene{}, err)
			}
			scene, err := DrawOneDimensionalBarcode(runs, normalized, config)
			if err != nil {
				return rf.Fail(drawinstructions.DrawScene{}, err)
			}
			return rf.Generate(true, false, scene)
		}).GetResult()
}

// RenderCode39 renders a Code 39 barcode using the given renderer.
// Note: This is a generic function that delegates to DrawCode39 internally.
func RenderCode39[T any](data string, renderer drawinstructions.Renderer[T], config RenderConfig) (T, error) {
	scene, err := DrawCode39(data, config)
	if err != nil {
		var zero T
		return zero, err
	}
	return renderer.Render(scene), nil
}
