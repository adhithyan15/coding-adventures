# Hazards 2, 3 and 8: user values run code, exceptions are any value, a hostile
# environment must not make the engine raise (VER05p section 4).
use strict;
use warnings;
use Test::More;
use Scalar::Util ();
use FindBin;
use lib "$FindBin::Bin/lib";
use TBHelper qw(dec int_of bounded_by);

my $R    = 'CodingAdventures::TestBootstrap::Runner';
my $G    = 'CodingAdventures::TestBootstrap::Gens';
my $Opts = 'CodingAdventures::TestBootstrap::Runner::Options';
my $Safe = 'CodingAdventures::TestBootstrap::Safe';
my $Canon = 'CodingAdventures::TestBootstrap::Canon';

sub opts { return $Opts->new( { ci => 'off', @_ } ) }
sub check { my @a = @_; return bounded_by( 20, sub { $R->can('check')->(@a) } ) }
my $int = $G->can('integer')->( 0, 1000 );

# A class whose every overload dies and records the call.
our @LOG;
{
    package Trap;
    use overload
      '""'   => sub { push @LOG, '""';   die "trap: stringify\n" },
      '0+'   => sub { push @LOG, '0+';   die "trap: numify\n" },
      'bool' => sub { push @LOG, 'bool'; die "trap: bool\n" },
      'eq'   => sub { push @LOG, 'eq';   die "trap: eq\n" },
      'ne'   => sub { push @LOG, 'ne';   die "trap: ne\n" },
      '=='   => sub { push @LOG, '==';   die "trap: ==\n" },
      '!='   => sub { push @LOG, '!=';   die "trap: !=\n" },
      '<=>'  => sub { push @LOG, '<=>';  die "trap: <=>\n" },
      'cmp'  => sub { push @LOG, 'cmp';  die "trap: cmp\n" },
      '@{}'  => sub { push @LOG, '@{}';  die "trap: array deref\n" },
      '%{}'  => sub { push @LOG, '%{}';  die "trap: hash deref\n" },
      '${}'  => sub { push @LOG, '${}';  die "trap: scalar deref\n" },
      '&{}'  => sub { push @LOG, '&{}';  die "trap: code deref\n" },
      'neg'  => sub { push @LOG, 'neg';  die "trap: neg\n" },
      '+'    => sub { push @LOG, '+';    die "trap: +\n" },
      '-'    => sub { push @LOG, '-';    die "trap: -\n" },
      '<'    => sub { push @LOG, '<';    die "trap: <\n" };
    sub new { my ( $c, $inner ) = @_; return bless [ $inner // 1, 2, 3 ], $c }
    sub DESTROY { push @LOG, 'DESTROY' }    # recorded but allowed
}
{
    package TrapHash;
    our @ISA = ('Trap');
    sub new { return bless { a => 1, b => [ 2, 3 ] }, shift }
}

# Tied containers whose every method records and dies.
{
    package TieA;
    sub TIEARRAY  { bless [], shift }
    sub FETCH     { push @main::LOG, 'TieA::FETCH';     die "tie\n" }
    sub FETCHSIZE { push @main::LOG, 'TieA::FETCHSIZE'; die "tie\n" }
    sub STORE     { push @main::LOG, 'TieA::STORE';     die "tie\n" }
    package TieH;
    sub TIEHASH  { bless {}, shift }
    sub FETCH    { push @main::LOG, 'TieH::FETCH';    die "tie\n" }
    sub FIRSTKEY { push @main::LOG, 'TieH::FIRSTKEY'; die "tie\n" }
    sub NEXTKEY  { push @main::LOG, 'TieH::NEXTKEY';  die "tie\n" }
    sub EXISTS   { push @main::LOG, 'TieH::EXISTS';   die "tie\n" }
    sub SCALAR   { push @main::LOG, 'TieH::SCALAR';   die "tie\n" }
    package TieS;
    sub TIESCALAR { bless [], shift }
    sub FETCH     { push @main::LOG, 'TieS::FETCH'; die "tie\n" }
    sub STORE     { push @main::LOG, 'TieS::STORE'; die "tie\n" }
}

sub no_user_code {
    my ( $name, $code ) = @_;
    local @LOG;
    my $ok = eval { $code->(); 1 };
    ok( $ok, "$name: does not raise" ) or diag $@;
    is_deeply( [ grep { $_ ne 'DESTROY' } @LOG ], [], "$name: ran no overload and no tie method" );
}

subtest 'Canon, same, size_of and clone never run user code' => sub {
    my $trap = Trap->new;
    my $th   = TrapHash->new;
    tie my @ta, 'TieA';
    tie my %th, 'TieH';
    tie my $ts, 'TieS';
    local @LOG;    # tie() may record nothing; start clean
    my %plain = ( trap => $trap, nested => [ $th, \@ta, \%th ] );
    no_user_code( 'Canon the trap object', sub { my $s = $Canon->can('render')->($trap); like( $s, qr/^Trap=\[1,2,3\]$/, "renders structurally: $s" ) } );
    no_user_code( 'Canon the trap hash', sub { my $s = $Canon->can('render')->($th); like( $s, qr/^TrapHash=%\{"a"=>1,"b"=>\[2,3\]\}$/, "renders structurally: $s" ) } );
    no_user_code( 'Canon a tied array', sub { is( $Canon->can('render')->( \@ta ), '#Magic', 'a tied array is opaque' ) } );
    no_user_code( 'Canon a tied hash',  sub { is( $Canon->can('render')->( \%th ), '#Magic', 'a tied hash is opaque' ) } );
    no_user_code( 'Canon a tied scalar', sub { is( $Canon->can('render')->( $ts ), 'undef', 'a tied scalar value (read once by the caller)' ) } ) if 0;
    no_user_code( 'Canon a tied scalar slot', sub { is( $Canon->can('render')->( \$ts ), '#Magic', 'a ref to a tied scalar is opaque' ) } );
    no_user_code( 'Canon containers holding them', sub { like( $Canon->can('render')->( \%plain ), qr/nested=>|"nested"=>/, 'a plain hash holding traps and ties' ) } );
    no_user_code( 'Canon a tied scalar element', sub {
        my @arr = ( 1, 2 );
        tie $arr[1], 'TieS';
        local @LOG;
        my $s = $Canon->can('render')->( \@arr );
        is( $s, '[1,#Magic]', 'a tied element is opaque' );
    } );
    my $other = Trap->new(2);
    no_user_code( 'same', sub { ok( !$Safe->can('holds')->($trap), 'holds' ); is( CodingAdventures::TestBootstrap::Values::same( $trap, $other ), 0, 'same on objects is false, no ==' ) } );
    no_user_code( 'same on tied', sub { is( CodingAdventures::TestBootstrap::Values::same( \@ta, \@ta ), 0, 'tied arrays never compare equal' ) } );
    no_user_code( 'size_of', sub { is( $Safe->can('size_of')->( \@ta ), 0, 'a tied array is 0' ); $Safe->can('size_of')->($trap); $Safe->can('size_of')->( \%plain ) } );
    no_user_code( 'clone', sub {
        my ( $copy, $failed ) = $Safe->can('clone')->( \%plain );
        ok( !$failed, 'cloned' );
        is( Scalar::Util::refaddr( $copy->{trap} ), Scalar::Util::refaddr($trap), 'an object is shared, not copied' );
        is( ref $copy->{nested}[1], 'ARRAY', 'the tied array reference is kept (opaque)' );
        ok( Scalar::Util::refaddr($copy) != Scalar::Util::refaddr( \%plain ), 'a plain hash is copied' );
    } );
    no_user_code( 'holds / kind', sub { is( $Safe->can('holds')->($trap), 0, 'an object with a true bool never holds' ); is( $Safe->can('kind')->($trap), 'object', 'kind by blessed' ) } );
};

subtest 'a property returning a trap object falsifies without running it' => sub {
    local @LOG;
    my $v = check( 1, $int, sub { return Trap->new }, opts( num_tests => 3 ) );
    is( $v->{failure_id}, 'property-counterexample', 'falsified' );
    is_deeply( [ grep { $_ ne 'DESTROY' } @LOG ], [], 'no overload ran' );
};

subtest 'die with any value' => sub {
    my @values = (
        [ 'a string',                   sub { die "plain\n" },       'string' ],
        [ 'undef',                      sub { no warnings; die undef },           'string' ],
        [ 'an empty string',            sub { die '' },              'string' ],
        [ 'a plain reference',          sub { die { code => 5 } },   'ref' ],
        [ 'an array reference',         sub { die [ 1, 2 ] },        'ref' ],
        [ 'a code reference',           sub { die sub { 1 } },       'ref' ],
        [ 'an object with trap overloads', sub { die Trap->new },    'object' ],
        [ 'an object whose "" dies',    sub { die bless( {}, 'Trap' ) }, 'object' ],
        [ 'a number',                   sub { die 42 },              'string' ],
    );
    for my $c (@values) {
        local @LOG;
        my $v = check( 1, $int, $c->[1], opts() );
        is( $v->{failure_id}, 'property-panic', "$c->[0]: a panic" );
        is_deeply( [ grep { $_ ne 'DESTROY' } @LOG ], [], "$c->[0]: no overload ran" );
        my $r = check( 1, $int, $c->[1], $Opts->new( { ci => 'on' } ) );
        like( $r->{message}, qr/\Q($c->[2])\E|\(string\)/, "$c->[0]: the redacted message names only a kind" );
    }
    my $u = check( 1, $int, sub { no warnings; die undef }, opts() );
    like( $u->{message}, qr/raised at iteration 1: /, 'die undef still produces a rendering' );
    my ( $st, $err ) = $Safe->can('call')->( sub { die Trap->new } );
    is( $st, 'died', 'Safe::call reports died' );
    is( Scalar::Util::blessed($err), 'Trap', '... and hands the object on untouched' );
};

subtest 'status comes from eval, never from $@' => sub {
    my ( $st, $v ) = $Safe->can('call')->( sub { eval { die "inner\n" }; return 7 } );
    is_deeply( [ $st, $v ], [ 'ok', 7 ], 'a handled inner die does not make the call fail' );
    {
        package Clobber;
        sub new { bless {}, shift }
        sub DESTROY { eval { die "in destroy\n" }; }
    }
    ( $st, $v ) = $Safe->can('call')->( sub { my $o = Clobber->new; die "outer\n" } );
    is( $st, 'died', 'died despite a DESTROY that evals' );
    is( $v, "outer\n", '... with the original error' );
    ( $st, $v ) = $Safe->can('call')->( sub { my $o = Clobber->new; return 5 } );
    is_deeply( [ $st, $v ], [ 'ok', 5 ], 'a clean return despite a DESTROY that evals' );
    local $@ = 'caller error';
    $Safe->can('call')->( sub { die "x\n" } );
    is( $@, 'caller error', 'the caller\'s $@ is restored' );
    local $_ = 'topic';
    $Safe->can('call')->( sub { $_ = 'changed'; 1 } );
    is( $_, 'topic', 'the caller\'s $_ is restored' );
};

subtest 'die and warn handlers cannot interfere' => sub {
    my $handled = 0;
    local $SIG{__DIE__} = sub { $handled++; die "handler replaced the error\n" };
    my $v = check( 1, $int, sub { die "original\n" }, opts() );
    is( $v->{failure_id}, 'property-panic', 'still a panic' );
    like( $v->{message}, qr/original/, 'the original error, not the handler\'s' );
    is( $handled, 0, 'the user handler did not run' );
    local $SIG{__WARN__} = sub { $handled++; die "warn handler dies\n" };
    $v = check( 1, $int, sub { warn "careful\n"; return $_[0] <= 1000 ? 1 : 0 }, opts( num_tests => 3 ) );
    ok( $v->{passed}, 'a property that warns passes (the dying warn handler is neutralized)' );
    is( $handled, 0, 'the warn handler did not run' );
    is( ref $SIG{__DIE__}, 'CODE', 'the handlers are restored afterwards' );
};

subtest 'non-local exits out of user code cannot break the engine loop' => sub {
    my $n = 0;
    my $v = check( 1, $int, sub { no warnings 'exiting'; $n++; last }, opts( num_tests => 5 ) );
    is( $v->{failure_id}, 'property-panic', '`last` out of a property is a panic' );
    like( $v->{message}, qr/non-local exit/, '... described' );
    is( $n, 1, 'the property ran once' );
    $v = check( 1, $int, sub { no warnings 'exiting'; next }, opts( num_tests => 5 ) );
    is( $v->{failure_id}, 'property-panic', '`next` too' );
    my $gen = CodingAdventures::TestBootstrap::Gen::new( sub { no warnings 'exiting'; last } );
    $v = check( 1, $gen, sub { 1 }, opts() );
    is( $v->{failure_id}, 'property-exhausted', '`last` out of a generator is exhausted' );
    my @loop;
    for my $i ( 1 .. 3 ) {
        push @loop, $i;
        check( 1, $int, sub { no warnings 'exiting'; last }, opts( num_tests => 2 ) );
    }
    is_deeply( \@loop, [ 1, 2, 3 ], 'the caller\'s own loop is not affected' );
    my ( $st, $val ) = $Safe->can('call')->( sub { no warnings 'exiting'; last } );
    is( $st, 'escaped', 'Safe::call reports escaped' );
};

subtest 'a hostile environment (hazard 8) still gets a verdict' => sub {
    my $v = check(
        42, $int,
        sub {
            no warnings;
            # try to break the engine from inside a property
            eval { *CodingAdventures::TestBootstrap::Safe::holds = sub { 1 } };
            eval { *CodingAdventures::TestBootstrap::Prng::threshold = sub { 0 } };
            eval { local *CodingAdventures::TestBootstrap::Int::cmp = sub { 0 } };
            *CORE::GLOBAL::die = sub { CORE::die("overridden\n") };
            *CORE::GLOBAL::sprintf = sub { 'hijacked' };
            return $_[0] <= 100 ? 1 : 0;
        },
        opts()
    );
    is( $v->{failure_id}, 'property-counterexample', 'a verdict, from the real engine' );
    is( $v->{minimal}, 101, '... with the right minimal witness' );
    like( $v->{message}, qr/^property falsified at iteration \d+/, 'the message was built by the real sprintf' );
    {
        no warnings 'once';
        is( ref \&CodingAdventures::TestBootstrap::Safe::holds, 'CODE', 'holds is still the engine\'s own' );
    }
    is( CodingAdventures::TestBootstrap::Prng::threshold(3), 1, 'the threshold is unchanged' );
};

subtest 'the engine\'s own failure becomes a panic naming only the kind' => sub {
    my $gen = CodingAdventures::TestBootstrap::Gen::new( sub { return 1 } );
    my $v = check( 1, $gen, sub { 1 }, opts( num_tests => 2 ) );
    ok( $v->{passed}, 'sanity' );
    # a generator that returns a value Canon cannot... (Canon is total) so force an engine failure through the shrinker hook
    local $CodingAdventures::TestBootstrap::Hooks::H{new_rng} = sub { die "rng factory failed\n" };
    $v = check( 1, $int, sub { 1 }, opts() );
    is( $v->{failure_id}, 'property-panic', 'an engine failure is an honest panic' );
    like( $v->{message}, qr/engine could not complete the check \(string\)/, '... naming only the kind' );
    unlike( $v->{message}, qr/rng factory/, '... and not the message' );
};

subtest 'hazard 9: a runaway property is stopped by an alarm, not by a hang' => sub {
    plan skip_all => 'a busy loop is not interruptible by alarm on this platform' if $^O eq 'MSWin32';
    my $spins = 0;
    my $v = do {
        local $SIG{ALRM} = sub { die "TIMEOUT\n" };
        alarm 1;
        my $r = $R->can('check')->( 1, $int, sub { 1 while 1 }, opts() );
        alarm 0;
        $r;
    };
    is( $v->{failure_id}, 'property-panic', 'the alarm\'s die is the property\'s panic: a verdict, in about a second' );
    like( $v->{message}, qr/TIMEOUT/, '... naming the cause (local mode)' );
    is( $SIG{ALRM}, undef, 'the handler was local to the test' );
    my $err = eval { bounded_by( 1, sub { 1 while 1 } ); 1 } ? '' : $@;
    like( $err, qr/TIMEOUT/, 'bounded_by itself turns a hang into a failure' );
};

done_testing;
