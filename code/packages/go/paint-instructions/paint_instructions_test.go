package paintinstructions

import "testing"

func TestPaintRectAndScene(t *testing.T) {
	rect := PaintRect(1, 2, 3, 4, "", Metadata{"role": "bar"})
	if rect.InstructionKind() != "rect" {
		t.Fatalf("unexpected rect kind: %s", rect.InstructionKind())
	}
	scene := CreateScene(10, 20, []PaintInstruction{rect}, "", nil)
	if scene.Width != 10 || scene.Height != 20 {
		t.Fatalf("unexpected scene dimensions: %#v", scene)
	}
}
