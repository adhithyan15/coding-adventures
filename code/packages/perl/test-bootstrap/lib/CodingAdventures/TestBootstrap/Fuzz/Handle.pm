package CodingAdventures::TestBootstrap::Fuzz::Handle;

# What a target sees: coverage reporting and two deterministic budgets (VER06q 5.2).
#
# A campaign makes a FRESH handle for every execution and retires it the moment the target
# returns.  A handle is a blessed EMPTY scalar reference: its state (budgets, counters, the
# edge set, the exhausted marker) lives in the file lexical `%STATE`, keyed by `refaddr`
# (inside-out), so a target holding the handle cannot read or rewrite a counter through it.
# After retirement the state is gone: a retired, retained or forged handle is INERT.  `edge`
# does nothing and `tick` / `reserve` die with the retired sentinel.
#
# SENTINELS.  Budget exhaustion is signalled by `die`ing a unique blessed object (an empty
# scalar) that is recognised by `refaddr` in the private registry `%SENT`
# (`refaddr -> [execution, which]`), NEVER by its message or contents.  The registry entry
# goes when the sentinel does (DESTROY), so it holds only live sentinels.  A sentinel is
# catchable (Perl has no uncatchable throwable); the guarantee comes from STICKINESS (every
# later `tick` / `reserve` dies again, whichever budget broke) plus the executor judging the
# verdict AFTER retirement from the sticky marker and the counters.
#
# The target-facing methods live in `Handle::Object`; this package holds the machinery the
# executor uses (a target given only the object cannot call `retire` or `new` as methods).

use strict;
use warnings;

use Scalar::Util ();

use CodingAdventures::TestBootstrap::Int          ();
use CodingAdventures::TestBootstrap::Prng         ();
use CodingAdventures::TestBootstrap::Fuzz::Bitmap ();

my $refaddr = \&Scalar::Util::refaddr;
my $blessed = \&Scalar::Util::blessed;
my $int_check    = \&CodingAdventures::TestBootstrap::Int::check;
my $int_is_big   = \&CodingAdventures::TestBootstrap::Int::is_big;
my $int_sign     = \&CodingAdventures::TestBootstrap::Int::sign;
my $reduce       = \&CodingAdventures::TestBootstrap::Fuzz::Bitmap::reduce;
my $fnv1a64_word = \&CodingAdventures::TestBootstrap::Prng::fnv1a64_word;

my $OBJECT   = 'CodingAdventures::TestBootstrap::Fuzz::Handle::Object';
my $SENTINEL = 'CodingAdventures::TestBootstrap::Fuzz::Handle::Sentinel';

my %STATE;    # refaddr(handle)   => state hash of a LIVE execution
my %SENT;     # refaddr(sentinel) => [execution, which]
my $EXEC_SEQ = 0;

# ------------------------------------------------------------------- sentinels

package CodingAdventures::TestBootstrap::Fuzz::Handle::Sentinel;

sub DESTROY { delete $SENT{ $refaddr->( $_[0] ) } if ref $_[0] }

package CodingAdventures::TestBootstrap::Fuzz::Handle;

sub _new_sentinel {
    my ( $exec, $which ) = @_;
    my $s = bless \( my $empty = 0 ), $SENTINEL;
    $SENT{ $refaddr->($s) } = [ $exec, $which ];
    return $s;
}

# The retired sentinel: raised by every retired, retained or forged handle.
my $RETIRED = _new_sentinel( 0, 'retired' );

# [execution, which] for a sentinel this module issued, else undef.  Looks the object up by
# identity; it never reads the argument.
sub sentinel_record {
    my ($x) = @_;
    my $c = $blessed->($x);
    return undef if !defined $c || $c ne $SENTINEL;
    return $SENT{ $refaddr->($x) };
}

# ---------------------------------------------------------------------- handles

# (handle, state) for a fresh execution with the given budgets.
sub new {
    my ( $step_budget, $mem_budget ) = @_;
    my $h  = bless \( my $empty = 0 ), $OBJECT;
    my $st = {
        step_budget => $step_budget,
        mem_budget  => $mem_budget,
        steps       => 0,
        reserved    => 0,
        map         => {},
        edge_calls  => 0,
        retired     => 0,
        exhausted   => undef,
        which       => undef,
        exec        => ++$EXEC_SEQ,
        addr        => $refaddr->($h),
    };
    $STATE{ $st->{addr} } = $st;
    return ( $h, $st );
}

# The live state of a handle, or undef for anything else (retired, retained, forged).
sub _state {
    my ($h) = @_;
    my $c = $blessed->($h);
    return undef if !defined $c || $c ne $OBJECT;
    my $st = $STATE{ $refaddr->($h) };
    return ( $st && !$st->{retired} ) ? $st : undef;
}

# The first breach wins and is remembered; the sentinel it made is returned.
sub breach {
    my ( $st, $which ) = @_;
    if ( !defined $st->{exhausted} ) {
        $st->{which}     = $which;
        $st->{exhausted} = _new_sentinel( $st->{exec}, $which );
    }
    return $st->{exhausted};
}

# Reports that the target executed the program point with this id (reduced mod 65,536).  A
# no-op on a retired, retained or forged handle.
sub edge {
    my ( $h, $id ) = @_;
    my $st = _state($h);
    return if !$st;
    $st->{map}{ $reduce->($id) } = 1;
    $st->{edge_calls}++;
    return;
}

# Counts `n` units of work (default 1).  After `step_budget` units the execution is a
# timeout: tick dies with a sentinel and keeps dying on every later call, whichever budget
# broke first.  `n <= 0` counts nothing but still checks.
sub tick {
    my ( $h, $n ) = @_;
    my $st = _state($h);
    die $RETIRED if !$st;
    die $st->{exhausted} if defined $st->{exhausted};
    $n = defined $n ? $int_check->( $n, 'n' ) : 1;
    if ( $n > 0 || $int_is_big->($n) && $int_sign->($n) > 0 ) {
        if ( $int_is_big->($n) || $n > $st->{step_budget} - $st->{steps} ) {
            $st->{steps} = $st->{step_budget} + 1;    # the counters say it too
            die breach( $st, 'steps' );
        }
        $st->{steps} += $n;
    }
    return;
}

# Declares an allocation of `n` bytes about to be made on behalf of the input.  When the
# running total of one execution passes `mem_budget` the execution is an out-of-memory
# outcome.  Non-positive `n` reserves nothing.  Exhaustion is shared with `tick`: the first
# breach of either wins and every later call dies with it.
sub reserve {
    my ( $h, $n ) = @_;
    my $st = _state($h);
    die $RETIRED if !$st;
    die $st->{exhausted} if defined $st->{exhausted};
    $n = $int_check->( $n, 'n' );
    return if $int_sign->($n) <= 0;
    if ( $int_is_big->($n) || $n > $st->{mem_budget} - $st->{reserved} ) {    # a big is above any native budget
        $st->{reserved} = $st->{mem_budget} + 1;
        die breach( $st, 'memory' );
    }
    $st->{reserved} += $n;
    return;
}

# TEST-ONLY, UNSAFE INTERNALS: clears the sticky exhausted marker of a LIVE handle's state, as a
# tampering target would if it could reach the private state.  It exists so a test can prove that the
# verdict is also judged from the counters-versus-budgets cross-check, not from the marker alone
# (the state is a private lexical, so no honest test can do this).  Never called by the engine.
sub _test_clear_exhausted {
    my ($h) = @_;
    my $st = _state($h) or return 0;
    $st->{exhausted} = undef;
    $st->{which}     = undef;
    return 1;
}

# The executor ends the handle's life.  The state entry goes, so the handle is inert.
sub retire {
    my ($st) = @_;
    $st->{retired} = 1;
    delete $STATE{ $st->{addr} };
    return;
}

# "steps" or "memory", whichever budget broke first, or undef.
sub exhausted_which { return $_[0]{which} }

# The budget the COUNTERS say was broken, whatever the marker says.
sub over_budget {
    my ($st) = @_;
    return 'steps'  if $st->{steps} > $st->{step_budget};
    return 'memory' if $st->{reserved} > $st->{mem_budget};
    return undef;
}

# The reduced edge ids of this execution, ascending.
# (hash keys are strings; `0 +` makes each an integer scalar, which is what an Int must be)
sub edge_list { return [ map { 0 + $_ } sort { $a <=> $b } keys %{ $_[0]{map} } ] }

# The stable slot of an FNV-1a label: the low 16 bits of fnv1a64(label), the slot every
# lane reduces the full id to.  Callable as a function or a class method.
sub edge_id {
    shift if @_ > 1;
    my ($label) = @_;
    return $fnv1a64_word->($label) & 0xFFFF;
}

# ------------------------------------------------------------- the target's object

package CodingAdventures::TestBootstrap::Fuzz::Handle::Object;

sub edge    { return CodingAdventures::TestBootstrap::Fuzz::Handle::edge(@_) }
sub tick    { return CodingAdventures::TestBootstrap::Fuzz::Handle::tick(@_) }
sub reserve { return CodingAdventures::TestBootstrap::Fuzz::Handle::reserve(@_) }
sub edge_id { return CodingAdventures::TestBootstrap::Fuzz::Handle::edge_id(@_) }

# A backstop: a handle that dies without being retired leaves no state behind.
sub DESTROY { delete $STATE{ $refaddr->( $_[0] ) } if ref $_[0] }

package CodingAdventures::TestBootstrap::Fuzz::Handle;

# ------------------------------------------------------------------- integrity

# What the three packages looked like when this file finished loading.  A target can reach
# the stashes: `delete $Pkg::{tick}` followed by `sub Pkg::tick {...}` defeats a read-only
# glob.  `intact` notices; `repair` puts the originals back.
my @PACKAGES = ( __PACKAGE__, $OBJECT, $SENTINEL );
my %SNAPSHOT;

sub _subs_of {
    my ($pkg) = @_;
    no strict 'refs';
    my %out;
    for my $name ( keys %{"${pkg}::"} ) {
        my $code = *{"${pkg}::$name"}{CODE};
        $out{$name} = $code if $code;
    }
    return \%out;
}

sub _snapshot { $SNAPSHOT{$_} = _subs_of($_) for @PACKAGES }
_snapshot();

# Are the engine's handle subs, the packages' @ISA and the set of subs in them exactly as
# loaded?
sub intact {
    no strict 'refs';
    for my $pkg (@PACKAGES) {
        return 0 if @{"${pkg}::ISA"};
        my $now  = _subs_of($pkg);
        my $then = $SNAPSHOT{$pkg};
        return 0 if keys %$now != keys %$then;
        for my $name ( keys %$then ) {
            return 0 if !$now->{$name} || $refaddr->( $now->{$name} ) != $refaddr->( $then->{$name} );
        }
    }
    return 1;
}

# Restores everything `intact` checks (and reseals the globs).
sub repair {
    no strict 'refs';
    no warnings 'redefine';
    for my $pkg (@PACKAGES) {
        @{"${pkg}::ISA"} = ();
        my $now  = _subs_of($pkg);
        my $then = $SNAPSHOT{$pkg};
        for my $name ( keys %$now ) {
            delete ${"${pkg}::"}{$name} if !$then->{$name};
        }
        for my $name ( keys %$then ) {
            next if $now->{$name} && $refaddr->( $now->{$name} ) == $refaddr->( $then->{$name} );
            eval { delete ${"${pkg}::"}{$name}; *{"${pkg}::$name"} = $then->{$name}; 1 };
        }
    }
    CodingAdventures::TestBootstrap::Safe::seal(@PACKAGES);
    return;
}

1;
