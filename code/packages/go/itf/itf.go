// Package itf implements the ITF barcode symbology and emits PaintScene.
package itf

import (
	"fmt"

	barcodelayout1d "github.com/adhithyan15/coding-adventures/code/packages/go/barcode-layout-1d"
	paintinstructions "github.com/adhithyan15/coding-adventures/code/packages/go/paint-instructions"
)

const Version = "0.1.0"

const startPattern = "1010"
const stopPattern = "11101"

type EncodedPair struct {
	Pair          string
	BarPattern    string
	SpacePattern  string
	BinaryPattern string
	SourceIndex   int
}

type BarcodeRun = barcodelayout1d.Barcode1DRun
type RenderConfig = barcodelayout1d.LayoutConfig

var DefaultLayoutConfig = barcodelayout1d.DefaultLayoutConfig
var DefaultRenderConfig = DefaultLayoutConfig

var digitPatterns = []string{
	"00110", "10001", "01001", "11000", "00101",
	"10100", "01100", "00011", "10010", "01010",
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

func retagRuns(runs []BarcodeRun, role string) []BarcodeRun {
	result := make([]BarcodeRun, len(runs))
	for index, run := range runs {
		result[index] = run
		result[index].Role = role
		result[index].Metadata = copyMetadata(run.Metadata)
	}
	return result
}

// NormalizeITF validates even-length digit strings.
func NormalizeITF(data string) (string, error) {
	if data == "" {
		return "", fmt.Errorf("ITF input must contain an even number of digits")
	}
	for _, ch := range data {
		if ch < '0' || ch > '9' {
			return "", fmt.Errorf("ITF input must contain digits only")
		}
	}
	if len(data)%2 != 0 {
		return "", fmt.Errorf("ITF input must contain an even number of digits")
	}
	return data, nil
}

// EncodeITF encodes an ITF payload into digit pairs.
func EncodeITF(data string) ([]EncodedPair, error) {
	normalized, err := NormalizeITF(data)
	if err != nil {
		return nil, err
	}
	encoded := make([]EncodedPair, 0, len(normalized)/2)
	for index := 0; index < len(normalized); index += 2 {
		pair := normalized[index : index+2]
		barPattern := digitPatterns[int(pair[0]-'0')]
		spacePattern := digitPatterns[int(pair[1]-'0')]
		binaryPattern := ""
		for offset := range barPattern {
			barToken := "1"
			if barPattern[offset] == '1' {
				barToken = "111"
			}
			spaceToken := "0"
			if spacePattern[offset] == '1' {
				spaceToken = "000"
			}
			binaryPattern += barToken + spaceToken
		}
		encoded = append(encoded, EncodedPair{
			Pair:          pair,
			BarPattern:    barPattern,
			SpacePattern:  spacePattern,
			BinaryPattern: binaryPattern,
			SourceIndex:   index / 2,
		})
	}
	return encoded, nil
}

// ExpandITFRuns expands encoded digit pairs into barcode runs.
func ExpandITFRuns(data string) ([]BarcodeRun, error) {
	encoded, err := EncodeITF(data)
	if err != nil {
		return nil, err
	}
	runs := make([]BarcodeRun, 0)
	startRuns, err := barcodelayout1d.RunsFromBinaryPattern(startPattern, '1', '0', "start", -1, nil)
	if err != nil {
		return nil, err
	}
	runs = append(runs, retagRuns(startRuns, "start")...)
	for _, entry := range encoded {
		segmentRuns, err := barcodelayout1d.RunsFromBinaryPattern(entry.BinaryPattern, '1', '0', entry.Pair, entry.SourceIndex, nil)
		if err != nil {
			return nil, err
		}
		runs = append(runs, retagRuns(segmentRuns, "data")...)
	}
	stopRuns, err := barcodelayout1d.RunsFromBinaryPattern(stopPattern, '1', '0', "stop", -2, nil)
	if err != nil {
		return nil, err
	}
	runs = append(runs, retagRuns(stopRuns, "stop")...)
	return runs, nil
}

// LayoutITF generates a paint scene for ITF.
func LayoutITF(data string, config RenderConfig) (paintinstructions.PaintScene, error) {
	normalized, err := NormalizeITF(data)
	if err != nil {
		return paintinstructions.PaintScene{}, err
	}
	runs, err := ExpandITFRuns(normalized)
	if err != nil {
		return paintinstructions.PaintScene{}, err
	}
	return barcodelayout1d.DrawOneDimensionalBarcode(runs, config, barcodelayout1d.PaintOptions{
		Fill:       "#000000",
		Background: "#ffffff",
		Metadata: paintinstructions.Metadata{
			"symbology":  "itf",
			"pair_count": len(normalized) / 2,
		},
	})
}

// DrawITF is a compatibility alias for LayoutITF.
func DrawITF(data string, config RenderConfig) (paintinstructions.PaintScene, error) {
	return LayoutITF(data, config)
}
