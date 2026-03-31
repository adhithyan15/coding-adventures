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

func TestRenderLine(t *testing.T) {
	line := drawinstructions.DrawLine(10, 20, 30.5, 40.5, "#ff0000", 2, nil)
	scene := drawinstructions.CreateScene(100, 100, []drawinstructions.DrawInstruction{line}, "", nil)
	svg := RenderSVG(scene)
	if !strings.Contains(svg, "<line") {
		t.Fatalf("expected <line> element in svg: %s", svg)
	}
	if !strings.Contains(svg, `x1="10"`) || !strings.Contains(svg, `y2="40.5"`) {
		t.Fatalf("expected correct coordinates in svg: %s", svg)
	}
	if !strings.Contains(svg, `stroke="#ff0000"`) || !strings.Contains(svg, `stroke-width="2"`) {
		t.Fatalf("expected stroke attributes in svg: %s", svg)
	}
}

func TestRenderClip(t *testing.T) {
	rect := drawinstructions.DrawRect(5, 5, 10, 10, "#000", nil)
	clip := drawinstructions.DrawClipRegion(0, 0, 50, 50, []drawinstructions.DrawInstruction{rect}, nil)
	scene := drawinstructions.CreateScene(100, 100, []drawinstructions.DrawInstruction{clip}, "", nil)
	svg := RenderSVG(scene)
	if !strings.Contains(svg, "<clipPath") {
		t.Fatalf("expected <clipPath> in svg: %s", svg)
	}
	if !strings.Contains(svg, `clip-path="url(#clip-1)"`) {
		t.Fatalf("expected clip-path reference in svg: %s", svg)
	}
	if !strings.Contains(svg, `<rect x="5"`) {
		t.Fatalf("expected child rect in clipped group: %s", svg)
	}
}

func TestRenderStrokedRect(t *testing.T) {
	rect := drawinstructions.DrawRect(0, 0, 50, 50, "#fff", nil)
	rect.Stroke = "#000"
	rect.StrokeWidth = 2
	scene := drawinstructions.CreateScene(100, 100, []drawinstructions.DrawInstruction{rect}, "", nil)
	svg := RenderSVG(scene)
	if !strings.Contains(svg, `stroke="#000"`) {
		t.Fatalf("expected stroke attribute in svg: %s", svg)
	}
	if !strings.Contains(svg, `stroke-width="2"`) {
		t.Fatalf("expected stroke-width attribute in svg: %s", svg)
	}
}

func TestRenderBoldText(t *testing.T) {
	text := drawinstructions.DrawText(10, 20, "hello", nil)
	text.FontWeight = "bold"
	scene := drawinstructions.CreateScene(100, 100, []drawinstructions.DrawInstruction{text}, "", nil)
	svg := RenderSVG(scene)
	if !strings.Contains(svg, `font-weight="bold"`) {
		t.Fatalf("expected font-weight attribute in svg: %s", svg)
	}
}

func TestRenderNormalTextNoFontWeight(t *testing.T) {
	text := drawinstructions.DrawText(10, 20, "hello", nil)
	scene := drawinstructions.CreateScene(100, 100, []drawinstructions.DrawInstruction{text}, "", nil)
	svg := RenderSVG(scene)
	if strings.Contains(svg, `font-weight`) {
		t.Fatalf("did not expect font-weight attribute for normal text: %s", svg)
	}
}

func TestRenderRectNoStrokeByDefault(t *testing.T) {
	rect := drawinstructions.DrawRect(0, 0, 50, 50, "#fff", nil)
	scene := drawinstructions.CreateScene(100, 100, []drawinstructions.DrawInstruction{rect}, "", nil)
	svg := RenderSVG(scene)
	if strings.Contains(svg, `stroke=`) {
		t.Fatalf("did not expect stroke attributes on unstoked rect: %s", svg)
	}
}
