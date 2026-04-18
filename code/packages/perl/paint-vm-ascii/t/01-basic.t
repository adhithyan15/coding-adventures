use strict;
use warnings;
use utf8;

use Test2::V0;

use lib '../paint-instructions/lib';
use lib 'lib';

use CodingAdventures::PaintInstructions;
use CodingAdventures::PaintVmAscii;

is(CodingAdventures::PaintVmAscii->VERSION, '0.1.0', 'version matches');

my $filled_scene = CodingAdventures::PaintInstructions->paint_scene(
    3,
    2,
    [
        CodingAdventures::PaintInstructions->paint_rect(0, 0, 2, 1, '#000000'),
    ],
    '#ffffff',
);

like(
    CodingAdventures::PaintVmAscii->render($filled_scene, { scale_x => 1, scale_y => 1 }),
    qr/\x{2588}/,
    'filled rect renders block characters',
);

my $transparent_scene = CodingAdventures::PaintInstructions->paint_scene(
    3,
    2,
    [
        CodingAdventures::PaintInstructions->paint_rect(0, 0, 2, 1, 'transparent'),
    ],
    '#ffffff',
);

is(
    CodingAdventures::PaintVmAscii->render($transparent_scene, { scale_x => 1, scale_y => 1 }),
    '',
    'transparent rect produces empty output',
);

done_testing;
