package barcodelayout1d

import (
	"math"
	"strconv"
	"unicode/utf8"

	paintinstructions "github.com/adhithyan15/coding-adventures/code/packages/go/paint-instructions"
)

const (
	maxPatternScalarsV1   = 65567
	maxRunsV1             = 40979
	maxContentModulesV1   = int64(65567)
	maxQuietZoneModulesV1 = int64(4096)
	maxSymbolsV1          = 40979
	maxLabelScalarsV1     = 4096
	maxMetadataEntriesV1  = 64
	maxMetadataKeyV1      = 128
	maxMetadataValueV1    = 4096
	maxMetadataBytesV1    = 65536
	maxRenderDimensionV1  = int64(8192)
	maxColorScalarsV1     = 128
)

// Barcode1DV1Error is the payload-blind error returned by the portable v1 API.
type Barcode1DV1Error struct{ ID string }

func (e *Barcode1DV1Error) Error() string { return e.ID }

type Barcode1DV1Run struct {
	Color, SourceLabel, Role string
	Modules, SourceIndex     int64
}

type Barcode1DV1SymbolDescriptor struct {
	Label, Role          string
	Modules, SourceIndex int64
}

type Barcode1DV1SymbolLayout struct {
	Label, Role            string
	StartModule, EndModule int64
	SourceIndex            int32
}

type Barcode1DV1Layout struct {
	LeftQuietZoneModules, RightQuietZoneModules int64
	ContentModules, TotalModules                int64
	SymbolLayouts                               []Barcode1DV1SymbolLayout
}

type Barcode1DV1BinaryOptions struct {
	SourceLabel string
	SourceIndex int64
	Role        string
}

type Barcode1DV1WidthOptions struct {
	SourceLabel, Role          string
	SourceIndex                int64
	NarrowMarker, WideMarker   *string
	NarrowModules, WideModules *int64
	StartingColor              *string
}

type Barcode1DV1RenderConfig struct {
	ModuleWidth, BarHeight   *int64
	Foreground, Background   *string
	IncludeHumanReadableText bool
}

type Barcode1DV1SceneOptions struct {
	QuietZoneModules  *int64
	RenderConfig      *Barcode1DV1RenderConfig
	HumanReadableText *string
	Label             *string
	Metadata          map[string]string
	Symbols           []Barcode1DV1SymbolDescriptor
}

func failV1(id string) error { return &Barcode1DV1Error{ID: id} }

func ExpandBinaryV1(pattern string, options Barcode1DV1BinaryOptions) ([]Barcode1DV1Run, error) {
	count, ok := scalarCountV1(pattern)
	if !ok {
		return nil, failV1("invalid-binary-token")
	}
	if count > maxPatternScalarsV1 {
		return nil, failV1("pattern-too-long")
	}
	if count == 0 {
		return nil, failV1("empty-pattern")
	}
	if err := validateSourceV1(options.SourceLabel, options.SourceIndex); err != nil {
		return nil, err
	}
	if !validRoleV1(options.Role) {
		return nil, failV1("invalid-source-attribution")
	}

	runes := []rune(pattern)
	runs := make([]Barcode1DV1Run, 0)
	current, width := runes[0], int64(1)
	appendRun := func(token rune, modules int64) error {
		color := ""
		if token == '1' {
			color = "bar"
		} else if token == '0' {
			color = "space"
		} else {
			return failV1("invalid-binary-token")
		}
		if len(runs) == maxRunsV1 {
			return failV1("too-many-runs")
		}
		runs = append(runs, Barcode1DV1Run{Color: color, Modules: modules, SourceLabel: options.SourceLabel, SourceIndex: options.SourceIndex, Role: options.Role})
		return nil
	}
	for _, token := range runes[1:] {
		if token == current {
			width++
			continue
		}
		if err := appendRun(current, width); err != nil {
			return nil, err
		}
		current, width = token, 1
	}
	if err := appendRun(current, width); err != nil {
		return nil, err
	}
	return runs, nil
}

func ExpandWidthV1(pattern string, options Barcode1DV1WidthOptions) ([]Barcode1DV1Run, error) {
	count, ok := scalarCountV1(pattern)
	if !ok {
		return nil, failV1("invalid-width-token")
	}
	if count > maxPatternScalarsV1 {
		return nil, failV1("pattern-too-long")
	}
	if count == 0 {
		return nil, failV1("empty-pattern")
	}
	narrowMarker, wideMarker := stringOrV1(options.NarrowMarker, "N"), stringOrV1(options.WideMarker, "W")
	narrowModules, wideModules := intOrV1(options.NarrowModules, 1), intOrV1(options.WideModules, 3)
	startingColor := stringOrV1(options.StartingColor, "bar")
	narrow, narrowOK := singleRuneV1(narrowMarker)
	wide, wideOK := singleRuneV1(wideMarker)
	if !narrowOK || !wideOK || narrow == wide {
		return nil, failV1("invalid-marker-configuration")
	}
	if narrowModules <= 0 || wideModules <= 0 {
		return nil, failV1("invalid-module-count")
	}
	if err := validateSourceV1(options.SourceLabel, options.SourceIndex); err != nil {
		return nil, err
	}
	if !validRoleV1(options.Role) || !validColorV1(startingColor) {
		return nil, failV1("invalid-source-attribution")
	}

	runs := make([]Barcode1DV1Run, 0, count)
	color, content := startingColor, int64(0)
	for _, token := range pattern {
		modules := int64(0)
		if token == narrow {
			modules = narrowModules
		} else if token == wide {
			modules = wideModules
		} else {
			return nil, failV1("invalid-width-token")
		}
		var err error
		content, err = checkedAddV1(content, modules, "content-too-wide")
		if err != nil || content > maxContentModulesV1 {
			return nil, failV1("content-too-wide")
		}
		if len(runs) == maxRunsV1 {
			return nil, failV1("too-many-runs")
		}
		runs = append(runs, Barcode1DV1Run{Color: color, Modules: modules, SourceLabel: options.SourceLabel, SourceIndex: options.SourceIndex, Role: options.Role})
		color = toggleColorV1(color)
	}
	return runs, nil
}

func ComputeLayoutV1(runs []Barcode1DV1Run, quietZoneModules int64, symbols []Barcode1DV1SymbolDescriptor) (Barcode1DV1Layout, error) {
	content, err := validateRunsV1(runs)
	if err != nil {
		return Barcode1DV1Layout{}, err
	}
	if quietZoneModules < 1 || quietZoneModules > maxQuietZoneModulesV1 {
		return Barcode1DV1Layout{}, failV1("invalid-quiet-zone")
	}
	total, err := checkedAddV1(content, quietZoneModules, "content-too-wide")
	if err == nil {
		total, err = checkedAddV1(total, quietZoneModules, "content-too-wide")
	}
	if err != nil {
		return Barcode1DV1Layout{}, err
	}
	var layouts []Barcode1DV1SymbolLayout
	if symbols == nil {
		layouts, err = inferSymbolsV1(runs)
	} else {
		layouts, err = layoutSymbolsV1(symbols, content)
	}
	if err != nil {
		return Barcode1DV1Layout{}, err
	}
	return Barcode1DV1Layout{LeftQuietZoneModules: quietZoneModules, RightQuietZoneModules: quietZoneModules, ContentModules: content, TotalModules: total, SymbolLayouts: layouts}, nil
}

func ProjectSceneV1(runs []Barcode1DV1Run, options *Barcode1DV1SceneOptions) (paintinstructions.PaintScene, error) {
	return projectSceneV1WithProbe(runs, options, func() { panic("native text resolution is forbidden") })
}

func projectSceneV1WithProbe(runs []Barcode1DV1Run, options *Barcode1DV1SceneOptions, forbiddenTextResolver func()) (paintinstructions.PaintScene, error) {
	if options == nil {
		options = &Barcode1DV1SceneOptions{}
	}
	config := options.RenderConfig
	if config == nil {
		config = &Barcode1DV1RenderConfig{}
	}
	if config.IncludeHumanReadableText || options.HumanReadableText != nil {
		return paintinstructions.PaintScene{}, failV1("human-readable-text-unsupported")
	}
	moduleWidth, barHeight := intOrV1(config.ModuleWidth, 4), intOrV1(config.BarHeight, 120)
	foreground, background := stringOrV1(config.Foreground, "#000000"), stringOrV1(config.Background, "#ffffff")
	quietZoneModules, label := intOrV1(options.QuietZoneModules, 10), stringOrV1(options.Label, "1D barcode")
	if moduleWidth < 1 || moduleWidth > maxRenderDimensionV1 || barHeight < 1 || barHeight > maxRenderDimensionV1 {
		return paintinstructions.PaintScene{}, failV1("invalid-render-config")
	}
	fgCount, fgOK := scalarCountV1(foreground)
	bgCount, bgOK := scalarCountV1(background)
	if !fgOK || !bgOK || fgCount > maxColorScalarsV1 || bgCount > maxColorScalarsV1 {
		return paintinstructions.PaintScene{}, failV1("invalid-render-config")
	}
	layout, err := ComputeLayoutV1(runs, quietZoneModules, options.Symbols)
	if err != nil {
		return paintinstructions.PaintScene{}, err
	}
	if err := validateMetadataV1(options.Metadata, label); err != nil {
		return paintinstructions.PaintScene{}, err
	}

	instructions := make([]paintinstructions.PaintInstruction, 0)
	cursor := layout.LeftQuietZoneModules
	for _, run := range runs {
		end, _ := checkedAddV1(cursor, run.Modules, "content-too-wide")
		if run.Color == "bar" {
			x, err := checkedMultiplyV1(cursor, moduleWidth, "invalid-render-config")
			if err != nil {
				return paintinstructions.PaintScene{}, err
			}
			width, err := checkedMultiplyV1(run.Modules, moduleWidth, "invalid-render-config")
			if err != nil {
				return paintinstructions.PaintScene{}, err
			}
			metadata := paintinstructions.Metadata{"sourceLabel": run.SourceLabel, "sourceIndex": strconv.FormatInt(run.SourceIndex, 10), "role": run.Role, "moduleStart": strconv.FormatInt(cursor, 10), "moduleEnd": strconv.FormatInt(end, 10)}
			instructions = append(instructions, paintinstructions.PaintRectInstruction{X: int(x), Y: 0, Width: int(width), Height: int(barHeight), Fill: foreground, Metadata: metadata})
		}
		cursor = end
	}
	sceneWidth, err := checkedMultiplyV1(layout.TotalModules, moduleWidth, "invalid-render-config")
	if err != nil {
		return paintinstructions.PaintScene{}, err
	}
	metadata := make(paintinstructions.Metadata, len(options.Metadata)+10)
	for key, value := range options.Metadata {
		metadata[key] = value
	}
	metadata["label"] = label
	metadata["leftQuietZoneModules"] = strconv.FormatInt(layout.LeftQuietZoneModules, 10)
	metadata["rightQuietZoneModules"] = strconv.FormatInt(layout.RightQuietZoneModules, 10)
	metadata["contentModules"] = strconv.FormatInt(layout.ContentModules, 10)
	metadata["totalModules"] = strconv.FormatInt(layout.TotalModules, 10)
	metadata["moduleWidthPx"] = strconv.FormatInt(moduleWidth, 10)
	metadata["barHeightPx"] = strconv.FormatInt(barHeight, 10)
	metadata["sceneWidthPx"] = strconv.FormatInt(sceneWidth, 10)
	metadata["sceneHeightPx"] = strconv.FormatInt(barHeight, 10)
	metadata["symbolCount"] = strconv.Itoa(len(layout.SymbolLayouts))
	return paintinstructions.PaintScene{Width: int(sceneWidth), Height: int(barHeight), Background: background, Instructions: instructions, Metadata: metadata}, nil
}

func validateRunsV1(runs []Barcode1DV1Run) (int64, error) {
	if len(runs) > maxRunsV1 {
		return 0, failV1("too-many-runs")
	}
	content := int64(0)
	for index, run := range runs {
		if err := validateSourceV1(run.SourceLabel, run.SourceIndex); err != nil {
			return 0, err
		}
		if run.Modules <= 0 {
			return 0, failV1("invalid-module-count")
		}
		if !validColorV1(run.Color) || !validRoleV1(run.Role) {
			return 0, failV1("invalid-source-attribution")
		}
		if index > 0 && runs[index-1].Color == run.Color {
			return 0, failV1("non-alternating-runs")
		}
		var err error
		content, err = checkedAddV1(content, run.Modules, "content-too-wide")
		if err != nil || content > maxContentModulesV1 {
			return 0, failV1("content-too-wide")
		}
	}
	return content, nil
}

func inferSymbolsV1(runs []Barcode1DV1Run) ([]Barcode1DV1SymbolLayout, error) {
	result := make([]Barcode1DV1SymbolLayout, 0)
	cursor := int64(0)
	var current *Barcode1DV1SymbolLayout
	for _, run := range runs {
		if run.Role != "inter-character-gap" {
			index := int32(run.SourceIndex)
			if current == nil || current.Label != run.SourceLabel || current.SourceIndex != index || current.Role != run.Role {
				if current != nil {
					current.EndModule = cursor
					result = append(result, *current)
					if len(result) > maxSymbolsV1 {
						return nil, failV1("too-many-symbols")
					}
				}
				current = &Barcode1DV1SymbolLayout{Label: run.SourceLabel, Role: run.Role, StartModule: cursor, SourceIndex: index}
			}
		}
		cursor += run.Modules
	}
	if current != nil {
		current.EndModule = cursor
		result = append(result, *current)
		if len(result) > maxSymbolsV1 {
			return nil, failV1("too-many-symbols")
		}
	}
	return result, nil
}

func layoutSymbolsV1(symbols []Barcode1DV1SymbolDescriptor, content int64) ([]Barcode1DV1SymbolLayout, error) {
	if len(symbols) > maxSymbolsV1 {
		return nil, failV1("too-many-symbols")
	}
	result := make([]Barcode1DV1SymbolLayout, 0, len(symbols))
	cursor := int64(0)
	for _, symbol := range symbols {
		if symbol.Modules <= 0 {
			return nil, failV1("invalid-module-count")
		}
		if err := validateSourceV1(symbol.Label, symbol.SourceIndex); err != nil {
			return nil, err
		}
		if !validSymbolRoleV1(symbol.Role) {
			return nil, failV1("invalid-source-attribution")
		}
		end, err := checkedAddV1(cursor, symbol.Modules, "symbol-width-mismatch")
		if err != nil {
			return nil, err
		}
		result = append(result, Barcode1DV1SymbolLayout{Label: symbol.Label, Role: symbol.Role, StartModule: cursor, EndModule: end, SourceIndex: int32(symbol.SourceIndex)})
		cursor = end
	}
	if cursor != content {
		return nil, failV1("symbol-width-mismatch")
	}
	return result, nil
}

func validateSourceV1(label string, sourceIndex int64) error {
	count, ok := scalarCountV1(label)
	if !ok || count > maxLabelScalarsV1 || sourceIndex < math.MinInt32 || sourceIndex > math.MaxInt32 {
		return failV1("invalid-source-attribution")
	}
	return nil
}

func validateMetadataV1(metadata map[string]string, label string) error {
	if len(metadata) > maxMetadataEntriesV1 {
		return failV1("metadata-too-large")
	}
	labelCount, labelOK := scalarCountV1(label)
	if !labelOK || labelCount > maxLabelScalarsV1 {
		return failV1("metadata-too-large")
	}
	bytes := 0
	for key, value := range metadata {
		keyCount, keyOK := scalarCountV1(key)
		valueCount, valueOK := scalarCountV1(value)
		if !keyOK || !valueOK || keyCount > maxMetadataKeyV1 || valueCount > maxMetadataValueV1 {
			return failV1("metadata-too-large")
		}
		bytes += len([]byte(key)) + len([]byte(value))
		if bytes > maxMetadataBytesV1 {
			return failV1("metadata-too-large")
		}
	}
	return nil
}

func scalarCountV1(value string) (int, bool) {
	return utf8.RuneCountInString(value), utf8.ValidString(value)
}
func singleRuneV1(value string) (rune, bool) {
	if !utf8.ValidString(value) || utf8.RuneCountInString(value) != 1 {
		return 0, false
	}
	result, _ := utf8.DecodeRuneInString(value)
	return result, true
}
func validColorV1(value string) bool { return value == "bar" || value == "space" }
func toggleColorV1(value string) string {
	if value == "bar" {
		return "space"
	}
	return "bar"
}
func validRoleV1(value string) bool {
	return value == "data" || value == "start" || value == "stop" || value == "guard" || value == "check" || value == "inter-character-gap"
}
func validSymbolRoleV1(value string) bool {
	return value != "inter-character-gap" && validRoleV1(value)
}
func checkedAddV1(left, right int64, id string) (int64, error) {
	if (right > 0 && left > math.MaxInt64-right) || (right < 0 && left < math.MinInt64-right) {
		return 0, failV1(id)
	}
	return left + right, nil
}
func checkedMultiplyV1(left, right int64, id string) (int64, error) {
	if left != 0 && right > math.MaxInt64/left {
		return 0, failV1(id)
	}
	return left * right, nil
}

func intOrV1(value *int64, fallback int64) int64 {
	if value == nil {
		return fallback
	}
	return *value
}

func stringOrV1(value *string, fallback string) string {
	if value == nil {
		return fallback
	}
	return *value
}
