use strict;
use warnings;
use Test2::V0;

use CodingAdventures::SqlBackend qw(column_def index_def trigger_def blob);

ok(column_def(name => 'id', type_name => 'INTEGER'), 'exports column_def');
ok(index_def(name => 'idx', table => 'users', columns => ['id']), 'exports index_def');
ok(trigger_def(name => 'trg', table => 'users', timing => 'AFTER', event => 'INSERT', body => 'SELECT 1'), 'exports trigger_def');
ok(blob('abc'), 'exports blob');

done_testing;
