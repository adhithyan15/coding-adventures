package CodingAdventures::TestBootstrap::Limits;

# Bounds that keep every loop and allocation finite (CWE-400).  A caller or a
# fixture can ask for more only by being refused.  (VER05p section 3, VER05h.)

use strict;
use warnings;

use CodingAdventures::TestBootstrap::Int ();

use constant {
    MAX_CASES           => 100_000,
    MAX_SHRINK_STEPS    => 10_000,
    MAX_INPUT_BYTES     => 1 << 20,
    MAX_LIST_LEN        => 100_000,
    MAX_GEN_COUNT       => 1_000_000,    # most values any corpus entry point generates
    ATTEMPT_FACTOR      => 16,           # attempt budget = FACTOR * max_steps + SLACK
    ATTEMPT_SLACK       => 64,
    MAX_GEN_BYTES_TOTAL => 1 << 20,      # count x width bound for gen_bytes / gen_bounded
    MAX_FREQUENCY_TOTAL => 1 << 32,
    MAX_SHRINK_WORK     => 64 * 1024 * 1024,
    MAX_RETRIES         => 10_000,
    DEFAULT_MAX_RETRIES => 100,
    MAX_TUPLE_ARITY     => 64,
    MAX_SPAN_BITS       => 16_384,       # k-word rejection sampling is linear in it
    MAX_INT_DIGITS      => 1000,         # most digits of a fixture integer
};

# A rejected attempt copies its candidate but is not an accepted step, so the
# step cap alone does not bound the work; this is the attempt cap of one shrink.
sub attempt_budget { return ATTEMPT_FACTOR * $_[0] + ATTEMPT_SLACK }

# `$value` must be a native Perl integer inside [$lo, $hi] (all native).  Floats,
# strings and bigs are refused with a message naming `$name`.
sub check_range {
    my ( $value, $lo, $hi, $name ) = @_;
    my $v = CodingAdventures::TestBootstrap::Int::check( $value, $name );
    if ( CodingAdventures::TestBootstrap::Int::is_big($v) || $v < $lo || $v > $hi ) {
        die "$name outside [$lo, $hi]\n";
    }
    return $v;
}

sub check_budget { return check_range( $_[0], 0, MAX_SHRINK_STEPS, 'max_steps' ) }
sub check_count  { return check_range( $_[0], 0, MAX_GEN_COUNT,    'count' ) }

1;
