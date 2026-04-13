// Package paintinstructions defines a tiny backend-neutral paint scene model.
package paintinstructions

const Version = "0.1.0"

type MetadataValue interface{}
type Metadata map[string]MetadataValue

type PaintInstruction interface {
	InstructionKind() string
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
