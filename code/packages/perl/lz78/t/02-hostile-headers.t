use strict;
use warnings;
use Test::More;
use Time::HiRes qw(time);

use CodingAdventures::LZ78 qw(compress decompress);

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

sub hdr { return u32($_[0]) . u32($_[1]) }

subtest 'hostile headers' => sub {
    is(quick(sub { decompress("\xFF" x 8) }), '', 'max header only');
    is(quick(sub { decompress(hdr(0xFFFFFFFF, 0)) }), '', 'max length, zero tokens');
    is(quick(sub { decompress(hdr(2, 0xFFFFFFFF) . bin(0, 0, 104, 0, 0, 0, 105, 0)) }), 'hi', 'count larger than data');
    is(quick(sub { decompress(hdr(0xFFFFFFFF, 1) . bin(0, 0, 104, 0)) }), 'h', 'length larger than tokens supply');
    is(quick(sub { decompress(hdr(10, 2) . bin(0, 0, 97, 0, 0x27, 0x0F, 98, 0)) }), 'a', 'dangling dict index');
    my $n = 3000;
    my $body = join '', map { pack('nCC', $_, 97, 0) } 0 .. $n - 1;
    my $out = quick(sub { decompress(hdr(1000, $n) . $body) });
    is(length($out), 1000, 'chain tokens stop at declared length');
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
