package drawinstructions

import "testing"

func TestVersion(t *testing.T) {
	if Version != "0.1.0" {
		t.Fatalf("expected version 0.1.0, got %s", Version)
	}
}

func TestHelpers(t *testing.T) {
	rect := DrawRect(1, 2, 3, 4, "#111111", Metadata{"kind": "demo"})
	if rect.InstructionKind() != "rect" || rect.Width != 3 {
		t.Fatalf("unexpected rect: %#v", rect)
	}

	text := DrawText(10, 20, "hello", nil)
	if text.Align != "middle" || text.FontFamily != "monospace" {
		t.Fatalf("unexpected text defaults: %#v", text)
	}

	scene := CreateScene(100, 50, []DrawInstruction{DrawGroup([]DrawInstruction{rect}, nil)}, "", nil)
	if scene.Background != "#ffffff" {
		t.Fatalf("expected default background, got %s", scene.Background)
	}
}

func TestDrawLine(t *testing.T) {
	line := DrawLine(0, 0, 100.5, 200.5, "#ff0000", 2, Metadata{"id": "L1"})
	if line.InstructionKind() != "line" {
		t.Fatalf("expected kind line, got %s", line.InstructionKind())
	}
	if line.X1 != 0 || line.Y1 != 0 || line.X2 != 100.5 || line.Y2 != 200.5 {
		t.Fatalf("unexpected coordinates: %#v", line)
	}
	if line.Stroke != "#ff0000" || line.StrokeWidth != 2 {
		t.Fatalf("unexpected stroke: %#v", line)
	}
}

func TestDrawLineDefaults(t *testing.T) {
	line := DrawLine(1, 2, 3, 4, "", 0, nil)
	if line.Stroke != "#000000" {
		t.Fatalf("expected default stroke #000000, got %s", line.Stroke)
	}
	if line.StrokeWidth != 1 {
		t.Fatalf("expected default stroke width 1, got %f", line.StrokeWidth)
	}
	if line.Metadata == nil {
		t.Fatal("expected non-nil metadata")
	}
}

func TestDrawClipRegion(t *testing.T) {
	rect := DrawRect(5, 5, 10, 10, "#000", nil)
	clip := DrawClipRegion(0, 0, 50.5, 50.5, []DrawInstruction{rect}, Metadata{"region": "header"})
	if clip.InstructionKind() != "clip" {
		t.Fatalf("expected kind clip, got %s", clip.InstructionKind())
	}
	if clip.X != 0 || clip.Y != 0 || clip.Width != 50.5 || clip.Height != 50.5 {
		t.Fatalf("unexpected clip bounds: %#v", clip)
	}
	if len(clip.Children) != 1 {
		t.Fatalf("expected 1 child, got %d", len(clip.Children))
	}
}

func TestDrawRectStroke(t *testing.T) {
	rect := DrawRect(0, 0, 10, 10, "#fff", nil)
	rect.Stroke = "#000"
	rect.StrokeWidth = 2.5
	if rect.Stroke != "#000" || rect.StrokeWidth != 2.5 {
		t.Fatalf("stroke fields not set correctly: %#v", rect)
	}
}

func TestDrawTextFontWeight(t *testing.T) {
	text := DrawText(0, 0, "bold text", nil)
	text.FontWeight = "bold"
	if text.FontWeight != "bold" {
		t.Fatalf("font weight not set: %#v", text)
	}
}

type testRenderer struct{}

func (testRenderer) Render(scene DrawScene) string {
	return "ok"
}

func TestRenderWith(t *testing.T) {
	if got := RenderWith(CreateScene(10, 10, nil, "", nil), testRenderer{}); got != "ok" {
		t.Fatalf("expected ok, got %s", got)
	}
}
