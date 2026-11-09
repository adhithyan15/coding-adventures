use strict;
use warnings;
use Test::More;
use Time::HiRes qw(time);

use CodingAdventures::LZ77 qw(compress decompress);

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
    is(quick(sub { decompress(bin(255, 255, 255, 255)) }), '', 'max count, no tokens');
    my $d = bin(255, 255, 255, 255, 0, 0, 0, 104, 0, 0, 0, 105);
    is(quick(sub { decompress($d) }), 'hi', 'count larger than data yields present tokens');
    is(decompress($d . bin(0, 0, 0)), 'hi', 'trailing partial token ignored');
    is(decompress(bin(0, 0, 0, 1, 0, 0, 0, 104, 0, 0, 0, 105)), 'h', 'count smaller than data stops at count');
    is(decompress(bin(0, 0, 0, 1, 0, 5, 3, 120)), '', 'offset beyond output stops decoding');
    is(decompress(bin(0, 0, 0, 1, 0, 0, 3, 120)), '', 'zero offset stops decoding');
    is(decompress(bin(0, 0, 0, 2, 0, 0, 0, 97, 0, 9, 2, 122)), 'a', 'valid prefix survives');
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
