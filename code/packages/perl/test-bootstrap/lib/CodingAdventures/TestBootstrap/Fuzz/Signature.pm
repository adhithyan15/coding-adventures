package CodingAdventures::TestBootstrap::Fuzz::Signature;

# Crash signatures, frame normalisation and the one public failure line (VER06 sections
# 5.2 and 6, VER06q sections 5.6 and 7).

use strict;
use warnings;

use CodingAdventures::TestBootstrap::Int              ();
use CodingAdventures::TestBootstrap::Canon            ();
use CodingAdventures::TestBootstrap::Prng             ();
use CodingAdventures::TestBootstrap::Fuzz::Limits     ();
use CodingAdventures::TestBootstrap::Fuzz::FailureIds ();

my $int_check    = \&CodingAdventures::TestBootstrap::Int::check;
my $int_is_big   = \&CodingAdventures::TestBootstrap::Int::is_big;
my $to_bytes     = \&CodingAdventures::TestBootstrap::Canon::to_bytes;
my $fnv1a64_word = \&CodingAdventures::TestBootstrap::Prng::fnv1a64_word;
my $is_failure_id = \&CodingAdventures::TestBootstrap::Fuzz::FailureIds::is_fuzz_failure_id;
my $MAX_TARGET_LEN = CodingAdventures::TestBootstrap::Fuzz::Limits::MAX_TARGET_LEN();
my $MAX_KIND_LEN   = CodingAdventures::TestBootstrap::Fuzz::Limits::MAX_KIND_LEN();
my $MAX_BASENAME   = CodingAdventures::TestBootstrap::Fuzz::Limits::MAX_BASENAME();
my $MAX_FRAMES     = CodingAdventures::TestBootstrap::Fuzz::Limits::MAX_FRAMES();
my $CRASH   = CodingAdventures::TestBootstrap::Fuzz::FailureIds::CRASH();
my $TIMEOUT = CodingAdventures::TestBootstrap::Fuzz::FailureIds::TIMEOUT();
my $OOM     = CodingAdventures::TestBootstrap::Fuzz::FailureIds::OOM();

# The low 32 bits of fnv1a64(kind ":" frames joined by ":"), 8 lowercase hex.
#
# Frames are NOT escaped (VER06 section 9.4 item 6): one frame "x:y" and two frames "x", "y"
# hash alike.  This lane never produces a frame with a colon (`normalise_frame`), so the
# ambiguity cannot arise from Perl targets.
sub crash_signature {
    my ( $kind, $frames ) = @_;
    die "kind must be a string\n" if !defined $kind || ref $kind;
    die "frames must be a list of strings\n" if ref $frames ne 'ARRAY';
    for my $f (@$frames) { die "frames must be a list of strings\n" if !defined $f || ref $f }
    return sprintf( '%08x', $fnv1a64_word->( $kind . ':' . join( ':', @$frames ) ) & 0xFFFFFFFF );
}

sub duplicate_crash {
    my ( $ka, $fa, $kb, $fb ) = @_;
    return crash_signature( $ka, $fa ) eq crash_signature( $kb, $fb ) ? 1 : 0;
}

# The bytes of `$text` reduced to the grammar [A-Za-z0-9._/-]: truncated to `limit` BYTES
# first (so a cut in the middle of a multi-byte character leaves one `_`), then every other
# BYTE (a control character, a newline, one byte of a multi-byte character) becomes `_`.
sub token {
    my ( $text, $limit ) = @_;
    my $b = substr( $to_bytes->($text), 0, $limit );
    $b =~ s/[^A-Za-z0-9._\/-]/_/g;
    return $b;
}

# The only public failure line: `not ok N - target [failure-id:hex8]`.  It carries no input
# bytes, seed, message or stack trace (VER00 7.3).
sub format_tap_failure {
    my ( $n, $target, $failure_id, $sig ) = @_;
    $n = $int_check->( $n, 'test_num' );
    die "test_num must be in 1..2147483647\n" if $int_is_big->($n) || $n < 1 || $n > 2147483647;
    die "target must be a string\n" if !defined $target || ref $target;
    die "failure id is not one of the registered fuzz failure ids\n" if !$is_failure_id->($failure_id);
    die "sig hash must be exactly 8 lowercase hex digits\n" if !defined $sig || ref $sig || $sig !~ /\A[0-9a-f]{8}\z/;
    return sprintf( 'not ok %d - %s [%s:%s]', $n, token( $target, $MAX_TARGET_LEN ), $failure_id, $sig );
}

# A report as public, opaque failure lines (none for a clean one): one per crash signature,
# then one for timeouts, one for OOMs and one for leaked executions, whose hash is the
# signature of the bare kind (they have no stack to hash).
sub tap_failures {
    my ( $report, $target, $first_test ) = @_;
    $first_test = 1 if !defined $first_test;
    my @lines;
    my $i = 0;
    for my $rec ( @{ $report->{crashes} } ) {
        push @lines, format_tap_failure( $first_test + $i++, $target, $CRASH, $rec->{signature} );
    }
    for my $e ( [ $report->{timeouts}, $TIMEOUT, 'timeout' ], [ $report->{ooms}, $OOM, 'oom' ], [ $report->{leaked} || 0, $TIMEOUT, 'executions-leaked' ] ) {
        push @lines, format_tap_failure( $first_test + @lines, $target, $e->[1], crash_signature( $e->[2], [] ) ) if $e->[0] > 0;
    }
    return \@lines;
}

# A crash kind from a name: anything outside [A-Za-z0-9._<>$/-] becomes `_`, at most 64
# characters.  (The engine's own kinds are fixed words or class names; this is for callers
# that build a kind from other text.)  Only the first 64 bytes are ever examined.
sub normalise_kind {
    my ($name) = @_;
    return '<anonymous>' if !defined $name || ref $name;
    my $k = substr( $to_bytes->($name), 0, $MAX_KIND_LEN );
    $k =~ s/[^A-Za-z0-9._<>\$\/-]/_/g;
    return $k;
}

# `basename#line`, every character outside [A-Za-z0-9._<>$/#-] made `_`, so no frame
# contains `:`.  `(eval N)` is normalised to `eval` (the number changes from run to run).
sub normalise_frame {
    my ( $path, $line ) = @_;
    $path = '' if !defined $path;
    $line = 0  if !defined $line;
    my $p = $to_bytes->($path);
    $p = 'eval' if $p =~ /\A\(eval [0-9]+\)\z/;
    my $base = $p =~ m{([^/\\]*)\z} ? $1 : $p;
    $base = substr( $base, 0, $MAX_BASENAME );
    my $f = $base . '#' . ( $line =~ /\A[0-9]+\z/ ? $line : 0 );
    $f =~ s/[^A-Za-z0-9._<>\$\/#-]/_/g;
    return $f;
}

# Drops consecutive duplicates (a recursive crash hashes alike at any depth) and keeps the
# innermost MAX_FRAMES.
sub collapse {
    my ($frames) = @_;
    my @out;
    for my $f (@$frames) {
        if ( !@out || $out[-1] ne $f ) {
            push @out, $f;
            last if @out >= $MAX_FRAMES;
        }
    }
    return \@out;
}

1;
