use strict;
use warnings;
use Test2::V0;
use FindBin;

# t/01-basic.t -- exercises the Rust-backed GF(256) XSUBs through the real
# DynaLoader path: the cdylib staged at
#     lib/auto/CodingAdventures/GF256Native/GF256Native.<dlext>
# is loaded by `bootstrap`.  The BUILD recipe on every OS (Unix `BUILD`,
# Windows `BUILD_windows`) stages that library before running prove, so a
# failure to load it is a real failure here, never a skip.
#
# GF(256) with the QR/Reed-Solomon polynomial 0x11D: addition and subtraction
# are XOR, multiplication goes through log/antilog tables.

use lib "$FindBin::Bin/../lib";

require CodingAdventures::GF256Native;
my $GF = 'CodingAdventures::GF256Native';

subtest 'add and subtract are XOR' => sub {
    is($GF->can('add')->(83, 202),      153, 'add(83, 202) = 83 xor 202');
    is($GF->can('add')->(1, 2),         3,   'add(1, 2)');
    is($GF->can('subtract')->(83, 202), 153, 'subtract equals add in characteristic 2');
    is($GF->can('add')->(77, 77),       0,   'x + x = 0');
};

subtest 'multiply, divide, power' => sub {
    is($GF->can('multiply')->(2, 16), 32, 'multiply(2, 16)');
    is($GF->can('multiply')->(0, 99), 0,  'multiply by zero');
    is($GF->can('multiply')->(1, 99), 99, 'multiply by one');
    is($GF->can('divide')->(4, 2),    2,  'divide(4, 2)');
    is($GF->can('power')->(2, 8),     29, 'power(2, 8) wraps through 0x11D');
    is($GF->can('power')->(7, 0),     1,  'power(x, 0) = 1');
};

subtest 'inverse is the multiplicative inverse' => sub {
    for my $x (1, 2, 83, 140, 255) {
        my $inv = $GF->can('inverse')->($x);
        is($GF->can('multiply')->($x, $inv), 1, "x * inverse(x) = 1 for $x");
    }
};

# The Rust side turns a panic into a Perl `die`; this runs last so that, if the
# error path ever misbehaved on some platform, every assertion above has already
# reported.
subtest 'zero divisor dies instead of crashing the interpreter' => sub {
    like(dies { $GF->can('divide')->(1, 0) }, qr/division by zero/, 'divide by zero');
    like(dies { $GF->can('inverse')->(0) },   qr/zero|inverse/i,    'inverse of zero');
};

done_testing;
