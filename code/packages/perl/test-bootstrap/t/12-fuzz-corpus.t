# The shared fuzz-v1 corpus, replayed through this package's own engine, with per-operation
# negative controls and sabotage hooks (VER06q sections 8 and 9).
use strict;
use warnings;
use Test::More;
use Storable ();
use FindBin;
use lib "$FindBin::Bin/lib";
use TBHelper qw(dec int_of repo_root read_file bounded_by);
use CodingAdventures::TestBootstrap;

my $Corpus = 'CodingAdventures::TestBootstrap::Fuzz::Corpus';
my $Hooks  = 'CodingAdventures::TestBootstrap::Fuzz::Hooks';
my $Int    = 'CodingAdventures::TestBootstrap::Int';

my $root = repo_root();
my $DATA = read_file("$root/code/specs/fixtures/fuzz-v1/cases.json");
my ( $CASES, $perr ) = CodingAdventures::TestBootstrap::Fuzz::Corpus::parse($DATA);
die "cannot parse the committed corpus: $perr" if !$CASES;

sub fresh    { return Storable::dclone($CASES) }
sub evaluate { my ( $cases, $expected ) = @_; return bounded_by( 120, sub { CodingAdventures::TestBootstrap::Fuzz::Corpus::evaluate( $cases, $expected ) } ) }
sub with_hook { my ( $h, $code ) = @_; return CodingAdventures::TestBootstrap::Fuzz::Hooks::with( $h, $code ) }
sub jint { return { kind => 'object', keys => [ 'kind', 'value' ], map => { kind => { kind => 'string', value => 'int' }, value => { kind => 'string', value => $_[0] } } } }
sub jbytes { return { kind => 'object', keys => [ 'kind', 'hex' ], map => { kind => { kind => 'string', value => 'bytes' }, hex => { kind => 'string', value => $_[0] } } } }

# ---- corruptions ---------------------------------------------------------

sub bump_int {
    my ($node) = @_;
    my $v = $node->{map}{value};
    $v->{value} = dec( CodingAdventures::TestBootstrap::Int::add( int_of( $v->{value} ), 1 ) );
}

sub corrupt_bytes {
    my ($node) = @_;
    my $hex = $node->{map}{hex}{value};
    $node->{map}{hex}{value} = $hex eq '' ? '00' : ( substr( $hex, 0, 1 ) eq '0' ? '1' : '0' ) . substr( $hex, 1 );
}

sub flip_bool { $_[0]{value} = $_[0]{value} ? 0 : 1 }

my @MUTATORS = qw(mutate_bit_flip mutate_byte_flip mutate_arithmetic mutate_boundary mutate_block_delete mutate_block_insert mutate_block_duplicate mutate_splice mutate_sequence);
my %CORRUPTIONS = (
    corpus_admit      => [ [ 'wrong new_edges', sub { bump_int( $_[0]{new_edges} ) } ], [ 'flipped admitted', sub { flip_bool( $_[0]{admitted} ) } ] ],
    crash_dedup       => [ [ 'flipped is_duplicate', sub { flip_bool( $_[0]{is_duplicate} ) } ] ],
    failure_id_valid  => [ [ 'flipped validity', sub { flip_bool( $_[0]{valid} ) } ] ],
    private_reporting => [
        [ 'a tap line changed by a character', sub { $_[0]{tap_line}{value} =~ s/ - / = / } ],
        [ 'a wrong hash in the line', sub { $_[0]{tap_line}{value} =~ s/:[0-9a-f]{8}\]/:00000000]/ } ],
        [ 'flipped privacy verdict', sub { flip_bool( $_[0]{contains_raw_payload} ) } ],
    ],
    corpus_reject => [ [ 'a valid TFV in place of a reject text', sub { $_[0]{text}{value} = "tfv 1\nok R1 Ka T\n" } ] ],
);
$CORRUPTIONS{$_} = [ [ 'wrong output', sub { corrupt_bytes( $_[0]{output} ) } ] ] for @MUTATORS;

subtest 'the committed fuzz-v1 corpus' => sub {
    is( scalar @$CASES, 25, 'exactly 25 cases' );
    my $report = evaluate( fresh() );
    is_deeply( $report->{problems}, [], 'zero problems' );
    is( $report->{evaluated}, 25, 'all 25 evaluated' );
    my $doc = bounded_by( 120, sub { CodingAdventures::TestBootstrap::Fuzz::Corpus::evaluate_document($DATA) } );
    is( $doc->{evaluated}, 25, 'straight from the document text: 25' );
    is( scalar @{ $doc->{problems} }, 0, '... with no problems' );
    ok( CodingAdventures::TestBootstrap::Tfv::valid( read_file("$root/code/specs/fixtures/fuzz-v1/cases.tfv") ), 'the TFV twin is valid' );
    diag('fuzz-v1: ' . $report->{evaluated} . '/' . scalar(@$CASES) . ' cases');
    my %ops;
    $ops{ $_->{map}{op}{value} }++ for @$CASES;
    ok( $ops{$_}, "a case for op $_" ) for sort keys %CORRUPTIONS;
    ok( $CORRUPTIONS{$_}, "a corruption defined for op $_" ) for sort keys %ops;
};

subtest 'per-operation negative controls: every corruption is detected, with the case id' => sub {
    my ( $detected, $total ) = ( 0, 0 );
    for my $i ( 0 .. $#$CASES ) {
        my $id = $CASES->[$i]{map}{id}{value};
        my $op = $CASES->[$i]{map}{op}{value};
        for my $c ( @{ $CORRUPTIONS{$op} } ) {
            my $cases = fresh();
            $c->[1]->( $cases->[$i]{map} );
            my $report = evaluate($cases);
            $total++;
            my $n = @{ $report->{problems} };
            my $bad_prefix = grep { index( $_, "$id:" ) != 0 } @{ $report->{problems} };
            if ( $n > 0 && !$bad_prefix ) { $detected++ }
            else { diag "NOT DETECTED / wrong prefix: $id ($c->[0]): $n problems, $bad_prefix with a foreign prefix" }
        }
    }
    is( $detected, $total, "all $total corruptions (every case, every op-defined corruption) were detected" );
    cmp_ok( $total, '>=', 29, 'and there were many of them' );
};

subtest 'structural negative controls' => sub {
    my $cases = fresh();
    pop @$cases;
    my $r = evaluate($cases);
    ok( ( grep { /expected 25 cases, got 24/ } @{ $r->{problems} } ), 'a dropped case is a problem' );
    $cases = fresh();
    splice @$cases, 3, 1;
    ok( scalar @{ evaluate($cases)->{problems} }, 'a case dropped from the middle is a problem' );
    $cases = fresh();
    push @$cases, Storable::dclone( $cases->[0] );
    $r = evaluate($cases);
    ok( ( grep { /expected 25 cases, got 26/ } @{ $r->{problems} } ) && ( grep { /duplicate or missing case id/ } @{ $r->{problems} } ), 'a duplicated case: count and id' );
    $r = evaluate( [ @{ fresh() }[ 0 .. 4 ] ] );
    ok( ( grep { /expected 25/ } @{ $r->{problems} } ) && ( grep { /never exercised/ } @{ $r->{problems} } ), 'a truncated corpus: wrong count and missing ops' );
    $cases = fresh();
    $cases->[0]{map}{op}{value} = 'mystery_op';
    $r = evaluate($cases);
    ok( ( grep { /^fuzz\/mutate_bit_flip\/bit0: unknown op/ } @{ $r->{problems} } ), 'an unknown op is a problem naming the case' );
    $cases = fresh();
    $cases->[1]{map}{id}{value} = $cases->[0]{map}{id}{value};
    ok( ( grep { /duplicate or missing case id/ } @{ evaluate($cases)->{problems} } ), 'a duplicate id' );
    $cases = fresh();
    push @{ $cases->[0]{keys} }, 'extra';
    $cases->[0]{map}{extra} = { kind => 'bool', value => 1 };
    ok( ( grep { /^fuzz\/mutate_bit_flip\/bit0: fields do not match/ } @{ evaluate($cases)->{problems} } ), 'an extra field' );
    $cases = fresh();
    pop @{ $cases->[0]{keys} };
    ok( ( grep { /fields do not match/ } @{ evaluate($cases)->{problems} } ), 'a missing field' );
    $cases = fresh();
    $cases->[0]{map}{input} = { kind => 'string', value => 'x' };
    ok( ( grep { /^fuzz\/mutate_bit_flip\/bit0: .*could not be evaluated/ } @{ evaluate($cases)->{problems} } ), 'a malformed value' );
    $cases = fresh();
    $cases->[0]{map}{bit_offset} = jbytes('00');
    ok( ( grep { /could not be evaluated/ } @{ evaluate($cases)->{problems} } ), 'bytes where an int belongs' );
    $r = evaluate( [ 5, { kind => 'object', keys => [], map => {} } ], 0 );
    ok( ( grep { /case is not an object/ } @{ $r->{problems} } ) && ( grep { /missing a string id|duplicate or missing/ } @{ $r->{problems} } ), 'non-object and id-less cases' );
    # a case id with a newline is escaped in the problem
    $cases = fresh();
    $cases->[0]{map}{id}{value} = "evil\nnot ok 1 - forged";
    $cases->[0]{map}{output} = jbytes('ff');
    my @p = @{ evaluate($cases)->{problems} };
    ok( !( grep { /\n/ } @p ) && ( grep { /^evil\\nnot ok 1 - forged: / } @p ), 'a hostile id cannot forge a line' );
    # the document level
    ok( ( grep { /^fuzz-v1: / } @{ CodingAdventures::TestBootstrap::Fuzz::Corpus::evaluate_document('{"schema_version": 2, "cases": []}')->{problems} } ), 'a wrong schema version' );
    ok( ( grep { /^fuzz-v1: / } @{ CodingAdventures::TestBootstrap::Fuzz::Corpus::evaluate_document('not json')->{problems} } ), 'not JSON' );
    ok( ( grep { /^fuzz-v1: / } @{ CodingAdventures::TestBootstrap::Fuzz::Corpus::evaluate_document('{"schema_version": 1, "cases": {}}')->{problems} } ), 'cases must be an array' );
    ok( ( grep { /^fuzz-v1: / } @{ CodingAdventures::TestBootstrap::Fuzz::Corpus::evaluate_document('{"schema_version": 1}')->{problems} } ), 'a missing cases key' );
    my ( $none, $why ) = CodingAdventures::TestBootstrap::Fuzz::Corpus::parse('{"schema_version": 1, "cases": [], "x": 1}');
    ok( !defined $none && $why, 'an extra top-level key' );
};

subtest 'fixture-controlled sizes are capped' => sub {
    my ($si) = grep { $CASES->[$_]{map}{op}{value} eq 'mutate_sequence' } 0 .. $#$CASES;
    my $cases = fresh();
    $cases->[$si]{map}{steps} = jint('1000000000');
    my @p = @{ evaluate($cases)->{problems} };
    ok( ( grep { /^\Q$CASES->[$si]{map}{id}{value}\E: .*steps outside/ } @p ), 'steps = 10^9 is a problem, not a hang' );
    $cases = fresh();
    $cases->[$si]{map}{seed} = jint('1' . '0' x 1001);
    ok( ( grep { /could not be evaluated|exceeds/ } @{ evaluate($cases)->{problems} } ), 'a 1,001-digit integer is refused' );
    # The aggregate work budget (2^24 bytes) is charged what each call REPORTS it copied, and a
    # breach zeroes the allowance.  Hooks stand in for an engine that copies a lot.
    my @many;
    for my $n ( 1 .. 4 ) {
        my $c = Storable::dclone( $CASES->[$si] );
        $c->{map}{id}{value} = "agg$n";
        $c->{map}{output} = jbytes('00');
        push @many, $c;
    }
    my @seen;
    my $r = with_hook( { mutate_sequence => sub { push @seen, $_[4]; return ( "\0", 9_000_000 ) } }, sub { evaluate( \@many, 0 ) } );
    is_deeply( \@seen, [ 16_777_216, 7_777_216 ], 'each call is handed what is left; the third is not run at all (nothing left)' );
    my @exhausted = grep { /^agg[34]: .*work budget of 16777216 byte-steps exhausted/ } @{ $r->{problems} };
    is( scalar @exhausted, 2, 'cases 3 and 4 fail at once, naming themselves' );
    ok( !( grep { /^agg[12]: / } @{ $r->{problems} } ), 'cases 1 and 2 ran' );
    @seen = ();
    $r = with_hook( { mutate_sequence => sub { push @seen, $_[4]; return ( undef, 123 ) } }, sub { evaluate( \@many, 0 ) } );
    is( scalar @seen, 1, 'a breach zeroes the allowance: only the first case ran the engine' );
    is( scalar( grep { /^agg[1-4]: .*exhausted/ } @{ $r->{problems} } ), 4, '... and every case reports it' );
};

subtest 'sabotage hooks: every broken entry point makes the run fail' => sub {
    my %broken = (
        bit_flip        => sub { $_[0] },
        byte_flip       => sub { $_[0] },
        byte_arithmetic => sub { $_[0] },
        boundary_insert => sub { $_[0] },
        block_delete    => sub { $_[0] },
        block_insert    => sub { $_[0] },
        block_duplicate => sub { $_[0] },
        splice          => sub { $_[0] },
        mutate_sequence => sub { return ( 'x', 1 ) },
        corpus_admit    => sub { return ( 1, 99 ) },
        duplicate_crash => sub { 1 },
        failure_id      => sub { 1 },
        format_tap      => sub { 'not ok 1 - x [fuzz-crash:00000000]' },
        tfv_valid       => sub { 1 },
    );
    my %known = $Hooks->can('defaults')->();
    is_deeply( [ sort keys %broken ], [ sort keys %known ], 'a sabotage for every hook' );
    for my $name ( sort keys %broken ) {
        my $r = with_hook( { $name => $broken{$name} }, sub { evaluate( fresh() ) } );
        ok( scalar @{ $r->{problems} }, "a broken $name is detected" );
    }
    is_deeply( evaluate( fresh() )->{problems}, [], 'the hooks were restored' );
    ok( !eval { with_hook( { nonsense => sub { 1 } }, sub { 1 } ); 1 }, 'an unknown hook is refused' );
    my $kept = eval { with_hook( { bit_flip => sub { die "boom\n" } }, sub { die "inner\n" } ); 1 };
    is( $@, "inner\n", 'with rethrows' );
    is_deeply( evaluate( fresh() )->{problems}, [], '... after restoring the hooks' );
    # the run copies the hooks when it starts: a mid-run change cannot alter it
    my $mid = with_hook( { corpus_admit => sub { %CodingAdventures::TestBootstrap::Fuzz::Hooks::H = CodingAdventures::TestBootstrap::Fuzz::Hooks::defaults(); return ( 1, 99 ) } }, sub { evaluate( fresh() ) } );
    ok( scalar @{ $mid->{problems} }, 'resetting the hooks mid-run does not heal a broken run' );
    CodingAdventures::TestBootstrap::Fuzz::Hooks::reset();
    is_deeply( evaluate( fresh() )->{problems}, [], 'reset restores the defaults' );
    my $sf = with_hook( { format_tap => sub { die "refused\n" } }, sub { evaluate( fresh() ) } );
    ok( ( grep { /private_reporting|opaque_token/ && /refused|could not be evaluated/ } @{ $sf->{problems} } ), 'a refusing format_tap is a problem naming the case' );
};

done_testing;
