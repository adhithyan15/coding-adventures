package paintvmascii

import (
	"strings"
	"testing"

	paintinstructions "github.com/adhithyan15/coding-adventures/code/packages/go/paint-instructions"
)

func TestVersion(t *testing.T) {
	if Version != "0.1.0" {
		t.Fatalf("Version = %q", Version)
	}
}

func TestRenderFilledRect(t *testing.T) {
	scene := paintinstructions.CreateScene(3, 2, []paintinstructions.PaintInstruction{
		paintinstructions.PaintRect(0, 0, 2, 1, "#000000", nil),
	}, "#ffffff", nil)

	result, err := Render(scene, &AsciiOptions{ScaleX: 1, ScaleY: 1})
	if err != nil {
		t.Fatalf("Render returned error: %v", err)
	}
	if !strings.Contains(result, "█") {
		t.Fatalf("expected output to contain block characters, got %q", result)
	}
}

func TestRenderTransparentRectProducesEmptyOutput(t *testing.T) {
	scene := paintinstructions.CreateScene(3, 2, []paintinstructions.PaintInstruction{
		paintinstructions.PaintRect(0, 0, 2, 1, "transparent", nil),
	}, "#ffffff", nil)

	result, err := Render(scene, &AsciiOptions{ScaleX: 1, ScaleY: 1})
	if err != nil {
		t.Fatalf("Render returned error: %v", err)
	}
	if result != "" {
		t.Fatalf("expected empty output, got %q", result)
	}
}
