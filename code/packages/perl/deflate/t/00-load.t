use strict;
use warnings;
use Test2::V0;

use CodingAdventures::Deflate qw(compress decompress);

ok(1, 'module loaded successfully');
ok(defined &compress,   'compress function is defined');
ok(defined &decompress, 'decompress function is defined');

done_testing;
