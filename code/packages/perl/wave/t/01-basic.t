use strict;
use warnings;
use Test2::V0;
use POSIX qw();

use CodingAdventures::Wave qw(
    sine_wave cosine_wave square_wave sawtooth_wave triangle_wave
    dc_offset add_waves scale_wave mix_waves
);
use CodingAdventures::Trig qw(sin_approx cos_approx);

# ---------------------------------------------------------------------------
# Helper: approximate equality
# ---------------------------------------------------------------------------
my $EPSILON = 1e-9;
sub approx_eq {
    my ($a, $b, $tol) = @_;
    $tol //= $EPSILON;
    return abs($a - $b) <= $tol;
}

# ---------------------------------------------------------------------------
# TWO_PI constant
# ---------------------------------------------------------------------------
ok(defined $CodingAdventures::Wave::TWO_PI, 'TWO_PI is defined');
ok(approx_eq($CodingAdventures::Wave::TWO_PI, 6.283185307179586, 1e-12),
   'TWO_PI ≈ 6.283185…');

# ===========================================================================
# sine_wave
# ===========================================================================

subtest 'sine_wave' => sub {
    # Correct number of samples
    my @s = sine_wave(1.0, 1.0, 0, 8, 8);
    is(scalar @s, 8, 'returns 8 samples');

    # Starts near zero (sin(0) = 0)
    my @s1 = sine_wave(440, 1.0, 0, 44100, 1);
    ok(approx_eq($s1[0], 0.0), 'sample[0] ≈ 0 for phase=0');

    # Quarter-period: sin(pi/2) = 1
    # f=1, sr=4 → sample index 1 is at t=0.25 → angle = pi/2
    my @sq = sine_wave(1.0, 1.0, 0, 4, 5);
    ok(approx_eq($sq[1], 1.0), 'sample[1] ≈ +1 (quarter period)');

    # Half-period: sin(pi) ≈ 0
    ok(approx_eq($sq[2], 0.0), 'sample[2] ≈ 0 (half period)');

    # Three-quarter period: sin(3*pi/2) = -1
    ok(approx_eq($sq[3], -1.0), 'sample[3] ≈ -1 (three-quarter period)');

    # Amplitude scaling
    my @sa = sine_wave(1.0, 3.5, 0, 4, 3);
    ok(approx_eq($sa[1], 3.5), 'sample[1] ≈ 3.5 with amplitude=3.5');

    # Phase offset: phase=pi/2 → first sample = amplitude (sin(pi/2) = 1)
    my $pi  = $CodingAdventures::Trig::PI;
    my @sp  = sine_wave(1.0, 1.0, $pi / 2, 4, 2);
    ok(approx_eq($sp[0], 1.0), 'phase pi/2 shifts sin to cos: first sample ≈ 1');

    # Pythagorean identity: sin² + cos² = amplitude²
    my $amp = 2.5;
    my @sv  = sine_wave(440, $amp, 0, 44100, 50);
    my @cv  = cosine_wave(440, $amp, 0, 44100, 50);
    for my $i ( 0 .. 49 ) {
        my $identity = $sv[$i]**2 + $cv[$i]**2;
        ok(approx_eq($identity, $amp**2, 1e-8),
           "Pythagorean identity at sample $i");
    }

    # Zero samples → empty list
    my @sz = sine_wave(440, 1.0, 0, 44100, 0);
    is(scalar @sz, 0, 'zero samples returns empty list');
};

# ===========================================================================
# cosine_wave
# ===========================================================================

subtest 'cosine_wave' => sub {
    my @c = cosine_wave(1.0, 1.0, 0, 8, 8);
    is(scalar @c, 8, 'returns 8 samples');

    # cos(0) = 1
    my $amp = 1.5;
    my @c1 = cosine_wave(440, $amp, 0, 44100, 1);
    ok(approx_eq($c1[0], $amp), 'sample[0] ≈ amplitude (cos(0)=1)');

    # Quarter period: cos(pi/2) = 0
    my @cq = cosine_wave(1.0, 1.0, 0, 4, 5);
    ok(approx_eq($cq[1], 0.0), 'sample[1] ≈ 0 (quarter period)');

    # Half period: cos(pi) = -1
    ok(approx_eq($cq[2], -1.0), 'sample[2] ≈ -1 (half period)');

    # Amplitude scaling
    my @ca = cosine_wave(440, 4.2, 0, 44100, 1);
    ok(approx_eq($ca[0], 4.2), 'sample[0] ≈ 4.2 with amplitude=4.2');
};

# ===========================================================================
# square_wave
# ===========================================================================

subtest 'square_wave' => sub {
    my @sq = square_wave(1.0, 1.0, 8, 8);
    is(scalar @sq, 8, 'returns 8 samples');

    # Every sample is exactly ±amplitude
    my $amp = 2.0;
    my @s   = square_wave(440, $amp, 44100, 100);
    for my $v (@s) {
        ok($v == $amp || $v == -$amp, "sample $v is ±$amp");
    }

    # First sample at t=0: sin(0) = 0 >= 0 → +amplitude
    my @s2 = square_wave(1.0, 1.0, 4, 4);
    is($s2[0], 1.0, 'first sample is +amplitude');

    # Half-period switch: f=1, sr=4
    # index 0,1 → positive; index 2,3 → negative
    is($s2[0],  1.0, 'sample[0] = +1');
    is($s2[1],  1.0, 'sample[1] = +1');
    is($s2[2], -1.0, 'sample[2] = -1');
    is($s2[3], -1.0, 'sample[3] = -1');
};

# ===========================================================================
# sawtooth_wave
# ===========================================================================

subtest 'sawtooth_wave' => sub {
    my @s = sawtooth_wave(1.0, 1.0, 8, 8);
    is(scalar @s, 8, 'returns 8 samples');

    # All samples within ±amplitude
    my $amp = 1.5;
    my @sa = sawtooth_wave(440, $amp, 44100, 100);
    for my $v (@sa) {
        ok($v >= -$amp - 1e-9 && $v <= $amp + 1e-9,
           "sawtooth sample $v in [-$amp, $amp]");
    }

    # At t=0: phase_frac = 0 - floor(0.5) = 0; y = 0
    my @s0 = sawtooth_wave(1.0, 1.0, 100, 1);
    ok(approx_eq($s0[0], 0.0), 'sawtooth at t=0 is 0');

    # Scales with amplitude
    my @s2 = sawtooth_wave(440, 3.0, 44100, 10);
    for my $v (@s2) {
        ok($v >= -3.0 - 1e-9 && $v <= 3.0 + 1e-9, "sawtooth amp=3 sample in range");
    }
};

# ===========================================================================
# triangle_wave
# ===========================================================================

subtest 'triangle_wave' => sub {
    my @t = triangle_wave(1.0, 1.0, 8, 8);
    is(scalar @t, 8, 'returns 8 samples');

    # All samples within ±amplitude
    my $amp = 2.0;
    my @ta = triangle_wave(440, $amp, 44100, 100);
    for my $v (@ta) {
        ok($v >= -$amp - 1e-9 && $v <= $amp + 1e-9,
           "triangle sample $v in [-$amp, $amp]");
    }

    # At t=0: phase_frac=0 for double-freq saw;
    # triangle = amp*(2*(2*|0|) - 1) = amp*(-1) = -amp
    my @t0 = triangle_wave(1.0, 1.0, 100, 1);
    ok(approx_eq($t0[0], -1.0), 'triangle at t=0 is -amplitude');

    # Symmetric: mean over integer number of periods should be near 0
    my @sym = triangle_wave(5, 1.0, 1000, 1000);  # 5 full periods in 1000 samples
    my $sum = 0;
    $sum += $_ for @sym;
    ok(abs($sum / 1000) < 0.01, 'triangle wave is symmetric (mean ≈ 0)');
};

# ===========================================================================
# dc_offset
# ===========================================================================

subtest 'dc_offset' => sub {
    my @d = dc_offset(5.0, 10);
    is(scalar @d, 10, 'returns 10 samples');
    is($_, 5.0, "sample = 5.0") for @d;

    # Zero value
    my @z = dc_offset(0, 5);
    is($_, 0, "zero sample = 0") for @z;

    # Negative
    my @n = dc_offset(-2.5, 3);
    is($_, -2.5, "negative sample = -2.5") for @n;

    # Zero samples
    my @e = dc_offset(1.0, 0);
    is(scalar @e, 0, 'zero samples returns empty list');
};

# ===========================================================================
# add_waves
# ===========================================================================

subtest 'add_waves' => sub {
    my @a = (1, 2, 3, 4);
    my @b = (5, 6, 7, 8);
    my @r = add_waves(\@a, \@b);
    is(\@r, [6, 8, 10, 12], 'add_waves element-wise sum');

    # Same length
    is(scalar add_waves([0.1, 0.2, 0.3], [0.4, 0.5, 0.6]), 3, 'result has same length');

    # Cancellation (negative + positive)
    my @c = add_waves([1.0, -1.0], [-1.0, 1.0]);
    ok(approx_eq($c[0], 0.0), 'cancellation: 1 + (-1) = 0');
    ok(approx_eq($c[1], 0.0), 'cancellation: (-1) + 1 = 0');

    # Destructive interference: sine + phase-shifted sine = 0
    my $pi  = $CodingAdventures::Trig::PI;
    my @s1  = sine_wave(440, 1.0, 0,   44100, 50);
    my @s2  = sine_wave(440, 1.0, $pi, 44100, 50);
    my @mix = add_waves(\@s1, \@s2);
    for my $i ( 0 .. 49 ) {
        ok(approx_eq($mix[$i], 0.0, 1e-8),
           "destructive interference at sample $i");
    }

    # Error on different lengths
    ok(dies { add_waves([1,2,3], [1,2]) }, 'dies on different lengths');
};

# ===========================================================================
# scale_wave
# ===========================================================================

subtest 'scale_wave' => sub {
    my @s = scale_wave([1, 2, 3, 4, 5], 2);
    is(\@s, [2, 4, 6, 8, 10], 'scale by 2');

    # Identity scaling
    my @orig = sine_wave(440, 1.0, 0, 44100, 10);
    my @sc1  = scale_wave(\@orig, 1);
    is(\@sc1, \@orig, 'scale by 1 is identity');

    # Scale by 0 → all zeros
    my @sc0 = scale_wave(\@orig, 0);
    is($_, 0, "scale by 0 → 0") for @sc0;

    # Inversion
    my @inv = scale_wave(\@orig, -1);
    for my $i ( 0 .. $#orig ) {
        ok(approx_eq($inv[$i], -$orig[$i], 1e-15),
           "scale by -1 inverts sample $i");
    }
};

# ===========================================================================
# mix_waves
# ===========================================================================

subtest 'mix_waves' => sub {
    # Single wave pass-through
    my @s = sine_wave(440, 1.0, 0, 44100, 10);
    my @m = mix_waves([\@s]);
    is(\@m, \@s, 'mixing one wave returns that wave');

    # Two identical waves → doubled
    my @m2 = mix_waves([\@s, \@s]);
    for my $i ( 0 .. $#s ) {
        ok(approx_eq($m2[$i], 2 * $s[$i], 1e-15),
           "mixing two identical waves doubles sample $i");
    }

    # Three waves
    my @a = (1, 2, 3);
    my @b = (4, 5, 6);
    my @c = (7, 8, 9);
    my @r = mix_waves([\@a, \@b, \@c]);
    is(\@r, [12, 15, 18], 'mix three waves');

    # DC bias via mix
    my @dc  = dc_offset(0.5, 10);
    my @mix = mix_waves([\@s, \@dc]);
    for my $i ( 0 .. 9 ) {
        ok(approx_eq($mix[$i], $s[$i] + 0.5, 1e-15),
           "mix with dc_offset shifts sample $i");
    }

    # Error: empty list
    ok(dies { mix_waves([]) }, 'dies on empty wave list');

    # Error: different lengths
    my @w1 = (1, 2, 3);
    my @w2 = (1, 2);
    ok(dies { mix_waves([\@w1, \@w2]) }, 'dies on different lengths');
};

done_testing;
