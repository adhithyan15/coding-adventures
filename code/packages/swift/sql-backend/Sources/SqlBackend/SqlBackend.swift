import Foundation

public typealias Row = [String: SQLValue]

public enum SQLValue: Equatable, Hashable, CustomStringConvertible {
    case null
    case bool(Bool)
    case integer(Int)
    case real(Double)
    case text(String)
    case blob([UInt8])

    public var description: String {
        switch self {
        case .null:
            return "NULL"
        case .bool(let value):
            return value ? "TRUE" : "FALSE"
        case .integer(let value):
            return String(value)
        case .real(let value):
            return String(value)
        case .text(let value):
            return value
        case .blob(let bytes):
            return "x'\(bytes.map { String(format: "%02x", $0) }.joined())'"
        }
    }
}

public enum SQLValues {
    public static func blob(_ bytes: [UInt8]) -> SQLValue {
        .blob(bytes)
    }

    public static func isSQLValue(_ value: Any?) -> Bool {
        guard let value else { return true }
        switch value {
        case is SQLValue, is Bool, is Int, is Double, is String, is [UInt8]:
            return true
        default:
            return false
        }
    }

    public static func typeName(_ value: SQLValue) -> String {
        switch value {
        case .null:
            return "NULL"
        case .bool:
            return "BOOLEAN"
        case .integer:
            return "INTEGER"
        case .real:
            return "REAL"
        case .text:
            return "TEXT"
        case .blob:
            return "BLOB"
        }
    }

    public static func compare(_ left: SQLValue, _ right: SQLValue) -> Int {
        let leftRank = rank(left)
        let rightRank = rank(right)
        if leftRank != rightRank {
            return leftRank < rightRank ? -1 : 1
        }

        switch (left, right) {
        case (.null, .null):
            return 0
        case (.bool(let lhs), .bool(let rhs)):
            return compareInts(lhs ? 1 : 0, rhs ? 1 : 0)
        case (.integer(let lhs), .integer(let rhs)):
            return compareInts(lhs, rhs)
        case (.integer(let lhs), .real(let rhs)):
            return compareDoubles(Double(lhs), rhs)
        case (.real(let lhs), .integer(let rhs)):
            return compareDoubles(lhs, Double(rhs))
        case (.real(let lhs), .real(let rhs)):
            return compareDoubles(lhs, rhs)
        case (.text(let lhs), .text(let rhs)):
            return compareComparable(lhs, rhs)
        case (.blob(let lhs), .blob(let rhs)):
            return compareBytes(lhs, rhs)
        default:
            return 0
        }
    }

    private static func rank(_ value: SQLValue) -> Int {
        switch value {
        case .null:
            return 0
        case .bool:
            return 1
        case .integer, .real:
            return 2
        case .text:
            return 3
        case .blob:
            return 4
        }
    }

    private static func compareInts(_ left: Int, _ right: Int) -> Int {
        if left < right { return -1 }
        if left > right { return 1 }
        return 0
    }

    private static func compareDoubles(_ left: Double, _ right: Double) -> Int {
        if left < right { return -1 }
        if left > right { return 1 }
        return 0
    }

    private static func compareComparable<T: Comparable>(_ left: T, _ right: T) -> Int {
        if left < right { return -1 }
        if left > right { return 1 }
        return 0
    }

    private static func compareBytes(_ left: [UInt8], _ right: [UInt8]) -> Int {
        for (lhs, rhs) in zip(left, right) {
            if lhs < rhs { return -1 }
            if lhs > rhs { return 1 }
        }
        return compareInts(left.count, right.count)
    }
}

public struct BackendError: Error, Equatable, CustomStringConvertible {
    public let kind: String
    public let message: String
    public let table: String?
    public let column: String?

    public init(_ kind: String, _ message: String, table: String? = nil, column: String? = nil) {
        self.kind = kind
        self.message = message
        self.table = table
        self.column = column
    }

    public var description: String {
        "\(kind): \(message)"
    }

    static func tableNotFound(_ table: String) -> BackendError {
        BackendError("TableNotFound", "table not found: \(table)", table: table)
    }

    static func tableAlreadyExists(_ table: String) -> BackendError {
        BackendError("TableAlreadyExists", "table already exists: \(table)", table: table)
    }

    static func columnNotFound(_ table: String, _ column: String) -> BackendError {
        BackendError("ColumnNotFound", "column not found: \(table).\(column)", table: table, column: column)
    }

    static func columnAlreadyExists(_ table: String, _ column: String) -> BackendError {
        BackendError("ColumnAlreadyExists", "column already exists: \(table).\(column)", table: table, column: column)
    }

    static func constraintViolation(_ table: String, _ column: String, _ message: String) -> BackendError {
        BackendError("ConstraintViolation", message, table: table, column: column)
    }

    static func unsupported(_ operation: String) -> BackendError {
        BackendError("Unsupported", "operation not supported: \(operation)")
    }

    static func internalError(_ message: String) -> BackendError {
        BackendError("Internal", message)
    }

    static func indexAlreadyExists(_ index: String) -> BackendError {
        BackendError("IndexAlreadyExists", "index already exists: \(index)")
    }

    static func indexNotFound(_ index: String) -> BackendError {
        BackendError("IndexNotFound", "index not found: \(index)")
    }

    static func triggerAlreadyExists(_ name: String) -> BackendError {
        BackendError("TriggerAlreadyExists", "trigger already exists: \(name)")
    }

    static func triggerNotFound(_ name: String) -> BackendError {
        BackendError("TriggerNotFound", "trigger not found: \(name)")
    }
}

public struct ColumnDef: Equatable {
    public var name: String
    public var typeName: String
    public var notNull: Bool
    public var primaryKey: Bool
    public var unique: Bool
    public var autoincrement: Bool
    public var defaultValue: SQLValue
    public var hasDefault: Bool
    public var checkExpression: String?
    public var foreignKey: String?

    public init(
        _ name: String,
        _ typeName: String,
        notNull: Bool = false,
        primaryKey: Bool = false,
        unique: Bool = false,
        autoincrement: Bool = false,
        defaultValue: SQLValue? = nil,
        checkExpression: String? = nil,
        foreignKey: String? = nil
    ) {
        self.name = name
        self.typeName = typeName
        self.notNull = notNull
        self.primaryKey = primaryKey
        self.unique = unique
        self.autoincrement = autoincrement
        self.defaultValue = defaultValue ?? .null
        self.hasDefault = defaultValue != nil
        self.checkExpression = checkExpression
        self.foreignKey = foreignKey
    }

    public var effectiveNotNull: Bool {
        notNull || primaryKey
    }

    public var effectiveUnique: Bool {
        unique || primaryKey
    }
}

public struct IndexDef: Equatable {
    public var name: String
    public var table: String
    public var columns: [String]
    public var unique: Bool
    public var auto: Bool

    public init(_ name: String, table: String, columns: [String], unique: Bool = false, auto: Bool = false) {
        self.name = name
        self.table = table
        self.columns = columns
        self.unique = unique
        self.auto = auto
    }
}

public struct TriggerDef: Equatable {
    public var name: String
    public var table: String
    public var timing: String
    public var event: String
    public var body: String

    public init(_ name: String, table: String, timing: String, event: String, body: String) {
        self.name = name
        self.table = table
        self.timing = timing.uppercased()
        self.event = event.uppercased()
        self.body = body
    }
}

public protocol SchemaProvider {
    func columns(_ table: String) throws -> [String]
    func listIndexes(_ table: String?) throws -> [IndexDef]
}

public struct BackendSchemaProvider: SchemaProvider {
    private let backend: InMemoryBackend

    fileprivate init(backend: InMemoryBackend) {
        self.backend = backend
    }

    public func columns(_ table: String) throws -> [String] {
        try backend.columns(table).map(\.name)
    }

    public func listIndexes(_ table: String? = nil) throws -> [IndexDef] {
        backend.listIndexes(table)
    }
}

public func backendAsSchemaProvider(_ backend: InMemoryBackend) -> BackendSchemaProvider {
    BackendSchemaProvider(backend: backend)
}

public final class ListRowIterator {
    private var rows: [Row]
    private var index: Int
    private var closed: Bool

    public init(_ rows: [Row]) {
        self.rows = rows.map { Row(uniqueKeysWithValues: $0.map { ($0.key, $0.value) }) }
        self.index = 0
        self.closed = false
    }

    public func next() -> Row? {
        guard !closed, index < rows.count else { return nil }
        defer { index += 1 }
        return rows[index]
    }

    public func close() {
        closed = true
    }

    public func toArray() -> [Row] {
        var output: [Row] = []
        while let row = next() {
            output.append(row)
        }
        return output
    }
}

fileprivate struct StoredRow: Equatable {
    var rowID: Int
    var row: Row
}

public final class ListCursor {
    public let tableKey: String
    private let rows: [StoredRow]
    private var index: Int
    public private(set) var currentRowID: Int?

    fileprivate init(tableKey: String, rows: [StoredRow]) {
        self.tableKey = tableKey
        self.rows = rows
        self.index = -1
        self.currentRowID = nil
    }

    public func next() -> Row? {
        index += 1
        guard index < rows.count else {
            currentRowID = nil
            return nil
        }
        currentRowID = rows[index].rowID
        return rows[index].row
    }

    public func currentRow() -> Row? {
        guard let rowID = currentRowID else { return nil }
        return rows.first(where: { $0.rowID == rowID })?.row
    }
}

fileprivate struct TableState: Equatable {
    var name: String
    var columns: [ColumnDef]
    var rows: [StoredRow]
    var nextRowID: Int

    init(name: String, columns: [ColumnDef], rows: [StoredRow] = [], nextRowID: Int = 0) {
        self.name = name
        self.columns = columns
        self.rows = rows
        self.nextRowID = nextRowID
    }
}

fileprivate struct BackendSnapshot {
    var tablesByKey: [String: TableState]
    var indexesByKey: [String: IndexDef]
    var triggersByKey: [String: TriggerDef]
    var triggersByTable: [String: [String]]
    var userVersion: Int
    var schemaVersion: Int
}

fileprivate struct Savepoint {
    var name: String
    var snapshot: BackendSnapshot
}

public final class InMemoryBackend {
    private var tablesByKey: [String: TableState]
    private var indexesByKey: [String: IndexDef]
    private var triggersByKey: [String: TriggerDef]
    private var triggersByTable: [String: [String]]
    private var transactionSnapshot: BackendSnapshot?
    private var nextTransaction: Int
    private var savepoints: [Savepoint]

    public var userVersion: Int
    public private(set) var schemaVersion: Int
    public private(set) var currentTransaction: Int?

    public init() {
        self.tablesByKey = [:]
        self.indexesByKey = [:]
        self.triggersByKey = [:]
        self.triggersByTable = [:]
        self.transactionSnapshot = nil
        self.nextTransaction = 1
        self.savepoints = []
        self.userVersion = 0
        self.schemaVersion = 0
        self.currentTransaction = nil
    }

    public func tables() -> [String] {
        tablesByKey.values.map(\.name).sorted()
    }

    public func columns(_ table: String) throws -> [ColumnDef] {
        try tableState(table).columns
    }

    public func scan(_ table: String) throws -> ListRowIterator {
        try ListRowIterator(tableState(table).rows.map(\.row))
    }

    public func openCursor(_ table: String) throws -> ListCursor {
        let state = try tableState(table)
        return ListCursor(tableKey: normalize(state.name), rows: state.rows)
    }

    @discardableResult
    public func insert(_ table: String, _ row: Row) throws -> InMemoryBackend {
        var state = try tableState(table)
        let candidate = try materializeRow(state, row)
        try validateRow(state, candidate)
        state.rows.append(StoredRow(rowID: state.nextRowID, row: candidate))
        state.nextRowID += 1
        putState(state)
        return self
    }

    @discardableResult
    public func update(_ table: String, cursor: ListCursor, assignments: Row) throws -> InMemoryBackend {
        var state = try tableState(table)
        guard cursor.tableKey == normalize(state.name), let rowID = cursor.currentRowID else {
            throw BackendError.internalError("cursor is not positioned on \(state.name)")
        }
        guard let rowIndex = state.rows.firstIndex(where: { $0.rowID == rowID }) else {
            throw BackendError.internalError("cursor row vanished")
        }

        var candidate = state.rows[rowIndex].row
        for (name, value) in assignments {
            guard let column = findColumn(state, name) else {
                throw BackendError.columnNotFound(state.name, name)
            }
            candidate[column.name] = value
        }

        try validateRow(state, candidate, skippingRowID: rowID)
        state.rows[rowIndex].row = candidate
        putState(state)
        return self
    }

    @discardableResult
    public func delete(_ table: String, cursor: ListCursor) throws -> InMemoryBackend {
        var state = try tableState(table)
        guard cursor.tableKey == normalize(state.name), let rowID = cursor.currentRowID else {
            throw BackendError.internalError("cursor is not positioned on \(state.name)")
        }
        state.rows.removeAll { $0.rowID == rowID }
        putState(state)
        return self
    }

    @discardableResult
    public func createTable(_ table: String, _ columns: [ColumnDef], ifNotExists: Bool = false) throws -> InMemoryBackend {
        let key = normalize(table)
        if tablesByKey[key] != nil {
            if ifNotExists { return self }
            throw BackendError.tableAlreadyExists(table)
        }

        var seen: Set<String> = []
        for column in columns {
            let columnKey = normalize(column.name)
            if seen.contains(columnKey) {
                throw BackendError.columnAlreadyExists(table, column.name)
            }
            seen.insert(columnKey)
        }

        schemaVersion += 1
        putState(TableState(name: table, columns: columns))
        return self
    }

    @discardableResult
    public func dropTable(_ table: String, ifExists: Bool = false) throws -> InMemoryBackend {
        let key = normalize(table)
        guard tablesByKey[key] != nil else {
            if ifExists { return self }
            throw BackendError.tableNotFound(table)
        }
        tablesByKey.removeValue(forKey: key)
        indexesByKey = indexesByKey.filter { normalize($0.value.table) != key }
        triggersByKey = triggersByKey.filter { normalize($0.value.table) != key }
        triggersByTable.removeValue(forKey: key)
        schemaVersion += 1
        return self
    }

    @discardableResult
    public func addColumn(_ table: String, _ column: ColumnDef) throws -> InMemoryBackend {
        var state = try tableState(table)
        if findColumn(state, column.name) != nil {
            throw BackendError.columnAlreadyExists(state.name, column.name)
        }
        if column.effectiveNotNull && !column.hasDefault && !state.rows.isEmpty {
            throw BackendError.constraintViolation(state.name, column.name, "NOT NULL constraint failed: \(state.name).\(column.name)")
        }
        state.columns.append(column)
        state.rows = state.rows.map { stored in
            var stored = stored
            stored.row[column.name] = column.defaultValue
            return stored
        }
        schemaVersion += 1
        putState(state)
        return self
    }

    @discardableResult
    public func createIndex(_ index: IndexDef) throws -> InMemoryBackend {
        let key = normalize(index.name)
        if indexesByKey[key] != nil {
            throw BackendError.indexAlreadyExists(index.name)
        }
        let state = try tableState(index.table)
        for column in index.columns {
            _ = try realColumn(state, column)
        }
        if index.unique {
            try validateUniqueIndex(state, index)
        }
        indexesByKey[key] = index
        schemaVersion += 1
        return self
    }

    @discardableResult
    public func dropIndex(_ name: String, ifExists: Bool = false) throws -> InMemoryBackend {
        let key = normalize(name)
        guard indexesByKey[key] != nil else {
            if ifExists { return self }
            throw BackendError.indexNotFound(name)
        }
        indexesByKey.removeValue(forKey: key)
        schemaVersion += 1
        return self
    }

    public func listIndexes(_ table: String? = nil) -> [IndexDef] {
        indexesByKey.values
            .filter { index in table == nil || normalize(index.table) == normalize(table!) }
            .sorted { $0.name < $1.name }
    }

    public func scanIndex(
        _ indexName: String,
        lo: [SQLValue]? = nil,
        hi: [SQLValue]? = nil,
        loInclusive: Bool = true,
        hiInclusive: Bool = true
    ) throws -> [Int] {
        guard let index = indexesByKey[normalize(indexName)] else {
            throw BackendError.indexNotFound(indexName)
        }
        let state = try tableState(index.table)
        var entries: [([SQLValue], Int)] = []
        for stored in state.rows {
            entries.append((try indexKey(state, stored.row, index.columns), stored.rowID))
        }

        entries.sort { left, right in
            let comparison = compareKeys(left.0, right.0)
            return comparison == 0 ? left.1 < right.1 : comparison < 0
        }

        return entries.compactMap { key, rowID in
            let lowerOK = lo.map { bound in
                let comparison = compareKeys(key, bound)
                return comparison > 0 || (loInclusive && comparison == 0)
            } ?? true

            let upperOK = hi.map { bound in
                let comparison = compareKeys(key, bound)
                return comparison < 0 || (hiInclusive && comparison == 0)
            } ?? true

            return lowerOK && upperOK ? rowID : nil
        }
    }

    public func scanByRowIDs(_ table: String, _ rowIDs: [Int]) throws -> ListRowIterator {
        let state = try tableState(table)
        let rowsByID = Dictionary(uniqueKeysWithValues: state.rows.map { ($0.rowID, $0.row) })
        return ListRowIterator(rowIDs.compactMap { rowsByID[$0] })
    }

    public func beginTransaction() throws -> Int {
        guard currentTransaction == nil else {
            throw BackendError.unsupported("nested transactions")
        }
        let handle = nextTransaction
        transactionSnapshot = snapshot()
        currentTransaction = handle
        nextTransaction += 1
        return handle
    }

    @discardableResult
    public func commit(_ handle: Int) throws -> InMemoryBackend {
        guard currentTransaction == handle else {
            throw BackendError.internalError("invalid transaction handle")
        }
        transactionSnapshot = nil
        currentTransaction = nil
        savepoints = []
        return self
    }

    @discardableResult
    public func rollback(_ handle: Int) throws -> InMemoryBackend {
        guard currentTransaction == handle, let original = transactionSnapshot else {
            throw BackendError.internalError("invalid transaction handle")
        }
        restore(original)
        transactionSnapshot = nil
        currentTransaction = nil
        savepoints = []
        return self
    }

    @discardableResult
    public func createSavepoint(_ name: String) throws -> InMemoryBackend {
        guard currentTransaction != nil else {
            throw BackendError.unsupported("savepoints outside transaction")
        }
        savepoints.append(Savepoint(name: name, snapshot: snapshot()))
        return self
    }

    @discardableResult
    public func releaseSavepoint(_ name: String) throws -> InMemoryBackend {
        let index = try savepointIndex(name)
        savepoints = Array(savepoints.prefix(index))
        return self
    }

    @discardableResult
    public func rollbackToSavepoint(_ name: String) throws -> InMemoryBackend {
        let index = try savepointIndex(name)
        let savepoint = savepoints[index]
        restore(savepoint.snapshot)
        savepoints = Array(savepoints.prefix(index + 1))
        return self
    }

    @discardableResult
    public func createTrigger(_ trigger: TriggerDef) throws -> InMemoryBackend {
        let key = normalize(trigger.name)
        if triggersByKey[key] != nil {
            throw BackendError.triggerAlreadyExists(trigger.name)
        }
        _ = try tableState(trigger.table)
        triggersByKey[key] = TriggerDef(trigger.name, table: trigger.table, timing: trigger.timing, event: trigger.event, body: trigger.body)
        let tableKey = normalize(trigger.table)
        triggersByTable[tableKey, default: []].append(key)
        schemaVersion += 1
        return self
    }

    @discardableResult
    public func dropTrigger(_ name: String, ifExists: Bool = false) throws -> InMemoryBackend {
        let key = normalize(name)
        guard let trigger = triggersByKey[key] else {
            if ifExists { return self }
            throw BackendError.triggerNotFound(name)
        }
        triggersByKey.removeValue(forKey: key)
        let tableKey = normalize(trigger.table)
        triggersByTable[tableKey] = triggersByTable[tableKey]?.filter { $0 != key }
        schemaVersion += 1
        return self
    }

    public func listTriggers(_ table: String) -> [TriggerDef] {
        (triggersByTable[normalize(table)] ?? [])
            .compactMap { triggersByKey[$0] }
            .sorted { $0.name < $1.name }
    }

    private func tableState(_ table: String) throws -> TableState {
        guard let state = tablesByKey[normalize(table)] else {
            throw BackendError.tableNotFound(table)
        }
        return state
    }

    private func putState(_ state: TableState) {
        tablesByKey[normalize(state.name)] = state
    }

    private func materializeRow(_ state: TableState, _ row: Row) throws -> Row {
        var candidate: Row = [:]
        for column in state.columns {
            let found = findValue(row, column.name)
            let value: SQLValue
            if let found {
                value = found
            } else if column.autoincrement && column.primaryKey {
                value = .integer(nextAutoincrementValue(state, column))
            } else if column.hasDefault {
                value = column.defaultValue
            } else {
                value = .null
            }
            candidate[column.name] = value
        }

        for name in row.keys where findColumn(state, name) == nil {
            throw BackendError.columnNotFound(state.name, name)
        }
        return candidate
    }

    private func findValue(_ row: Row, _ name: String) -> SQLValue? {
        row.first { normalize($0.key) == normalize(name) }?.value
    }

    private func nextAutoincrementValue(_ state: TableState, _ column: ColumnDef) -> Int {
        (state.rows.compactMap { stored -> Int? in
            if case .integer(let value)? = stored.row[column.name] {
                return value
            }
            return nil
        }.max() ?? 0) + 1
    }

    private func validateRow(_ state: TableState, _ row: Row, skippingRowID: Int? = nil) throws {
        for column in state.columns {
            let value = row[column.name] ?? .null
            if column.effectiveNotNull && value == .null {
                throw BackendError.constraintViolation(state.name, column.name, "NOT NULL constraint failed: \(state.name).\(column.name)")
            }

            if column.effectiveUnique && value != .null {
                for stored in state.rows where stored.rowID != skippingRowID {
                    if SQLValues.compare(stored.row[column.name] ?? .null, value) == 0 {
                        let label = column.primaryKey ? "PRIMARY KEY" : "UNIQUE"
                        throw BackendError.constraintViolation(state.name, column.name, "\(label) constraint failed: \(state.name).\(column.name)")
                    }
                }
            }
        }

        for index in indexesByKey.values where index.unique && normalize(index.table) == normalize(state.name) {
            try validateUniqueIndex(state, index, candidate: row, skippingRowID: skippingRowID)
        }
    }

    private func validateUniqueIndex(
        _ state: TableState,
        _ index: IndexDef,
        candidate: Row? = nil,
        skippingRowID: Int? = nil
    ) throws {
        if let candidate {
            let candidateKey = try indexKey(state, candidate, index.columns)
            if candidateKey.contains(.null) { return }
            for stored in state.rows where stored.rowID != skippingRowID {
                if compareKeys(try indexKey(state, stored.row, index.columns), candidateKey) == 0 {
                    let columns = index.columns.joined(separator: ",")
                    throw BackendError.constraintViolation(state.name, columns, "UNIQUE constraint failed: \(state.name).\(columns)")
                }
            }
            return
        }

        var keys: [[SQLValue]] = []
        for stored in state.rows {
            let key = try indexKey(state, stored.row, index.columns)
            if !key.contains(.null) {
                keys.append(key)
            }
        }
        for leftIndex in keys.indices {
            for rightIndex in keys.indices where rightIndex > leftIndex {
                if compareKeys(keys[leftIndex], keys[rightIndex]) == 0 {
                    let columns = index.columns.joined(separator: ",")
                    throw BackendError.constraintViolation(state.name, columns, "UNIQUE constraint failed: \(state.name).\(columns)")
                }
            }
        }
    }

    private func findColumn(_ state: TableState, _ name: String) -> ColumnDef? {
        state.columns.first { normalize($0.name) == normalize(name) }
    }

    private func realColumn(_ state: TableState, _ name: String) throws -> String {
        guard let column = findColumn(state, name) else {
            throw BackendError.columnNotFound(state.name, name)
        }
        return column.name
    }

    private func indexKey(_ state: TableState, _ row: Row, _ columns: [String]) throws -> [SQLValue] {
        try columns.map { column in
            let realName = try realColumn(state, column)
            return row[realName] ?? .null
        }
    }

    private func savepointIndex(_ name: String) throws -> Int {
        guard currentTransaction != nil else {
            throw BackendError.unsupported("savepoints outside transaction")
        }
        guard let index = savepoints.firstIndex(where: { $0.name == name }) else {
            throw BackendError.internalError("savepoint not found: \(name)")
        }
        return index
    }

    private func snapshot() -> BackendSnapshot {
        BackendSnapshot(
            tablesByKey: tablesByKey,
            indexesByKey: indexesByKey,
            triggersByKey: triggersByKey,
            triggersByTable: triggersByTable,
            userVersion: userVersion,
            schemaVersion: schemaVersion
        )
    }

    private func restore(_ snapshot: BackendSnapshot) {
        tablesByKey = snapshot.tablesByKey
        indexesByKey = snapshot.indexesByKey
        triggersByKey = snapshot.triggersByKey
        triggersByTable = snapshot.triggersByTable
        userVersion = snapshot.userVersion
        schemaVersion = snapshot.schemaVersion
    }
}

private func normalize(_ value: String) -> String {
    value.lowercased()
}

private func compareKeys(_ left: [SQLValue], _ right: [SQLValue]) -> Int {
    for (lhs, rhs) in zip(left, right) {
        let comparison = SQLValues.compare(lhs, rhs)
        if comparison != 0 {
            return comparison
        }
    }
    if left.count < right.count { return -1 }
    if left.count > right.count { return 1 }
    return 0
}
