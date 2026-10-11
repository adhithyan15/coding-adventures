---
category: Testing & coverage
---

# A ductus path's upper and lower parts are in y-up font units, so pin their positions, not just their labels

**Context:** the Kannada visarga ಃ in `script-ductus`
(`src/strokes/kannada.ts`). Its record cites an animation that draws the upper
dot first and then the lower one.

**What happened:** the first path was labelled "draw the upper dot as a closed
loop", but it drew its first loop at y 28 to 195. The lower dot sits there. The
loop labelled "lower" was the upper dot (y 290 to 478). The book's ಃ strip
taught the wrong order. Every test still passed. The strokes test
pinned only the two labels and the lift count. The honesty checks only ask
whether each path lies on ink, which both loops did.

**Why:** pen paths use font units, and in font units **y points up** (the
baseline is 0). Screen and SVG coordinates point down. A path hand-fitted in
screen terms ("upper" means smaller y) is upside down in font units, and no
check on labels or ink can see that.

**Fix:** the loops were refitted with the upper dot first. A test now asserts
that the first loop's mean y is above the second's. It also checks each loop's
turning direction with a shoelace sum (positive means anticlockwise in y-up).

**Do differently:** when a caption names a place ("upper", "left", "the top
bar") or a direction ("clockwise"), pin it with a geometric assertion on the
points, such as mean y, minimum x or the sign of the shoelace sum. A label
match alone is not enough. When fitting a path, read coordinates from a render
with labelled font-unit axes, not from a screen image.
