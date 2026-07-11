//! `sqlite_schema` lookup and table-by-name reads.
//!
//! Low-level callers can use [`crate::btree::walk_table`] directly when they
//! already know a root page. Most import code, though, starts with a table name:
//! `col`, `notes`, `cards`, `revlog`, or `graves`. This module is that public
//! convenience layer. It reads the `sqlite_schema` table rooted on page 1,
//! resolves a table name to its root page, walks that table b-tree, and decodes
//! each row into [`SqlValue`] columns.

use crate::btree;
use crate::error::SqliteError;
use crate::header::Header;
use crate::pager::Pager;
use crate::record::{self, SqlValue};

/// One decoded row from `sqlite_schema`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SchemaEntry {
    /// Object kind, such as `table`, `index`, `view`, or `trigger`.
    pub object_type: String,
    /// Object name.
    pub name: String,
    /// Table name this object belongs to.
    pub table_name: String,
    /// Root b-tree page for table/index objects. Views/triggers use no root.
    pub root_page: Option<u32>,
    /// Original SQL text, if SQLite stored one for this object.
    pub sql: Option<String>,
}

/// Decode every row from `sqlite_schema`.
pub fn read_schema(data: &[u8]) -> Result<Vec<SchemaEntry>, SqliteError> {
    let (header, pager) = Pager::open(data)?;
    read_schema_from(&pager, &header)
}

/// Return the root page for a table named `name`.
pub fn table_root_page(data: &[u8], name: &str) -> Result<u32, SqliteError> {
    let (header, pager) = Pager::open(data)?;
    table_root_page_from(&pager, &header, name)
}

/// Read a table by name, returning `(rowid, decoded columns)` in rowid order.
///
/// For an `INTEGER PRIMARY KEY` column, SQLite stores the primary-key value as
/// the rowid and leaves that record column as `NULL`; callers should use the
/// returned rowid when comparing such columns.
pub fn read_table(data: &[u8], name: &str) -> Result<Vec<(i64, Vec<SqlValue>)>, SqliteError> {
    let (header, pager) = Pager::open(data)?;
    let root_page = table_root_page_from(&pager, &header, name)?;
    let rows = btree::walk_table(&pager, &header, root_page)?;
    rows.into_iter()
        .map(|(rowid, record)| {
            let columns = record::decode(&record).ok_or(SqliteError::Corrupt("bad table row"))?;
            Ok((rowid, columns))
        })
        .collect()
}

/// Read a `WITHOUT ROWID` table by name, returning each row's decoded columns.
///
/// A `WITHOUT ROWID` table has no rowid: it stores its rows in an *index* b-tree
/// keyed by the primary key, so this walks that tree via
/// [`crate::btree::walk_index`] rather than [`crate::btree::walk_table`]. Each
/// record still holds every column in the table's declared order — exactly like
/// an ordinary table's record, only the rowid is absent — so no column needs to
/// be reconstructed from a rowid. Order is unspecified (an index b-tree yields no
/// natural rowid ordering); callers sort in SQL when it matters.
pub fn read_without_rowid_table(data: &[u8], name: &str) -> Result<Vec<Vec<SqlValue>>, SqliteError> {
    let (header, pager) = Pager::open(data)?;
    let root_page = table_root_page_from(&pager, &header, name)?;
    let records = btree::walk_index(&pager, &header, root_page)?;
    records
        .into_iter()
        .map(|record| record::decode(&record).ok_or(SqliteError::Corrupt("bad table row")))
        .collect()
}

fn read_schema_from(pager: &Pager<'_>, header: &Header) -> Result<Vec<SchemaEntry>, SqliteError> {
    let rows = btree::walk_table(pager, header, 1)?;
    rows.into_iter()
        .map(|(_rowid, record)| decode_schema_row(&record))
        .collect()
}

fn table_root_page_from(
    pager: &Pager<'_>,
    header: &Header,
    name: &str,
) -> Result<u32, SqliteError> {
    for entry in read_schema_from(pager, header)? {
        if entry.object_type == "table" && entry.name == name {
            return entry
                .root_page
                .ok_or(SqliteError::Corrupt("table without root page"));
        }
    }
    Err(SqliteError::NoSuchTable(name.to_string()))
}

fn decode_schema_row(record: &[u8]) -> Result<SchemaEntry, SqliteError> {
    let columns = record::decode(record).ok_or(SqliteError::Corrupt("bad schema row"))?;
    if columns.len() != 5 {
        return Err(SqliteError::Corrupt("bad schema column count"));
    }

    let object_type = expect_text(&columns[0], "schema type")?;
    let name = expect_text(&columns[1], "schema name")?;
    let table_name = expect_text(&columns[2], "schema table name")?;
    let root_page = match &columns[3] {
        SqlValue::Null => None,
        SqlValue::Int(n) if *n == 0 => None,
        SqlValue::Int(n) => {
            Some(u32::try_from(*n).map_err(|_| SqliteError::Corrupt("bad schema root page"))?)
        }
        _ => return Err(SqliteError::Corrupt("bad schema root page")),
    };
    let sql = match &columns[4] {
        SqlValue::Null => None,
        SqlValue::Text(s) => Some(s.clone()),
        _ => return Err(SqliteError::Corrupt("bad schema sql")),
    };

    Ok(SchemaEntry {
        object_type,
        name,
        table_name,
        root_page,
        sql,
    })
}

fn expect_text(value: &SqlValue, what: &'static str) -> Result<String, SqliteError> {
    match value {
        SqlValue::Text(s) => Ok(s.clone()),
        _ => Err(SqliteError::Corrupt(what)),
    }
}
