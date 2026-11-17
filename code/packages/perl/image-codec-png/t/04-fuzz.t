use strict;
use warnings;
use Test2::V0;
use Scalar::Util qw(blessed);

use CodingAdventures::PixelContainer;
use CodingAdventures::ImageCodecPNG qw(:all);
use CodingAdventures::ImageCodecPNG::Constants qw(:all);

my %KNOWN_CODES = map { $_ => 1 } png_error_codes();

# Base seeds
sub make_seed {
    my ($w, $h, $r, $g, $b, $a) = @_;
    my $c = CodingAdventures::PixelContainer->new($w, $h);
    for (my $y = 0; $y < $h; $y++) {
        for (my $x = 0; $x < $w; $x++) {
            $c->set_pixel($x, $y, $r, $g, $b, $a);
        }
    }
    return encode_png($c);
}

my @seeds = (
    make_seed(1, 1, 100, 150, 200, 255),
    make_seed(4, 4, 10, 20, 30, 40),
    make_seed(8, 2, 255, 0, 128, 255),
);

# Deterministic LCG pseudo-random number generator
my $seed = 1337;
sub prng {
    $seed = ($seed * 1103515245 + 12345) & 0x7FFFFFFF;
    return $seed;
}

sub prng_range {
    my ($min, $max) = @_;
    return $min + (prng() % ($max - $min + 1));
}

sub mutate {
    my ($data) = @_;
    my $buf = $data;
    my $len = length($buf);
    return $buf if $len == 0;

    my $num_mutations = prng_range(1, 4);
    for (1 .. $num_mutations) {
        my $op = prng_range(0, 4);
        $len = length($buf);
        next if $len == 0;
        my $idx = prng_range(0, $len - 1);

        if ($op == 0) {
            # Bit flip
            my $bit = 1 << prng_range(0, 7);
            substr($buf, $idx, 1) = chr(ord(substr($buf, $idx, 1)) ^ $bit);
        } elsif ($op == 1) {
            # Random byte replacement
            substr($buf, $idx, 1) = chr(prng_range(0, 255));
        } elsif ($op == 2) {
            # Byte deletion
            substr($buf, $idx, 1, '');
        } elsif ($op == 3) {
            # Byte insertion
            substr($buf, $idx, 0, chr(prng_range(0, 255)));
        } else {
            # Truncation
            $buf = substr($buf, 0, $idx);
        }
    }
    return $buf;
}

my $iterations = 10_000;
my $successes = 0;
my $failures = 0;
my $uncaught_errors = 0;
my $roundtrip_errors = 0;

for (my $i = 0; $i < $iterations; $i++) {
    my $base = $seeds[$i % scalar(@seeds)];
    my $mutated = mutate($base);

    my $decoded;
    my $err;
    eval {
        $decoded = decode_png($mutated);
        1;
    } or $err = $@;

    if (defined $decoded) {
        $successes++;
        if (!ref($decoded) || !$decoded->isa('CodingAdventures::PixelContainer') ||
            $decoded->width <= 0 || $decoded->height <= 0 ||
            length(${ $decoded->data }) != $decoded->width * $decoded->height * 4) {
            $uncaught_errors++;
            diag("Iter $i: malformed decoded container");
        }

        my $re_encoded = encode_png($decoded);
        my $re_decoded = decode_png($re_encoded);
        if (${ $re_decoded->data } ne ${ $decoded->data }) {
            $roundtrip_errors++;
            diag("Iter $i: lossless re-encode roundtrip mismatch");
        }
    } else {
        $failures++;
        if (!ref($err) || !$err->can('code') || !$KNOWN_CODES{$err->code}) {
            $uncaught_errors++;
            diag("Iter $i: uncaught exception: $err");
        }
    }
}

is($uncaught_errors, 0, 'Zero uncaught or unregistered errors across 10,000 iterations');
is($roundtrip_errors, 0, 'Zero roundtrip mismatches across decoded inputs');
ok($successes > 0, 'Observed valid decoded mutations');
ok($failures > 0, 'Observed invalid mutations cleanly rejected');
diag("Fuzzing finished: $iterations iterations ($successes decoded, $failures cleanly rejected)");

done_testing;
