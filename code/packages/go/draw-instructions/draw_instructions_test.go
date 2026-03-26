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

type testRenderer struct{}

func (testRenderer) Render(scene DrawScene) string {
	return "ok"
}

func TestRenderWith(t *testing.T) {
	if got := RenderWith(CreateScene(10, 10, nil, "", nil), testRenderer{}); got != "ok" {
		t.Fatalf("expected ok, got %s", got)
	}
}
