use strict;
use warnings;
use Test::More;

sub lives (&) {
    my ($code) = @_;
    return eval { $code->(); 1 } ? 1 : 0;
}

ok lives {
    require CodingAdventures::Heap;
    1;
}, 'CodingAdventures::Heap loads';

done_testing;
