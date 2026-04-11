package CodingAdventures::WasmRuntime::WasiClock;

# ============================================================================
# WasiClock — duck-typed clock interface for WASI Tier 3
# ============================================================================
#
# WASI exposes two "clock domains" to WebAssembly programs:
#
#   Realtime  (id=0): Wall-clock time since the Unix epoch (1970-01-01 00:00
#             UTC), measured in nanoseconds. Also called "civil time" or
#             "calendar time". It can jump backwards if the system clock is
#             adjusted (NTP, manual set, leap seconds).
#
#   Monotonic (id=1): A counter that only ever moves forward, making it safe
#             for measuring elapsed time. It does NOT correspond to any
#             calendar date.
#
#   Process (id=2) and Thread (id=3) clocks: Per-process CPU-time clocks.
#             We map these to realtime for simplicity.
#
# ## Duck-typing / dependency injection
#
# Rather than hard-coding `Time::HiRes` calls inside the WASI host functions,
# we accept a $clock object at construction time. This means:
#
#   * Tests can inject FakeClock that returns deterministic values.
#   * Future code can inject a custom PRNG-driven clock for reproducible
#     simulation.
#   * The default (SystemClock) uses the real OS clock.
#
# The interface (duck-typed contract — no formal Perl mechanism enforces this):
#
#   $clock->realtime_ns()       → integer, nanoseconds since Unix epoch
#   $clock->monotonic_ns()      → integer, nanoseconds (monotonically increasing)
#   $clock->resolution_ns($id)  → integer, nanoseconds (clock resolution)
#
# ============================================================================

# (This is just the interface documentation package — SystemClock implements it.)

1;

package CodingAdventures::WasmRuntime::SystemClock;

# ============================================================================
# SystemClock — real OS clock, default implementation of WasiClock
# ============================================================================
#
# Uses Time::HiRes for sub-second precision. Time::HiRes is a core Perl module
# (included in perl 5.8+), so no CPAN install is required.
#
# ## Why multiply by 1_000_000_000?
#
# Time::HiRes::time() returns a floating-point Unix timestamp in seconds,
# e.g., 1700000000.123456789. WASI expects nanoseconds, so we multiply by
# 10^9 and take the integer part:
#
#   1700000000.123456789 * 1e9  ≈  1_700_000_000_123_456_789  ns
#
# ## CLOCK_MONOTONIC availability
#
# POSIX CLOCK_MONOTONIC is available on Linux, macOS, and other POSIX systems,
# but not on older Windows Perls. We use eval {} to catch the failure gracefully
# and fall back to realtime (which is "good enough" for most use cases).

use strict;
use warnings;

# Import Time::HiRes functions. 'time' is the high-precision version of
# CORE::time(). 'clock_gettime' allows selecting a specific POSIX clock.
use Time::HiRes qw(time clock_gettime CLOCK_MONOTONIC);

sub new { bless {}, shift }

# realtime_ns() — return the number of nanoseconds since the Unix epoch.
#
# Example:
#   If Time::HiRes::time() returns 1700000000.5,
#   then realtime_ns() returns 1_700_000_000_500_000_000.
sub realtime_ns {
    my $t = Time::HiRes::time();
    return int($t * 1_000_000_000);
}

# monotonic_ns() — return a monotonically increasing nanosecond counter.
#
# On POSIX systems, CLOCK_MONOTONIC is guaranteed to never go backwards,
# making it ideal for measuring elapsed time between two events. Falls back
# to realtime if the OS does not support CLOCK_MONOTONIC.
sub monotonic_ns {
    my $ns = eval {
        my $t = Time::HiRes::clock_gettime(Time::HiRes::CLOCK_MONOTONIC());
        int($t * 1_000_000_000);
    };
    if ($@) {
        # Fallback: realtime is not strictly monotonic, but close enough for
        # platforms that lack CLOCK_MONOTONIC (e.g., some Windows builds).
        return int(Time::HiRes::time() * 1_000_000_000);
    }
    return $ns;
}

# resolution_ns($id) — the resolution of the given clock.
#
# We report 1 millisecond (1_000_000 nanoseconds) as a conservative estimate.
# Many modern systems have nanosecond-resolution clocks, but 1ms is a safe
# lower bound that avoids over-promising.
#
# The $id parameter is accepted but ignored (same resolution for all clocks).
sub resolution_ns { return 1_000_000 }

1;

package CodingAdventures::WasmRuntime::WasiRandom;

# ============================================================================
# WasiRandom — duck-typed random interface for WASI Tier 3
# ============================================================================
#
# WASI exposes wasi_snapshot_preview1::random_get, which fills a caller-
# specified buffer with cryptographically secure random bytes.
#
# ## Why inject the random source?
#
# Like the clock, we inject the random source so tests can control exactly
# what bytes are produced (FakeRandom), and so future code can swap in a
# deterministic PRNG for reproducible fuzzing or simulation.
#
# ## Interface (duck-typed contract):
#
#   $random->fill_bytes($n) → arrayref of $n integers, each in [0, 255]
#
# ============================================================================

1;

package CodingAdventures::WasmRuntime::SystemRandom;

# ============================================================================
# SystemRandom — reads from /dev/urandom (or rand() fallback)
# ============================================================================
#
# /dev/urandom is the standard POSIX source of cryptographically secure random
# bytes. It never blocks (unlike /dev/random on some Linux kernels), making it
# appropriate for general use.
#
# On Windows (where /dev/urandom doesn't exist), we fall back to Perl's built-in
# rand(), which is NOT cryptographically secure but allows the code to run.
#
# ## Binary read mode
#
# We open the file with '<:raw' to disable encoding transformations. Without
# :raw, Perl may interpret certain byte values as multi-byte UTF-8 sequences
# or line endings, corrupting the random data.
#
# ## unpack('C*', $buf)
#
# 'C' is the pack/unpack template for an unsigned char (8-bit value, 0–255).
# '*' means "repeat for all bytes in the string". So unpack('C*', $buf) turns
# a binary string of N bytes into a list of N integers in [0, 255].

use strict;
use warnings;

sub new { bless {}, shift }

# fill_bytes($n) — return an arrayref of $n random bytes (integers 0–255).
sub fill_bytes {
    my ($self, $n) = @_;
    my @bytes;

    if (open my $fh, '<:raw', '/dev/urandom') {
        my $buf = '';
        read($fh, $buf, $n);
        close $fh;
        @bytes = unpack('C*', $buf);
    } else {
        # Fallback for environments without /dev/urandom (e.g., some Windows).
        # Note: rand() is NOT cryptographically secure.
        @bytes = map { int(rand(256)) } 1 .. $n;
    }

    return \@bytes;
}

1;

__END__

=head1 NAME

CodingAdventures::WasmRuntime::WasiClockRandom - Clock and random providers for WASI Tier 3

=head1 SYNOPSIS

    use CodingAdventures::WasmRuntime::WasiClockRandom;

    # Use system defaults
    my $clock  = CodingAdventures::WasmRuntime::SystemClock->new();
    my $random = CodingAdventures::WasmRuntime::SystemRandom->new();

    # Or inject fakes for testing
    my $clock  = FakeClock->new();
    my $random = FakeRandom->new();

=head1 DESCRIPTION

Provides pluggable clock and random-byte implementations for the WASI Tier 3
host functions (clock_time_get, clock_res_get, random_get).

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
