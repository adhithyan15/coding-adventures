// Package drawinstructionssvg serializes generic draw scenes to SVG.
//
// # Operations
//
// Every public function/method is wrapped in an Operation, giving each call
// automatic timing, structured logging, and panic recovery. This
// package declares zero OS capabilities, so no op.File / op.Net
// namespace fields are available inside callbacks.
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

// renderInstruction dispatches a single draw instruction to the
// appropriate SVG serializer. The counter is used to generate unique
// IDs for clip paths.
func renderInstruction(instruction drawinstructions.DrawInstruction, counter *int) string {
	switch item := instruction.(type) {
	case drawinstructions.DrawRectInstruction:
		return renderRect(item)
	case drawinstructions.DrawTextInstruction:
		return renderText(item)
	case drawinstructions.DrawGroupInstruction:
		children := make([]string, 0, len(item.Children))
		for _, child := range item.Children {
			children = append(children, renderInstruction(child, counter))
		}
		return fmt.Sprintf("  <g%s>\n%s\n  </g>", metadataToAttributes(item.Metadata), strings.Join(children, "\n"))
	case drawinstructions.DrawLineInstruction:
		return renderLine(item)
	case drawinstructions.DrawClipInstruction:
		return renderClip(item, counter)
	default:
		return ""
	}
}

// renderRect serializes a rectangle. When Stroke is set, the outline
// attributes are included.
func renderRect(item drawinstructions.DrawRectInstruction) string {
	strokeAttrs := ""
	if item.Stroke != "" {
		strokeAttrs = fmt.Sprintf(` stroke="%s" stroke-width="%.4g"`,
			html.EscapeString(item.Stroke), item.StrokeWidth)
	}
	return fmt.Sprintf(
		`  <rect x="%d" y="%d" width="%d" height="%d" fill="%s"%s%s />`,
		item.X, item.Y, item.Width, item.Height, html.EscapeString(item.Fill),
		strokeAttrs, metadataToAttributes(item.Metadata),
	)
}

// renderText serializes a text label. When FontWeight is "bold", the
// font-weight attribute is emitted.
func renderText(item drawinstructions.DrawTextInstruction) string {
	weightAttr := ""
	if item.FontWeight == "bold" {
		weightAttr = ` font-weight="bold"`
	}
	return fmt.Sprintf(
		`  <text x="%d" y="%d" text-anchor="%s" font-family="%s" font-size="%d"%s fill="%s"%s>%s</text>`,
		item.X, item.Y, item.Align, html.EscapeString(item.FontFamily), item.FontSize,
		weightAttr, html.EscapeString(item.Fill),
		metadataToAttributes(item.Metadata), html.EscapeString(item.Value),
	)
}

// renderLine serializes a line segment to an SVG <line> element.
func renderLine(item drawinstructions.DrawLineInstruction) string {
	return fmt.Sprintf(
		`  <line x1="%.4g" y1="%.4g" x2="%.4g" y2="%.4g" stroke="%s" stroke-width="%.4g"%s />`,
		item.X1, item.Y1, item.X2, item.Y2,
		html.EscapeString(item.Stroke), item.StrokeWidth,
		metadataToAttributes(item.Metadata),
	)
}

// renderClip wraps children in a clipPath-limited group. Each clip
// gets a unique ID derived from the shared counter.
func renderClip(item drawinstructions.DrawClipInstruction, counter *int) string {
	*counter++
	id := fmt.Sprintf("clip-%d", *counter)
	children := make([]string, 0, len(item.Children))
	for _, child := range item.Children {
		children = append(children, renderInstruction(child, counter))
	}
	return fmt.Sprintf(
		"  <defs>\n    <clipPath id=\"%s\">\n      <rect x=\"%.4g\" y=\"%.4g\" width=\"%.4g\" height=\"%.4g\" />\n    </clipPath>\n  </defs>\n  <g clip-path=\"url(#%s)\"%s>\n%s\n  </g>",
		id, item.X, item.Y, item.Width, item.Height,
		id, metadataToAttributes(item.Metadata),
		strings.Join(children, "\n"),
	)
}

type SvgRenderer struct{}

func (SvgRenderer) Render(scene drawinstructions.DrawScene) string {
	result, _ := StartNew[string]("draw-instructions-svg.SvgRenderer.Render", "",
		func(op *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
			label := "draw instructions scene"
			if value, ok := scene.Metadata["label"]; ok {
				label = fmt.Sprint(value)
			}
			counter := 0
			lines := []string{
				fmt.Sprintf(`<svg xmlns="http://www.w3.org/2000/svg" width="%d" height="%d" viewBox="0 0 %d %d" role="img" aria-label="%s">`,
					scene.Width, scene.Height, scene.Width, scene.Height, html.EscapeString(label)),
				fmt.Sprintf(`  <rect x="0" y="0" width="%d" height="%d" fill="%s" />`,
					scene.Width, scene.Height, html.EscapeString(scene.Background)),
			}
			for _, instruction := range scene.Instructions {
				lines = append(lines, renderInstruction(instruction, &counter))
			}
			lines = append(lines, "</svg>")
			return rf.Generate(true, false, strings.Join(lines, "\n"))
		}).GetResult()
	return result
}

var SVGRenderer = SvgRenderer{}

func RenderSVG(scene drawinstructions.DrawScene) string {
	result, _ := StartNew[string]("draw-instructions-svg.RenderSVG", "",
		func(op *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
			return rf.Generate(true, false, SVGRenderer.Render(scene))
		}).GetResult()
	return result
}
