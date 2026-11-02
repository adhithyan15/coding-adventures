use strict;
use warnings;
use Test2::V0;
use Math::BigInt;

use CodingAdventures::Rng;

# The shared rng corpus (spec rng.md, "Conformance corpus"), read from its
# flat form code/specs/fixtures/rng-v1/cases.txt.
#
# The corpus pins exact outputs every language's rng package must reproduce;
# its results come from the independent reference
# code/scripts/rng_conformance.py. Each line is whitespace-separated:
#
#     case <id> <generator> <seed>
#     next_u32 <result>
#     next_u64 <result>
#     next_float <16 hex digits of the IEEE-754 bits>
#     next_int_in_range <min> <max> <result | error>
#     end
#
# prove runs from the package directory. Values are compared as decimal
# strings, so 64-bit results never pass through a lossy float.

my $CORPUS = '../../../specs/fixtures/rng-v1/cases.txt';
my %GENERATORS = (
    lcg        => 'CodingAdventures::Rng::LCG',
    xorshift64 => 'CodingAdventures::Rng::Xorshift64',
    pcg32      => 'CodingAdventures::Rng::PCG32',
);

sub float_bits {
    my ($x) = @_;
    return unpack('H16', pack('d>', $x));
}

open my $fh, '<', $CORPUS or die "cannot open $CORPUS: $!";
my ($cases, $calls, $id, $g) = (0, 0, '', undef);
while (my $line = <$fh>) {
    chomp $line;
    next if $line eq '' || $line =~ /\A#/;
    my @t = split / /, $line;
    my $op = $t[0];
    if ($op eq 'case') {
        $id = $t[1];
        $g  = $GENERATORS{ $t[2] }->new($t[3]);
        $cases++;
    }
    elsif ($op eq 'end') {
        $g = undef;
    }
    elsif ($op eq 'next_u32') {
        $calls++;
        is("" . $g->next_u32, $t[1], "$id next_u32");
    }
    elsif ($op eq 'next_u64') {
        $calls++;
        is("" . $g->next_u64, $t[1], "$id next_u64");
    }
    elsif ($op eq 'next_float') {
        $calls++;
        is(float_bits($g->next_float), $t[1], "$id next_float");
    }
    elsif ($op eq 'next_int_in_range') {
        $calls++;
        if ($t[3] eq 'error') {
            # Croaks before any draw, so the sequence carries on.
            like(dies { $g->next_int_in_range($t[1], $t[2]) }, qr/next_int_in_range/, "$id error");
        }
        else {
            is("" . $g->next_int_in_range($t[1], $t[2]), $t[3], "$id next_int_in_range");
        }
    }
    else {
        fail("malformed corpus line: $line");
    }
}
close $fh;

# Guard against a truncated or empty file passing vacuously.
ok($cases >= 27, "corpus has $cases cases");
ok($calls >= 150, "corpus has $calls calls");

# ── Regressions for the v1 range bug ─────────────────────────────────────────

subtest 'wide ranges reach beyond 2^32' => sub {
    my $g = CodingAdventures::Rng::PCG32->new(3);
    my @v = map { $g->next_int_in_range(0, 2**40) } 1 .. 64;
    ok((grep { $_ >= 2**32 } @v), 'some draw above 2^32');
};

subtest 'the whole signed 64-bit domain' => sub {
    my $g = CodingAdventures::Rng::LCG->new(3);
    my @v = map { $g->next_int_in_range('-9223372036854775808', '9223372036854775807') } 1 .. 64;
    ok((grep { $_ < 0 } @v) && (grep { $_ > 0 } @v), 'both signs');
};

subtest 'invalid bounds croak without drawing' => sub {
    for my $bounds ([1, 0], [0.5, 2], ['-9223372036854775809', 0], [0, '9223372036854775808'], ['x', 1]) {
        my $g     = CodingAdventures::Rng::Xorshift64->new(9);
        my $fresh = CodingAdventures::Rng::Xorshift64->new(9);
        ok(dies { $g->next_int_in_range(@$bounds) }, "croaks for [@$bounds]");
        is($g->next_u32, $fresh->next_u32, 'no draw consumed');
    }
};

done_testing;
