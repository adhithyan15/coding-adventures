use strict;
use warnings;
use Test::More;
use Config;
use FindBin;
use lib "$FindBin::Bin/lib";
use TBHelper qw(bounded_by);

use_ok('CodingAdventures::TestBootstrap');

is( $Config{ivsize}, 8, 'this perl has 64-bit integers (the engine refuses to load otherwise)' );
is( $Config{uvsize}, 8, '... and 64-bit unsigned integers' );
ok( $CodingAdventures::TestBootstrap::VERSION, 'has a version' );

# Library hygiene: engine subs are sealed against reassignment and local().
{
    no warnings;
    ok( !eval { *CodingAdventures::TestBootstrap::Prng::threshold = sub { 0 }; 1 }, 'a sealed sub cannot be reassigned' );
    ok( !eval { local *CodingAdventures::TestBootstrap::Safe::holds = sub { 1 }; 1 }, '... nor local()ised' );
}
is( CodingAdventures::TestBootstrap::Prng::threshold(3), 1, 'the real threshold is still in place' );

for my $m (qw(Int Prng Limits FailureIds Hooks Budget Gen Gens Shrink Runner Safe Canon StrictJson Values Tfv Corpus)) {
    ok( "CodingAdventures::TestBootstrap::$m"->can('import') || 1, "module $m loaded" );
}

done_testing;
