use strict;
use warnings;
use Test2::V0;

use CodingAdventures::SqlExecutionEngine;
use CodingAdventures::SqlExecutionEngine::InMemoryDataSource;

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

sub make_ds {
    return CodingAdventures::SqlExecutionEngine::InMemoryDataSource->new({
        employees => [
            { id => 1, name => 'Alice', dept => 'Engineering', salary => 95000 },
            { id => 2, name => 'Bob',   dept => 'Marketing',   salary => 72000 },
            { id => 3, name => 'Carol', dept => 'Engineering', salary => 88000 },
            { id => 4, name => 'Dave',  dept => 'Marketing',   salary => 65000 },
            { id => 5, name => 'Eve',   dept => 'HR',          salary => 70000 },
            { id => 6, name => 'Frank', dept => 'Engineering', salary => 91000 },
            { id => 7, name => 'Grace', dept => undef,         salary => 60000 },
        ],
        departments => [
            { id => 1, dept_name => 'Engineering', budget => 500000 },
            { id => 2, dept_name => 'Marketing',   budget => 200000 },
            { id => 3, dept_name => 'HR',          budget => 150000 },
        ],
    });
}

sub exec_ok {
    my ($sql) = @_;
    my $ds = make_ds();
    my ($ok, $result) = CodingAdventures::SqlExecutionEngine->execute($sql, $ds);
    ok($ok, "SQL succeeded: $sql") or diag("Error: $result");
    return $result;
}

# ---------------------------------------------------------------------------
# InMemoryDataSource
# ---------------------------------------------------------------------------

subtest 'InMemoryDataSource schema' => sub {
    my $ds   = make_ds();
    my $cols = $ds->schema('employees');
    my %col_set = map { $_ => 1 } @$cols;
    ok($col_set{id},     'schema has id');
    ok($col_set{name},   'schema has name');
    ok($col_set{dept},   'schema has dept');
    ok($col_set{salary}, 'schema has salary');
};

subtest 'InMemoryDataSource scan' => sub {
    my $ds   = make_ds();
    my $rows = $ds->scan('employees');
    is(scalar @$rows, 7, '7 employee rows');
    my ($alice) = grep { $_->{name} eq 'Alice' } @$rows;
    is($alice->{salary}, 95000, 'Alice salary');
    is($alice->{dept},   'Engineering', 'Alice dept');
};

subtest 'InMemoryDataSource unknown table' => sub {
    my $ds = make_ds();
    ok(dies { $ds->schema('nonexistent') }, 'schema raises on unknown table');
    ok(dies { $ds->scan('nonexistent')   }, 'scan raises on unknown table');
};

# ---------------------------------------------------------------------------
# SELECT *
# ---------------------------------------------------------------------------

subtest 'SELECT *' => sub {
    my $result = exec_ok('SELECT * FROM employees');
    is(scalar @{$result->{rows}}, 7, 'all 7 rows returned');
    ok(scalar @{$result->{columns}} >= 4, 'at least 4 columns');
};

# ---------------------------------------------------------------------------
# SELECT specific columns
# ---------------------------------------------------------------------------

subtest 'SELECT specific columns' => sub {
    my $result = exec_ok('SELECT name, salary FROM employees');
    is(scalar @{$result->{columns}}, 2, '2 columns');
    is(scalar @{$result->{rows}},    7, '7 rows');
    is($result->{columns}[0], 'name',   'first column is name');
    is($result->{columns}[1], 'salary', 'second column is salary');
};

# ---------------------------------------------------------------------------
# WHERE clause
# ---------------------------------------------------------------------------

subtest 'WHERE equality' => sub {
    my $result = exec_ok("SELECT name FROM employees WHERE dept = 'Engineering'");
    is(scalar @{$result->{rows}}, 3, '3 Engineering employees');
};

subtest 'WHERE numeric comparison' => sub {
    my $result = exec_ok('SELECT name FROM employees WHERE salary > 80000');
    is(scalar @{$result->{rows}}, 3, '3 employees with salary > 80000');
};

subtest 'WHERE AND' => sub {
    my $result = exec_ok("SELECT name FROM employees WHERE dept = 'Engineering' AND salary > 90000");
    is(scalar @{$result->{rows}}, 2, '2 rows (Alice + Frank)');
};

subtest 'WHERE OR' => sub {
    my $result = exec_ok("SELECT name FROM employees WHERE dept = 'HR' OR salary > 90000");
    is(scalar @{$result->{rows}}, 3, '3 rows (Eve + Alice + Frank)');
};

subtest 'WHERE !=' => sub {
    my $r1 = exec_ok("SELECT name FROM employees WHERE dept != 'Engineering'");
    my $r2 = exec_ok("SELECT name FROM employees WHERE dept <> 'Engineering'");
    is(scalar @{$r1->{rows}}, scalar @{$r2->{rows}}, '!= and <> agree');
};

subtest 'WHERE IS NULL' => sub {
    my $result = exec_ok('SELECT name FROM employees WHERE dept IS NULL');
    is(scalar @{$result->{rows}}, 1, '1 NULL-dept row');
    is($result->{rows}[0][0], 'Grace', 'Grace has NULL dept');
};

subtest 'WHERE IS NOT NULL' => sub {
    my $result = exec_ok('SELECT name FROM employees WHERE dept IS NOT NULL');
    is(scalar @{$result->{rows}}, 6, '6 non-NULL dept rows');
};

subtest 'WHERE >= and <=' => sub {
    my $result = exec_ok('SELECT name FROM employees WHERE salary >= 88000 AND salary <= 95000');
    is(scalar @{$result->{rows}}, 3, '3 rows in salary range');
};

# ---------------------------------------------------------------------------
# BETWEEN and IN
# ---------------------------------------------------------------------------

subtest 'BETWEEN inclusive' => sub {
    my $result = exec_ok('SELECT name FROM employees WHERE salary BETWEEN 70000 AND 90000');
    is(scalar @{$result->{rows}}, 3, '3 rows BETWEEN 70k and 90k');
};

subtest 'IN matches' => sub {
    my $result = exec_ok("SELECT name FROM employees WHERE dept IN ('Engineering', 'HR')");
    is(scalar @{$result->{rows}}, 4, '4 rows in Engineering or HR');
};

# ---------------------------------------------------------------------------
# LIKE
# ---------------------------------------------------------------------------

subtest 'LIKE with %' => sub {
    my $result = exec_ok("SELECT name FROM employees WHERE name LIKE 'A%'");
    is(scalar @{$result->{rows}}, 1, '1 name starting with A');
    is($result->{rows}[0][0], 'Alice', 'Alice matches A%');
};

subtest 'LIKE with _' => sub {
    my $result = exec_ok("SELECT name FROM employees WHERE name LIKE '_ob'");
    is(scalar @{$result->{rows}}, 1, '1 name matching _ob');
    is($result->{rows}[0][0], 'Bob', 'Bob matches _ob');
};

subtest 'LIKE dept ending in ing' => sub {
    my $result = exec_ok("SELECT name FROM employees WHERE dept LIKE '%ing'");
    is(scalar @{$result->{rows}}, 5, '5 employees in dept ending with ing');
};

# ---------------------------------------------------------------------------
# ORDER BY
# ---------------------------------------------------------------------------

subtest 'ORDER BY ASC (default)' => sub {
    my $result = exec_ok('SELECT name, salary FROM employees ORDER BY salary');
    my @salaries = map { $_->[1] } @{$result->{rows}};
    for my $i (1 .. $#salaries) {
        ok($salaries[$i] >= $salaries[$i-1], "row $i salary >= row ".($i-1));
    }
};

subtest 'ORDER BY DESC' => sub {
    my $result = exec_ok('SELECT name, salary FROM employees ORDER BY salary DESC');
    my @salaries = map { $_->[1] } @{$result->{rows}};
    for my $i (1 .. $#salaries) {
        ok($salaries[$i] <= $salaries[$i-1], "row $i salary <= row ".($i-1));
    }
};

subtest 'ORDER BY name ASC' => sub {
    my $result = exec_ok('SELECT name FROM employees ORDER BY name ASC');
    my @names = map { $_->[0] } @{$result->{rows}};
    for my $i (1 .. $#names) {
        ok($names[$i] ge $names[$i-1], "row $i name ge row ".($i-1));
    }
};

# ---------------------------------------------------------------------------
# LIMIT and OFFSET
# ---------------------------------------------------------------------------

subtest 'LIMIT' => sub {
    my $result = exec_ok('SELECT name FROM employees LIMIT 3');
    is(scalar @{$result->{rows}}, 3, '3 rows with LIMIT 3');
};

subtest 'OFFSET' => sub {
    my $all    = exec_ok('SELECT name FROM employees ORDER BY id');
    my $offset = exec_ok('SELECT name FROM employees ORDER BY id OFFSET 2');
    is(scalar @{$offset->{rows}}, scalar(@{$all->{rows}}) - 2, 'OFFSET 2 removes 2 rows');
};

subtest 'LIMIT with OFFSET' => sub {
    my $result = exec_ok('SELECT name FROM employees ORDER BY id LIMIT 2 OFFSET 1');
    is(scalar @{$result->{rows}}, 2, '2 rows with LIMIT 2 OFFSET 1');
};

subtest 'LIMIT larger than row count' => sub {
    my $result = exec_ok('SELECT name FROM employees LIMIT 100');
    is(scalar @{$result->{rows}}, 7, 'all 7 rows returned when LIMIT > count');
};

# ---------------------------------------------------------------------------
# DISTINCT
# ---------------------------------------------------------------------------

subtest 'DISTINCT' => sub {
    my $result = exec_ok('SELECT DISTINCT dept FROM employees');
    # Engineering, Marketing, HR, undef = 4 distinct
    is(scalar @{$result->{rows}}, 4, '4 distinct dept values (including NULL)');
};

subtest 'DISTINCT IS NOT NULL' => sub {
    my $result = exec_ok('SELECT DISTINCT dept FROM employees WHERE dept IS NOT NULL');
    is(scalar @{$result->{rows}}, 3, '3 distinct non-NULL depts');
};

# ---------------------------------------------------------------------------
# Aggregate functions
# ---------------------------------------------------------------------------

subtest 'COUNT(*)' => sub {
    my $result = exec_ok('SELECT COUNT(*) FROM employees');
    is($result->{rows}[0][0], 7, 'COUNT(*) = 7');
};

subtest 'COUNT(col) excludes NULL' => sub {
    my $result = exec_ok('SELECT COUNT(dept) FROM employees');
    is($result->{rows}[0][0], 6, 'COUNT(dept) = 6 (Grace excluded)');
};

subtest 'SUM' => sub {
    my $result = exec_ok('SELECT SUM(salary) FROM employees');
    is($result->{rows}[0][0], 541000, 'SUM(salary) = 541000');
};

subtest 'MIN' => sub {
    my $result = exec_ok('SELECT MIN(salary) FROM employees');
    is($result->{rows}[0][0], 60000, 'MIN(salary) = 60000');
};

subtest 'MAX' => sub {
    my $result = exec_ok('SELECT MAX(salary) FROM employees');
    is($result->{rows}[0][0], 95000, 'MAX(salary) = 95000');
};

subtest 'AVG' => sub {
    my $result = exec_ok('SELECT AVG(salary) FROM employees');
    my $avg = $result->{rows}[0][0];
    ok(abs($avg - 77285.71) < 1, "AVG(salary) ≈ 77285.71 (got $avg)");
};

# ---------------------------------------------------------------------------
# GROUP BY
# ---------------------------------------------------------------------------

subtest 'GROUP BY count per group' => sub {
    my $result = exec_ok('SELECT dept, COUNT(*) FROM employees GROUP BY dept');
    is(scalar @{$result->{rows}}, 4, '4 groups (Engineering, Marketing, HR, NULL)');
};

subtest 'GROUP BY sum per group' => sub {
    my $result = exec_ok(
        "SELECT dept, SUM(salary) FROM employees WHERE dept IS NOT NULL GROUP BY dept ORDER BY dept"
    );
    my ($eng) = grep { defined $_->[0] && $_->[0] eq 'Engineering' } @{$result->{rows}};
    is($eng->[1], 274000, 'Engineering SUM = 95000+88000+91000 = 274000');
};

subtest 'HAVING filters groups' => sub {
    my $result = exec_ok("SELECT dept, COUNT(*) FROM employees GROUP BY dept HAVING COUNT(*) > 1");
    is(scalar @{$result->{rows}}, 2, '2 groups with COUNT > 1');
};

subtest 'HAVING AVG' => sub {
    my $result = exec_ok(
        "SELECT dept, AVG(salary) FROM employees GROUP BY dept HAVING AVG(salary) > 80000"
    );
    is(scalar @{$result->{rows}}, 1, '1 group with AVG > 80000 (Engineering)');
    is($result->{rows}[0][0], 'Engineering', 'Engineering has highest avg');
};

# ---------------------------------------------------------------------------
# Expressions and arithmetic
# ---------------------------------------------------------------------------

subtest 'arithmetic in SELECT' => sub {
    my $result = exec_ok("SELECT name, salary * 1.1 FROM employees WHERE name = 'Alice'");
    is(scalar @{$result->{rows}}, 1, '1 row');
    my $raised = $result->{rows}[0][1];
    ok(abs($raised - 104500) < 1, "salary * 1.1 ≈ 104500 (got $raised)");
};

subtest 'column alias with AS' => sub {
    my $result = exec_ok("SELECT name, salary AS pay FROM employees WHERE name = 'Alice'");
    is($result->{columns}[1], 'pay', 'alias pay');
};

subtest 'literal string in SELECT' => sub {
    my $result = exec_ok("SELECT 'hello' FROM employees LIMIT 1");
    is($result->{rows}[0][0], 'hello', 'literal string');
};

subtest 'literal number in SELECT' => sub {
    my $result = exec_ok('SELECT 42 FROM employees LIMIT 1');
    is($result->{rows}[0][0], 42, 'literal number');
};

# ---------------------------------------------------------------------------
# String functions
# ---------------------------------------------------------------------------

subtest 'UPPER' => sub {
    my $result = exec_ok("SELECT UPPER(name) FROM employees WHERE name = 'Alice'");
    is($result->{rows}[0][0], 'ALICE', 'UPPER');
};

subtest 'LOWER' => sub {
    my $result = exec_ok("SELECT LOWER(name) FROM employees WHERE name = 'Alice'");
    is($result->{rows}[0][0], 'alice', 'LOWER');
};

subtest 'LENGTH' => sub {
    my $result = exec_ok("SELECT LENGTH(name) FROM employees WHERE name = 'Alice'");
    is($result->{rows}[0][0], 5, 'LENGTH(Alice) = 5');
};

# ---------------------------------------------------------------------------
# INNER JOIN
# ---------------------------------------------------------------------------

subtest 'INNER JOIN' => sub {
    my $result = exec_ok(q{
        SELECT employees.name, departments.budget
        FROM employees
        INNER JOIN departments ON employees.dept = departments.dept_name
    });
    # Grace has NULL dept → excluded by INNER JOIN
    is(scalar @{$result->{rows}}, 6, '6 rows from INNER JOIN');
};

subtest 'implicit INNER JOIN' => sub {
    my $result = exec_ok(q{
        SELECT employees.name, departments.budget
        FROM employees
        JOIN departments ON employees.dept = departments.dept_name
    });
    is(scalar @{$result->{rows}}, 6, '6 rows from JOIN');
};

# ---------------------------------------------------------------------------
# NULL handling
# ---------------------------------------------------------------------------

subtest 'NULL comparison returns NULL (excluded from WHERE)' => sub {
    my $result = exec_ok("SELECT name FROM employees WHERE dept = 'Engineering'");
    my %names = map { $_->[0] => 1 } @{$result->{rows}};
    ok(!$names{Grace}, 'Grace (NULL dept) not in Engineering results');
};

subtest 'COUNT(*) includes NULL rows' => sub {
    my $result = exec_ok('SELECT COUNT(*) FROM employees');
    is($result->{rows}[0][0], 7, 'COUNT(*) includes Grace');
};

# ---------------------------------------------------------------------------
# execute_all
# ---------------------------------------------------------------------------

subtest 'execute_all' => sub {
    my $ds = make_ds();
    my ($results, $err) = CodingAdventures::SqlExecutionEngine->execute_all(
        'SELECT COUNT(*) FROM employees; SELECT COUNT(*) FROM departments',
        $ds,
    );
    ok(!defined $err,     'no error');
    is(scalar @$results,  2, '2 results');
    is($results->[0]{rows}[0][0], 7, 'first query: 7 employees');
    is($results->[1]{rows}[0][0], 3, 'second query: 3 departments');
};

# ---------------------------------------------------------------------------
# Error handling
# ---------------------------------------------------------------------------

subtest 'syntax error returns false' => sub {
    my $ds = make_ds();
    my ($ok, $msg) = CodingAdventures::SqlExecutionEngine->execute('SELECT FROM', $ds);
    ok(!$ok, 'syntax error returns false');
    ok(defined $msg && length($msg) > 0, 'error message present');
};

subtest 'unknown table returns false' => sub {
    my $ds = make_ds();
    my ($ok, $msg) = CodingAdventures::SqlExecutionEngine->execute(
        'SELECT * FROM nonexistent', $ds
    );
    ok(!$ok, 'unknown table returns false');
    ok(defined $msg && length($msg) > 0, 'error message present');
};

# ---------------------------------------------------------------------------
# Complex queries
# ---------------------------------------------------------------------------

subtest 'combined WHERE GROUP BY HAVING ORDER BY LIMIT' => sub {
    my $result = exec_ok(q{
        SELECT dept, COUNT(*) AS cnt, AVG(salary) AS avg_pay
        FROM employees
        WHERE salary > 60000
        GROUP BY dept
        HAVING COUNT(*) >= 2
        ORDER BY avg_pay DESC
        LIMIT 2
    });
    ok(scalar @{$result->{rows}} <= 2, 'at most 2 rows with LIMIT 2');
    is(scalar @{$result->{columns}}, 3, '3 columns');
};

subtest 'SELECT expressions with ORDER BY DESC' => sub {
    my $result = exec_ok(q{
        SELECT name, salary * 12 AS annual_salary
        FROM employees
        WHERE dept = 'Engineering'
        ORDER BY annual_salary DESC
    });
    is(scalar @{$result->{rows}}, 3, '3 Engineering employees');
    is($result->{columns}[1], 'annual_salary', 'alias annual_salary');
    # Verify DESC order
    my @annuals = map { $_->[1] } @{$result->{rows}};
    for my $i (1 .. $#annuals) {
        ok($annuals[$i] <= $annuals[$i-1], "row $i <= row ".($i-1)." (DESC)");
    }
};

done_testing;
