use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::rc::Rc;

use coding_adventures_sql_execution_engine::{
    execute as execute_select, DataSource, ExecutionError, QueryResult,
};
pub use coding_adventures_sql_execution_engine::{SqlPrimitive, SqlValue};
use coding_adventures_sql_lexer::tokenize_sql;
use lexer::token::{Token, TokenType};

pub const API_LEVEL: &str = "2.0";
pub const THREAD_SAFETY: u8 = 1;
pub const PARAM_STYLE: &str = "qmark";

const ROW_ID_COLUMN: &str = "__mini_sqlite_rowid";

pub type Result<T> = std::result::Result<T, MiniSqliteError>;

#[derive(Debug, Clone, PartialEq)]
pub enum MiniSqliteError {
    ProgrammingError(String),
    OperationalError(String),
    IntegrityError(String),
    NotSupportedError(String),
}

impl fmt::Display for MiniSqliteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MiniSqliteError::ProgrammingError(message) => write!(f, "Programming error: {message}"),
            MiniSqliteError::OperationalError(message) => write!(f, "Operational error: {message}"),
            MiniSqliteError::IntegrityError(message) => write!(f, "Integrity error: {message}"),
            MiniSqliteError::NotSupportedError(message) => write!(f, "Not supported: {message}"),
        }
    }
}

impl std::error::Error for MiniSqliteError {}

#[derive(Debug, Clone, Copy, Default)]
pub struct ConnectOptions {
    pub autocommit: bool,
}

#[derive(Clone, Debug)]
pub struct Connection {
    state: Rc<RefCell<ConnectionState>>,
}

#[derive(Debug)]
struct ConnectionState {
    db: InMemoryDatabase,
    autocommit: bool,
    snapshot: Option<Snapshot>,
    closed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnDescription {
    pub name: String,
}

#[derive(Debug)]
pub struct Cursor {
    conn: Rc<RefCell<ConnectionState>>,
    pub description: Vec<ColumnDescription>,
    rowcount: isize,
    lastrowid: Option<i64>,
    arraysize: usize,
    rows: Vec<Vec<SqlValue>>,
    offset: usize,
    closed: bool,
}

#[derive(Debug, Clone)]
struct StatementResult {
    columns: Vec<String>,
    rows: Vec<Vec<SqlValue>>,
    rows_affected: isize,
    last_row_id: Option<i64>,
}

#[derive(Debug, Clone)]
struct TableData {
    columns: Vec<String>,
    rows: Vec<HashMap<String, SqlValue>>,
}

type Snapshot = HashMap<String, TableData>;

#[derive(Debug, Clone, Default)]
struct InMemoryDatabase {
    tables: HashMap<String, TableData>,
}

#[derive(Debug, Clone)]
struct CreateTableStatement {
    table: String,
    columns: Vec<String>,
    if_not_exists: bool,
}

#[derive(Debug, Clone)]
struct DropTableStatement {
    table: String,
    if_exists: bool,
}

#[derive(Debug, Clone)]
struct InsertStatement {
    table: String,
    columns: Vec<String>,
    rows: Vec<Vec<SqlValue>>,
}

#[derive(Debug, Clone)]
struct UpdateStatement {
    table: String,
    assignments: Vec<(String, SqlValue)>,
    where_sql: String,
}

#[derive(Debug, Clone)]
struct DeleteStatement {
    table: String,
    where_sql: String,
}

pub fn connect(database: &str) -> Result<Connection> {
    connect_with_options(database, ConnectOptions::default())
}

pub fn connect_with_options(database: &str, options: ConnectOptions) -> Result<Connection> {
    if database != ":memory:" {
        return Err(MiniSqliteError::NotSupportedError(
            "Rust mini-sqlite supports only :memory: in Level 0".to_string(),
        ));
    }

    Ok(Connection {
        state: Rc::new(RefCell::new(ConnectionState {
            db: InMemoryDatabase::default(),
            autocommit: options.autocommit,
            snapshot: None,
            closed: false,
        })),
    })
}

pub fn null() -> SqlValue {
    None
}

pub fn int(value: i64) -> SqlValue {
    Some(SqlPrimitive::Int(value))
}

pub fn real(value: f64) -> SqlValue {
    Some(SqlPrimitive::Float(value))
}

pub fn text(value: impl Into<String>) -> SqlValue {
    Some(SqlPrimitive::Text(value.into()))
}

pub fn boolean(value: bool) -> SqlValue {
    Some(SqlPrimitive::Bool(value))
}

impl Connection {
    pub fn cursor(&self) -> Result<Cursor> {
        self.assert_open()?;
        Ok(Cursor {
            conn: Rc::clone(&self.state),
            description: Vec::new(),
            rowcount: -1,
            lastrowid: None,
            arraysize: 1,
            rows: Vec::new(),
            offset: 0,
            closed: false,
        })
    }

    pub fn execute(&self, sql: &str, params: &[SqlValue]) -> Result<Cursor> {
        let mut cursor = self.cursor()?;
        cursor.execute(sql, params)?;
        Ok(cursor)
    }

    pub fn executemany(&self, sql: &str, params_seq: &[Vec<SqlValue>]) -> Result<Cursor> {
        let mut cursor = self.cursor()?;
        cursor.executemany(sql, params_seq)?;
        Ok(cursor)
    }

    pub fn commit(&self) -> Result<()> {
        let mut state = self.state.borrow_mut();
        state.assert_open()?;
        state.snapshot = None;
        Ok(())
    }

    pub fn rollback(&self) -> Result<()> {
        let mut state = self.state.borrow_mut();
        state.assert_open()?;
        if let Some(snapshot) = state.snapshot.take() {
            state.db.restore(snapshot);
        }
        Ok(())
    }

    pub fn close(&self) -> Result<()> {
        let mut state = self.state.borrow_mut();
        if state.closed {
            return Ok(());
        }
        if let Some(snapshot) = state.snapshot.take() {
            state.db.restore(snapshot);
        }
        state.closed = true;
        Ok(())
    }

    fn assert_open(&self) -> Result<()> {
        self.state.borrow().assert_open()
    }
}

impl Cursor {
    pub fn rowcount(&self) -> isize {
        self.rowcount
    }

    pub fn lastrowid(&self) -> Option<i64> {
        self.lastrowid
    }

    pub fn arraysize(&self) -> usize {
        self.arraysize
    }

    pub fn set_arraysize(&mut self, arraysize: usize) {
        if arraysize > 0 {
            self.arraysize = arraysize;
        }
    }

    pub fn execute(&mut self, sql: &str, params: &[SqlValue]) -> Result<&mut Self> {
        self.assert_open()?;
        let result = self.conn.borrow_mut().execute_bound(sql, params)?;
        self.rows = result.rows;
        self.offset = 0;
        self.rowcount = result.rows_affected;
        self.lastrowid = result.last_row_id;
        self.description = result
            .columns
            .into_iter()
            .map(|name| ColumnDescription { name })
            .collect();
        Ok(self)
    }

    pub fn executemany(&mut self, sql: &str, params_seq: &[Vec<SqlValue>]) -> Result<&mut Self> {
        self.assert_open()?;
        let mut total = 0;
        for params in params_seq {
            self.execute(sql, params)?;
            if self.rowcount > 0 {
                total += self.rowcount;
            }
        }
        if !params_seq.is_empty() {
            self.rowcount = total;
        }
        Ok(self)
    }

    pub fn fetchone(&mut self) -> Option<Vec<SqlValue>> {
        if self.closed || self.offset >= self.rows.len() {
            return None;
        }
        let row = self.rows[self.offset].clone();
        self.offset += 1;
        Some(row)
    }

    pub fn fetchmany(&mut self, size: usize) -> Vec<Vec<SqlValue>> {
        if self.closed {
            return Vec::new();
        }
        let count = if size == 0 { self.arraysize } else { size };
        let end = (self.offset + count).min(self.rows.len());
        let rows = self.rows[self.offset..end].to_vec();
        self.offset = end;
        rows
    }

    pub fn fetchall(&mut self) -> Vec<Vec<SqlValue>> {
        if self.closed {
            return Vec::new();
        }
        let rows = self.rows[self.offset..].to_vec();
        self.offset = self.rows.len();
        rows
    }

    pub fn close(&mut self) {
        self.closed = true;
        self.rows.clear();
        self.description.clear();
    }

    fn assert_open(&self) -> Result<()> {
        if self.closed {
            return Err(MiniSqliteError::ProgrammingError(
                "cursor is closed".to_string(),
            ));
        }
        self.conn.borrow().assert_open()
    }
}

impl ConnectionState {
    fn execute_bound(&mut self, sql: &str, params: &[SqlValue]) -> Result<StatementResult> {
        self.assert_open()?;
        let bound = bind_parameters(sql, params)?;

        match first_keyword(&bound).as_str() {
            "BEGIN" => {
                self.ensure_snapshot();
                Ok(empty_statement_result())
            }
            "COMMIT" => {
                self.snapshot = None;
                Ok(empty_statement_result())
            }
            "ROLLBACK" => {
                if let Some(snapshot) = self.snapshot.take() {
                    self.db.restore(snapshot);
                }
                Ok(empty_statement_result())
            }
            "SELECT" => self.db.select_sql(&bound),
            "CREATE" => {
                self.ensure_snapshot();
                let statement = parse_create(&bound)?;
                self.db.create(statement)
            }
            "DROP" => {
                self.ensure_snapshot();
                let statement = parse_drop(&bound)?;
                self.db.drop(statement)
            }
            "INSERT" => {
                self.ensure_snapshot();
                let statement = parse_insert(&bound)?;
                self.db.insert(statement)
            }
            "UPDATE" => {
                self.ensure_snapshot();
                let statement = parse_update(&bound)?;
                self.db.update(statement)
            }
            "DELETE" => {
                self.ensure_snapshot();
                let statement = parse_delete(&bound)?;
                self.db.delete(statement)
            }
            _ => Err(MiniSqliteError::ProgrammingError(
                "unsupported SQL statement".to_string(),
            )),
        }
    }

    fn ensure_snapshot(&mut self) {
        if !self.autocommit && self.snapshot.is_none() {
            self.snapshot = Some(self.db.snapshot());
        }
    }

    fn assert_open(&self) -> Result<()> {
        if self.closed {
            return Err(MiniSqliteError::ProgrammingError(
                "connection is closed".to_string(),
            ));
        }
        Ok(())
    }
}

impl InMemoryDatabase {
    fn snapshot(&self) -> Snapshot {
        self.tables.clone()
    }

    fn restore(&mut self, snapshot: Snapshot) {
        self.tables = snapshot;
    }

    fn create(&mut self, statement: CreateTableStatement) -> Result<StatementResult> {
        let key = normalize_name(&statement.table);
        if self.tables.contains_key(&key) {
            if statement.if_not_exists {
                return Ok(empty_statement_result());
            }
            return Err(MiniSqliteError::OperationalError(format!(
                "table already exists: {}",
                statement.table
            )));
        }

        if statement.columns.is_empty() {
            return Err(MiniSqliteError::ProgrammingError(
                "CREATE TABLE requires at least one column".to_string(),
            ));
        }

        let mut seen = HashSet::new();
        for column in &statement.columns {
            let key = normalize_name(column);
            if !seen.insert(key) {
                return Err(MiniSqliteError::ProgrammingError(format!(
                    "duplicate column: {column}"
                )));
            }
        }

        self.tables.insert(
            key,
            TableData {
                columns: statement.columns,
                rows: Vec::new(),
            },
        );
        Ok(empty_statement_result())
    }

    fn drop(&mut self, statement: DropTableStatement) -> Result<StatementResult> {
        let key = normalize_name(&statement.table);
        if !self.tables.contains_key(&key) {
            if statement.if_exists {
                return Ok(empty_statement_result());
            }
            return Err(MiniSqliteError::OperationalError(format!(
                "no such table: {}",
                statement.table
            )));
        }
        self.tables.remove(&key);
        Ok(empty_statement_result())
    }

    fn insert(&mut self, statement: InsertStatement) -> Result<StatementResult> {
        let key = normalize_name(&statement.table);
        let table = self.tables.get_mut(&key).ok_or_else(|| {
            MiniSqliteError::OperationalError(format!("no such table: {}", statement.table))
        })?;

        let columns = if statement.columns.is_empty() {
            table.columns.clone()
        } else {
            canonical_columns(table, &statement.columns)?
        };

        for values in &statement.rows {
            if values.len() != columns.len() {
                return Err(MiniSqliteError::IntegrityError(format!(
                    "INSERT expected {} values, got {}",
                    columns.len(),
                    values.len()
                )));
            }

            let mut row = HashMap::new();
            for column in &table.columns {
                row.insert(column.clone(), None);
            }
            for (column, value) in columns.iter().zip(values.iter()) {
                row.insert(column.clone(), value.clone());
            }
            table.rows.push(row);
        }

        Ok(StatementResult {
            rows_affected: statement.rows.len() as isize,
            ..empty_statement_result()
        })
    }

    fn update(&mut self, statement: UpdateStatement) -> Result<StatementResult> {
        let table = self.table(&statement.table)?.clone();
        let mut assignments = Vec::new();
        for (column, value) in &statement.assignments {
            let canonical = canonical_column(&table, column)?;
            assignments.push((canonical, value.clone()));
        }

        let row_ids = self.matching_row_ids(&statement.table, &statement.where_sql)?;
        let table = self
            .tables
            .get_mut(&normalize_name(&statement.table))
            .unwrap();
        for row_id in &row_ids {
            if let Some(row) = table.rows.get_mut(*row_id) {
                for (column, value) in &assignments {
                    row.insert(column.clone(), value.clone());
                }
            }
        }

        Ok(StatementResult {
            rows_affected: row_ids.len() as isize,
            ..empty_statement_result()
        })
    }

    fn delete(&mut self, statement: DeleteStatement) -> Result<StatementResult> {
        let row_ids = self.matching_row_ids(&statement.table, &statement.where_sql)?;
        let remove: HashSet<usize> = row_ids.iter().copied().collect();
        let table = self
            .tables
            .get_mut(&normalize_name(&statement.table))
            .ok_or_else(|| {
                MiniSqliteError::OperationalError(format!("no such table: {}", statement.table))
            })?;
        let old_len = table.rows.len();
        table
            .rows
            .retain_with_index(|index, _row| !remove.contains(&index));
        Ok(StatementResult {
            rows_affected: (old_len - table.rows.len()) as isize,
            ..empty_statement_result()
        })
    }

    fn select_sql(&self, sql: &str) -> Result<StatementResult> {
        let result = execute_select(sql, self).map_err(map_execution_error)?;
        Ok(query_result_to_statement(result))
    }

    fn matching_row_ids(&self, table_name: &str, where_sql: &str) -> Result<Vec<usize>> {
        let table = self.table(table_name)?.clone();
        if where_sql.trim().is_empty() {
            return Ok((0..table.rows.len()).collect());
        }

        let source = RowIdSource {
            table_name: table_name.to_string(),
            table,
        };
        let sql = format!("SELECT {ROW_ID_COLUMN} FROM {table_name} WHERE {where_sql}");
        let result = execute_select(&sql, &source).map_err(map_execution_error)?;
        let mut ids = Vec::new();
        for row in result.rows {
            if let Some(Some(SqlPrimitive::Int(id))) = row.get(ROW_ID_COLUMN) {
                if *id >= 0 {
                    ids.push(*id as usize);
                }
            }
        }
        Ok(ids)
    }

    fn table(&self, table_name: &str) -> Result<&TableData> {
        self.tables.get(&normalize_name(table_name)).ok_or_else(|| {
            MiniSqliteError::OperationalError(format!("no such table: {table_name}"))
        })
    }
}

impl DataSource for InMemoryDatabase {
    fn schema(&self, table_name: &str) -> std::result::Result<Vec<String>, ExecutionError> {
        self.tables
            .get(&normalize_name(table_name))
            .map(|table| table.columns.clone())
            .ok_or_else(|| ExecutionError::TableNotFound(table_name.to_string()))
    }

    fn scan(
        &self,
        table_name: &str,
    ) -> std::result::Result<Vec<HashMap<String, SqlValue>>, ExecutionError> {
        self.tables
            .get(&normalize_name(table_name))
            .map(|table| table.rows.clone())
            .ok_or_else(|| ExecutionError::TableNotFound(table_name.to_string()))
    }
}

#[derive(Debug, Clone)]
struct RowIdSource {
    table_name: String,
    table: TableData,
}

impl DataSource for RowIdSource {
    fn schema(&self, table_name: &str) -> std::result::Result<Vec<String>, ExecutionError> {
        if normalize_name(table_name) != normalize_name(&self.table_name) {
            return Err(ExecutionError::TableNotFound(table_name.to_string()));
        }
        let mut columns = self.table.columns.clone();
        columns.push(ROW_ID_COLUMN.to_string());
        Ok(columns)
    }

    fn scan(
        &self,
        table_name: &str,
    ) -> std::result::Result<Vec<HashMap<String, SqlValue>>, ExecutionError> {
        if normalize_name(table_name) != normalize_name(&self.table_name) {
            return Err(ExecutionError::TableNotFound(table_name.to_string()));
        }
        Ok(self
            .table
            .rows
            .iter()
            .enumerate()
            .map(|(index, row)| {
                let mut row = row.clone();
                row.insert(ROW_ID_COLUMN.to_string(), int(index as i64));
                row
            })
            .collect())
    }
}

trait RetainWithIndex<T> {
    fn retain_with_index<F>(&mut self, f: F)
    where
        F: FnMut(usize, &mut T) -> bool;
}

impl<T> RetainWithIndex<T> for Vec<T> {
    fn retain_with_index<F>(&mut self, mut f: F)
    where
        F: FnMut(usize, &mut T) -> bool,
    {
        let mut index = 0;
        self.retain_mut(|item| {
            let keep = f(index, item);
            index += 1;
            keep
        });
    }
}

fn query_result_to_statement(result: QueryResult) -> StatementResult {
    let rows = result
        .rows
        .into_iter()
        .map(|row| {
            result
                .columns
                .iter()
                .map(|column| row.get(column).cloned().unwrap_or(None))
                .collect()
        })
        .collect();
    StatementResult {
        columns: result.columns,
        rows,
        rows_affected: -1,
        last_row_id: None,
    }
}

fn empty_statement_result() -> StatementResult {
    StatementResult {
        columns: Vec::new(),
        rows: Vec::new(),
        rows_affected: 0,
        last_row_id: None,
    }
}

fn map_execution_error(error: ExecutionError) -> MiniSqliteError {
    match error {
        ExecutionError::TableNotFound(name) => {
            MiniSqliteError::OperationalError(format!("no such table: {name}"))
        }
        ExecutionError::ColumnNotFound(name) => {
            MiniSqliteError::OperationalError(format!("no such column: {name}"))
        }
        ExecutionError::ParseError(message) => MiniSqliteError::ProgrammingError(message),
        ExecutionError::Other(message) => MiniSqliteError::OperationalError(message),
    }
}

fn canonical_columns(table: &TableData, columns: &[String]) -> Result<Vec<String>> {
    columns
        .iter()
        .map(|column| canonical_column(table, column))
        .collect()
}

fn canonical_column(table: &TableData, column: &str) -> Result<String> {
    let normalized = normalize_name(column);
    table
        .columns
        .iter()
        .find(|candidate| normalize_name(candidate) == normalized)
        .cloned()
        .ok_or_else(|| MiniSqliteError::OperationalError(format!("no such column: {column}")))
}

fn normalize_name(name: &str) -> String {
    name.to_lowercase()
}

fn first_keyword(sql: &str) -> String {
    sql.trim_start()
        .chars()
        .take_while(|ch| ch.is_ascii_alphabetic())
        .collect::<String>()
        .to_uppercase()
}

fn bind_parameters(sql: &str, params: &[SqlValue]) -> Result<String> {
    let bytes = sql.as_bytes();
    let mut out = String::new();
    let mut index = 0;
    let mut i = 0;

    while i < bytes.len() {
        let ch = bytes[i] as char;
        if ch == '\'' || ch == '"' {
            let next = read_quoted(sql, i, ch);
            out.push_str(&sql[i..next]);
            i = next;
            continue;
        }
        if ch == '-' && i + 1 < bytes.len() && bytes[i + 1] == b'-' {
            let next = read_line_comment(sql, i);
            out.push_str(&sql[i..next]);
            i = next;
            continue;
        }
        if ch == '/' && i + 1 < bytes.len() && bytes[i + 1] == b'*' {
            let next = read_block_comment(sql, i);
            out.push_str(&sql[i..next]);
            i = next;
            continue;
        }
        if ch == '?' {
            if index >= params.len() {
                return Err(MiniSqliteError::ProgrammingError(
                    "not enough parameters for SQL statement".to_string(),
                ));
            }
            out.push_str(&to_sql_literal(&params[index])?);
            index += 1;
            i += 1;
            continue;
        }
        out.push(ch);
        i += 1;
    }

    if index != params.len() {
        return Err(MiniSqliteError::ProgrammingError(
            "too many parameters for SQL statement".to_string(),
        ));
    }
    Ok(out)
}

fn read_quoted(sql: &str, start: usize, quote: char) -> usize {
    let bytes = sql.as_bytes();
    let mut i = start + 1;
    while i < bytes.len() {
        let ch = bytes[i] as char;
        if ch == '\\' {
            i += 2;
            continue;
        }
        if ch == quote {
            return i + 1;
        }
        i += 1;
    }
    bytes.len()
}

fn read_line_comment(sql: &str, start: usize) -> usize {
    let bytes = sql.as_bytes();
    let mut i = start + 2;
    while i < bytes.len() && bytes[i] != b'\n' {
        i += 1;
    }
    i
}

fn read_block_comment(sql: &str, start: usize) -> usize {
    let bytes = sql.as_bytes();
    let mut i = start + 2;
    while i + 1 < bytes.len() {
        if bytes[i] == b'*' && bytes[i + 1] == b'/' {
            return i + 2;
        }
        i += 1;
    }
    bytes.len()
}

fn to_sql_literal(value: &SqlValue) -> Result<String> {
    match value {
        None => Ok("NULL".to_string()),
        Some(SqlPrimitive::Bool(true)) => Ok("TRUE".to_string()),
        Some(SqlPrimitive::Bool(false)) => Ok("FALSE".to_string()),
        Some(SqlPrimitive::Int(value)) => Ok(value.to_string()),
        Some(SqlPrimitive::Float(value)) => {
            if !value.is_finite() {
                return Err(MiniSqliteError::ProgrammingError(
                    "non-finite numeric parameter is not supported".to_string(),
                ));
            }
            Ok(value.to_string())
        }
        Some(SqlPrimitive::Text(value)) => Ok(quote_sql_string(value)),
    }
}

fn quote_sql_string(value: &str) -> String {
    let escaped = value
        .replace('\\', "\\\\")
        .replace('\'', "\\'")
        .replace('\n', "\\n")
        .replace('\t', "\\t");
    format!("'{escaped}'")
}

fn parse_create(sql: &str) -> Result<CreateTableStatement> {
    let mut stream = TokenStream::new(sql)?;
    stream.expect_keyword("CREATE")?;
    stream.expect_keyword("TABLE")?;
    let if_not_exists =
        stream.match_keyword("IF") && stream.match_keyword("NOT") && stream.match_keyword("EXISTS");
    let table = stream.expect_name()?;
    stream.expect_type(TokenType::LParen)?;

    let mut columns = Vec::new();
    while !stream.at_end() {
        if stream.match_type(TokenType::RParen) {
            break;
        }
        columns.push(stream.expect_name()?);
        stream.skip_column_definition()?;
        if stream.match_type(TokenType::Comma) {
            continue;
        }
        stream.expect_type(TokenType::RParen)?;
        break;
    }
    stream.expect_end()?;

    Ok(CreateTableStatement {
        table,
        columns,
        if_not_exists,
    })
}

fn parse_drop(sql: &str) -> Result<DropTableStatement> {
    let mut stream = TokenStream::new(sql)?;
    stream.expect_keyword("DROP")?;
    stream.expect_keyword("TABLE")?;
    let if_exists = stream.match_keyword("IF") && stream.match_keyword("EXISTS");
    let table = stream.expect_name()?;
    stream.expect_end()?;
    Ok(DropTableStatement { table, if_exists })
}

fn parse_insert(sql: &str) -> Result<InsertStatement> {
    let mut stream = TokenStream::new(sql)?;
    stream.expect_keyword("INSERT")?;
    stream.expect_keyword("INTO")?;
    let table = stream.expect_name()?;

    let mut columns = Vec::new();
    if stream.match_type(TokenType::LParen) {
        loop {
            columns.push(stream.expect_name()?);
            if stream.match_type(TokenType::Comma) {
                continue;
            }
            stream.expect_type(TokenType::RParen)?;
            break;
        }
    }

    stream.expect_keyword("VALUES")?;
    let mut rows = Vec::new();
    loop {
        rows.push(stream.parse_value_row()?);
        if stream.match_type(TokenType::Comma) {
            continue;
        }
        break;
    }
    stream.expect_end()?;

    Ok(InsertStatement {
        table,
        columns,
        rows,
    })
}

fn parse_update(sql: &str) -> Result<UpdateStatement> {
    let mut stream = TokenStream::new(sql)?;
    stream.expect_keyword("UPDATE")?;
    let table = stream.expect_name()?;
    stream.expect_keyword("SET")?;

    let mut assignments = Vec::new();
    loop {
        let column = stream.expect_name()?;
        stream.expect_type(TokenType::Equals)?;
        let value = stream.parse_literal()?;
        assignments.push((column, value));
        if stream.match_type(TokenType::Comma) {
            continue;
        }
        break;
    }

    let where_sql = if stream.match_keyword("WHERE") {
        stream.remaining_sql()
    } else {
        String::new()
    };
    stream.consume_remaining();

    if assignments.is_empty() {
        return Err(MiniSqliteError::ProgrammingError(
            "UPDATE requires at least one assignment".to_string(),
        ));
    }

    Ok(UpdateStatement {
        table,
        assignments,
        where_sql,
    })
}

fn parse_delete(sql: &str) -> Result<DeleteStatement> {
    let mut stream = TokenStream::new(sql)?;
    stream.expect_keyword("DELETE")?;
    stream.expect_keyword("FROM")?;
    let table = stream.expect_name()?;
    let where_sql = if stream.match_keyword("WHERE") {
        stream.remaining_sql()
    } else {
        String::new()
    };
    stream.consume_remaining();
    Ok(DeleteStatement { table, where_sql })
}

struct TokenStream {
    tokens: Vec<Token>,
    pos: usize,
}

impl TokenStream {
    fn new(sql: &str) -> Result<Self> {
        let mut tokens = tokenize_sql(sql)
            .map_err(|message| MiniSqliteError::ProgrammingError(message))?
            .into_iter()
            .filter(|token| token.type_ != TokenType::Eof)
            .collect::<Vec<_>>();
        while matches!(tokens.last(), Some(token) if token.type_ == TokenType::Semicolon) {
            tokens.pop();
        }
        Ok(Self { tokens, pos: 0 })
    }

    fn at_end(&self) -> bool {
        self.pos >= self.tokens.len()
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn advance(&mut self) -> Option<Token> {
        let token = self.tokens.get(self.pos).cloned();
        if token.is_some() {
            self.pos += 1;
        }
        token
    }

    fn match_keyword(&mut self, keyword: &str) -> bool {
        if self.peek().is_some_and(|token| is_keyword(token, keyword)) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn expect_keyword(&mut self, keyword: &str) -> Result<()> {
        if self.match_keyword(keyword) {
            return Ok(());
        }
        Err(MiniSqliteError::ProgrammingError(format!(
            "expected keyword {keyword}"
        )))
    }

    fn match_type(&mut self, token_type: TokenType) -> bool {
        if self.peek().is_some_and(|token| token.type_ == token_type) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn expect_type(&mut self, token_type: TokenType) -> Result<()> {
        if self.match_type(token_type) {
            return Ok(());
        }
        Err(MiniSqliteError::ProgrammingError(format!(
            "expected token {token_type}"
        )))
    }

    fn expect_name(&mut self) -> Result<String> {
        let token = self
            .advance()
            .ok_or_else(|| MiniSqliteError::ProgrammingError("expected identifier".to_string()))?;
        if token.type_ == TokenType::Name {
            Ok(strip_quoted_identifier(&token.value))
        } else {
            Err(MiniSqliteError::ProgrammingError(format!(
                "expected identifier, got {}",
                token.value
            )))
        }
    }

    fn expect_end(&self) -> Result<()> {
        if self.at_end() {
            Ok(())
        } else {
            Err(MiniSqliteError::ProgrammingError(format!(
                "unexpected token: {}",
                self.peek().map(|token| token.value.as_str()).unwrap_or("")
            )))
        }
    }

    fn skip_column_definition(&mut self) -> Result<()> {
        let mut depth = 0usize;
        while let Some(token) = self.peek() {
            if depth == 0 && (token.type_ == TokenType::Comma || token.type_ == TokenType::RParen) {
                return Ok(());
            }
            match token.type_ {
                TokenType::LParen => depth += 1,
                TokenType::RParen => {
                    if depth == 0 {
                        return Ok(());
                    }
                    depth -= 1;
                }
                _ => {}
            }
            self.pos += 1;
        }
        Err(MiniSqliteError::ProgrammingError(
            "unterminated CREATE TABLE column list".to_string(),
        ))
    }

    fn parse_value_row(&mut self) -> Result<Vec<SqlValue>> {
        self.expect_type(TokenType::LParen)?;
        let mut values = Vec::new();
        loop {
            if self
                .peek()
                .is_some_and(|token| token.type_ == TokenType::RParen)
            {
                if values.is_empty() {
                    return Err(MiniSqliteError::ProgrammingError(
                        "INSERT row requires at least one value".to_string(),
                    ));
                }
                self.pos += 1;
                break;
            }
            values.push(self.parse_literal()?);
            if self.match_type(TokenType::Comma) {
                continue;
            }
            self.expect_type(TokenType::RParen)?;
            break;
        }
        Ok(values)
    }

    fn parse_literal(&mut self) -> Result<SqlValue> {
        let negative = self.match_type(TokenType::Minus);
        let token = self.advance().ok_or_else(|| {
            MiniSqliteError::ProgrammingError("expected literal value".to_string())
        })?;
        match token.type_ {
            TokenType::Keyword if token.value == "NULL" && !negative => Ok(None),
            TokenType::Keyword if token.value == "TRUE" && !negative => Ok(boolean(true)),
            TokenType::Keyword if token.value == "FALSE" && !negative => Ok(boolean(false)),
            TokenType::Number => {
                let raw = if negative {
                    format!("-{}", token.value)
                } else {
                    token.value
                };
                if raw.contains('.') {
                    raw.parse::<f64>().map(real).map_err(|_| {
                        MiniSqliteError::ProgrammingError(format!("invalid number: {raw}"))
                    })
                } else {
                    raw.parse::<i64>().map(int).map_err(|_| {
                        MiniSqliteError::ProgrammingError(format!("invalid number: {raw}"))
                    })
                }
            }
            TokenType::String if !negative => Ok(text(token.value)),
            _ => Err(MiniSqliteError::ProgrammingError(format!(
                "expected literal value, got {}",
                token.value
            ))),
        }
    }

    fn remaining_sql(&self) -> String {
        tokens_to_sql(&self.tokens[self.pos..])
    }

    fn consume_remaining(&mut self) {
        self.pos = self.tokens.len();
    }
}

fn is_keyword(token: &Token, keyword: &str) -> bool {
    token.type_ == TokenType::Keyword && token.value.eq_ignore_ascii_case(keyword)
}

fn tokens_to_sql(tokens: &[Token]) -> String {
    tokens
        .iter()
        .map(token_to_sql)
        .collect::<Vec<_>>()
        .join(" ")
}

fn token_to_sql(token: &Token) -> String {
    if token.type_ == TokenType::String {
        quote_sql_string(&token.value)
    } else {
        token.value.clone()
    }
}

fn strip_quoted_identifier(value: &str) -> String {
    if value.starts_with('`') && value.ends_with('`') && value.len() >= 2 {
        value[1..value.len() - 1].to_string()
    } else {
        value.to_string()
    }
}
