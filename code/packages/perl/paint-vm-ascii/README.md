# paint-vm-ascii

Perl terminal backend for `CodingAdventures::PaintInstructions`.

Executes a scene's instructions into a character grid made from box-drawing
glyphs, block fill characters, and direct text glyphs. Implements the full
`P2D02-paint-vm-ascii.md` contract:

- `rect` — filled and/or stroked rectangles (stroke via box-drawing corner/edge
  characters, fill via `█`)
- `line` — horizontal, vertical, and diagonal (Bresenham) lines
- `glyph_run` — direct character placement; `glyph_id` is treated as a literal
  Unicode code point (this backend has no font resolution)
- `group` / `clip` / `layer` — recurse into children; `group`/`layer` reject
  non-identity transforms, non-default opacity, filters, and non-normal blend
  modes (`die`s loudly rather than degrading silently, per P2D02)

Unsupported instruction kinds fail loudly (`die`) rather than being silently
ignored.

## Usage

```perl
use CodingAdventures::PaintInstructions;
use CodingAdventures::PaintVmAscii;

my $scene = CodingAdventures::PaintInstructions->paint_scene(
    16, 16,
    [
        CodingAdventures::PaintInstructions->paint_rect(0, 0, 16, 16, undef, undef),
        {
            kind      => 'glyph_run',
            glyphs    => [ { glyph_id => ord('H'), x => 0, y => 0 } ],
            font_ref  => 'terminal-mono',
            font_size => 16,
            fill      => '#000000',
        },
    ],
    'transparent',
);

my $ascii = CodingAdventures::PaintVmAscii->render($scene, { scale_x => 8, scale_y => 16 });
```

`glyph_run`, `line`, `group`, `clip`, and `layer` instructions are plain
hashrefs (no constructor helper exists yet in `PaintInstructions.pm` for
these kinds, since `render()` only needs the hash fields) — see
`P2D00-paint-instructions.md` for the field shapes.

## Development

```bash
bash BUILD
```
