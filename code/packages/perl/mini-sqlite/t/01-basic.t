use strict;
use warnings;
use Test2::V0;

use CodingAdventures::MiniSqlite;

sub connect_ok {
    my ($conn, $err) = CodingAdventures::MiniSqlite->connect(':memory:');
    is($err, undef, 'connect has no error');
    ok($conn, 'connect returns a connection');
    return $conn;
}

subtest 'DB-API style constants' => sub {
    is($CodingAdventures::MiniSqlite::apilevel, '2.0', 'apilevel');
    is($CodingAdventures::MiniSqlite::threadsafety, 1, 'threadsafety');
    is($CodingAdventures::MiniSqlite::paramstyle, 'qmark', 'paramstyle');
};

subtest 'creates, inserts, and selects rows' => sub {
    my $conn = connect_ok();
    ok($conn->execute('CREATE TABLE users (id INTEGER, name TEXT, active BOOLEAN)'), 'created table');
    ok($conn->executemany('INSERT INTO users VALUES (?, ?, ?)', [
        [1, 'Alice', 1],
        [2, 'Bob', 0],
        [3, 'Carol', 1],
    ]), 'inserted rows');

    my $cursor = $conn->execute('SELECT name FROM users WHERE active = ? ORDER BY id ASC', [1]);
    is($cursor->{description}[0]{name}, 'name', 'description names selected column');
    my $rows = $cursor->fetchall;
    is($rows->[0][0], 'Alice', 'first active user');
    is($rows->[1][0], 'Carol', 'second active user');
};

subtest 'fetches incrementally' => sub {
    my $conn = connect_ok();
    $conn->execute('CREATE TABLE nums (n INTEGER)');
    $conn->executemany('INSERT INTO nums VALUES (?)', [[1], [2], [3]]);
    my $cursor = $conn->execute('SELECT n FROM nums ORDER BY n ASC');

    is($cursor->fetchone->[0], 1, 'fetchone');
    is($cursor->fetchmany(1)->[0][0], 2, 'fetchmany');
    is($cursor->fetchall->[0][0], 3, 'fetchall');
    is($cursor->fetchone, undef, 'fetchone after exhaustion');
};

subtest 'updates and deletes rows' => sub {
    my $conn = connect_ok();
    $conn->execute('CREATE TABLE users (id INTEGER, name TEXT)');
    $conn->executemany('INSERT INTO users VALUES (?, ?)', [
        [1, 'Alice'],
        [2, 'Bob'],
        [3, 'Carol'],
    ]);

    my $updated = $conn->execute('UPDATE users SET name = ? WHERE id = ?', ['Bobby', 2]);
    is($updated->{rowcount}, 1, 'one updated row');

    my $deleted = $conn->execute('DELETE FROM users WHERE id IN (?, ?)', [1, 3]);
    is($deleted->{rowcount}, 2, 'two deleted rows');

    my $rows = $conn->execute('SELECT id, name FROM users')->fetchall;
    is($rows->[0][0], 2, 'remaining id');
    is($rows->[0][1], 'Bobby', 'remaining name');
};

subtest 'rolls back and commits snapshots' => sub {
    my $conn = connect_ok();
    $conn->execute('CREATE TABLE users (id INTEGER, name TEXT)');
    $conn->commit;
    $conn->execute('INSERT INTO users VALUES (?, ?)', [1, 'Alice']);
    $conn->rollback;
    is(scalar @{ $conn->execute('SELECT * FROM users')->fetchall }, 0, 'rollback removes uncommitted insert');

    $conn->execute('INSERT INTO users VALUES (?, ?)', [1, 'Alice']);
    $conn->commit;
    $conn->rollback;
    is(scalar @{ $conn->execute('SELECT * FROM users')->fetchall }, 1, 'committed insert survives rollback');
};

subtest 'rejects file-backed connections' => sub {
    my ($conn, $err) = CodingAdventures::MiniSqlite->connect('app.db');
    is($conn, undef, 'no connection');
    is($err->{kind}, 'NotSupportedError', 'not supported error');
};

done_testing;
