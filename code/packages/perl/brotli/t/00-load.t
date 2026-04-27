use strict;
use warnings;
use Test::More tests => 3;

use CodingAdventures::Brotli qw(compress decompress);

ok(1, 'module loaded successfully');
ok(defined &compress,   'compress function is defined');
ok(defined &decompress, 'decompress function is defined');
