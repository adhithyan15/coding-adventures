# CodingAdventures::SqlBackend

Perl port of the mini-sqlite backend contract. The distribution exposes SQL
value helpers, schema/index/trigger metadata, typed backend errors, row
iterators, positioned cursors, transactions, savepoints, version fields, and an
in-memory backend.

```perl
use CodingAdventures::SqlBackend qw(column_def);

my $db = CodingAdventures::SqlBackend::InMemoryBackend->new;
$db->create_table('users', [
    column_def(name => 'id', type_name => 'INTEGER', primary_key => 1),
    column_def(name => 'name', type_name => 'TEXT', not_null => 1),
], if_not_exists => 0);

$db->insert('users', { id => 1, name => 'Ada' });
my $rows = $db->scan('users')->to_array;
```
