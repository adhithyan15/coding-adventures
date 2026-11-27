package CodingAdventures::TestBootstrap::Fuzz::Engine;

# The deterministic, budgeted coverage-guided campaign (VER06 sections 5 and 9.1, VER06q
# section 6).
#
# A campaign is a pure function of (corpus, options, exec): triage every seed, then
# repeatedly pick a seed, give it an energy-scaled number of mutated children, run each
# child and either admit it (new coverage), record it (crash, timeout, OOM) or drop it
# (redundant).  Nothing here reads a clock, a file or the environment and nothing arms a
# timer: the executor does that.
#
# EVERY execution goes through `guard`, the one choke point that (a) refuses to run once
# the leak cap is reached and (b) counts leaked executions.  So the cap holds in triage, the
# mutation loop, every minimisation candidate and re-verification alike.

use strict;
use warnings;

use CodingAdventures::TestBootstrap::Canon             ();
use CodingAdventures::TestBootstrap::Prng              ();
use CodingAdventures::TestBootstrap::Fuzz::Limits      ();
use CodingAdventures::TestBootstrap::Fuzz::Mutate      ();
use CodingAdventures::TestBootstrap::Fuzz::Bitmap      ();
use CodingAdventures::TestBootstrap::Fuzz::Signature   ();
use CodingAdventures::TestBootstrap::Fuzz::Outcome     ();
use CodingAdventures::TestBootstrap::Fuzz::Options     ();
use CodingAdventures::TestBootstrap::Fuzz::Report      ();

# --- captured at load (VER06q section 1)
my $to_bytes      = \&CodingAdventures::TestBootstrap::Canon::to_bytes;
my $new_rng       = \&CodingAdventures::TestBootstrap::Prng::new_rng;
my $seq_counted   = \&CodingAdventures::TestBootstrap::Fuzz::Mutate::mutate_sequence_counted;
my $splice_bytes  = \&CodingAdventures::TestBootstrap::Fuzz::Mutate::splice_bytes;
my $bm_new        = \&CodingAdventures::TestBootstrap::Fuzz::Bitmap::new;
my $bm_admit      = \&CodingAdventures::TestBootstrap::Fuzz::Bitmap::admit;
my $bm_count      = \&CodingAdventures::TestBootstrap::Fuzz::Bitmap::count;
my $calc_energy   = \&CodingAdventures::TestBootstrap::Fuzz::Bitmap::calculate_energy;
my $crash_sig     = \&CodingAdventures::TestBootstrap::Fuzz::Signature::crash_signature;
my $out_is        = \&CodingAdventures::TestBootstrap::Fuzz::Outcome::is;
my $out_is_crash  = \&CodingAdventures::TestBootstrap::Fuzz::Outcome::is_crash;
my $options_is    = \&CodingAdventures::TestBootstrap::Fuzz::Options::is;
my $crash_record  = \&CodingAdventures::TestBootstrap::Fuzz::Report::crash_record;
my $report_new    = \&CodingAdventures::TestBootstrap::Fuzz::Report::new;
my $MAX_LEAKED          = CodingAdventures::TestBootstrap::Fuzz::Limits::MAX_LEAKED();
my $MAX_MINIMIZE_BUDGET = CodingAdventures::TestBootstrap::Fuzz::Limits::MAX_MINIMIZE_BUDGET();
my $DEFAULT_MINIMIZE    = CodingAdventures::TestBootstrap::Fuzz::Limits::DEFAULT_MINIMIZE_BUDGET();
my $MAX_INPUT_BYTES     = CodingAdventures::TestBootstrap::Fuzz::Limits::MAX_INPUT_BYTES();
my $MAX_INITIAL_CORPUS  = CodingAdventures::TestBootstrap::Fuzz::Limits::MAX_INITIAL_CORPUS();
my $MAX_RETAINED_BYTES  = CodingAdventures::TestBootstrap::Fuzz::Limits::MAX_RETAINED_BYTES();
my $MAX_CRASHES         = CodingAdventures::TestBootstrap::Fuzz::Limits::MAX_CRASHES();
my $MAX_SEQUENCE_WORK   = CodingAdventures::TestBootstrap::Fuzz::Limits::MAX_SEQUENCE_WORK();
my $TIER_REPORTED  = CodingAdventures::TestBootstrap::Fuzz::Report::TIER_REPORTED();
my $TIER_BLACK_BOX = CodingAdventures::TestBootstrap::Fuzz::Report::TIER_BLACK_BOX();

# ---------------------------------------------------------------------------
# The leak tracker and the guarded executor
# ---------------------------------------------------------------------------

# A tracker of leaked executions: `count` so far against `cap`.  One tracker is shared by
# every path that runs a target (VER06q 5.4).
sub new_leaks {
    my ( $count, $cap ) = @_;
    return { count => defined $count ? $count : 0, cap => defined $cap ? $cap : $MAX_LEAKED };
}

sub leak_cap_reached { return $_[0]{count} >= $_[0]{cap} ? 1 : 0 }

# Wraps `exec(data) -> (outcome, edges)` so that it (1) returns the empty list, running
# nothing, once the leak cap is reached, (2) refuses anything that is not an Outcome and (3)
# counts an escaped execution.
sub guard {
    my ( $exec, $leaks ) = @_;
    return sub {
        return if leak_cap_reached($leaks);
        my ( $out, $edges ) = $exec->( $_[0] );
        die "exec must return a Fuzz::Outcome\n" if !$out_is->($out);
        $leaks->{count}++ if $out->{escaped};
        return ( $out, $edges );
    };
}

# ---------------------------------------------------------------------------
# Minimisation (VER06 section 5.3)
# ---------------------------------------------------------------------------

sub same_crash {
    my ( $outcome, $sig ) = @_;
    return ( defined $outcome && $out_is_crash->($outcome) && $crash_sig->( $outcome->{crash_kind}, $outcome->{frames} ) eq $sig ) ? 1 : 0;
}

# Shrinks `data` while it keeps the exact signature.  `test(candidate)` returns an outcome, or
# undef when the campaign is aborted.  Returns (bytes, executions spent, finished).  EVERY
# candidate execution, accepted or rejected, is counted and the budget is checked before each
# one, so no stream of rejected candidates can run past it; an aborted `test` ends the
# minimisation with what it has.
sub minimize {
    my ( $data, $sig, $test, $budget ) = @_;
    my $cur   = $data;
    my $spent = 0;
    return ( $cur, 0, 1 ) if $cur eq '';
    # Pass 1: delete blocks, halving the block size.  Starting at max(1, len div 2) (not
    # len div 2) is what makes the empty input reachable from a 1-byte crasher.
    my $chunk = int( length($cur) / 2 );
    $chunk = 1 if $chunk < 1;
    while ( $chunk > 0 ) {
        my $offset = 0;
        while ( $offset < length $cur ) {
            return ( $cur, $spent, 0 ) if $spent >= $budget;
            my $take = length($cur) - $offset;
            $take = $chunk if $chunk < $take;
            my $candidate = substr( $cur, 0, $offset ) . substr( $cur, $offset + $take );
            $spent++;
            my $out = $test->($candidate);
            return ( $cur, $spent, 0 ) if !defined $out;
            if ( same_crash( $out, $sig ) ) { $cur = $candidate }    # same offset again: the tail slid left
            else                            { $offset += $chunk }
        }
        $chunk = int( $chunk / 2 );
    }
    # Pass 2: zero individual non-essential bytes.
    my $i = 0;
    while ( $i < length $cur ) {
        if ( substr( $cur, $i, 1 ) ne "\0" ) {
            return ( $cur, $spent, 0 ) if $spent >= $budget;
            my $candidate = $cur;
            substr( $candidate, $i, 1 ) = "\0";
            $spent++;
            my $out = $test->($candidate);
            return ( $cur, $spent, 0 ) if !defined $out;
            $cur = $candidate if same_crash( $out, $sig );
        }
        $i++;
    }
    return ( $cur, $spent, 1 );
}

# Shrinks `data` while it keeps the EXACT crash signature (never trading one bug for
# another).  `budget` caps executions of `test`, which returns an outcome.
sub minimize_crashing_input {
    my ( $data, $sig, $budget, $test ) = @_;
    $budget = $DEFAULT_MINIMIZE if !defined $budget;
    die "a test function is required\n" if ref $test ne 'CODE';
    die "data must be a string\n" if !defined $data || ref $data;
    die "budget must be an integer in [0, $MAX_MINIMIZE_BUDGET]\n"
      if ref $budget || $budget !~ /\A(?:0|[1-9][0-9]*)\z/ || $budget > $MAX_MINIMIZE_BUDGET;
    die "data exceeds $MAX_INPUT_BYTES bytes\n" if length($data) > $MAX_INPUT_BYTES;
    my ($cur) = minimize( $data, $sig, $test, $budget );
    return $cur;
}

# ---------------------------------------------------------------------------
# The campaign
# ---------------------------------------------------------------------------

# Runs a bounded coverage-guided campaign over `exec`, a function `(data) -> (outcome,
# \@edges)` that runs the code under test and returns the edges it reported (a list of Ints).
# Contracts (VER06 section 9.1), each pinned by a test:
#
#   * a pure function of (corpus, options, exec): same inputs, same report;
#   * with a live corpus `iterations == max_iterations` exactly, and no mutated candidate
#     exceeds `max_input_len` (initial seeds run as given);
#   * an empty corpus is replaced by one 00 seed; seeds that crash, time out or exhaust
#     memory are never mutated, so a corpus of crashers fuzzes nothing;
#   * every execution lands in crashes, timeouts, ooms or the corpus; `clean` needs at least
#     one iteration and no failure at all;
#   * one crash record per signature; a novel crash is minimised.
#
# Per iteration the stream draws: bounded(#corpus) for the seed; per energy slot
# 1 + bounded(4) steps, one next_u64 sub-seed, then, if there is more than one seed,
# bounded(4) and on 0 a donor index and two split draws.
#
# `extras` (all optional): `tier(total) -> string` labels the coverage, `retained_cap` lowers
# the retained-bytes cap (a test seam), `leaks` is the shared leak tracker.
sub run_outcomes {
    my ( $initial, $options, $exec, $extras ) = @_;
    die "options must be Fuzz::Options\n" if !$options_is->($options);
    die "exec must be a function\n" if ref $exec ne 'CODE';
    die "initial_corpus must be a list of strings\n" if ref $initial ne 'ARRAY';
    my $n_seeds = scalar @$initial;
    die "initial corpus exceeds $MAX_INITIAL_CORPUS seeds\n" if $n_seeds > $MAX_INITIAL_CORPUS;
    my @seeds;
    for my $i ( 0 .. $n_seeds - 1 ) {
        my $s = $initial->[$i];
        die "initial corpus seed " . ( $i + 1 ) . " is not a string\n" if !defined $s || ref $s;
        $s = $to_bytes->($s);
        die "initial corpus seed " . ( $i + 1 ) . " exceeds $MAX_INPUT_BYTES bytes\n" if length($s) > $MAX_INPUT_BYTES;
        push @seeds, $s;
    }
    $extras = {} if !defined $extras;
    my $retained_cap = defined $extras->{retained_cap} ? $extras->{retained_cap} : $MAX_RETAINED_BYTES;
    my $tier         = $extras->{tier};
    my $leaks        = $extras->{leaks} || new_leaks();
    my $run_exec     = guard( $exec, $leaks );
    @seeds = ("\0") if !@seeds;

    my $rng        = $new_rng->( $options->{seed} );
    my $cumulative = $bm_new->();
    my ( @corpus, @crashes, %by_sig );
    my ( $timeouts, $ooms, $dropped, $admissions_dropped ) = ( 0, 0, 0, 0 );
    my $minimize_left = $options->{max_iterations};
    my $retained      = 0;

    # Puts a passing input in the corpus iff it covered a new edge.  The first passing seed
    # is always admitted, even with no coverage, so there is something to mutate in
    # black-box mode.  The cap is checked BEFORE the bytes are stored.
    my $admit = sub {
        my ( $data, $generation, $edges ) = @_;
        my ( $has_new, $count_new ) = $bm_admit->( $cumulative, $edges );
        my $first_seed = $generation == 0 && !@corpus;
        return if !( $has_new || $first_seed );
        if ( $retained + length($data) > $retained_cap ) { $admissions_dropped++; return }
        $retained += length $data;
        push @corpus, { data => $data, new_edges => $count_new, generation => $generation };
    };

    # The minimiser's re-run: coverage is discarded and nothing is counted.
    my $probe = sub { my ($out) = $run_exec->( $_[0] ); return $out };

    my $record_crash = sub {
        my ( $out, $data ) = @_;
        my $sig = $crash_sig->( $out->{crash_kind}, $out->{frames} );
        if ( defined( my $at = $by_sig{$sig} ) ) {
            my $rec = $crashes[$at];
            $rec->{hits}++;
            if ( !$rec->{input_dropped} && length($data) < length( $rec->{minimal} ) ) {
                $retained -= length( $rec->{minimal} ) - length($data);
                $rec->{minimal} = $data;
            }
            return;
        }
        if ( @crashes >= $MAX_CRASHES ) { $dropped++; return }
        my $allowance = $options->{minimize_budget} < $minimize_left ? $options->{minimize_budget} : $minimize_left;
        my ( $kept, $spent, $finished ) = minimize( $data, $sig, $probe, $allowance );
        $minimize_left -= $spent;
        my $dropped_input = 0;
        if ( $retained + length($kept) > $retained_cap ) { $kept = ''; $dropped_input = 1 }
        else                                              { $retained += length $kept }
        push @crashes, {
            signature => $sig, kind => $out->{crash_kind}, frames => $out->{frames}, minimal => $kept, hits => 1,
            minimized => ( $finished && !$dropped_input ) ? 1 : 0, input_dropped => $dropped_input,
        };
        $by_sig{$sig} = $#crashes;
    };

    # Puts one non-passing execution in its bucket.
    my $file = sub {
        my ( $out, $data ) = @_;
        if    ( $out->{kind} eq 'timeout' ) { $timeouts++ }
        elsif ( $out->{kind} eq 'oom' )     { $ooms++ }
        elsif ( $out->{kind} eq 'crash' )   { $record_crash->( $out, $data ) }
    };

    # 1. Triage: run every seed once.
    for my $data (@seeds) {
        my ( $out, $edges ) = $run_exec->($data);
        last if !defined $out;    # the leak cap: nothing more runs
        if ( $out->{kind} eq 'pass' ) { $admit->( $data, 0, $edges ) }
        else                          { $file->( $out, $data ) }
    }

    # 2. The mutation loop.
    my $iterations = 0;
    my $stopped    = leak_cap_reached($leaks);
    while ( !$stopped && $iterations < $options->{max_iterations} && @corpus ) {
        my $seed   = $corpus[ $rng->bounded( scalar @corpus ) ];
        my $energy = $calc_energy->( $options->{base_energy}, length( $seed->{data} ), $seed->{new_edges}, $bm_count->($cumulative) );
        my $slot   = 0;
        while ( $slot < $energy && $iterations < $options->{max_iterations} ) {
            $slot++;
            my $steps = 1 + $rng->bounded(4);
            my $sub   = $rng->next_u64;
            my ($candidate) = $seq_counted->( $seed->{data}, $sub, $steps, $MAX_INPUT_BYTES, $MAX_SEQUENCE_WORK );
            $candidate = $seed->{data} if !defined $candidate;    # cannot happen at these sizes; fail safe
            if ( @corpus > 1 && $rng->bounded(4) == 0 ) {
                my $donor   = $corpus[ $rng->bounded( scalar @corpus ) ]{data};
                my $t_split = $rng->bounded( length($candidate) + 1 );
                my $d_split = $rng->bounded( length($donor) + 1 );
                $candidate = $splice_bytes->( $candidate, $donor, $t_split, $d_split );
            }
            $candidate = substr( $candidate, 0, $options->{max_input_len} );
            my ( $out, $edges ) = $run_exec->($candidate);
            if ( !defined $out ) { $stopped = 1; last }
            $iterations++;
            if ( $out->{kind} eq 'pass' ) { $admit->( $candidate, $seed->{generation} + 1, $edges ) }
            else                          { $file->( $out, $candidate ) }
        }
    }

    my @records = map {
        $crash_record->( {
            signature => $_->{signature}, kind => $_->{kind}, frames => $_->{frames}, minimal_input => $_->{minimal},
            hits => $_->{hits}, minimized => $_->{minimized}, input_dropped => $_->{input_dropped},
        } )
    } @crashes;
    my @kept  = map { $_->{data} } @corpus;
    my $total = $bm_count->($cumulative);
    my $label = $tier ? $tier->($total) : ( $total > 0 ? $TIER_REPORTED : $TIER_BLACK_BOX );
    return $report_new->( {
        iterations => $iterations, admitted_seeds => scalar @corpus, total_edges => $total, crashes => \@records,
        timeouts => $timeouts, ooms => $ooms, crashes_dropped => $dropped, admissions_dropped => $admissions_dropped,
        leaked => $leaks->{count}, aborted => leak_cap_reached($leaks), coverage_tier => $label,
        seed => $options->{seed}, corpus => \@kept,
    } );
}

1;
