//! Integration tests for the SQL execution engine.
//!
//! Uses an InMemorySource with employees and departments tables.
//!
//! # Data Model
//!
//! employees:
//!   id | name  | dept_id | salary | active
//!   1  | Alice | 1       | 90000  | true
//!   2  | Bob   | 2       | 75000  | true
//!   3  | Carol | 1       | 95000  | false
//!   4  | Dave  | NULL    | 60000  | true
//!
//! departments:
//!   id | name        | budget
//!   1  | Engineering | 500000
//!   2  | Marketing   | 200000

use std::collections::HashMap;

use coding_adventures_sql_execution_engine::{
    execute, execute_all, DataSource, ExecutionError, QueryResult, SqlPrimitive, SqlValue,
};

// ---------------------------------------------------------------------------
// In-memory test source
// ---------------------------------------------------------------------------

struct InMemorySource;

fn i(v: i64) -> SqlValue { Some(SqlPrimitive::Int(v)) }
fn s(v: &str) -> SqlValue { Some(SqlPrimitive::Text(v.to_string())) }
fn b(v: bool) -> SqlValue { Some(SqlPrimitive::Bool(v)) }
fn null() -> SqlValue { None }
fn f(v: f64) -> SqlValue { Some(SqlPrimitive::Float(v)) }

impl DataSource for InMemorySource {
    fn schema(&self, table_name: &str) -> Result<Vec<String>, ExecutionError> {
        match table_name {
            "employees" => Ok(vec![
                "id".to_string(), "name".to_string(), "dept_id".to_string(),
                "salary".to_string(), "active".to_string(),
            ]),
            "departments" => Ok(vec![
                "id".to_string(), "name".to_string(), "budget".to_string(),
            ]),
            other => Err(ExecutionError::TableNotFound(other.to_string())),
        }
    }

    fn scan(&self, table_name: &str) -> Result<Vec<HashMap<String, SqlValue>>, ExecutionError> {
        match table_name {
            "employees" => Ok(vec![
                row(&[("id", i(1)), ("name", s("Alice")), ("dept_id", i(1)),  ("salary", i(90000)), ("active", b(true))]),
                row(&[("id", i(2)), ("name", s("Bob")),   ("dept_id", i(2)),  ("salary", i(75000)), ("active", b(true))]),
                row(&[("id", i(3)), ("name", s("Carol")), ("dept_id", i(1)),  ("salary", i(95000)), ("active", b(false))]),
                row(&[("id", i(4)), ("name", s("Dave")),  ("dept_id", null()),("salary", i(60000)), ("active", b(true))]),
            ]),
            "departments" => Ok(vec![
                row(&[("id", i(1)), ("name", s("Engineering")), ("budget", i(500000))]),
                row(&[("id", i(2)), ("name", s("Marketing")),   ("budget", i(200000))]),
            ]),
            other => Err(ExecutionError::TableNotFound(other.to_string())),
        }
    }
}

fn row(entries: &[(&str, SqlValue)]) -> HashMap<String, SqlValue> {
    entries.iter().map(|(k, v)| (k.to_string(), v.clone())).collect()
}

fn run(sql: &str) -> QueryResult {
    execute(sql, &InMemorySource).expect("execution failed")
}

fn get_str(row: &HashMap<String, SqlValue>, col: &str) -> Option<String> {
    match row.get(col)? {
        Some(SqlPrimitive::Text(s)) => Some(s.clone()),
        _ => None,
    }
}

fn get_int(row: &HashMap<String, SqlValue>, col: &str) -> Option<i64> {
    match row.get(col)? {
        Some(SqlPrimitive::Int(i)) => Some(*i),
        Some(SqlPrimitive::Float(f)) => Some(*f as i64),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Test 1: SELECT *
// ---------------------------------------------------------------------------

#[test]
fn test_select_star_row_count() {
    let result = run("SELECT * FROM employees");
    assert_eq!(result.rows.len(), 4);
}

#[test]
fn test_select_star_includes_name_column() {
    let result = run("SELECT * FROM employees");
    assert!(result.columns.contains(&"name".to_string()));
}

// ---------------------------------------------------------------------------
// Test 2: SELECT specific columns
// ---------------------------------------------------------------------------

#[test]
fn test_select_specific_columns() {
    let result = run("SELECT id, name FROM employees");
    assert_eq!(result.columns, vec!["id".to_string(), "name".to_string()]);
    assert_eq!(result.rows.len(), 4);
}

// ---------------------------------------------------------------------------
// Test 3: AS alias
// ---------------------------------------------------------------------------

#[test]
fn test_as_alias() {
    let result = run("SELECT id, name AS employee_name FROM employees");
    assert!(result.columns.contains(&"employee_name".to_string()));
    assert!(!result.columns.contains(&"name".to_string()));
}

// ---------------------------------------------------------------------------
// Test 4: WHERE salary > N
// ---------------------------------------------------------------------------

#[test]
fn test_where_salary_greater_than() {
    let result = run("SELECT name FROM employees WHERE salary > 80000");
    let names: Vec<String> = result.rows.iter()
        .filter_map(|r| get_str(r, "name"))
        .collect();
    assert!(names.contains(&"Alice".to_string()));
    assert!(names.contains(&"Carol".to_string()));
    assert_eq!(names.len(), 2);
}

// ---------------------------------------------------------------------------
// Test 5: WHERE active = TRUE
// ---------------------------------------------------------------------------

#[test]
fn test_where_boolean() {
    let result = run("SELECT name FROM employees WHERE active = TRUE");
    let names: Vec<String> = result.rows.iter()
        .filter_map(|r| get_str(r, "name"))
        .collect();
    assert_eq!(names.len(), 3);
    assert!(!names.contains(&"Carol".to_string()));
}

// ---------------------------------------------------------------------------
// Test 6: WHERE IS NULL
// ---------------------------------------------------------------------------

#[test]
fn test_where_is_null() {
    let result = run("SELECT name FROM employees WHERE dept_id IS NULL");
    assert_eq!(result.rows.len(), 1);
    assert_eq!(get_str(&result.rows[0], "name"), Some("Dave".to_string()));
}

// ---------------------------------------------------------------------------
// Test 7: WHERE IS NOT NULL
// ---------------------------------------------------------------------------

#[test]
fn test_where_is_not_null() {
    let result = run("SELECT name FROM employees WHERE dept_id IS NOT NULL");
    assert_eq!(result.rows.len(), 3);
    let names: Vec<String> = result.rows.iter()
        .filter_map(|r| get_str(r, "name"))
        .collect();
    assert!(!names.contains(&"Dave".to_string()));
}

// ---------------------------------------------------------------------------
// Test 8: WHERE BETWEEN
// ---------------------------------------------------------------------------

#[test]
fn test_where_between() {
    let result = run("SELECT name FROM employees WHERE salary BETWEEN 70000 AND 90000");
    let names: Vec<String> = result.rows.iter()
        .filter_map(|r| get_str(r, "name"))
        .collect();
    assert!(names.contains(&"Alice".to_string()));
    assert!(names.contains(&"Bob".to_string()));
    assert_eq!(names.len(), 2);
}

// ---------------------------------------------------------------------------
// Test 9: WHERE IN
// ---------------------------------------------------------------------------

#[test]
fn test_where_in() {
    let result = run("SELECT name FROM employees WHERE id IN (1, 3)");
    let names: Vec<String> = result.rows.iter()
        .filter_map(|r| get_str(r, "name"))
        .collect();
    assert!(names.contains(&"Alice".to_string()));
    assert!(names.contains(&"Carol".to_string()));
    assert_eq!(names.len(), 2);
}

// ---------------------------------------------------------------------------
// Test 10: WHERE LIKE
// ---------------------------------------------------------------------------

#[test]
fn test_where_like() {
    let result = run("SELECT name FROM employees WHERE name LIKE 'A%'");
    assert_eq!(result.rows.len(), 1);
    assert_eq!(get_str(&result.rows[0], "name"), Some("Alice".to_string()));
}

// ---------------------------------------------------------------------------
// Test 11: WHERE AND / OR / NOT
// ---------------------------------------------------------------------------

#[test]
fn test_where_and() {
    let result = run("SELECT name FROM employees WHERE salary > 70000 AND active = TRUE");
    let names: Vec<String> = result.rows.iter()
        .filter_map(|r| get_str(r, "name"))
        .collect();
    assert_eq!(names.len(), 2);
    assert!(names.contains(&"Alice".to_string()));
    assert!(names.contains(&"Bob".to_string()));
}

#[test]
fn test_where_not() {
    let result = run("SELECT name FROM employees WHERE NOT active = TRUE");
    assert_eq!(result.rows.len(), 1);
    assert_eq!(get_str(&result.rows[0], "name"), Some("Carol".to_string()));
}

// ---------------------------------------------------------------------------
// Test 12: ORDER BY
// ---------------------------------------------------------------------------

#[test]
fn test_order_by_salary_desc() {
    let result = run("SELECT name FROM employees ORDER BY salary DESC");
    let names: Vec<String> = result.rows.iter()
        .filter_map(|r| get_str(r, "name"))
        .collect();
    assert_eq!(names[0], "Carol");
    assert_eq!(names[names.len() - 1], "Dave");
}

#[test]
fn test_order_by_name_asc() {
    let result = run("SELECT name FROM employees ORDER BY name ASC");
    let names: Vec<String> = result.rows.iter()
        .filter_map(|r| get_str(r, "name"))
        .collect();
    let mut sorted = names.clone();
    sorted.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));
    assert_eq!(names, sorted);
}

// ---------------------------------------------------------------------------
// Test 13: LIMIT and OFFSET
// ---------------------------------------------------------------------------

#[test]
fn test_limit() {
    let result = run("SELECT id FROM employees LIMIT 2");
    assert_eq!(result.rows.len(), 2);
}

#[test]
fn test_limit_offset() {
    let all = run("SELECT id FROM employees ORDER BY id ASC");
    let page = run("SELECT id FROM employees ORDER BY id ASC LIMIT 2 OFFSET 1");
    assert_eq!(page.rows.len(), 2);
    assert_eq!(get_int(&page.rows[0], "id"), get_int(&all.rows[1], "id"));
}

// ---------------------------------------------------------------------------
// Test 14: SELECT DISTINCT
// ---------------------------------------------------------------------------

#[test]
fn test_select_distinct() {
    let result = run("SELECT DISTINCT dept_id FROM employees");
    assert_eq!(result.rows.len(), 3); // 1, 2, NULL
}

// ---------------------------------------------------------------------------
// Test 15: INNER JOIN
// ---------------------------------------------------------------------------

#[test]
fn test_inner_join() {
    let result = run(
        "SELECT employees.name, departments.name \
         FROM employees INNER JOIN departments \
         ON employees.dept_id = departments.id"
    );
    // Dave (NULL dept_id) excluded
    assert_eq!(result.rows.len(), 3);
}

// ---------------------------------------------------------------------------
// Test 16: LEFT JOIN
// ---------------------------------------------------------------------------

#[test]
fn test_left_join_includes_dave() {
    let result = run(
        "SELECT employees.name \
         FROM employees LEFT JOIN departments \
         ON employees.dept_id = departments.id"
    );
    assert_eq!(result.rows.len(), 4);
    let names: Vec<String> = result.rows.iter()
        .filter_map(|r| get_str(r, "employees.name"))
        .collect();
    assert!(names.contains(&"Dave".to_string()));
}

#[test]
fn test_left_join_null_for_dave() {
    let result = run(
        "SELECT employees.name, departments.name AS dept_name \
         FROM employees LEFT JOIN departments \
         ON employees.dept_id = departments.id"
    );
    let dave = result.rows.iter()
        .find(|r| get_str(r, "employees.name") == Some("Dave".to_string()))
        .expect("Dave not found");
    assert_eq!(dave.get("dept_name"), Some(&None));
}

// ---------------------------------------------------------------------------
// Test 17: COUNT(*) and AVG
// ---------------------------------------------------------------------------

#[test]
fn test_count_star() {
    let result = run("SELECT COUNT(*) FROM employees");
    assert_eq!(result.rows.len(), 1);
    let val = result.rows[0].values().next().unwrap();
    assert_eq!(val, &i(4));
}

#[test]
fn test_avg_salary() {
    let result = run("SELECT AVG(salary) FROM employees");
    assert_eq!(result.rows.len(), 1);
    let val = result.rows[0].values().next().unwrap();
    let fval = match val {
        Some(SqlPrimitive::Float(f)) => *f,
        Some(SqlPrimitive::Int(i)) => *i as f64,
        _ => panic!("Expected numeric value"),
    };
    let expected = (90000.0 + 75000.0 + 95000.0 + 60000.0) / 4.0;
    assert!((fval - expected).abs() < 0.01);
}

// ---------------------------------------------------------------------------
// Test 18: GROUP BY
// ---------------------------------------------------------------------------

#[test]
fn test_group_by_count() {
    let result = run("SELECT dept_id, COUNT(*) FROM employees GROUP BY dept_id");
    assert_eq!(result.rows.len(), 3);
}

#[test]
fn test_group_by_sum() {
    let result = run(
        "SELECT dept_id, SUM(salary) FROM employees \
         WHERE dept_id IS NOT NULL GROUP BY dept_id"
    );
    let dept1 = result.rows.iter()
        .find(|r| get_int(r, "dept_id") == Some(1))
        .expect("dept 1 not found");
    let sum1 = match dept1.get("SUM(salary)") {
        Some(Some(SqlPrimitive::Int(i))) => *i,
        Some(Some(SqlPrimitive::Float(f))) => *f as i64,
        _ => panic!("unexpected value"),
    };
    assert_eq!(sum1, 185000);
}

// ---------------------------------------------------------------------------
// Test 19: HAVING
// ---------------------------------------------------------------------------

#[test]
fn test_having() {
    let result = run(
        "SELECT dept_id, SUM(salary) FROM employees \
         WHERE dept_id IS NOT NULL \
         GROUP BY dept_id \
         HAVING SUM(salary) > 100000"
    );
    assert_eq!(result.rows.len(), 1);
    assert_eq!(get_int(&result.rows[0], "dept_id"), Some(1));
}

// ---------------------------------------------------------------------------
// Test 20: Arithmetic
// ---------------------------------------------------------------------------

#[test]
fn test_arithmetic_multiply() {
    let result = run("SELECT salary * 1.1 AS adjusted FROM employees WHERE id = 1");
    assert_eq!(result.rows.len(), 1);
    let adj = match result.rows[0].get("adjusted") {
        Some(Some(SqlPrimitive::Float(f))) => *f,
        Some(Some(SqlPrimitive::Int(i))) => *i as f64,
        other => panic!("Unexpected value: {:?}", other),
    };
    assert!((adj - 99000.0).abs() < 1.0);
}

// ---------------------------------------------------------------------------
// Test 21: TableNotFoundError
// ---------------------------------------------------------------------------

#[test]
fn test_table_not_found() {
    let result = execute("SELECT * FROM nonexistent", &InMemorySource);
    match result {
        Err(ExecutionError::TableNotFound(name)) => assert_eq!(name, "nonexistent"),
        other => panic!("Expected TableNotFound, got {:?}", other),
    }
}

// ---------------------------------------------------------------------------
// Test 22: ColumnNotFoundError
// ---------------------------------------------------------------------------

#[test]
fn test_column_not_found() {
    let result = execute("SELECT id FROM employees WHERE fake_col = 1", &InMemorySource);
    assert!(matches!(result, Err(ExecutionError::ColumnNotFound(_))));
}

// ---------------------------------------------------------------------------
// Test 23: execute_all
// ---------------------------------------------------------------------------

#[test]
fn test_execute_all() {
    let results = execute_all(
        "SELECT id FROM employees; SELECT id FROM departments",
        &InMemorySource,
    ).expect("execute_all failed");
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].rows.len(), 4);
    assert_eq!(results[1].rows.len(), 2);
}

// ---------------------------------------------------------------------------
// Test 24: MIN / MAX
// ---------------------------------------------------------------------------

#[test]
fn test_min_salary() {
    let result = run("SELECT MIN(salary) FROM employees");
    let val = result.rows[0].values().next().unwrap();
    assert_eq!(val, &i(60000));
}

#[test]
fn test_max_salary() {
    let result = run("SELECT MAX(salary) FROM employees");
    let val = result.rows[0].values().next().unwrap();
    assert_eq!(val, &i(95000));
}

// ---------------------------------------------------------------------------
// Test 25: COUNT(col) skips NULLs
// ---------------------------------------------------------------------------

#[test]
fn test_count_column_skips_nulls() {
    let result = run("SELECT COUNT(dept_id) FROM employees");
    let val = result.rows[0].values().next().unwrap();
    assert_eq!(val, &i(3)); // Dave has NULL dept_id
}
