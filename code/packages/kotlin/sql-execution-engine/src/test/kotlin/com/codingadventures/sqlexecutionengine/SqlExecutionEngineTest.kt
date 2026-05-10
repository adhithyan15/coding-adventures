package com.codingadventures.sqlexecutionengine

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertFalse
import kotlin.test.assertNotNull

class SqlExecutionEngineTest {
    private fun dataSource(): InMemoryDataSource =
        InMemoryDataSource()
            .addTable(
                "employees",
                listOf("id", "name", "dept", "salary", "active"),
                listOf(
                    row("id", 1L, "name", "Alice", "dept", "Engineering", "salary", 95000L, "active", true),
                    row("id", 2L, "name", "Bob", "dept", "Marketing", "salary", 72000L, "active", true),
                    row("id", 3L, "name", "Carol", "dept", "Engineering", "salary", 88000L, "active", false),
                    row("id", 4L, "name", "Dave", "dept", null, "salary", 60000L, "active", true),
                    row("id", 5L, "name", "Eve", "dept", "HR", "salary", 70000L, "active", false)
                )
            )
            .addTable(
                "departments",
                listOf("dept", "budget"),
                listOf(
                    row("dept", "Engineering", "budget", 500000L),
                    row("dept", "Marketing", "budget", 200000L),
                    row("dept", "HR", "budget", 150000L)
                )
            )

    @Test
    fun scansInMemoryTables() {
        val source = dataSource()
        assertEquals(listOf("id", "name", "dept", "salary", "active"), source.schema("employees"))
        assertEquals(5, source.scan("employees").size)
        assertFailsWith<SqlExecutionException> { source.schema("missing") }
    }

    @Test
    fun selectsAndFiltersRows() {
        val result = SqlExecutionEngine.execute(
            "SELECT name, salary FROM employees WHERE active = true AND salary >= 70000 ORDER BY salary DESC",
            dataSource()
        )
        assertEquals(listOf("name", "salary"), result.columns)
        assertEquals(listOf("Alice", 95000L), result.rows[0])
        assertEquals(listOf("Bob", 72000L), result.rows[1])
    }

    @Test
    fun supportsNullPredicatesAndLike() {
        val nullResult = SqlExecutionEngine.execute("SELECT name FROM employees WHERE dept IS NULL", dataSource())
        assertEquals(listOf(listOf("Dave")), nullResult.rows)

        val likeResult = SqlExecutionEngine.execute("SELECT name FROM employees WHERE name LIKE 'A%'", dataSource())
        assertEquals(listOf(listOf("Alice")), likeResult.rows)
    }

    @Test
    fun supportsJoins() {
        val result = SqlExecutionEngine.execute(
            "SELECT e.name, d.budget FROM employees AS e INNER JOIN departments AS d ON e.dept = d.dept ORDER BY e.id",
            dataSource()
        )
        assertEquals(listOf("name", "budget"), result.columns)
        assertEquals(4, result.rows.size)
        assertEquals(listOf("Alice", 500000L), result.rows[0])
        assertEquals(listOf("Eve", 150000L), result.rows[3])
    }

    @Test
    fun supportsGroupingAndAggregates() {
        val result = SqlExecutionEngine.execute(
            "SELECT dept, COUNT(*) AS cnt, SUM(salary) AS total FROM employees WHERE dept IS NOT NULL GROUP BY dept HAVING COUNT(*) >= 1 ORDER BY dept",
            dataSource()
        )
        assertEquals(listOf("dept", "cnt", "total"), result.columns)
        assertEquals(listOf("Engineering", 2, 183000.0), result.rows[0])
        assertEquals(listOf("HR", 1, 70000.0), result.rows[1])
        assertEquals(listOf("Marketing", 1, 72000.0), result.rows[2])
    }

    @Test
    fun supportsDistinctLimitAndOffset() {
        val result = SqlExecutionEngine.execute(
            "SELECT DISTINCT dept FROM employees WHERE dept IS NOT NULL ORDER BY dept LIMIT 2 OFFSET 1",
            dataSource()
        )
        assertEquals(listOf("dept"), result.columns)
        assertEquals(listOf(listOf("HR"), listOf("Marketing")), result.rows)
    }

    @Test
    fun reportsErrorsThroughTryExecute() {
        val result = SqlExecutionEngine.tryExecute("SELECT * FROM ghosts", dataSource())
        assertFalse(result.ok)
        assertNotNull(result.error)
    }

    @Test
    fun selectStarUsesBareColumns() {
        val result = SqlExecutionEngine.execute("SELECT * FROM employees WHERE id = 1", dataSource())
        assertEquals(listOf("active", "dept", "id", "name", "salary"), result.columns)
        assertEquals(listOf(true, "Engineering", 1L, "Alice", 95000L), result.rows[0])
    }

    private fun row(vararg pairs: Any?): Map<String, Any?> {
        val row = linkedMapOf<String, Any?>()
        var index = 0
        while (index < pairs.size) {
            row[pairs[index] as String] = pairs[index + 1]
            index += 2
        }
        return row
    }
}
