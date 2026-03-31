use strict;
use warnings;
use Test2::V0;

ok( eval { require CodingAdventures::SqlExecutionEngine; 1 },                  'CodingAdventures::SqlExecutionEngine loads' );
ok( eval { require CodingAdventures::SqlExecutionEngine::InMemoryDataSource; 1 }, 'CodingAdventures::SqlExecutionEngine::InMemoryDataSource loads' );

ok( CodingAdventures::SqlExecutionEngine->can('execute'),     'SqlExecutionEngine->execute exists' );
ok( CodingAdventures::SqlExecutionEngine->can('execute_all'), 'SqlExecutionEngine->execute_all exists' );
ok( CodingAdventures::SqlExecutionEngine::InMemoryDataSource->can('new'),    'InMemoryDataSource->new exists' );
ok( CodingAdventures::SqlExecutionEngine::InMemoryDataSource->can('schema'), 'InMemoryDataSource->schema exists' );
ok( CodingAdventures::SqlExecutionEngine::InMemoryDataSource->can('scan'),   'InMemoryDataSource->scan exists' );

done_testing;
