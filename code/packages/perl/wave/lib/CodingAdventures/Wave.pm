package CodingAdventures::Wave;

# ============================================================================
# CodingAdventures::Wave — Signal and waveform generation
# ============================================================================
#
# This module generates digital waveforms: arrays of floating-point samples
# that model continuous periodic signals. Waveforms are the building blocks of
# audio synthesis, digital signal processing, and communications engineering.
#
# ## What Is a Waveform?
#
# A "waveform" describes how a signal changes over time. In the digital world
# we represent it as a sequence of numbers ("samples"), each recording the
# signal's amplitude at a specific moment. The time between consecutive samples
# is 1/sample_rate seconds. For CD-quality audio that is 1/44100 ≈ 22.7 μs.
#
# ## The Five Classic Waveforms
#
#   Sine       — pure oscillation; no harmonics; the smoothest possible wave.
#   Cosine     — a sine wave advanced by 90°; starts at its peak.
#   Square     — alternates ±amplitude based on the sign of a sine.
#                Contains only odd harmonics: 1f, 3f, 5f, … (amplitude 1/n).
#   Sawtooth   — linear ramp with instantaneous reset.
#                Contains ALL harmonics: 1f, 2f, 3f, … (amplitude 1/n).
#   Triangle   — linear rise and fall derived from sawtooth via |x|.
#                Odd harmonics only: 1f, 3f, 5f, … (amplitude 1/n²).
#
# ## Relationship to the Trig Package
#
# This module depends on CodingAdventures::Trig for sin_approx and cos_approx,
# which are computed from first principles via Maclaurin series.  This keeps
# the "from scratch" educational philosophy of the coding-adventures stack.
#
# ## Usage
#
#   use CodingAdventures::Wave qw(
#       sine_wave cosine_wave square_wave sawtooth_wave triangle_wave
#       dc_offset add_waves scale_wave mix_waves
#   );
#
#   # 440 Hz concert-A sine wave, 1 second at CD quality
#   my @samples = sine_wave(440, 1.0, 0, 44100, 44100);
#
#   # Mix two harmonics
#   my @fund = sine_wave(440,  0.8, 0, 44100, 44100);
#   my @harm = sine_wave(880,  0.2, 0, 44100, 44100);
#   my @mix  = add_waves(\@fund, \@harm);
#
# ============================================================================

use strict;
use warnings;
use Carp qw(croak);
use POSIX qw(floor);
use CodingAdventures::Trig qw(sin_approx cos_approx);

our $VERSION = '0.01';

use Exporter 'import';
our @EXPORT_OK = qw(
    sine_wave  cosine_wave  square_wave  sawtooth_wave  triangle_wave
    dc_offset
    add_waves  scale_wave   mix_waves
);

# ============================================================================
# Constants
# ============================================================================

# TWO_PI — the angular period of one complete oscillation cycle.
#
# All periodic waveforms repeat every 2*pi radians. We re-export this constant
# from the Trig module so callers don't need to import it themselves.
our $TWO_PI = $CodingAdventures::Trig::TWO_PI;

# ============================================================================
# sine_wave — Pure sinusoidal oscillation
# ============================================================================
#
# THE SINE WAVE FORMULA
#
# For sample index i (0-based), the time in seconds is:
#
#     t = i / sample_rate
#
# The amplitude at that time is:
#
#     y(t) = amplitude * sin(2*pi * frequency * t + phase)
#
# Breaking this down:
#   - frequency * t  counts how many cycles have elapsed by time t.
#   - 2*pi converts cycles to radians (one full cycle = 2*pi radians).
#   - phase shifts the wave left/right in time (measured in radians).
#   - amplitude scales the peak value.
#
# EXAMPLE: 1 Hz at 8 samples/sec (8 samples = one full cycle)
#
#   i=0: t=0.000  sin(0.000) =  0.000
#   i=1: t=0.125  sin(0.785) = +0.707
#   i=2: t=0.250  sin(1.571) = +1.000   (peak)
#   i=3: t=0.375  sin(2.356) = +0.707
#   i=4: t=0.500  sin(3.142) =  0.000
#   i=5: t=0.625  sin(3.927) = -0.707
#   i=6: t=0.750  sin(4.712) = -1.000   (trough)
#   i=7: t=0.875  sin(5.497) = -0.707
#
# @param $frequency    Cycles per second (Hz).
# @param $amplitude    Peak value; wave ranges from -amplitude to +amplitude.
# @param $phase        Phase offset in radians (default 0.0).
# @param $sample_rate  Samples per second (e.g., 44100).
# @param $num_samples  Number of samples to generate.
# @return              List of num_samples floating-point values.

sub sine_wave {
    my ($frequency, $amplitude, $phase, $sample_rate, $num_samples) = @_;
    my @samples;
    for my $i ( 0 .. $num_samples - 1 ) {
        my $t     = $i / $sample_rate;
        my $angle = $TWO_PI * $frequency * $t + $phase;
        push @samples, $amplitude * sin_approx($angle);
    }
    return @samples;
}

# ============================================================================
# cosine_wave — Cosine variant (sine shifted 90°)
# ============================================================================
#
# Cosine is a sine wave with a 90-degree (pi/2 radian) phase advance:
#
#     cos(x) = sin(x + pi/2)
#
# A cosine wave starts at its peak value (+amplitude) at t=0, rather than
# starting at zero like a sine. This is useful when you need a wave that
# begins at its maximum.
#
# FORMULA: y(t) = amplitude * cos(2*pi * frequency * t + phase)
#
# @param $frequency    Frequency in Hz.
# @param $amplitude    Peak value.
# @param $phase        Phase offset in radians.
# @param $sample_rate  Samples per second.
# @param $num_samples  Number of samples.
# @return              List of floats.

sub cosine_wave {
    my ($frequency, $amplitude, $phase, $sample_rate, $num_samples) = @_;
    my @samples;
    for my $i ( 0 .. $num_samples - 1 ) {
        my $t     = $i / $sample_rate;
        my $angle = $TWO_PI * $frequency * $t + $phase;
        push @samples, $amplitude * cos_approx($angle);
    }
    return @samples;
}

# ============================================================================
# square_wave — Hard-clipped binary oscillation
# ============================================================================
#
# A square wave alternates instantly between +amplitude and -amplitude, with
# equal time at each level (50% duty cycle). The transition point is determined
# by the zero-crossings of a sine wave at the same frequency:
#
#     y(t) = +amplitude   if sin(2*pi * frequency * t) >= 0
#     y(t) = -amplitude   if sin(2*pi * frequency * t) <  0
#
# HARMONIC CONTENT
#
# A square wave contains only ODD harmonics (1f, 3f, 5f, …) with amplitudes
# decreasing as 1/n:
#
#     square(t) = (4/π) * [sin(f*t)/1 + sin(3f*t)/3 + sin(5f*t)/5 + ...]
#
# This is why square waves sound "buzzy" — they have far more overtones than
# a pure sine. In audio synthesizers, the square is the classic "pulse wave".
#
# @param $frequency    Frequency in Hz.
# @param $amplitude    Peak value (each sample is exactly ±amplitude).
# @param $sample_rate  Samples per second.
# @param $num_samples  Number of samples.
# @return              List of floats (each exactly +amplitude or -amplitude).

sub square_wave {
    my ($frequency, $amplitude, $sample_rate, $num_samples) = @_;
    my @samples;
    for my $i ( 0 .. $num_samples - 1 ) {
        my $t     = $i / $sample_rate;
        my $angle = $TWO_PI * $frequency * $t;
        my $s     = sin_approx($angle);
        push @samples, ($s >= 0) ? $amplitude : -$amplitude;
    }
    return @samples;
}

# ============================================================================
# sawtooth_wave — Linearly rising ramp with instantaneous reset
# ============================================================================
#
# A sawtooth wave rises linearly from -amplitude to +amplitude over one period,
# then resets instantaneously. The shape resembles a saw blade.
#
# FORMULA
#
#     phase_frac = t * frequency - floor(t * frequency + 0.5)
#     y(t)       = 2 * amplitude * phase_frac
#
# The floor(x + 0.5) operation centers the ramp around zero:
#   - Without it, each period starts at 0 and rises to +2*amplitude.
#   - With it, phase_frac ∈ (-0.5, +0.5], so y ∈ (-amplitude, +amplitude].
#
# HARMONIC CONTENT
#
# Unlike square and triangle waves, a sawtooth contains ALL harmonics (1f, 2f,
# 3f, …) with amplitudes 1/n. This makes it the "brightest" basic waveform.
# In audio synthesis, it forms the basis for brass and string sounds.
#
# @param $frequency    Frequency in Hz.
# @param $amplitude    Peak value.
# @param $sample_rate  Samples per second.
# @param $num_samples  Number of samples.
# @return              List of floats.

sub sawtooth_wave {
    my ($frequency, $amplitude, $sample_rate, $num_samples) = @_;
    my @samples;
    for my $i ( 0 .. $num_samples - 1 ) {
        my $t          = $i / $sample_rate;
        my $phase_frac = $t * $frequency - floor($t * $frequency + 0.5);
        push @samples, 2.0 * $amplitude * $phase_frac;
    }
    return @samples;
}

# ============================================================================
# triangle_wave — Linear rise and fall
# ============================================================================
#
# A triangle wave rises linearly from -amplitude to +amplitude over the first
# half-cycle, then falls back to -amplitude over the second half. Unlike a
# sawtooth, it has no discontinuities.
#
# DERIVATION FROM SAWTOOTH
#
# A triangle wave can be derived from a sawtooth at double the frequency:
#
#   1. Compute sawtooth at 2*frequency: phase_frac ∈ (-0.5, 0.5].
#   2. Take the absolute value: |phase_frac| ∈ [0, 0.5].
#   3. Rescale: 2*|phase_frac| ∈ [0, 1].
#   4. Shift: 2*|phase_frac| - 0.5 ∈ [-0.5, 0.5].
#   5. Scale by 2*amplitude: result ∈ [-amplitude, +amplitude].
#
#     triangle(t) = amplitude * (2*(2*|phase_frac|) - 1)
#
# HARMONIC CONTENT
#
# Triangle waves, like square waves, contain only ODD harmonics. However, the
# amplitudes fall off as 1/n² (much faster than 1/n for square waves). This is
# why triangles sound mellow/hollow rather than buzzy.
#
# @param $frequency    Frequency in Hz.
# @param $amplitude    Peak value.
# @param $sample_rate  Samples per second.
# @param $num_samples  Number of samples.
# @return              List of floats.

sub triangle_wave {
    my ($frequency, $amplitude, $sample_rate, $num_samples) = @_;
    my @samples;
    for my $i ( 0 .. $num_samples - 1 ) {
        my $t          = $i / $sample_rate;
        my $double_f   = 2.0 * $frequency;
        my $phase_frac = $t * $double_f - floor($t * $double_f + 0.5);
        push @samples, $amplitude * (2.0 * (2.0 * abs($phase_frac)) - 1.0);
    }
    return @samples;
}

# ============================================================================
# dc_offset — Constant-value "flat" signal
# ============================================================================
#
# A DC offset (Direct Current offset, also called DC bias) is a constant
# component added to a signal. In electronics, DC means a non-oscillating
# constant voltage or current. In digital signal processing, a DC offset
# represents the zero-frequency component of a signal.
#
# Common uses:
#   - Bias a signal to a non-zero baseline before further processing.
#   - Represent the mean value of a zero-mean signal.
#   - Act as one input to a mixer (to shift the combined waveform).
#
# @param $value        The constant value for every sample.
# @param $num_samples  Number of samples to generate.
# @return              List of $num_samples identical values.

sub dc_offset {
    my ($value, $num_samples) = @_;
    return ($value) x $num_samples;
}

# ============================================================================
# add_waves — Element-wise sum of two waves
# ============================================================================
#
# SUPERPOSITION PRINCIPLE
#
# When two waves occupy the same medium simultaneously, their amplitudes add at
# every point. This is the "principle of superposition" — one of the most
# fundamental concepts in wave physics.
#
#     y_combined(t) = y1(t) + y2(t)
#
# In discrete form: result[$i] = wave1[$i] + wave2[$i].
#
# DESTRUCTIVE INTERFERENCE
#
# If two identical waves are exactly 180° out of phase, they cancel:
#
#     sin(x) + sin(x + pi) = sin(x) - sin(x) = 0
#
# This principle is used in noise-canceling headphones.
#
# ERROR CONDITION
#
# Adding waves of different lengths doesn't make physical sense, and silent
# truncation would cause confusing bugs. We croak with a clear message.
#
# @param $wave1_ref  Array reference to the first wave.
# @param $wave2_ref  Array reference to the second wave (must be same length).
# @return            List where result[$i] = wave1[$i] + wave2[$i].
# @dies              If the waves have different lengths.

sub add_waves {
    my ($wave1_ref, $wave2_ref) = @_;
    croak sprintf(
        "CodingAdventures::Wave::add_waves: waves must be the same length (%d vs %d)",
        scalar @$wave1_ref, scalar @$wave2_ref
    ) unless @$wave1_ref == @$wave2_ref;

    my @result;
    for my $i ( 0 .. $#$wave1_ref ) {
        push @result, $wave1_ref->[$i] + $wave2_ref->[$i];
    }
    return @result;
}

# ============================================================================
# scale_wave — Multiply every sample by a scalar
# ============================================================================
#
# Scaling multiplies each sample by a constant factor. This is used to:
#   - Adjust volume/amplitude (factor < 1 to quiet, > 1 to amplify).
#   - Invert phase (factor = -1 flips the waveform upside down).
#   - Normalize a wave to fit within a specific range.
#
# FORMULA: result[$i] = wave[$i] * $scalar
#
# @param $wave_ref  Array reference to the wave.
# @param $scalar    Multiplicative factor (any real number).
# @return           List where result[$i] = wave[$i] * $scalar.

sub scale_wave {
    my ($wave_ref, $scalar) = @_;
    return map { $_ * $scalar } @$wave_ref;
}

# ============================================================================
# mix_waves — Element-wise sum of multiple waves
# ============================================================================
#
# mix_waves extends add_waves to handle an arbitrary number of input waves.
# This is convenient for audio applications where many tracks are combined
# (a 4-track recording, an orchestra, etc.).
#
# Internally, mix_waves calls add_waves iteratively, accumulating a running
# sum. All waves in the input list must have the same length.
#
# EXAMPLE: mixing a fundamental with its harmonics
#
#   my @rich = mix_waves([
#       [ sine_wave(440,  1.00, 0, 44100, 44100) ],   # fundamental
#       [ sine_wave(880,  0.50, 0, 44100, 44100) ],   # 2nd harmonic
#       [ sine_wave(1320, 0.25, 0, 44100, 44100) ],   # 3rd harmonic
#   ]);
#
# @param $waves_ref  Array reference of array references (the list of waves).
# @return            List where result[$i] = sum of all waves[$j][$i].
# @dies              If $waves_ref is empty or waves have different lengths.

sub mix_waves {
    my ($waves_ref) = @_;
    croak "CodingAdventures::Wave::mix_waves: wave list must not be empty"
        unless @$waves_ref;

    my @result = @{ $waves_ref->[0] };
    for my $j ( 1 .. $#$waves_ref ) {
        @result = add_waves(\@result, $waves_ref->[$j]);
    }
    return @result;
}

1;

__END__

=head1 NAME

CodingAdventures::Wave - Signal and waveform generation (sine, square, sawtooth, triangle, cosine)

=head1 SYNOPSIS

    use CodingAdventures::Wave qw(
        sine_wave cosine_wave square_wave sawtooth_wave triangle_wave
        dc_offset add_waves scale_wave mix_waves
    );

    # 1-second 440 Hz sine wave at CD quality
    my @samples = sine_wave(440, 1.0, 0, 44100, 44100);

    # Mix fundamental + second harmonic
    my @mix = add_waves(
        [ sine_wave(440, 0.8, 0, 44100, 44100) ],
        [ sine_wave(880, 0.2, 0, 44100, 44100) ],
    );

=head1 DESCRIPTION

Pure-Perl waveform generation using CodingAdventures::Trig for trigonometric
operations. All waveforms are returned as flat lists of floating-point samples.

=head1 FUNCTIONS

=over 4

=item B<sine_wave($freq, $amp, $phase, $sr, $n)>

Generates C<$n> samples of C<$amp * sin(2*pi*$freq*t + $phase)>.

=item B<cosine_wave($freq, $amp, $phase, $sr, $n)>

Generates C<$n> samples of C<$amp * cos(2*pi*$freq*t + $phase)>.

=item B<square_wave($freq, $amp, $sr, $n)>

Generates C<$n> samples alternating between C<±$amp> based on sign of sine.

=item B<sawtooth_wave($freq, $amp, $sr, $n)>

Generates C<$n> samples of a sawtooth wave using
C<2*$amp*(t*f - floor(t*f + 0.5))>.

=item B<triangle_wave($freq, $amp, $sr, $n)>

Generates C<$n> samples of a triangle wave derived from a double-frequency
sawtooth via absolute value.

=item B<dc_offset($value, $n)>

Returns a list of C<$n> values all equal to C<$value>.

=item B<add_waves(\@wave1, \@wave2)>

Returns a list that is the element-wise sum of the two waves. Dies if lengths differ.

=item B<scale_wave(\@wave, $scalar)>

Returns a list where each element is multiplied by C<$scalar>.

=item B<mix_waves([\@w1, \@w2, ...])>

Returns the element-wise sum of all waves. All must be the same length.

=back

=head1 CONSTANTS

=over 4

=item C<$CodingAdventures::Wave::TWO_PI>

6.283185307179586

=back

=head1 VERSION

0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
