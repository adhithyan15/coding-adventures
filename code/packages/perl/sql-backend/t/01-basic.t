use strict;
use warnings;
use Test2::V0;

use CodingAdventures::SqlBackend qw(
    blob is_sql_value sql_type_name compare_sql_values
    column_def index_def trigger_def backend_as_schema_provider
);

sub new_users_backend {
    my $db = CodingAdventures::SqlBackend::InMemoryBackend->new;
    $db->create_table('users', [
        column_def(name => 'id', type_name => 'INTEGER', primary_key => 1),
        column_def(name => 'name', type_name => 'TEXT', not_null => 1),
        column_def(name => 'email', type_name => 'TEXT', unique => 1),
    ], if_not_exists => 0);
    return $db;
}

sub expect_error {
    my ($kind, $code) = @_;
    my $ok = eval { $code->(); 1 };
    my $err = $@;
    ok(!$ok, "$kind was thrown");
    is($err->{kind}, $kind, "$kind kind");
    return $err;
}

subtest 'SQL value helpers' => sub {
    ok(is_sql_value(undef), 'undef is SQL NULL');
    ok(is_sql_value(42), 'number');
    ok(is_sql_value('text'), 'text');
    ok(is_sql_value(blob('abc')), 'blob');
    ok(!is_sql_value({}), 'plain ref rejected');

    is(sql_type_name(undef), 'NULL', 'null type');
    is(sql_type_name('x'), 'TEXT', 'text type');
    is(sql_type_name(blob('x')), 'BLOB', 'blob type');
    ok(compare_sql_values(undef, 1) < 0, 'null sorts first');
    ok(compare_sql_values(1, 2) < 0, 'number compare');
    ok(compare_sql_values('b', 'a') > 0, 'text compare');
    is(compare_sql_values(blob('a'), blob('a')), 0, 'blob compare');
};

subtest 'iterators and cursors copy rows' => sub {
    my $rows = [{ id => 1, name => 'Ada' }, { id => 2, name => 'Grace' }];
    my $iter = CodingAdventures::SqlBackend::ListRowIterator->new($rows);
    my $first = $iter->next;
    $first->{name} = 'mutated';
    is($iter->next->{name}, 'Grace', 'iterator advances');
    is($iter->next, undef, 'iterator exhausts');

    my $cursor = CodingAdventures::SqlBackend::ListCursor->new($rows, 'users');
    is($cursor->next->{name}, 'Ada', 'cursor first row');
    my $current = $cursor->current_row;
    $current->{name} = 'mutated';
    is($cursor->current_row->{name}, 'Ada', 'cursor returns copies');
    is($cursor->current_index, 0, 'cursor index');
};

subtest 'schema and scans' => sub {
    my $db = new_users_backend();
    $db->insert('users', { id => 1, name => 'Ada' });
    $db->insert('users', { id => 2, name => 'Grace', email => 'grace@example.test' });

    is($db->tables, ['users'], 'table names');
    is([ map { $_->{name} } @{ $db->columns('USERS') } ], ['id', 'name', 'email'], 'columns');
    is(backend_as_schema_provider($db)->columns('users'), ['id', 'name', 'email'], 'schema adapter');

    my $rows = $db->scan('users')->to_array;
    is(scalar @$rows, 2, 'two rows');
    is($rows->[0]{name}, 'Ada', 'first row');
    is($rows->[0]{email}, undef, 'missing value is NULL');
};

subtest 'constraints' => sub {
    my $db = new_users_backend();
    $db->insert('users', { id => 1, name => 'Ada' });
    expect_error('ConstraintViolation', sub { $db->insert('users', { id => 2 }) });
    expect_error('ConstraintViolation', sub { $db->insert('users', { id => 1, name => 'Ada Again' }) });
    expect_error('ColumnNotFound', sub { $db->insert('users', { id => 3, name => 'Lin', missing => 1 }) });

    $db->insert('users', { id => 2, name => 'Grace' });
    $db->insert('users', { id => 3, name => 'Lin', email => 'lin@example.test' });
    expect_error('ConstraintViolation', sub {
        $db->insert('users', { id => 4, name => 'Other Lin', email => 'lin@example.test' });
    });
};

subtest 'positioned update and delete' => sub {
    my $db = new_users_backend();
    $db->insert('users', { id => 1, name => 'Ada' });
    $db->insert('users', { id => 2, name => 'Grace' });

    my $cursor = $db->open_cursor('users');
    $cursor->next;
    $db->update('users', $cursor, { name => 'Augusta Ada' });
    is($db->scan('users')->to_array->[0]{name}, 'Augusta Ada', 'updated');

    $cursor->next;
    $db->delete('users', $cursor);
    my $rows = $db->scan('users')->to_array;
    is(scalar @$rows, 1, 'deleted one');
    is($rows->[0]{name}, 'Augusta Ada', 'remaining row');
};

subtest 'DDL' => sub {
    my $db = new_users_backend();
    expect_error('TableAlreadyExists', sub { $db->create_table('users', [], if_not_exists => 0) });
    $db->create_table('users', [], if_not_exists => 1);
    $db->insert('users', { id => 1, name => 'Ada' });
    $db->add_column('users', column_def(name => 'active', type_name => 'BOOLEAN', default => 1));
    is($db->scan('users')->to_array->[0]{active}, 1, 'default applied');
    expect_error('ColumnAlreadyExists', sub {
        $db->add_column('users', column_def(name => 'ACTIVE', type_name => 'BOOLEAN'));
    });
    $db->drop_table('users', if_exists => 0);
    expect_error('TableNotFound', sub { $db->scan('users') });
    $db->drop_table('users', if_exists => 1);
};

subtest 'indexes' => sub {
    my $db = new_users_backend();
    $db->insert('users', { id => 1, name => 'Ada' });
    $db->insert('users', { id => 2, name => 'Grace' });
    $db->insert('users', { id => 3, name => 'Lin' });
    $db->create_index(index_def(name => 'idx_users_name', table => 'users', columns => ['name']));

    my $rowids = $db->scan_index('idx_users_name', ['G'], ['M'], lo_inclusive => 0, hi_inclusive => 0);
    my $rows = $db->scan_by_rowids('users', $rowids)->to_array;
    is([ map { $_->{name} } @$rows ], ['Grace', 'Lin'], 'index range rows');
    is($db->list_indexes('users')->[0]{name}, 'idx_users_name', 'listed index');
    expect_error('IndexAlreadyExists', sub {
        $db->create_index(index_def(name => 'idx_users_name', table => 'users', columns => ['id']));
    });
    $db->drop_index('idx_users_name');
    is($db->list_indexes, [], 'index dropped');
    $db->drop_index('idx_users_name', if_exists => 1);
    expect_error('IndexNotFound', sub { $db->scan_index('idx_users_name') });
};

subtest 'unique indexes' => sub {
    my $db = new_users_backend();
    $db->create_index(index_def(name => 'idx_email', table => 'users', columns => ['email'], unique => 1));
    $db->insert('users', { id => 1, name => 'Ada', email => 'ada@example.test' });
    $db->insert('users', { id => 2, name => 'Grace', email => 'grace@example.test' });
    expect_error('ConstraintViolation', sub {
        $db->insert('users', { id => 3, name => 'Other Ada', email => 'ada@example.test' });
    });
    my $cursor = $db->open_cursor('users');
    $cursor->next;
    $cursor->next;
    expect_error('ConstraintViolation', sub {
        $db->update('users', $cursor, { email => 'ada@example.test' });
    });
};

subtest 'transactions and savepoints' => sub {
    my $db = new_users_backend();
    my $tx = $db->begin_transaction;
    $db->insert('users', { id => 1, name => 'Ada' });
    is($db->current_transaction, $tx, 'current transaction');
    $db->rollback($tx);
    is($db->scan('users')->to_array, [], 'rollback restores');

    $tx = $db->begin_transaction;
    $db->insert('users', { id => 1, name => 'Ada' });
    $db->create_savepoint('after_ada');
    $db->insert('users', { id => 2, name => 'Grace' });
    $db->rollback_to_savepoint('after_ada');
    is(scalar @{ $db->scan('users')->to_array }, 1, 'savepoint rollback');
    $db->release_savepoint('after_ada');
    $db->commit($tx);
};

subtest 'triggers and versions' => sub {
    my $db = new_users_backend();
    my $initial = $db->{schema_version};
    my $trigger = trigger_def(
        name => 'users_ai',
        table => 'users',
        timing => 'after',
        event => 'insert',
        body => 'SELECT 1',
    );
    $db->create_trigger($trigger);
    ok($db->{schema_version} > $initial, 'schema version bumped');
    is($db->list_triggers('users')->[0]{name}, 'users_ai', 'listed trigger');
    expect_error('TriggerAlreadyExists', sub { $db->create_trigger($trigger) });
    $db->{user_version} = 7;
    is($db->{user_version}, 7, 'user version');
    $db->drop_trigger('users_ai');
    is($db->list_triggers('users'), [], 'trigger dropped');
    $db->drop_trigger('users_ai', if_exists => 1);
    expect_error('TriggerNotFound', sub { $db->drop_trigger('users_ai') });
};

done_testing;
