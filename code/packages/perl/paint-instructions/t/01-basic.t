use strict;
use warnings;
use Test2::V0;

require CodingAdventures::PaintInstructions;
my $pkg = 'CodingAdventures::PaintInstructions';

my $rect = $pkg->paint_rect(1, 2, 3, 4);
is($rect->{kind}, 'rect', 'rect kind');
is($rect->{width}, 3, 'rect width');

my $scene = $pkg->paint_scene(10, 20, [$rect]);
is($scene->{width}, 10, 'scene width');
is($scene->{height}, 20, 'scene height');

done_testing;
