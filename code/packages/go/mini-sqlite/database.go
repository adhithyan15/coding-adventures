package minisqlite

import (
	"fmt"
	"strings"

	sqlengine "github.com/adhithyan15/coding-adventures/code/packages/go/sql-execution-engine"
)

const rowIDColumn = "__mini_sqlite_rowid"

type statementResult struct {
	columns      []string
	rows         [][]any
	rowsAffected int
}

type tableData struct {
	columns []string
	rows    []map[string]any
}

type snapshot map[string]tableData

type inMemoryDatabase struct {
	tables map[string]tableData
}

func newInMemoryDatabase() *inMemoryDatabase {
	return &inMemoryDatabase{tables: map[string]tableData{}}
}

func (db *inMemoryDatabase) Schema(tableName string) ([]string, error) {
	table, err := db.table(tableName)
	if err != nil {
		return nil, err
	}
	return append([]string{}, table.columns...), nil
}

func (db *inMemoryDatabase) Scan(tableName string) ([]map[string]any, error) {
	table, err := db.table(tableName)
	if err != nil {
		return nil, err
	}
	rows := make([]map[string]any, len(table.rows))
	for i, row := range table.rows {
		rows[i] = copyRow(row)
	}
	return rows, nil
}

func (db *inMemoryDatabase) snapshot() snapshot {
	out := snapshot{}
	for name, table := range db.tables {
		rows := make([]map[string]any, len(table.rows))
		for i, row := range table.rows {
			rows[i] = copyRow(row)
		}
		out[name] = tableData{
			columns: append([]string{}, table.columns...),
			rows:    rows,
		}
	}
	return out
}

func (db *inMemoryDatabase) restore(s snapshot) {
	db.tables = map[string]tableData{}
	for name, table := range s {
		rows := make([]map[string]any, len(table.rows))
		for i, row := range table.rows {
			rows[i] = copyRow(row)
		}
		db.tables[name] = tableData{
			columns: append([]string{}, table.columns...),
			rows:    rows,
		}
	}
}

func (db *inMemoryDatabase) create(stmt *createTableStatement) (*statementResult, error) {
	key := normalizeName(stmt.table)
	if _, ok := db.tables[key]; ok {
		if stmt.ifNotExists {
			return emptyStatementResult(), nil
		}
		return nil, &OperationalError{Message: "table already exists: " + stmt.table}
	}
	seen := map[string]bool{}
	for _, column := range stmt.columns {
		k := normalizeName(column)
		if seen[k] {
			return nil, &ProgrammingError{Message: "duplicate column: " + column}
		}
		seen[k] = true
	}
	db.tables[key] = tableData{columns: append([]string{}, stmt.columns...), rows: []map[string]any{}}
	return emptyStatementResult(), nil
}

func (db *inMemoryDatabase) drop(stmt *dropTableStatement) (*statementResult, error) {
	key := normalizeName(stmt.table)
	if _, ok := db.tables[key]; !ok {
		if stmt.ifExists {
			return emptyStatementResult(), nil
		}
		return nil, &OperationalError{Message: "no such table: " + stmt.table}
	}
	delete(db.tables, key)
	return emptyStatementResult(), nil
}

func (db *inMemoryDatabase) insert(stmt *insertStatement) (*statementResult, error) {
	table, err := db.table(stmt.table)
	if err != nil {
		return nil, err
	}
	columns := stmt.columns
	if len(columns) == 0 {
		columns = table.columns
	}
	if err := assertKnownColumns(table, columns); err != nil {
		return nil, err
	}
	for _, values := range stmt.rows {
		if len(values) != len(columns) {
			return nil, &IntegrityError{Message: fmt.Sprintf("INSERT expected %d values, got %d", len(columns), len(values))}
		}
		row := map[string]any{}
		for _, column := range table.columns {
			row[column] = nil
		}
		for i, column := range columns {
			row[column] = values[i]
		}
		table.rows = append(table.rows, row)
	}
	db.tables[normalizeName(stmt.table)] = table
	return &statementResult{rowsAffected: len(stmt.rows)}, nil
}

func (db *inMemoryDatabase) update(stmt *updateStatement) (*statementResult, error) {
	table, err := db.table(stmt.table)
	if err != nil {
		return nil, err
	}
	var columns []string
	for column := range stmt.assignments {
		columns = append(columns, column)
	}
	if err := assertKnownColumns(table, columns); err != nil {
		return nil, err
	}
	rowIDs, err := db.matchingRowIDs(stmt.table, stmt.where)
	if err != nil {
		return nil, err
	}
	for _, rowID := range rowIDs {
		for column, value := range stmt.assignments {
			table.rows[rowID][column] = value
		}
	}
	db.tables[normalizeName(stmt.table)] = table
	return &statementResult{rowsAffected: len(rowIDs)}, nil
}

func (db *inMemoryDatabase) delete(stmt *deleteStatement) (*statementResult, error) {
	table, err := db.table(stmt.table)
	if err != nil {
		return nil, err
	}
	rowIDs, err := db.matchingRowIDs(stmt.table, stmt.where)
	if err != nil {
		return nil, err
	}
	remove := map[int]bool{}
	for _, rowID := range rowIDs {
		remove[rowID] = true
	}
	var rows []map[string]any
	for i, row := range table.rows {
		if !remove[i] {
			rows = append(rows, row)
		}
	}
	table.rows = rows
	db.tables[normalizeName(stmt.table)] = table
	return &statementResult{rowsAffected: len(rowIDs)}, nil
}

func (db *inMemoryDatabase) selectSQL(sql string) (*statementResult, error) {
	result, err := sqlengine.Execute(sql, db)
	if err != nil {
		return nil, translateError(err)
	}
	return &statementResult{
		columns:      result.Columns,
		rows:         result.Rows,
		rowsAffected: -1,
	}, nil
}

func (db *inMemoryDatabase) matchingRowIDs(tableName, whereSQL string) ([]int, error) {
	table, err := db.table(tableName)
	if err != nil {
		return nil, err
	}
	if strings.TrimSpace(whereSQL) == "" {
		ids := make([]int, len(table.rows))
		for i := range table.rows {
			ids[i] = i
		}
		return ids, nil
	}
	source := &rowIDSource{tableName: tableName, table: table}
	result, err := sqlengine.Execute(fmt.Sprintf("SELECT %s FROM %s WHERE %s", rowIDColumn, tableName, whereSQL), source)
	if err != nil {
		return nil, translateError(err)
	}
	ids := make([]int, 0, len(result.Rows))
	for _, row := range result.Rows {
		if len(row) == 0 {
			continue
		}
		switch v := row[0].(type) {
		case int:
			ids = append(ids, v)
		case int64:
			ids = append(ids, int(v))
		}
	}
	return ids, nil
}

func (db *inMemoryDatabase) table(tableName string) (tableData, error) {
	table, ok := db.tables[normalizeName(tableName)]
	if !ok {
		return tableData{}, &OperationalError{Message: "no such table: " + tableName}
	}
	return table, nil
}

type rowIDSource struct {
	tableName string
	table     tableData
}

func (s *rowIDSource) Schema(tableName string) ([]string, error) {
	if normalizeName(tableName) != normalizeName(s.tableName) {
		return nil, &sqlengine.TableNotFoundError{TableName: tableName}
	}
	columns := append([]string{}, s.table.columns...)
	return append(columns, rowIDColumn), nil
}

func (s *rowIDSource) Scan(tableName string) ([]map[string]any, error) {
	if normalizeName(tableName) != normalizeName(s.tableName) {
		return nil, &sqlengine.TableNotFoundError{TableName: tableName}
	}
	rows := make([]map[string]any, len(s.table.rows))
	for i, row := range s.table.rows {
		copy := copyRow(row)
		copy[rowIDColumn] = int64(i)
		rows[i] = copy
	}
	return rows, nil
}

func assertKnownColumns(table tableData, columns []string) error {
	known := map[string]bool{}
	for _, column := range table.columns {
		known[normalizeName(column)] = true
	}
	for _, column := range columns {
		if !known[normalizeName(column)] {
			return &OperationalError{Message: "no such column: " + column}
		}
	}
	return nil
}

func copyRow(row map[string]any) map[string]any {
	out := map[string]any{}
	for k, v := range row {
		out[k] = v
	}
	return out
}

func normalizeName(name string) string {
	return strings.ToLower(name)
}

func emptyStatementResult() *statementResult {
	return &statementResult{rowsAffected: 0}
}
