use strict;
use warnings;
use Test2::V0;

use lib '../csv-parser/lib';
use lib '../sql-execution-engine/lib';

use CodingAdventures::SqlCsvSource qw(coerce execute_csv);
use CodingAdventures::SqlCsvSource::CsvDataSource;

my $fixtures = 't/fixtures';

sub row_hashes {
    my ($result) = @_;
    my @columns = @{$result->{columns}};
    my @rows;
    for my $row (@{$result->{rows}}) {
        my %mapped;
        for my $index (0 .. $#columns) {
            $mapped{$columns[$index]} = $row->[$index];
        }
        push @rows, \%mapped;
    }
    return \@rows;
}

subtest 'coerce CSV values' => sub {
    is(coerce(''), undef, 'empty string is NULL');
    is(coerce('true'), 1, 'true is boolean-ish 1');
    is(coerce('False'), 0, 'false is boolean-ish 0');
    is(coerce('42'), 42, 'integer');
    is(coerce('-7'), -7, 'negative integer');
    is(coerce('3.14'), 3.14, 'float');
    is(coerce('Alice Smith'), 'Alice Smith', 'string');
};

subtest 'schema and scan' => sub {
    my $source = CodingAdventures::SqlCsvSource::CsvDataSource->new($fixtures);

    is($source->schema('employees'), ['id', 'name', 'dept_id', 'salary', 'active'], 'employee schema');
    is($source->schema('departments'), ['id', 'name', 'budget'], 'department schema');

    my $rows = $source->scan('employees');
    is(scalar @$rows, 4, 'four rows');
    is($rows->[0]{id}, 1, 'id coerced');
    is($rows->[0]{name}, 'Alice', 'string value');
    is($rows->[0]{salary}, 90000, 'salary coerced');
    is($rows->[0]{active}, 1, 'boolean true coerced');
    is($rows->[2]{active}, 0, 'boolean false coerced');
    is($rows->[3]{dept_id}, undef, 'empty field is NULL');
};

subtest 'missing table errors' => sub {
    my $source = CodingAdventures::SqlCsvSource::CsvDataSource->new($fixtures);

    like(dies { $source->schema('missing') }, qr/table not found: missing/, 'schema missing table');
    like(dies { $source->scan('ghosts') }, qr/table not found: ghosts/, 'scan missing table');
};

subtest 'execute SELECT queries' => sub {
    my ($ok, $result) = execute_csv('SELECT * FROM employees', $fixtures);
    ok($ok, 'SELECT * succeeds') or diag($result);
    is(scalar @{$result->{rows}}, 4, 'four projected rows');

    my $rows = row_hashes($result);
    is($rows->[0]{name}, 'Alice', 'first row name');
    is($rows->[0]{active}, 1, 'active value');
    is($rows->[3]{dept_id}, undef, 'NULL value survives projection');
};

subtest 'filter predicates' => sub {
    my ($ok, $result) = execute_csv('SELECT name FROM employees WHERE active = true', $fixtures);
    ok($ok, 'active filter succeeds') or diag($result);
    is([sort map { $_->{name} } @{row_hashes($result)}], ['Alice', 'Bob', 'Dave'], 'active employees');

    ($ok, $result) = execute_csv('SELECT * FROM employees WHERE dept_id IS NULL', $fixtures);
    ok($ok, 'IS NULL succeeds') or diag($result);
    is(scalar @{$result->{rows}}, 1, 'one NULL row');
    is(row_hashes($result)->[0]{name}, 'Dave', 'Dave has NULL department');
};

subtest 'joins through execution engine' => sub {
    my ($ok, $result) = execute_csv(
        'SELECT e.name, d.name FROM employees AS e INNER JOIN departments AS d ON e.dept_id = d.id',
        $fixtures,
    );
    ok($ok, 'join succeeds') or diag($result);
    is(scalar @{$result->{rows}}, 3, 'three employees have departments');
};

subtest 'execute_csv reports missing tables' => sub {
    my ($ok, $err) = execute_csv('SELECT * FROM ghosts', $fixtures);
    ok(!$ok, 'missing table returns false');
    like($err, qr/table not found: ghosts/, 'error mentions table');
};

done_testing;
