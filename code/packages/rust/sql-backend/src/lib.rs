use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

pub type Row = BTreeMap<String, SqlValue>;
pub type TransactionHandle = u64;

#[derive(Clone, Debug, PartialEq)]
pub enum SqlValue {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Text(String),
    Blob(Vec<u8>),
}

impl SqlValue {
    pub fn type_name(&self) -> &'static str {
        match self {
            SqlValue::Null => "NULL",
            SqlValue::Bool(_) => "BOOLEAN",
            SqlValue::Int(_) => "INTEGER",
            SqlValue::Float(_) => "REAL",
            SqlValue::Text(_) => "TEXT",
            SqlValue::Blob(_) => "BLOB",
        }
    }
}

pub fn compare_sql_values(left: &SqlValue, right: &SqlValue) -> Ordering {
    let rank_cmp = value_rank(left).cmp(&value_rank(right));
    if rank_cmp != Ordering::Equal {
        return rank_cmp;
    }
    match (left, right) {
        (SqlValue::Null, SqlValue::Null) => Ordering::Equal,
        (SqlValue::Bool(l), SqlValue::Bool(r)) => l.cmp(r),
        (SqlValue::Int(l), SqlValue::Int(r)) => l.cmp(r),
        (SqlValue::Float(l), SqlValue::Float(r)) => l.partial_cmp(r).unwrap_or(Ordering::Equal),
        (SqlValue::Int(l), SqlValue::Float(r)) => {
            (*l as f64).partial_cmp(r).unwrap_or(Ordering::Equal)
        }
        (SqlValue::Float(l), SqlValue::Int(r)) => {
            l.partial_cmp(&(*r as f64)).unwrap_or(Ordering::Equal)
        }
        (SqlValue::Text(l), SqlValue::Text(r)) => l.cmp(r),
        (SqlValue::Blob(l), SqlValue::Blob(r)) => l.cmp(r),
        _ => Ordering::Equal,
    }
}

fn value_rank(value: &SqlValue) -> u8 {
    match value {
        SqlValue::Null => 0,
        SqlValue::Bool(_) => 1,
        SqlValue::Int(_) | SqlValue::Float(_) => 2,
        SqlValue::Text(_) => 3,
        SqlValue::Blob(_) => 4,
    }
}

pub trait RowIterator {
    fn next(&mut self) -> Option<Row>;
    fn close(&mut self);
}

pub trait Cursor: RowIterator {
    fn current_row(&self) -> Option<Row>;
    fn current_index(&self) -> Option<usize>;
    fn table_key(&self) -> Option<&str>;
    fn adjust_after_delete(&mut self);
}

#[derive(Clone, Debug)]
pub struct ListRowIterator {
    rows: Vec<Row>,
    index: usize,
    closed: bool,
}

impl ListRowIterator {
    pub fn new(rows: impl IntoIterator<Item = Row>) -> Self {
        Self {
            rows: rows.into_iter().collect(),
            index: 0,
            closed: false,
        }
    }
}

impl RowIterator for ListRowIterator {
    fn next(&mut self) -> Option<Row> {
        if self.closed || self.index >= self.rows.len() {
            return None;
        }
        let row = self.rows[self.index].clone();
        self.index += 1;
        Some(row)
    }

    fn close(&mut self) {
        self.closed = true;
    }
}

#[derive(Clone, Debug)]
pub struct ListCursor {
    rows: Vec<Row>,
    table_key: Option<String>,
    index: isize,
    current: Option<Row>,
    closed: bool,
}

impl ListCursor {
    pub fn new(rows: Vec<Row>) -> Self {
        Self {
            rows,
            table_key: None,
            index: -1,
            current: None,
            closed: false,
        }
    }

    fn for_table(table_key: String, rows: Vec<Row>) -> Self {
        Self {
            rows,
            table_key: Some(table_key),
            index: -1,
            current: None,
            closed: false,
        }
    }
}

impl RowIterator for ListCursor {
    fn next(&mut self) -> Option<Row> {
        if self.closed {
            return None;
        }
        self.index += 1;
        let idx = usize::try_from(self.index).ok()?;
        if idx >= self.rows.len() {
            self.current = None;
            return None;
        }
        self.current = Some(self.rows[idx].clone());
        self.current.clone()
    }

    fn close(&mut self) {
        self.closed = true;
    }
}

impl Cursor for ListCursor {
    fn current_row(&self) -> Option<Row> {
        self.current.clone()
    }

    fn current_index(&self) -> Option<usize> {
        usize::try_from(self.index).ok()
    }

    fn table_key(&self) -> Option<&str> {
        self.table_key.as_deref()
    }

    fn adjust_after_delete(&mut self) {
        self.index -= 1;
        self.current = None;
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ColumnDef {
    pub name: String,
    pub type_name: String,
    pub not_null: bool,
    pub primary_key: bool,
    pub unique: bool,
    pub autoincrement: bool,
    pub default_value: SqlValue,
    pub has_default: bool,
    pub check_expression: Option<String>,
    pub foreign_key: Option<String>,
}

impl ColumnDef {
    pub fn new(name: impl Into<String>, type_name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            type_name: type_name.into(),
            not_null: false,
            primary_key: false,
            unique: false,
            autoincrement: false,
            default_value: SqlValue::Null,
            has_default: false,
            check_expression: None,
            foreign_key: None,
        }
    }

    pub fn not_null(mut self) -> Self {
        self.not_null = true;
        self
    }

    pub fn primary_key(mut self) -> Self {
        self.primary_key = true;
        self
    }

    pub fn unique(mut self) -> Self {
        self.unique = true;
        self
    }

    pub fn default(mut self, value: SqlValue) -> Self {
        self.default_value = value;
        self.has_default = true;
        self
    }

    pub fn effective_not_null(&self) -> bool {
        self.not_null || self.primary_key
    }

    pub fn effective_unique(&self) -> bool {
        self.unique || self.primary_key
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IndexDef {
    pub name: String,
    pub table: String,
    pub columns: Vec<String>,
    pub unique: bool,
    pub auto: bool,
}

impl IndexDef {
    pub fn new(name: impl Into<String>, table: impl Into<String>, columns: Vec<String>) -> Self {
        Self {
            name: name.into(),
            table: table.into(),
            columns,
            unique: false,
            auto: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TriggerDef {
    pub name: String,
    pub table: String,
    pub timing: String,
    pub event: String,
    pub body: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum BackendError {
    TableNotFound {
        table: String,
    },
    TableAlreadyExists {
        table: String,
    },
    ColumnNotFound {
        table: String,
        column: String,
    },
    ColumnAlreadyExists {
        table: String,
        column: String,
    },
    ConstraintViolation {
        table: String,
        column: String,
        message: String,
    },
    Unsupported {
        operation: String,
    },
    Internal {
        message: String,
    },
    IndexAlreadyExists {
        index: String,
    },
    IndexNotFound {
        index: String,
    },
    TriggerAlreadyExists {
        trigger: String,
    },
    TriggerNotFound {
        trigger: String,
    },
}

impl fmt::Display for BackendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BackendError::TableNotFound { table } => write!(f, "table not found: {table:?}"),
            BackendError::TableAlreadyExists { table } => {
                write!(f, "table already exists: {table:?}")
            }
            BackendError::ColumnNotFound { table, column } => {
                write!(f, "column not found: {table:?}.{column:?}")
            }
            BackendError::ColumnAlreadyExists { table, column } => {
                write!(f, "column already exists: {table:?}.{column:?}")
            }
            BackendError::ConstraintViolation { message, .. } => f.write_str(message),
            BackendError::Unsupported { operation } => {
                write!(f, "operation not supported: {operation}")
            }
            BackendError::Internal { message } => f.write_str(message),
            BackendError::IndexAlreadyExists { index } => {
                write!(f, "index already exists: {index:?}")
            }
            BackendError::IndexNotFound { index } => write!(f, "index not found: {index:?}"),
            BackendError::TriggerAlreadyExists { trigger } => {
                write!(f, "trigger already exists: {trigger:?}")
            }
            BackendError::TriggerNotFound { trigger } => {
                write!(f, "trigger not found: {trigger:?}")
            }
        }
    }
}

impl Error for BackendError {}

pub trait Backend {
    fn tables(&self) -> Vec<String>;
    fn columns(&self, table: &str) -> Result<Vec<ColumnDef>, BackendError>;
    fn scan(&self, table: &str) -> Result<Box<dyn RowIterator>, BackendError>;
    fn insert(&mut self, table: &str, row: Row) -> Result<(), BackendError>;
    fn update(
        &mut self,
        table: &str,
        cursor: &dyn Cursor,
        assignments: Row,
    ) -> Result<(), BackendError>;
    fn delete(&mut self, table: &str, cursor: &mut dyn Cursor) -> Result<(), BackendError>;
    fn create_table(
        &mut self,
        table: &str,
        columns: Vec<ColumnDef>,
        if_not_exists: bool,
    ) -> Result<(), BackendError>;
    fn drop_table(&mut self, table: &str, if_exists: bool) -> Result<(), BackendError>;
    fn add_column(&mut self, table: &str, column: ColumnDef) -> Result<(), BackendError>;
    fn create_index(&mut self, index: IndexDef) -> Result<(), BackendError>;
    fn drop_index(&mut self, name: &str, if_exists: bool) -> Result<(), BackendError>;
    fn list_indexes(&self, table: Option<&str>) -> Vec<IndexDef>;
    fn scan_index(
        &self,
        index_name: &str,
        lo: Option<&[SqlValue]>,
        hi: Option<&[SqlValue]>,
        lo_inclusive: bool,
        hi_inclusive: bool,
    ) -> Result<Vec<usize>, BackendError>;
    fn scan_by_rowids(
        &self,
        table: &str,
        rowids: &[usize],
    ) -> Result<Box<dyn RowIterator>, BackendError>;
    fn begin_transaction(&mut self) -> Result<TransactionHandle, BackendError>;
    fn commit(&mut self, handle: TransactionHandle) -> Result<(), BackendError>;
    fn rollback(&mut self, handle: TransactionHandle) -> Result<(), BackendError>;
}

pub trait SchemaProvider {
    fn column_names(&self, table: &str) -> Result<Vec<String>, BackendError>;
}

pub struct BackendSchemaProvider<'a> {
    backend: &'a dyn Backend,
}

pub fn backend_as_schema_provider(backend: &dyn Backend) -> BackendSchemaProvider<'_> {
    BackendSchemaProvider { backend }
}

impl SchemaProvider for BackendSchemaProvider<'_> {
    fn column_names(&self, table: &str) -> Result<Vec<String>, BackendError> {
        Ok(self
            .backend
            .columns(table)?
            .into_iter()
            .map(|c| c.name)
            .collect())
    }
}

#[derive(Clone, Debug)]
struct TableState {
    name: String,
    columns: Vec<ColumnDef>,
    rows: Vec<Row>,
}

#[derive(Clone, Debug)]
struct Snapshot {
    tables: BTreeMap<String, TableState>,
    indexes: BTreeMap<String, IndexDef>,
    triggers: BTreeMap<String, TriggerDef>,
    triggers_by_table: BTreeMap<String, Vec<TriggerDef>>,
    user_version: u32,
    schema_version: u32,
}

#[derive(Clone, Debug)]
struct Savepoint {
    name: String,
    snapshot: Snapshot,
}

#[derive(Clone, Debug)]
pub struct InMemoryBackend {
    tables: BTreeMap<String, TableState>,
    indexes: BTreeMap<String, IndexDef>,
    triggers: BTreeMap<String, TriggerDef>,
    triggers_by_table: BTreeMap<String, Vec<TriggerDef>>,
    snapshot: Option<Snapshot>,
    savepoints: Vec<Savepoint>,
    active_handle: Option<TransactionHandle>,
    next_handle: TransactionHandle,
    user_version: u32,
    schema_version: u32,
}

impl InMemoryBackend {
    pub fn new() -> Self {
        Self {
            tables: BTreeMap::new(),
            indexes: BTreeMap::new(),
            triggers: BTreeMap::new(),
            triggers_by_table: BTreeMap::new(),
            snapshot: None,
            savepoints: Vec::new(),
            active_handle: None,
            next_handle: 1,
            user_version: 0,
            schema_version: 0,
        }
    }

    pub fn from_tables(tables: BTreeMap<String, (Vec<ColumnDef>, Vec<Row>)>) -> Self {
        let mut backend = Self::new();
        for (name, (columns, rows)) in tables {
            backend.tables.insert(
                normalize_name(&name),
                TableState {
                    name,
                    columns,
                    rows,
                },
            );
        }
        backend
    }

    pub fn open_cursor(&self, table: &str) -> Result<ListCursor, BackendError> {
        let key = normalize_name(table);
        let state = self.require_table(table)?;
        Ok(ListCursor::for_table(key, state.rows.clone()))
    }

    pub fn current_transaction(&self) -> Option<TransactionHandle> {
        self.active_handle
    }

    pub fn create_savepoint(&mut self, name: &str) -> Result<(), BackendError> {
        if self.active_handle.is_none() {
            self.begin_transaction()?;
        }
        self.savepoints.push(Savepoint {
            name: name.to_string(),
            snapshot: self.capture_snapshot(),
        });
        Ok(())
    }

    pub fn release_savepoint(&mut self, name: &str) -> Result<(), BackendError> {
        let idx = self
            .find_savepoint(name)
            .ok_or_else(|| BackendError::Unsupported {
                operation: format!("RELEASE {name:?}: no such savepoint"),
            })?;
        self.savepoints.truncate(idx);
        Ok(())
    }

    pub fn rollback_to_savepoint(&mut self, name: &str) -> Result<(), BackendError> {
        let idx = self
            .find_savepoint(name)
            .ok_or_else(|| BackendError::Unsupported {
                operation: format!("ROLLBACK TO {name:?}: no such savepoint"),
            })?;
        let snapshot = self.savepoints[idx].snapshot.clone();
        self.restore_snapshot(snapshot);
        self.savepoints.truncate(idx + 1);
        Ok(())
    }

    pub fn create_trigger(&mut self, trigger: TriggerDef) -> Result<(), BackendError> {
        let key = normalize_name(&trigger.name);
        if self.triggers.contains_key(&key) {
            return Err(BackendError::TriggerAlreadyExists {
                trigger: trigger.name,
            });
        }
        let table_key = normalize_name(&trigger.table);
        self.triggers.insert(key, trigger.clone());
        self.triggers_by_table
            .entry(table_key)
            .or_default()
            .push(trigger);
        Ok(())
    }

    pub fn drop_trigger(&mut self, name: &str, if_exists: bool) -> Result<(), BackendError> {
        let key = normalize_name(name);
        let trigger = match self.triggers.remove(&key) {
            Some(trigger) => trigger,
            None if if_exists => return Ok(()),
            None => {
                return Err(BackendError::TriggerNotFound {
                    trigger: name.to_string(),
                })
            }
        };
        let table_key = normalize_name(&trigger.table);
        if let Some(triggers) = self.triggers_by_table.get_mut(&table_key) {
            triggers.retain(|candidate| !same_name(&candidate.name, name));
            if triggers.is_empty() {
                self.triggers_by_table.remove(&table_key);
            }
        }
        Ok(())
    }

    pub fn list_triggers(&self, table: &str) -> Vec<TriggerDef> {
        self.triggers_by_table
            .get(&normalize_name(table))
            .cloned()
            .unwrap_or_default()
    }

    pub fn user_version(&self) -> u32 {
        self.user_version
    }

    pub fn set_user_version(&mut self, value: u64) -> Result<(), BackendError> {
        if value > u32::MAX as u64 {
            return Err(BackendError::Internal {
                message: format!("user_version must fit in u32, got {value}"),
            });
        }
        self.user_version = value as u32;
        Ok(())
    }

    pub fn schema_version(&self) -> u32 {
        self.schema_version
    }

    fn require_table(&self, table: &str) -> Result<&TableState, BackendError> {
        self.tables
            .get(&normalize_name(table))
            .ok_or_else(|| BackendError::TableNotFound {
                table: table.to_string(),
            })
    }

    fn require_table_mut(&mut self, table: &str) -> Result<&mut TableState, BackendError> {
        self.tables
            .get_mut(&normalize_name(table))
            .ok_or_else(|| BackendError::TableNotFound {
                table: table.to_string(),
            })
    }

    fn normalize_row(table: &str, state: &TableState, row: Row) -> Result<Row, BackendError> {
        let mut normalized = Row::new();
        for (name, value) in row {
            let canonical = canonical_column(table, state, &name)?;
            normalized.insert(canonical, value);
        }
        for column in &state.columns {
            normalized.entry(column.name.clone()).or_insert_with(|| {
                if column.has_default {
                    column.default_value.clone()
                } else {
                    SqlValue::Null
                }
            });
        }
        Ok(normalized)
    }

    fn require_active(&self, handle: TransactionHandle) -> Result<(), BackendError> {
        match self.active_handle {
            None => Err(BackendError::Unsupported {
                operation: "no active transaction".to_string(),
            }),
            Some(active) if active != handle => Err(BackendError::Unsupported {
                operation: "stale transaction handle".to_string(),
            }),
            Some(_) => Ok(()),
        }
    }

    fn capture_snapshot(&self) -> Snapshot {
        Snapshot {
            tables: self.tables.clone(),
            indexes: self.indexes.clone(),
            triggers: self.triggers.clone(),
            triggers_by_table: self.triggers_by_table.clone(),
            user_version: self.user_version,
            schema_version: self.schema_version,
        }
    }

    fn restore_snapshot(&mut self, snapshot: Snapshot) {
        self.tables = snapshot.tables;
        self.indexes = snapshot.indexes;
        self.triggers = snapshot.triggers;
        self.triggers_by_table = snapshot.triggers_by_table;
        self.user_version = snapshot.user_version;
        self.schema_version = snapshot.schema_version;
    }

    fn find_savepoint(&self, name: &str) -> Option<usize> {
        self.savepoints
            .iter()
            .rposition(|savepoint| savepoint.name == name)
    }

    fn delete_indexes_for_table(&mut self, table: &str) {
        self.indexes
            .retain(|_, index| !same_name(&index.table, table));
    }

    fn delete_triggers_for_table(&mut self, table: &str) {
        let table_key = normalize_name(table);
        self.triggers
            .retain(|_, trigger| !same_name(&trigger.table, table));
        self.triggers_by_table.remove(&table_key);
    }

    fn bump_schema_version(&mut self) {
        self.schema_version = self.schema_version.wrapping_add(1);
    }
}

impl Default for InMemoryBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl Backend for InMemoryBackend {
    fn tables(&self) -> Vec<String> {
        self.tables
            .values()
            .map(|table| table.name.clone())
            .collect()
    }

    fn columns(&self, table: &str) -> Result<Vec<ColumnDef>, BackendError> {
        Ok(self.require_table(table)?.columns.clone())
    }

    fn scan(&self, table: &str) -> Result<Box<dyn RowIterator>, BackendError> {
        Ok(Box::new(ListRowIterator::new(
            self.require_table(table)?.rows.clone(),
        )))
    }

    fn insert(&mut self, table: &str, row: Row) -> Result<(), BackendError> {
        let state = self.require_table(table)?.clone();
        let normalized = Self::normalize_row(table, &state, row)?;
        check_not_null(table, &state, &normalized)?;
        check_unique(table, &state, &normalized, None)?;
        self.require_table_mut(table)?.rows.push(normalized);
        Ok(())
    }

    fn update(
        &mut self,
        table: &str,
        cursor: &dyn Cursor,
        assignments: Row,
    ) -> Result<(), BackendError> {
        let key = normalize_name(table);
        if cursor.table_key() != Some(key.as_str()) {
            return Err(BackendError::Unsupported {
                operation: format!("foreign cursor for table {table}"),
            });
        }
        let idx = cursor
            .current_index()
            .ok_or_else(|| BackendError::Unsupported {
                operation: "update without current row".to_string(),
            })?;
        let state = self.require_table(table)?.clone();
        if idx >= state.rows.len() {
            return Err(BackendError::Unsupported {
                operation: "update without current row".to_string(),
            });
        }
        let mut updated = state.rows[idx].clone();
        for (name, value) in assignments {
            let canonical = canonical_column(table, &state, &name)?;
            updated.insert(canonical, value);
        }
        check_not_null(table, &state, &updated)?;
        check_unique(table, &state, &updated, Some(idx))?;
        self.require_table_mut(table)?.rows[idx] = updated;
        Ok(())
    }

    fn delete(&mut self, table: &str, cursor: &mut dyn Cursor) -> Result<(), BackendError> {
        let key = normalize_name(table);
        if cursor.table_key() != Some(key.as_str()) {
            return Err(BackendError::Unsupported {
                operation: format!("foreign cursor for table {table}"),
            });
        }
        let idx = cursor
            .current_index()
            .ok_or_else(|| BackendError::Unsupported {
                operation: "delete without current row".to_string(),
            })?;
        let state = self.require_table_mut(table)?;
        if idx >= state.rows.len() {
            return Err(BackendError::Unsupported {
                operation: "delete without current row".to_string(),
            });
        }
        state.rows.remove(idx);
        cursor.adjust_after_delete();
        Ok(())
    }

    fn create_table(
        &mut self,
        table: &str,
        columns: Vec<ColumnDef>,
        if_not_exists: bool,
    ) -> Result<(), BackendError> {
        let key = normalize_name(table);
        if self.tables.contains_key(&key) {
            if if_not_exists {
                return Ok(());
            }
            return Err(BackendError::TableAlreadyExists {
                table: table.to_string(),
            });
        }
        let mut seen = BTreeMap::new();
        for column in &columns {
            let column_key = normalize_name(&column.name);
            if seen.insert(column_key, ()).is_some() {
                return Err(BackendError::ColumnAlreadyExists {
                    table: table.to_string(),
                    column: column.name.clone(),
                });
            }
        }
        self.tables.insert(
            key,
            TableState {
                name: table.to_string(),
                columns,
                rows: Vec::new(),
            },
        );
        self.bump_schema_version();
        Ok(())
    }

    fn drop_table(&mut self, table: &str, if_exists: bool) -> Result<(), BackendError> {
        if self.tables.remove(&normalize_name(table)).is_none() {
            if if_exists {
                return Ok(());
            }
            return Err(BackendError::TableNotFound {
                table: table.to_string(),
            });
        }
        self.delete_indexes_for_table(table);
        self.delete_triggers_for_table(table);
        self.bump_schema_version();
        Ok(())
    }

    fn add_column(&mut self, table: &str, column: ColumnDef) -> Result<(), BackendError> {
        let state = self.require_table(table)?.clone();
        if canonical_column(table, &state, &column.name).is_ok() {
            return Err(BackendError::ColumnAlreadyExists {
                table: table.to_string(),
                column: column.name,
            });
        }
        if !state.rows.is_empty() && column.effective_not_null() && !column.has_default {
            return Err(BackendError::ConstraintViolation {
                table: table.to_string(),
                column: column.name.clone(),
                message: format!("NOT NULL constraint failed: {table}.{}", column.name),
            });
        }
        let fill = if column.has_default {
            column.default_value.clone()
        } else {
            SqlValue::Null
        };
        let state = self.require_table_mut(table)?;
        for row in &mut state.rows {
            row.insert(column.name.clone(), fill.clone());
        }
        state.columns.push(column);
        self.bump_schema_version();
        Ok(())
    }

    fn create_index(&mut self, index: IndexDef) -> Result<(), BackendError> {
        let key = normalize_name(&index.name);
        if self.indexes.contains_key(&key) {
            return Err(BackendError::IndexAlreadyExists { index: index.name });
        }
        let state = self.require_table(&index.table)?;
        for column in &index.columns {
            canonical_column(&index.table, state, column)?;
        }
        self.indexes.insert(key, index);
        self.bump_schema_version();
        Ok(())
    }

    fn drop_index(&mut self, name: &str, if_exists: bool) -> Result<(), BackendError> {
        if self.indexes.remove(&normalize_name(name)).is_none() {
            if if_exists {
                return Ok(());
            }
            return Err(BackendError::IndexNotFound {
                index: name.to_string(),
            });
        }
        self.bump_schema_version();
        Ok(())
    }

    fn list_indexes(&self, table: Option<&str>) -> Vec<IndexDef> {
        self.indexes
            .values()
            .filter(|index| table.map_or(true, |table| same_name(&index.table, table)))
            .cloned()
            .collect()
    }

    fn scan_index(
        &self,
        index_name: &str,
        lo: Option<&[SqlValue]>,
        hi: Option<&[SqlValue]>,
        lo_inclusive: bool,
        hi_inclusive: bool,
    ) -> Result<Vec<usize>, BackendError> {
        let index = self
            .indexes
            .get(&normalize_name(index_name))
            .ok_or_else(|| BackendError::IndexNotFound {
                index: index_name.to_string(),
            })?;
        let state = self.require_table(&index.table)?;
        let mut keyed = Vec::new();
        for (rowid, row) in state.rows.iter().enumerate() {
            let mut key = Vec::new();
            for column in &index.columns {
                let canonical = canonical_column(&index.table, state, column)?;
                key.push(row.get(&canonical).cloned().unwrap_or(SqlValue::Null));
            }
            keyed.push((key, rowid));
        }
        keyed.sort_by(|left, right| {
            compare_key(&left.0, &right.0).then_with(|| left.1.cmp(&right.1))
        });
        let mut rowids = Vec::new();
        for (key, rowid) in keyed {
            if let Some(lo) = lo {
                let cmp = compare_prefix(&key, lo);
                if cmp == Ordering::Less || (cmp == Ordering::Equal && !lo_inclusive) {
                    continue;
                }
            }
            if let Some(hi) = hi {
                let cmp = compare_prefix(&key, hi);
                if cmp == Ordering::Greater || (cmp == Ordering::Equal && !hi_inclusive) {
                    break;
                }
            }
            rowids.push(rowid);
        }
        Ok(rowids)
    }

    fn scan_by_rowids(
        &self,
        table: &str,
        rowids: &[usize],
    ) -> Result<Box<dyn RowIterator>, BackendError> {
        let state = self.require_table(table)?;
        let rows = rowids
            .iter()
            .filter_map(|rowid| state.rows.get(*rowid).cloned())
            .collect::<Vec<_>>();
        Ok(Box::new(ListRowIterator::new(rows)))
    }

    fn begin_transaction(&mut self) -> Result<TransactionHandle, BackendError> {
        if self.active_handle.is_some() {
            return Err(BackendError::Unsupported {
                operation: "nested transactions".to_string(),
            });
        }
        let handle = self.next_handle;
        self.next_handle += 1;
        self.snapshot = Some(self.capture_snapshot());
        self.active_handle = Some(handle);
        Ok(handle)
    }

    fn commit(&mut self, handle: TransactionHandle) -> Result<(), BackendError> {
        self.require_active(handle)?;
        self.snapshot = None;
        self.active_handle = None;
        self.savepoints.clear();
        Ok(())
    }

    fn rollback(&mut self, handle: TransactionHandle) -> Result<(), BackendError> {
        self.require_active(handle)?;
        if let Some(snapshot) = self.snapshot.take() {
            self.restore_snapshot(snapshot);
        }
        self.active_handle = None;
        self.savepoints.clear();
        Ok(())
    }
}

fn canonical_column(table: &str, state: &TableState, column: &str) -> Result<String, BackendError> {
    state
        .columns
        .iter()
        .find(|candidate| same_name(&candidate.name, column))
        .map(|candidate| candidate.name.clone())
        .ok_or_else(|| BackendError::ColumnNotFound {
            table: table.to_string(),
            column: column.to_string(),
        })
}

fn check_not_null(table: &str, state: &TableState, row: &Row) -> Result<(), BackendError> {
    for column in &state.columns {
        if column.effective_not_null()
            && matches!(row.get(&column.name), None | Some(SqlValue::Null))
        {
            return Err(BackendError::ConstraintViolation {
                table: table.to_string(),
                column: column.name.clone(),
                message: format!("NOT NULL constraint failed: {table}.{}", column.name),
            });
        }
    }
    Ok(())
}

fn check_unique(
    table: &str,
    state: &TableState,
    row: &Row,
    ignore_index: Option<usize>,
) -> Result<(), BackendError> {
    for column in &state.columns {
        if !column.effective_unique() {
            continue;
        }
        let value = row.get(&column.name).unwrap_or(&SqlValue::Null);
        if matches!(value, SqlValue::Null) {
            continue;
        }
        for (idx, existing) in state.rows.iter().enumerate() {
            if ignore_index == Some(idx) {
                continue;
            }
            let existing_value = existing.get(&column.name).unwrap_or(&SqlValue::Null);
            if compare_sql_values(existing_value, value) == Ordering::Equal {
                let label = if column.primary_key {
                    "PRIMARY KEY"
                } else {
                    "UNIQUE"
                };
                return Err(BackendError::ConstraintViolation {
                    table: table.to_string(),
                    column: column.name.clone(),
                    message: format!("{label} constraint failed: {table}.{}", column.name),
                });
            }
        }
    }
    Ok(())
}

fn compare_key(left: &[SqlValue], right: &[SqlValue]) -> Ordering {
    let limit = left.len().min(right.len());
    for i in 0..limit {
        let cmp = compare_sql_values(&left[i], &right[i]);
        if cmp != Ordering::Equal {
            return cmp;
        }
    }
    left.len().cmp(&right.len())
}

fn compare_prefix(key: &[SqlValue], bound: &[SqlValue]) -> Ordering {
    for (idx, bound_value) in bound.iter().enumerate() {
        let value = key.get(idx).unwrap_or(&SqlValue::Null);
        let cmp = compare_sql_values(value, bound_value);
        if cmp != Ordering::Equal {
            return cmp;
        }
    }
    Ordering::Equal
}

fn normalize_name(name: &str) -> String {
    name.to_ascii_lowercase()
}

fn same_name(left: &str, right: &str) -> bool {
    normalize_name(left) == normalize_name(right)
}
