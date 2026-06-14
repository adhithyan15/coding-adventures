//! # Data Frame — R-style tabular type.
//!
//! A data frame is a list of equal-length, typed, named columns. It is
//! the most-used data structure in R (`data.frame`), the input shape of
//! every statistical model, the row-of-cells abstraction over a
//! spreadsheet range, and the Rust analog of a Pandas DataFrame or
//! Polars DataFrame — but smaller, since this crate carries no I/O, no
//! query engine, no SQL, just the column-aligned container.
//!
//! Three invariants the crate enforces at construction and preserves
//! across every operation:
//!
//! 1. **Every column has the same length.** A data frame with 5 rows
//!    has 5-element columns, no exceptions.
//! 2. **Column names are unique and ASCII case-sensitive.** Duplicate
//!    names produce `DataFrameError::DuplicateColumn` at construction
//!    rather than silent shadowing.
//! 3. **Row names, if present, are unique and equal in count to nrow.**
//!    Row names are optional; when absent the crate uses positional
//!    indexing (0-based internally, 1-based at the public surface to
//!    match R/Excel).
//!
//! The column types follow `r-vector`'s atomic-type set. Phase 1 of
//! r-vector ships `Double` (NA-aware f64) and `Character`
//! (`Vec<Option<String>>`); future r-vector phases will add `Logical`,
//! `Integer`, `Complex`, and `Raw`, and this crate will extend the
//! `Column` enum to match.
//!
//! ## Portability
//!
//! Per `backend-crate-catalog.md` §1: no `unsafe`, no platform-specific
//! code, no file I/O, no global state, WASM-compatible. Crate
//! depends only on `r-vector` and `numeric-tower`.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use r_vector::{Character, Double, Vector as _};

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors raised by the data-frame crate. All variants are recoverable
/// by the caller (no panics, no aborts).
#[derive(Debug, Clone, PartialEq)]
pub enum DataFrameError {
    /// Construction received columns of mismatched length.
    UnequalColumnLengths {
        /// Name of the column whose length differs.
        column: String,
        /// Expected length (from the first column or explicit nrow).
        expected: usize,
        /// Actual length of this column.
        found: usize,
    },
    /// Construction received two columns with the same name.
    DuplicateColumn {
        /// The duplicated name.
        name: String,
    },
    /// Construction received empty column names or names not matching
    /// columns one-to-one.
    NameCountMismatch {
        /// How many columns were supplied.
        columns: usize,
        /// How many names were supplied.
        names: usize,
    },
    /// Lookup by column name failed.
    ColumnNotFound {
        /// The name that was queried.
        name: String,
    },
    /// Lookup by column index failed.
    ColumnIndexOutOfRange {
        /// The 0-based index that was queried.
        index: usize,
        /// The number of columns in the frame.
        ncol: usize,
    },
    /// Row index out of range.
    RowIndexOutOfRange {
        /// The 0-based row index queried.
        index: usize,
        /// The number of rows in the frame.
        nrow: usize,
    },
    /// Row-mask length mismatch in subset_rows.
    MaskLengthMismatch {
        /// Expected length (== nrow).
        expected: usize,
        /// Actual mask length supplied.
        found: usize,
    },
    /// rbind got frames with incompatible column schemas.
    SchemaMismatch {
        /// Description of the mismatch.
        what: String,
    },
}

impl core::fmt::Display for DataFrameError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            DataFrameError::UnequalColumnLengths {
                column,
                expected,
                found,
            } => write!(
                f,
                "column '{column}': expected length {expected}, found {found}"
            ),
            DataFrameError::DuplicateColumn { name } => {
                write!(f, "duplicate column name '{name}'")
            }
            DataFrameError::NameCountMismatch { columns, names } => {
                write!(f, "name count mismatch: {columns} columns, {names} names")
            }
            DataFrameError::ColumnNotFound { name } => {
                write!(f, "column '{name}' not found")
            }
            DataFrameError::ColumnIndexOutOfRange { index, ncol } => {
                write!(f, "column index {index} out of range (ncol = {ncol})")
            }
            DataFrameError::RowIndexOutOfRange { index, nrow } => {
                write!(f, "row index {index} out of range (nrow = {nrow})")
            }
            DataFrameError::MaskLengthMismatch { expected, found } => {
                write!(
                    f,
                    "row mask length mismatch: expected {expected}, found {found}"
                )
            }
            DataFrameError::SchemaMismatch { what } => write!(f, "schema mismatch: {what}"),
        }
    }
}

impl std::error::Error for DataFrameError {}

// ---------------------------------------------------------------------------
// Column — one typed column
// ---------------------------------------------------------------------------

/// One column of a data frame, tagged by atomic type.
///
/// This enum is the small-and-extensible variant: it currently lists
/// only the atomic types that `r-vector` Phase 1 ships (`Double`,
/// `Character`). Future r-vector phases will add `Logical`, `Integer`,
/// `Complex`, `Raw` — this enum will grow to match. The `#[non_exhaustive]`
/// attribute lets us add variants without breaking downstream code that
/// only matches the variants it understands.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Column {
    /// NA-aware floating-point column.
    Double(Double),
    /// UTF-8 string column with explicit `None` for NA slots.
    Character(Character),
}

impl Column {
    /// Number of elements in the column.
    pub fn len(&self) -> usize {
        match self {
            Column::Double(d) => d.len(),
            Column::Character(c) => c.len(),
        }
    }

    /// Whether the column is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Name of the atomic type, matching R's storage modes.
    pub fn type_name(&self) -> &'static str {
        match self {
            Column::Double(_) => "double",
            Column::Character(_) => "character",
        }
    }

    /// Whether the element at the given 0-based index is NA.
    /// Returns `false` for out-of-range indices (caller is expected to
    /// have bounds-checked first; this method panics in debug builds).
    pub fn is_na(&self, i: usize) -> bool {
        match self {
            Column::Double(d) => {
                debug_assert!(i < d.len(), "Column::is_na index out of range");
                d.is_na(i)
            }
            Column::Character(c) => {
                debug_assert!(i < c.len(), "Column::is_na index out of range");
                c.is_na(i)
            }
        }
    }
}

// Provide ergonomic From impls so callers can write
// `Column::from(double_vec)` or just pass `double_vec.into()` into a
// DataFrame constructor.
impl From<Double> for Column {
    fn from(d: Double) -> Self {
        Column::Double(d)
    }
}

impl From<Character> for Column {
    fn from(c: Character) -> Self {
        Column::Character(c)
    }
}

// ---------------------------------------------------------------------------
// DataFrame
// ---------------------------------------------------------------------------

/// An R-style data frame: a list of equal-length, typed, named columns.
#[derive(Debug, Clone, PartialEq)]
pub struct DataFrame {
    columns: Vec<Column>,
    column_names: Vec<String>,
    row_names: Option<Vec<String>>,
    nrow: usize,
}

impl DataFrame {
    /// Construct an empty data frame with zero columns and zero rows.
    pub fn empty() -> Self {
        Self {
            columns: Vec::new(),
            column_names: Vec::new(),
            row_names: None,
            nrow: 0,
        }
    }

    /// Construct from `(name, column)` pairs. Validates that all
    /// columns have the same length and that names are unique.
    ///
    /// ```
    /// use data_frame::{Column, DataFrame};
    /// use r_vector::{Character, Double};
    ///
    /// let ages = Double::from_values(vec![42.0, 17.0, 99.0]);
    /// let names = Character::from_strings(vec!["alice", "bob", "carol"]);
    /// let df = DataFrame::from_columns(vec![
    ///     ("age".to_string(),  Column::Double(ages)),
    ///     ("name".to_string(), Column::Character(names)),
    /// ]).expect("schema is valid");
    /// assert_eq!(df.nrow(), 3);
    /// assert_eq!(df.ncol(), 2);
    /// ```
    pub fn from_columns(pairs: Vec<(String, Column)>) -> Result<Self, DataFrameError> {
        if pairs.is_empty() {
            return Ok(Self::empty());
        }

        // First column determines nrow.
        let nrow = pairs[0].1.len();

        // Enforce equal lengths.
        for (name, column) in &pairs {
            if column.len() != nrow {
                return Err(DataFrameError::UnequalColumnLengths {
                    column: name.clone(),
                    expected: nrow,
                    found: column.len(),
                });
            }
        }

        // Enforce unique names. Linear scan because column counts in
        // practice are < 100; HashSet would be wasteful.
        for i in 0..pairs.len() {
            for j in (i + 1)..pairs.len() {
                if pairs[i].0 == pairs[j].0 {
                    return Err(DataFrameError::DuplicateColumn {
                        name: pairs[i].0.clone(),
                    });
                }
            }
        }

        let mut columns = Vec::with_capacity(pairs.len());
        let mut column_names = Vec::with_capacity(pairs.len());
        for (name, column) in pairs {
            column_names.push(name);
            columns.push(column);
        }

        Ok(Self {
            columns,
            column_names,
            row_names: None,
            nrow,
        })
    }

    /// Attach row names. Length must equal `nrow`.
    pub fn with_row_names(mut self, names: Vec<String>) -> Result<Self, DataFrameError> {
        if names.len() != self.nrow {
            return Err(DataFrameError::MaskLengthMismatch {
                expected: self.nrow,
                found: names.len(),
            });
        }
        self.row_names = Some(names);
        Ok(self)
    }

    /// Number of rows.
    pub fn nrow(&self) -> usize {
        self.nrow
    }

    /// Number of columns.
    pub fn ncol(&self) -> usize {
        self.columns.len()
    }

    /// Whether the frame has zero rows AND zero columns.
    pub fn is_empty(&self) -> bool {
        self.nrow == 0 && self.columns.is_empty()
    }

    /// Iterate over column names in their stored order.
    pub fn column_names(&self) -> &[String] {
        &self.column_names
    }

    /// Iterate over columns in their stored order.
    pub fn columns(&self) -> &[Column] {
        &self.columns
    }

    /// Row names, if attached. R-style frames may or may not name rows.
    pub fn row_names(&self) -> Option<&[String]> {
        self.row_names.as_deref()
    }

    /// Get a column by name.
    pub fn column(&self, name: &str) -> Result<&Column, DataFrameError> {
        self.column_names
            .iter()
            .position(|n| n == name)
            .map(|i| &self.columns[i])
            .ok_or_else(|| DataFrameError::ColumnNotFound {
                name: name.to_string(),
            })
    }

    /// Get a column by 0-based index.
    pub fn column_at(&self, index: usize) -> Result<&Column, DataFrameError> {
        self.columns
            .get(index)
            .ok_or(DataFrameError::ColumnIndexOutOfRange {
                index,
                ncol: self.columns.len(),
            })
    }

    /// Whether a column with the given name exists.
    pub fn has_column(&self, name: &str) -> bool {
        self.column_names.iter().any(|n| n == name)
    }

    /// Add a new column to the right end. Errors if the column length
    /// does not match `nrow` (or, when the frame is empty, sets `nrow`
    /// from the column's length).
    pub fn add_column(&mut self, name: String, column: Column) -> Result<(), DataFrameError> {
        if self.has_column(&name) {
            return Err(DataFrameError::DuplicateColumn { name });
        }
        if self.columns.is_empty() {
            self.nrow = column.len();
        } else if column.len() != self.nrow {
            return Err(DataFrameError::UnequalColumnLengths {
                column: name,
                expected: self.nrow,
                found: column.len(),
            });
        }
        self.columns.push(column);
        self.column_names.push(name);
        Ok(())
    }

    /// Remove a column by name. Returns the removed column.
    pub fn remove_column(&mut self, name: &str) -> Result<Column, DataFrameError> {
        let i = self
            .column_names
            .iter()
            .position(|n| n == name)
            .ok_or_else(|| DataFrameError::ColumnNotFound {
                name: name.to_string(),
            })?;
        self.column_names.remove(i);
        Ok(self.columns.remove(i))
    }

    /// Rename a column. Errors if the new name collides.
    pub fn rename_column(&mut self, old: &str, new: String) -> Result<(), DataFrameError> {
        if old == new {
            return Ok(());
        }
        if self.has_column(&new) {
            return Err(DataFrameError::DuplicateColumn { name: new });
        }
        let i = self
            .column_names
            .iter()
            .position(|n| n == old)
            .ok_or_else(|| DataFrameError::ColumnNotFound {
                name: old.to_string(),
            })?;
        self.column_names[i] = new;
        Ok(())
    }

    /// Select a subset of columns by name. Order matches the supplied
    /// list. Names must be unique within the list and must exist in the
    /// source frame.
    pub fn select(&self, names: &[&str]) -> Result<DataFrame, DataFrameError> {
        // Check duplicates in the request.
        for i in 0..names.len() {
            for j in (i + 1)..names.len() {
                if names[i] == names[j] {
                    return Err(DataFrameError::DuplicateColumn {
                        name: names[i].to_string(),
                    });
                }
            }
        }

        let mut out_columns = Vec::with_capacity(names.len());
        let mut out_names = Vec::with_capacity(names.len());
        for &name in names {
            let column = self.column(name)?.clone();
            out_columns.push(column);
            out_names.push(name.to_string());
        }

        Ok(DataFrame {
            columns: out_columns,
            column_names: out_names,
            row_names: self.row_names.clone(),
            nrow: self.nrow,
        })
    }

    /// Subset rows by a boolean mask. Mask length must equal `nrow`.
    /// `None` mask positions are treated as `false` (matches Excel
    /// FILTER semantics).
    pub fn subset_rows(&self, mask: &[Option<bool>]) -> Result<DataFrame, DataFrameError> {
        if mask.len() != self.nrow {
            return Err(DataFrameError::MaskLengthMismatch {
                expected: self.nrow,
                found: mask.len(),
            });
        }

        // Compute kept indices once; reuse for every column.
        let kept: Vec<usize> = mask
            .iter()
            .enumerate()
            .filter_map(|(i, m)| if *m == Some(true) { Some(i) } else { None })
            .collect();

        let columns = self
            .columns
            .iter()
            .map(|col| column_take_indices(col, &kept))
            .collect();

        let row_names = self.row_names.as_ref().map(|names| {
            kept.iter()
                .map(|&i| names[i].clone())
                .collect::<Vec<String>>()
        });

        Ok(DataFrame {
            columns,
            column_names: self.column_names.clone(),
            row_names,
            nrow: kept.len(),
        })
    }

    /// Subset rows by an explicit list of 0-based indices. Indices may
    /// repeat (the row is included multiple times) and may appear in any
    /// order.
    pub fn take_rows(&self, indices: &[usize]) -> Result<DataFrame, DataFrameError> {
        for &i in indices {
            if i >= self.nrow {
                return Err(DataFrameError::RowIndexOutOfRange {
                    index: i,
                    nrow: self.nrow,
                });
            }
        }

        let columns = self
            .columns
            .iter()
            .map(|col| column_take_indices(col, indices))
            .collect();

        let row_names = self.row_names.as_ref().map(|names| {
            indices
                .iter()
                .map(|&i| names[i].clone())
                .collect::<Vec<String>>()
        });

        Ok(DataFrame {
            columns,
            column_names: self.column_names.clone(),
            row_names,
            nrow: indices.len(),
        })
    }

    /// Vertical concat (R's `rbind`). Both frames must have the same
    /// columns (by name and type).
    pub fn rbind(&self, other: &DataFrame) -> Result<DataFrame, DataFrameError> {
        if self.column_names != other.column_names {
            return Err(DataFrameError::SchemaMismatch {
                what: format!(
                    "column names differ: {:?} vs {:?}",
                    self.column_names, other.column_names
                ),
            });
        }

        let mut columns = Vec::with_capacity(self.columns.len());
        for (a, b) in self.columns.iter().zip(other.columns.iter()) {
            columns.push(column_concat(a, b)?);
        }

        // Concatenate row names if both have them; drop if either lacks.
        let row_names = match (&self.row_names, &other.row_names) {
            (Some(a), Some(b)) => Some([&a[..], &b[..]].concat()),
            _ => None,
        };

        Ok(DataFrame {
            columns,
            column_names: self.column_names.clone(),
            row_names,
            nrow: self.nrow + other.nrow,
        })
    }

    /// Horizontal concat (R's `cbind`). Frames must have the same nrow.
    /// Column names must not collide.
    pub fn cbind(&self, other: &DataFrame) -> Result<DataFrame, DataFrameError> {
        if self.nrow != other.nrow {
            return Err(DataFrameError::SchemaMismatch {
                what: format!("nrow differs: {} vs {}", self.nrow, other.nrow),
            });
        }
        for name in &other.column_names {
            if self.has_column(name) {
                return Err(DataFrameError::DuplicateColumn { name: name.clone() });
            }
        }

        let mut columns = self.columns.clone();
        columns.extend(other.columns.iter().cloned());
        let mut column_names = self.column_names.clone();
        column_names.extend(other.column_names.iter().cloned());

        Ok(DataFrame {
            columns,
            column_names,
            row_names: self.row_names.clone(),
            nrow: self.nrow,
        })
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Extract a single value from a `Double` column at index `i`. Returns
/// `None` for NA slots (so the result round-trips correctly through
/// `Double::from_optional`).
fn double_get_optional(d: &Double, i: usize) -> Option<f64> {
    if d.is_na(i) {
        None
    } else {
        d.get_value(i)
    }
}

/// Extract a single value from a `Character` column at index `i`.
/// Returns `None` for NA slots.
fn character_get_optional(c: &Character, i: usize) -> Option<String> {
    // `Vector::get` returns `Option<&Option<String>>`; `.cloned()`
    // lifts the inner reference to an owned `Option<String>`; we then
    // `.flatten()` so NA-or-missing collapses to `None`.
    c.get(i).cloned().flatten()
}

/// Pick rows from a column at the given 0-based indices.
fn column_take_indices(column: &Column, indices: &[usize]) -> Column {
    match column {
        Column::Double(d) => {
            let elements: Vec<Option<f64>> =
                indices.iter().map(|&i| double_get_optional(d, i)).collect();
            Column::Double(Double::from_optional(elements))
        }
        Column::Character(c) => {
            let elements: Vec<Option<String>> = indices
                .iter()
                .map(|&i| character_get_optional(c, i))
                .collect();
            Column::Character(Character::from_options(elements))
        }
    }
}

/// Concatenate two columns of identical type.
fn column_concat(a: &Column, b: &Column) -> Result<Column, DataFrameError> {
    match (a, b) {
        (Column::Double(x), Column::Double(y)) => {
            let mut elements: Vec<Option<f64>> = Vec::with_capacity(x.len() + y.len());
            for i in 0..x.len() {
                elements.push(double_get_optional(x, i));
            }
            for i in 0..y.len() {
                elements.push(double_get_optional(y, i));
            }
            Ok(Column::Double(Double::from_optional(elements)))
        }
        (Column::Character(x), Column::Character(y)) => {
            let mut elements: Vec<Option<String>> = Vec::with_capacity(x.len() + y.len());
            for i in 0..x.len() {
                elements.push(character_get_optional(x, i));
            }
            for i in 0..y.len() {
                elements.push(character_get_optional(y, i));
            }
            Ok(Column::Character(Character::from_options(elements)))
        }
        (lhs, rhs) => Err(DataFrameError::SchemaMismatch {
            what: format!(
                "column types differ: {} vs {}",
                lhs.type_name(),
                rhs.type_name()
            ),
        }),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use r_vector::{Character, Double};

    fn doubles(values: &[f64]) -> Double {
        Double::from_values(values.to_vec())
    }

    fn strings(values: &[&str]) -> Character {
        Character::from_strings(values.iter().map(|s| s.to_string()).collect::<Vec<_>>())
    }

    fn sample_df() -> DataFrame {
        DataFrame::from_columns(vec![
            ("age".to_string(), Column::Double(doubles(&[42.0, 17.0, 99.0]))),
            (
                "name".to_string(),
                Column::Character(strings(&["alice", "bob", "carol"])),
            ),
        ])
        .expect("valid schema")
    }

    #[test]
    fn empty_construction() {
        let df = DataFrame::empty();
        assert_eq!(df.nrow(), 0);
        assert_eq!(df.ncol(), 0);
        assert!(df.is_empty());
    }

    #[test]
    fn from_columns_basic_shape() {
        let df = sample_df();
        assert_eq!(df.nrow(), 3);
        assert_eq!(df.ncol(), 2);
        assert_eq!(df.column_names(), &["age", "name"]);
    }

    #[test]
    fn unequal_length_rejected() {
        let err = DataFrame::from_columns(vec![
            ("a".to_string(), Column::Double(doubles(&[1.0, 2.0]))),
            ("b".to_string(), Column::Double(doubles(&[1.0, 2.0, 3.0]))),
        ])
        .unwrap_err();
        assert!(matches!(err, DataFrameError::UnequalColumnLengths { .. }));
    }

    #[test]
    fn duplicate_name_rejected() {
        let err = DataFrame::from_columns(vec![
            ("a".to_string(), Column::Double(doubles(&[1.0]))),
            ("a".to_string(), Column::Double(doubles(&[2.0]))),
        ])
        .unwrap_err();
        assert!(matches!(err, DataFrameError::DuplicateColumn { .. }));
    }

    #[test]
    fn column_lookup_by_name() {
        let df = sample_df();
        let age = df.column("age").unwrap();
        assert_eq!(age.len(), 3);
        assert_eq!(age.type_name(), "double");

        let err = df.column("missing").unwrap_err();
        assert!(matches!(err, DataFrameError::ColumnNotFound { .. }));
    }

    #[test]
    fn column_lookup_by_index() {
        let df = sample_df();
        assert_eq!(df.column_at(0).unwrap().type_name(), "double");
        assert_eq!(df.column_at(1).unwrap().type_name(), "character");
        assert!(matches!(
            df.column_at(2).unwrap_err(),
            DataFrameError::ColumnIndexOutOfRange { .. }
        ));
    }

    #[test]
    fn add_remove_column() {
        let mut df = sample_df();
        df.add_column("active".to_string(), Column::Double(doubles(&[1.0, 0.0, 1.0])))
            .unwrap();
        assert_eq!(df.ncol(), 3);
        assert!(df.has_column("active"));

        let removed = df.remove_column("active").unwrap();
        assert_eq!(removed.len(), 3);
        assert_eq!(df.ncol(), 2);
        assert!(!df.has_column("active"));

        // Adding wrong-length column fails.
        let err = df
            .add_column("wrong".to_string(), Column::Double(doubles(&[1.0])))
            .unwrap_err();
        assert!(matches!(err, DataFrameError::UnequalColumnLengths { .. }));
    }

    #[test]
    fn rename_column() {
        let mut df = sample_df();
        df.rename_column("age", "years".to_string()).unwrap();
        assert!(df.has_column("years"));
        assert!(!df.has_column("age"));

        // Renaming to an existing name fails.
        let err = df
            .rename_column("years", "name".to_string())
            .unwrap_err();
        assert!(matches!(err, DataFrameError::DuplicateColumn { .. }));

        // Renaming a missing column fails.
        let err = df
            .rename_column("nope", "ok".to_string())
            .unwrap_err();
        assert!(matches!(err, DataFrameError::ColumnNotFound { .. }));

        // No-op rename is fine.
        df.rename_column("years", "years".to_string()).unwrap();
    }

    #[test]
    fn select_subset_in_order() {
        let df = sample_df();
        let sub = df.select(&["name", "age"]).unwrap();
        assert_eq!(sub.column_names(), &["name", "age"]);
        assert_eq!(sub.nrow(), 3);
    }

    #[test]
    fn select_rejects_duplicates_and_missing() {
        let df = sample_df();
        assert!(matches!(
            df.select(&["age", "age"]).unwrap_err(),
            DataFrameError::DuplicateColumn { .. }
        ));
        assert!(matches!(
            df.select(&["nope"]).unwrap_err(),
            DataFrameError::ColumnNotFound { .. }
        ));
    }

    #[test]
    fn subset_rows_by_mask() {
        let df = sample_df();
        let mask = vec![Some(true), Some(false), Some(true)];
        let sub = df.subset_rows(&mask).unwrap();
        assert_eq!(sub.nrow(), 2);
        let age = match sub.column("age").unwrap() {
            Column::Double(d) => d,
            _ => panic!("wrong type"),
        };
        assert_eq!(age.get_value(0), Some(42.0));
        assert_eq!(age.get_value(1), Some(99.0));
    }

    #[test]
    fn subset_rows_treats_na_mask_as_false() {
        let df = sample_df();
        let mask = vec![Some(true), None, Some(true)];
        let sub = df.subset_rows(&mask).unwrap();
        assert_eq!(sub.nrow(), 2);
    }

    #[test]
    fn subset_rows_length_mismatch_rejected() {
        let df = sample_df();
        let err = df.subset_rows(&[Some(true), Some(true)]).unwrap_err();
        assert!(matches!(err, DataFrameError::MaskLengthMismatch { .. }));
    }

    #[test]
    fn take_rows_supports_reordering_and_repetition() {
        let df = sample_df();
        let sub = df.take_rows(&[2, 0, 2]).unwrap();
        assert_eq!(sub.nrow(), 3);
        let name = match sub.column("name").unwrap() {
            Column::Character(c) => c,
            _ => panic!("wrong type"),
        };
        // Vector::get on Character returns Option<&Option<String>>; the
        // outer Some is "in range," the inner Some is "not NA."
        assert_eq!(name.get(0), Some(&Some("carol".to_string())));
        assert_eq!(name.get(1), Some(&Some("alice".to_string())));
        assert_eq!(name.get(2), Some(&Some("carol".to_string())));
    }

    #[test]
    fn take_rows_out_of_range_rejected() {
        let df = sample_df();
        let err = df.take_rows(&[0, 5]).unwrap_err();
        assert!(matches!(err, DataFrameError::RowIndexOutOfRange { .. }));
    }

    #[test]
    fn rbind_concatenates_compatible_frames() {
        let a = sample_df();
        let b = DataFrame::from_columns(vec![
            ("age".to_string(), Column::Double(doubles(&[55.0]))),
            ("name".to_string(), Column::Character(strings(&["dave"]))),
        ])
        .unwrap();
        let combined = a.rbind(&b).unwrap();
        assert_eq!(combined.nrow(), 4);
    }

    #[test]
    fn rbind_rejects_schema_mismatch() {
        let a = sample_df();
        let b = DataFrame::from_columns(vec![(
            "wrong".to_string(),
            Column::Double(doubles(&[1.0])),
        )])
        .unwrap();
        let err = a.rbind(&b).unwrap_err();
        assert!(matches!(err, DataFrameError::SchemaMismatch { .. }));
    }

    #[test]
    fn cbind_appends_columns() {
        let a = DataFrame::from_columns(vec![(
            "x".to_string(),
            Column::Double(doubles(&[1.0, 2.0, 3.0])),
        )])
        .unwrap();
        let b = DataFrame::from_columns(vec![(
            "y".to_string(),
            Column::Double(doubles(&[10.0, 20.0, 30.0])),
        )])
        .unwrap();
        let combined = a.cbind(&b).unwrap();
        assert_eq!(combined.ncol(), 2);
        assert_eq!(combined.nrow(), 3);
        assert_eq!(combined.column_names(), &["x", "y"]);
    }

    #[test]
    fn cbind_rejects_nrow_mismatch() {
        let a = DataFrame::from_columns(vec![(
            "x".to_string(),
            Column::Double(doubles(&[1.0, 2.0])),
        )])
        .unwrap();
        let b = DataFrame::from_columns(vec![(
            "y".to_string(),
            Column::Double(doubles(&[1.0])),
        )])
        .unwrap();
        let err = a.cbind(&b).unwrap_err();
        assert!(matches!(err, DataFrameError::SchemaMismatch { .. }));
    }

    #[test]
    fn cbind_rejects_name_collision() {
        let a = DataFrame::from_columns(vec![(
            "x".to_string(),
            Column::Double(doubles(&[1.0])),
        )])
        .unwrap();
        let b = DataFrame::from_columns(vec![(
            "x".to_string(),
            Column::Double(doubles(&[2.0])),
        )])
        .unwrap();
        let err = a.cbind(&b).unwrap_err();
        assert!(matches!(err, DataFrameError::DuplicateColumn { .. }));
    }

    #[test]
    fn row_names_attach_and_round_trip() {
        let df = sample_df()
            .with_row_names(vec!["r1".to_string(), "r2".to_string(), "r3".to_string()])
            .unwrap();
        assert_eq!(df.row_names().unwrap()[1], "r2");

        let sub = df.take_rows(&[0, 2]).unwrap();
        assert_eq!(sub.row_names().unwrap(), &["r1", "r3"]);
    }

    #[test]
    fn row_names_length_mismatch_rejected() {
        let err = sample_df()
            .with_row_names(vec!["only_one".to_string()])
            .unwrap_err();
        assert!(matches!(err, DataFrameError::MaskLengthMismatch { .. }));
    }

    #[test]
    fn column_is_na_for_double_and_character() {
        let d_with_na = Double::from_optional(vec![Some(1.0), None, Some(3.0)]);
        let c_with_na = Character::from_options(vec![
            Some("a".to_string()),
            None,
            Some("c".to_string()),
        ]);
        let df = DataFrame::from_columns(vec![
            ("d".to_string(), Column::Double(d_with_na)),
            ("c".to_string(), Column::Character(c_with_na)),
        ])
        .unwrap();

        let d_col = df.column("d").unwrap();
        assert!(!d_col.is_na(0));
        assert!(d_col.is_na(1));
        assert!(!d_col.is_na(2));

        let c_col = df.column("c").unwrap();
        assert!(!c_col.is_na(0));
        assert!(c_col.is_na(1));
        assert!(!c_col.is_na(2));
    }

    #[test]
    fn column_from_impls_compile() {
        let d: Column = doubles(&[1.0, 2.0]).into();
        assert_eq!(d.len(), 2);
        let c: Column = strings(&["x", "y"]).into();
        assert_eq!(c.len(), 2);
    }
}
