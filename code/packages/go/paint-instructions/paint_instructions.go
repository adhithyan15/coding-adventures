// Package paintinstructions defines a tiny backend-neutral paint scene model.
package paintinstructions

import (
	"fmt"
	"strconv"
	"strings"
)

const Version = "0.1.0"

type MetadataValue interface{}
type Metadata map[string]MetadataValue

type PaintInstruction interface {
	InstructionKind() string
}

type PaintColorRGBA8 struct {
	R byte
	G byte
	B byte
	A byte
}

type PaintRectInstruction struct {
	X, Y          int
	Width, Height int
	Fill          string
	Metadata      Metadata
}

func (PaintRectInstruction) InstructionKind() string { return "rect" }

type PaintScene struct {
	Width, Height int
	Background    string
	Instructions  []PaintInstruction
	Metadata      Metadata
}

func PaintRect(x, y, width, height int, fill string, metadata Metadata) PaintRectInstruction {
	if fill == "" {
		fill = "#000000"
	}
	if metadata == nil {
		metadata = Metadata{}
	}
	return PaintRectInstruction{
		X: x, Y: y, Width: width, Height: height, Fill: fill, Metadata: metadata,
	}
}

func CreateScene(width, height int, instructions []PaintInstruction, background string, metadata Metadata) PaintScene {
	if background == "" {
		background = "#ffffff"
	}
	if metadata == nil {
		metadata = Metadata{}
	}
	return PaintScene{
		Width: width, Height: height, Background: background, Instructions: instructions, Metadata: metadata,
	}
}

func PaintSceneOf(width, height int, instructions []PaintInstruction, background string, metadata Metadata) PaintScene {
	return CreateScene(width, height, instructions, background, metadata)
}

// PathCommand represents a single drawing command within a path.
//
// Kind must be one of:
//
//	"move_to"  — lift the pen and move to (X, Y)
//	"line_to"  — draw a straight line from the current position to (X, Y)
//	"close"    — close the current sub-path back to the most recent move_to
//
// For "close" the X and Y fields are unused; they should be set to 0.
type PathCommand struct {
	Kind string
	X    float64
	Y    float64
}

// PaintPathInstruction draws a closed polygon described by a series of
// PathCommands.  Typically the commands form:
//
//	move_to → line_to … → close
type PaintPathInstruction struct {
	Commands []PathCommand
	Fill     string
	Metadata Metadata
}

func (PaintPathInstruction) InstructionKind() string { return "path" }

// PaintPath builds a PaintPathInstruction from the given commands.
// If fill is empty it defaults to "#000000".
func PaintPath(commands []PathCommand, fill string, metadata Metadata) PaintPathInstruction {
	if fill == "" {
		fill = "#000000"
	}
	if metadata == nil {
		metadata = Metadata{}
	}
	return PaintPathInstruction{Commands: commands, Fill: fill, Metadata: metadata}
}

func ParseColorRGBA8(value string) (PaintColorRGBA8, error) {
	value = strings.TrimSpace(value)
	if !strings.HasPrefix(value, "#") {
		return PaintColorRGBA8{}, fmt.Errorf("paint color must start with #")
	}

	hex := value[1:]
	switch len(hex) {
	case 3:
		hex = strings.Repeat(string(hex[0]), 2) +
			strings.Repeat(string(hex[1]), 2) +
			strings.Repeat(string(hex[2]), 2) + "ff"
	case 4:
		hex = strings.Repeat(string(hex[0]), 2) +
			strings.Repeat(string(hex[1]), 2) +
			strings.Repeat(string(hex[2]), 2) +
			strings.Repeat(string(hex[3]), 2)
	case 6:
		hex += "ff"
	case 8:
	default:
		return PaintColorRGBA8{}, fmt.Errorf("paint color must be #rgb, #rgba, #rrggbb, or #rrggbbaa")
	}

	channel := func(offset int) (byte, error) {
		value, err := strconv.ParseUint(hex[offset:offset+2], 16, 8)
		if err != nil {
			return 0, fmt.Errorf("paint color contains invalid hex digits")
		}
		return byte(value), nil
	}

	r, err := channel(0)
	if err != nil {
		return PaintColorRGBA8{}, err
	}
	g, err := channel(2)
	if err != nil {
		return PaintColorRGBA8{}, err
	}
	b, err := channel(4)
	if err != nil {
		return PaintColorRGBA8{}, err
	}
	a, err := channel(6)
	if err != nil {
		return PaintColorRGBA8{}, err
	}

	return PaintColorRGBA8{R: r, G: g, B: b, A: a}, nil
}
