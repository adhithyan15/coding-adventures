use strict;
use warnings;
use Test::More tests => 3;
use lib 'lib';
use ActivationFunctions qw(sigmoid relu tanh);

ok(abs(sigmoid(0) - 0.5) < 0.0001, 'sigmoid');
is(relu(5), 5.0, 'relu pos');
ok(abs(tanh(0) - 0.0) < 0.0001, 'tanh');
