use strict;
use warnings;
use Test::More;
use Time::HiRes qw(time);

use CodingAdventures::LZW qw(compress decompress);

# Security regression tests: hostile headers must not drive work or memory.
#
# Each case is a few bytes of input whose header claims an enormous count or
# length.  An implementation that sized an allocation or loop from the claim
# would exhaust memory or run for minutes; these tests assert bounded time and
# sane results, plus round trips over every input of length <= 2 and a seeded
# random sample.

my $LIMIT_SECONDS = 10;

sub quick {
    my ($code) = @_;
    my $start  = time;
    my $result = $code->();
    my $took   = time - $start;
    cmp_ok($took, '<', $LIMIT_SECONDS, 'hostile input completed quickly');
    return $result;
}

sub bin { return pack 'C*', @_ }
sub u32 { return pack 'N', $_[0] }

subtest 'hostile headers' => sub {
    quick(sub { decompress("\xFF" x 4) });
    quick(sub { decompress("\xFF" x 64) });
    my $c = compress('hello hello hello');
    substr($c, 0, 4) = u32(0xFFFFFFFF);
    is(quick(sub { decompress($c) }), 'hello hello hello', 'max declared length on a small stream');
    substr($c, 0, 4) = u32(5);
    is(quick(sub { decompress($c) }), 'hello', 'declared length truncates output');
};


subtest 'round trip: every input of length <= 2' => sub {
    my $bad = 0;
    my @inputs = ('');
    for my $a (0 .. 255) {
        push @inputs, bin($a);
        push @inputs, bin($a, $_) for 0 .. 255;
    }
    for my $d (@inputs) {
        $bad++ unless decompress(compress($d)) eq $d;
    }
    is($bad, 0, 'all ' . scalar(@inputs) . ' short inputs round-trip');
};

subtest 'round trip: seeded random sample' => sub {
    srand(0x5EED);
    my @alphabets = (2, 4, 16, 256);
    my $bad = 0;
    for my $i (0 .. 299) {
        my $n = 3 + int(rand(597));
        my $d = bin(map { int(rand($alphabets[$i % 4])) } 1 .. $n);
        $bad++ unless decompress(compress($d)) eq $d;
    }
    is($bad, 0, '300 random inputs round-trip');
};

done_testing;
