# The shared pbt-v1 corpus, replayed through this package's own engine, with
# per-operation negative controls and sabotage hooks (VER05p section 7).
use strict;
use warnings;
use Test::More;
use Storable ();
use FindBin;
use lib "$FindBin::Bin/lib";
use TBHelper qw(dec int_of repo_root read_file bounded_by);

my $Corpus = 'CodingAdventures::TestBootstrap::Corpus';
my $Hooks  = 'CodingAdventures::TestBootstrap::Hooks';
my $Int    = 'CodingAdventures::TestBootstrap::Int';
my $Prng   = 'CodingAdventures::TestBootstrap::Prng';

my $root = repo_root();
my $DATA = read_file("$root/code/specs/fixtures/pbt-v1/cases.json");
my ( $CASES, $perr ) = $Corpus->can('parse')->($DATA);
die "cannot parse the committed corpus: $perr" if !$CASES;

sub fresh    { return Storable::dclone($CASES) }
sub evaluate { my ( $cases, $expected ) = @_; return bounded_by( 120, sub { $Corpus->can('evaluate')->( $cases, $expected ) } ) }

# ---- corruptions ---------------------------------------------------------

sub bump_int {
    my ($node) = @_;
    my $v = $node->{map}{value};
    $v->{value} = dec( $Int->can('add')->( int_of( $v->{value} ), 1 ) );
}

sub jint { return { kind => 'object', keys => [ 'kind', 'value' ], map => { kind => { kind => 'string', value => 'int' }, value => { kind => 'string', value => $_[0] } } } }

sub corrupt_value {
    my ($node) = @_;
    if ( $node->{kind} eq 'bool' ) { $node->{value} = $node->{value} ? 0 : 1; return $node }
    if ( $node->{kind} eq 'array' ) {
        if   ( !@{ $node->{items} } ) { push @{ $node->{items} }, jint('0') }
        else                          { corrupt_value( $node->{items}[0] ) }
        return $node;
    }
    my $k = $node->{map}{kind}{value};
    if ( $k eq 'int' ) { bump_int($node); return $node }
    if ( $k eq 'bytes' ) {
        my $hex = $node->{map}{hex}{value};
        $node->{map}{hex}{value} = $hex eq '' ? '00' : ( substr( $hex, 0, 1 ) eq '0' ? '1' : '0' ) . substr( $hex, 1 );
        return $node;
    }
    die "cannot corrupt";
}

my %CORRUPTIONS = (
    gen_primitive      => [ [ 'flipped output', sub { corrupt_value( $_[0]{outputs} ) } ] ],
    gen_bounded        => [ [ 'flipped output', sub { corrupt_value( $_[0]{outputs} ) } ] ],
    gen_bytes          => [ [ 'flipped output', sub { corrupt_value( $_[0]{outputs} ) } ] ],
    shrink_int         => [ [ 'wrong minimal', sub { bump_int( $_[0]{expected_minimal} ) } ], [ 'steps + 1', sub { bump_int( $_[0]{expected_steps} ) } ] ],
    shrink_refinement  => [ [ 'wrong minimal', sub { bump_int( $_[0]{expected_minimal} ) } ], [ 'steps + 1', sub { bump_int( $_[0]{expected_steps} ) } ] ],
    shrink_list        => [ [ 'wrong minimal', sub { corrupt_value( $_[0]{expected_minimal} ) } ], [ 'steps + 1', sub { bump_int( $_[0]{expected_steps} ) } ] ],
    shrink_bytes       => [ [ 'wrong minimal', sub { corrupt_value( $_[0]{expected_minimal} ) } ], [ 'steps + 1', sub { bump_int( $_[0]{expected_steps} ) } ] ],
    replay_consistency => [ [ 'wrong initial_fail', sub { bump_int( $_[0]{initial_fail} ) } ], [ 'wrong minimal', sub { bump_int( $_[0]{expected_minimal} ) } ] ],
    filter_exhausted   => [ [ 'wrong failure id', sub { $_[0]{failure_id}{value} = 'property-panic' } ], [ 'verdict pass', sub { $_[0]{verdict}{value} = 'pass' } ] ],
    failure_id_valid   => [ [ 'flipped validity', sub { $_[0]{valid}{value} = $_[0]{valid}{value} ? 0 : 1 } ] ],
    corpus_reject      => [ [ 'a valid TFV in place of a reject text', sub { $_[0]{text}{value} = "tfv 1\nok R1 Ka T\n" } ] ],
);

subtest 'the committed pbt-v1 corpus' => sub {
    is( scalar @$CASES, 25, 'exactly 25 cases' );
    my $report = evaluate( fresh() );
    is_deeply( $report->{problems}, [], 'zero problems' );
    is( $report->{evaluated}, 25, 'all 25 evaluated' );
    my $doc = bounded_by( 120, sub { $Corpus->can('evaluate_document')->($DATA) } );
    is( $doc->{evaluated}, 25, 'straight from the document text: 25' );
    is( scalar @{ $doc->{problems} }, 0, '... with no problems' );
    ok( CodingAdventures::TestBootstrap::Tfv::valid( read_file("$root/code/specs/fixtures/pbt-v1/cases.tfv") ), 'the TFV twin is valid' );
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
    cmp_ok( $total, '>=', 34, 'and there were many of them' );
};

subtest 'structural negative controls' => sub {
    my $cases = fresh();
    pop @$cases;
    my $r = evaluate($cases);
    ok( ( grep { /expected 25 cases, got 24/ } @{ $r->{problems} } ), 'a dropped case is a problem' );
    $cases = fresh();
    splice @$cases, 3, 1;
    $r = evaluate($cases);
    ok( scalar @{ $r->{problems} }, 'a case dropped from the middle is a problem' );
    $r = evaluate( [ @{ fresh() }[ 0 .. 4 ] ] );
    ok( ( grep { /expected 25/ } @{ $r->{problems} } ) && ( grep { /never exercised/ } @{ $r->{problems} } ), 'a truncated corpus: wrong count and missing ops' );
    $cases = fresh();
    $cases->[0]{map}{op}{value} = 'mystery_op';
    $r = evaluate($cases);
    ok( ( grep { /^pbt\/gen_primitive\/bool_seed0: unknown op/ } @{ $r->{problems} } ), 'an unknown op is a problem naming the case' );
    $cases = fresh();
    $cases->[1]{map}{id}{value} = $cases->[0]{map}{id}{value};
    ok( ( grep { /duplicate or missing case id/ } @{ evaluate($cases)->{problems} } ), 'a duplicate id' );
    $cases = fresh();
    push @{ $cases->[0]{keys} }, 'extra';
    $cases->[0]{map}{extra} = { kind => 'bool', value => 1 };
    ok( ( grep { /^pbt\/gen_primitive\/bool_seed0: fields do not match/ } @{ evaluate($cases)->{problems} } ), 'an extra field' );
    $cases = fresh();
    pop @{ $cases->[0]{keys} };
    ok( ( grep { /fields do not match/ } @{ evaluate($cases)->{problems} } ), 'a missing field' );
    $cases = fresh();
    $cases->[0]{map}{type}{value} = 'u7';
    ok( ( grep { /could not be evaluated/ } @{ evaluate($cases)->{problems} } ), 'an unknown primitive type is a problem' );
    my ($pi) = grep { $CASES->[$_]{map}{op}{value} eq 'shrink_int' } 0 .. $#$CASES;
    $cases = fresh();
    $cases->[$pi]{map}{property}{value} = 'nonsense';
    ok( ( grep { /unknown property/ } @{ evaluate($cases)->{problems} } ), 'an unknown property' );
    $cases = fresh();
    $cases->[$pi]{map}{initial} = { kind => 'string', value => 'x' };
    ok( ( grep { /could not be evaluated/ } @{ evaluate($cases)->{problems} } ), 'a malformed value' );
    $cases = fresh();
    my ($ci) = grep { $CASES->[$_]{map}{op}{value} eq 'gen_primitive' } 0 .. $#$CASES;
    $cases->[$ci]{map}{count} = jint('999999');
    ok( ( grep { /outside|exactly count|could not be evaluated/ } @{ evaluate($cases)->{problems} } ), 'an absurd count is refused before generating' );
    my @unreg = grep { $CASES->[$_]{map}{op}{value} eq 'failure_id_valid' && $CASES->[$_]{map}{input}{value} eq 'unregistered-id' } 0 .. $#$CASES;
    $cases = fresh();
    splice @$cases, $unreg[0], 1;
    ok( ( grep { /unregistered-id rejection/ } @{ evaluate($cases)->{problems} } ), 'dropping the unregistered-id case' );
    $cases = fresh();
    my ($ti) = grep { $CASES->[$_]{map}{op}{value} eq 'failure_id_valid' && $CASES->[$_]{map}{input}{value} eq 'property-timeout' } 0 .. $#$CASES;
    splice @$cases, $ti, 1;
    ok( ( grep { /registered failure ids missing: property-timeout/ } @{ evaluate($cases)->{problems} } ), 'dropping a registered id' );
    is( scalar @{ evaluate( fresh(), 0 )->{problems} }, 0, 'the case-count check can be turned off for ad-hoc lists' );
    my @broken = ( 'not json', '{}', '{"schema_version":2,"cases":[]}', '{"schema_version":1}', '{"schema_version":1,"cases":{}}', '{"schema_version":1,"cases":[],"x":1}', '[]' );
    for my $b (@broken) {
        my $rep = $Corpus->can('evaluate_document')->($b);
        ok( scalar @{ $rep->{problems} } && $rep->{evaluated} == 0, "a broken document is a problem: $b" );
    }
};

subtest 'every sabotage hook makes the corpus fail' => sub {
    my %sabotage = (
        'bisection'        => { bisect => sub { $Int->can('sub')->( $_[0], 1 ) } },
        'unit step'        => { unit => sub { $Int->can('sub')->( $_[0], 2 ) } },
        'skipped deletion' => { skip_deletion => 1 },
        'a shifted RNG'    => { new_rng => sub { my $r = $Prng->can('new')->( $_[0] ); $r->next_word; return $r } },
        'a wrong threshold' => { threshold => sub { 9223372036854775808 } },
    );
    for my $name ( sort keys %sabotage ) {
        my $report = $Hooks->can('with')->( $sabotage{$name}, sub { evaluate( fresh() ) } );
        ok( scalar @{ $report->{problems} } > 0, "$name: the corpus run fails (" . scalar( @{ $report->{problems} } ) . ' problems)' );
    }
    is( scalar @{ evaluate( fresh() )->{problems} }, 0, 'and the hooks were restored: the clean run passes again' );
};

done_testing;
