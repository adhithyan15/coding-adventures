package drawinstructionssvg

import (
	"strings"
	"testing"

	drawinstructions "github.com/adhithyan15/coding-adventures/code/packages/go/draw-instructions"
)

func TestRenderSVG(t *testing.T) {
	scene := drawinstructions.CreateScene(
		100,
		50,
		[]drawinstructions.DrawInstruction{drawinstructions.DrawRect(10, 10, 20, 30, "#000000", nil)},
		"",
		drawinstructions.Metadata{"label": "demo"},
	)
	svg := RenderSVG(scene)
	if !strings.Contains(svg, "<svg") || !strings.Contains(svg, `aria-label="demo"`) {
		t.Fatalf("unexpected svg: %s", svg)
	}
}
