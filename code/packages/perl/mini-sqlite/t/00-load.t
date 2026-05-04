use strict;
use warnings;
use Test2::V0;

ok(eval { require CodingAdventures::MiniSqlite; 1 }, 'module loads') or diag($@);

done_testing;
