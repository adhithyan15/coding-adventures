use strict;
use warnings;
use Test::More tests => 2;
use lib 'lib';
use GradientDescent qw(sgd);

my $res = sgd([1.0, 2.0], [0.1, 0.2], 0.5);
ok(abs($res->[0] - 0.95) < 0.0001, 'sgd 1');
ok(abs($res->[1] - 1.9) < 0.0001, 'sgd 2');
