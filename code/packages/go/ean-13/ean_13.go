// Package ean13 implements the EAN-13 barcode symbology and emits PaintScene.
package ean13

import (
	"fmt"
	"strings"

	barcodelayout1d "github.com/adhithyan15/coding-adventures/code/packages/go/barcode-layout-1d"
	paintinstructions "github.com/adhithyan15/coding-adventures/code/packages/go/paint-instructions"
)

const Version = "0.1.0"

type EncodedDigit struct {
	Digit       string
	Encoding    string
	Pattern     string
	SourceIndex int
	Role        string
}

type BarcodeRun = barcodelayout1d.Barcode1DRun
type RenderConfig = barcodelayout1d.LayoutConfig

var DefaultLayoutConfig = barcodelayout1d.DefaultLayoutConfig
var DefaultRenderConfig = DefaultLayoutConfig

const sideGuard = "101"
const centerGuard = "01010"

var digitPatterns = map[string][]string{
	"L": {"0001101", "0011001", "0010011", "0111101", "0100011", "0110001", "0101111", "0111011", "0110111", "0001011"},
	"G": {"0100111", "0110011", "0011011", "0100001", "0011101", "0111001", "0000101", "0010001", "0001001", "0010111"},
	"R": {"1110010", "1100110", "1101100", "1000010", "1011100", "1001110", "1010000", "1000100", "1001000", "1110100"},
}

var leftParityPatterns = []string{
	"LLLLLL", "LLGLGG", "LLGGLG", "LLGGGL", "LGLLGG",
	"LGGLLG", "LGGGLL", "LGLGLG", "LGLGGL", "LGGLGL",
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

func assertDigits(data string, allowedLengths ...int) error {
	if data == "" {
		return fmt.Errorf("EAN-13 input must contain digits only")
	}
	for _, ch := range data {
		if ch < '0' || ch > '9' {
			return fmt.Errorf("EAN-13 input must contain digits only")
		}
	}
	for _, length := range allowedLengths {
		if len(data) == length {
			return nil
		}
	}
	return fmt.Errorf("EAN-13 input must contain 12 digits or 13 digits")
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

// ComputeEAN13CheckDigit computes the check digit for a 12-digit payload.
func ComputeEAN13CheckDigit(payload12 string) (string, error) {
	if err := assertDigits(payload12, 12); err != nil {
		return "", err
	}
	total := 0
	reversed := []rune(payload12)
	for index := len(reversed) - 1; index >= 0; index-- {
		position := len(reversed) - 1 - index
		multiplier := 1
		if position%2 == 0 {
			multiplier = 3
		}
		total += int(reversed[index]-'0') * multiplier
	}
	return fmt.Sprintf("%d", (10-(total%10))%10), nil
}

// NormalizeEAN13 normalizes EAN-13 payloads and computes the check digit when needed.
func NormalizeEAN13(data string) (string, error) {
	if err := assertDigits(data, 12, 13); err != nil {
		return "", err
	}
	if len(data) == 12 {
		checkDigit, err := ComputeEAN13CheckDigit(data)
		if err != nil {
			return "", err
		}
		return data + checkDigit, nil
	}
	expected, err := ComputeEAN13CheckDigit(data[:12])
	if err != nil {
		return "", err
	}
	if expected != data[12:] {
		return "", fmt.Errorf("invalid EAN-13 check digit: expected %s but received %s", expected, data[12:])
	}
	return data, nil
}

// LeftParityPattern returns the left-side parity pattern for a normalized EAN-13 value.
func LeftParityPattern(data string) (string, error) {
	normalized, err := NormalizeEAN13(data)
	if err != nil {
		return "", err
	}
	return leftParityPatterns[int(normalized[0]-'0')], nil
}

// EncodeEAN13 encodes an EAN-13 payload into logical digits.
func EncodeEAN13(data string) ([]EncodedDigit, error) {
	normalized, err := NormalizeEAN13(data)
	if err != nil {
		return nil, err
	}
	parity, err := LeftParityPattern(normalized)
	if err != nil {
		return nil, err
	}
	digits := strings.Split(normalized, "")
	encoded := make([]EncodedDigit, 0, 12)
	for offset, digit := range digits[1:7] {
		encoding := string(parity[offset])
		encoded = append(encoded, EncodedDigit{
			Digit:       digit,
			Encoding:    encoding,
			Pattern:     digitPatterns[encoding][int(digit[0]-'0')],
			SourceIndex: offset + 1,
			Role:        "data",
		})
	}
	for offset, digit := range digits[7:] {
		role := "data"
		if offset == 5 {
			role = "check"
		}
		encoded = append(encoded, EncodedDigit{
			Digit:       digit,
			Encoding:    "R",
			Pattern:     digitPatterns["R"][int(digit[0]-'0')],
			SourceIndex: offset + 7,
			Role:        role,
		})
	}
	return encoded, nil
}

// ExpandEAN13Runs expands an encoded EAN-13 payload into barcode runs.
func ExpandEAN13Runs(data string) ([]BarcodeRun, error) {
	encoded, err := EncodeEAN13(data)
	if err != nil {
		return nil, err
	}
	runs := make([]BarcodeRun, 0)
	startRuns, err := barcodelayout1d.RunsFromBinaryPattern(sideGuard, '1', '0', "start", -1, nil)
	if err != nil {
		return nil, err
	}
	runs = append(runs, retagRuns(startRuns, "guard")...)
	for _, entry := range encoded[:6] {
		segmentRuns, err := barcodelayout1d.RunsFromBinaryPattern(entry.Pattern, '1', '0', entry.Digit, entry.SourceIndex, nil)
		if err != nil {
			return nil, err
		}
		runs = append(runs, retagRuns(segmentRuns, entry.Role)...)
	}
	centerRuns, err := barcodelayout1d.RunsFromBinaryPattern(centerGuard, '1', '0', "center", -2, nil)
	if err != nil {
		return nil, err
	}
	runs = append(runs, retagRuns(centerRuns, "guard")...)
	for _, entry := range encoded[6:] {
		segmentRuns, err := barcodelayout1d.RunsFromBinaryPattern(entry.Pattern, '1', '0', entry.Digit, entry.SourceIndex, nil)
		if err != nil {
			return nil, err
		}
		runs = append(runs, retagRuns(segmentRuns, entry.Role)...)
	}
	endRuns, err := barcodelayout1d.RunsFromBinaryPattern(sideGuard, '1', '0', "end", -3, nil)
	if err != nil {
		return nil, err
	}
	runs = append(runs, retagRuns(endRuns, "guard")...)
	return runs, nil
}

// LayoutEAN13 generates a paint scene for EAN-13.
func LayoutEAN13(data string, config RenderConfig) (paintinstructions.PaintScene, error) {
	normalized, err := NormalizeEAN13(data)
	if err != nil {
		return paintinstructions.PaintScene{}, err
	}
	parity, err := LeftParityPattern(normalized)
	if err != nil {
		return paintinstructions.PaintScene{}, err
	}
	runs, err := ExpandEAN13Runs(normalized)
	if err != nil {
		return paintinstructions.PaintScene{}, err
	}
	return barcodelayout1d.DrawOneDimensionalBarcode(runs, config, barcodelayout1d.PaintOptions{
		Fill:       "#000000",
		Background: "#ffffff",
		Metadata: paintinstructions.Metadata{
			"symbology":       "ean-13",
			"leading_digit":   string(normalized[0]),
			"left_parity":     parity,
			"content_modules": 95,
		},
	})
}

// DrawEAN13 is a compatibility alias for LayoutEAN13.
func DrawEAN13(data string, config RenderConfig) (paintinstructions.PaintScene, error) {
	return LayoutEAN13(data, config)
}
