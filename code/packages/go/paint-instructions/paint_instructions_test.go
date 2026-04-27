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

func TestParseColorRGBA8(t *testing.T) {
	color, err := ParseColorRGBA8("#4a90d9")
	if err != nil {
		t.Fatalf("ParseColorRGBA8 returned error: %v", err)
	}
	if color != (PaintColorRGBA8{R: 0x4a, G: 0x90, B: 0xd9, A: 0xff}) {
		t.Fatalf("unexpected parsed color: %#v", color)
	}

	withAlpha, err := ParseColorRGBA8("#1234")
	if err != nil {
		t.Fatalf("ParseColorRGBA8 returned error for short rgba: %v", err)
	}
	if withAlpha != (PaintColorRGBA8{R: 0x11, G: 0x22, B: 0x33, A: 0x44}) {
		t.Fatalf("unexpected parsed short rgba: %#v", withAlpha)
	}
}

func TestParseColorRGBA8RejectsInvalidInput(t *testing.T) {
	if _, err := ParseColorRGBA8("red"); err == nil {
		t.Fatal("expected ParseColorRGBA8 to reject non-hex input")
	}
}
