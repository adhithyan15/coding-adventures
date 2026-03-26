//! Integration tests for sql-csv-source.
//!
//! These tests run end-to-end against real CSV fixture files, exercising
//! the full pipeline:
//!
//! ```text
//! CSV files on disk → CsvDataSource → sql-execution-engine → QueryResult
//! ```
//!
//! Fixture data:
//!
//! ```text
//! employees.csv
//!   id | name  | dept_id | salary | active
//!   1  | Alice | 1       | 90000  | true
//!   2  | Bob   | 2       | 75000  | true
//!   3  | Carol | 1       | 95000  | false
//!   4  | Dave  | (null)  | 60000  | true
//!
//! departments.csv
//!   id | name        | budget
//!   1  | Engineering | 500000
//!   2  | Marketing   | 200000
//! ```

use std::path::PathBuf;

use coding_adventures_sql_csv_source::CsvDataSource;
use coding_adventures_sql_execution_engine::{
    execute, DataSource, ExecutionError, SqlPrimitive,
};

/// Return the path to the test fixtures directory.
///
/// `CARGO_MANIFEST_DIR` is set by cargo to the crate root at compile time.
/// This is the reliable way to find test fixtures regardless of the working
/// directory from which tests are run.
fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
}

fn source() -> CsvDataSource {
    CsvDataSource::new(fixtures())
}

// ── schema() tests ────────────────────────────────────────────────────────────

#[test]
fn test_employees_schema() {
    let cols = source().schema("employees").unwrap();
    assert_eq!(
        cols,
        vec!["id", "name", "dept_id", "salary", "active"]
    );
}

#[test]
fn test_departments_schema() {
    let cols = source().schema("departments").unwrap();
    assert_eq!(cols, vec!["id", "name", "budget"]);
}

#[test]
fn test_schema_unknown_table() {
    let err = source().schema("nonexistent").unwrap_err();
    assert!(matches!(err, ExecutionError::TableNotFound(ref name) if name == "nonexistent"));
}

// ── scan() tests ──────────────────────────────────────────────────────────────

#[test]
fn test_scan_employees_count() {
    let rows = source().scan("employees").unwrap();
    assert_eq!(rows.len(), 4);
}

#[test]
fn test_scan_alice_types() {
    let rows = source().scan("employees").unwrap();
    // Find Alice by name (HashMap order is not guaranteed).
    let alice = rows
        .iter()
        .find(|r| r.get("name") == Some(&Some(SqlPrimitive::Text("Alice".to_string()))))
        .expect("Alice not found");

    assert_eq!(alice.get("id"), Some(&Some(SqlPrimitive::Int(1))));
    assert_eq!(alice.get("dept_id"), Some(&Some(SqlPrimitive::Int(1))));
    assert_eq!(alice.get("salary"), Some(&Some(SqlPrimitive::Int(90000))));
    assert_eq!(alice.get("active"), Some(&Some(SqlPrimitive::Bool(true))));
}

#[test]
fn test_scan_dave_dept_id_is_null() {
    // Dave has an empty dept_id field — must coerce to None (SQL NULL).
    let rows = source().scan("employees").unwrap();
    let dave = rows
        .iter()
        .find(|r| r.get("name") == Some(&Some(SqlPrimitive::Text("Dave".to_string()))))
        .expect("Dave not found");

    assert_eq!(dave.get("dept_id"), Some(&None));
}

#[test]
fn test_scan_carol_active_is_false() {
    let rows = source().scan("employees").unwrap();
    let carol = rows
        .iter()
        .find(|r| r.get("name") == Some(&Some(SqlPrimitive::Text("Carol".to_string()))))
        .expect("Carol not found");

    assert_eq!(carol.get("active"), Some(&Some(SqlPrimitive::Bool(false))));
}

#[test]
fn test_scan_departments_budget_is_int() {
    let rows = source().scan("departments").unwrap();
    let eng = rows
        .iter()
        .find(|r| {
            r.get("name")
                == Some(&Some(SqlPrimitive::Text("Engineering".to_string())))
        })
        .expect("Engineering not found");

    assert_eq!(eng.get("budget"), Some(&Some(SqlPrimitive::Int(500_000))));
}

#[test]
fn test_scan_unknown_table() {
    let err = source().scan("ghosts").unwrap_err();
    assert!(matches!(err, ExecutionError::TableNotFound(_)));
}

// ── End-to-end SQL query tests ────────────────────────────────────────────────

/// Test 1: SELECT * FROM employees — 4 rows, types coerced.
#[test]
fn test_select_star_employees() {
    let src = source();
    let result = execute("SELECT * FROM employees", &src).unwrap();
    assert_eq!(result.columns, vec!["id", "name", "dept_id", "salary", "active"]);
    assert_eq!(result.rows.len(), 4);
}

/// Test 2: SELECT name WHERE active = true — Alice, Bob, Dave.
#[test]
fn test_select_active_employees() {
    let src = source();
    let result = execute(
        "SELECT name FROM employees WHERE active = true",
        &src,
    )
    .unwrap();

    let mut names: Vec<String> = result
        .rows
        .iter()
        .filter_map(|r| {
            r.get("name").and_then(|v| {
                if let Some(SqlPrimitive::Text(s)) = v {
                    Some(s.clone())
                } else {
                    None
                }
            })
        })
        .collect();
    names.sort();

    assert_eq!(names, vec!["Alice", "Bob", "Dave"]);
}

/// Test 3: SELECT WHERE dept_id IS NULL — only Dave.
#[test]
fn test_select_where_null() {
    let src = source();
    let result = execute(
        "SELECT * FROM employees WHERE dept_id IS NULL",
        &src,
    )
    .unwrap();

    assert_eq!(result.rows.len(), 1);
    assert_eq!(
        result.rows[0].get("name"),
        Some(&Some(SqlPrimitive::Text("Dave".to_string())))
    );
}

/// Test 4: INNER JOIN — 3 rows (Dave excluded, NULL dept_id).
#[test]
fn test_inner_join() {
    let src = source();
    let result = execute(
        "SELECT e.name, d.name \
         FROM employees AS e \
         INNER JOIN departments AS d ON e.dept_id = d.id",
        &src,
    )
    .unwrap();

    assert_eq!(result.rows.len(), 3);

    let mut emp_names: Vec<String> = result
        .rows
        .iter()
        .filter_map(|r| {
            r.get("e.name").and_then(|v| {
                if let Some(SqlPrimitive::Text(s)) = v {
                    Some(s.clone())
                } else {
                    None
                }
            })
        })
        .collect();
    emp_names.sort();

    assert_eq!(emp_names, vec!["Alice", "Bob", "Carol"]);
}

/// Test 5: GROUP BY dept_id — 3 groups including NULL.
#[test]
fn test_group_by_dept_id() {
    let src = source();
    let result = execute(
        "SELECT dept_id, COUNT(*) AS cnt FROM employees GROUP BY dept_id",
        &src,
    )
    .unwrap();

    // Three groups: dept_id=1 (Alice+Carol), dept_id=2 (Bob), NULL (Dave).
    assert_eq!(result.rows.len(), 3);

    // Find the group for dept_id = 1 and check its count.
    let dept1 = result
        .rows
        .iter()
        .find(|r| r.get("dept_id") == Some(&Some(SqlPrimitive::Int(1))))
        .expect("dept_id=1 group not found");

    assert_eq!(dept1.get("cnt"), Some(&Some(SqlPrimitive::Int(2))));
}

/// Test 6: ORDER BY salary DESC LIMIT 2 — Carol (95000), Alice (90000).
#[test]
fn test_order_by_salary_desc_limit_2() {
    let src = source();
    let result = execute(
        "SELECT name, salary FROM employees ORDER BY salary DESC LIMIT 2",
        &src,
    )
    .unwrap();

    assert_eq!(result.rows.len(), 2);
    assert_eq!(
        result.rows[0].get("name"),
        Some(&Some(SqlPrimitive::Text("Carol".to_string())))
    );
    assert_eq!(
        result.rows[0].get("salary"),
        Some(&Some(SqlPrimitive::Int(95000)))
    );
    assert_eq!(
        result.rows[1].get("name"),
        Some(&Some(SqlPrimitive::Text("Alice".to_string())))
    );
    assert_eq!(
        result.rows[1].get("salary"),
        Some(&Some(SqlPrimitive::Int(90000)))
    );
}

/// Test 7: Unknown table → ExecutionError::TableNotFound.
#[test]
fn test_unknown_table_raises() {
    let src = source();
    let err = execute("SELECT * FROM ghosts", &src).unwrap_err();
    assert!(matches!(err, ExecutionError::TableNotFound(_)));
}
