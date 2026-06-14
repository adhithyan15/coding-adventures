# CodingAdventures::MiniSqlite

`CodingAdventures::MiniSqlite` is the Perl Level 0 port of the mini-sqlite facade. It provides an in-memory connection and cursor API while delegating `SELECT` execution to `CodingAdventures::SqlExecutionEngine`.

## Scope

- `CodingAdventures::MiniSqlite->connect(':memory:')`
- `CREATE TABLE`, `DROP TABLE`, `INSERT`, `UPDATE`, `DELETE`
- `SELECT` queries supported by the Perl SQL execution engine
- qmark parameter binding
- `commit`, `rollback`, and close-time rollback snapshots when autocommit is disabled

File-backed SQLite pages are out of scope for Level 0. Opening anything other than `:memory:` returns a `NotSupportedError`.

## Example

```perl
use CodingAdventures::MiniSqlite;

my $conn = CodingAdventures::MiniSqlite->connect(':memory:');
$conn->execute('CREATE TABLE users (id INTEGER, name TEXT)');
$conn->execute('INSERT INTO users VALUES (?, ?)', [1, 'Alice']);

my $cursor = $conn->execute('SELECT name FROM users');
my $rows = $cursor->fetchall;
die unless $rows->[0][0] eq 'Alice';
```
