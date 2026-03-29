use strict;
use warnings;
use Test2::V0;

use_ok('CodingAdventures::EventLoop');

my @methods = qw(new on once off emit on_tick tick run step);
for my $m (@methods) {
    ok(
        CodingAdventures::EventLoop->can($m),
        "CodingAdventures::EventLoop can $m"
    );
}

ok(defined $CodingAdventures::EventLoop::VERSION, 'has VERSION');

done_testing();
