// Package barcode1d composes Go 1D barcode symbologies into a renderable pipeline.
package barcode1d

import (
	"fmt"
	"runtime"
	"strings"

	barcodelayout1d "github.com/adhithyan15/coding-adventures/code/packages/go/barcode-layout-1d"
	codabar "github.com/adhithyan15/coding-adventures/code/packages/go/codabar"
	code128 "github.com/adhithyan15/coding-adventures/code/packages/go/code128"
	code39 "github.com/adhithyan15/coding-adventures/code/packages/go/code39"
	ean13 "github.com/adhithyan15/coding-adventures/code/packages/go/ean-13"
	itf "github.com/adhithyan15/coding-adventures/code/packages/go/itf"
	paintinstructions "github.com/adhithyan15/coding-adventures/code/packages/go/paint-instructions"
	pixelcontainer "github.com/adhithyan15/coding-adventures/code/packages/go/pixel-container"
	upca "github.com/adhithyan15/coding-adventures/code/packages/go/upc-a"
)

const Version = "0.1.0"

type LayoutConfig = barcodelayout1d.LayoutConfig
type RenderConfig = LayoutConfig

type Options struct {
	Symbology    string
	LayoutConfig RenderConfig
}

type UnsupportedSymbologyError struct {
	Symbology string
}

func (err UnsupportedSymbologyError) Error() string {
	return fmt.Sprintf("unsupported symbology: %s", err.Symbology)
}

var DefaultLayoutConfig = code39.DefaultLayoutConfig
var DefaultRenderConfig = DefaultLayoutConfig
var DefaultOptions = Options{
	Symbology:    "code39",
	LayoutConfig: DefaultLayoutConfig,
}

func CurrentBackend() string {
	switch {
	case runtime.GOOS == "darwin" && runtime.GOARCH == "arm64":
		return "metal"
	case runtime.GOOS == "windows":
		return "gdi"
	default:
		return "raster"
	}
}

func normalizeSymbology(symbology string) (string, error) {
	normalized := strings.ToLower(strings.ReplaceAll(strings.ReplaceAll(symbology, "-", ""), "_", ""))
	if normalized == "" {
		normalized = DefaultOptions.Symbology
	}
	switch normalized {
	case "codabar", "code128", "code39", "ean13", "itf", "upca":
		return normalized, nil
	default:
		return "", UnsupportedSymbologyError{Symbology: symbology}
	}
}

func normalizeOptions(options *Options) (Options, error) {
	normalized := DefaultOptions
	if options == nil {
		return normalized, nil
	}
	if options.Symbology != "" {
		symbology, err := normalizeSymbology(options.Symbology)
		if err != nil {
			return Options{}, err
		}
		normalized.Symbology = symbology
	}
	if options.LayoutConfig != (RenderConfig{}) {
		normalized.LayoutConfig = options.LayoutConfig
	}
	return normalized, nil
}

// BuildScene routes a barcode payload through the selected symbology and layout package.
func BuildScene(data string, options *Options) (paintinstructions.PaintScene, error) {
	normalized, err := normalizeOptions(options)
	if err != nil {
		return paintinstructions.PaintScene{}, err
	}
	switch normalized.Symbology {
	case "codabar":
		return codabar.LayoutCodabar(data, normalized.LayoutConfig)
	case "code128":
		return code128.LayoutCode128(data, normalized.LayoutConfig)
	case "code39":
		return code39.LayoutCode39(data, normalized.LayoutConfig)
	case "ean13":
		return ean13.LayoutEAN13(data, normalized.LayoutConfig)
	case "itf":
		return itf.LayoutITF(data, normalized.LayoutConfig)
	case "upca":
		return upca.LayoutUPCA(data, normalized.LayoutConfig)
	default:
		return paintinstructions.PaintScene{}, UnsupportedSymbologyError{Symbology: normalized.Symbology}
	}
}

// RenderPixels executes the paint scene into a PixelContainer.
func RenderPixels(data string, options *Options) (*pixelcontainer.PixelContainer, error) {
	scene, err := BuildScene(data, options)
	if err != nil {
		return nil, err
	}
	return renderPixelsForCurrentBackend(scene)
}

// RenderPNG renders the barcode to PNG bytes.
func RenderPNG(data string, options *Options) ([]byte, error) {
	scene, err := BuildScene(data, options)
	if err != nil {
		return nil, err
	}
	return renderPNGForCurrentBackend(scene)
}
