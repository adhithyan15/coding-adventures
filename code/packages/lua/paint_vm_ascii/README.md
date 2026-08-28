# paint_vm_ascii

Lua terminal backend for `coding_adventures.paint_instructions`.

Implements the full `P2D02-paint-vm-ascii.md` contract:

| kind | behavior |
|---|---|
| `rect` | fill and/or stroke via block (`█`) and box-drawing (`┌┐└┘─│┬┴├┤┼`) characters |
| `line` | horizontal/vertical fast paths, Bresenham for the diagonal case |
| `glyph_run` | direct character placement from a literal Unicode code point (`glyph_id`) |
| `group` | recurse into children (must be untransformed, fully opaque) |
| `clip` | intersect clip bounds, recurse into children |
| `layer` | recurse into children (must have no transform, filters, non-default opacity, or non-normal blend mode) |

`path` and any other instruction kind raise a loud error rather than being
silently skipped, per spec.

## Usage

```lua
local paint = require("coding_adventures.paint_instructions")
local vm = require("coding_adventures.paint_vm_ascii")

local scene = paint.paint_scene(16, 16, {
    paint.paint_glyph_run({
        paint.paint_glyph_placement(string.byte("H"), 0, 0),
        paint.paint_glyph_placement(string.byte("i"), 8, 0),
    }, "terminal-mono", 16, "#000000"),
})

print(vm.render(scene, { scale_x = 8, scale_y = 16 })) -- "Hi"
```

`render(scene, options)` defaults `options.scale_x`/`options.scale_y` to `8`
and `16` (the spec's documented defaults) when `options` is omitted or a
field is `nil`. It raises via Lua's `error(...)` on invalid geometry, a
scene too large to render, an unsupported feature (transformed/opaque-only
group/layer violations, `path` instructions), or nesting deeper than 64
`group`/`clip`/`layer` levels -- wrap a call in `pcall` to trap a failure
instead of letting it propagate.

## Consumers

`code/programs/lua/cowsay` is the first real consumer: it converts the
composed bubble+cow text block into a `PaintScene` of `glyph_run`
instructions (one per line) and renders it through this module instead of
printing the text directly. See
`code/specs/cowsay-paintvm-pipeline.md`.
