// Package drawinstructionssvg serializes generic draw scenes to SVG.
package drawinstructionssvg

import (
	"fmt"
	"html"
	"sort"
	"strings"

	drawinstructions "github.com/adhithyan15/coding-adventures/code/packages/go/draw-instructions"
)

const Version = "0.1.0"

func metadataToAttributes(metadata drawinstructions.Metadata) string {
	if len(metadata) == 0 {
		return ""
	}
	keys := make([]string, 0, len(metadata))
	for key := range metadata {
		keys = append(keys, key)
	}
	sort.Strings(keys)
	var builder strings.Builder
	for _, key := range keys {
		builder.WriteString(fmt.Sprintf(` data-%s="%s"`, key, html.EscapeString(fmt.Sprint(metadata[key]))))
	}
	return builder.String()
}

func renderInstruction(instruction drawinstructions.DrawInstruction) string {
	switch item := instruction.(type) {
	case drawinstructions.DrawRectInstruction:
		return fmt.Sprintf(
			`  <rect x="%d" y="%d" width="%d" height="%d" fill="%s"%s />`,
			item.X, item.Y, item.Width, item.Height, html.EscapeString(item.Fill), metadataToAttributes(item.Metadata),
		)
	case drawinstructions.DrawTextInstruction:
		return fmt.Sprintf(
			`  <text x="%d" y="%d" text-anchor="%s" font-family="%s" font-size="%d" fill="%s"%s>%s</text>`,
			item.X, item.Y, item.Align, html.EscapeString(item.FontFamily), item.FontSize, html.EscapeString(item.Fill),
			metadataToAttributes(item.Metadata), html.EscapeString(item.Value),
		)
	case drawinstructions.DrawGroupInstruction:
		children := make([]string, 0, len(item.Children))
		for _, child := range item.Children {
			children = append(children, renderInstruction(child))
		}
		return fmt.Sprintf("  <g%s>\n%s\n  </g>", metadataToAttributes(item.Metadata), strings.Join(children, "\n"))
	default:
		return ""
	}
}

type SvgRenderer struct{}

func (SvgRenderer) Render(scene drawinstructions.DrawScene) string {
	label := "draw instructions scene"
	if value, ok := scene.Metadata["label"]; ok {
		label = fmt.Sprint(value)
	}
	lines := []string{
		fmt.Sprintf(`<svg xmlns="http://www.w3.org/2000/svg" width="%d" height="%d" viewBox="0 0 %d %d" role="img" aria-label="%s">`,
			scene.Width, scene.Height, scene.Width, scene.Height, html.EscapeString(label)),
		fmt.Sprintf(`  <rect x="0" y="0" width="%d" height="%d" fill="%s" />`,
			scene.Width, scene.Height, html.EscapeString(scene.Background)),
	}
	for _, instruction := range scene.Instructions {
		lines = append(lines, renderInstruction(instruction))
	}
	lines = append(lines, "</svg>")
	return strings.Join(lines, "\n")
}

var SVGRenderer = SvgRenderer{}

func RenderSVG(scene drawinstructions.DrawScene) string {
	return SVGRenderer.Render(scene)
}
