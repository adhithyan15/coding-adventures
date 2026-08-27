# paint_instructions

Backend-neutral paint scene primitives.

Every "instruction" is a plain Lua table tagged with a `kind` field, built
by a constructor function rather than a class -- any backend (terminal
ASCII, SVG, native GPU) can read the table and render it in its own way
without depending on this module's internals.

## Instructions

| kind | constructor | notes |
|---|---|---|
| `rect` | `paint_rect(x, y, width, height, fill, metadata, stroke, stroke_width)` | `stroke`/`stroke_width` are optional trailing params, default "no stroke" |
| `line` | `paint_line(x1, y1, x2, y2, stroke, stroke_width, metadata)` | `stroke` is required -- an unstroked line is invisible |
| `glyph_run` | `paint_glyph_run(glyphs, font_ref, font_size, fill, metadata)` | `glyphs` is a list of `paint_glyph_placement(glyph_id, x, y)` results |
| `group` | `paint_group(children, opts)` | `opts` keys: `transform`, `opacity`, `metadata` |
| `clip` | `paint_clip(x, y, width, height, children, metadata)` | rectangular clip region wrapping `children` |
| `layer` | `paint_layer(children, opts)` | `opts` keys: `has_filters`, `blend_mode`, `opacity`, `transform`, `metadata` |
| `path` | `paint_path(commands, fill, metadata)` | `commands` is a list of `move_to`/`line_to`/`close` tables |

`paint_scene(width, height, instructions, background, metadata)` (alias:
`create_scene`) wraps a list of instructions into a renderable frame.

`transform2d(a, b, c, d, e, f)` / `identity_transform()` /
`is_identity_transform(transform)` build and inspect the six-value affine
transform used by `group.transform` / `layer.transform`.

See `code/specs/P2D00-paint-instructions.md` for the general contract and
`code/specs/P2D02-paint-vm-ascii.md` for the terminal backend that consumes
these instructions (`code/packages/lua/paint_vm_ascii`).

## Development

```bash
bash BUILD
```
