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
//
// # Operations
//
// Every public function is wrapped in an Operation, giving each call
// automatic timing, structured logging, and panic recovery. This
// package declares zero OS capabilities, so no op.File / op.Net
// namespace fields are available inside callbacks.
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
	result, _ := StartNew[DrawRectInstruction]("draw-instructions.DrawRect", DrawRectInstruction{},
		func(op *Operation[DrawRectInstruction], rf *ResultFactory[DrawRectInstruction]) *OperationResult[DrawRectInstruction] {
			op.AddProperty("x", x)
			op.AddProperty("y", y)
			op.AddProperty("width", width)
			op.AddProperty("height", height)
			op.AddProperty("fill", fill)
			if fill == "" {
				fill = "#000000"
			}
			if metadata == nil {
				metadata = Metadata{}
			}
			return rf.Generate(true, false, DrawRectInstruction{X: x, Y: y, Width: width, Height: height, Fill: fill, Metadata: metadata})
		}).GetResult()
	return result
}

func DrawText(x, y int, value string, metadata Metadata) DrawTextInstruction {
	result, _ := StartNew[DrawTextInstruction]("draw-instructions.DrawText", DrawTextInstruction{},
		func(op *Operation[DrawTextInstruction], rf *ResultFactory[DrawTextInstruction]) *OperationResult[DrawTextInstruction] {
			op.AddProperty("x", x)
			op.AddProperty("y", y)
			op.AddProperty("value", value)
			if metadata == nil {
				metadata = Metadata{}
			}
			return rf.Generate(true, false, DrawTextInstruction{
				X: x, Y: y, Value: value, Fill: "#000000",
				FontFamily: "monospace", FontSize: 16, Align: "middle", Metadata: metadata,
			})
		}).GetResult()
	return result
}

func DrawGroup(children []DrawInstruction, metadata Metadata) DrawGroupInstruction {
	result, _ := StartNew[DrawGroupInstruction]("draw-instructions.DrawGroup", DrawGroupInstruction{},
		func(op *Operation[DrawGroupInstruction], rf *ResultFactory[DrawGroupInstruction]) *OperationResult[DrawGroupInstruction] {
			if metadata == nil {
				metadata = Metadata{}
			}
			return rf.Generate(true, false, DrawGroupInstruction{Children: children, Metadata: metadata})
		}).GetResult()
	return result
}

// DrawLine creates a line segment from (x1,y1) to (x2,y2).
// Stroke defaults to black and StrokeWidth defaults to 1 when omitted.
func DrawLine(x1, y1, x2, y2 float64, stroke string, strokeWidth float64, metadata Metadata) DrawLineInstruction {
	result, _ := StartNew[DrawLineInstruction]("draw-instructions.DrawLine", DrawLineInstruction{},
		func(op *Operation[DrawLineInstruction], rf *ResultFactory[DrawLineInstruction]) *OperationResult[DrawLineInstruction] {
			op.AddProperty("x1", x1)
			op.AddProperty("y1", y1)
			op.AddProperty("x2", x2)
			op.AddProperty("y2", y2)
			op.AddProperty("stroke", stroke)
			op.AddProperty("strokeWidth", strokeWidth)
			if stroke == "" {
				stroke = "#000000"
			}
			if strokeWidth == 0 {
				strokeWidth = 1
			}
			if metadata == nil {
				metadata = Metadata{}
			}
			return rf.Generate(true, false, DrawLineInstruction{
				X1: x1, Y1: y1, X2: x2, Y2: y2,
				Stroke: stroke, StrokeWidth: strokeWidth, Metadata: metadata,
			})
		}).GetResult()
	return result
}

// DrawClipRegion creates a clipping rectangle that masks its children.
// The name avoids collision with the DrawClipInstruction struct.
func DrawClipRegion(x, y, width, height float64, children []DrawInstruction, metadata Metadata) DrawClipInstruction {
	result, _ := StartNew[DrawClipInstruction]("draw-instructions.DrawClipRegion", DrawClipInstruction{},
		func(op *Operation[DrawClipInstruction], rf *ResultFactory[DrawClipInstruction]) *OperationResult[DrawClipInstruction] {
			op.AddProperty("x", x)
			op.AddProperty("y", y)
			op.AddProperty("width", width)
			op.AddProperty("height", height)
			if metadata == nil {
				metadata = Metadata{}
			}
			return rf.Generate(true, false, DrawClipInstruction{
				X: x, Y: y, Width: width, Height: height,
				Children: children, Metadata: metadata,
			})
		}).GetResult()
	return result
}

func CreateScene(width, height int, instructions []DrawInstruction, background string, metadata Metadata) DrawScene {
	result, _ := StartNew[DrawScene]("draw-instructions.CreateScene", DrawScene{},
		func(op *Operation[DrawScene], rf *ResultFactory[DrawScene]) *OperationResult[DrawScene] {
			op.AddProperty("width", width)
			op.AddProperty("height", height)
			op.AddProperty("background", background)
			if background == "" {
				background = "#ffffff"
			}
			if metadata == nil {
				metadata = Metadata{}
			}
			return rf.Generate(true, false, DrawScene{
				Width: width, Height: height, Background: background, Instructions: instructions, Metadata: metadata,
			})
		}).GetResult()
	return result
}

func RenderWith[T any](scene DrawScene, renderer Renderer[T]) T {
	return renderer.Render(scene)
}
