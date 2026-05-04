// Package sqlbackend defines the pluggable storage boundary for the SQL stack.
package sqlbackend

import (
	"bytes"
	"fmt"
	"math"
	"reflect"
	"sort"
	"strings"
)

// Row is one table row, keyed by column name.
type Row map[string]any

// TransactionHandle identifies an active transaction.
type TransactionHandle int

// IsSQLValue reports whether value is in the portable SQL value set.
func IsSQLValue(value any) bool {
	if value == nil {
		return true
	}
	switch value.(type) {
	case bool, string, []byte:
		return true
	case int, int8, int16, int32, int64:
		return true
	case uint, uint8, uint16, uint32, uint64:
		return true
	case float32:
		return !math.IsNaN(float64(value.(float32))) && !math.IsInf(float64(value.(float32)), 0)
	case float64:
		return !math.IsNaN(value.(float64)) && !math.IsInf(value.(float64), 0)
	default:
		return false
	}
}

// SQLTypeName returns the SQL storage class name for value.
func SQLTypeName(value any) (string, error) {
	if value == nil {
		return "NULL", nil
	}
	switch value.(type) {
	case bool:
		return "BOOLEAN", nil
	case int, int8, int16, int32, int64, uint, uint8, uint16, uint32, uint64:
		return "INTEGER", nil
	case float32, float64:
		if !IsSQLValue(value) {
			return "", fmt.Errorf("not a SQL value: %T", value)
		}
		return "REAL", nil
	case string:
		return "TEXT", nil
	case []byte:
		return "BLOB", nil
	default:
		return "", fmt.Errorf("not a SQL value: %T", value)
	}
}

// CompareSQLValues compares two SQL values using the package's stable ordering.
func CompareSQLValues(left any, right any) (int, error) {
	if !IsSQLValue(left) {
		return 0, fmt.Errorf("not a SQL value: %T", left)
	}
	if !IsSQLValue(right) {
		return 0, fmt.Errorf("not a SQL value: %T", right)
	}
	leftRank := valueRank(left)
	rightRank := valueRank(right)
	if leftRank != rightRank {
		return sign(leftRank - rightRank), nil
	}
	if left == nil {
		return 0, nil
	}
	switch l := left.(type) {
	case bool:
		r := right.(bool)
		return sign(boolInt(l) - boolInt(r)), nil
	case string:
		r := right.(string)
		return strings.Compare(l, r), nil
	case []byte:
		return bytes.Compare(l, right.([]byte)), nil
	default:
		lf, lok := numericAsFloat(left)
		rf, rok := numericAsFloat(right)
		if lok && rok {
			return signFloat(lf - rf), nil
		}
	}
	return 0, nil
}

func valueRank(value any) int {
	if value == nil {
		return 0
	}
	switch value.(type) {
	case bool:
		return 1
	case int, int8, int16, int32, int64, uint, uint8, uint16, uint32, uint64, float32, float64:
		return 2
	case string:
		return 3
	case []byte:
		return 4
	default:
		return 5
	}
}

// RowIterator streams rows one at a time.
type RowIterator interface {
	Next() (Row, bool)
	Close()
}

// Cursor is a row iterator that can identify its current row for DML.
type Cursor interface {
	RowIterator
	CurrentRow() (Row, bool)
}

// ListRowIterator is a copy-on-read iterator over an in-memory row slice.
type ListRowIterator struct {
	rows   []Row
	index  int
	closed bool
}

func NewListRowIterator(rows []Row) *ListRowIterator {
	return &ListRowIterator{rows: copyRows(rows)}
}

func (it *ListRowIterator) Next() (Row, bool) {
	if it.closed || it.index >= len(it.rows) {
		return nil, false
	}
	row := copyRow(it.rows[it.index])
	it.index++
	return row, true
}

func (it *ListRowIterator) Close() {
	it.closed = true
}

// ListCursor is the in-memory positioned cursor implementation.
type ListCursor struct {
	table   *tableState
	index   int
	current Row
	closed  bool
}

func NewListCursor(rows []Row) *ListCursor {
	return &ListCursor{table: &tableState{rows: rows}, index: -1}
}

func (c *ListCursor) Next() (Row, bool) {
	if c.closed {
		return nil, false
	}
	c.index++
	if c.index >= len(c.table.rows) {
		c.current = nil
		return nil, false
	}
	c.current = c.table.rows[c.index]
	return copyRow(c.current), true
}

func (c *ListCursor) Close() {
	c.closed = true
}

func (c *ListCursor) CurrentRow() (Row, bool) {
	if c.current == nil {
		return nil, false
	}
	return copyRow(c.current), true
}

func (c *ListCursor) currentIndex() int {
	return c.index
}

func (c *ListCursor) isBackedBy(table *tableState) bool {
	return c.table == table
}

func (c *ListCursor) adjustAfterDelete() {
	c.index--
	c.current = nil
}

// ColumnDef describes one table column.
type ColumnDef struct {
	Name            string
	TypeName        string
	NotNull         bool
	PrimaryKey      bool
	Unique          bool
	Autoincrement   bool
	DefaultValue    any
	HasDefault      bool
	CheckExpression any
	ForeignKey      any
}

func (c ColumnDef) EffectiveNotNull() bool {
	return c.NotNull || c.PrimaryKey
}

func (c ColumnDef) EffectiveUnique() bool {
	return c.Unique || c.PrimaryKey
}

func (c ColumnDef) clone() ColumnDef {
	c.DefaultValue = copyValue(c.DefaultValue)
	return c
}

// IndexDef describes one backend index.
type IndexDef struct {
	Name    string
	Table   string
	Columns []string
	Unique  bool
	Auto    bool
}

func (d IndexDef) clone() IndexDef {
	d.Columns = append([]string{}, d.Columns...)
	return d
}

// TriggerDef stores a trigger definition.
type TriggerDef struct {
	Name   string
	Table  string
	Timing string
	Event  string
	Body   string
}

// BackendError marks expected backend failures.
type BackendError interface {
	error
	backendError()
}

type TableNotFound struct{ Table string }

func (e *TableNotFound) Error() string { return fmt.Sprintf("table not found: %q", e.Table) }
func (e *TableNotFound) backendError() {}

type TableAlreadyExists struct{ Table string }

func (e *TableAlreadyExists) Error() string { return fmt.Sprintf("table already exists: %q", e.Table) }
func (e *TableAlreadyExists) backendError() {}

type ColumnNotFound struct {
	Table  string
	Column string
}

func (e *ColumnNotFound) Error() string {
	return fmt.Sprintf("column not found: %q.%q", e.Table, e.Column)
}
func (e *ColumnNotFound) backendError() {}

type ColumnAlreadyExists struct {
	Table  string
	Column string
}

func (e *ColumnAlreadyExists) Error() string {
	return fmt.Sprintf("column already exists: %q.%q", e.Table, e.Column)
}
func (e *ColumnAlreadyExists) backendError() {}

type ConstraintViolation struct {
	Table   string
	Column  string
	Message string
}

func (e *ConstraintViolation) Error() string { return e.Message }
func (e *ConstraintViolation) backendError() {}

type Unsupported struct{ Operation string }

func (e *Unsupported) Error() string { return "operation not supported: " + e.Operation }
func (e *Unsupported) backendError() {}

type Internal struct{ Message string }

func (e *Internal) Error() string { return e.Message }
func (e *Internal) backendError() {}

type IndexAlreadyExists struct{ Index string }

func (e *IndexAlreadyExists) Error() string { return fmt.Sprintf("index already exists: %q", e.Index) }
func (e *IndexAlreadyExists) backendError() {}

type IndexNotFound struct{ Index string }

func (e *IndexNotFound) Error() string { return fmt.Sprintf("index not found: %q", e.Index) }
func (e *IndexNotFound) backendError() {}

type TriggerAlreadyExists struct{ Trigger string }

func (e *TriggerAlreadyExists) Error() string {
	return fmt.Sprintf("trigger already exists: %q", e.Trigger)
}
func (e *TriggerAlreadyExists) backendError() {}

type TriggerNotFound struct{ Trigger string }

func (e *TriggerNotFound) Error() string { return fmt.Sprintf("trigger not found: %q", e.Trigger) }
func (e *TriggerNotFound) backendError() {}

// Backend is the storage interface used by the SQL VM and related packages.
type Backend interface {
	Tables() []string
	Columns(table string) ([]ColumnDef, error)
	Scan(table string) (RowIterator, error)
	Insert(table string, row Row) error
	Update(table string, cursor Cursor, assignments Row) error
	Delete(table string, cursor Cursor) error
	CreateTable(table string, columns []ColumnDef, ifNotExists bool) error
	DropTable(table string, ifExists bool) error
	AddColumn(table string, column ColumnDef) error
	CreateIndex(index IndexDef) error
	DropIndex(name string, ifExists bool) error
	ListIndexes(table string) []IndexDef
	ScanIndex(indexName string, lo []any, hi []any, loInclusive bool, hiInclusive bool) ([]int, error)
	ScanByRowids(table string, rowids []int) (RowIterator, error)
	BeginTransaction() (TransactionHandle, error)
	Commit(handle TransactionHandle) error
	Rollback(handle TransactionHandle) error
}

type SchemaProvider interface {
	ColumnNames(table string) ([]string, error)
}

func BackendAsSchemaProvider(backend Backend) SchemaProvider {
	return backendSchemaProvider{backend: backend}
}

type backendSchemaProvider struct{ backend Backend }

func (p backendSchemaProvider) ColumnNames(table string) ([]string, error) {
	columns, err := p.backend.Columns(table)
	if err != nil {
		return nil, err
	}
	names := make([]string, len(columns))
	for i, column := range columns {
		names[i] = column.Name
	}
	return names, nil
}

type tableState struct {
	name    string
	columns []ColumnDef
	rows    []Row
}

func (t *tableState) copy() *tableState {
	return &tableState{
		name:    t.name,
		columns: copyColumns(t.columns),
		rows:    copyRows(t.rows),
	}
}

// InMemoryBackend is the reference Backend implementation.
type InMemoryBackend struct {
	tables          map[string]*tableState
	tableOrder      []string
	indexes         map[string]IndexDef
	indexOrder      []string
	triggers        map[string]TriggerDef
	triggersByTable map[string][]TriggerDef
	snapshot        *snapshotState
	savepoints      []savepoint
	activeHandle    *TransactionHandle
	nextHandle      TransactionHandle
	userVersion     uint32
	schemaVersion   uint32
}

func NewInMemoryBackend() *InMemoryBackend {
	return &InMemoryBackend{
		tables:          map[string]*tableState{},
		indexes:         map[string]IndexDef{},
		triggers:        map[string]TriggerDef{},
		triggersByTable: map[string][]TriggerDef{},
		nextHandle:      1,
	}
}

func NewInMemoryBackendFromTables(tables map[string]struct {
	Columns []ColumnDef
	Rows    []Row
}) *InMemoryBackend {
	backend := NewInMemoryBackend()
	for name, table := range tables {
		key := normalizeName(name)
		backend.tables[key] = &tableState{name: name, columns: copyColumns(table.Columns), rows: copyRows(table.Rows)}
		backend.tableOrder = append(backend.tableOrder, key)
	}
	sort.SliceStable(backend.tableOrder, func(i, j int) bool {
		return backend.tables[backend.tableOrder[i]].name < backend.tables[backend.tableOrder[j]].name
	})
	return backend
}

func (b *InMemoryBackend) Tables() []string {
	names := make([]string, 0, len(b.tableOrder))
	for _, key := range b.tableOrder {
		if table := b.tables[key]; table != nil {
			names = append(names, table.name)
		}
	}
	return names
}

func (b *InMemoryBackend) Columns(table string) ([]ColumnDef, error) {
	state, err := b.requireTable(table)
	if err != nil {
		return nil, err
	}
	return copyColumns(state.columns), nil
}

func (b *InMemoryBackend) Scan(table string) (RowIterator, error) {
	state, err := b.requireTable(table)
	if err != nil {
		return nil, err
	}
	return NewListRowIterator(state.rows), nil
}

func (b *InMemoryBackend) OpenCursor(table string) (*ListCursor, error) {
	state, err := b.requireTable(table)
	if err != nil {
		return nil, err
	}
	return &ListCursor{table: state, index: -1}, nil
}

func (b *InMemoryBackend) Insert(table string, row Row) error {
	state, err := b.requireTable(table)
	if err != nil {
		return err
	}
	normalized, err := b.normalizeRow(table, state, row)
	if err != nil {
		return err
	}
	if err := checkNotNull(table, state, normalized); err != nil {
		return err
	}
	if err := checkUnique(table, state, normalized, -1); err != nil {
		return err
	}
	state.rows = append(state.rows, normalized)
	return nil
}

func (b *InMemoryBackend) Update(table string, cursor Cursor, assignments Row) error {
	state, err := b.requireTable(table)
	if err != nil {
		return err
	}
	listCursor, err := requireListCursor(table, state, cursor)
	if err != nil {
		return err
	}
	idx := listCursor.currentIndex()
	if idx < 0 || idx >= len(state.rows) {
		return &Unsupported{Operation: "update without current row"}
	}
	updated := copyRow(state.rows[idx])
	for name, value := range assignments {
		if !IsSQLValue(value) {
			return fmt.Errorf("not a SQL value: %T", value)
		}
		canonical, err := canonicalColumn(table, state, name)
		if err != nil {
			return err
		}
		updated[canonical] = copyValue(value)
	}
	if err := checkNotNull(table, state, updated); err != nil {
		return err
	}
	if err := checkUnique(table, state, updated, idx); err != nil {
		return err
	}
	state.rows[idx] = updated
	return nil
}

func (b *InMemoryBackend) Delete(table string, cursor Cursor) error {
	state, err := b.requireTable(table)
	if err != nil {
		return err
	}
	listCursor, err := requireListCursor(table, state, cursor)
	if err != nil {
		return err
	}
	idx := listCursor.currentIndex()
	if idx < 0 || idx >= len(state.rows) {
		return &Unsupported{Operation: "delete without current row"}
	}
	state.rows = append(state.rows[:idx], state.rows[idx+1:]...)
	listCursor.adjustAfterDelete()
	return nil
}

func (b *InMemoryBackend) CreateTable(table string, columns []ColumnDef, ifNotExists bool) error {
	key := normalizeName(table)
	if _, ok := b.tables[key]; ok {
		if ifNotExists {
			return nil
		}
		return &TableAlreadyExists{Table: table}
	}
	seen := map[string]struct{}{}
	for _, column := range columns {
		columnKey := normalizeName(column.Name)
		if _, ok := seen[columnKey]; ok {
			return &ColumnAlreadyExists{Table: table, Column: column.Name}
		}
		seen[columnKey] = struct{}{}
	}
	b.tables[key] = &tableState{name: table, columns: copyColumns(columns)}
	b.tableOrder = append(b.tableOrder, key)
	b.bumpSchemaVersion()
	return nil
}

func (b *InMemoryBackend) DropTable(table string, ifExists bool) error {
	key := normalizeName(table)
	if _, ok := b.tables[key]; !ok {
		if ifExists {
			return nil
		}
		return &TableNotFound{Table: table}
	}
	delete(b.tables, key)
	b.tableOrder = removeString(b.tableOrder, key)
	b.deleteIndexesForTable(table)
	b.deleteTriggersForTable(table)
	b.bumpSchemaVersion()
	return nil
}

func (b *InMemoryBackend) AddColumn(table string, column ColumnDef) error {
	state, err := b.requireTable(table)
	if err != nil {
		return err
	}
	if _, err := canonicalColumn(table, state, column.Name); err == nil {
		return &ColumnAlreadyExists{Table: table, Column: column.Name}
	}
	if len(state.rows) > 0 && column.EffectiveNotNull() && !column.HasDefault {
		return &ConstraintViolation{
			Table:   table,
			Column:  column.Name,
			Message: fmt.Sprintf("NOT NULL constraint failed: %s.%s", table, column.Name),
		}
	}
	cloned := column.clone()
	state.columns = append(state.columns, cloned)
	for _, row := range state.rows {
		if cloned.HasDefault {
			row[cloned.Name] = copyValue(cloned.DefaultValue)
		} else {
			row[cloned.Name] = nil
		}
	}
	b.bumpSchemaVersion()
	return nil
}

func (b *InMemoryBackend) CreateIndex(index IndexDef) error {
	key := normalizeName(index.Name)
	if _, ok := b.indexes[key]; ok {
		return &IndexAlreadyExists{Index: index.Name}
	}
	state, err := b.requireTable(index.Table)
	if err != nil {
		return err
	}
	for _, column := range index.Columns {
		if _, err := canonicalColumn(index.Table, state, column); err != nil {
			return err
		}
	}
	b.indexes[key] = index.clone()
	b.indexOrder = append(b.indexOrder, key)
	b.bumpSchemaVersion()
	return nil
}

func (b *InMemoryBackend) DropIndex(name string, ifExists bool) error {
	key := normalizeName(name)
	if _, ok := b.indexes[key]; !ok {
		if ifExists {
			return nil
		}
		return &IndexNotFound{Index: name}
	}
	delete(b.indexes, key)
	b.indexOrder = removeString(b.indexOrder, key)
	b.bumpSchemaVersion()
	return nil
}

func (b *InMemoryBackend) ListIndexes(table string) []IndexDef {
	indexes := []IndexDef{}
	for _, key := range b.indexOrder {
		index, ok := b.indexes[key]
		if !ok {
			continue
		}
		if table == "" || sameName(index.Table, table) {
			indexes = append(indexes, index.clone())
		}
	}
	return indexes
}

func (b *InMemoryBackend) ScanIndex(indexName string, lo []any, hi []any, loInclusive bool, hiInclusive bool) ([]int, error) {
	index, ok := b.indexes[normalizeName(indexName)]
	if !ok {
		return nil, &IndexNotFound{Index: indexName}
	}
	state, err := b.requireTable(index.Table)
	if err != nil {
		return nil, err
	}
	keyed := make([]keyedRow, 0, len(state.rows))
	for rowid, row := range state.rows {
		key := make([]any, len(index.Columns))
		for i, column := range index.Columns {
			canonical, err := canonicalColumn(index.Table, state, column)
			if err != nil {
				return nil, err
			}
			key[i] = row[canonical]
		}
		keyed = append(keyed, keyedRow{key: key, rowid: rowid})
	}
	sort.SliceStable(keyed, func(i, j int) bool {
		cmp, _ := compareKey(keyed[i].key, keyed[j].key)
		if cmp == 0 {
			return keyed[i].rowid < keyed[j].rowid
		}
		return cmp < 0
	})
	rowids := []int{}
	for _, row := range keyed {
		if lo != nil {
			cmp, err := comparePrefix(row.key, lo)
			if err != nil {
				return nil, err
			}
			if cmp < 0 || (cmp == 0 && !loInclusive) {
				continue
			}
		}
		if hi != nil {
			cmp, err := comparePrefix(row.key, hi)
			if err != nil {
				return nil, err
			}
			if cmp > 0 || (cmp == 0 && !hiInclusive) {
				break
			}
		}
		rowids = append(rowids, row.rowid)
	}
	return rowids, nil
}

func (b *InMemoryBackend) ScanByRowids(table string, rowids []int) (RowIterator, error) {
	state, err := b.requireTable(table)
	if err != nil {
		return nil, err
	}
	rows := []Row{}
	for _, rowid := range rowids {
		if rowid >= 0 && rowid < len(state.rows) {
			rows = append(rows, state.rows[rowid])
		}
	}
	return NewListRowIterator(rows), nil
}

func (b *InMemoryBackend) BeginTransaction() (TransactionHandle, error) {
	if b.activeHandle != nil {
		return 0, &Unsupported{Operation: "nested transactions"}
	}
	handle := b.nextHandle
	b.nextHandle++
	b.snapshot = b.captureSnapshot()
	b.activeHandle = &handle
	return handle, nil
}

func (b *InMemoryBackend) Commit(handle TransactionHandle) error {
	if err := b.requireActive(handle); err != nil {
		return err
	}
	b.snapshot = nil
	b.activeHandle = nil
	b.savepoints = nil
	return nil
}

func (b *InMemoryBackend) Rollback(handle TransactionHandle) error {
	if err := b.requireActive(handle); err != nil {
		return err
	}
	if b.snapshot != nil {
		b.restoreSnapshot(b.snapshot)
	}
	b.snapshot = nil
	b.activeHandle = nil
	b.savepoints = nil
	return nil
}

func (b *InMemoryBackend) CurrentTransaction() (TransactionHandle, bool) {
	if b.activeHandle == nil {
		return 0, false
	}
	return *b.activeHandle, true
}

func (b *InMemoryBackend) CreateSavepoint(name string) error {
	if b.activeHandle == nil {
		if _, err := b.BeginTransaction(); err != nil {
			return err
		}
	}
	b.savepoints = append(b.savepoints, savepoint{name: name, snapshot: b.captureSnapshot()})
	return nil
}

func (b *InMemoryBackend) ReleaseSavepoint(name string) error {
	idx := b.findSavepoint(name)
	if idx < 0 {
		return &Unsupported{Operation: fmt.Sprintf("RELEASE %q: no such savepoint", name)}
	}
	b.savepoints = b.savepoints[:idx]
	return nil
}

func (b *InMemoryBackend) RollbackToSavepoint(name string) error {
	idx := b.findSavepoint(name)
	if idx < 0 {
		return &Unsupported{Operation: fmt.Sprintf("ROLLBACK TO %q: no such savepoint", name)}
	}
	b.restoreSnapshot(b.savepoints[idx].snapshot)
	b.savepoints = b.savepoints[:idx+1]
	return nil
}

func (b *InMemoryBackend) CreateTrigger(trigger TriggerDef) error {
	key := normalizeName(trigger.Name)
	if _, ok := b.triggers[key]; ok {
		return &TriggerAlreadyExists{Trigger: trigger.Name}
	}
	b.triggers[key] = trigger
	tableKey := normalizeName(trigger.Table)
	b.triggersByTable[tableKey] = append(b.triggersByTable[tableKey], trigger)
	return nil
}

func (b *InMemoryBackend) DropTrigger(name string, ifExists bool) error {
	key := normalizeName(name)
	trigger, ok := b.triggers[key]
	if !ok {
		if ifExists {
			return nil
		}
		return &TriggerNotFound{Trigger: name}
	}
	delete(b.triggers, key)
	tableKey := normalizeName(trigger.Table)
	triggers := b.triggersByTable[tableKey]
	kept := triggers[:0]
	for _, candidate := range triggers {
		if !sameName(candidate.Name, name) {
			kept = append(kept, candidate)
		}
	}
	if len(kept) == 0 {
		delete(b.triggersByTable, tableKey)
	} else {
		b.triggersByTable[tableKey] = kept
	}
	return nil
}

func (b *InMemoryBackend) ListTriggers(table string) []TriggerDef {
	triggers := b.triggersByTable[normalizeName(table)]
	return append([]TriggerDef{}, triggers...)
}

func (b *InMemoryBackend) GetUserVersion() uint32 {
	return b.userVersion
}

func (b *InMemoryBackend) SetUserVersion(value int64) error {
	if value < 0 || value > 0xffffffff {
		return fmt.Errorf("user_version must fit in u32, got %d", value)
	}
	b.userVersion = uint32(value)
	return nil
}

func (b *InMemoryBackend) GetSchemaVersion() uint32 {
	return b.schemaVersion
}

func (b *InMemoryBackend) requireTable(table string) (*tableState, error) {
	state := b.tables[normalizeName(table)]
	if state == nil {
		return nil, &TableNotFound{Table: table}
	}
	return state, nil
}

func (b *InMemoryBackend) normalizeRow(table string, state *tableState, row Row) (Row, error) {
	normalized := Row{}
	for name, value := range row {
		if !IsSQLValue(value) {
			return nil, fmt.Errorf("not a SQL value: %T", value)
		}
		canonical, err := canonicalColumn(table, state, name)
		if err != nil {
			return nil, err
		}
		normalized[canonical] = copyValue(value)
	}
	for _, column := range state.columns {
		if _, ok := normalized[column.Name]; !ok {
			if column.HasDefault {
				normalized[column.Name] = copyValue(column.DefaultValue)
			} else {
				normalized[column.Name] = nil
			}
		}
	}
	return normalized, nil
}

func (b *InMemoryBackend) requireActive(handle TransactionHandle) error {
	if b.activeHandle == nil {
		return &Unsupported{Operation: "no active transaction"}
	}
	if *b.activeHandle != handle {
		return &Unsupported{Operation: "stale transaction handle"}
	}
	return nil
}

func (b *InMemoryBackend) captureSnapshot() *snapshotState {
	return &snapshotState{
		tables:          copyTableMap(b.tables),
		tableOrder:      append([]string{}, b.tableOrder...),
		indexes:         copyIndexMap(b.indexes),
		indexOrder:      append([]string{}, b.indexOrder...),
		triggers:        copyTriggerMap(b.triggers),
		triggersByTable: copyTriggersByTable(b.triggersByTable),
		userVersion:     b.userVersion,
		schemaVersion:   b.schemaVersion,
	}
}

func (b *InMemoryBackend) restoreSnapshot(snapshot *snapshotState) {
	b.tables = copyTableMap(snapshot.tables)
	b.tableOrder = append([]string{}, snapshot.tableOrder...)
	b.indexes = copyIndexMap(snapshot.indexes)
	b.indexOrder = append([]string{}, snapshot.indexOrder...)
	b.triggers = copyTriggerMap(snapshot.triggers)
	b.triggersByTable = copyTriggersByTable(snapshot.triggersByTable)
	b.userVersion = snapshot.userVersion
	b.schemaVersion = snapshot.schemaVersion
}

func (b *InMemoryBackend) findSavepoint(name string) int {
	for i := len(b.savepoints) - 1; i >= 0; i-- {
		if b.savepoints[i].name == name {
			return i
		}
	}
	return -1
}

func (b *InMemoryBackend) deleteIndexesForTable(table string) {
	for key, index := range b.indexes {
		if sameName(index.Table, table) {
			delete(b.indexes, key)
			b.indexOrder = removeString(b.indexOrder, key)
		}
	}
}

func (b *InMemoryBackend) deleteTriggersForTable(table string) {
	tableKey := normalizeName(table)
	for key, trigger := range b.triggers {
		if sameName(trigger.Table, table) {
			delete(b.triggers, key)
		}
	}
	delete(b.triggersByTable, tableKey)
}

func (b *InMemoryBackend) bumpSchemaVersion() {
	b.schemaVersion++
}

type snapshotState struct {
	tables          map[string]*tableState
	tableOrder      []string
	indexes         map[string]IndexDef
	indexOrder      []string
	triggers        map[string]TriggerDef
	triggersByTable map[string][]TriggerDef
	userVersion     uint32
	schemaVersion   uint32
}

type savepoint struct {
	name     string
	snapshot *snapshotState
}

type keyedRow struct {
	key   []any
	rowid int
}

func requireListCursor(table string, state *tableState, cursor Cursor) (*ListCursor, error) {
	listCursor, ok := cursor.(*ListCursor)
	if !ok || !listCursor.isBackedBy(state) {
		return nil, &Unsupported{Operation: fmt.Sprintf("foreign cursor for table %s", table)}
	}
	return listCursor, nil
}

func canonicalColumn(table string, state *tableState, column string) (string, error) {
	for _, candidate := range state.columns {
		if sameName(candidate.Name, column) {
			return candidate.Name, nil
		}
	}
	return "", &ColumnNotFound{Table: table, Column: column}
}

func checkNotNull(table string, state *tableState, row Row) error {
	for _, column := range state.columns {
		if column.EffectiveNotNull() && row[column.Name] == nil {
			return &ConstraintViolation{
				Table:   table,
				Column:  column.Name,
				Message: fmt.Sprintf("NOT NULL constraint failed: %s.%s", table, column.Name),
			}
		}
	}
	return nil
}

func checkUnique(table string, state *tableState, row Row, ignoreIndex int) error {
	for _, column := range state.columns {
		if !column.EffectiveUnique() {
			continue
		}
		value := row[column.Name]
		if value == nil {
			continue
		}
		for i, existing := range state.rows {
			if i == ignoreIndex {
				continue
			}
			equal, err := sqlValuesEqual(existing[column.Name], value)
			if err != nil {
				return err
			}
			if equal {
				label := "UNIQUE"
				if column.PrimaryKey {
					label = "PRIMARY KEY"
				}
				return &ConstraintViolation{
					Table:   table,
					Column:  column.Name,
					Message: fmt.Sprintf("%s constraint failed: %s.%s", label, table, column.Name),
				}
			}
		}
	}
	return nil
}

func sqlValuesEqual(left any, right any) (bool, error) {
	if !IsSQLValue(left) || !IsSQLValue(right) {
		return false, fmt.Errorf("not a SQL value")
	}
	if lb, ok := left.([]byte); ok {
		rb, ok := right.([]byte)
		return ok && bytes.Equal(lb, rb), nil
	}
	cmp, err := CompareSQLValues(left, right)
	return cmp == 0, err
}

func compareKey(left []any, right []any) (int, error) {
	limit := min(len(left), len(right))
	for i := 0; i < limit; i++ {
		cmp, err := CompareSQLValues(left[i], right[i])
		if err != nil {
			return 0, err
		}
		if cmp != 0 {
			return cmp, nil
		}
	}
	return sign(len(left) - len(right)), nil
}

func comparePrefix(key []any, bound []any) (int, error) {
	for i := range bound {
		var value any
		if i < len(key) {
			value = key[i]
		}
		cmp, err := CompareSQLValues(value, bound[i])
		if err != nil {
			return 0, err
		}
		if cmp != 0 {
			return cmp, nil
		}
	}
	return 0, nil
}

func copyRow(row Row) Row {
	out := Row{}
	for key, value := range row {
		out[key] = copyValue(value)
	}
	return out
}

func copyRows(rows []Row) []Row {
	out := make([]Row, len(rows))
	for i, row := range rows {
		out[i] = copyRow(row)
	}
	return out
}

func copyColumns(columns []ColumnDef) []ColumnDef {
	out := make([]ColumnDef, len(columns))
	for i, column := range columns {
		out[i] = column.clone()
	}
	return out
}

func copyValue(value any) any {
	if blob, ok := value.([]byte); ok {
		return append([]byte{}, blob...)
	}
	return value
}

func copyTableMap(source map[string]*tableState) map[string]*tableState {
	out := map[string]*tableState{}
	for key, table := range source {
		out[key] = table.copy()
	}
	return out
}

func copyIndexMap(source map[string]IndexDef) map[string]IndexDef {
	out := map[string]IndexDef{}
	for key, index := range source {
		out[key] = index.clone()
	}
	return out
}

func copyTriggerMap(source map[string]TriggerDef) map[string]TriggerDef {
	out := map[string]TriggerDef{}
	for key, trigger := range source {
		out[key] = trigger
	}
	return out
}

func copyTriggersByTable(source map[string][]TriggerDef) map[string][]TriggerDef {
	out := map[string][]TriggerDef{}
	for key, triggers := range source {
		out[key] = append([]TriggerDef{}, triggers...)
	}
	return out
}

func normalizeName(name string) string {
	return strings.ToLower(name)
}

func sameName(left string, right string) bool {
	return normalizeName(left) == normalizeName(right)
}

func removeString(values []string, value string) []string {
	out := values[:0]
	for _, candidate := range values {
		if candidate != value {
			out = append(out, candidate)
		}
	}
	return out
}

func boolInt(value bool) int {
	if value {
		return 1
	}
	return 0
}

func sign(value int) int {
	switch {
	case value < 0:
		return -1
	case value > 0:
		return 1
	default:
		return 0
	}
}

func signFloat(value float64) int {
	switch {
	case value < 0:
		return -1
	case value > 0:
		return 1
	default:
		return 0
	}
}

func numericAsFloat(value any) (float64, bool) {
	v := reflect.ValueOf(value)
	switch v.Kind() {
	case reflect.Int, reflect.Int8, reflect.Int16, reflect.Int32, reflect.Int64:
		return float64(v.Int()), true
	case reflect.Uint, reflect.Uint8, reflect.Uint16, reflect.Uint32, reflect.Uint64:
		return float64(v.Uint()), true
	case reflect.Float32, reflect.Float64:
		return v.Convert(reflect.TypeOf(float64(0))).Float(), true
	default:
		return 0, false
	}
}
