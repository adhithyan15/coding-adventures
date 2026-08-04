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

impl SchemaEntry {
    /// Serialise this entry into the five `sqlite_schema` columns, in order —
    /// the exact inverse of the private `decode_schema_row`, so that decoding the
    /// [`record::encode`] of these columns reconstructs the entry. The columns
    /// are `type TEXT`, `name TEXT`, `tbl_name TEXT`, `rootpage INTEGER`,
    /// `sql TEXT`; a `None` root page or SQL is written as `NULL`.
    ///
    /// This is a *write*-path helper: paired with [`record::encode`],
    /// [`crate::page_writer`], and [`Header::encode`], a caller can lay down the
    /// `sqlite_schema` b-tree on page 1 that describes a user table — the last
    /// piece needed to emit a file our own reader resolves by table name.
    pub fn to_record_columns(&self) -> Vec<SqlValue> {
        vec![
            SqlValue::Text(self.object_type.clone()),
            SqlValue::Text(self.name.clone()),
            SqlValue::Text(self.table_name.clone()),
            match self.root_page {
                Some(page) => SqlValue::Int(i64::from(page)),
                None => SqlValue::Null,
            },
            match &self.sql {
                Some(sql) => SqlValue::Text(sql.clone()),
                None => SqlValue::Null,
            },
        ]
    }
}

/// Build the `sqlite_schema` row describing an ordinary rowid `table` — the
/// common case for emitting a single-table database. Returns the five schema
/// columns for `type='table'`, `name = tbl_name = name`, the given root page, and
/// the `CREATE TABLE` SQL. Feed the result to [`record::encode`] to get the cell
/// payload for the schema b-tree.
pub fn table_schema_row(name: &str, root_page: u32, sql: &str) -> Vec<SqlValue> {
    SchemaEntry {
        object_type: "table".to_string(),
        name: name.to_string(),
        table_name: name.to_string(),
        root_page: Some(root_page),
        sql: Some(sql.to_string()),
    }
    .to_record_columns()
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::header::{Header, TextEncoding, MAGIC};
    use crate::page_writer::encode_table_leaf_page;
    use crate::{record, varint};

    /// `to_record_columns` is the inverse of `decode_schema_row`: a schema row
    /// encoded and decoded through the record codec reconstructs the entry.
    #[test]
    fn schema_row_round_trips_through_the_record_codec() {
        let entry = SchemaEntry {
            object_type: "table".to_string(),
            name: "widgets".to_string(),
            table_name: "widgets".to_string(),
            root_page: Some(4),
            sql: Some("CREATE TABLE widgets(id, label)".to_string()),
        };
        let bytes = record::encode(&entry.to_record_columns());
        let decoded = decode_schema_row(&bytes).unwrap();
        assert_eq!(decoded, entry);

        // The convenience builder produces the same five columns for a table.
        assert_eq!(
            table_schema_row("widgets", 4, "CREATE TABLE widgets(id, label)"),
            entry.to_record_columns()
        );
    }

    /// The milestone: assemble a full two-page single-table database entirely
    /// from the writer helpers — page 1 = the 100-byte DB header followed by a
    /// `sqlite_schema` leaf (at offset 100) describing table `t` rooted on page
    /// 2, and page 2 = a data leaf holding `t`'s rows — then read it back through
    /// the real reader (`schema::read_table`) by table name.
    #[test]
    fn writes_a_readable_single_table_database() {
        let ps = 512usize;

        // --- Page 2: the user table's data leaf, via the page writer. ---------
        let rows: Vec<(i64, Vec<u8>)> = [
            (1i64, vec![SqlValue::Int(1), SqlValue::Text("a".into())]),
            (2, vec![SqlValue::Int(2), SqlValue::Text("b".into())]),
        ]
        .iter()
        .map(|(rowid, cols)| (*rowid, record::encode(cols)))
        .collect();
        let data_leaf = encode_table_leaf_page(ps, 0, &rows).unwrap();

        // --- Page 1: DB header (100 bytes) + sqlite_schema leaf at offset 100. -
        // The schema b-tree has one cell: rowid 1, the record describing table
        // `t` at root page 2. Page 1 is the only page whose b-tree header is
        // offset by the 100-byte database header, so we lay the leaf out by hand.
        let header = Header {
            page_size: ps as u32,
            reserved_space: 0,
            page_count: 2,
            change_counter: 1,
            freelist_trunk: 0,
            freelist_count: 0,
            schema_cookie: 1,
            schema_format: 1,
            text_encoding: TextEncoding::Utf8,
        };
        let mut page1 = vec![0u8; ps];
        page1[0..100].copy_from_slice(&header.encode());
        assert_eq!(&page1[0..16], MAGIC);

        let schema_record =
            record::encode(&table_schema_row("t", 2, "CREATE TABLE t(id, v)"));
        let mut cell = Vec::new();
        varint::write(schema_record.len() as i64, &mut cell); // payload length
        varint::write(1, &mut cell); // rowid
        cell.extend_from_slice(&schema_record);

        let h = 100; // page-1 b-tree header offset
        page1[h] = 0x0D; // leaf table page
        page1[h + 3..h + 5].copy_from_slice(&1u16.to_be_bytes()); // one cell
        let content_top = ps - cell.len();
        page1[content_top..ps].copy_from_slice(&cell);
        page1[h + 5..h + 7].copy_from_slice(&(content_top as u16).to_be_bytes()); // content start
        page1[h + 8..h + 10].copy_from_slice(&(content_top as u16).to_be_bytes()); // cell ptr[0]

        // --- Assemble the file and read table `t` back by name. ---------------
        let mut file = page1;
        file.extend_from_slice(&data_leaf);

        let read = read_table(&file, "t").unwrap();
        assert_eq!(
            read,
            vec![
                (1, vec![SqlValue::Int(1), SqlValue::Text("a".into())]),
                (2, vec![SqlValue::Int(2), SqlValue::Text("b".into())]),
            ]
        );

        // The schema itself reads back as one table entry rooted on page 2.
        let schema = read_schema(&file).unwrap();
        assert_eq!(schema.len(), 1);
        assert_eq!(schema[0].object_type, "table");
        assert_eq!(schema[0].name, "t");
        assert_eq!(schema[0].root_page, Some(2));
    }
}
