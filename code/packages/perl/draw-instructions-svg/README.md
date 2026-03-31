# CodingAdventures::DrawInstructionsSvg

SVG renderer for the draw instructions intermediate representation.

## What It Does

Takes a scene hashref (produced by `CodingAdventures::DrawInstructions`) and serialises it into a complete SVG document string. Handles all instruction kinds: rect, text, line, circle, clip, and group.

## How It Fits

This sits one layer above `draw-instructions` in the stack. The draw instructions package defines *what* to draw; this package decides *how* to draw it as SVG. Other renderers (Canvas, terminal, etc.) can consume the same instructions.

## Usage

```perl
use CodingAdventures::DrawInstructions;
use CodingAdventures::DrawInstructionsSvg;

my $scene = CodingAdventures::DrawInstructions::create_scene(
    800, 600,
    [
        CodingAdventures::DrawInstructions::draw_rect(10, 20, 100, 50, "#ff0000"),
        CodingAdventures::DrawInstructions::draw_text(50, 45, "Hello"),
        CodingAdventures::DrawInstructions::draw_circle(200, 200, 30, "#0000ff"),
    ],
    "#ffffff",
);

my $svg = CodingAdventures::DrawInstructionsSvg::render_svg($scene);
print $svg;
```

## Features

- XML escaping for all text and attribute values
- Metadata hashrefs serialised as `data-*` attributes
- Accessible SVG output with `role="img"` and `aria-label`
- Deterministic clip IDs (counter resets each render)
- Stroke support on rectangles
- Font weight support on text
