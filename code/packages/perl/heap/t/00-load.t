use strict;
use warnings;
use Test2::V0;

ok lives {
    require CodingAdventures::Heap;
    1;
}, 'CodingAdventures::Heap loads';

done_testing;
