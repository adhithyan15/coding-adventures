use strict;
use warnings;
use utf8;

use Test2::V0;

use lib '../paint-instructions/lib';
use lib 'lib';

use CodingAdventures::PaintInstructions;
use CodingAdventures::PaintVmAscii;

# --- glyph_run -------------------------------------------------------------

subtest 'glyph_run places literal characters at their scene positions' => sub {
    my $scene = CodingAdventures::PaintInstructions->paint_scene(
        16, 16,
        [
            {
                kind      => 'glyph_run',
                glyphs    => [
                    { glyph_id => ord('h'), x => 0, y => 0 },
                    { glyph_id => ord('i'), x => 8, y => 0 },
                ],
                font_ref  => 'terminal-mono',
                font_size => 16,
                fill      => '#000000',
            },
        ],
        'transparent',
    );

    is(
        CodingAdventures::PaintVmAscii->render($scene, { scale_x => 8, scale_y => 16 }),
        'hi',
        'glyph_run renders the literal characters',
    );
};

subtest 'glyph_run maps unsafe code points to a placeholder' => sub {
    my $scene = CodingAdventures::PaintInstructions->paint_scene(
        16, 16,
        [
            {
                kind      => 'glyph_run',
                glyphs    => [ { glyph_id => 0x07, x => 0, y => 0 } ],    # BEL, a C0 control char
                font_ref  => 'terminal-mono',
                font_size => 16,
                fill      => '#000000',
            },
        ],
        'transparent',
    );

    is(
        CodingAdventures::PaintVmAscii->render($scene, { scale_x => 8, scale_y => 16 }),
        '?',
        'unsafe control code point becomes a placeholder',
    );
};

# --- line --------------------------------------------------------------------

subtest 'horizontal line draws box-drawing characters' => sub {
    my $scene = CodingAdventures::PaintInstructions->paint_scene(
        32, 16,
        [
            { kind => 'line', x1 => 0, y1 => 0, x2 => 24, y2 => 0, stroke => '#000000', stroke_width => 1 },
        ],
        'transparent',
    );

    is(
        CodingAdventures::PaintVmAscii->render($scene, { scale_x => 8, scale_y => 16 }),
        "\x{2500}\x{2500}\x{2500}\x{2500}",
        'horizontal line renders as a run of horizontal box characters',
    );
};

subtest 'vertical line draws box-drawing characters' => sub {
    my $scene = CodingAdventures::PaintInstructions->paint_scene(
        8, 48,
        [
            { kind => 'line', x1 => 0, y1 => 0, x2 => 0, y2 => 32, stroke => '#000000', stroke_width => 1 },
        ],
        'transparent',
    );

    is(
        CodingAdventures::PaintVmAscii->render($scene, { scale_x => 8, scale_y => 16 }),
        "\x{2502}\n\x{2502}\n\x{2502}",
        'vertical line renders as a column of vertical box characters',
    );
};

# --- rect with stroke ---------------------------------------------------------

subtest 'stroked rect draws box-drawing corners and edges' => sub {
    my $scene = CodingAdventures::PaintInstructions->paint_scene(
        24, 32,
        [
            {
                kind         => 'rect',
                x            => 0,
                y            => 0,
                width        => 16,
                height       => 16,
                stroke       => '#000000',
                stroke_width => 1,
            },
        ],
        'transparent',
    );

    my $output = CodingAdventures::PaintVmAscii->render($scene, { scale_x => 8, scale_y => 16 });
    my @lines = split /\n/, $output;

    is(scalar @lines, 2, 'stroked rect spans the expected number of rows');
    like($lines[0], qr/^\x{250C}\x{2500}\x{2510}$/, 'top row has the two corners with a horizontal edge between');
    like($lines[1], qr/^\x{2514}\x{2500}\x{2518}$/, 'bottom row has the two corners with a horizontal edge between');
};

subtest 'rect with an undefined fill has no fill (not black)' => sub {
    my $scene = CodingAdventures::PaintInstructions->paint_scene(
        8, 16,
        [
            { kind => 'rect', x => 0, y => 0, width => 8, height => 16 },
        ],
        'transparent',
    );

    is(
        CodingAdventures::PaintVmAscii->render($scene, { scale_x => 8, scale_y => 16 }),
        '',
        'an omitted fill produces no output, matching the P2D00 "no fill" default',
    );
};

# --- group / clip / layer -----------------------------------------------------

subtest 'group recurses into its children' => sub {
    my $scene = CodingAdventures::PaintInstructions->paint_scene(
        16, 16,
        [
            {
                kind     => 'group',
                children => [
                    CodingAdventures::PaintInstructions->paint_rect(0, 0, 8, 16, '#000000'),
                ],
            },
        ],
        'transparent',
    );

    like(
        CodingAdventures::PaintVmAscii->render($scene, { scale_x => 8, scale_y => 16 }),
        qr/\x{2588}/,
        'group children are rendered',
    );
};

subtest 'group with a non-identity transform is rejected' => sub {
    my $scene = CodingAdventures::PaintInstructions->paint_scene(
        16, 16,
        [
            {
                kind      => 'group',
                transform => { a => 2, b => 0, c => 0, d => 1, e => 0, f => 0 },
                children  => [],
            },
        ],
        'transparent',
    );

    like(
        dies { CodingAdventures::PaintVmAscii->render($scene, { scale_x => 8, scale_y => 16 }) },
        qr/does not support transformed groups/,
        'a scaled group transform is rejected loudly',
    );
};

subtest 'layer with filters is rejected' => sub {
    my $scene = CodingAdventures::PaintInstructions->paint_scene(
        16, 16,
        [
            {
                kind     => 'layer',
                filters  => [ { kind => 'blur', radius => 2 } ],
                children => [],
            },
        ],
        'transparent',
    );

    like(
        dies { CodingAdventures::PaintVmAscii->render($scene, { scale_x => 8, scale_y => 16 }) },
        qr/does not support layer filters/,
        'a layer with filters is rejected loudly',
    );
};

subtest 'clip constrains children to the clip rectangle' => sub {
    my $scene = CodingAdventures::PaintInstructions->paint_scene(
        16, 16,
        [
            {
                kind     => 'clip',
                x        => 0,
                y        => 0,
                width    => 8,
                height   => 16,
                children => [
                    {
                        kind      => 'glyph_run',
                        glyphs    => [
                            { glyph_id => ord('a'), x => 0,  y => 0 },
                            { glyph_id => ord('b'), x => 8,  y => 0 },    # outside the clip
                        ],
                        font_ref  => 'terminal-mono',
                        font_size => 16,
                        fill      => '#000000',
                    },
                ],
            },
        ],
        'transparent',
    );

    is(
        CodingAdventures::PaintVmAscii->render($scene, { scale_x => 8, scale_y => 16 }),
        'a',
        'the glyph outside the clip rectangle is dropped',
    );
};

# --- unsupported kind ----------------------------------------------------------

subtest 'an unknown instruction kind dies loudly' => sub {
    my $scene = CodingAdventures::PaintInstructions->paint_scene(
        16, 16,
        [ { kind => 'gradient' } ],
        'transparent',
    );

    like(
        dies { CodingAdventures::PaintVmAscii->render($scene, { scale_x => 8, scale_y => 16 }) },
        qr/unsupported paint instruction kind: gradient/,
        'unsupported kinds fail loudly instead of degrading silently',
    );
};

done_testing;
