package CodingAdventures::TestBootstrap::Corpus;

# The production runner for the shared pbt-v1 corpus
# (`code/specs/fixtures/pbt-v1/cases.json`, VER05 section 6, VER05p section 7).
#
# It takes the raw JSON, strictly decodes it, evaluates every case with THIS
# package's own public engine functions and returns human-readable problems.  An
# empty list means "conforms".  It does no file I/O (the package is
# capability-free); the test that reads the committed fixture lives in the test
# tree.
#
# Why a runner that can fail matters: a conformance check that cannot report a
# problem is worse than none.  So the runner is strict:
#
#   * a case must have exactly the fields of its op, with canonical encodings;
#   * an unknown op, unknown property name or malformed value is a PROBLEM, never a
#     silent skip;
#   * every op and all four failure ids must appear, an unregistered id must be
#     rejected, and the number of cases must equal PBTV1_CASE_COUNT, so deleting
#     cases is itself a problem;
#   * every problem about a case starts with that case's id.

use strict;
use warnings;

use CodingAdventures::TestBootstrap::Int        ();
use CodingAdventures::TestBootstrap::Canon      ();
use CodingAdventures::TestBootstrap::StrictJson ();
use CodingAdventures::TestBootstrap::Values     ();
use CodingAdventures::TestBootstrap::Tfv        ();
use CodingAdventures::TestBootstrap::FailureIds ();
use CodingAdventures::TestBootstrap::Gens       ();
use CodingAdventures::TestBootstrap::Shrink     ();
use CodingAdventures::TestBootstrap::Runner     ();

use constant {
    PBTV1_CASE_COUNT => 25,
    MAX_CASES        => 10_000,
    MAX_PROBLEMS     => 1000,
    MAX_GEN_BYTES    => 1 << 20,    # bounds the total bytes one gen case may generate and hold (CWE-400)
};

my $Int    = 'CodingAdventures::TestBootstrap::Int';
my $Canon  = 'CodingAdventures::TestBootstrap::Canon';
my $Values = 'CodingAdventures::TestBootstrap::Values';
my $Gens   = 'CodingAdventures::TestBootstrap::Gens';

sub _show { return CodingAdventures::TestBootstrap::Values::show( $_[0] ) }
sub _same { return CodingAdventures::TestBootstrap::Values::same( $_[0], $_[1] ) }
sub _fail { CodingAdventures::TestBootstrap::Values::fail( $_[0] ) }

# Strictly decodes a pbt-v1 document into its case list: exactly
# {"schema_version": 1, "cases": [...]}.  Cases are returned undecoded.  Returns
# `$cases` or `(undef, $message)`.
sub parse {
    my ($data) = @_;
    my ( $root, $err ) = CodingAdventures::TestBootstrap::StrictJson::parse($data);
    return ( undef, $err ) if !defined $root;
    return ( undef, 'pbt corpus: want exactly {"schema_version": 1, "cases": [...]}' )
      if $root->{kind} ne 'object' || @{ $root->{keys} } != 2;
    my $version = $root->{map}{schema_version};
    return ( undef, 'pbt corpus: schema_version must be 1' ) if !$version || $version->{kind} ne 'number' || $version->{text} ne '1';
    my $cases = $root->{map}{cases};
    return ( undef, 'pbt corpus: "cases" must be an array' ) if !$cases || $cases->{kind} ne 'array';
    return $cases->{items};
}

# Predicates named by the corpus.  They return the plain 1 when the property HOLDS.
my %PROPERTIES = (
    int_le_100            => sub { CodingAdventures::TestBootstrap::Int::le( $_[0], 100 ) ? 1 : 0 },
    int_in_range_neg10_50 => sub { CodingAdventures::TestBootstrap::Int::le( -10, $_[0] ) && CodingAdventures::TestBootstrap::Int::le( $_[0], 50 ) ? 1 : 0 },
    int_is_even           => sub { CodingAdventures::TestBootstrap::Int::is_even( $_[0] ) },
    int_abs_le_10         => sub { CodingAdventures::TestBootstrap::Int::le( CodingAdventures::TestBootstrap::Int::abs( $_[0] ), 10 ) ? 1 : 0 },
    refinement_x_le_50    => sub { CodingAdventures::TestBootstrap::Int::le( $_[0], 50 ) ? 1 : 0 },
    list_no_consecutive_duplicates => sub {
        my ($l) = @_;
        for my $i ( 0 .. $#$l - 1 ) {
            return 0 if CodingAdventures::TestBootstrap::Int::eq( $l->[$i], $l->[ $i + 1 ] );
        }
        return 1;
    },
    list_sum_le_20 => sub {
        my ($l) = @_;
        my $sum = 0;
        $sum = CodingAdventures::TestBootstrap::Int::add( $sum, $_ ) for @$l;
        return CodingAdventures::TestBootstrap::Int::le( $sum, 20 ) ? 1 : 0;
    },
    bytes_len_lt_5      => sub { length( $_[0] ) < 5 ? 1 : 0 },
    bytes_no_zero_byte  => sub { index( $_[0], "\0" ) < 0 ? 1 : 0 },
);
my @INT_PROPS   = qw(int_le_100 int_in_range_neg10_50 int_is_even int_abs_le_10 refinement_x_le_50);
my @LIST_PROPS  = qw(list_no_consecutive_duplicates list_sum_le_20);
my @BYTES_PROPS = qw(bytes_len_lt_5 bytes_no_zero_byte);

# The closed field set of each op (besides id and op).
my %SHAPES = (
    gen_primitive      => [qw(type seed count outputs)],
    gen_bounded        => [qw(min max seed count outputs)],
    gen_bytes          => [qw(min_len max_len seed count outputs)],
    shrink_int         => [qw(initial target min max property expected_minimal expected_steps)],
    shrink_refinement  => [qw(initial target min max property expected_minimal expected_steps)],
    shrink_list        => [qw(initial elem_target property expected_minimal expected_steps)],
    shrink_bytes       => [qw(initial property expected_minimal expected_steps)],
    replay_consistency => [qw(seed property initial_fail expected_minimal)],
    filter_exhausted   => [qw(max_retries verdict failure_id)],
    failure_id_valid   => [qw(input valid)],
    corpus_reject      => [qw(text)],
);

sub _int_of {
    my ( $value, $what ) = @_;
    _fail("$what must be an int") if !CodingAdventures::TestBootstrap::Int::is($value);
    return $value;
}

# An int that drives a loop or an allocation: 0 <= n <= MAX_SIZE.
sub _size_of {
    my ( $value, $what ) = @_;
    my $n = _int_of( $value, $what );
    _fail( "$what outside [0, " . CodingAdventures::TestBootstrap::Values::MAX_SIZE() . ']' ) if CodingAdventures::TestBootstrap::Int::is_big($n) || $n < 0 || $n > CodingAdventures::TestBootstrap::Values::MAX_SIZE();
    return $n;
}

sub _text_of {
    my ( $value, $what ) = @_;
    _fail("$what must be a string") if !CodingAdventures::TestBootstrap::Canon::is_text($value);
    return CodingAdventures::TestBootstrap::Canon::text_value($value);
}

sub _is_plain_list { return ref $_[0] eq 'ARRAY' && !defined Scalar::Util::blessed( $_[0] ) }

sub _prop_of {
    my ( $name, $allowed, $op ) = @_;
    my $text = CodingAdventures::TestBootstrap::Canon::is_text($name) ? CodingAdventures::TestBootstrap::Canon::text_value($name) : undef;
    _fail( 'unknown property ' . _show($name) . " for $op" ) if !defined $text || !grep { $_ eq $text } @$allowed;
    return $PROPERTIES{$text};
}

sub _check_gen {
    my ( $cid, $op, $v ) = @_;
    my $seed  = _int_of( $v->{seed}, 'seed' );
    my $count = _size_of( $v->{count}, 'count' );
    # The expected outputs are in hand already: a case whose list length is not count
    # cannot conform, so refuse it before generating anything.
    _fail("outputs must be a list of exactly count ($count) values") if !_is_plain_list( $v->{outputs} ) || @{ $v->{outputs} } != $count;
    my $got;
    if ( $op eq 'gen_primitive' ) {
        my $type = _text_of( $v->{type}, 'type' );
        $got = CodingAdventures::TestBootstrap::Gens::gen_primitive( $seed, $type, $count );
        $got = [ map { CodingAdventures::TestBootstrap::Values::bool($_) } @$got ] if $type eq 'bool';
    }
    elsif ( $op eq 'gen_bounded' ) {
        $got = CodingAdventures::TestBootstrap::Gens::gen_bounded( $seed, _int_of( $v->{min}, 'min' ), _int_of( $v->{max}, 'max' ), $count );
    }
    else {
        my $min_len = _size_of( $v->{min_len}, 'min_len' );
        my $max_len = _size_of( $v->{max_len}, 'max_len' );
        _fail("count $count x max_len $max_len exceeds the generation budget of " . MAX_GEN_BYTES . ' bytes') if $count * $max_len > MAX_GEN_BYTES;
        $got = CodingAdventures::TestBootstrap::Gens::gen_bytes( $seed, $min_len, $max_len, $count );
    }
    return [] if _same( $got, $v->{outputs} );
    return ["$cid: $op outputs mismatch"];
}

sub _check_shrink_int {
    my ( $cid, $op, $v ) = @_;
    my $prop    = _prop_of( $v->{property}, \@INT_PROPS, $op );
    my $initial = _int_of( $v->{initial}, 'initial' );
    my $target  = _int_of( $v->{target},  'target' );
    my $lo      = _int_of( $v->{min},     'min' );
    my $hi      = _int_of( $v->{max},     'max' );
    my ( $minimal, $steps ) = CodingAdventures::TestBootstrap::Shrink::shrink_int( $initial, $target, $lo, $hi, $prop, 1000 );
    my @problems;
    push @problems, sprintf( '%s: %s minimal mismatch: got %s, corpus %s', $cid, $op, _show($minimal), _show( $v->{expected_minimal} ) ) if !_same( $minimal, $v->{expected_minimal} );
    push @problems, sprintf( '%s: %s step count mismatch: got %s, corpus %s', $cid, $op, _show($steps), _show( $v->{expected_steps} ) ) if !_same( $steps, $v->{expected_steps} );
    # Engine invariants, independent of the corpus's expected values.
    push @problems, sprintf( '%s: %s minimal %s does not falsify the property', $cid, $op, _show($minimal) ) if $prop->($minimal) == 1;
    push @problems, sprintf( '%s: %s minimal %s left [%s, %s]', $cid, $op, _show($minimal), CodingAdventures::TestBootstrap::Int::tostring($lo), CodingAdventures::TestBootstrap::Int::tostring($hi) )
      if !( CodingAdventures::TestBootstrap::Int::le( $lo, $minimal ) && CodingAdventures::TestBootstrap::Int::le( $minimal, $hi ) );
    return \@problems;
}

sub _check_shrink_list {
    my ( $cid, $v ) = @_;
    my $prop = _prop_of( $v->{property}, \@LIST_PROPS, 'shrink_list' );
    _fail('initial must be a list of ints') if !_is_plain_list( $v->{initial} );
    my @initial     = map { _int_of( $_, 'element' ) } @{ $v->{initial} };
    my $elem_target = _int_of( $v->{elem_target}, 'elem_target' );
    my ( $minimal, $steps ) = CodingAdventures::TestBootstrap::Shrink::shrink_list( \@initial, $elem_target, $prop, 1000 );
    my @problems;
    push @problems, sprintf( '%s: shrink_list minimal mismatch: got %s, corpus %s', $cid, _show($minimal), _show( $v->{expected_minimal} ) ) if !_same( $minimal, $v->{expected_minimal} );
    push @problems, sprintf( '%s: shrink_list steps mismatch: got %s, corpus %s', $cid, _show($steps), _show( $v->{expected_steps} ) ) if !_same( $steps, $v->{expected_steps} );
    push @problems, sprintf( '%s: shrink_list minimal %s does not falsify the property', $cid, _show($minimal) ) if $prop->($minimal) == 1;
    return \@problems;
}

sub _check_shrink_bytes {
    my ( $cid, $v ) = @_;
    my $prop = _prop_of( $v->{property}, \@BYTES_PROPS, 'shrink_bytes' );
    _fail('initial must be bytes') if !defined $v->{initial} || ref $v->{initial};
    my ( $minimal, $steps ) = CodingAdventures::TestBootstrap::Shrink::shrink_bytes( $v->{initial}, $prop, 1000 );
    my @problems;
    push @problems, sprintf( '%s: shrink_bytes minimal mismatch: got %s, corpus %s', $cid, _show($minimal), _show( $v->{expected_minimal} ) ) if !_same( $minimal, $v->{expected_minimal} );
    push @problems, sprintf( '%s: shrink_bytes steps mismatch: got %s, corpus %s', $cid, _show($steps), _show( $v->{expected_steps} ) ) if !_same( $steps, $v->{expected_steps} );
    push @problems, sprintf( '%s: shrink_bytes minimal %s does not falsify the property', $cid, _show($minimal) ) if $prop->($minimal) == 1;
    return \@problems;
}

sub _check_replay {
    my ( $cid, $v ) = @_;
    my $seed = _int_of( $v->{seed}, 'seed' );
    my $prop = _prop_of( $v->{property}, \@INT_PROPS, 'replay_consistency' );
    my ( $initial_fail, $minimal ) = CodingAdventures::TestBootstrap::Shrink::simulate_replay( $seed, $prop );
    my @problems;
    push @problems, sprintf( '%s: replay initial_fail mismatch: got %s, corpus %s', $cid, _show($initial_fail), _show( $v->{initial_fail} ) ) if !_same( $initial_fail, $v->{initial_fail} );
    push @problems, sprintf( '%s: replay minimal mismatch: got %s, corpus %s', $cid, _show($minimal), _show( $v->{expected_minimal} ) ) if !_same( $minimal, $v->{expected_minimal} );
    return \@problems;
}

sub _check_filter_exhausted {
    my ( $cid, $v ) = @_;
    my $max_retries = _size_of( $v->{max_retries}, 'max_retries' );
    _fail('max_retries must be at least 1') if $max_retries < 1;
    my $calls = 0;
    my $gen   = CodingAdventures::TestBootstrap::Gens::filter( CodingAdventures::TestBootstrap::Gens::integer( 0, 10 ), sub { $calls++; return 0 }, $max_retries );
    my $verdict = CodingAdventures::TestBootstrap::Runner::check( 0, $gen, sub { return 1 },
        CodingAdventures::TestBootstrap::Runner::Options->new( { num_tests => 1, max_shrink_steps => 1000, ci => 'on' } ) );
    my @problems;
    push @problems, "$cid: an unsatisfiable filter must be a fail verdict" if !( _same( $v->{verdict}, CodingAdventures::TestBootstrap::Canon::text('fail') ) && !$verdict->{passed} );
    my $got_id = defined $verdict->{failure_id} ? CodingAdventures::TestBootstrap::Canon::text( $verdict->{failure_id} ) : undef;
    push @problems, sprintf( '%s: failure id %s, corpus %s', $cid, _show($got_id), _show( $v->{failure_id} ) ) if !_same( $got_id, $v->{failure_id} );
    push @problems, sprintf( '%s: filter sampled %d times, expected %d', $cid, $calls, $max_retries ) if $calls != $max_retries;
    return \@problems;
}

# Decodes and checks one case.  Any error (a malformed case or an engine that died)
# is reported as a problem.  Returns (\@found, $candidate, $registered).
sub _evaluate_case {
    my ( $cid, $op, $raw ) = @_;
    my %v;
    for my $k ( @{ $raw->{keys} } ) {
        $v{$k} = CodingAdventures::TestBootstrap::Values::decode( $raw->{map}{$k} ) if $k ne 'id' && $k ne 'op';
    }
    my ( $found, $candidate, $registered ) = ( [], '', 0 );
    if    ( $op eq 'gen_primitive' || $op eq 'gen_bounded' || $op eq 'gen_bytes' ) { $found = _check_gen( $cid, $op, \%v ) }
    elsif ( $op eq 'shrink_int' || $op eq 'shrink_refinement' )                      { $found = _check_shrink_int( $cid, $op, \%v ) }
    elsif ( $op eq 'shrink_list' )                                                   { $found = _check_shrink_list( $cid, \%v ) }
    elsif ( $op eq 'shrink_bytes' )                                                  { $found = _check_shrink_bytes( $cid, \%v ) }
    elsif ( $op eq 'replay_consistency' )                                            { $found = _check_replay( $cid, \%v ) }
    elsif ( $op eq 'filter_exhausted' )                                              { $found = _check_filter_exhausted( $cid, \%v ) }
    elsif ( $op eq 'failure_id_valid' ) {
        my $input = CodingAdventures::TestBootstrap::Canon::is_text( $v{input} ) ? CodingAdventures::TestBootstrap::Canon::text_value( $v{input} ) : undef;
        my $got = defined $input ? CodingAdventures::TestBootstrap::FailureIds::is_failure_id($input) : 0;
        push @$found, sprintf( '%s: failure id membership disagrees for %s', $cid, _show( $v{input} ) ) if !_same( CodingAdventures::TestBootstrap::Values::bool($got), $v{valid} );
        $candidate  = defined $input ? $input : '';
        $registered = $got;
    }
    else {    # corpus_reject
        my $text = _text_of( $v{text}, 'text' );
        push @$found, "$cid: corpus_reject text was accepted by the strict parser" if CodingAdventures::TestBootstrap::Tfv::valid($text);
    }
    return ( $found, $candidate, $registered );
}

sub _safe_evaluate_case {
    my ( $cid, $op, $raw ) = @_;
    my ( $ok, $found, $candidate, $registered, $err );
    {
        local $@;
        local $SIG{__DIE__} = 'DEFAULT';
        $ok = eval { ( $found, $candidate, $registered ) = _evaluate_case( $cid, $op, $raw ); 1 };
        $err = $@ if !$ok;
    }
    return ( $found, 1, $candidate, $registered ) if $ok;
    return ( [ sprintf( '%s: %s could not be evaluated: %s', $cid, $op, CodingAdventures::TestBootstrap::Values::error_message($err) ) ], 0, '', 0 );
}

sub _same_key_set {
    my ( $have, $want ) = @_;
    my @h = sort @$have;
    my @w = sort @$want;
    return 0 if @h != @w;
    for my $i ( 0 .. $#h ) { return 0 if $h[$i] ne $w[$i] }
    return 1;
}

# Evaluates every case; returns `{ evaluated => n, problems => [...] }`.  `$cases_in`
# is the case list (an array of JSON nodes).  An undef `$expected` means 25; pass 0
# to skip the case-count check (ad-hoc case lists in tests).
sub evaluate {
    my ( $cases_in, $expected ) = @_;
    $expected = PBTV1_CASE_COUNT if !defined $expected;
    my @problems;
    my $evaluated = 0;
    my $cases     = $cases_in;
    push @problems, sprintf( 'pbt-v1: expected %d cases, got %d', $expected, scalar @$cases ) if $expected && @$cases != $expected;
    if ( @$cases > MAX_CASES ) {
        push @problems, sprintf( 'pbt-v1: %d cases exceeds the cap of %d', scalar @$cases, MAX_CASES );
        $cases = [ @{$cases}[ 0 .. MAX_CASES - 1 ] ];
    }

    my %seen_ids;
    for my $raw (@$cases) {
        my $id = ( ref $raw eq 'HASH' && $raw->{kind} eq 'object' && ref $raw->{map}{id} eq 'HASH' && $raw->{map}{id}{kind} eq 'string' ) ? $raw->{map}{id}{value} : undef;
        if ( !defined $id || $id eq '' || $seen_ids{$id}++ ) {
            push @problems, 'duplicate or missing case id';
            last;
        }
    }

    my ( %ops_seen, %registered_seen, $rejected_id_seen );
    for my $raw (@$cases) {
        if ( ref $raw ne 'HASH' || $raw->{kind} ne 'object' ) {
            push @problems, 'case is not an object: ' . _show($raw);
        }
        elsif ( ref $raw->{map}{id} ne 'HASH' || $raw->{map}{id}{kind} ne 'string' || $raw->{map}{id}{value} eq '' ) {
            push @problems, 'case missing a string id';
        }
        else {
            my $cid = CodingAdventures::TestBootstrap::Canon::escape( CodingAdventures::TestBootstrap::Canon::clip( $raw->{map}{id}{value}, 100 ) );
            my $op = ( ref $raw->{map}{op} eq 'HASH' && $raw->{map}{op}{kind} eq 'string' ) ? $raw->{map}{op}{value} : '';
            if ( !exists $SHAPES{$op} ) {
                push @problems, "$cid: unknown op " . _show( $raw->{map}{op} && $raw->{map}{op}{value} );
            }
            else {
                my @want = ( 'id', 'op', @{ $SHAPES{$op} } );
                if ( !_same_key_set( $raw->{keys}, \@want ) ) {
                    push @problems, sprintf( '%s: fields do not match %s', $cid, join( ',', sort @want ) );
                }
                else {
                    my ( $found, $ran_ok, $candidate, $registered ) = _safe_evaluate_case( $cid, $op, $raw );
                    push @problems, @$found;
                    if ($ran_ok) {
                        if ( $op eq 'failure_id_valid' ) {
                            if ($registered) { $registered_seen{$candidate} = 1 }
                            else             { $rejected_id_seen = 1 }
                        }
                        $ops_seen{$op} = 1;
                        $evaluated++;
                    }
                }
            }
        }
    }

    my @missing_ops = grep { !$ops_seen{$_} } sort keys %SHAPES;
    push @problems, 'pbt-v1: operations never exercised: ' . join( ',', @missing_ops ) if @missing_ops;
    my @missing_ids = grep { !$registered_seen{$_} } @CodingAdventures::TestBootstrap::FailureIds::ALL;
    push @problems, 'pbt-v1: registered failure ids missing: ' . join( ',', @missing_ids ) if @missing_ids;
    push @problems, 'pbt-v1: failure_id_valid needs an unregistered-id rejection' if !$rejected_id_seen;

    my @out;
    for my $p (@problems) {
        last if @out >= MAX_PROBLEMS;
        push @out, CodingAdventures::TestBootstrap::Canon::clip( $p, CodingAdventures::TestBootstrap::Values::MAX_PROBLEM_LEN() );
    }
    return { evaluated => $evaluated, problems => \@out };
}

# Parses `$data` strictly and evaluates it.  Returns the report; a document that
# fails to parse is one problem.
sub evaluate_document {
    my ($data) = @_;
    my ( $cases, $err ) = parse($data);
    return { evaluated => 0, problems => ["pbt-v1: $err"] } if !defined $cases;
    return evaluate($cases);
}

1;
