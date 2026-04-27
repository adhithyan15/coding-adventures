// Package paintvmraster executes rect-based PaintScene values into PixelContainer.
package paintvmraster

import (
	"fmt"
	"strconv"
	"strings"

	paintinstructions "github.com/adhithyan15/coding-adventures/code/packages/go/paint-instructions"
	pixelcontainer "github.com/adhithyan15/coding-adventures/code/packages/go/pixel-container"
)

const Version = "0.1.0"

func parseHexColor(value string) (byte, byte, byte, byte, error) {
	if value == "" {
		value = "#000000"
	}
	if strings.HasPrefix(value, "#") {
		value = value[1:]
	}
	if len(value) != 6 && len(value) != 8 {
		return 0, 0, 0, 0, fmt.Errorf("unsupported color format: %q", value)
	}

	parseComponent := func(component string) (byte, error) {
		parsed, err := strconv.ParseUint(component, 16, 8)
		if err != nil {
			return 0, fmt.Errorf("invalid color component %q: %w", component, err)
		}
		return byte(parsed), nil
	}

	r, err := parseComponent(value[0:2])
	if err != nil {
		return 0, 0, 0, 0, err
	}
	g, err := parseComponent(value[2:4])
	if err != nil {
		return 0, 0, 0, 0, err
	}
	b, err := parseComponent(value[4:6])
	if err != nil {
		return 0, 0, 0, 0, err
	}
	a := byte(255)
	if len(value) == 8 {
		a, err = parseComponent(value[6:8])
		if err != nil {
			return 0, 0, 0, 0, err
		}
	}
	return r, g, b, a, nil
}

func minInt(a, b int) int {
	if a < b {
		return a
	}
	return b
}

func maxInt(a, b int) int {
	if a > b {
		return a
	}
	return b
}

func drawRect(buffer *pixelcontainer.PixelContainer, rect paintinstructions.PaintRectInstruction) error {
	if rect.Width <= 0 || rect.Height <= 0 {
		return nil
	}
	r, g, b, a, err := parseHexColor(rect.Fill)
	if err != nil {
		return err
	}

	startX := maxInt(rect.X, 0)
	startY := maxInt(rect.Y, 0)
	endX := minInt(rect.X+rect.Width, int(buffer.Width))
	endY := minInt(rect.Y+rect.Height, int(buffer.Height))
	if startX >= endX || startY >= endY {
		return nil
	}

	for y := startY; y < endY; y++ {
		for x := startX; x < endX; x++ {
			pixelcontainer.SetPixel(buffer, uint32(x), uint32(y), r, g, b, a)
		}
	}
	return nil
}

// Render executes a rect-only PaintScene into a pixel buffer.
func Render(scene paintinstructions.PaintScene) (*pixelcontainer.PixelContainer, error) {
	if scene.Width < 0 || scene.Height < 0 {
		return nil, fmt.Errorf("scene dimensions must be non-negative")
	}
	if scene.Width > pixelcontainer.MaxDimension || scene.Height > pixelcontainer.MaxDimension {
		return nil, fmt.Errorf("scene dimensions exceed pixelcontainer.MaxDimension")
	}

	buffer := pixelcontainer.New(uint32(scene.Width), uint32(scene.Height))
	bgR, bgG, bgB, bgA, err := parseHexColor(scene.Background)
	if err != nil {
		return nil, err
	}
	pixelcontainer.FillPixels(buffer, bgR, bgG, bgB, bgA)

	for _, instruction := range scene.Instructions {
		switch current := instruction.(type) {
		case paintinstructions.PaintRectInstruction:
			if err := drawRect(buffer, current); err != nil {
				return nil, err
			}
		case *paintinstructions.PaintRectInstruction:
			if current == nil {
				continue
			}
			if err := drawRect(buffer, *current); err != nil {
				return nil, err
			}
		default:
			return nil, fmt.Errorf("unsupported paint instruction kind: %s", instruction.InstructionKind())
		}
	}

	return buffer, nil
}
