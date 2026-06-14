use strict;
use warnings;
use Test2::V0;

use lib '../csv-parser/lib';
use lib '../sql-execution-engine/lib';

use CodingAdventures::SqlCsvSource qw(coerce execute_csv);
use CodingAdventures::SqlCsvSource::CsvDataSource;

ok(CodingAdventures::SqlCsvSource->can('execute_csv'), 'execute_csv is available');
ok(CodingAdventures::SqlCsvSource->can('coerce'), 'coerce is available');
ok(CodingAdventures::SqlCsvSource::CsvDataSource->can('new'), 'CsvDataSource constructor is available');

is(CodingAdventures::SqlCsvSource->VERSION, '0.01', 'version');
is(coerce('42'), 42, 'exported coerce works');

my ($ok, $result) = execute_csv('SELECT name FROM employees LIMIT 1', 't/fixtures');
ok($ok, 'execute_csv runs a smoke query') or diag($result);

done_testing;
