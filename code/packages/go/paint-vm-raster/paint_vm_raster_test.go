package paintvmraster

import (
	"testing"

	paintinstructions "github.com/adhithyan15/coding-adventures/code/packages/go/paint-instructions"
)

type unsupportedInstruction struct{}

func (unsupportedInstruction) InstructionKind() string { return "unsupported" }

func TestRenderFillsBackgroundAndBars(t *testing.T) {
	scene := paintinstructions.CreateScene(
		8,
		4,
		[]paintinstructions.PaintInstruction{
			paintinstructions.PaintRect(2, 0, 3, 4, "#000000", nil),
		},
		"#ffffff",
		nil,
	)

	buffer, err := Render(scene)
	if err != nil {
		t.Fatal(err)
	}
	if buffer.Width != 8 || buffer.Height != 4 {
		t.Fatalf("unexpected buffer size: %dx%d", buffer.Width, buffer.Height)
	}

	r, g, b, a := buffer.Data[0], buffer.Data[1], buffer.Data[2], buffer.Data[3]
	if r != 255 || g != 255 || b != 255 || a != 255 {
		t.Fatalf("unexpected background pixel: %d %d %d %d", r, g, b, a)
	}

	offset := (1*buffer.Width + 3) * 4
	r, g, b, a = buffer.Data[offset], buffer.Data[offset+1], buffer.Data[offset+2], buffer.Data[offset+3]
	if r != 0 || g != 0 || b != 0 || a != 255 {
		t.Fatalf("unexpected bar pixel: %d %d %d %d", r, g, b, a)
	}
}

func TestRenderRejectsUnsupportedInstructions(t *testing.T) {
	scene := paintinstructions.CreateScene(1, 1, []paintinstructions.PaintInstruction{unsupportedInstruction{}}, "#ffffff", nil)
	if _, err := Render(scene); err == nil {
		t.Fatal("expected unsupported instruction error")
	}
}
