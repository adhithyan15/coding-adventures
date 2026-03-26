// Package sqlcsvsource implements the sql-execution-engine DataSource interface
// backed by a directory of CSV files.
//
// # Architecture
//
// The sql-execution-engine defines a two-method DataSource interface:
//
//	type DataSource interface {
//	    Schema(tableName string) ([]string, error)
//	    Scan(tableName string)   ([]map[string]interface{}, error)
//	}
//
// This package provides CSVDataSource, which satisfies that interface by
// reading files from a directory:
//
//	employees.csv    →  table "employees"
//	departments.csv  →  table "departments"
//
// The adapter is deliberately thin. All CSV parsing is delegated to the
// csv-parser package. All SQL execution is delegated to sql-execution-engine.
// This package's only responsibilities are:
//
//  1. File I/O: find and read the right .csv file for a given table name.
//  2. Column ordering: return Schema columns in the order they appear in the
//     CSV header line (csv-parser returns map[string]string, which has no
//     guaranteed key order in Go).
//  3. Type coercion: convert string CSV values to typed Go values (nil, bool,
//     int64, float64, or string) that the SQL engine can compare correctly.
//
// # Type coercion
//
// CSV has no type system — every field value is a string. The SQL engine needs
// typed values so predicates like `salary > 80000` and `active = true` work:
//
//	CSV text   →  Go value       Notes
//	─────────────────────────────────────────────────────────────────
//	""         →  nil            Empty field = SQL NULL
//	"true"     →  true           Case-sensitive boolean literal
//	"false"    →  false          Case-sensitive boolean literal
//	"42"       →  int64(42)      Integer — no decimal point, no suffix
//	"3.14"     →  float64(3.14)  Float — decimal point, no suffix
//	"123abc"   →  "123abc"       "No suffix" guard prevents truncation
//	"hello"    →  "hello"        Falls through all coercions
//
// The "no suffix" rule: strconv.ParseInt("123abc", 10, 64) succeeds with
// remainder "abc". We reject this by checking that err == nil AND the full
// string was consumed (i.e. len(s) == number of consumed characters — but
// strconv functions return an error if not all bytes are consumed when
// the full-parse variant is used). We use strconv.ParseInt with base 10
// and bitSize 64; if the string contains any non-digit characters (other
// than a leading +/-), ParseInt returns an error. This means "123abc"
// fails and stays a string. See the coerce function for details.
//
// # Usage
//
//	source := sqlcsvsource.New("./data")
//
//	result, err := sqlengine.Execute(
//	    "SELECT name, salary FROM employees WHERE active = true",
//	    source,
//	)
//	if err != nil {
//	    log.Fatal(err)
//	}
//	for _, row := range result.Rows {
//	    fmt.Println(row)
//	}
package sqlcsvsource

import (
	"bufio"
	"bytes"
	"fmt"
	"path/filepath"
	"strconv"
	"strings"

	csvparser "github.com/adhithyan15/coding-adventures/code/packages/go/csv-parser"
	sqlengine "github.com/adhithyan15/coding-adventures/code/packages/go/sql-execution-engine"
)

// CSVDataSource is a DataSource backed by a directory of CSV files.
//
// Each file named <tableName>.csv in Dir is a queryable table. The directory
// is not validated at construction time — files are opened lazily when a query
// references them.
//
// CSVDataSource is safe for concurrent read-only use (multiple goroutines can
// call Schema and Scan simultaneously) because it only performs file I/O and
// pure in-memory transformations.
type CSVDataSource struct {
	// Dir is the path to the directory containing .csv files.
	// It is set at construction time via New and never modified.
	Dir string
}

// New creates a CSVDataSource pointing at dir.
//
// dir must be a path to a directory containing CSV files named <tableName>.csv.
// The directory is not validated at construction — it is accessed lazily.
//
// Example:
//
//	source := New("testdata")
//	result, err := sqlengine.Execute("SELECT * FROM employees", source)
func New(dir string) *CSVDataSource {
	return &CSVDataSource{Dir: dir}
}

// Schema returns the ordered list of column names for the named table.
//
// Column order is determined by reading the first line of the CSV file (the
// header) and splitting it on commas. This preserves declaration order
// faithfully, which is what the SQL engine uses for SELECT *.
//
// Why not derive column order from the map keys returned by csvparser.ParseCSV?
// In Go, map iteration order is intentionally randomized per run. The sql-
// execution-engine calls Schema to expand SELECT * into a concrete column list,
// so stable ordering is essential. Reading the raw header line sidesteps the
// map-ordering problem entirely.
//
// Returns *sqlengine.TableNotFoundError if the file does not exist.
func (s *CSVDataSource) Schema(tableName string) ([]string, error) {
	return StartNew[[]string]("sqlcsvsource.Schema", nil,
		func(op *Operation[[]string], rf *ResultFactory[[]string]) *OperationResult[[]string] {
			path := s.csvPath(tableName)

			// Read the file into memory, then use a bufio.Scanner on the
			// in-memory buffer to extract just the header line.
			data, err := op.File.ReadFile(path)
			if err != nil {
				return rf.Fail(nil, &sqlengine.TableNotFoundError{TableName: tableName})
			}

			scanner := bufio.NewScanner(bytes.NewReader(data))
			if !scanner.Scan() {
				// Empty file or scanner error — no header.
				if scanErr := scanner.Err(); scanErr != nil {
					return rf.Fail(nil, fmt.Errorf("reading schema for %q: %w", tableName, scanErr))
				}
				return rf.Fail(nil, &sqlengine.TableNotFoundError{TableName: tableName})
			}

			headerLine := scanner.Text()
			return rf.Generate(true, false, parseHeaderLine(headerLine))
		}).GetResult()
}

// Scan returns all rows from the named table as typed-value maps.
//
// Each row is a map from column name (string) to a typed Go value. The type
// coercion rules are:
//
//	""       → nil        (SQL NULL)
//	"true"   → true       (bool)
//	"false"  → false      (bool)
//	"42"     → int64(42)  (integer if no non-digit suffix)
//	"3.14"   → float64(3.14)
//	other    → string
//
// The engine does not mutate the returned maps. Each call to Scan reads the
// file fresh — there is no caching.
//
// Returns *sqlengine.TableNotFoundError if the file does not exist.
func (s *CSVDataSource) Scan(tableName string) ([]map[string]interface{}, error) {
	return StartNew[[]map[string]interface{}]("sqlcsvsource.Scan", nil,
		func(op *Operation[[]map[string]interface{}], rf *ResultFactory[[]map[string]interface{}]) *OperationResult[[]map[string]interface{}] {
			path := s.csvPath(tableName)

			content, err := op.File.ReadFile(path)
			if err != nil {
				return rf.Fail(nil, &sqlengine.TableNotFoundError{TableName: tableName})
			}

			// Parse the CSV file. csvparser.ParseCSV returns []map[string]string,
			// where every value is a string. We then apply coerce() to each value.
			strRows, err := csvparser.ParseCSV(string(content))
			if err != nil {
				return rf.Fail(nil, fmt.Errorf("parsing CSV for table %q: %w", tableName, err))
			}

			// Convert []map[string]string → []map[string]interface{} with type coercion.
			result := make([]map[string]interface{}, len(strRows))
			for i, strRow := range strRows {
				typed := make(map[string]interface{}, len(strRow))
				for col, val := range strRow {
					typed[col] = coerce(val)
				}
				result[i] = typed
			}

			return rf.Generate(true, false, result)
		}).GetResult()
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

// csvPath builds the filesystem path to the CSV file for a table.
//
// Convention: table "employees" → "<dir>/employees.csv"
// filepath.Join handles OS-specific path separators correctly.
func (s *CSVDataSource) csvPath(tableName string) string {
	return filepath.Join(s.Dir, tableName+".csv")
}

// parseHeaderLine splits a CSV header line into column names.
//
// We split on "," and trim whitespace from each token. This handles headers
// like "id, name, salary" (space after comma) gracefully.
//
// Limitation: this does not handle quoted column names containing commas
// (e.g. `"last,name"`). In practice CSV column names are never quoted with
// embedded commas, so this is acceptable.
func parseHeaderLine(line string) []string {
	parts := strings.Split(strings.TrimSpace(line), ",")
	cols := make([]string, 0, len(parts))
	for _, p := range parts {
		col := strings.TrimSpace(p)
		if col != "" {
			cols = append(cols, col)
		}
	}
	return cols
}

// coerce converts a raw CSV string value to the appropriate Go type.
//
// The conversion rules are applied in this order:
//
//  1. Empty string → nil (SQL NULL). This must come first because all other
//     conversions would either fail ("" is not a valid integer) or succeed
//     wrongly (strconv.ParseBool("") returns an error, so it's fine, but
//     being explicit is clearer).
//
//  2. "true" / "false" → bool. Checked before numeric parsers to handle
//     these exact literals. Case-sensitive: "True" or "TRUE" stays a string.
//
//  3. Parseable int64 → int64. We use strconv.ParseInt with base 10 and
//     bitSize 64. This function returns an error for any string that contains
//     non-digit characters (except an optional leading +/-). So "123abc"
//     returns an error and falls through to the next case.
//
//  4. Parseable float64 → float64. We use strconv.ParseFloat with bitSize 64.
//     "3.14" and "1e10" parse successfully. "3.14abc" does not.
//     Note: integer strings like "42" also parse as float64, but we check
//     integers first, so "42" → int64(42), not float64(42.0).
//
//  5. Fallthrough → string. Anything that doesn't match the above rules is
//     returned as-is.
//
// The "fully parseable" guarantee comes from strconv's strict parsing:
// strconv.ParseInt returns an *strconv.NumError with Err == ErrSyntax if the
// string contains anything other than digits (and optional sign). There is no
// separate "remainder" check needed — unlike some languages, Go's strconv
// functions don't parse a prefix and ignore the rest.
func coerce(s string) interface{} {
	// Rule 1: Empty string → SQL NULL.
	if s == "" {
		return nil
	}

	// Rule 2: Boolean literals — case-sensitive.
	if s == "true" {
		return true
	}
	if s == "false" {
		return false
	}

	// Rule 3: Integer — strconv.ParseInt is strict; "123abc" returns an error.
	//
	// We use base 10 and bitSize 64. Base 10 means "0x1F" stays a string
	// (strconv.ParseInt("0x1F", 10, 64) returns an error). This matches what
	// most SQL CSV dumps produce: decimal integers only.
	if i, err := strconv.ParseInt(s, 10, 64); err == nil {
		return i
	}

	// Rule 4: Float — strconv.ParseFloat is also strict; "3.14abc" errors.
	//
	// We use bitSize 64 (float64). The returned float64 value is the best
	// approximation of the decimal in the string.
	if f, err := strconv.ParseFloat(s, 64); err == nil {
		return f
	}

	// Rule 5: Everything else stays as a string.
	return s
}
