package CodingAdventures::TestBootstrap::Fuzz::Report;

# What a campaign found (VER06 section 5, VER06q section 6).
#
# A report and its crash records are blessed hashes with a `""` overload that prints counts
# and signatures only.  `seed`, `corpus` and a crash's `minimal_input` are PRIVATE (VER06
# section 6.3): they live in inside-out tables behind accessors, are never in a printed
# form, and public output goes through `Signature::format_tap_failure` only.

use strict;
use warnings;

use Scalar::Util ();

my $refaddr = \&Scalar::Util::refaddr;

use constant {
    TIER_REPORTED  => 'caller-reported',
    TIER_BLACK_BOX => 'black-box-uninstrumented',
};

my %PRIVATE;    # refaddr => { seed, corpus } of a report, { minimal_input } of a record

package CodingAdventures::TestBootstrap::Fuzz::Report::CrashRecord;

use overload '""' => sub {
    my ($r) = @_;
    return "#<Fuzz.CrashRecord $r->{signature} $r->{kind} frames=" . scalar( @{ $r->{frames} } ) . " hits=$r->{hits}>";
  },
  fallback => 1;

sub DESTROY { delete $PRIVATE{ $refaddr->( $_[0] ) } if ref $_[0] }

# The shortest input known to reproduce the signature (private: callers must not log it).
sub minimal_input { my $p = $PRIVATE{ $refaddr->( $_[0] ) }; return $p ? $p->{minimal_input} : undef }

package CodingAdventures::TestBootstrap::Fuzz::Report;

# One deduplicated crash.
sub crash_record {
    my ($f) = @_;
    my $r = bless {
        signature     => $f->{signature},
        kind          => $f->{kind},
        frames        => $f->{frames},
        hits          => $f->{hits},
        minimized     => $f->{minimized},
        input_dropped => $f->{input_dropped},
    }, 'CodingAdventures::TestBootstrap::Fuzz::Report::CrashRecord';
    $PRIVATE{ $refaddr->($r) } = { minimal_input => $f->{minimal_input} };
    return $r;
}

use overload '""' => sub {
    my ($r) = @_;
    return "#<Fuzz.Report iterations=$r->{iterations} seeds=$r->{admitted_seeds} edges=$r->{total_edges} crashes="
      . unique_crashes($r) . " timeouts=$r->{timeouts} ooms=$r->{ooms} leaked=$r->{leaked} tier=$r->{coverage_tier}>";
  },
  fallback => 1;

sub DESTROY { delete $PRIVATE{ $refaddr->( $_[0] ) } if ref $_[0] }

# Every execution lands in exactly one bucket: `iterations` counts MUTATION executions;
# crashes/timeouts/ooms count triage and mutation executions; anything that passed is in the
# corpus or was redundant.
sub new {
    my ($f) = @_;
    my $r = bless {
        iterations         => $f->{iterations},
        admitted_seeds     => $f->{admitted_seeds},
        total_edges        => $f->{total_edges},
        crashes            => $f->{crashes},
        timeouts           => $f->{timeouts},
        ooms               => $f->{ooms},
        crashes_dropped    => $f->{crashes_dropped},
        admissions_dropped => $f->{admissions_dropped},
        leaked             => $f->{leaked} || 0,
        aborted            => $f->{aborted} ? 1 : 0,
        coverage_tier      => $f->{coverage_tier},
    }, __PACKAGE__;
    $PRIVATE{ $refaddr->($r) } = { seed => $f->{seed}, corpus => $f->{corpus} };
    return $r;
}

sub seed   { my $p = $PRIVATE{ $refaddr->( $_[0] ) }; return $p ? $p->{seed} : undef }
sub corpus { my $p = $PRIVATE{ $refaddr->( $_[0] ) }; return $p ? $p->{corpus} : undef }

# Can this be read as "the campaign found nothing"?  Only if at least one mutation iteration
# ran and nothing crashed, timed out, ran out of memory, leaked an execution or was dropped
# (a crash record, or a corpus admission the retention cap refused: guided exploration that
# was cut short is not "found nothing").  Zero executions is never a pass.
sub clean {
    my ($r) = @_;
    return ( $r->{iterations} > 0
          && !@{ $r->{crashes} }
          && $r->{timeouts} == 0
          && $r->{ooms} == 0
          && $r->{crashes_dropped} == 0
          && $r->{admissions_dropped} == 0
          && $r->{leaked} == 0 ) ? 1 : 0;
}

sub unique_crashes { return scalar( @{ $_[0]{crashes} } ) + $_[0]{crashes_dropped} }

1;
