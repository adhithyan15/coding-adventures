package CodingAdventures::TestBootstrap::Fuzz::Corpus;

# The production runner for the shared fuzz-v1 corpus
# (`code/specs/fixtures/fuzz-v1/cases.json`, VER06 section 8, VER06q section 8).
#
# It takes the raw JSON, strictly decodes it, evaluates every case with THIS package's own
# public engine functions and returns human-readable problems.  An empty list means
# "conforms".  It does no file I/O (the package is capability-free apart from the clock); the
# tests read the committed fixture.
#
# A runner that cannot report a problem is worse than none, so this one is strict:
#
#   * a case has exactly the fields of its op, in canonical encodings;
#   * an unknown op or a malformed value is a PROBLEM, never a silent skip;
#   * every op appears, a failure id is accepted and one rejected, and the case count equals
#     FUZZV1_CASE_COUNT, so deleting cases is itself a problem;
#   * fixture-controlled sizes are capped (CWE-400): a hostile `steps = 10**9` is a reported
#     problem, not a hang;
#   * every problem about a case starts with that case's id, ESCAPED (an id comes from the
#     fixture and may hold a newline).

use strict;
use warnings;

use B ();
use Scalar::Util ();

use CodingAdventures::TestBootstrap::Int              ();
use CodingAdventures::TestBootstrap::Canon            ();
use CodingAdventures::TestBootstrap::StrictJson       ();
use CodingAdventures::TestBootstrap::Values           ();
use CodingAdventures::TestBootstrap::Fuzz::Limits     ();
use CodingAdventures::TestBootstrap::Fuzz::FailureIds ();
use CodingAdventures::TestBootstrap::Fuzz::Hooks      ();

use constant {
    FUZZV1_CASE_COUNT => 25,
    MAX_CASES         => 10_000,
    MAX_PROBLEMS      => 1000,
};

my $Int    = 'CodingAdventures::TestBootstrap::Int';
my $Canon  = 'CodingAdventures::TestBootstrap::Canon';
my $Values = 'CodingAdventures::TestBootstrap::Values';
my $TWO64  = CodingAdventures::TestBootstrap::Int::pow2(64);
my $MAX_INPUT_BYTES        = CodingAdventures::TestBootstrap::Fuzz::Limits::MAX_INPUT_BYTES();
my $MAX_CORPUS_SEQUENCE_WORK = CodingAdventures::TestBootstrap::Fuzz::Limits::MAX_CORPUS_SEQUENCE_WORK();

# The hooks of the run in progress: copied when `evaluate` starts.
my %HK;

sub _show { return CodingAdventures::TestBootstrap::Values::show( $_[0] ) }
sub _fail { CodingAdventures::TestBootstrap::Values::fail( $_[0] ) }

# Strictly decodes a fuzz-v1 document into its case list: exactly
# {"schema_version": 1, "cases": [...]}.  Cases are returned undecoded.  Returns `$cases` or
# `(undef, $message)`.
sub parse {
    my ($data) = @_;
    my ( $root, $err ) = CodingAdventures::TestBootstrap::StrictJson::parse($data);
    return ( undef, $err ) if !defined $root;
    return ( undef, 'fuzz corpus: want exactly {"schema_version": 1, "cases": [...]}' )
      if $root->{kind} ne 'object' || @{ $root->{keys} } != 2;
    my $version = $root->{map}{schema_version};
    return ( undef, 'fuzz corpus: schema_version must be 1' ) if !$version || $version->{kind} ne 'number' || $version->{text} ne '1';
    my $cases = $root->{map}{cases};
    return ( undef, 'fuzz corpus: "cases" must be an array' ) if !$cases || $cases->{kind} ne 'array';
    return $cases->{items};
}

# The closed field set of each op (besides id and op).
my %SHAPES = (
    mutate_bit_flip        => [qw(input bit_offset count output)],
    mutate_byte_flip       => [qw(input byte_offset count output)],
    mutate_arithmetic      => [qw(input byte_offset delta width endian output)],
    mutate_boundary        => [qw(input byte_offset width val endian output)],
    mutate_block_delete    => [qw(input offset length output)],
    mutate_block_insert    => [qw(input offset insert_bytes output)],
    mutate_block_duplicate => [qw(input src_offset length dst_offset output)],
    mutate_splice          => [qw(target donor target_split donor_split output)],
    mutate_sequence        => [qw(input seed steps output)],
    corpus_admit           => [qw(existing_edges trace_edges admitted new_edges)],
    crash_dedup            => [qw(crash_a_kind crash_a_frames crash_b_kind crash_b_frames is_duplicate)],
    failure_id_valid       => [qw(input valid)],
    private_reporting      => [qw(target failure_id raw_payload sig_hash tap_line contains_raw_payload)],
    corpus_reject          => [qw(text)],
);

sub _same_key_set {
    my ( $have, $want ) = @_;
    my @h = sort @$have;
    my @w = sort @$want;
    return 0 if @h != @w;
    for my $i ( 0 .. $#h ) { return 0 if $h[$i] ne $w[$i] }
    return 1;
}

# ------------------------------------------------------- typed field access

# A byte string: a defined scalar that is a string and not a number (decoded `bytes` values
# are plain strings; ints are integers, text is a wrapped record).
sub _is_bytes {
    return 0 if !defined $_[0] || ref $_[0];
    my $f = B::svref_2object( \$_[0] )->FLAGS;
    return ( ( $f & B::SVf_POK ) && !( $f & ( B::SVf_IOK | B::SVf_NOK ) ) ) ? 1 : 0;
}

sub _bytes_of {
    my ( $v, $what ) = @_;
    _fail("$what must be bytes") if !_is_bytes($v);
    return $v;
}

sub _text_of {
    my ( $v, $what ) = @_;
    _fail("$what must be a string") if !CodingAdventures::TestBootstrap::Canon::is_text($v);
    return CodingAdventures::TestBootstrap::Canon::text_value($v);
}

sub _bool_of {
    my ( $v, $what ) = @_;
    my $c = Scalar::Util::blessed($v);
    _fail("$what must be a bool") if !defined $c || $c ne 'CodingAdventures::TestBootstrap::Values::Bool';
    return ${$v} ? 1 : 0;
}

sub _int_of {
    my ( $v, $what ) = @_;
    _fail("$what must be an int") if !CodingAdventures::TestBootstrap::Int::is($v);
    return $v;
}

# An int that drives a loop or an allocation: 0 <= n <= MAX_SIZE.
sub _size_of {
    my ( $v, $what ) = @_;
    my $n = _int_of( $v, $what );
    _fail( "$what outside [0, " . CodingAdventures::TestBootstrap::Values::MAX_SIZE() . ']' )
      if CodingAdventures::TestBootstrap::Int::is_big($n) || $n < 0 || $n > CodingAdventures::TestBootstrap::Values::MAX_SIZE();
    return $n;
}

sub _is_plain_list { return ref $_[0] eq 'ARRAY' && !defined Scalar::Util::blessed( $_[0] ) }

sub _texts_of {
    my ( $v, $what ) = @_;
    _fail("$what must be a list of strings") if !_is_plain_list($v);
    return [ map { _fail("$what must be a list of strings") if !CodingAdventures::TestBootstrap::Canon::is_text($_); CodingAdventures::TestBootstrap::Canon::text_value($_) } @$v ];
}

# A list of edge ids, each within [0, 2**64).
sub _edges_of {
    my ( $v, $what ) = @_;
    _fail("$what must be a list of ints") if !_is_plain_list($v);
    my @out;
    for my $e (@$v) {
        _fail("$what must be a list of ints") if !CodingAdventures::TestBootstrap::Int::is($e);
        _fail( "$what element " . _show($e) . ' outside [0, 2^64)' )
          if CodingAdventures::TestBootstrap::Int::sign($e) < 0 || CodingAdventures::TestBootstrap::Int::cmp( $e, $TWO64 ) >= 0;
        push @out, $e;
    }
    return \@out;
}

sub _hex { return CodingAdventures::TestBootstrap::Canon::hex_of( $_[0] ) }

# ------------------------------------------------------------ per-op checks

sub _clip { return CodingAdventures::TestBootstrap::Canon::clip( $_[0], CodingAdventures::TestBootstrap::Values::MAX_FRAGMENT_LEN() ) }

sub _compare {
    my ( $cid, $op, $got, $v ) = @_;
    my $want = $v->{output};
    return ["$cid: $op output must be bytes"] if !_is_bytes($want);
    return [] if defined $got && !ref $got && $got eq $want;
    return [ sprintf( '%s: %s expected %s, got %s', $cid, $op, _clip( _hex($want) ), ( defined $got && !ref $got ) ? _clip( _hex($got) ) : 'nothing' ) ];
}

# Charges the REAL work: the call is handed what is left of the aggregate cap and what it
# actually copied (inserts and duplicates grow the buffer, so steps x initial length would
# undercount) is deducted afterwards.  A call that hits its cap is charged too, the aggregate
# is then ZEROED, and an exhausted aggregate fails every later case at once without running it.
sub _sequence_result {
    my ( $v, $work ) = @_;
    my $input = _bytes_of( $v->{input}, 'input' );
    my $seed  = _int_of( $v->{seed}, 'seed' );
    my $steps = _size_of( $v->{steps}, 'steps' );
    my $exhausted = "mutate_sequence work budget of $MAX_CORPUS_SEQUENCE_WORK byte-steps exhausted";
    _fail($exhausted) if $$work <= 0;
    my ( $data, $done ) = $HK{mutate_sequence}->( $input, $seed, $steps, $MAX_INPUT_BYTES, $$work );
    if ( !defined $data ) { $$work = 0; _fail($exhausted) }
    $$work -= $done;
    return $data;
}

sub _clamp_split {
    my ( $x, $hi ) = @_;
    return 0 if CodingAdventures::TestBootstrap::Int::sign($x) < 0;
    return $hi if CodingAdventures::TestBootstrap::Int::is_big($x) || $x > $hi;
    return $x;
}

sub _check_mutator {
    my ( $cid, $op, $v, $work ) = @_;
    my $got;
    if ( $op eq 'mutate_bit_flip' ) {
        $got = $HK{bit_flip}->( _bytes_of( $v->{input}, 'input' ), _int_of( $v->{bit_offset}, 'bit_offset' ), _int_of( $v->{count}, 'count' ) );
    }
    elsif ( $op eq 'mutate_byte_flip' ) {
        $got = $HK{byte_flip}->( _bytes_of( $v->{input}, 'input' ), _int_of( $v->{byte_offset}, 'byte_offset' ), _int_of( $v->{count}, 'count' ) );
    }
    elsif ( $op eq 'mutate_arithmetic' ) {
        $got = $HK{byte_arithmetic}->( _bytes_of( $v->{input}, 'input' ), _int_of( $v->{byte_offset}, 'byte_offset' ),
            _int_of( $v->{delta}, 'delta' ), _int_of( $v->{width}, 'width' ), _text_of( $v->{endian}, 'endian' ) );
    }
    elsif ( $op eq 'mutate_boundary' ) {
        $got = $HK{boundary_insert}->( _bytes_of( $v->{input}, 'input' ), _int_of( $v->{byte_offset}, 'byte_offset' ),
            _int_of( $v->{width}, 'width' ), _int_of( $v->{val}, 'val' ), _text_of( $v->{endian}, 'endian' ) );
    }
    elsif ( $op eq 'mutate_block_delete' ) {
        $got = $HK{block_delete}->( _bytes_of( $v->{input}, 'input' ), _int_of( $v->{offset}, 'offset' ), _int_of( $v->{length}, 'length' ) );
    }
    elsif ( $op eq 'mutate_block_insert' ) {
        $got = $HK{block_insert}->( _bytes_of( $v->{input}, 'input' ), _int_of( $v->{offset}, 'offset' ), _bytes_of( $v->{insert_bytes}, 'insert_bytes' ) );
    }
    elsif ( $op eq 'mutate_block_duplicate' ) {
        $got = $HK{block_duplicate}->( _bytes_of( $v->{input}, 'input' ), _int_of( $v->{src_offset}, 'src_offset' ),
            _int_of( $v->{length}, 'length' ), _int_of( $v->{dst_offset}, 'dst_offset' ) );
    }
    elsif ( $op eq 'mutate_splice' ) {
        $got = $HK{splice}->( _bytes_of( $v->{target}, 'target' ), _bytes_of( $v->{donor}, 'donor' ),
            _int_of( $v->{target_split}, 'target_split' ), _int_of( $v->{donor_split}, 'donor_split' ) );
    }
    else {
        $got = _sequence_result( $v, $work );
    }
    my $problems = _compare( $cid, $op, $got, $v );
    if ( defined $got && !ref $got ) {
        if ( $op =~ /\Amutate_(?:bit_flip|byte_flip|arithmetic|boundary)\z/ ) {
            push @$problems, "$cid: $op changed the length of its input" if length($got) != length( $v->{input} );
        }
        elsif ( $op eq 'mutate_splice' ) {
            my $want = _clamp_split( $v->{target_split}, length $v->{target} ) + length( $v->{donor} ) - _clamp_split( $v->{donor_split}, length $v->{donor} );
            push @$problems, "$cid: splice length is not prefix + suffix" if length($got) != $want;
        }
    }
    return $problems;
}

sub _check_corpus_admit {
    my ( $cid, $v ) = @_;
    my $existing  = _edges_of( $v->{existing_edges}, 'existing_edges' );
    my $trace     = _edges_of( $v->{trace_edges}, 'trace_edges' );
    my $admitted  = _bool_of( $v->{admitted}, 'admitted' );
    my $new_edges = _int_of( $v->{new_edges}, 'new_edges' );
    my ( $got_admitted, $got_new ) = $HK{corpus_admit}->( $existing, $trace );
    my @problems;
    push @problems, sprintf( '%s: corpus_admit admitted %s, corpus %s', $cid, $got_admitted ? 1 : 0, $admitted ) if ( $got_admitted ? 1 : 0 ) != $admitted;
    push @problems, sprintf( '%s: corpus_admit new_edges %s, corpus %s', $cid, _show($got_new), _show($new_edges) )
      if !CodingAdventures::TestBootstrap::Int::is($got_new) || !CodingAdventures::TestBootstrap::Int::eq( $got_new, $new_edges );
    # Invariant: admission is exactly "some trace edge is unseen".
    my %have = map { CodingAdventures::TestBootstrap::Int::tostring($_) => 1 } @$existing;
    my $any = grep { !$have{ CodingAdventures::TestBootstrap::Int::tostring($_) } } @$trace;
    push @problems, "$cid: corpus_admit disagrees with set difference" if ( $got_admitted ? 1 : 0 ) != ( $any ? 1 : 0 );
    return \@problems;
}

sub _check_crash_dedup {
    my ( $cid, $v ) = @_;
    my $ka   = _text_of( $v->{crash_a_kind}, 'crash_a_kind' );
    my $fa   = _texts_of( $v->{crash_a_frames}, 'crash_a_frames' );
    my $kb   = _text_of( $v->{crash_b_kind}, 'crash_b_kind' );
    my $fb   = _texts_of( $v->{crash_b_frames}, 'crash_b_frames' );
    my $want = _bool_of( $v->{is_duplicate}, 'is_duplicate' );
    my $got  = $HK{duplicate_crash}->( $ka, $fa, $kb, $fb ) ? 1 : 0;
    return [] if $got == $want;
    return [ sprintf( '%s: crash_dedup is_duplicate %s, corpus %s', $cid, $got, $want ) ];
}

sub _check_private_reporting {
    my ( $cid, $v ) = @_;
    my $raw       = _bytes_of( $v->{raw_payload}, 'raw_payload' );
    my $target    = _text_of( $v->{target}, 'target' );
    my $fid       = _text_of( $v->{failure_id}, 'failure_id' );
    my $sig       = _text_of( $v->{sig_hash}, 'sig_hash' );
    my $tap       = _text_of( $v->{tap_line}, 'tap_line' );
    my $want_leak = _bool_of( $v->{contains_raw_payload}, 'contains_raw_payload' );
    my ( $ok, $line );
    {
        local $@;
        $ok = eval { $line = $HK{format_tap}->( 1, $target, $fid, $sig ); 1 };
        $line = $@ if !$ok;
    }
    _fail( 'format_tap_failure refused: ' . CodingAdventures::TestBootstrap::Values::error_message($line) ) if !$ok;
    my @problems;
    push @problems, sprintf( '%s: tap_line expected %s, got %s', $cid, _clip( CodingAdventures::TestBootstrap::Canon::escape($tap) ), _clip( CodingAdventures::TestBootstrap::Canon::escape($line) ) ) if $line ne $tap;
    my $leaked = ( $raw ne '' && index( $line, _hex($raw) ) >= 0 ) ? 1 : 0;
    push @problems, sprintf( '%s: privacy verdict wrong, payload leaked: %s', $cid, $leaked ) if $leaked != $want_leak;
    push @problems, "$cid: failure line spans more than one line" if $line =~ /[\r\n]/;
    return \@problems;
}

# Decodes and checks one case.  Any error (a malformed case or an engine that died) is
# reported as a problem.  Returns (\@found, $accepted).
sub _evaluate_case {
    my ( $cid, $op, $raw, $work ) = @_;
    my %v;
    for my $k ( @{ $raw->{keys} } ) {
        $v{$k} = CodingAdventures::TestBootstrap::Values::decode( $raw->{map}{$k} ) if $k ne 'id' && $k ne 'op';
    }
    my ( $found, $accepted ) = ( [], 0 );
    if    ( $op eq 'corpus_admit' )      { $found = _check_corpus_admit( $cid, \%v ) }
    elsif ( $op eq 'crash_dedup' )       { $found = _check_crash_dedup( $cid, \%v ) }
    elsif ( $op eq 'private_reporting' ) { $found = _check_private_reporting( $cid, \%v ) }
    elsif ( $op eq 'failure_id_valid' ) {
        my $input = CodingAdventures::TestBootstrap::Canon::is_text( $v{input} ) ? CodingAdventures::TestBootstrap::Canon::text_value( $v{input} ) : undef;
        $accepted = ( defined $input && $HK{failure_id}->($input) ) ? 1 : 0;
        push @$found, sprintf( '%s: failure id membership disagrees for %s', $cid, _show( $v{input} ) )
          if !CodingAdventures::TestBootstrap::Values::same( CodingAdventures::TestBootstrap::Values::bool($accepted), $v{valid} );
    }
    elsif ( $op eq 'corpus_reject' ) {
        my $text = _text_of( $v{text}, 'text' );
        push @$found, "$cid: corpus_reject text was accepted by the strict parser" if $HK{tfv_valid}->($text);
    }
    else { $found = _check_mutator( $cid, $op, \%v, $work ) }
    return ( $found, $accepted );
}

sub _safe_evaluate_case {
    my ( $cid, $op, $raw, $work ) = @_;
    my ( $ok, $found, $accepted, $err );
    {
        local $@;
        local $SIG{__DIE__} = 'DEFAULT';
        $ok = eval { ( $found, $accepted ) = _evaluate_case( $cid, $op, $raw, $work ); 1 };
        $err = $@ if !$ok;
    }
    return ( $found, 1, $accepted ) if $ok;
    return ( [ sprintf( '%s: %s could not be evaluated: %s', $cid, $op, CodingAdventures::TestBootstrap::Values::error_message($err) ) ], 0, 0 );
}

# Evaluates every case; returns `{ evaluated => n, problems => [...] }`.  `$cases_in` is the
# decoded case list (an array of JSON nodes).  Pass 0 as `$expected` to skip the case-count
# check (ad-hoc case lists in tests); an undef `$expected` means 25.
sub evaluate {
    my ( $cases_in, $expected ) = @_;
    $expected = FUZZV1_CASE_COUNT if !defined $expected;
    %HK = %CodingAdventures::TestBootstrap::Fuzz::Hooks::H;
    my @problems;
    my $evaluated = 0;
    my $cases     = $cases_in;
    push @problems, sprintf( 'fuzz-v1: expected %d cases, got %d', $expected, scalar @$cases ) if $expected && @$cases != $expected;
    if ( @$cases > MAX_CASES ) {
        push @problems, sprintf( 'fuzz-v1: %d cases exceeds the cap of %d', scalar @$cases, MAX_CASES );
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

    # the aggregate mutate_sequence work of one corpus, in bytes ACTUALLY copied
    my $work = $MAX_CORPUS_SEQUENCE_WORK;
    my ( %ops_seen, $accepted_seen, $rejected_seen );

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
                    my ( $found, $ran_ok, $accepted ) = _safe_evaluate_case( $cid, $op, $raw, \$work );
                    push @problems, @$found;
                    if ($ran_ok) {
                        if ( $op eq 'failure_id_valid' ) {
                            if ($accepted) { $accepted_seen = 1 }
                            else           { $rejected_seen = 1 }
                        }
                        $ops_seen{$op} = 1;
                        $evaluated++;
                    }
                }
            }
        }
    }

    my @missing_ops = grep { !$ops_seen{$_} } sort keys %SHAPES;
    push @problems, 'fuzz-v1: operations never exercised: ' . join( ',', @missing_ops ) if @missing_ops;
    push @problems, 'fuzz-v1: failure_id_valid needs a registered-id acceptance' if !$accepted_seen;
    push @problems, 'fuzz-v1: failure_id_valid needs an unregistered-id rejection' if !$rejected_seen;
    my @unregistered = grep { !$HK{failure_id}->($_) } @CodingAdventures::TestBootstrap::Fuzz::FailureIds::ALL;
    push @problems, 'fuzz-v1: engine rejects registered ids ' . join( ',', @unregistered ) if @unregistered;

    my @out;
    for my $p (@problems) {
        last if @out >= MAX_PROBLEMS;
        push @out, CodingAdventures::TestBootstrap::Canon::clip( $p, CodingAdventures::TestBootstrap::Values::MAX_PROBLEM_LEN() );
    }
    return { evaluated => $evaluated, problems => \@out };
}

# Parses `$data` strictly and evaluates it.  Returns the report; a document that fails to
# parse is one problem.
sub evaluate_document {
    my ($data) = @_;
    my ( $cases, $err ) = parse($data);
    return { evaluated => 0, problems => ["fuzz-v1: $err"] } if !defined $cases;
    return evaluate($cases);
}

1;
