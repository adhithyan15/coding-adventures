use coding_adventures_mini_sqlite::{
    boolean, connect, int, text, ConnectOptions, MiniSqliteError, API_LEVEL, PARAM_STYLE,
    THREAD_SAFETY,
};

fn must_connect() -> coding_adventures_mini_sqlite::Connection {
    connect(":memory:").expect("connect failed")
}

fn must_execute(
    conn: &coding_adventures_mini_sqlite::Connection,
    sql: &str,
    params: &[coding_adventures_mini_sqlite::SqlValue],
) -> coding_adventures_mini_sqlite::Cursor {
    conn.execute(sql, params).expect("execute failed")
}

#[test]
fn exposes_module_constants() {
    assert_eq!(API_LEVEL, "2.0");
    assert_eq!(THREAD_SAFETY, 1);
    assert_eq!(PARAM_STYLE, "qmark");
}

#[test]
fn creates_inserts_and_selects_rows() {
    let conn = must_connect();
    must_execute(
        &conn,
        "CREATE TABLE users (id INTEGER, name TEXT, active BOOLEAN)",
        &[],
    );
    conn.executemany(
        "INSERT INTO users VALUES (?, ?, ?)",
        &[
            vec![int(1), text("Alice"), boolean(true)],
            vec![int(2), text("Bob"), boolean(false)],
            vec![int(3), text("Carol"), boolean(true)],
        ],
    )
    .expect("executemany failed");

    let mut cursor = must_execute(
        &conn,
        "SELECT name FROM users WHERE active = ? ORDER BY id ASC",
        &[boolean(true)],
    );
    assert_eq!(cursor.description[0].name, "name");
    assert_eq!(
        cursor.fetchall(),
        vec![vec![text("Alice")], vec![text("Carol")]]
    );
}

#[test]
fn fetchone_fetchmany_and_fetchall_advance_the_cursor() {
    let conn = must_connect();
    must_execute(&conn, "CREATE TABLE nums (n INTEGER)", &[]);
    conn.executemany(
        "INSERT INTO nums VALUES (?)",
        &[vec![int(1)], vec![int(2)], vec![int(3)]],
    )
    .expect("executemany failed");

    let mut cursor = must_execute(&conn, "SELECT n FROM nums ORDER BY n ASC", &[]);
    assert_eq!(cursor.fetchone(), Some(vec![int(1)]));
    assert_eq!(cursor.fetchmany(1), vec![vec![int(2)]]);
    assert_eq!(cursor.fetchall(), vec![vec![int(3)]]);
    assert_eq!(cursor.fetchone(), None);
}

#[test]
fn updates_rows_through_where_clause() {
    let conn = must_connect();
    must_execute(&conn, "CREATE TABLE users (id INTEGER, name TEXT)", &[]);
    conn.executemany(
        "INSERT INTO users VALUES (?, ?)",
        &[vec![int(1), text("Alice")], vec![int(2), text("Bob")]],
    )
    .expect("executemany failed");

    let cursor = must_execute(
        &conn,
        "UPDATE users SET name = ? WHERE id = ?",
        &[text("Bobby"), int(2)],
    );
    assert_eq!(cursor.rowcount(), 1);

    let mut cursor = must_execute(&conn, "SELECT name FROM users ORDER BY id ASC", &[]);
    assert_eq!(
        cursor.fetchall(),
        vec![vec![text("Alice")], vec![text("Bobby")]]
    );
}

#[test]
fn deletes_rows_through_where_clause() {
    let conn = must_connect();
    must_execute(&conn, "CREATE TABLE users (id INTEGER, name TEXT)", &[]);
    conn.executemany(
        "INSERT INTO users VALUES (?, ?)",
        &[
            vec![int(1), text("Alice")],
            vec![int(2), text("Bob")],
            vec![int(3), text("Carol")],
        ],
    )
    .expect("executemany failed");

    let cursor = must_execute(
        &conn,
        "DELETE FROM users WHERE id IN (?, ?)",
        &[int(1), int(3)],
    );
    assert_eq!(cursor.rowcount(), 2);

    let mut cursor = must_execute(&conn, "SELECT id, name FROM users", &[]);
    assert_eq!(cursor.fetchall(), vec![vec![int(2), text("Bob")]]);
}

#[test]
fn rollback_restores_snapshot() {
    let conn = must_connect();
    must_execute(&conn, "CREATE TABLE users (id INTEGER, name TEXT)", &[]);
    conn.commit().expect("commit failed");
    must_execute(
        &conn,
        "INSERT INTO users VALUES (?, ?)",
        &[int(1), text("Alice")],
    );
    conn.rollback().expect("rollback failed");

    let mut cursor = must_execute(&conn, "SELECT * FROM users", &[]);
    assert!(cursor.fetchall().is_empty());
}

#[test]
fn commit_keeps_changes() {
    let conn = must_connect();
    must_execute(&conn, "CREATE TABLE users (id INTEGER, name TEXT)", &[]);
    must_execute(
        &conn,
        "INSERT INTO users VALUES (?, ?)",
        &[int(1), text("Alice")],
    );
    conn.commit().expect("commit failed");
    conn.rollback().expect("rollback failed");

    let mut cursor = must_execute(&conn, "SELECT name FROM users", &[]);
    assert_eq!(cursor.fetchall(), vec![vec![text("Alice")]]);
}

#[test]
fn autocommit_disables_snapshot_rollback() {
    let conn = coding_adventures_mini_sqlite::connect_with_options(
        ":memory:",
        ConnectOptions { autocommit: true },
    )
    .expect("connect failed");
    must_execute(&conn, "CREATE TABLE users (id INTEGER)", &[]);
    must_execute(&conn, "INSERT INTO users VALUES (?)", &[int(1)]);
    conn.rollback().expect("rollback failed");

    let mut cursor = must_execute(&conn, "SELECT id FROM users", &[]);
    assert_eq!(cursor.fetchall(), vec![vec![int(1)]]);
}

#[test]
fn drops_tables() {
    let conn = must_connect();
    must_execute(&conn, "CREATE TABLE users (id INTEGER)", &[]);
    must_execute(&conn, "DROP TABLE users", &[]);

    let err = conn.execute("SELECT * FROM users", &[]).unwrap_err();
    assert!(matches!(err, MiniSqliteError::OperationalError(_)));
}

#[test]
fn rejects_wrong_parameter_counts() {
    let conn = must_connect();
    let err = conn.execute("SELECT ? FROM t", &[]).unwrap_err();
    assert!(matches!(err, MiniSqliteError::ProgrammingError(_)));

    let err = conn.execute("SELECT 1 FROM t", &[int(1)]).unwrap_err();
    assert!(matches!(err, MiniSqliteError::ProgrammingError(_)));
}

#[test]
fn rejects_file_connections_for_level_zero() {
    let err = connect("app.db").unwrap_err();
    assert!(matches!(err, MiniSqliteError::NotSupportedError(_)));
}
