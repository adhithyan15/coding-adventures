package CodingAdventures::MarkovChain;

# ============================================================================
# CodingAdventures::MarkovChain — Pure-Perl Markov Chain implementation
# ============================================================================
#
# A Markov Chain is a mathematical model of a system that hops between a
# finite set of states over time.  The key insight — called the Markov
# property — is that the next state depends ONLY on the current state, not
# on any of the history that led there.
#
# Analogy: imagine the weather.  Whether tomorrow will be sunny, cloudy, or
# rainy depends mostly on today's weather, not on what the weather was a
# week ago.  A Markov Chain captures this by storing, for every state, the
# probabilities of jumping to each other state.
#
#   P(next = B  |  current = A) = T[A][B]
#
# The full table T is called the "transition matrix".  Every row sums to 1.0
# because the chain must always move to *some* state (possibly the same one).
#
# Historical note
# ===============
# Andrei Andreyevich Markov (1856–1922) introduced this model in 1906 while
# studying the distribution of vowels and consonants in Pushkin's "Eugene
# Onegin".  He wanted to show that the law of large numbers applied to
# *dependent* events, not just independent coin-flips.  Claude Shannon
# then used the same idea in 1948 to model English text statistically.
#
# What this module provides
# =========================
#
#   1. Training: slides a window over a sequence to count how often each
#      state follows each context, then normalises to probabilities.
#
#   2. Sampling: given a current state, picks the next state at random
#      from that state's probability row (CDF sampling).
#
#   3. Generation: chains sampling steps to produce sequences of any length.
#
#   4. Order-k extension: instead of looking back 1 step, remember the last
#      k steps.  The "context key" becomes a k-gram (e.g., "ab" for order=2).
#
#   5. Laplace / Lidstone smoothing: add a small count α to every possible
#      transition before normalising, so no transition has zero probability.
#
#   6. Stationary distribution: finds the long-run fraction of time the
#      chain spends in each state, by power-iteration.
#
# This module uses CodingAdventures::DirectedGraph to track which states
# actually have edges between them (topology), while keeping a separate
# _transitions hash for the numeric probabilities.

use strict;
use warnings;
use List::Util qw(sum);
use CodingAdventures::DirectedGraph;

our $VERSION = '0.1.0';

# ============================================================================
# Constructor
# ============================================================================

# new(%args)
#
# Creates a new, empty Markov Chain.
#
# Keyword arguments:
#
#   order     => $k   (default 1)
#     Memory window length.  order=1 means "depends only on current state".
#     order=2 means "depends on the last 2 states", etc.
#
#   smoothing => $α   (default 0.0)
#     Lidstone smoothing coefficient.  0 = no smoothing.  1 = Laplace.
#     When α > 0, every possible transition from a seen state gets at least
#     α added to its raw count before normalisation.  This prevents the
#     chain from ever getting "stuck" at a state with no outgoing edges.
#
#   states    => \@list   (optional)
#     Pre-register the state alphabet.  Useful when you know all possible
#     states ahead of time (e.g., ['A','B','C'] for a 3-letter alphabet).
#     Pre-registered states participate in smoothing even if never seen as
#     a transition *target* during training.
sub new {
    my ($class, %args) = @_;

    my $order     = $args{order}     // 1;
    my $smoothing = $args{smoothing} // 0.0;
    my @states    = @{ $args{states} // [] };

    # Validate inputs
    die "order must be a positive integer" unless $order >= 1;
    die "smoothing must be >= 0"           unless $smoothing >= 0;

    # The directed graph tracks topology: which states have outgoing edges to
    # which other states.  We use new_allow_self_loops() because a state can
    # transition to itself (e.g., 'B' -> 'B' in "ABBA").
    my $graph = CodingAdventures::DirectedGraph->new_allow_self_loops;

    my $self = bless {
        _order       => $order,
        _smoothing   => $smoothing,
        _graph       => $graph,

        # _counts{context_key}{target} = integer raw count
        # Built up during train(); normalised into _transitions afterwards.
        _counts      => {},

        # _transitions{context_key}{target} = float probability [0,1]
        # After each train() call these are fully renormalised.
        _transitions => {},

        # _states is the master list of all known single-step states
        # (not k-gram contexts, just the atomic state values).
        _states      => {},
    }, $class;

    # Pre-register any states provided at construction time.
    for my $s (@states) {
        $self->{_states}{$s} = 1;
        $self->{_graph}->add_node($s);
    }

    return $self;
}

# ============================================================================
# State registration helpers
# ============================================================================

# _register_state($state)
#
# Adds $state to the known-state set and the graph (if not already present).
# Called automatically during training so callers don't need to pre-register.
sub _register_state {
    my ($self, $state) = @_;
    unless (exists $self->{_states}{$state}) {
        $self->{_states}{$state} = 1;
        $self->{_graph}->add_node($state);
    }
    return;
}

# _all_states()
#
# Returns a sorted list of all known atomic states.
# Sorted for determinism regardless of hash insertion order.
sub _all_states {
    my ($self) = @_;
    return sort keys %{ $self->{_states} };
}

# ============================================================================
# Training
# ============================================================================

# train(\@sequence)  or  train(@sequence)
#
# Slides a window of width (order + 1) over the sequence and accumulates
# transition counts.  After counting, renormalises all rows to probabilities.
#
# May be called multiple times: counts accumulate across calls before
# renormalisation, so the final probabilities reflect all training data.
#
# Example (order=1, sequence=[A,B,A,C]):
#
#   Window [A,B]: context="A"  target="B"  → counts{A}{B}++
#   Window [B,A]: context="B"  target="A"  → counts{B}{A}++
#   Window [A,C]: context="A"  target="C"  → counts{A}{C}++
#
#   After normalisation (no smoothing):
#     A → {B: 0.5, C: 0.5}
#     B → {A: 1.0}
#
# For order=2 the context key is the two-state k-gram joined by a NUL byte,
# e.g., "A\0B" for the context (A, B).
sub train {
    my $self = shift;

    # Accept either an arrayref or a flat list.
    my @sequence;
    if (@_ == 1 && ref($_[0]) eq 'ARRAY') {
        @sequence = @{ $_[0] };
    } else {
        @sequence = @_;
    }

    my $order = $self->{_order};

    # Need at least (order + 1) elements to form even one window.
    return unless @sequence > $order;

    # Register every element of the sequence as a known state.
    for my $s (@sequence) {
        $self->_register_state($s);
    }

    # Slide the (order+1)-wide window across the sequence.
    #
    # For order=1:
    #   i=0: context="seq[0]"         target=seq[1]
    #   i=1: context="seq[1]"         target=seq[2]
    #   ...
    #
    # For order=2:
    #   i=0: context="seq[0]\0seq[1]" target=seq[2]
    #   i=1: context="seq[1]\0seq[2]" target=seq[3]
    #   ...
    #
    # The NUL byte (\0) is the separator: it never appears in typical text,
    # so it can't clash with state names.
    my $n = scalar @sequence;
    for my $i (0 .. $n - $order - 1) {
        # Build the k-gram context key from the current window.
        my $context = join("\0", @sequence[$i .. $i + $order - 1]);
        my $target  = $sequence[$i + $order];

        $self->{_counts}{$context}{$target} //= 0;
        $self->{_counts}{$context}{$target}++;

        # Track the edge in the directed graph for topology queries.
        # For order-1 chains, context == a single state, so it's a normal edge.
        # For order-k chains the context is a k-gram; we still track the
        # last state in the k-gram as the graph node (for stationary_distribution).
        # However, since this is primarily used for topology, we record
        # context -> target as a simple string edge in the graph.
        # The graph nodes are the context keys and target states.
        $self->{_graph}->add_node($context)
            unless $self->{_graph}->has_node($context);
        $self->{_graph}->add_node($target)
            unless $self->{_graph}->has_node($target);
        unless ($self->{_graph}->has_edge($context, $target)) {
            $self->{_graph}->add_edge($context, $target);
        }
    }

    # Renormalise all rows to probabilities.
    $self->_renormalise;

    return;
}

# train_string($text)
#
# Convenience method: splits $text into individual characters and calls train().
#
# Example:
#   $chain->train_string("abcabc");
#   # Equivalent to: $chain->train(['a','b','c','a','b','c'])
sub train_string {
    my ($self, $text) = @_;
    $self->train([split //, $text]);
    return;
}

# _renormalise()
#
# Converts raw counts into probabilities by dividing each count by the
# row total.  Applies Laplace/Lidstone smoothing if $self->{_smoothing} > 0.
#
# Smoothing formula (Lidstone):
#
#   smoothed_count(context → target) = raw_count(context → target) + α
#   row_total                         = Σ_t (raw_count(context → t) + α)
#                                     = Σ_t raw_count(context → t) + α * |Σ|
#
# where |Σ| is the total number of known atomic states.
#
# This ensures every possible target state gets at least a small probability,
# preventing the chain from getting "stuck" at a dead-end context.
sub _renormalise {
    my ($self) = @_;

    my $alpha      = $self->{_smoothing};
    my @all_states = $self->_all_states;
    my $n_states   = scalar @all_states;

    $self->{_transitions} = {};

    for my $context (keys %{ $self->{_counts} }) {
        my %row_counts = %{ $self->{_counts}{$context} };

        # Sum of raw counts for this row.
        my $raw_total = sum(values %row_counts) // 0;

        # Denominator: raw total plus α for every state in the alphabet.
        my $denom = $raw_total + $alpha * $n_states;

        next if $denom <= 0;  # Shouldn't happen, but guard against division by zero.

        for my $target (@all_states) {
            my $raw = $row_counts{$target} // 0;
            my $p   = ($raw + $alpha) / $denom;

            # Only store non-zero probabilities to keep the table sparse.
            if ($p > 0) {
                $self->{_transitions}{$context}{$target} = $p;
            }
        }
    }

    return;
}

# ============================================================================
# Querying and sampling
# ============================================================================

# next_state($current)
#
# Samples one transition from the row T[$current].
#
# Uses CDF (Cumulative Distribution Function) sampling:
#   1. Draw r ∈ [0, 1) uniformly at random.
#   2. Walk through states in sorted order, accumulating probabilities.
#   3. Return the first state where the cumulative probability exceeds r.
#
# Sorted iteration ensures deterministic tie-breaking and makes unit tests
# reproducible when the random seed is fixed externally.
#
# Dies if $current is an unknown context (never seen during training).
sub next_state {
    my ($self, $current) = @_;

    die "unknown state: '$current'"
        unless exists $self->{_transitions}{$current};

    my %row = %{ $self->{_transitions}{$current} };

    # CDF sampling over states in sorted order.
    my $r   = rand(1.0);
    my $cum = 0.0;
    for my $target (sort keys %row) {
        $cum += $row{$target};
        return $target if $r < $cum;
    }

    # Floating-point rounding might leave us just below 1.0 at the end;
    # return the last state (highest probability state in sorted order) as
    # a safe fallback.
    return (sort keys %row)[-1];
}

# generate($start, $length)
#
# Generates a sequence of exactly $length states, starting from $start.
# The $start state IS included in the output as the first element.
#
# For order-1 chains, $start is a single state string.
# For order-k chains (k > 1), $start is a k-character / k-element seed.
#   The function splits it by the NUL separator if needed, or accepts
#   a string that will be split into individual characters.
#
# The sliding-window logic for order-k:
#   - Keep a ring buffer of the last k emitted states.
#   - At each step, build the context key from the ring buffer.
#   - Sample the next state.
#   - Append the next state to the ring buffer (shifting out the oldest).
#
# Returns an arrayref of exactly $length states.
sub generate {
    my ($self, $start, $length) = @_;

    die "length must be a positive integer" unless defined $length && $length >= 1;

    my $order = $self->{_order};

    # For order-k chains, $start must supply k initial states.
    # If $start is a plain string with length >= k, treat each character
    # as an element.  If it contains NUL bytes, split on those instead.
    my @window;
    if (index($start, "\0") >= 0) {
        @window = split /\0/, $start, -1;
    } elsif (length($start) == $order) {
        @window = split //, $start;
    } elsif (length($start) > $order) {
        # Take the last $order characters as the seed window.
        @window = split //, substr($start, -$order);
    } else {
        # Single-state start (or user passed exactly one element for order=1).
        @window = ($start);
    }

    # The output list begins with all the start elements.
    my @result = @window;

    # Generate states until we have exactly $length.
    while (scalar @result < $length) {
        # Build context key from the current window.
        my $context = join("\0", @window);

        die "unknown context: '$context' — cannot generate next state"
            unless exists $self->{_transitions}{$context};

        my $next = $self->next_state($context);
        push @result, $next;

        # Slide the window forward by one: drop oldest, add newest.
        shift @window;
        push  @window, $next;
    }

    # Trim to exactly $length in case the start seed was longer.
    return [splice @result, 0, $length];
}

# generate_string($seed, $length)
#
# Convenience method for character-level chains.
# Trains and generates on individual characters.
# Returns a string of exactly $length characters.
#
# $seed must be at least $order characters long.
sub generate_string {
    my ($self, $seed, $length) = @_;

    my $list = $self->generate($seed, $length);
    return join('', @$list);
}

# ============================================================================
# Probability queries
# ============================================================================

# probability($from, $to)
#
# Returns the trained transition probability T[$from][$to].
# Returns 0.0 if the transition was never observed (and no smoothing was
# applied, or the state was never seen at all).
sub probability {
    my ($self, $from, $to) = @_;

    return $self->{_transitions}{$from}{$to} // 0.0;
}

# ============================================================================
# Stationary distribution (power iteration)
# ============================================================================

# stationary_distribution()
#
# Computes the long-run fraction of time the chain spends in each state.
#
# For an ergodic chain (all states reachable from all others, no periodic
# traps), there exists a unique stationary distribution π such that:
#
#   π · T = π      (π is unchanged by one step of the chain)
#
# This function approximates π using power iteration:
#
#   1. Start with a uniform distribution over all states.
#   2. Multiply by T: each state's new probability is the sum of all the
#      ways to arrive at it from the current distribution.
#   3. Repeat until the distribution stops changing (|π_new - π| < ε).
#
# The iteration converges because T is a stochastic matrix (all rows sum
# to 1) with real eigenvalues between -1 and 1; the dominant eigenvalue
# is exactly 1, and power iteration extracts it.
#
# Note: works on order-1 chains (states are single atomic values).
# For order-k chains the context keys are k-grams, not single states.
# The stationary distribution over k-grams is still computed but the
# keys will be the k-gram strings.
#
# Returns a hashref: { state => probability, ... }.
# Dies if the chain has not been trained (no states).
sub stationary_distribution {
    my ($self) = @_;

    my @all_states = sort keys %{ $self->{_transitions} };
    die "chain has not been trained — no states to distribute over"
        unless @all_states;

    my $n = scalar @all_states;

    # Step 1: initialise π to the uniform distribution.
    my %pi;
    for my $s (@all_states) {
        $pi{$s} = 1.0 / $n;
    }

    my $epsilon   = 1e-10;
    my $max_iters = 10_000;

    for my $iter (1 .. $max_iters) {
        my %pi_new;
        for my $target (@all_states) {
            $pi_new{$target} = 0.0;
        }

        # Multiply π by T: π_new[j] = Σ_i π[i] * T[i][j]
        for my $from (@all_states) {
            next unless exists $self->{_transitions}{$from};
            next unless exists $pi{$from};
            my $p_from = $pi{$from};
            for my $to (keys %{ $self->{_transitions}{$from} }) {
                $pi_new{$to} //= 0.0;
                $pi_new{$to} += $p_from * $self->{_transitions}{$from}{$to};
            }
        }

        # Check convergence: max absolute difference between old and new π.
        my $max_diff = 0.0;
        for my $s (@all_states) {
            my $diff = abs(($pi_new{$s} // 0) - ($pi{$s} // 0));
            $max_diff = $diff if $diff > $max_diff;
        }

        %pi = %pi_new;
        last if $max_diff < $epsilon;
    }

    return \%pi;
}

# ============================================================================
# Inspection
# ============================================================================

# states()
#
# Returns a sorted arrayref of all known atomic states.
# These are individual state values, not k-gram context keys.
sub states {
    my ($self) = @_;
    return [sort keys %{ $self->{_states} }];
}

# transition_matrix()
#
# Returns the full transition matrix as a hashref of hashrefs:
#   { context_key => { target => probability, ... }, ... }
#
# The probabilities in each row sum to 1.0 (or close to it given floating-
# point arithmetic).
sub transition_matrix {
    my ($self) = @_;

    # Return a deep copy so callers cannot accidentally mutate internal state.
    my %copy;
    for my $from (keys %{ $self->{_transitions} }) {
        $copy{$from} = { %{ $self->{_transitions}{$from} } };
    }
    return \%copy;
}

1;

__END__

=head1 NAME

CodingAdventures::MarkovChain - Pure-Perl general-purpose Markov Chain

=head1 SYNOPSIS

    use CodingAdventures::MarkovChain;

    # Order-1 chain, no smoothing
    my $chain = CodingAdventures::MarkovChain->new;
    $chain->train(['A', 'B', 'A', 'C', 'A', 'B', 'B', 'A']);

    printf "P(A->B) = %.3f\n", $chain->probability('A', 'B');  # 0.667
    printf "P(A->C) = %.3f\n", $chain->probability('A', 'C');  # 0.333

    my $seq = $chain->generate('A', 10);   # arrayref of 10 states

    # Order-2 character chain
    my $c2 = CodingAdventures::MarkovChain->new(order => 2);
    $c2->train_string("abcabcabc");
    print $c2->generate_string("ab", 9);   # "abcabcabc"

    # Laplace-smoothed chain with pre-registered alphabet
    my $smooth = CodingAdventures::MarkovChain->new(
        order     => 1,
        smoothing => 1.0,
        states    => ['A', 'B', 'C'],
    );
    $smooth->train(['A', 'B']);
    printf "P(A->C) = %.4f\n", $smooth->probability('A', 'C');  # 0.25

=head1 DESCRIPTION

A general-purpose Markov Chain trained on sequences and used to generate new
sequences by sampling the learned transition probabilities.

Supports:
- Order-k chains (memory window of k steps)
- Laplace / Lidstone smoothing
- Stationary distribution via power iteration
- Character-level text generation convenience methods

=head1 VERSION

Version 0.1.0

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
