package com.codingadventures.sqlcsvsource;

import com.codingadventures.sqlexecutionengine.SqlExecutionEngine;
import org.junit.jupiter.api.Test;

import java.net.URISyntaxException;
import java.nio.file.Path;
import java.util.List;
import java.util.Map;

import static com.codingadventures.sqlcsvsource.SqlCsvSource.executeCsv;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertInstanceOf;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

class SqlCsvSourceTest {
    private static Path fixtures() {
        try {
            return Path.of(SqlCsvSourceTest.class.getResource("/fixtures/employees.csv").toURI()).getParent();
        } catch (URISyntaxException ex) {
            throw new AssertionError(ex);
        }
    }

    @Test
    void exposesSchemaInHeaderOrder() {
        var source = new SqlCsvSource.CsvDataSource(fixtures());

        assertEquals(List.of("id", "name", "dept_id", "salary", "active"), source.schema("employees"));
        assertEquals(List.of("id", "name", "budget"), source.schema("departments"));
    }

    @Test
    void scansRowsWithCoercedValues() {
        var rows = new SqlCsvSource.CsvDataSource(fixtures()).scan("employees");

        assertEquals(4, rows.size());
        Map<String, Object> alice = rows.get(0);
        assertEquals(1L, alice.get("id"));
        assertEquals("Alice", alice.get("name"));
        assertEquals(90_000L, alice.get("salary"));
        assertEquals(true, alice.get("active"));

        Map<String, Object> dave = rows.get(3);
        assertEquals("Dave", dave.get("name"));
        assertNull(dave.get("dept_id"));
    }

    @Test
    void executesSelectsAgainstCsvFiles() {
        var result = executeCsv(
            "SELECT name, salary FROM employees WHERE active = true AND salary > 70000 ORDER BY salary DESC",
            fixtures()
        );

        assertEquals(List.of("name", "salary"), result.columns());
        assertEquals(List.of("Alice", 90_000L), result.rows().get(0));
        assertEquals(List.of("Bob", 75_000L), result.rows().get(1));
    }

    @Test
    void supportsNullPredicates() {
        var result = executeCsv("SELECT name FROM employees WHERE dept_id IS NULL", fixtures());

        assertEquals(List.of(List.of("Dave")), result.rows());
    }

    @Test
    void supportsJoinsAcrossCsvFiles() {
        var result = executeCsv("""
            SELECT e.name AS emp_name, d.name AS dept_name
            FROM employees AS e
            INNER JOIN departments AS d ON e.dept_id = d.id
            ORDER BY e.id
            """, fixtures());

        assertEquals(List.of("emp_name", "dept_name"), result.columns());
        assertEquals(List.of("Alice", "Engineering"), result.rows().get(0));
        assertEquals(List.of("Bob", "Marketing"), result.rows().get(1));
        assertEquals(List.of("Carol", "Engineering"), result.rows().get(2));
    }

    @Test
    void supportsGroupingAggregatesLimitAndOffset() {
        var result = executeCsv(
            "SELECT dept_id, COUNT(*) AS cnt FROM employees WHERE dept_id IS NOT NULL GROUP BY dept_id ORDER BY dept_id LIMIT 2",
            fixtures()
        );

        assertEquals(List.of("dept_id", "cnt"), result.columns());
        assertEquals(List.of(1L, 2), result.rows().get(0));
        assertEquals(List.of(2L, 1), result.rows().get(1));
    }

    @Test
    void reportsMissingTablesThroughEngineErrors() {
        var ex = assertThrows(
            SqlExecutionEngine.SqlExecutionException.class,
            () -> executeCsv("SELECT * FROM no_such_table", fixtures())
        );

        assertTrue(ex.getMessage().contains("table not found: no_such_table"));
        var result = SqlCsvSource.tryExecuteCsv("SELECT * FROM no_such_table", fixtures());
        assertEquals(false, result.ok());
        assertNotNull(result.error());
    }

    @Test
    void coercesScalarValues() {
        assertNull(SqlCsvSource.coerce(""));
        assertEquals(true, SqlCsvSource.coerce("TRUE"));
        assertEquals(false, SqlCsvSource.coerce("false"));
        assertEquals(42L, SqlCsvSource.coerce("42"));
        assertEquals(-5L, SqlCsvSource.coerce("-5"));
        assertEquals(3.14, SqlCsvSource.coerce("3.14"));
        assertEquals("123abc", SqlCsvSource.coerce("123abc"));
        assertInstanceOf(String.class, SqlCsvSource.coerce("hello"));
    }
}
