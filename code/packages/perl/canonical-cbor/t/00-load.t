use strict;
use warnings;
use Test2::V0;

ok(eval { require CodingAdventures::CanonicalCbor; 1 }, 'CodingAdventures::CanonicalCbor loads');
ok(eval { require CodingAdventures::CanonicalCbor::Constants; 1 }, 'Constants loads');
ok(eval { require CodingAdventures::CanonicalCbor::Messages; 1 }, 'Messages loads');
ok(eval { require CodingAdventures::CanonicalCbor::Types; 1 }, 'Types loads');
ok(eval { require CodingAdventures::CanonicalCbor::Codec; 1 }, 'Codec loads');

done_testing;
