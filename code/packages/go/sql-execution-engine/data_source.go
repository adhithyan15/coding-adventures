// DataSource is the storage backend abstraction for the SQL execution engine.
//
// Real databases separate the query planner/executor from the storage engine.
// PostgreSQL has pluggable table access methods; SQLite has a B-tree backend.
// We model this separation with a simple two-method interface:
//
//   - Schema: returns the ordered list of column names for a table.
//     The engine needs this to expand SELECT * into concrete column names.
//   - Scan: returns all rows from the table as maps from column name to value.
//     The engine is responsible for all filtering, projection, and ordering.
//
// This is a full-table-scan model — there are no indexes or push-down filters.
// That makes the implementation straightforward while still illustrating the
// fundamental executor → storage boundary.
//
// To use the engine with your own data, implement this interface:
//
//	type MySource struct{ db *sql.DB }
//
//	func (s MySource) Schema(table string) ([]string, error) {
//	    rows, _ := s.db.Query("PRAGMA table_info(" + table + ")")
//	    // ... extract column names
//	}
//
//	func (s MySource) Scan(table string) ([]map[string]interface{}, error) {
//	    rows, _ := s.db.Query("SELECT * FROM " + table)
//	    // ... scan into maps
//	}
package sqlengine

// DataSource is the only interface between the query engine and the storage
// backend. Implementing this two-method interface is all that is required to
// connect the engine to any data store.
type DataSource interface {
	// Schema returns the ordered list of column names for the named table.
	// The order matters: it determines the column order for SELECT *.
	//
	// Returns TableNotFoundError if the table does not exist.
	Schema(tableName string) ([]string, error)

	// Scan returns all rows from the named table. Each row is a map from
	// column name (as returned by Schema) to its value. Values must be one of:
	//   nil        → SQL NULL
	//   int64      → integer
	//   float64    → floating-point number
	//   string     → text
	//   bool       → boolean
	//
	// The engine does not mutate the returned maps. Callers may reuse them.
	//
	// Returns TableNotFoundError if the table does not exist.
	Scan(tableName string) ([]map[string]interface{}, error)
}
