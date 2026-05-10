package com.codingadventures.sqlcsvsource

import com.codingadventures.sqlexecutionengine.SqlExecutionException
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertIs
import kotlin.test.assertNotNull
import kotlin.test.assertNull
import kotlin.test.assertTrue
import java.nio.file.Path

class SqlCsvSourceTest {
    private fun fixtures(): Path {
        val resource = checkNotNull(javaClass.getResource("/fixtures/employees.csv"))
        return Path.of(resource.toURI()).parent
    }

    @Test
    fun exposesSchemaInHeaderOrder() {
        val source = CsvDataSource(fixtures())

        assertEquals(listOf("id", "name", "dept_id", "salary", "active"), source.schema("employees"))
        assertEquals(listOf("id", "name", "budget"), source.schema("departments"))
    }

    @Test
    fun scansRowsWithCoercedValues() {
        val rows = CsvDataSource(fixtures()).scan("employees")

        assertEquals(4, rows.size)
        val alice = rows[0]
        assertEquals(1L, alice["id"])
        assertEquals("Alice", alice["name"])
        assertEquals(90000L, alice["salary"])
        assertEquals(true, alice["active"])
        assertNull(rows[3]["dept_id"])
    }

    @Test
    fun executesSelectsAgainstCsvFiles() {
        val result = SqlCsvSource.executeCsv(
            "SELECT name, salary FROM employees WHERE active = true AND salary > 70000 ORDER BY salary DESC",
            fixtures()
        )

        assertEquals(listOf("name", "salary"), result.columns)
        assertEquals(listOf("Alice", 90000L), result.rows[0])
        assertEquals(listOf("Bob", 75000L), result.rows[1])
    }

    @Test
    fun supportsNullPredicates() {
        val result = SqlCsvSource.executeCsv("SELECT name FROM employees WHERE dept_id IS NULL", fixtures())

        assertEquals(listOf(listOf("Dave")), result.rows)
    }

    @Test
    fun supportsJoinsAcrossCsvFiles() {
        val result = SqlCsvSource.executeCsv(
            """
            SELECT e.name AS emp_name, d.name AS dept_name
            FROM employees AS e
            INNER JOIN departments AS d ON e.dept_id = d.id
            ORDER BY e.id
            """.trimIndent(),
            fixtures()
        )

        assertEquals(listOf("emp_name", "dept_name"), result.columns)
        assertEquals(listOf("Alice", "Engineering"), result.rows[0])
        assertEquals(listOf("Bob", "Marketing"), result.rows[1])
        assertEquals(listOf("Carol", "Engineering"), result.rows[2])
    }

    @Test
    fun supportsGroupingAggregatesLimitAndOffset() {
        val result = SqlCsvSource.executeCsv(
            "SELECT dept_id, COUNT(*) AS cnt FROM employees WHERE dept_id IS NOT NULL GROUP BY dept_id ORDER BY dept_id LIMIT 2",
            fixtures()
        )

        assertEquals(listOf("dept_id", "cnt"), result.columns)
        assertEquals(listOf(1L, 2), result.rows[0])
        assertEquals(listOf(2L, 1), result.rows[1])
    }

    @Test
    fun reportsMissingTablesThroughEngineErrors() {
        val ex = assertFailsWith<SqlExecutionException> {
            SqlCsvSource.executeCsv("SELECT * FROM no_such_table", fixtures())
        }

        assertTrue(ex.message!!.contains("table not found: no_such_table"))
        val result = SqlCsvSource.tryExecuteCsv("SELECT * FROM no_such_table", fixtures())
        assertEquals(false, result.ok)
        assertNotNull(result.error)
    }

    @Test
    fun coercesScalarValues() {
        assertNull(CsvDataSource.coerce(""))
        assertEquals(true, CsvDataSource.coerce("TRUE"))
        assertEquals(false, CsvDataSource.coerce("false"))
        assertEquals(42L, CsvDataSource.coerce("42"))
        assertEquals(-5L, CsvDataSource.coerce("-5"))
        assertEquals(3.14, CsvDataSource.coerce("3.14"))
        assertEquals("123abc", CsvDataSource.coerce("123abc"))
        assertIs<String>(CsvDataSource.coerce("hello"))
    }
}
