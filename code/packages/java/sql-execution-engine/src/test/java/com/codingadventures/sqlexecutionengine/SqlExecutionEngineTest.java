package com.codingadventures.sqlexecutionengine;

import org.junit.jupiter.api.Test;

import java.util.List;
import java.util.Map;

import static com.codingadventures.sqlexecutionengine.SqlExecutionEngine.execute;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

class SqlExecutionEngineTest {
    private static SqlExecutionEngine.InMemoryDataSource dataSource() {
        return new SqlExecutionEngine.InMemoryDataSource()
            .addTable(
                "employees",
                List.of("id", "name", "dept", "salary", "active"),
                List.of(
                    row("id", 1, "name", "Alice", "dept", "Engineering", "salary", 95_000, "active", true),
                    row("id", 2, "name", "Bob", "dept", "Marketing", "salary", 72_000, "active", true),
                    row("id", 3, "name", "Carol", "dept", "Engineering", "salary", 88_000, "active", false),
                    row("id", 4, "name", "Dave", "dept", null, "salary", 60_000, "active", true),
                    row("id", 5, "name", "Eve", "dept", "HR", "salary", 70_000, "active", false)
                )
            )
            .addTable(
                "departments",
                List.of("dept", "budget"),
                List.of(
                    row("dept", "Engineering", "budget", 500_000),
                    row("dept", "Marketing", "budget", 200_000),
                    row("dept", "HR", "budget", 150_000)
                )
            );
    }

    @Test
    void scansInMemoryTables() {
        var source = dataSource();

        assertEquals(List.of("id", "name", "dept", "salary", "active"), source.schema("employees"));
        assertEquals(5, source.scan("employees").size());
        assertThrows(SqlExecutionEngine.SqlExecutionException.class, () -> source.schema("missing"));
    }

    @Test
    void selectsAndFiltersRows() {
        var result = execute(
            "SELECT name, salary FROM employees WHERE active = true AND salary >= 70000 ORDER BY salary DESC",
            dataSource()
        );

        assertEquals(List.of("name", "salary"), result.columns());
        assertEquals(List.of("Alice", 95_000), result.rows().get(0));
        assertEquals(List.of("Bob", 72_000), result.rows().get(1));
    }

    @Test
    void supportsNullPredicatesAndLike() {
        var nullResult = execute("SELECT name FROM employees WHERE dept IS NULL", dataSource());
        assertEquals(List.of(List.of("Dave")), nullResult.rows());

        var likeResult = execute("SELECT name FROM employees WHERE name LIKE 'A%'", dataSource());
        assertEquals(List.of(List.of("Alice")), likeResult.rows());
    }

    @Test
    void supportsJoins() {
        var result = execute(
            "SELECT e.name, d.budget FROM employees AS e INNER JOIN departments AS d ON e.dept = d.dept ORDER BY e.id",
            dataSource()
        );

        assertEquals(List.of("name", "budget"), result.columns());
        assertEquals(4, result.rows().size());
        assertEquals(List.of("Alice", 500_000), result.rows().get(0));
        assertEquals(List.of("Eve", 150_000), result.rows().get(3));
    }

    @Test
    void supportsGroupingAndAggregates() {
        var result = execute(
            "SELECT dept, COUNT(*) AS cnt, SUM(salary) AS total FROM employees WHERE dept IS NOT NULL GROUP BY dept HAVING COUNT(*) >= 1 ORDER BY dept",
            dataSource()
        );

        assertEquals(List.of("dept", "cnt", "total"), result.columns());
        assertEquals(List.of("Engineering", 2, 183_000.0), result.rows().get(0));
        assertEquals(List.of("HR", 1, 70_000.0), result.rows().get(1));
        assertEquals(List.of("Marketing", 1, 72_000.0), result.rows().get(2));
    }

    @Test
    void supportsDistinctLimitAndOffset() {
        var result = execute("SELECT DISTINCT dept FROM employees WHERE dept IS NOT NULL ORDER BY dept LIMIT 2 OFFSET 1", dataSource());

        assertEquals(List.of("dept"), result.columns());
        assertEquals(List.of(List.of("HR"), List.of("Marketing")), result.rows());
    }

    @Test
    void reportsErrorsThroughTryExecute() {
        var result = SqlExecutionEngine.tryExecute("SELECT * FROM ghosts", dataSource());

        assertFalse(result.ok());
        assertNotNull(result.error());
        assertTrue(result.error().contains("table not found: ghosts"));
    }

    @Test
    void selectStarUsesBareColumns() {
        var result = execute("SELECT * FROM employees WHERE id = 1", dataSource());

        assertEquals(List.of("active", "dept", "id", "name", "salary"), result.columns());
        assertEquals(List.of(true, "Engineering", 1, "Alice", 95_000), result.rows().get(0));
    }

    private static Map<String, Object> row(Object... pairs) {
        var row = new java.util.LinkedHashMap<String, Object>();
        for (int index = 0; index < pairs.length; index += 2) {
            row.put((String) pairs[index], pairs[index + 1]);
        }
        return row;
    }
}
