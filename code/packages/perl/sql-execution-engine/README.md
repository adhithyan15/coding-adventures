# sql-execution-engine (Perl)

SELECT-only SQL execution engine with pluggable data sources.

## Usage

```perl
use CodingAdventures::SqlExecutionEngine;
use CodingAdventures::SqlExecutionEngine::InMemoryDataSource;

my $ds = CodingAdventures::SqlExecutionEngine::InMemoryDataSource->new({
    employees => [
        { id => 1, name => 'Alice', dept => 'Engineering', salary => 95000 },
        { id => 2, name => 'Bob',   dept => 'Marketing',   salary => 72000 },
    ],
});

my ($ok, $result) = CodingAdventures::SqlExecutionEngine->execute(
    "SELECT name, salary FROM employees WHERE salary > 80000 ORDER BY salary DESC",
    $ds,
);

if ($ok) {
    print join(', ', @{$result->{columns}}), "\n";
    for my $row (@{$result->{rows}}) {
        print join(', ', map { defined $_ ? $_ : 'NULL' } @$row), "\n";
    }
}
```

## Supported SQL

```sql
SELECT [DISTINCT] col1, col2, expr AS alias
FROM table
[JOIN table ON condition]
[WHERE expr]
[GROUP BY col]
[HAVING expr]
[ORDER BY col [ASC|DESC]]
[LIMIT n [OFFSET m]]
```

## Dependencies

None beyond core Perl modules (`Scalar::Util`, `POSIX`, `List::Util`).
