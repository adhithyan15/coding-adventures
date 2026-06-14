use std::collections::BTreeMap;

use coding_adventures_sql_backend::{
    backend_as_schema_provider, compare_sql_values, Backend, BackendError, ColumnDef, Cursor,
    InMemoryBackend, IndexDef, ListCursor, ListRowIterator, Row, RowIterator, SchemaProvider,
    SqlValue, TriggerDef,
};

#[test]
fn sql_values_classify_and_compare() {
    assert_eq!(SqlValue::Null.type_name(), "NULL");
    assert_eq!(SqlValue::Bool(true).type_name(), "BOOLEAN");
    assert_eq!(SqlValue::Int(42).type_name(), "INTEGER");
    assert_eq!(SqlValue::Float(1.5).type_name(), "REAL");
    assert_eq!(SqlValue::Text("hi".to_string()).type_name(), "TEXT");
    assert_eq!(SqlValue::Blob(vec![1, 2]).type_name(), "BLOB");
    assert!(compare_sql_values(&SqlValue::Null, &SqlValue::Int(0)).is_lt());
    assert!(compare_sql_values(&SqlValue::Bool(true), &SqlValue::Int(0)).is_lt());
    assert!(compare_sql_values(&SqlValue::Int(2), &SqlValue::Text("2".to_string())).is_lt());
    assert!(compare_sql_values(&SqlValue::Blob(vec![1]), &SqlValue::Blob(vec![1, 2])).is_lt());
}

#[test]
fn iterators_return_copies() {
    let mut blob = vec![1, 2];
    let mut iterator = ListRowIterator::new(vec![row([
        ("id", SqlValue::Int(1)),
        ("blob", SqlValue::Blob(blob.clone())),
    ])]);
    let mut first = iterator.next().expect("row");
    assert_eq!(first["id"], SqlValue::Int(1));
    if let SqlValue::Blob(bytes) = first.get_mut("blob").unwrap() {
        bytes[0] = 9;
    }
    assert_eq!(blob[0], 1);
    assert!(iterator.next().is_none());
    iterator.close();
    assert!(iterator.next().is_none());

    blob[0] = 3;
    let rows = vec![
        row([("id", SqlValue::Int(1))]),
        row([("id", SqlValue::Int(2))]),
    ];
    let mut cursor = ListCursor::new(rows);
    assert!(cursor.current_row().is_none());
    assert_eq!(cursor.next().unwrap()["id"], SqlValue::Int(1));
    let mut current = cursor.current_row().unwrap();
    current.insert("id".to_string(), SqlValue::Int(99));
    assert_eq!(cursor.current_row().unwrap()["id"], SqlValue::Int(1));
    cursor.close();
    assert!(cursor.next().is_none());
}

#[test]
fn schema_adapter_exposes_column_names() {
    let backend = users();
    let schema = backend_as_schema_provider(&backend);
    assert_eq!(
        schema.column_names("users").unwrap(),
        vec!["id", "name", "age", "email"]
    );
    assert!(matches!(
        schema.column_names("missing"),
        Err(BackendError::TableNotFound { .. })
    ));
}

#[test]
fn tables_columns_scan_and_fixtures_work() {
    let backend = users();
    assert_eq!(backend.tables(), vec!["users"]);
    assert_eq!(backend.columns("USERS").unwrap()[1].name, "name");
    let rows = collect(backend.scan("users").unwrap());
    assert_eq!(
        values(&rows, "id"),
        vec![SqlValue::Int(1), SqlValue::Int(2), SqlValue::Int(3)]
    );

    let mut fixtures = BTreeMap::new();
    fixtures.insert(
        "logs".to_string(),
        (
            vec![ColumnDef::new("id", "INTEGER")],
            vec![row([("id", SqlValue::Int(1))])],
        ),
    );
    let fixture_backend = InMemoryBackend::from_tables(fixtures);
    assert_eq!(
        collect(fixture_backend.scan("logs").unwrap()),
        vec![row([("id", SqlValue::Int(1))])]
    );
}

#[test]
fn insert_applies_defaults_and_constraints() {
    let mut backend = InMemoryBackend::new();
    backend
        .create_table(
            "items",
            vec![
                ColumnDef::new("id", "INTEGER").primary_key(),
                ColumnDef::new("status", "TEXT").default(SqlValue::Text("active".to_string())),
            ],
            false,
        )
        .unwrap();
    backend
        .insert("items", row([("id", SqlValue::Int(1))]))
        .unwrap();
    assert_eq!(
        collect(backend.scan("items").unwrap())[0]["status"],
        SqlValue::Text("active".to_string())
    );
    assert!(matches!(
        backend.insert(
            "items",
            row([
                ("id", SqlValue::Int(2)),
                ("ghost", SqlValue::Text("x".to_string()))
            ])
        ),
        Err(BackendError::ColumnNotFound { .. })
    ));

    let mut backend = users();
    assert!(matches!(
        backend.insert(
            "users",
            row([
                ("id", SqlValue::Int(1)),
                ("name", SqlValue::Text("Dup".to_string())),
                ("age", SqlValue::Int(9)),
                ("email", SqlValue::Text("dup@example.com".to_string())),
            ])
        ),
        Err(BackendError::ConstraintViolation { .. })
    ));
    assert!(matches!(
        backend.insert(
            "users",
            row([
                ("id", SqlValue::Int(4)),
                ("name", SqlValue::Null),
                ("age", SqlValue::Int(9)),
                ("email", SqlValue::Text("dup@example.com".to_string())),
            ])
        ),
        Err(BackendError::ConstraintViolation { .. })
    ));
    assert!(matches!(
        backend.insert(
            "users",
            row([
                ("id", SqlValue::Int(4)),
                ("name", SqlValue::Text("Dup".to_string())),
                ("age", SqlValue::Int(9)),
                ("email", SqlValue::Text("alice@example.com".to_string())),
            ])
        ),
        Err(BackendError::ConstraintViolation { .. })
    ));
}

#[test]
fn unique_allows_multiple_nulls() {
    let mut backend = InMemoryBackend::new();
    backend
        .create_table(
            "users",
            vec![
                ColumnDef::new("id", "INTEGER").primary_key(),
                ColumnDef::new("email", "TEXT").unique(),
            ],
            false,
        )
        .unwrap();
    backend
        .insert(
            "users",
            row([("id", SqlValue::Int(1)), ("email", SqlValue::Null)]),
        )
        .unwrap();
    backend
        .insert(
            "users",
            row([("id", SqlValue::Int(2)), ("email", SqlValue::Null)]),
        )
        .unwrap();
    assert_eq!(collect(backend.scan("users").unwrap()).len(), 2);
}

#[test]
fn update_and_delete_positioned_rows() {
    let mut backend = users();
    let mut cursor = backend.open_cursor("users").unwrap();
    assert_eq!(cursor.next().unwrap()["id"], SqlValue::Int(1));
    backend
        .update(
            "users",
            &cursor,
            row([("NAME", SqlValue::Text("ALICE".to_string()))]),
        )
        .unwrap();
    assert_eq!(
        collect(backend.scan("users").unwrap())[0]["name"],
        SqlValue::Text("ALICE".to_string())
    );
    assert!(matches!(
        backend.update(
            "users",
            &cursor,
            row([("missing", SqlValue::Text("x".to_string()))])
        ),
        Err(BackendError::ColumnNotFound { .. })
    ));
    backend.delete("users", &mut cursor).unwrap();
    assert_eq!(
        collect(backend.scan("users").unwrap())[0]["id"],
        SqlValue::Int(2)
    );
    assert!(matches!(
        backend.update(
            "users",
            &cursor,
            row([("name", SqlValue::Text("x".to_string()))])
        ),
        Err(BackendError::Unsupported { .. })
    ));
    let mut foreign = ListCursor::new(vec![]);
    assert!(matches!(
        backend.delete("users", &mut foreign),
        Err(BackendError::Unsupported { .. })
    ));
}

#[test]
fn ddl_operations_work() {
    let mut backend = InMemoryBackend::new();
    backend
        .create_table("t", vec![ColumnDef::new("id", "INTEGER")], false)
        .unwrap();
    backend.create_table("T", vec![], true).unwrap();
    assert!(matches!(
        backend.create_table("t", vec![], false),
        Err(BackendError::TableAlreadyExists { .. })
    ));
    assert!(matches!(
        backend.create_table(
            "dupe",
            vec![
                ColumnDef::new("id", "INTEGER"),
                ColumnDef::new("ID", "INTEGER")
            ],
            false
        ),
        Err(BackendError::ColumnAlreadyExists { .. })
    ));
    backend
        .insert("t", row([("id", SqlValue::Int(1))]))
        .unwrap();
    backend
        .add_column(
            "t",
            ColumnDef::new("status", "TEXT").default(SqlValue::Text("new".to_string())),
        )
        .unwrap();
    assert_eq!(
        collect(backend.scan("t").unwrap())[0]["status"],
        SqlValue::Text("new".to_string())
    );
    assert!(matches!(
        backend.add_column("t", ColumnDef::new("status", "TEXT")),
        Err(BackendError::ColumnAlreadyExists { .. })
    ));
    assert!(matches!(
        backend.add_column("t", ColumnDef::new("required", "TEXT").not_null()),
        Err(BackendError::ConstraintViolation { .. })
    ));
    backend.drop_table("t", false).unwrap();
    backend.drop_table("t", true).unwrap();
    assert!(matches!(
        backend.drop_table("t", false),
        Err(BackendError::TableNotFound { .. })
    ));
}

#[test]
fn transactions_and_savepoints_work() {
    let mut backend = users();
    let handle = backend.begin_transaction().unwrap();
    backend
        .insert(
            "users",
            row([
                ("id", SqlValue::Int(4)),
                ("name", SqlValue::Text("Dave".to_string())),
                ("age", SqlValue::Int(41)),
                ("email", SqlValue::Text("dave@example.com".to_string())),
            ]),
        )
        .unwrap();
    backend.rollback(handle).unwrap();
    assert!(!contains_id(&backend, 4));

    let committed = backend.begin_transaction().unwrap();
    backend
        .insert(
            "users",
            row([
                ("id", SqlValue::Int(4)),
                ("name", SqlValue::Text("Dave".to_string())),
                ("age", SqlValue::Int(41)),
                ("email", SqlValue::Text("dave@example.com".to_string())),
            ]),
        )
        .unwrap();
    backend.commit(committed).unwrap();
    assert!(contains_id(&backend, 4));

    let active = backend.begin_transaction().unwrap();
    assert_eq!(backend.current_transaction(), Some(active));
    assert!(matches!(
        backend.begin_transaction(),
        Err(BackendError::Unsupported { .. })
    ));
    backend.commit(active).unwrap();
    assert!(matches!(
        backend.commit(active),
        Err(BackendError::Unsupported { .. })
    ));

    let handle = backend.begin_transaction().unwrap();
    backend.create_savepoint("s1").unwrap();
    backend
        .insert(
            "users",
            row([
                ("id", SqlValue::Int(5)),
                ("name", SqlValue::Text("Eve".to_string())),
                ("age", SqlValue::Int(22)),
                ("email", SqlValue::Text("eve@example.com".to_string())),
            ]),
        )
        .unwrap();
    backend.rollback_to_savepoint("s1").unwrap();
    assert!(!contains_id(&backend, 5));
    backend.release_savepoint("s1").unwrap();
    assert!(matches!(
        backend.release_savepoint("s1"),
        Err(BackendError::Unsupported { .. })
    ));
    backend.commit(handle).unwrap();

    backend.create_savepoint("implicit").unwrap();
    let current = backend.current_transaction().unwrap();
    backend.rollback(current).unwrap();
}

#[test]
fn indexes_work() {
    let mut backend = users();
    backend
        .create_index(IndexDef::new("idx_age", "users", vec!["age".to_string()]))
        .unwrap();
    assert_eq!(backend.list_indexes(Some("USERS"))[0].name, "idx_age");
    let rowids = backend
        .scan_index(
            "idx_age",
            Some(&[SqlValue::Int(25)]),
            Some(&[SqlValue::Int(30)]),
            true,
            true,
        )
        .unwrap();
    assert_eq!(rowids, vec![1, 0]);
    assert_eq!(
        values(
            &collect(backend.scan_by_rowids("users", &rowids).unwrap()),
            "id"
        ),
        vec![SqlValue::Int(2), SqlValue::Int(1)]
    );
    backend.drop_index("idx_age", false).unwrap();
    assert!(backend.list_indexes(None).is_empty());
    assert!(matches!(
        backend.drop_index("idx_age", false),
        Err(BackendError::IndexNotFound { .. })
    ));
    backend.drop_index("idx_age", true).unwrap();
}

#[test]
fn index_inputs_are_validated() {
    let mut backend = users();
    backend
        .create_index(IndexDef {
            name: "idx_email".to_string(),
            table: "users".to_string(),
            columns: vec!["email".to_string()],
            unique: true,
            auto: false,
        })
        .unwrap();
    assert!(matches!(
        backend.create_index(IndexDef::new(
            "IDX_EMAIL",
            "users",
            vec!["email".to_string()]
        )),
        Err(BackendError::IndexAlreadyExists { .. })
    ));
    assert!(matches!(
        backend.create_index(IndexDef::new(
            "idx_missing",
            "missing",
            vec!["id".to_string()]
        )),
        Err(BackendError::TableNotFound { .. })
    ));
    assert!(matches!(
        backend.create_index(IndexDef::new(
            "idx_bad",
            "users",
            vec!["missing".to_string()]
        )),
        Err(BackendError::ColumnNotFound { .. })
    ));
    assert!(matches!(
        backend.scan_index("missing", None, None, true, true),
        Err(BackendError::IndexNotFound { .. })
    ));
}

#[test]
fn triggers_and_versions_work() {
    let mut backend = users();
    assert!(backend.schema_version() > 0);
    backend.set_user_version(7).unwrap();
    assert_eq!(backend.user_version(), 7);
    assert!(matches!(
        backend.set_user_version(u32::MAX as u64 + 1),
        Err(BackendError::Internal { .. })
    ));
    let trigger = TriggerDef {
        name: "tr_users_ai".to_string(),
        table: "users".to_string(),
        timing: "AFTER".to_string(),
        event: "INSERT".to_string(),
        body: "SELECT 1".to_string(),
    };
    backend.create_trigger(trigger.clone()).unwrap();
    assert_eq!(backend.list_triggers("USERS"), vec![trigger.clone()]);
    assert!(matches!(
        backend.create_trigger(trigger.clone()),
        Err(BackendError::TriggerAlreadyExists { .. })
    ));
    backend.drop_trigger("TR_USERS_AI", false).unwrap();
    assert!(backend.list_triggers("users").is_empty());
    assert!(matches!(
        backend.drop_trigger("tr_users_ai", false),
        Err(BackendError::TriggerNotFound { .. })
    ));
    backend.drop_trigger("tr_users_ai", true).unwrap();
}

#[test]
fn drop_table_removes_owned_indexes_and_triggers() {
    let mut backend = users();
    backend
        .create_index(IndexDef::new("idx_age", "users", vec!["age".to_string()]))
        .unwrap();
    backend
        .create_trigger(TriggerDef {
            name: "tr_users_ai".to_string(),
            table: "users".to_string(),
            timing: "AFTER".to_string(),
            event: "INSERT".to_string(),
            body: "SELECT 1".to_string(),
        })
        .unwrap();
    backend.drop_table("users", false).unwrap();
    assert!(backend.list_indexes(None).is_empty());
    assert!(backend.list_triggers("users").is_empty());
}

fn users() -> InMemoryBackend {
    let mut backend = InMemoryBackend::new();
    backend
        .create_table(
            "users",
            vec![
                ColumnDef::new("id", "INTEGER").primary_key(),
                ColumnDef::new("name", "TEXT").not_null(),
                ColumnDef::new("age", "INTEGER"),
                ColumnDef::new("email", "TEXT").unique(),
            ],
            false,
        )
        .unwrap();
    backend
        .insert(
            "users",
            row([
                ("id", SqlValue::Int(1)),
                ("name", SqlValue::Text("Alice".to_string())),
                ("age", SqlValue::Int(30)),
                ("email", SqlValue::Text("alice@example.com".to_string())),
            ]),
        )
        .unwrap();
    backend
        .insert(
            "users",
            row([
                ("id", SqlValue::Int(2)),
                ("name", SqlValue::Text("Bob".to_string())),
                ("age", SqlValue::Int(25)),
                ("email", SqlValue::Text("bob@example.com".to_string())),
            ]),
        )
        .unwrap();
    backend
        .insert(
            "users",
            row([
                ("id", SqlValue::Int(3)),
                ("name", SqlValue::Text("Carol".to_string())),
                ("age", SqlValue::Null),
                ("email", SqlValue::Null),
            ]),
        )
        .unwrap();
    backend
}

fn row<const N: usize>(values: [(&str, SqlValue); N]) -> Row {
    values
        .into_iter()
        .map(|(k, v)| (k.to_string(), v))
        .collect()
}

fn collect(mut iterator: Box<dyn RowIterator>) -> Vec<Row> {
    let mut rows = Vec::new();
    while let Some(row) = iterator.next() {
        rows.push(row);
    }
    iterator.close();
    rows
}

fn values(rows: &[Row], column: &str) -> Vec<SqlValue> {
    rows.iter().map(|row| row[column].clone()).collect()
}

fn contains_id(backend: &InMemoryBackend, id: i64) -> bool {
    collect(backend.scan("users").unwrap())
        .into_iter()
        .any(|row| row["id"] == SqlValue::Int(id))
}
