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
//
// Stroke and StrokeWidth are optional: when Stroke is non-empty the
// rectangle is rendered with an outline of that color and width.
type DrawRectInstruction struct {
	X, Y          int
	Width, Height int
	Fill          string
	Stroke        string
	StrokeWidth   float64
	Metadata      Metadata
}

func (DrawRectInstruction) InstructionKind() string { return "rect" }

// DrawTextInstruction is a text label in scene coordinates.
//
// FontWeight controls boldness: "" or "normal" means default weight,
// "bold" renders heavier glyphs.
type DrawTextInstruction struct {
	X, Y       int
	Value      string
	Fill       string
	FontFamily string
	FontSize   int
	FontWeight string
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

// DrawLineInstruction is a straight line segment between two points.
//
// Unlike rectangles and text, line endpoints use float64 because lines
// frequently need sub-pixel positioning (e.g. hairline grid lines).
type DrawLineInstruction struct {
	X1, Y1      float64
	X2, Y2      float64
	Stroke      string
	StrokeWidth float64
	Metadata    Metadata
}

func (DrawLineInstruction) InstructionKind() string { return "line" }

// DrawClipInstruction restricts its children to a rectangular region.
//
// Anything rendered by children that falls outside the clip rectangle
// is invisible. This mirrors SVG's clipPath concept.
type DrawClipInstruction struct {
	X, Y          float64
	Width, Height float64
	Children      []DrawInstruction
	Metadata      Metadata
}

func (DrawClipInstruction) InstructionKind() string { return "clip" }

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

// DrawLine creates a line segment from (x1,y1) to (x2,y2).
// Stroke defaults to black and StrokeWidth defaults to 1 when omitted.
func DrawLine(x1, y1, x2, y2 float64, stroke string, strokeWidth float64, metadata Metadata) DrawLineInstruction {
	if stroke == "" {
		stroke = "#000000"
	}
	if strokeWidth == 0 {
		strokeWidth = 1
	}
	if metadata == nil {
		metadata = Metadata{}
	}
	return DrawLineInstruction{
		X1: x1, Y1: y1, X2: x2, Y2: y2,
		Stroke: stroke, StrokeWidth: strokeWidth, Metadata: metadata,
	}
}

// DrawClipRegion creates a clipping rectangle that masks its children.
// The name avoids collision with the DrawClipInstruction struct.
func DrawClipRegion(x, y, width, height float64, children []DrawInstruction, metadata Metadata) DrawClipInstruction {
	if metadata == nil {
		metadata = Metadata{}
	}
	return DrawClipInstruction{
		X: x, Y: y, Width: width, Height: height,
		Children: children, Metadata: metadata,
	}
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
