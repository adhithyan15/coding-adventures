package com.codingadventures.sqlbackend;

import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Base64;
import java.util.Collections;
import java.util.Comparator;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.Objects;
import java.util.Optional;
import java.util.TreeMap;
import java.util.stream.Collectors;

public final class SqlBackend {
    private SqlBackend() {
    }

    public record TransactionHandle(int value) {
    }

    public static final class SqlValues {
        private SqlValues() {
        }

        public static boolean isSqlValue(Object value) {
            return value == null
                || value instanceof Boolean
                || value instanceof String
                || value instanceof byte[]
                || isInteger(value)
                || isReal(value);
        }

        public static String typeName(Object value) {
            if (value == null) {
                return "NULL";
            }
            if (value instanceof Boolean) {
                return "BOOLEAN";
            }
            if (isInteger(value)) {
                return "INTEGER";
            }
            if (isReal(value)) {
                return "REAL";
            }
            if (value instanceof String) {
                return "TEXT";
            }
            if (value instanceof byte[]) {
                return "BLOB";
            }
            throw new IllegalArgumentException("not a SQL value: " + value.getClass().getSimpleName());
        }

        @SuppressWarnings({"unchecked", "rawtypes"})
        static int compareValues(Object left, Object right) {
            int rank = Integer.compare(rank(left), rank(right));
            if (rank != 0) {
                return rank;
            }
            if (left == null) {
                return 0;
            }
            if (left instanceof Boolean leftBool && right instanceof Boolean rightBool) {
                return Boolean.compare(leftBool, rightBool);
            }
            if (isNumber(left) && isNumber(right)) {
                return Double.compare(((Number) left).doubleValue(), ((Number) right).doubleValue());
            }
            if (left instanceof String leftText && right instanceof String rightText) {
                return leftText.compareTo(rightText);
            }
            if (left instanceof byte[] leftBytes && right instanceof byte[] rightBytes) {
                for (int i = 0; i < Math.min(leftBytes.length, rightBytes.length); i++) {
                    int cmp = Byte.compare(leftBytes[i], rightBytes[i]);
                    if (cmp != 0) {
                        return cmp;
                    }
                }
                return Integer.compare(leftBytes.length, rightBytes.length);
            }
            if (left instanceof Comparable comparable && right != null && left.getClass().isInstance(right)) {
                return comparable.compareTo(right);
            }
            return String.valueOf(left).compareTo(String.valueOf(right));
        }

        private static int rank(Object value) {
            if (value == null) {
                return 0;
            }
            if (value instanceof Boolean) {
                return 1;
            }
            if (isNumber(value)) {
                return 2;
            }
            if (value instanceof String) {
                return 3;
            }
            if (value instanceof byte[]) {
                return 4;
            }
            return 5;
        }

        private static boolean isInteger(Object value) {
            return value instanceof Byte
                || value instanceof Short
                || value instanceof Integer
                || value instanceof Long;
        }

        private static boolean isReal(Object value) {
            return value instanceof Float || value instanceof Double;
        }

        private static boolean isNumber(Object value) {
            return isInteger(value) || isReal(value);
        }
    }

    public static final class Row extends LinkedHashMap<String, Object> {
        public Row() {
            super();
        }

        public Row(Map<String, ?> values) {
            this();
            values.forEach(this::put);
        }

        public Row copy() {
            return new Row(this);
        }
    }

    public interface RowIterator extends AutoCloseable {
        Row next();

        @Override
        void close();
    }

    public interface Cursor extends RowIterator {
        Row currentRow();
    }

    public static final class ListRowIterator implements RowIterator {
        private final List<Row> rows;
        private int index;
        private boolean closed;

        public ListRowIterator(List<Row> rows) {
            this.rows = rows.stream().map(Row::copy).toList();
        }

        @Override
        public Row next() {
            if (closed || index >= rows.size()) {
                return null;
            }
            return rows.get(index++).copy();
        }

        @Override
        public void close() {
            closed = true;
        }
    }

    public static final class ListCursor implements Cursor {
        private final List<Row> rows;
        private int index = -1;
        private Row current;
        private boolean closed;

        private ListCursor(List<Row> rows) {
            this.rows = rows;
        }

        @Override
        public Row currentRow() {
            return current == null ? null : current.copy();
        }

        @Override
        public Row next() {
            if (closed) {
                return null;
            }
            index++;
            if (index >= rows.size()) {
                current = null;
                return null;
            }
            current = rows.get(index);
            return current.copy();
        }

        @Override
        public void close() {
            closed = true;
        }

        private boolean isBackedBy(List<Row> candidate) {
            return rows == candidate;
        }

        private int currentIndex() {
            return index;
        }

        private void adjustAfterDelete() {
            index--;
            current = null;
        }
    }

    public record ColumnDef(
        String name,
        String typeName,
        boolean notNull,
        boolean primaryKey,
        boolean unique,
        boolean autoincrement,
        Object defaultValue,
        boolean hasDefault,
        Object checkExpression,
        Object foreignKey
    ) {
        public ColumnDef(String name, String typeName) {
            this(name, typeName, false, false, false, false, null, false, null, null);
        }

        public ColumnDef(String name, String typeName, boolean notNull, boolean primaryKey, boolean unique) {
            this(name, typeName, notNull, primaryKey, unique, false, null, false, null, null);
        }

        public static ColumnDef withDefault(String name, String typeName, Object defaultValue) {
            return new ColumnDef(name, typeName, false, false, false, false, defaultValue, true, null, null);
        }

        public boolean effectiveNotNull() {
            return notNull || primaryKey;
        }

        public boolean effectiveUnique() {
            return unique || primaryKey;
        }
    }

    public record TriggerDef(String name, String table, String timing, String event, String body) {
    }

    public record IndexDef(String name, String table, List<String> columns, boolean unique, boolean auto) {
        public IndexDef(String name, String table, List<String> columns) {
            this(name, table, columns, false, false);
        }

        public IndexDef(String name, String table) {
            this(name, table, List.of(), false, false);
        }

        public IndexDef {
            columns = List.copyOf(columns == null ? List.of() : columns);
        }
    }

    public abstract static class BackendError extends RuntimeException {
        protected BackendError(String message) {
            super(message);
        }
    }

    public static final class TableNotFound extends BackendError {
        public TableNotFound(String table) {
            super("table not found: '" + table + "'");
        }
    }

    public static final class TableAlreadyExists extends BackendError {
        public TableAlreadyExists(String table) {
            super("table already exists: '" + table + "'");
        }
    }

    public static final class ColumnNotFound extends BackendError {
        public ColumnNotFound(String table, String column) {
            super("column not found: '" + table + "." + column + "'");
        }
    }

    public static final class ColumnAlreadyExists extends BackendError {
        public ColumnAlreadyExists(String table, String column) {
            super("column already exists: '" + table + "." + column + "'");
        }
    }

    public static final class ConstraintViolation extends BackendError {
        public ConstraintViolation(String table, String column, String message) {
            super(message);
        }
    }

    public static final class Unsupported extends BackendError {
        public Unsupported(String operation) {
            super("unsupported operation: " + operation);
        }
    }

    public static final class Internal extends BackendError {
        public Internal(String message) {
            super(message);
        }
    }

    public static final class IndexAlreadyExists extends BackendError {
        public IndexAlreadyExists(String index) {
            super("index already exists: '" + index + "'");
        }
    }

    public static final class IndexNotFound extends BackendError {
        public IndexNotFound(String index) {
            super("index not found: '" + index + "'");
        }
    }

    public static final class TriggerAlreadyExists extends BackendError {
        public TriggerAlreadyExists(String trigger) {
            super("trigger already exists: '" + trigger + "'");
        }
    }

    public static final class TriggerNotFound extends BackendError {
        public TriggerNotFound(String trigger) {
            super("trigger not found: '" + trigger + "'");
        }
    }

    public abstract static class Backend {
        public abstract List<String> tables();
        public abstract List<ColumnDef> columns(String table);
        public abstract RowIterator scan(String table);
        public abstract void insert(String table, Row row);
        public abstract void update(String table, Cursor cursor, Map<String, Object> assignments);
        public abstract void delete(String table, Cursor cursor);
        public abstract void createTable(String table, List<ColumnDef> columns, boolean ifNotExists);
        public abstract void dropTable(String table, boolean ifExists);
        public abstract void addColumn(String table, ColumnDef column);
        public abstract void createIndex(IndexDef index);
        public abstract void dropIndex(String name, boolean ifExists);
        public abstract List<IndexDef> listIndexes(String table);
        public abstract Iterable<Integer> scanIndex(String indexName, List<Object> lo, List<Object> hi, boolean loInclusive, boolean hiInclusive);
        public abstract RowIterator scanByRowids(String table, List<Integer> rowids);
        public abstract TransactionHandle beginTransaction();
        public abstract void commit(TransactionHandle handle);
        public abstract void rollback(TransactionHandle handle);

        public Optional<TransactionHandle> currentTransaction() {
            return Optional.empty();
        }

        public void createSavepoint(String name) {
            throw new Unsupported("savepoints");
        }

        public void releaseSavepoint(String name) {
            throw new Unsupported("savepoints");
        }

        public void rollbackToSavepoint(String name) {
            throw new Unsupported("savepoints");
        }

        public void createTrigger(TriggerDef defn) {
            throw new Unsupported("triggers");
        }

        public void dropTrigger(String name, boolean ifExists) {
            throw new Unsupported("triggers");
        }

        public List<TriggerDef> listTriggers(String table) {
            return List.of();
        }
    }

    public interface SchemaProvider {
        List<String> columns(String table);
    }

    public static SchemaProvider asSchemaProvider(Backend backend) {
        return table -> backend.columns(table).stream().map(ColumnDef::name).toList();
    }

    public static final class InMemoryBackend extends Backend {
        private final Map<String, TableState> tables = new TreeMap<>(String.CASE_INSENSITIVE_ORDER);
        private final Map<String, IndexDef> indexes = new TreeMap<>(String.CASE_INSENSITIVE_ORDER);
        private Snapshot snapshot;
        private TransactionHandle activeHandle;
        private int nextHandle = 1;

        @Override
        public List<String> tables() {
            return List.copyOf(tables.keySet());
        }

        @Override
        public List<ColumnDef> columns(String table) {
            return List.copyOf(requireTable(table).columns);
        }

        @Override
        public RowIterator scan(String table) {
            return new ListRowIterator(requireTable(table).rows);
        }

        public ListCursor openCursor(String table) {
            return new ListCursor(requireTable(table).rows);
        }

        @Override
        public void insert(String table, Row row) {
            TableState state = requireTable(table);
            Row normalized = applyDefaults(table, state, row);
            checkNotNull(table, state, normalized);
            checkUnique(table, state, normalized, -1);
            state.rows.add(normalized);
        }

        @Override
        public void update(String table, Cursor cursor, Map<String, Object> assignments) {
            TableState state = requireTable(table);
            ListCursor listCursor = requireListCursor(table, state, cursor);
            int index = listCursor.currentIndex();
            if (index < 0 || index >= state.rows.size()) {
                throw new Unsupported("cursor has no current row");
            }

            Row updated = state.rows.get(index).copy();
            assignments.forEach((column, value) -> updated.put(canonicalColumn(table, state, column), value));
            checkNotNull(table, state, updated);
            checkUnique(table, state, updated, index);
            state.rows.set(index, updated);
        }

        @Override
        public void delete(String table, Cursor cursor) {
            TableState state = requireTable(table);
            ListCursor listCursor = requireListCursor(table, state, cursor);
            int index = listCursor.currentIndex();
            if (index < 0 || index >= state.rows.size()) {
                throw new Unsupported("cursor has no current row");
            }
            state.rows.remove(index);
            listCursor.adjustAfterDelete();
        }

        @Override
        public void createTable(String table, List<ColumnDef> columns, boolean ifNotExists) {
            if (tables.containsKey(table)) {
                if (!ifNotExists) {
                    throw new TableAlreadyExists(table);
                }
                return;
            }

            List<String> seen = new ArrayList<>();
            for (ColumnDef column : columns) {
                if (seen.stream().anyMatch(existing -> same(existing, column.name()))) {
                    throw new ColumnAlreadyExists(table, column.name());
                }
                seen.add(column.name());
            }
            tables.put(table, new TableState(columns, List.of()));
        }

        @Override
        public void dropTable(String table, boolean ifExists) {
            if (tables.remove(table) == null) {
                if (!ifExists) {
                    throw new TableNotFound(table);
                }
                return;
            }
            indexes.values().removeIf(index -> same(index.table(), table));
        }

        @Override
        public void addColumn(String table, ColumnDef column) {
            TableState state = requireTable(table);
            if (state.columns.stream().anyMatch(existing -> same(existing.name(), column.name()))) {
                throw new ColumnAlreadyExists(table, column.name());
            }
            if (column.effectiveNotNull() && !column.hasDefault()) {
                throw new ConstraintViolation(table, column.name(), "NOT NULL constraint failed: " + table + "." + column.name());
            }
            state.columns.add(column);
            state.rows.forEach(row -> row.put(column.name(), column.hasDefault() ? column.defaultValue() : null));
        }

        @Override
        public void createIndex(IndexDef index) {
            if (indexes.containsKey(index.name())) {
                throw new IndexAlreadyExists(index.name());
            }
            TableState state = requireTable(index.table());
            index.columns().forEach(column -> canonicalColumn(index.table(), state, column));
            indexes.put(index.name(), cloneIndex(index));
        }

        @Override
        public void dropIndex(String name, boolean ifExists) {
            if (indexes.remove(name) == null && !ifExists) {
                throw new IndexNotFound(name);
            }
        }

        @Override
        public List<IndexDef> listIndexes(String table) {
            return indexes.values().stream()
                .filter(index -> table == null || same(index.table(), table))
                .map(InMemoryBackend::cloneIndex)
                .toList();
        }

        @Override
        public Iterable<Integer> scanIndex(String indexName, List<Object> lo, List<Object> hi, boolean loInclusive, boolean hiInclusive) {
            IndexDef index = indexes.get(indexName);
            if (index == null) {
                throw new IndexNotFound(indexName);
            }
            TableState state = requireTable(index.table());
            List<KeyedRow> keyed = new ArrayList<>();
            for (int rowid = 0; rowid < state.rows.size(); rowid++) {
                keyed.add(new KeyedRow(indexKey(state, state.rows.get(rowid), index.columns()), rowid));
            }
            keyed.sort((left, right) -> {
                int cmp = compareKey(left.key(), right.key());
                return cmp != 0 ? cmp : Integer.compare(left.rowid(), right.rowid());
            });

            List<Integer> rowids = new ArrayList<>();
            for (KeyedRow row : keyed) {
                if (lo != null) {
                    int cmp = comparePrefix(row.key(), lo);
                    if (cmp < 0 || (cmp == 0 && !loInclusive)) {
                        continue;
                    }
                }
                if (hi != null) {
                    int cmp = comparePrefix(row.key(), hi);
                    if (cmp > 0 || (cmp == 0 && !hiInclusive)) {
                        break;
                    }
                }
                rowids.add(row.rowid());
            }
            return rowids;
        }

        @Override
        public RowIterator scanByRowids(String table, List<Integer> rowids) {
            TableState state = requireTable(table);
            List<Row> rows = rowids.stream()
                .filter(rowid -> rowid >= 0 && rowid < state.rows.size())
                .map(state.rows::get)
                .toList();
            return new ListRowIterator(rows);
        }

        @Override
        public TransactionHandle beginTransaction() {
            if (activeHandle != null) {
                throw new Unsupported("nested transactions");
            }
            TransactionHandle handle = new TransactionHandle(nextHandle++);
            snapshot = capture();
            activeHandle = handle;
            return handle;
        }

        @Override
        public void commit(TransactionHandle handle) {
            requireActive(handle);
            snapshot = null;
            activeHandle = null;
        }

        @Override
        public void rollback(TransactionHandle handle) {
            requireActive(handle);
            if (snapshot != null) {
                restore(snapshot);
            }
            snapshot = null;
            activeHandle = null;
        }

        @Override
        public Optional<TransactionHandle> currentTransaction() {
            return Optional.ofNullable(activeHandle);
        }

        private TableState requireTable(String table) {
            TableState state = tables.get(table);
            if (state == null) {
                throw new TableNotFound(table);
            }
            return state;
        }

        private ListCursor requireListCursor(String table, TableState state, Cursor cursor) {
            if (cursor instanceof ListCursor listCursor && listCursor.isBackedBy(state.rows)) {
                return listCursor;
            }
            throw new Unsupported("foreign cursor for table " + table);
        }

        private Row applyDefaults(String table, TableState state, Row row) {
            Row normalized = row.copy();
            state.columns.forEach(column -> {
                if (!containsColumn(normalized, column.name())) {
                    normalized.put(column.name(), column.hasDefault() ? column.defaultValue() : null);
                }
            });
            for (String column : normalized.keySet()) {
                if (state.columns.stream().noneMatch(existing -> same(existing.name(), column))) {
                    throw new ColumnNotFound(table, column);
                }
            }
            return normalized;
        }

        private void checkNotNull(String table, TableState state, Row row) {
            for (ColumnDef column : state.columns) {
                if (column.effectiveNotNull() && row.get(column.name()) == null) {
                    throw new ConstraintViolation(table, column.name(), "NOT NULL constraint failed: " + table + "." + column.name());
                }
            }
        }

        private void checkUnique(String table, TableState state, Row row, int ignoreIndex) {
            for (ColumnDef column : state.columns) {
                if (!column.effectiveUnique()) {
                    continue;
                }
                Object value = row.get(column.name());
                if (value == null) {
                    continue;
                }
                for (int i = 0; i < state.rows.size(); i++) {
                    if (i == ignoreIndex) {
                        continue;
                    }
                    if (Objects.equals(state.rows.get(i).get(column.name()), value)) {
                        String label = column.primaryKey() ? "PRIMARY KEY" : "UNIQUE";
                        throw new ConstraintViolation(table, column.name(), label + " constraint failed: " + table + "." + column.name());
                    }
                }
            }
        }

        private String canonicalColumn(String table, TableState state, String column) {
            return state.columns.stream()
                .filter(candidate -> same(candidate.name(), column))
                .findFirst()
                .map(ColumnDef::name)
                .orElseThrow(() -> new ColumnNotFound(table, column));
        }

        private List<Object> indexKey(TableState state, Row row, List<String> columns) {
            return columns.stream()
                .map(column -> row.get(canonicalColumn("", state, column)))
                .collect(Collectors.toCollection(ArrayList::new));
        }

        private Snapshot capture() {
            Map<String, TableState> tableCopy = new TreeMap<>(String.CASE_INSENSITIVE_ORDER);
            tables.forEach((name, table) -> tableCopy.put(name, table.copy()));
            Map<String, IndexDef> indexCopy = new TreeMap<>(String.CASE_INSENSITIVE_ORDER);
            indexes.forEach((name, index) -> indexCopy.put(name, cloneIndex(index)));
            return new Snapshot(tableCopy, indexCopy);
        }

        private void restore(Snapshot snapshot) {
            tables.clear();
            snapshot.tables().forEach((name, table) -> tables.put(name, table.copy()));
            indexes.clear();
            snapshot.indexes().forEach((name, index) -> indexes.put(name, cloneIndex(index)));
        }

        private void requireActive(TransactionHandle handle) {
            if (activeHandle == null) {
                throw new Unsupported("no active transaction");
            }
            if (!activeHandle.equals(handle)) {
                throw new Unsupported("stale transaction handle");
            }
        }

        private static boolean same(String left, String right) {
            return left.equalsIgnoreCase(right);
        }

        private static boolean containsColumn(Row row, String column) {
            return row.keySet().stream().anyMatch(key -> same(key, column));
        }

        private static IndexDef cloneIndex(IndexDef index) {
            return new IndexDef(index.name(), index.table(), index.columns(), index.unique(), index.auto());
        }
    }

    private record KeyedRow(List<Object> key, int rowid) {
    }

    private record Snapshot(Map<String, TableState> tables, Map<String, IndexDef> indexes) {
    }

    private static final class TableState {
        private final List<ColumnDef> columns;
        private final List<Row> rows;

        private TableState(List<ColumnDef> columns, List<Row> rows) {
            this.columns = new ArrayList<>(columns);
            this.rows = rows.stream().map(Row::copy).collect(Collectors.toCollection(ArrayList::new));
        }

        private TableState copy() {
            return new TableState(columns, rows);
        }
    }

    private static int compareKey(List<Object> left, List<Object> right) {
        for (int i = 0; i < Math.min(left.size(), right.size()); i++) {
            int cmp = SqlValues.compareValues(left.get(i), right.get(i));
            if (cmp != 0) {
                return cmp;
            }
        }
        return Integer.compare(left.size(), right.size());
    }

    private static int comparePrefix(List<Object> key, List<Object> bound) {
        for (int i = 0; i < bound.size(); i++) {
            Object value = i < key.size() ? key.get(i) : null;
            int cmp = SqlValues.compareValues(value, bound.get(i));
            if (cmp != 0) {
                return cmp;
            }
        }
        return 0;
    }

    @SuppressWarnings("unused")
    private static String serializeKey(List<Object> key) {
        return key.stream().map(value -> {
            if (value == null) {
                return "NULL";
            }
            if (value instanceof byte[] bytes) {
                return Base64.getEncoder().encodeToString(bytes);
            }
            return String.format(Locale.ROOT, "%s", value);
        }).collect(Collectors.joining("\u001f"));
    }
}
