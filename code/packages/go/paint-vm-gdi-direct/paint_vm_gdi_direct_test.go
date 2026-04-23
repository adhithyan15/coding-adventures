package paintvmgdidirect

import (
	"testing"

	paintinstructions "github.com/adhithyan15/coding-adventures/code/packages/go/paint-instructions"
)

func TestSupportedRuntimeMatchesAvailability(t *testing.T) {
	if SupportedRuntime() != Available() {
		t.Fatalf("SupportedRuntime and Available should agree")
	}
}

func TestRender(t *testing.T) {
	if !Available() {
		return
	}

	scene := paintinstructions.CreateScene(
		16,
		8,
		[]paintinstructions.PaintInstruction{
			paintinstructions.PaintRect(4, 0, 8, 8, "#000000", nil),
		},
		"#ffffff",
		nil,
	)

	pixels, err := Render(scene)
	if err != nil {
		t.Fatalf("Render returned error: %v", err)
	}
	if pixels.Width != 16 || pixels.Height != 8 {
		t.Fatalf("unexpected pixel size: %dx%d", pixels.Width, pixels.Height)
	}
	if pixels.Data[0] != 255 || pixels.Data[1] != 255 || pixels.Data[2] != 255 || pixels.Data[3] != 255 {
		t.Fatalf("expected white background pixel, got %v", pixels.Data[:4])
	}
}
