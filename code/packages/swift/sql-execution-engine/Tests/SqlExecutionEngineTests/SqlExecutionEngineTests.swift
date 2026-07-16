import SqlExecutionEngine
import XCTest

final class SqlExecutionEngineTests: XCTestCase {
  func testInMemoryDataSourceCopiesSchemaAndReportsMissingTables() throws {
    let source = makeSource()
    XCTAssertEqual(try source.schema("employees"), ["id", "name", "dept", "salary", "active"])
    XCTAssertEqual(try source.scan("employees").count, 5)
    XCTAssertThrowsError(try source.schema("missing")) { error in
      XCTAssertEqual(error as? SqlExecutionError, .tableNotFound("missing"))
    }
  }

  func testSelectFilterArithmeticAndOrder() throws {
    let result = try SqlEngine.execute(
      "SELECT name, salary * 1.5 AS adjusted FROM employees "
        + "WHERE active = true AND salary >= 70000 ORDER BY salary DESC",
      dataSource: makeSource()
    )
    XCTAssertEqual(result.columns, ["name", "adjusted"])
    XCTAssertEqual(
      result.rows,
      [
        [.text("Alice"), .real(142_500)],
        [.text("Bob"), .real(108_000)],
      ])
  }

  func testNullBetweenInAndLikePredicates() throws {
    let source = makeSource()
    XCTAssertEqual(
      try SqlEngine.execute("SELECT name FROM employees WHERE dept IS NULL", dataSource: source)
        .rows,
      [[.text("Dave")]]
    )
    XCTAssertEqual(
      try SqlEngine.execute(
        "SELECT name FROM employees WHERE salary BETWEEN 70000 AND 90000 "
          + "AND dept IN ('Engineering', 'Marketing') AND name LIKE '_o%'",
        dataSource: source
      ).rows,
      [[.text("Bob")]]
    )
  }

  func testInnerAndOuterJoins() throws {
    let source = makeSource()
    let inner = try SqlEngine.execute(
      "SELECT e.name, d.budget FROM employees AS e "
        + "INNER JOIN departments AS d ON e.dept = d.dept ORDER BY e.id",
      dataSource: source
    )
    XCTAssertEqual(inner.rows.count, 4)
    XCTAssertEqual(inner.rows.first, [.text("Alice"), .integer(500_000)])

    let left = try SqlEngine.execute(
      "SELECT e.name, d.budget FROM employees e LEFT JOIN departments d "
        + "ON e.dept = d.dept ORDER BY e.id",
      dataSource: source
    )
    XCTAssertEqual(left.rows[3], [.text("Dave"), .null])

    let full = try SqlEngine.execute(
      "SELECT e.name, d.dept FROM employees e FULL JOIN departments d "
        + "ON e.dept = d.dept WHERE e.name IS NULL OR d.dept IS NULL",
      dataSource: source
    )
    XCTAssertEqual(full.rows, [[.text("Dave"), .null], [.null, .text("Sales")]])

    let right = try SqlEngine.execute(
      "SELECT e.name, d.dept FROM employees e RIGHT JOIN departments d "
        + "ON e.dept = d.dept WHERE e.name IS NULL",
      dataSource: source
    )
    XCTAssertEqual(right.rows, [[.null, .text("Sales")]])

    let cross = try SqlEngine.execute(
      "SELECT e.id, d.dept FROM employees e CROSS JOIN departments d",
      dataSource: source
    )
    XCTAssertEqual(cross.rows.count, 20)

    XCTAssertThrowsError(
      try SqlEngine.execute(
        "SELECT dept FROM employees e JOIN departments d ON e.dept = d.dept",
        dataSource: source
      )
    ) { error in
      XCTAssertEqual(error as? SqlExecutionError, .ambiguousColumn("dept"))
    }
  }

  func testGroupingAggregatesAndHaving() throws {
    let result = try SqlEngine.execute(
      "SELECT dept, COUNT(*) AS cnt, SUM(salary) AS total, AVG(salary) AS average "
        + "FROM employees WHERE dept IS NOT NULL GROUP BY dept "
        + "HAVING COUNT(*) >= 1 ORDER BY dept",
      dataSource: makeSource()
    )
    XCTAssertEqual(result.columns, ["dept", "cnt", "total", "average"])
    XCTAssertEqual(
      result.rows,
      [
        [.text("Engineering"), .integer(2), .real(183_000), .real(91_500)],
        [.text("HR"), .integer(1), .real(70_000), .real(70_000)],
        [.text("Marketing"), .integer(1), .real(72_000), .real(72_000)],
      ])
  }

  func testDistinctLimitOffsetAndScalarFunctions() throws {
    let result = try SqlEngine.execute(
      "SELECT DISTINCT UPPER(dept) AS department FROM employees "
        + "WHERE dept IS NOT NULL ORDER BY dept LIMIT 2 OFFSET 1",
      dataSource: makeSource()
    )
    XCTAssertEqual(result.rows, [[.text("HR")], [.text("MARKETING")]])
  }

  func testSelectStarReturnsBareColumns() throws {
    let result = try SqlEngine.execute(
      "SELECT * FROM employees WHERE id = 1",
      dataSource: makeSource()
    )
    XCTAssertEqual(result.columns, ["active", "dept", "id", "name", "salary"])
    XCTAssertEqual(
      result.rows,
      [
        [
          .boolean(true), .text("Engineering"), .integer(1), .text("Alice"), .integer(95_000),
        ]
      ])
  }

  func testCommentsCaseInsensitivityAndQuotedIdentifiers() throws {
    let result = try SqlEngine.execute(
      "/* lead */ select `name` from employees -- tail\nwhere id = 2;",
      dataSource: makeSource()
    )
    XCTAssertEqual(result.rows, [[.text("Bob")]])
  }

  func testTryExecuteAndSpecificErrors() throws {
    let failed = SqlEngine.tryExecute("SELECT * FROM ghosts", dataSource: makeSource())
    XCTAssertFalse(failed.isSuccess)
    XCTAssertEqual(failed.error, "table not found: ghosts")

    XCTAssertThrowsError(
      try SqlEngine.execute("SELECT salary / 0 FROM employees", dataSource: makeSource())
    ) { error in
      XCTAssertEqual(error as? SqlExecutionError, .divisionByZero)
    }
  }

  private func makeSource() -> InMemoryDataSource {
    InMemoryDataSource()
      .addTable(
        "employees",
        schema: ["id", "name", "dept", "salary", "active"],
        rows: [
          ["id": 1, "name": "Alice", "dept": "Engineering", "salary": 95_000, "active": true],
          ["id": 2, "name": "Bob", "dept": "Marketing", "salary": 72_000, "active": true],
          ["id": 3, "name": "Carol", "dept": "Engineering", "salary": 88_000, "active": false],
          ["id": 4, "name": "Dave", "dept": nil, "salary": 60_000, "active": true],
          ["id": 5, "name": "Eve", "dept": "HR", "salary": 70_000, "active": false],
        ]
      )
      .addTable(
        "departments",
        schema: ["dept", "budget"],
        rows: [
          ["dept": "Engineering", "budget": 500_000],
          ["dept": "Marketing", "budget": 200_000],
          ["dept": "HR", "budget": 150_000],
          ["dept": "Sales", "budget": 100_000],
        ]
      )
  }
}
