// Package drawinstructions defines a tiny backend-neutral scene model.
//
// The goal is to separate:
//   - producer logic, which decides what should be drawn
//   - renderer logic, which decides how to serialize or paint that scene
//
// In barcode terms:
//   - Code 39 decides where the bars go
//   - this package provides generic rectangles, text, and groups
//   - an SVG backend can then serialize the scene without knowing barcode rules
package drawinstructions

const Version = "0.1.0"

type MetadataValue interface{}
type Metadata map[string]MetadataValue

// DrawInstruction is implemented by every scene instruction type.
type DrawInstruction interface {
	InstructionKind() string
}

// DrawRectInstruction is a filled rectangle in scene coordinates.
type DrawRectInstruction struct {
	X, Y          int
	Width, Height int
	Fill          string
	Metadata      Metadata
}

func (DrawRectInstruction) InstructionKind() string { return "rect" }

// DrawTextInstruction is a text label in scene coordinates.
type DrawTextInstruction struct {
	X, Y       int
	Value      string
	Fill       string
	FontFamily string
	FontSize   int
	Align      string
	Metadata   Metadata
}

func (DrawTextInstruction) InstructionKind() string { return "text" }

// DrawGroupInstruction keeps a logical grouping of child instructions.
type DrawGroupInstruction struct {
	Children []DrawInstruction
	Metadata Metadata
}

func (DrawGroupInstruction) InstructionKind() string { return "group" }

// DrawScene is the complete unit consumed by renderers.
type DrawScene struct {
	Width, Height int
	Background    string
	Instructions  []DrawInstruction
	Metadata      Metadata
}

// Renderer consumes a scene and returns some backend-specific output.
type Renderer[T any] interface {
	Render(scene DrawScene) T
}

func DrawRect(x, y, width, height int, fill string, metadata Metadata) DrawRectInstruction {
	if fill == "" {
		fill = "#000000"
	}
	if metadata == nil {
		metadata = Metadata{}
	}
	return DrawRectInstruction{X: x, Y: y, Width: width, Height: height, Fill: fill, Metadata: metadata}
}

func DrawText(x, y int, value string, metadata Metadata) DrawTextInstruction {
	if metadata == nil {
		metadata = Metadata{}
	}
	return DrawTextInstruction{
		X: x, Y: y, Value: value, Fill: "#000000",
		FontFamily: "monospace", FontSize: 16, Align: "middle", Metadata: metadata,
	}
}

func DrawGroup(children []DrawInstruction, metadata Metadata) DrawGroupInstruction {
	if metadata == nil {
		metadata = Metadata{}
	}
	return DrawGroupInstruction{Children: children, Metadata: metadata}
}

func CreateScene(width, height int, instructions []DrawInstruction, background string, metadata Metadata) DrawScene {
	if background == "" {
		background = "#ffffff"
	}
	if metadata == nil {
		metadata = Metadata{}
	}
	return DrawScene{
		Width: width, Height: height, Background: background, Instructions: instructions, Metadata: metadata,
	}
}

func RenderWith[T any](scene DrawScene, renderer Renderer[T]) T {
	return renderer.Render(scene)
}
