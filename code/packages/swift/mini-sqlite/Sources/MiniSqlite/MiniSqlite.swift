import Foundation

public enum SqlValue: Equatable, CustomStringConvertible {
    case null
    case integer(Int)
    case real(Double)
    case text(String)
    case bool(Bool)

    public var description: String {
        switch self {
        case .null: return "NULL"
        case .integer(let value): return String(value)
        case .real(let value): return String(value)
        case .text(let value): return value
        case .bool(let value): return value ? "TRUE" : "FALSE"
        }
    }
}

public struct MiniSqliteError: Error, Equatable, CustomStringConvertible {
    public let kind: String
    public let message: String

    public init(_ kind: String, _ message: String) {
        self.kind = kind
        self.message = message
    }

    public var description: String { "\(kind): \(message)" }
}

public struct ConnectionOptions: Equatable {
    public let autocommit: Bool

    public init(autocommit: Bool = false) {
        self.autocommit = autocommit
    }
}

public struct Column: Equatable {
    public let name: String

    public init(_ name: String) {
        self.name = name
    }
}

public enum MiniSqlite {
    public static let apiLevel = "2.0"
    public static let threadSafety = 1
    public static let paramStyle = "qmark"

    public static func connect(_ database: String, options: ConnectionOptions = ConnectionOptions()) throws -> Connection {
        guard database == ":memory:" else {
            throw MiniSqliteError("NotSupportedError", "Swift mini-sqlite supports only :memory: in Level 0")
        }
        return Connection(options: options)
    }
}

public final class Connection {
    private var db = Database()
    private let autocommit: Bool
    private var snapshot: Database?
    private var closed = false

    fileprivate init(options: ConnectionOptions) {
        self.autocommit = options.autocommit
    }

    public func cursor() throws -> Cursor {
        try assertOpen()
        return Cursor(connection: self)
    }

    @discardableResult
    public func execute(_ sql: String, _ parameters: [SqlValue] = []) throws -> Cursor {
        try cursor().execute(sql, parameters)
    }

    @discardableResult
    public func executeMany(_ sql: String, _ parameterSets: [[SqlValue]]) throws -> Cursor {
        try cursor().executeMany(sql, parameterSets)
    }

    public func commit() throws {
        try assertOpen()
        snapshot = nil
    }

    public func rollback() throws {
        try assertOpen()
        if let original = snapshot {
            db = original.copy()
            snapshot = nil
        }
    }

    public func close() {
        guard !closed else { return }
        if let original = snapshot {
            db = original.copy()
        }
        snapshot = nil
        closed = true
    }

    fileprivate func executeBound(_ sql: String, _ parameters: [SqlValue]) throws -> ExecutionResult {
        try assertOpen()
        let bound = try SqlText.bindParameters(sql, parameters)
        do {
            switch SqlText.firstKeyword(bound) {
            case "BEGIN":
                ensureSnapshot()
                return ExecutionResult.empty(rowCount: 0)
            case "COMMIT":
                snapshot = nil
                return ExecutionResult.empty(rowCount: 0)
            case "ROLLBACK":
                if let original = snapshot {
                    db = original.copy()
                }
                snapshot = nil
                return ExecutionResult.empty(rowCount: 0)
            case "SELECT":
                return try db.select(try Statements.parseSelect(bound))
            case "CREATE":
                return try withSnapshot { try db.create(try Statements.parseCreate(bound)) }
            case "DROP":
                return try withSnapshot { try db.drop(try Statements.parseDrop(bound)) }
            case "INSERT":
                return try withSnapshot { try db.insert(try Statements.parseInsert(bound)) }
            case "UPDATE":
                return try withSnapshot { try db.update(try Statements.parseUpdate(bound)) }
            case "DELETE":
                return try withSnapshot { try db.delete(try Statements.parseDelete(bound)) }
            default:
                throw MiniSqliteError("OperationalError", "unsupported SQL statement")
            }
        } catch let error as MiniSqliteError {
            throw error
        } catch {
            throw MiniSqliteError("OperationalError", String(describing: error))
        }
    }

    private func withSnapshot(_ action: () throws -> ExecutionResult) throws -> ExecutionResult {
        ensureSnapshot()
        return try action()
    }

    private func ensureSnapshot() {
        if !autocommit && snapshot == nil {
            snapshot = db.copy()
        }
    }

    private func assertOpen() throws {
        if closed {
            throw MiniSqliteError("ProgrammingError", "connection is closed")
        }
    }
}

public final class Cursor {
    private let connection: Connection
    private var rows: [[SqlValue]] = []
    private var offset = 0
    private var closed = false

    public private(set) var description: [Column] = []
    public private(set) var rowCount = -1
    public private(set) var lastRowId: SqlValue?
    public var arraySize = 1

    fileprivate init(connection: Connection) {
        self.connection = connection
    }

    @discardableResult
    public func execute(_ sql: String, _ parameters: [SqlValue] = []) throws -> Cursor {
        try assertOpen()
        let result = try connection.executeBound(sql, parameters)
        rows = result.rows
        offset = 0
        description = result.columns.map(Column.init)
        rowCount = result.rowCount
        lastRowId = result.lastRowId
        return self
    }

    @discardableResult
    public func executeMany(_ sql: String, _ parameterSets: [[SqlValue]]) throws -> Cursor {
        var last = self
        for parameters in parameterSets {
            last = try execute(sql, parameters)
        }
        return last
    }

    public func fetchOne() throws -> [SqlValue]? {
        try assertOpen()
        guard offset < rows.count else { return nil }
        let row = rows[offset]
        offset += 1
        return row
    }

    public func fetchMany(_ size: Int? = nil) throws -> [[SqlValue]] {
        try assertOpen()
        let limit = max(0, size ?? arraySize)
        var output: [[SqlValue]] = []
        while output.count < limit && offset < rows.count {
            output.append(rows[offset])
            offset += 1
        }
        return output
    }

    public func fetchAll() throws -> [[SqlValue]] {
        try assertOpen()
        var output: [[SqlValue]] = []
        while offset < rows.count {
            output.append(rows[offset])
            offset += 1
        }
        return output
    }

    public func close() {
        closed = true
    }

    private func assertOpen() throws {
        if closed {
            throw MiniSqliteError("ProgrammingError", "cursor is closed")
        }
    }
}

private struct ExecutionResult {
    let columns: [String]
    let rows: [[SqlValue]]
    let rowCount: Int
    let lastRowId: SqlValue?

    static func empty(rowCount: Int) -> ExecutionResult {
        ExecutionResult(columns: [], rows: [], rowCount: rowCount, lastRowId: nil)
    }
}

private struct Table {
    let columns: [String]
    var rows: [[String: SqlValue]]
    var nextRowId: Int

    init(columns: [String]) {
        self.columns = columns
        self.rows = []
        self.nextRowId = 1
    }

    func copy() -> Table {
        Table(columns: columns, rows: rows, nextRowId: nextRowId)
    }

    private init(columns: [String], rows: [[String: SqlValue]], nextRowId: Int) {
        self.columns = columns
        self.rows = rows
        self.nextRowId = nextRowId
    }
}

private struct Database {
    var tables: [String: Table] = [:]

    func copy() -> Database {
        Database(tables: tables)
    }

    mutating func create(_ statement: CreateStatement) throws -> ExecutionResult {
        let key = identifierKey(statement.tableName)
        if tables[key] != nil {
            throw MiniSqliteError("OperationalError", "table already exists: \(statement.tableName)")
        }
        tables[key] = Table(columns: statement.columns)
        return .empty(rowCount: 0)
    }

    mutating func drop(_ tableName: String) throws -> ExecutionResult {
        let removed = tables.removeValue(forKey: identifierKey(tableName))
        if removed == nil {
            throw MiniSqliteError("OperationalError", "no such table: \(tableName)")
        }
        return .empty(rowCount: 0)
    }

    mutating func insert(_ statement: InsertStatement) throws -> ExecutionResult {
        let key = identifierKey(statement.tableName)
        guard var table = tables[key] else {
            throw MiniSqliteError("OperationalError", "no such table: \(statement.tableName)")
        }
        let columns = statement.columns ?? table.columns
        if columns.count != statement.values.count {
            throw MiniSqliteError("ProgrammingError", "column/value count mismatch")
        }

        var row: [String: SqlValue] = [:]
        for column in table.columns {
            row[identifierKey(column)] = .null
        }
        for (column, valueSql) in zip(columns, statement.values) {
            row[identifierKey(column)] = SqlValueParser.parseLiteral(valueSql)
        }

        let rowId = table.nextRowId
        table.nextRowId += 1
        table.rows.append(row)
        tables[key] = table
        return ExecutionResult(columns: [], rows: [], rowCount: 1, lastRowId: .integer(rowId))
    }

    mutating func update(_ statement: UpdateStatement) throws -> ExecutionResult {
        let key = identifierKey(statement.tableName)
        guard var table = tables[key] else {
            throw MiniSqliteError("OperationalError", "no such table: \(statement.tableName)")
        }

        let assignments = statement.assignments.map { ($0.column, SqlValueParser.parseLiteral($0.valueSql)) }
        var count = 0
        for index in table.rows.indices {
            if Conditions.matches(statement.whereSql, row: table.rows[index]) {
                for (column, value) in assignments {
                    table.rows[index][identifierKey(column)] = value
                }
                count += 1
            }
        }
        tables[key] = table
        return .empty(rowCount: count)
    }

    mutating func delete(_ statement: DeleteStatement) throws -> ExecutionResult {
        let key = identifierKey(statement.tableName)
        guard var table = tables[key] else {
            throw MiniSqliteError("OperationalError", "no such table: \(statement.tableName)")
        }
        let before = table.rows.count
        table.rows.removeAll { Conditions.matches(statement.whereSql, row: $0) }
        tables[key] = table
        return .empty(rowCount: before - table.rows.count)
    }

    func select(_ statement: SelectStatement) throws -> ExecutionResult {
        guard let table = tables[identifierKey(statement.tableName)] else {
            throw MiniSqliteError("OperationalError", "no such table: \(statement.tableName)")
        }
        let projection = statement.projection == ["*"] ? table.columns : statement.projection
        var matching = table.rows.filter { Conditions.matches(statement.whereSql, row: $0) }
        applyOrder(statement.orderBySql, to: &matching)
        return ExecutionResult(
            columns: projection,
            rows: matching.map { row in projection.map { row[identifierKey($0)] ?? .null } },
            rowCount: -1,
            lastRowId: nil
        )
    }
}

private enum SqlText {
    static func trimSql(_ sql: String) -> String {
        let trimmed = sql.trimmingCharacters(in: .whitespacesAndNewlines)
        if trimmed.hasSuffix(";") {
            return String(trimmed.dropLast()).trimmingCharacters(in: .whitespacesAndNewlines)
        }
        return trimmed
    }

    static func firstKeyword(_ sql: String) -> String {
        let trimmed = sql.trimmingCharacters(in: .whitespacesAndNewlines)
        return String(trimmed.prefix { $0.isLetter }).uppercased()
    }

    static func bindParameters(_ sql: String, _ parameters: [SqlValue]) throws -> String {
        var output = ""
        var parameterIndex = 0
        var quote: Character?
        let chars = Array(sql)
        var index = 0

        while index < chars.count {
            let ch = chars[index]
            if let quoteChar = quote {
                output.append(ch)
                if ch == quoteChar {
                    if index + 1 < chars.count && chars[index + 1] == quoteChar {
                        index += 1
                        output.append(chars[index])
                    } else {
                        quote = nil
                    }
                }
            } else if ch == "'" || ch == "\"" {
                quote = ch
                output.append(ch)
            } else if ch == "?" {
                guard parameterIndex < parameters.count else {
                    throw MiniSqliteError("ProgrammingError", "not enough query parameters")
                }
                output += SqlValueParser.formatParameter(parameters[parameterIndex])
                parameterIndex += 1
            } else {
                output.append(ch)
            }
            index += 1
        }

        if parameterIndex != parameters.count {
            throw MiniSqliteError("ProgrammingError", "too many query parameters")
        }
        return output
    }

    static func splitTopLevel(_ text: String, separator: Character) -> [String] {
        var parts: [String] = []
        var current = ""
        var depth = 0
        var quote: Character?
        let chars = Array(text)
        var index = 0

        while index < chars.count {
            let ch = chars[index]
            if let quoteChar = quote {
                current.append(ch)
                if ch == quoteChar {
                    if index + 1 < chars.count && chars[index + 1] == quoteChar {
                        index += 1
                        current.append(chars[index])
                    } else {
                        quote = nil
                    }
                }
            } else if ch == "'" || ch == "\"" {
                quote = ch
                current.append(ch)
            } else if ch == "(" {
                depth += 1
                current.append(ch)
            } else if ch == ")" {
                depth = max(0, depth - 1)
                current.append(ch)
            } else if ch == separator && depth == 0 {
                parts.append(current.trimmingCharacters(in: .whitespacesAndNewlines))
                current = ""
            } else {
                current.append(ch)
            }
            index += 1
        }

        parts.append(current.trimmingCharacters(in: .whitespacesAndNewlines))
        return parts.filter { !$0.isEmpty }
    }
}

private enum SqlValueParser {
    static func formatParameter(_ value: SqlValue) -> String {
        switch value {
        case .null:
            return "NULL"
        case .integer(let value):
            return String(value)
        case .real(let value):
            return String(value)
        case .bool(let value):
            return value ? "TRUE" : "FALSE"
        case .text(let value):
            return "'" + value.replacingOccurrences(of: "'", with: "''") + "'"
        }
    }

    static func parseLiteral(_ token: String) -> SqlValue {
        let text = token.trimmingCharacters(in: .whitespacesAndNewlines)
        if isQuoted(text, "'") || isQuoted(text, "\"") {
            let quote = String(text.first!)
            let inner = String(text.dropFirst().dropLast())
            return .text(inner.replacingOccurrences(of: quote + quote, with: quote))
        }
        switch text.uppercased() {
        case "NULL": return .null
        case "TRUE": return .bool(true)
        case "FALSE": return .bool(false)
        default:
            if let integer = Int(text) {
                return .integer(integer)
            }
            if let real = Double(text) {
                return .real(real)
            }
            return .text(text)
        }
    }

    static func resolve(_ row: [String: SqlValue], _ token: String) -> SqlValue {
        let text = token.trimmingCharacters(in: .whitespacesAndNewlines)
        return row[identifierKey(text)] ?? parseLiteral(text)
    }

    static func valuesEqual(_ left: SqlValue, _ right: SqlValue) -> Bool {
        switch (left, right) {
        case (.null, .null):
            return true
        case (.integer(let left), .integer(let right)):
            return left == right
        case (.integer(let left), .real(let right)):
            return Double(left) == right
        case (.real(let left), .integer(let right)):
            return left == Double(right)
        default:
            return left == right
        }
    }

    static func compare(_ left: SqlValue, _ right: SqlValue) -> ComparisonResult {
        if valuesEqual(left, right) { return .orderedSame }
        switch (left, right) {
        case (.null, _): return .orderedAscending
        case (_, .null): return .orderedDescending
        case (.integer(let left), .integer(let right)):
            return left < right ? .orderedAscending : .orderedDescending
        case (.integer(let left), .real(let right)):
            return Double(left) < right ? .orderedAscending : .orderedDescending
        case (.real(let left), .integer(let right)):
            return left < Double(right) ? .orderedAscending : .orderedDescending
        case (.real(let left), .real(let right)):
            return left < right ? .orderedAscending : .orderedDescending
        default:
            return String(describing: left).compare(String(describing: right))
        }
    }
}

private enum Conditions {
    static func matches(_ whereSql: String?, row: [String: SqlValue]) -> Bool {
        guard let whereSql, !whereSql.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else {
            return true
        }
        return splitByKeyword(whereSql, "OR").contains { disjunct in
            splitByKeyword(disjunct, "AND").allSatisfy { matchesAtom($0, row: row) }
        }
    }

    private static func matchesAtom(_ atom: String, row: [String: SqlValue]) -> Bool {
        let text = atom.trimmingCharacters(in: .whitespacesAndNewlines)
        if let (leftSql, negate) = parseIsNull(text) {
            let value = SqlValueParser.resolve(row, leftSql)
            return negate ? value != .null : value == .null
        }
        if let (leftSql, valueSqls) = parseIn(text) {
            let left = SqlValueParser.resolve(row, leftSql)
            return valueSqls.contains { SqlValueParser.valuesEqual(left, SqlValueParser.resolve(row, $0)) }
        }
        if let (leftSql, op, rightSql) = parseComparison(text) {
            let left = SqlValueParser.resolve(row, leftSql)
            let right = SqlValueParser.resolve(row, rightSql)
            switch op {
            case "=": return SqlValueParser.valuesEqual(left, right)
            case "!=", "<>": return !SqlValueParser.valuesEqual(left, right)
            case "<": return SqlValueParser.compare(left, right) == .orderedAscending
            case "<=": return SqlValueParser.compare(left, right) != .orderedDescending
            case ">": return SqlValueParser.compare(left, right) == .orderedDescending
            case ">=": return SqlValueParser.compare(left, right) != .orderedAscending
            default: return false
            }
        }
        switch SqlValueParser.resolve(row, text) {
        case .bool(let value): return value
        case .null: return false
        default: return true
        }
    }

    private static func parseIsNull(_ text: String) -> (String, Bool)? {
        guard let range = rangeOfKeyword("IS", in: text) else { return nil }
        let left = String(text[..<range.lowerBound])
        let rest = String(text[range.upperBound...]).trimmingCharacters(in: .whitespacesAndNewlines).uppercased()
        if rest == "NULL" { return (left, false) }
        if rest == "NOT NULL" { return (left, true) }
        return nil
    }

    private static func parseIn(_ text: String) -> (String, [String])? {
        guard let range = rangeOfKeyword("IN", in: text) else { return nil }
        let left = String(text[..<range.lowerBound])
        let rest = String(text[range.upperBound...]).trimmingCharacters(in: .whitespacesAndNewlines)
        guard let inside = parenthesized(rest) else { return nil }
        return (left, SqlText.splitTopLevel(inside, separator: ","))
    }

    private static func parseComparison(_ text: String) -> (String, String, String)? {
        for op in ["!=", "<>", "<=", ">=", "=", "<", ">"] {
            if let range = text.range(of: op) {
                let left = String(text[..<range.lowerBound])
                let right = String(text[range.upperBound...])
                return (left, op, right)
            }
        }
        return nil
    }
}

private enum Statements {
    static func parseCreate(_ sql: String) throws -> CreateStatement {
        let rest = try stripKeyword("CREATE TABLE", from: SqlText.trimSql(sql))
        let (name, afterName) = takeIdentifier(rest)
        guard let inside = parenthesized(afterName) else {
            throw MiniSqliteError("OperationalError", "could not parse CREATE TABLE")
        }
        let columns = SqlText.splitTopLevel(inside, separator: ",").map(identifierFromColumn)
        guard !name.isEmpty && !columns.isEmpty else {
            throw MiniSqliteError("OperationalError", "could not parse CREATE TABLE")
        }
        return CreateStatement(tableName: name, columns: columns)
    }

    static func parseDrop(_ sql: String) throws -> String {
        let rest = try stripKeyword("DROP TABLE", from: SqlText.trimSql(sql))
        let (name, remainder) = takeIdentifier(rest)
        guard !name.isEmpty && remainder.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else {
            throw MiniSqliteError("OperationalError", "could not parse DROP TABLE")
        }
        return name
    }

    static func parseInsert(_ sql: String) throws -> InsertStatement {
        let rest = try stripKeyword("INSERT INTO", from: SqlText.trimSql(sql))
        let (name, afterName) = takeIdentifier(rest)
        var suffix = afterName.trimmingCharacters(in: .whitespacesAndNewlines)
        var columns: [String]?
        if suffix.hasPrefix("("), let (inside, remaining) = takeParenthesized(suffix) {
            columns = SqlText.splitTopLevel(inside, separator: ",").map(identifierFromColumn)
            suffix = remaining.trimmingCharacters(in: .whitespacesAndNewlines)
        }
        let valuesRest = try stripKeyword("VALUES", from: suffix)
        guard let inside = parenthesized(valuesRest), !name.isEmpty else {
            throw MiniSqliteError("OperationalError", "could not parse INSERT")
        }
        return InsertStatement(tableName: name, columns: columns, values: SqlText.splitTopLevel(inside, separator: ","))
    }

    static func parseUpdate(_ sql: String) throws -> UpdateStatement {
        let rest = try stripKeyword("UPDATE", from: SqlText.trimSql(sql))
        let (name, afterName) = takeIdentifier(rest)
        let setRest = try stripKeyword("SET", from: afterName)
        let (assignmentSql, whereSql) = splitOptionalKeyword("WHERE", in: setRest)
        let assignments = try SqlText.splitTopLevel(assignmentSql, separator: ",").map(parseAssignment)
        guard !name.isEmpty && !assignments.isEmpty else {
            throw MiniSqliteError("OperationalError", "could not parse UPDATE")
        }
        return UpdateStatement(tableName: name, assignments: assignments, whereSql: whereSql)
    }

    static func parseDelete(_ sql: String) throws -> DeleteStatement {
        let rest = try stripKeyword("DELETE FROM", from: SqlText.trimSql(sql))
        let (name, afterName) = takeIdentifier(rest)
        let (_, whereSql) = splitOptionalKeyword("WHERE", in: afterName)
        guard !name.isEmpty else {
            throw MiniSqliteError("OperationalError", "could not parse DELETE")
        }
        return DeleteStatement(tableName: name, whereSql: whereSql)
    }

    static func parseSelect(_ sql: String) throws -> SelectStatement {
        let rest = try stripKeyword("SELECT", from: SqlText.trimSql(sql))
        guard let fromRange = rangeOfKeyword("FROM", in: rest) else {
            throw MiniSqliteError("OperationalError", "could not parse SELECT")
        }
        let projectionSql = String(rest[..<fromRange.lowerBound])
        let fromRest = String(rest[fromRange.upperBound...])
        let (name, suffix) = takeIdentifier(fromRest)
        let (beforeOrder, orderSql) = splitOptionalKeyword("ORDER BY", in: suffix)
        let (_, whereSql) = splitOptionalKeyword("WHERE", in: beforeOrder)
        let projection = SqlText.splitTopLevel(projectionSql, separator: ",").map(identifierFromColumn)
        guard !name.isEmpty && !projection.isEmpty else {
            throw MiniSqliteError("OperationalError", "could not parse SELECT")
        }
        return SelectStatement(tableName: name, projection: projection, whereSql: whereSql, orderBySql: orderSql)
    }

    private static func parseAssignment(_ text: String) throws -> Assignment {
        guard let range = text.range(of: "=") else {
            throw MiniSqliteError("OperationalError", "invalid assignment")
        }
        return Assignment(
            column: identifierFromColumn(String(text[..<range.lowerBound])),
            valueSql: String(text[range.upperBound...]).trimmingCharacters(in: .whitespacesAndNewlines)
        )
    }
}

private struct CreateStatement {
    let tableName: String
    let columns: [String]
}

private struct InsertStatement {
    let tableName: String
    let columns: [String]?
    let values: [String]
}

private struct Assignment {
    let column: String
    let valueSql: String
}

private struct UpdateStatement {
    let tableName: String
    let assignments: [Assignment]
    let whereSql: String?
}

private struct DeleteStatement {
    let tableName: String
    let whereSql: String?
}

private struct SelectStatement {
    let tableName: String
    let projection: [String]
    let whereSql: String?
    let orderBySql: String?
}

private func applyOrder(_ orderBySql: String?, to rows: inout [[String: SqlValue]]) {
    guard let orderBySql, !orderBySql.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else {
        return
    }
    let parts = orderBySql.split(whereSeparator: { $0.isWhitespace }).map(String.init)
    guard let column = parts.first else { return }
    let descending = parts.count > 1 && parts[1].uppercased() == "DESC"
    rows.sort {
        SqlValueParser.compare(SqlValueParser.resolve($0, column), SqlValueParser.resolve($1, column)) == .orderedAscending
    }
    if descending {
        rows.reverse()
    }
}

private func splitByKeyword(_ text: String, _ keyword: String) -> [String] {
    var parts: [String] = []
    var remaining = text
    while let range = rangeOfKeyword(keyword, in: remaining) {
        parts.append(String(remaining[..<range.lowerBound]).trimmingCharacters(in: .whitespacesAndNewlines))
        remaining = String(remaining[range.upperBound...])
    }
    parts.append(remaining.trimmingCharacters(in: .whitespacesAndNewlines))
    return parts.filter { !$0.isEmpty }
}

private func splitOptionalKeyword(_ keyword: String, in text: String) -> (String, String?) {
    guard let range = rangeOfKeyword(keyword, in: text) else {
        return (text.trimmingCharacters(in: .whitespacesAndNewlines), nil)
    }
    return (
        String(text[..<range.lowerBound]).trimmingCharacters(in: .whitespacesAndNewlines),
        String(text[range.upperBound...]).trimmingCharacters(in: .whitespacesAndNewlines)
    )
}

private func rangeOfKeyword(_ keyword: String, in text: String) -> Range<String.Index>? {
    var index = text.startIndex
    while index < text.endIndex {
        let suffix = text[index...]
        if suffix.uppercased().hasPrefix(keyword.uppercased()) {
            let end = text.index(index, offsetBy: keyword.count)
            let before = index == text.startIndex ? " " : String(text[text.index(before: index)])
            let after = end == text.endIndex ? " " : String(text[end])
            if !isIdentifierChar(before) && !isIdentifierChar(after) {
                return index..<end
            }
        }
        index = text.index(after: index)
    }
    return nil
}

private func stripKeyword(_ keyword: String, from text: String) throws -> String {
    let trimmed = text.trimmingCharacters(in: .whitespacesAndNewlines)
    guard trimmed.uppercased().hasPrefix(keyword.uppercased()) else {
        throw MiniSqliteError("OperationalError", "expected \(keyword)")
    }
    return String(trimmed.dropFirst(keyword.count)).trimmingCharacters(in: .whitespacesAndNewlines)
}

private func takeIdentifier(_ text: String) -> (String, String) {
    let trimmed = text.trimmingCharacters(in: .whitespacesAndNewlines)
    let name = String(trimmed.prefix { $0.isLetter || $0.isNumber || $0 == "_" })
    let rest = String(trimmed.dropFirst(name.count))
    return (name, rest)
}

private func parenthesized(_ text: String) -> String? {
    guard let (inside, remainder) = takeParenthesized(text), remainder.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else {
        return nil
    }
    return inside
}

private func takeParenthesized(_ text: String) -> (String, String)? {
    let trimmed = text.trimmingCharacters(in: .whitespacesAndNewlines)
    guard trimmed.first == "(" else { return nil }
    var depth = 0
    var quote: Character?
    var inside = ""
    let chars = Array(trimmed)
    var index = 0

    while index < chars.count {
        let ch = chars[index]
        if let quoteChar = quote {
            inside.append(ch)
            if ch == quoteChar {
                if index + 1 < chars.count && chars[index + 1] == quoteChar {
                    index += 1
                    inside.append(chars[index])
                } else {
                    quote = nil
                }
            }
        } else if ch == "'" || ch == "\"" {
            quote = ch
            inside.append(ch)
        } else if ch == "(" {
            if depth > 0 { inside.append(ch) }
            depth += 1
        } else if ch == ")" {
            depth -= 1
            if depth == 0 {
                return (inside, String(chars[(index + 1)...]))
            }
            inside.append(ch)
        } else {
            inside.append(ch)
        }
        index += 1
    }
    return nil
}

private func identifierFromColumn(_ text: String) -> String {
    let trimmed = text.trimmingCharacters(in: .whitespacesAndNewlines)
    return String(trimmed.prefix { !$0.isWhitespace }).trimmingCharacters(in: CharacterSet(charactersIn: "\"'"))
}

private func identifierKey(_ name: String) -> String {
    name.trimmingCharacters(in: CharacterSet(charactersIn: "\"'").union(.whitespacesAndNewlines)).lowercased()
}

private func isQuoted(_ text: String, _ quote: Character) -> Bool {
    text.count >= 2 && text.first == quote && text.last == quote
}

private func isIdentifierChar(_ text: String) -> Bool {
    guard let ch = text.first else { return false }
    return ch.isLetter || ch.isNumber || ch == "_"
}
