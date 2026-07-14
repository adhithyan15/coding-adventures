/// A value that can participate in SQL expression evaluation.
public enum SqlValue: Hashable, CustomStringConvertible {
  case null
  case integer(Int)
  case real(Double)
  case text(String)
  case boolean(Bool)

  public var description: String {
    switch self {
    case .null: "NULL"
    case .integer(let value): String(value)
    case .real(let value): String(value)
    case .text(let value): value
    case .boolean(let value): value ? "TRUE" : "FALSE"
    }
  }
}

extension SqlValue: ExpressibleByNilLiteral {
  public init(nilLiteral: ()) { self = .null }
}

extension SqlValue: ExpressibleByIntegerLiteral {
  public init(integerLiteral value: Int) { self = .integer(value) }
}

extension SqlValue: ExpressibleByFloatLiteral {
  public init(floatLiteral value: Double) { self = .real(value) }
}

extension SqlValue: ExpressibleByStringLiteral {
  public init(stringLiteral value: String) { self = .text(value) }
}

extension SqlValue: ExpressibleByBooleanLiteral {
  public init(booleanLiteral value: Bool) { self = .boolean(value) }
}

public typealias SqlRow = [String: SqlValue]

/// Storage boundary consumed by the execution engine.
public protocol SqlDataSource {
  func schema(_ tableName: String) throws -> [String]
  func scan(_ tableName: String) throws -> [SqlRow]
}

/// A small data source useful for tests and embedded use.
public final class InMemoryDataSource: SqlDataSource {
  private var schemas: [String: [String]] = [:]
  private var tables: [String: [SqlRow]] = [:]

  public init() {}

  @discardableResult
  public func addTable(_ name: String, schema: [String], rows: [SqlRow]) -> Self {
    schemas[name] = schema
    tables[name] = rows
    return self
  }

  public func schema(_ tableName: String) throws -> [String] {
    guard let schema = schemas[tableName] else {
      throw SqlExecutionError.tableNotFound(tableName)
    }
    return schema
  }

  public func scan(_ tableName: String) throws -> [SqlRow] {
    guard let rows = tables[tableName] else {
      throw SqlExecutionError.tableNotFound(tableName)
    }
    return rows
  }
}

public struct QueryResult: Equatable {
  public let columns: [String]
  public let rows: [[SqlValue]]

  public init(columns: [String], rows: [[SqlValue]]) {
    self.columns = columns
    self.rows = rows
  }
}

public struct ExecutionResult: Equatable {
  public let result: QueryResult?
  public let error: String?

  public var isSuccess: Bool { result != nil }

  public init(result: QueryResult?, error: String?) {
    self.result = result
    self.error = error
  }
}

public enum SqlExecutionError: Error, Equatable, CustomStringConvertible {
  case parse(String)
  case tableNotFound(String)
  case columnNotFound(String)
  case ambiguousColumn(String)
  case typeMismatch(String)
  case divisionByZero
  case unknownFunction(String)

  public var description: String {
    switch self {
    case .parse(let message): "parse error: \(message)"
    case .tableNotFound(let name): "table not found: \(name)"
    case .columnNotFound(let name): "column not found: \(name)"
    case .ambiguousColumn(let name): "ambiguous column: \(name)"
    case .typeMismatch(let message): "type mismatch: \(message)"
    case .divisionByZero: "division by zero"
    case .unknownFunction(let name): "unknown function: \(name)"
    }
  }
}
