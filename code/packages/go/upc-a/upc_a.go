// Package upca implements the UPC-A barcode symbology and emits PaintScene.
package upca

import (
	"fmt"

	barcodelayout1d "github.com/adhithyan15/coding-adventures/code/packages/go/barcode-layout-1d"
	paintinstructions "github.com/adhithyan15/coding-adventures/code/packages/go/paint-instructions"
)

const Version = "0.1.0"

const sideGuard = "101"
const centerGuard = "01010"

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

var digitPatterns = map[string][]string{
	"L": {"0001101", "0011001", "0010011", "0111101", "0100011", "0110001", "0101111", "0111011", "0110111", "0001011"},
	"R": {"1110010", "1100110", "1101100", "1000010", "1011100", "1001110", "1010000", "1000100", "1001000", "1110100"},
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
		return fmt.Errorf("UPC-A input must contain digits only")
	}
	for _, ch := range data {
		if ch < '0' || ch > '9' {
			return fmt.Errorf("UPC-A input must contain digits only")
		}
	}
	for _, length := range allowedLengths {
		if len(data) == length {
			return nil
		}
	}
	return fmt.Errorf("UPC-A input must contain 11 digits or 12 digits")
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

// ComputeUPCACheckDigit computes the check digit for an 11-digit payload.
func ComputeUPCACheckDigit(payload11 string) (string, error) {
	if err := assertDigits(payload11, 11); err != nil {
		return "", err
	}
	oddSum := 0
	evenSum := 0
	for index, ch := range payload11 {
		if index%2 == 0 {
			oddSum += int(ch - '0')
		} else {
			evenSum += int(ch - '0')
		}
	}
	return fmt.Sprintf("%d", (10-(((oddSum*3)+evenSum)%10))%10), nil
}

// NormalizeUPCA normalizes UPC-A payloads and computes the check digit when needed.
func NormalizeUPCA(data string) (string, error) {
	if err := assertDigits(data, 11, 12); err != nil {
		return "", err
	}
	if len(data) == 11 {
		checkDigit, err := ComputeUPCACheckDigit(data)
		if err != nil {
			return "", err
		}
		return data + checkDigit, nil
	}
	expected, err := ComputeUPCACheckDigit(data[:11])
	if err != nil {
		return "", err
	}
	if expected != data[11:] {
		return "", fmt.Errorf("invalid UPC-A check digit: expected %s but received %s", expected, data[11:])
	}
	return data, nil
}

// EncodeUPCA encodes a UPC-A payload into logical digits.
func EncodeUPCA(data string) ([]EncodedDigit, error) {
	normalized, err := NormalizeUPCA(data)
	if err != nil {
		return nil, err
	}
	encoded := make([]EncodedDigit, 0, len(normalized))
	for index, ch := range normalized {
		encoding := "R"
		if index < 6 {
			encoding = "L"
		}
		role := "data"
		if index == 11 {
			role = "check"
		}
		encoded = append(encoded, EncodedDigit{
			Digit:       string(ch),
			Encoding:    encoding,
			Pattern:     digitPatterns[encoding][int(ch-'0')],
			SourceIndex: index,
			Role:        role,
		})
	}
	return encoded, nil
}

// ExpandUPCARuns expands an encoded UPC-A payload into barcode runs.
func ExpandUPCARuns(data string) ([]BarcodeRun, error) {
	encoded, err := EncodeUPCA(data)
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

// LayoutUPCA generates a paint scene for UPC-A.
func LayoutUPCA(data string, config RenderConfig) (paintinstructions.PaintScene, error) {
	normalized, err := NormalizeUPCA(data)
	if err != nil {
		return paintinstructions.PaintScene{}, err
	}
	runs, err := ExpandUPCARuns(normalized)
	if err != nil {
		return paintinstructions.PaintScene{}, err
	}
	return barcodelayout1d.DrawOneDimensionalBarcode(runs, config, barcodelayout1d.PaintOptions{
		Fill:       "#000000",
		Background: "#ffffff",
		Metadata: paintinstructions.Metadata{
			"symbology":       "upc-a",
			"content_modules": 95,
		},
	})
}

// DrawUPCA is a compatibility alias for LayoutUPCA.
func DrawUPCA(data string, config RenderConfig) (paintinstructions.PaintScene, error) {
	return LayoutUPCA(data, config)
}
