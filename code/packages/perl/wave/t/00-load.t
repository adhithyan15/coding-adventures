use strict;
use warnings;
use Test2::V0;

use CodingAdventures::Wave;

ok(1, 'module loads');
is($CodingAdventures::Wave::VERSION, '0.01', 'has VERSION 0.01');

done_testing;
