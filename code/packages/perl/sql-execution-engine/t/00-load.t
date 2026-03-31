use strict;
use warnings;
use Test2::V0;

use_ok('CodingAdventures::SqlExecutionEngine');
use_ok('CodingAdventures::SqlExecutionEngine::InMemoryDataSource');

can_ok('CodingAdventures::SqlExecutionEngine', 'execute');
can_ok('CodingAdventures::SqlExecutionEngine', 'execute_all');
can_ok('CodingAdventures::SqlExecutionEngine::InMemoryDataSource', 'new');
can_ok('CodingAdventures::SqlExecutionEngine::InMemoryDataSource', 'schema');
can_ok('CodingAdventures::SqlExecutionEngine::InMemoryDataSource', 'scan');

done_testing;
