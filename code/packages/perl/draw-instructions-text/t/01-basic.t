use strict;
use warnings;
use utf8;
use Test2::V0;

use CodingAdventures::DrawInstructions;
use CodingAdventures::DrawInstructionsText;

# Convenience aliases
my $di  = 'CodingAdventures::DrawInstructions';
my $txt = 'CodingAdventures::DrawInstructionsText';

# All tests use 1:1 scale for easy reasoning unless noted otherwise.

# ---------------------------------------------------------------------------
# 1. Stroked rectangle — box with corners and edges
# ---------------------------------------------------------------------------
# A stroked 4x2 rectangle should produce:
#   ┌───┐
#   │   │
#   └───┘

subtest 'stroked rectangle draws box with corners and edges' => sub {
    my $scene = $di->can('create_scene')->(5, 3, [
        $di->can('draw_rect')->(0, 0, 4, 2, 'transparent',
            stroke => '#000', stroke_width => 1),
    ], '#fff');

    my $result = $txt->can('render_text')->($scene, scale_x => 1, scale_y => 1);

    my @lines = split(/\n/, $result);
    is(scalar @lines, 3, 'three rows');
    is($lines[0], "\x{250C}\x{2500}\x{2500}\x{2500}\x{2510}", 'top row: corners and horizontal');
    is($lines[1], "\x{2502}   \x{2502}", 'middle row: vertical edges');
    is($lines[2], "\x{2514}\x{2500}\x{2500}\x{2500}\x{2518}", 'bottom row: corners and horizontal');
};

# ---------------------------------------------------------------------------
# 2. Filled rectangle — block characters
# ---------------------------------------------------------------------------

subtest 'filled rectangle uses block characters' => sub {
    my $scene = $di->can('create_scene')->(3, 2, [
        $di->can('draw_rect')->(0, 0, 2, 1, '#000'),
    ], '#fff');

    my $result = $txt->can('render_text')->($scene, scale_x => 1, scale_y => 1);
    like($result, qr/\x{2588}/, 'contains block character');
};

# ---------------------------------------------------------------------------
# 3. Transparent rect with no stroke is invisible
# ---------------------------------------------------------------------------

subtest 'transparent rect with no stroke is invisible' => sub {
    my $scene = $di->can('create_scene')->(5, 3, [
        $di->can('draw_rect')->(0, 0, 4, 2, 'transparent'),
    ], '#fff');

    my $result = $txt->can('render_text')->($scene, scale_x => 1, scale_y => 1);
    is($result, '', 'empty result for transparent rect');
};

# ---------------------------------------------------------------------------
# 4. Horizontal line
# ---------------------------------------------------------------------------

subtest 'horizontal line' => sub {
    my $scene = $di->can('create_scene')->(5, 1, [
        $di->can('draw_line')->(0, 0, 4, 0, '#000'),
    ], '#fff');

    my $result = $txt->can('render_text')->($scene, scale_x => 1, scale_y => 1);
    is($result, "\x{2500}\x{2500}\x{2500}\x{2500}\x{2500}", 'five horizontal chars');
};

# ---------------------------------------------------------------------------
# 5. Vertical line
# ---------------------------------------------------------------------------

subtest 'vertical line' => sub {
    my $scene = $di->can('create_scene')->(1, 3, [
        $di->can('draw_line')->(0, 0, 0, 2, '#000'),
    ], '#fff');

    my $result = $txt->can('render_text')->($scene, scale_x => 1, scale_y => 1);
    is($result, "\x{2502}\n\x{2502}\n\x{2502}", 'three vertical chars');
};

# ---------------------------------------------------------------------------
# 6. Crossing lines produce a cross character
# ---------------------------------------------------------------------------

subtest 'crossing lines produce cross' => sub {
    my $scene = $di->can('create_scene')->(5, 3, [
        $di->can('draw_line')->(0, 1, 4, 1, '#000'),
        $di->can('draw_line')->(2, 0, 2, 2, '#000'),
    ], '#fff');

    my $result = $txt->can('render_text')->($scene, scale_x => 1, scale_y => 1);
    my @lines = split(/\n/, $result);

    is(substr($lines[0], 2, 1), "\x{2502}", 'row 0: vertical at col 2');
    is(substr($lines[1], 2, 1), "\x{253C}", 'row 1: cross at col 2');
    is(substr($lines[2], 2, 1), "\x{2502}", 'row 2: vertical at col 2');
};

# ---------------------------------------------------------------------------
# 7. Table grid — box with internal horizontal line
# ---------------------------------------------------------------------------

subtest 'table-like grid with tee characters' => sub {
    my $scene = $di->can('create_scene')->(7, 3, [
        $di->can('draw_rect')->(0, 0, 6, 2, 'transparent',
            stroke => '#000', stroke_width => 1),
        $di->can('draw_line')->(0, 1, 6, 1, '#000'),
    ], '#fff');

    my $result = $txt->can('render_text')->($scene, scale_x => 1, scale_y => 1);
    my @lines = split(/\n/, $result);

    is($lines[0], "\x{250C}\x{2500}\x{2500}\x{2500}\x{2500}\x{2500}\x{2510}", 'top row');
    is(substr($lines[1], 0, 1), "\x{251C}", 'left tee');
    is(substr($lines[1], 6, 1), "\x{2524}", 'right tee');
    is($lines[2], "\x{2514}\x{2500}\x{2500}\x{2500}\x{2500}\x{2500}\x{2518}", 'bottom row');
};

# ---------------------------------------------------------------------------
# 8. Text with start alignment
# ---------------------------------------------------------------------------

subtest 'text with start alignment' => sub {
    my $scene = $di->can('create_scene')->(10, 1, [
        $di->can('draw_text')->(0, 0, 'Hello', '#000', 'monospace', 16, 'start'),
    ], '#fff');

    my $result = $txt->can('render_text')->($scene, scale_x => 1, scale_y => 1);
    is($result, 'Hello', 'text starts at position');
};

# ---------------------------------------------------------------------------
# 9. Text with middle alignment
# ---------------------------------------------------------------------------

subtest 'text with middle alignment' => sub {
    my $scene = $di->can('create_scene')->(10, 1, [
        $di->can('draw_text')->(5, 0, 'Hi'),
    ], '#fff');

    my $result = $txt->can('render_text')->($scene, scale_x => 1, scale_y => 1);
    is(substr($result, 4, 1), 'H', 'H at col 4');
    is(substr($result, 5, 1), 'i', 'i at col 5');
};

# ---------------------------------------------------------------------------
# 10. Text with end alignment
# ---------------------------------------------------------------------------

subtest 'text with end alignment' => sub {
    my $scene = $di->can('create_scene')->(10, 1, [
        $di->can('draw_text')->(9, 0, 'End', '#000', 'monospace', 16, 'end'),
    ], '#fff');

    my $result = $txt->can('render_text')->($scene, scale_x => 1, scale_y => 1);
    is(substr($result, 6, 1), 'E', 'E at col 6');
    is(substr($result, 7, 1), 'n', 'n at col 7');
    is(substr($result, 8, 1), 'd', 'd at col 8');
};

# ---------------------------------------------------------------------------
# 11. Text inside a stroked rectangle
# ---------------------------------------------------------------------------

subtest 'text inside a stroked rectangle' => sub {
    my $scene = $di->can('create_scene')->(12, 3, [
        $di->can('draw_rect')->(0, 0, 11, 2, 'transparent',
            stroke => '#000', stroke_width => 1),
        $di->can('draw_text')->(1, 1, 'Hello', '#000', 'monospace', 16, 'start'),
    ], '#fff');

    my $result = $txt->can('render_text')->($scene, scale_x => 1, scale_y => 1);
    my @lines = split(/\n/, $result);

    is($lines[0], "\x{250C}\x{2500}\x{2500}\x{2500}\x{2500}\x{2500}\x{2500}\x{2500}\x{2500}\x{2500}\x{2500}\x{2510}", 'top border');
    is($lines[1], "\x{2502}Hello     \x{2502}", 'text row');
    is($lines[2], "\x{2514}\x{2500}\x{2500}\x{2500}\x{2500}\x{2500}\x{2500}\x{2500}\x{2500}\x{2500}\x{2500}\x{2518}", 'bottom border');
};

# ---------------------------------------------------------------------------
# 12. Clip — text clipped to region
# ---------------------------------------------------------------------------

subtest 'clips text beyond region' => sub {
    my $scene = $di->can('create_scene')->(10, 1, [
        $di->can('draw_clip')->(0, 0, 3, 1, [
            $di->can('draw_text')->(0, 0, 'Hello World', '#000', 'monospace', 16, 'start'),
        ]),
    ], '#fff');

    my $result = $txt->can('render_text')->($scene, scale_x => 1, scale_y => 1);
    is($result, 'Hel', 'only first 3 chars visible');
};

# ---------------------------------------------------------------------------
# 13. Nested clips intersect
# ---------------------------------------------------------------------------

subtest 'nested clips intersect properly' => sub {
    my $scene = $di->can('create_scene')->(10, 1, [
        $di->can('draw_clip')->(0, 0, 5, 1, [
            $di->can('draw_clip')->(2, 0, 5, 1, [
                $di->can('draw_text')->(0, 0, 'ABCDEFGHIJ', '#000', 'monospace', 16, 'start'),
            ]),
        ]),
    ], '#fff');

    my $result = $txt->can('render_text')->($scene, scale_x => 1, scale_y => 1);
    # Outer: cols 0-4, inner: cols 2-6, intersection: cols 2-4
    is($result, '  CDE', 'intersection of nested clips');
};

# ---------------------------------------------------------------------------
# 14. Groups — recursive rendering
# ---------------------------------------------------------------------------

subtest 'groups recurse into children' => sub {
    my $scene = $di->can('create_scene')->(5, 1, [
        $di->can('draw_group')->([
            $di->can('draw_text')->(0, 0, 'AB', '#000', 'monospace', 16, 'start'),
            $di->can('draw_text')->(3, 0, 'CD', '#000', 'monospace', 16, 'start'),
        ]),
    ], '#fff');

    my $result = $txt->can('render_text')->($scene, scale_x => 1, scale_y => 1);
    is($result, 'AB CD', 'both text fragments present');
};

# ---------------------------------------------------------------------------
# 15. Nested groups
# ---------------------------------------------------------------------------

subtest 'nested groups work' => sub {
    my $inner = $di->can('draw_group')->([
        $di->can('draw_text')->(0, 0, 'X', '#000', 'monospace', 16, 'start'),
    ]);
    my $outer = $di->can('draw_group')->([$inner]);
    my $scene = $di->can('create_scene')->(3, 1, [$outer], '#fff');

    my $result = $txt->can('render_text')->($scene, scale_x => 1, scale_y => 1);
    is($result, 'X', 'nested group renders');
};

# ---------------------------------------------------------------------------
# 16. Complete table demo
# ---------------------------------------------------------------------------

subtest 'complete table with headers and data' => sub {
    my $scene = $di->can('create_scene')->(13, 6, [
        # Outer border
        $di->can('draw_rect')->(0, 0, 12, 5, 'transparent',
            stroke => '#000', stroke_width => 1),
        # Vertical divider at x=6
        $di->can('draw_line')->(6, 0, 6, 5, '#000'),
        # Horizontal divider at y=2
        $di->can('draw_line')->(0, 2, 12, 2, '#000'),
        # Header text
        $di->can('draw_text')->(1, 1, 'Name', '#000', 'monospace', 16, 'start'),
        $di->can('draw_text')->(7, 1, 'Age', '#000', 'monospace', 16, 'start'),
        # Data row 1
        $di->can('draw_text')->(1, 3, 'Alice', '#000', 'monospace', 16, 'start'),
        $di->can('draw_text')->(7, 3, '30', '#000', 'monospace', 16, 'start'),
        # Data row 2
        $di->can('draw_text')->(1, 4, 'Bob', '#000', 'monospace', 16, 'start'),
        $di->can('draw_text')->(7, 4, '25', '#000', 'monospace', 16, 'start'),
    ], '#fff');

    my $result = $txt->can('render_text')->($scene, scale_x => 1, scale_y => 1);
    my @lines = split(/\n/, $result);

    # ┌─────┬─────┐
    is($lines[0],
        "\x{250C}\x{2500}\x{2500}\x{2500}\x{2500}\x{2500}\x{252C}\x{2500}\x{2500}\x{2500}\x{2500}\x{2500}\x{2510}",
        'top border with tee');

    like($lines[1], qr/Name/, 'header has Name');
    like($lines[1], qr/Age/, 'header has Age');

    # ├─────┼─────┤
    is(substr($lines[2], 0, 1), "\x{251C}", 'left tee');
    is(substr($lines[2], 6, 1), "\x{253C}", 'cross at divider');
    is(substr($lines[2], 12, 1), "\x{2524}", 'right tee');

    like($lines[3], qr/Alice/, 'data has Alice');
    like($lines[3], qr/30/, 'data has 30');
    like($lines[4], qr/Bob/, 'data has Bob');
    like($lines[4], qr/25/, 'data has 25');

    # └─────┴─────┘
    is($lines[5],
        "\x{2514}\x{2500}\x{2500}\x{2500}\x{2500}\x{2500}\x{2534}\x{2500}\x{2500}\x{2500}\x{2500}\x{2500}\x{2518}",
        'bottom border with tee');
};

# ---------------------------------------------------------------------------
# 17. Default scale factor
# ---------------------------------------------------------------------------

subtest 'default scale maps pixel coordinates' => sub {
    # Default: 8px/col, 16px/row
    # Rect at (0,0) with width=80 height=32 -> 10 cols, 2 rows -> 3 row box
    my $scene = $di->can('create_scene')->(88, 48, [
        $di->can('draw_rect')->(0, 0, 80, 32, 'transparent',
            stroke => '#000', stroke_width => 1),
    ], '#fff');

    my $result = $txt->can('render_text')->($scene);
    my @lines = split(/\n/, $result);

    is(scalar @lines, 3, 'three rows');
    is(substr($lines[0], 0, 1), "\x{250C}", 'top-left corner');
    is(substr($lines[2], 0, 1), "\x{2514}", 'bottom-left corner');
};

# ---------------------------------------------------------------------------
# 18. Custom scale factor
# ---------------------------------------------------------------------------

subtest 'custom scale factor' => sub {
    my $scene = $di->can('create_scene')->(12, 8, [
        $di->can('draw_line')->(0, 0, 12, 0, '#000'),
    ], '#fff');

    my $result = $txt->can('render_text')->($scene, scale_x => 4, scale_y => 4);
    like($result, qr/\x{2500}/, 'contains horizontal character');
};

# ---------------------------------------------------------------------------
# 19. Empty scene
# ---------------------------------------------------------------------------

subtest 'empty scene returns empty string' => sub {
    my $scene = $di->can('create_scene')->(0, 0, [], '#fff');
    my $result = $txt->can('render_text')->($scene, scale_x => 1, scale_y => 1);
    is($result, '', 'empty for zero-size scene');
};

subtest 'scene with no instructions returns empty string' => sub {
    my $scene = $di->can('create_scene')->(5, 5, [], '#fff');
    my $result = $txt->can('render_text')->($scene, scale_x => 1, scale_y => 1);
    is($result, '', 'empty for no instructions');
};

# ---------------------------------------------------------------------------
# 20. Text overwrites box-drawing
# ---------------------------------------------------------------------------

subtest 'text overwrites box-drawing characters' => sub {
    my $scene = $di->can('create_scene')->(5, 1, [
        $di->can('draw_line')->(0, 0, 4, 0, '#000'),
        $di->can('draw_text')->(1, 0, 'X', '#000', 'monospace', 16, 'start'),
    ], '#fff');

    my $result = $txt->can('render_text')->($scene, scale_x => 1, scale_y => 1);
    is(substr($result, 0, 1), "\x{2500}", 'col 0 is horizontal');
    is(substr($result, 1, 1), 'X', 'col 1 is text');
    is(substr($result, 2, 1), "\x{2500}", 'col 2 is horizontal');
};

# ---------------------------------------------------------------------------
# 21. Box-drawing does not overwrite text
# ---------------------------------------------------------------------------

subtest 'box-drawing does not overwrite text' => sub {
    my $scene = $di->can('create_scene')->(5, 1, [
        $di->can('draw_text')->(1, 0, 'X', '#000', 'monospace', 16, 'start'),
        $di->can('draw_line')->(0, 0, 4, 0, '#000'),
    ], '#fff');

    my $result = $txt->can('render_text')->($scene, scale_x => 1, scale_y => 1);
    is(substr($result, 1, 1), 'X', 'text preserved');
};

# ---------------------------------------------------------------------------
# 22. Diagonal line
# ---------------------------------------------------------------------------

subtest 'diagonal line produces characters' => sub {
    my $scene = $di->can('create_scene')->(4, 4, [
        $di->can('draw_line')->(0, 0, 3, 3, '#000'),
    ], '#fff');

    my $result = $txt->can('render_text')->($scene, scale_x => 1, scale_y => 1);
    ok(length($result) > 0, 'non-empty for diagonal');
    my @lines = split(/\n/, $result);
    ok(scalar @lines >= 3, 'at least 3 lines for diagonal');
};

# ---------------------------------------------------------------------------
# 23. Trailing whitespace trimming
# ---------------------------------------------------------------------------

subtest 'trims trailing whitespace per line' => sub {
    my $scene = $di->can('create_scene')->(10, 2, [
        $di->can('draw_text')->(0, 0, 'Hi', '#000', 'monospace', 16, 'start'),
        $di->can('draw_text')->(0, 1, 'Lo', '#000', 'monospace', 16, 'start'),
    ], '#fff');

    my $result = $txt->can('render_text')->($scene, scale_x => 1, scale_y => 1);
    my @lines = split(/\n/, $result);

    for my $line (@lines) {
        unlike($line, qr/\s$/, "no trailing whitespace: '$line'");
    }
};

# ---------------------------------------------------------------------------
# 24. Rect with fill 'none' is invisible
# ---------------------------------------------------------------------------

subtest 'rect with fill none and no stroke is invisible' => sub {
    my $scene = $di->can('create_scene')->(5, 3, [
        $di->can('draw_rect')->(0, 0, 4, 2, 'none'),
    ], '#fff');

    my $result = $txt->can('render_text')->($scene, scale_x => 1, scale_y => 1);
    is($result, '', 'empty result');
};

done_testing;
