// Package barcodelayout1d translates logical barcode runs into PaintScene.
package barcodelayout1d

import (
	"fmt"

	paintinstructions "github.com/adhithyan15/coding-adventures/code/packages/go/paint-instructions"
)

const Version = "0.1.0"

type Barcode1DRun struct {
	Color       string
	Modules     int
	SourceChar  string
	SourceIndex int
	Role        string
	Metadata    paintinstructions.Metadata
}

type LayoutConfig struct {
	ModuleUnit       int
	BarHeight        int
	QuietZoneModules int
}

type PaintOptions struct {
	Fill       string
	Background string
	Metadata   paintinstructions.Metadata
}

var DefaultLayoutConfig = LayoutConfig{
	ModuleUnit:       4,
	BarHeight:        120,
	QuietZoneModules: 10,
}

var DefaultPaintOptions = PaintOptions{
	Fill:       "#000000",
	Background: "#ffffff",
	Metadata:   paintinstructions.Metadata{},
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

func validateLayoutConfig(config LayoutConfig) error {
	if config.ModuleUnit <= 0 {
		return fmt.Errorf("module_unit must be a positive integer")
	}
	if config.BarHeight <= 0 {
		return fmt.Errorf("bar_height must be a positive integer")
	}
	if config.QuietZoneModules < 0 {
		return fmt.Errorf("quiet_zone_modules must be zero or a positive integer")
	}
	return nil
}

func validateRun(run Barcode1DRun) error {
	if run.Color != "bar" && run.Color != "space" {
		return fmt.Errorf("run color must be 'bar' or 'space'")
	}
	if run.Modules <= 0 {
		return fmt.Errorf("run modules must be a positive integer")
	}
	return nil
}

func RunsFromBinaryPattern(pattern string, barChar rune, spaceChar rune, sourceChar string, sourceIndex int, metadata paintinstructions.Metadata) ([]Barcode1DRun, error) {
	if pattern == "" {
		return []Barcode1DRun{}, nil
	}
	runes := []rune(pattern)
	current := runes[0]
	count := 1
	runs := make([]Barcode1DRun, 0)

	flush := func(token rune, modules int) error {
		color := ""
		switch token {
		case barChar:
			color = "bar"
		case spaceChar:
			color = "space"
		default:
			return fmt.Errorf("binary pattern contains unsupported token: %q", string(token))
		}
		runs = append(runs, Barcode1DRun{
			Color:       color,
			Modules:     modules,
			SourceChar:  sourceChar,
			SourceIndex: sourceIndex,
			Role:        "data",
			Metadata:    copyMetadata(metadata),
		})
		return nil
	}

	for _, token := range runes[1:] {
		if token == current {
			count++
			continue
		}
		if err := flush(current, count); err != nil {
			return nil, err
		}
		current = token
		count = 1
	}
	if err := flush(current, count); err != nil {
		return nil, err
	}
	return runs, nil
}

func RunsFromWidthPattern(pattern string, colors []string, sourceChar string, sourceIndex int, narrowModules int, wideModules int, role string, metadata paintinstructions.Metadata) ([]Barcode1DRun, error) {
	if len(pattern) != len(colors) {
		return nil, fmt.Errorf("pattern length must match colors length")
	}
	if narrowModules <= 0 || wideModules <= 0 {
		return nil, fmt.Errorf("narrow_modules and wide_modules must be positive integers")
	}
	runs := make([]Barcode1DRun, 0, len(pattern))
	for index, token := range pattern {
		modules := 0
		switch token {
		case 'N':
			modules = narrowModules
		case 'W':
			modules = wideModules
		default:
			return nil, fmt.Errorf("width pattern contains unsupported token: %q", string(token))
		}
		runs = append(runs, Barcode1DRun{
			Color:       colors[index],
			Modules:     modules,
			SourceChar:  sourceChar,
			SourceIndex: sourceIndex,
			Role:        role,
			Metadata:    copyMetadata(metadata),
		})
	}
	return runs, nil
}

func LayoutBarcode1D(runs []Barcode1DRun, config LayoutConfig, options PaintOptions) (paintinstructions.PaintScene, error) {
	if err := validateLayoutConfig(config); err != nil {
		return paintinstructions.PaintScene{}, err
	}
	quietZoneWidth := config.QuietZoneModules * config.ModuleUnit
	cursorX := quietZoneWidth
	instructions := make([]paintinstructions.PaintInstruction, 0)

	for _, run := range runs {
		if err := validateRun(run); err != nil {
			return paintinstructions.PaintScene{}, err
		}
		width := run.Modules * config.ModuleUnit
		if run.Color == "bar" {
			metadata := copyMetadata(run.Metadata)
			metadata["source_char"] = run.SourceChar
			metadata["source_index"] = run.SourceIndex
			metadata["modules"] = run.Modules
			metadata["role"] = run.Role
			instructions = append(instructions, paintinstructions.PaintRect(
				cursorX, 0, width, config.BarHeight, options.Fill, metadata,
			))
		}
		cursorX += width
	}

	metadata := copyMetadata(options.Metadata)
	metadata["content_width"] = cursorX - quietZoneWidth
	metadata["quiet_zone_width"] = quietZoneWidth
	metadata["module_unit"] = config.ModuleUnit
	metadata["bar_height"] = config.BarHeight

	return paintinstructions.CreateScene(
		cursorX+quietZoneWidth,
		config.BarHeight,
		instructions,
		options.Background,
		metadata,
	), nil
}

func DrawOneDimensionalBarcode(runs []Barcode1DRun, config LayoutConfig, options PaintOptions) (paintinstructions.PaintScene, error) {
	return LayoutBarcode1D(runs, config, options)
}
