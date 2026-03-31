# draw-instructions-text

ASCII/Unicode text renderer for the `DrawInstructions` scene model.

This package converts draw instruction scene hashrefs into box-drawing character strings suitable for terminal output. It proves the draw-instructions abstraction is truly backend-neutral: the same scene that produces SVG can also render as Unicode art.

## How It Works

The renderer maps pixel-coordinate scenes to a fixed-width character grid. Each cell is one character. The mapping uses a configurable scale factor (default: 8px per column, 16px per row).

### Character Palette

| Character | Purpose |
|-----------|---------|
| `\x{250C} \x{2510} \x{2514} \x{2518}` | Corners |
| `\x{2500} \x{2502}` | Horizontal / vertical edges |
| `\x{252C} \x{2534} \x{251C} \x{2524}` | Tee junctions |
| `\x{253C}` | Cross junction |
| `\x{2588}` | Filled block |

### Intersection Merging

When two drawing operations overlap at the same cell, the renderer merges them using a direction bitmask (UP, DOWN, LEFT, RIGHT) and resolves the combined tag to the correct box-drawing character.

## Usage

```perl
use CodingAdventures::DrawInstructions;
use CodingAdventures::DrawInstructionsText;

my $scene = CodingAdventures::DrawInstructions::create_scene(160, 48, [
    CodingAdventures::DrawInstructions::draw_rect(
        0, 0, 160, 48, "transparent",
        stroke => "#000", stroke_width => 1),
    CodingAdventures::DrawInstructions::draw_line(0, 16, 160, 16, "#000"),
    CodingAdventures::DrawInstructions::draw_text(8, 8, "Hello"),
], "#fff");

my $text = CodingAdventures::DrawInstructionsText::render_text($scene);
print "$text\n";
```

### Custom Scale

```perl
my $text = CodingAdventures::DrawInstructionsText::render_text(
    $scene, scale_x => 4, scale_y => 8);
```

## Dependencies

- `CodingAdventures::DrawInstructions` (sibling package)
- `Test2::V0` (for testing)

## Part of coding-adventures

This package is part of the coding-adventures educational computing stack.
