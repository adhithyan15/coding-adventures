package CodingAdventures::TestBootstrap::Fuzz;

# The Perl lane of the coverage-guided fuzz engine (VER06q).  Core Perl only, no
# dependencies, no I/O (it reads a clock and arms a one-shot alarm per execution).
#
#   use CodingAdventures::TestBootstrap::Fuzz;
#   my $F = 'CodingAdventures::TestBootstrap::Fuzz';
#   my $options = CodingAdventures::TestBootstrap::Fuzz::Options->new( { max_iterations => 500, seed => 7 } );
#   my $report  = CodingAdventures::TestBootstrap::Fuzz::run(
#       ["\0\0"], $options,
#       sub {
#           my ( $data, $h ) = @_;
#           $h->edge( $h->edge_id('entered') );
#           die "boom" if $data eq "\xff\xff";
#       } );
#   CodingAdventures::TestBootstrap::Fuzz::Report::clean($report);
#
# Every execution of `target` runs through `Safe::call` under a one-shot alarm with a fresh
# `Handle` (see `Fuzz::Executor`).  Coverage is what the target reports through
# `Handle::edge` (VER06q section 4.2: the debugger hooks are not used).

use strict;
use warnings;

our $VERSION = '0.01';

use CodingAdventures::TestBootstrap::Safe              ();
use CodingAdventures::TestBootstrap::Fuzz::Limits      ();
use CodingAdventures::TestBootstrap::Fuzz::FailureIds  ();
use CodingAdventures::TestBootstrap::Fuzz::Mutate      ();
use CodingAdventures::TestBootstrap::Fuzz::Bitmap      ();
use CodingAdventures::TestBootstrap::Fuzz::Signature   ();
use CodingAdventures::TestBootstrap::Fuzz::Outcome     ();
use CodingAdventures::TestBootstrap::Fuzz::Options     ();
use CodingAdventures::TestBootstrap::Fuzz::Report      ();
use CodingAdventures::TestBootstrap::Fuzz::Handle      ();
use CodingAdventures::TestBootstrap::Fuzz::Executor    ();
use CodingAdventures::TestBootstrap::Fuzz::Engine      ();
use CodingAdventures::TestBootstrap::Fuzz::Hooks       ();
use CodingAdventures::TestBootstrap::Fuzz::Corpus      ();

# --- captured at load (VER06q section 1)
my $run_outcomes   = \&CodingAdventures::TestBootstrap::Fuzz::Engine::run_outcomes;
my $guard          = \&CodingAdventures::TestBootstrap::Fuzz::Engine::guard;
my $same_crash     = \&CodingAdventures::TestBootstrap::Fuzz::Engine::same_crash;
my $new_leaks      = \&CodingAdventures::TestBootstrap::Fuzz::Engine::new_leaks;
my $executor_make  = \&CodingAdventures::TestBootstrap::Fuzz::Executor::make;
my $report_clean   = \&CodingAdventures::TestBootstrap::Fuzz::Report::clean;
my $report_seed    = \&CodingAdventures::TestBootstrap::Fuzz::Report::seed;
my $tap_failures   = \&CodingAdventures::TestBootstrap::Fuzz::Signature::tap_failures;
my $options_is     = \&CodingAdventures::TestBootstrap::Fuzz::Options::is;
my $options_new    = \&CodingAdventures::TestBootstrap::Fuzz::Options::new;
my $TIER_REPORTED  = CodingAdventures::TestBootstrap::Fuzz::Report::TIER_REPORTED();
my $TIER_BLACK_BOX = CodingAdventures::TestBootstrap::Fuzz::Report::TIER_BLACK_BOX();

sub run_outcomes { return $run_outcomes->(@_) }
sub new_leaks    { return $new_leaks->(@_) }
sub minimize_crashing_input { return CodingAdventures::TestBootstrap::Fuzz::Engine::minimize_crashing_input(@_) }
sub corpus_admit     { return CodingAdventures::TestBootstrap::Fuzz::Bitmap::corpus_admit(@_) }
sub calculate_energy { return CodingAdventures::TestBootstrap::Fuzz::Bitmap::calculate_energy(@_) }
sub mutate_sequence  { return CodingAdventures::TestBootstrap::Fuzz::Mutate::mutate_sequence(@_) }
sub tap_failures     { return $tap_failures->(@_) }
sub format_tap_failure { return CodingAdventures::TestBootstrap::Fuzz::Signature::format_tap_failure(@_) }
sub crash_signature  { return CodingAdventures::TestBootstrap::Fuzz::Signature::crash_signature(@_) }

# The outcome-level executor for `target`: ($exec, $edge_calls), `exec(data) -> (outcome,
# edges)`, one fresh handle and alarm per call, plus a function counting the target's `edge`
# calls so far.  Used by `run` and by `verdict` / `verdict_for` to re-verify.
sub executor {
    my ( $target, $options ) = @_;
    return $executor_make->( $target, defined $options ? $options : $options_new->('CodingAdventures::TestBootstrap::Fuzz::Options') );
}

# Runs a campaign against `target` (a sub `($data, $handle)`).  Any error escaping it is a
# crash.  The coverage tier is `caller-reported` if the target called `Handle::edge` during the
# campaign and `black-box-uninstrumented` otherwise.  `$flags->{leaks}` is an optional shared
# leak tracker (`new_leaks()`): pass the same one to `verdict_for` and the leak cap is shared
# by the campaign and the re-verification.
sub run {
    my ( $initial, $options, $target, $flags ) = @_;
    $options = $options_new->('CodingAdventures::TestBootstrap::Fuzz::Options') if !defined $options;
    die "options must be Fuzz::Options\n" if !$options_is->($options);
    my ( $exec, $edge_calls ) = $executor_make->( $target, $options );
    my $leaks = ( ref $flags eq 'HASH' ) ? $flags->{leaks} : undef;
    my $tier = sub { return $edge_calls->() > 0 ? $TIER_REPORTED : $TIER_BLACK_BOX };
    return $run_outcomes->( $initial, $options, $exec, { tier => $tier, leaks => $leaks } );
}

# The honest reason a report is not a pass, most specific first.
sub _reason_of {
    my ($r) = @_;
    return 'executions-leaked'  if $r->{leaked} > 0;
    return 'crash'              if @{ $r->{crashes} };
    return 'timeout'            if $r->{timeouts} > 0;
    return 'oom'                if $r->{ooms} > 0;
    return 'crashes-dropped'    if $r->{crashes_dropped} > 0;
    return 'admissions-dropped' if $r->{admissions_dropped} > 0;
    return 'no-iterations';
}

# The honest verdict of a report (VER06q section 6).
#
# Before a crash is reported its minimal input is run once more through `exec`
# (`exec(data) -> (outcome, edges)`, guarded by the leak cap): the crash is `reproduced` if the
# signature comes back and `flaky` if it does not (it still fails the verdict, fail closed);
# `unverified` when no input was kept or the leak cap was reached.  `passed` is true only for a
# clean report, so zero executions never pass.  With `options->{ci}` of "on" or "unset" the
# verdict carries no seed and no witness.
#
# `leaks` is the tracker shared with the campaign (see `verdict_for`); without one the cap is
# seeded from `report->{leaked}`, so a campaign that already leaked four executions re-verifies
# nothing.
sub verdict {
    my ( $report, $options, $exec, $target_id, $leaks ) = @_;
    die "options must be Fuzz::Options\n" if !$options_is->($options);
    die "exec must be a function\n" if ref $exec ne 'CODE';
    if ( !defined $target_id ) { $target_id = 'fuzz' }
    elsif ( ref $target_id ) { die "target_id must be a string\n" }
    $leaks = $new_leaks->() if !defined $leaks;
    $leaks->{count} = $report->{leaked} if $report->{leaked} > $leaks->{count};
    my $run_exec = $guard->( $exec, $leaks );
    my $redact   = $options->{ci} ne 'off';
    my $clean    = $report_clean->($report);
    # (`$view` is the report with the leaks of THIS verdict's re-verification added, so the
    # reason and the public lines say so too.)

    my ( @crashes, @witnesses );
    for my $rec ( @{ $report->{crashes} } ) {
        my $status = 'unverified';
        if ( !$rec->{input_dropped} ) {
            my ($out) = $run_exec->( $rec->minimal_input );
            $status = $same_crash->( $out, $rec->{signature} ) ? 'reproduced' : 'flaky' if defined $out;
            push @witnesses, { signature => $rec->{signature}, input => $rec->minimal_input };
        }
        push @crashes, { signature => $rec->{signature}, kind => $rec->{kind}, frames => $rec->{frames}, hits => $rec->{hits}, status => $status };
    }
    my $view = { %$report, leaked => $leaks->{count} };
    return {
        passed     => $clean,
        reason     => $clean ? undef : _reason_of($view),
        lines      => $tap_failures->( $view, $target_id, 1 ),
        crashes    => \@crashes,
        iterations => $report->{iterations},
        timeouts   => $report->{timeouts},
        ooms       => $report->{ooms},
        leaked     => $leaks->{count},
        seed       => $redact ? undef : $report_seed->($report),
        witnesses  => $redact ? undef : \@witnesses,
    };
}

# `verdict` for a report that `run` produced from `target`: re-verification runs the same
# target through an executor wired to ONE leak tracker (`leaks`, or one seeded from the
# report), so the leak cap spans the campaign and its re-verification.
sub verdict_for {
    my ( $target, $report, $options, $target_id, $leaks ) = @_;
    $leaks = $new_leaks->( $report->{leaked} ) if !defined $leaks;
    my ($exec) = $executor_make->( $target, $options );
    return verdict( $report, $options, $exec, $target_id, $leaks );
}

# Library hygiene (VER06q section 1): the new engine subs become read-only globs.
CodingAdventures::TestBootstrap::Safe::seal(
    map { "CodingAdventures::TestBootstrap::Fuzz$_" }
      ( '', qw(::Limits ::FailureIds ::Mutate ::Bitmap ::Signature ::Outcome ::Options ::Report ::Report::CrashRecord
          ::Handle ::Handle::Object ::Handle::Sentinel ::Executor ::Engine ::Hooks ::Corpus) )
);

1;
