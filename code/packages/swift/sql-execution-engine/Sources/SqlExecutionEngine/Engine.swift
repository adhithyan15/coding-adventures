private struct RowContext {
  var values: [String: SqlValue]
  var ambiguous: Set<String>

  func value(table: String?, name: String) throws -> SqlValue {
    if let table {
      let key = "\(table).\(name)"
      guard let value = values[key] else { throw SqlExecutionError.columnNotFound(key) }
      return value
    }
    if ambiguous.contains(name) { throw SqlExecutionError.ambiguousColumn(name) }
    guard let value = values[name] else { throw SqlExecutionError.columnNotFound(name) }
    return value
  }

  func merged(with other: RowContext) -> RowContext {
    var merged = self
    for (key, value) in other.values where key.contains(".") {
      merged.values[key] = value
    }
    let bareNames = Set(values.keys.filter { !$0.contains(".") })
      .union(other.values.keys.filter { !$0.contains(".") })
      .union(ambiguous)
      .union(other.ambiguous)
    for name in bareNames {
      let leftHas = values[name] != nil || ambiguous.contains(name)
      let rightHas = other.values[name] != nil || other.ambiguous.contains(name)
      if leftHas, rightHas {
        merged.values.removeValue(forKey: name)
        merged.ambiguous.insert(name)
      } else if let value = other.values[name] {
        merged.values[name] = value
      }
    }
    return merged
  }
}

private struct TableRows {
  let rows: [RowContext]
  let nullRow: RowContext
}

private struct RowFrame {
  let row: RowContext
  let groupRows: [RowContext]?
}

/// Entry points for executing SELECT statements.
public enum SqlEngine {
  public static func execute(_ sql: String, dataSource: SqlDataSource) throws -> QueryResult {
    var parser = try SelectParser(sql)
    return try execute(parser.parse(), dataSource: dataSource)
  }

  public static func tryExecute(_ sql: String, dataSource: SqlDataSource) -> ExecutionResult {
    do {
      return ExecutionResult(result: try execute(sql, dataSource: dataSource), error: nil)
    } catch let error as SqlExecutionError {
      return ExecutionResult(result: nil, error: error.description)
    } catch {
      return ExecutionResult(result: nil, error: String(describing: error))
    }
  }

  private static func execute(
    _ statement: SelectStatement,
    dataSource: SqlDataSource
  ) throws -> QueryResult {
    var current = try scan(statement.from, from: dataSource)
    var rows = current.rows

    for join in statement.joins {
      let right = try scan(join.table, from: dataSource)
      rows = try applyJoin(
        leftRows: rows,
        leftNullRow: current.nullRow,
        rightRows: right,
        clause: join
      )
      current = TableRows(
        rows: rows,
        nullRow: current.nullRow.merged(with: right.nullRow)
      )
    }

    if let whereExpression = statement.whereExpression {
      rows = try rows.filter {
        try truth(evaluate(whereExpression, row: $0, groupRows: nil)) == true
      }
    }

    var frames = try makeFrames(rows, statement: statement)
    if let having = statement.having {
      frames = try frames.filter {
        try truth(evaluate(having, row: $0.row, groupRows: $0.groupRows)) == true
      }
    }
    if !statement.orderBy.isEmpty {
      try stableSort(&frames, orderBy: statement.orderBy)
    }

    let projection = try project(frames, statement: statement)
    var resultRows = projection.rows
    if statement.distinct {
      var seen: Set<[SqlValue]> = []
      resultRows = resultRows.filter { seen.insert($0).inserted }
    }

    let offset = max(0, statement.offset ?? 0)
    guard offset < resultRows.count else {
      return QueryResult(columns: projection.columns, rows: [])
    }
    let end: Int
    if let limit = statement.limit {
      end = min(resultRows.count, offset + max(0, limit))
    } else {
      end = resultRows.count
    }
    return QueryResult(
      columns: projection.columns,
      rows: Array(resultRows[offset..<end])
    )
  }

  private static func scan(
    _ table: TableReference,
    from dataSource: SqlDataSource
  ) throws -> TableRows {
    let schema = try dataSource.schema(table.name)
    let rows = try dataSource.scan(table.name).map { raw -> RowContext in
      var values: [String: SqlValue] = [:]
      for column in schema {
        let value = raw[column] ?? .null
        values[column] = value
        values["\(table.name).\(column)"] = value
        values["\(table.alias).\(column)"] = value
      }
      return RowContext(values: values, ambiguous: [])
    }
    var nullValues: [String: SqlValue] = [:]
    for column in schema {
      nullValues[column] = .null
      nullValues["\(table.name).\(column)"] = .null
      nullValues["\(table.alias).\(column)"] = .null
    }
    return TableRows(
      rows: rows,
      nullRow: RowContext(values: nullValues, ambiguous: [])
    )
  }

  private static func applyJoin(
    leftRows: [RowContext],
    leftNullRow: RowContext,
    rightRows: TableRows,
    clause: JoinClause
  ) throws -> [RowContext] {
    if clause.type == .cross {
      return leftRows.flatMap { left in rightRows.rows.map { left.merged(with: $0) } }
    }

    var result: [RowContext] = []
    var matchedRight: Set<Int> = []
    for left in leftRows {
      var matched = false
      for (index, right) in rightRows.rows.enumerated() {
        let merged = left.merged(with: right)
        let matches =
          if let condition = clause.condition {
            try truth(evaluate(condition, row: merged, groupRows: nil)) == true
          } else {
            true
          }
        if matches {
          result.append(merged)
          matched = true
          matchedRight.insert(index)
        }
      }
      if !matched, clause.type == .left || clause.type == .full {
        result.append(left.merged(with: rightRows.nullRow))
      }
    }
    if clause.type == .right || clause.type == .full {
      for (index, right) in rightRows.rows.enumerated() where !matchedRight.contains(index) {
        result.append(leftNullRow.merged(with: right))
      }
    }
    return result
  }

  private static func makeFrames(
    _ rows: [RowContext],
    statement: SelectStatement
  ) throws -> [RowFrame] {
    let grouped = !statement.groupBy.isEmpty
    let aggregated =
      statement.selectItems.contains { hasAggregate($0.expression) }
      || statement.having.map(hasAggregate) == true
    if !grouped, !aggregated {
      return rows.map { RowFrame(row: $0, groupRows: nil) }
    }
    if !grouped {
      return [
        RowFrame(
          row: rows.first ?? RowContext(values: [:], ambiguous: []),
          groupRows: rows
        )
      ]
    }

    var groups: [[SqlValue]: [RowContext]] = [:]
    var order: [[SqlValue]] = []
    for row in rows {
      let key = try statement.groupBy.map { try evaluate($0, row: row, groupRows: nil) }
      if groups[key] == nil { order.append(key) }
      groups[key, default: []].append(row)
    }
    return order.compactMap { key in
      guard let group = groups[key], let first = group.first else { return nil }
      return RowFrame(row: first, groupRows: group)
    }
  }

  private static func project(
    _ frames: [RowFrame],
    statement: SelectStatement
  ) throws -> (columns: [String], rows: [[SqlValue]]) {
    if statement.selectItems.count == 1, case .star = statement.selectItems[0].expression {
      let columns =
        frames.first?.row.values.keys
        .filter { !$0.contains(".") }
        .sorted() ?? []
      let rows = frames.map { frame in columns.map { frame.row.values[$0] ?? .null } }
      return (columns, rows)
    }
    let columns = statement.selectItems.map { item in
      item.alias ?? expressionLabel(item.expression)
    }
    let rows = try frames.map { frame in
      try statement.selectItems.map {
        try evaluate($0.expression, row: frame.row, groupRows: frame.groupRows)
      }
    }
    return (columns, rows)
  }

  private static func evaluate(
    _ expression: Expression,
    row: RowContext,
    groupRows: [RowContext]?
  ) throws -> SqlValue {
    switch expression {
    case .literal(let value):
      return value
    case .column(let table, let name):
      return try row.value(table: table, name: name)
    case .star:
      throw SqlExecutionError.typeMismatch("STAR is only valid in projection or COUNT")
    case .unary(let operation, let operand):
      let value = try evaluate(operand, row: row, groupRows: groupRows)
      if operation == "NOT" {
        return truth(value).map { .boolean(!$0) } ?? .null
      }
      guard operation == "-" else {
        throw SqlExecutionError.typeMismatch("unknown unary \(operation)")
      }
      guard value != .null else { return .null }
      return .real(-(try number(value)))
    case .binary(let operation, let leftExpression, let rightExpression):
      return try evaluateBinary(
        operation,
        left: leftExpression,
        right: rightExpression,
        row: row,
        groupRows: groupRows
      )
    case .isNull(let operand, let negated):
      let isNull = try evaluate(operand, row: row, groupRows: groupRows) == .null
      return .boolean(negated ? !isNull : isNull)
    case .between(let operand, let lowerExpression, let upperExpression, let negated):
      let value = try evaluate(operand, row: row, groupRows: groupRows)
      let lower = try evaluate(lowerExpression, row: row, groupRows: groupRows)
      let upper = try evaluate(upperExpression, row: row, groupRows: groupRows)
      guard value != .null, lower != .null, upper != .null else { return .null }
      let match = try compare(value, lower) >= 0 && compare(value, upper) <= 0
      return .boolean(negated ? !match : match)
    case .inList(let operand, let expressions, let negated):
      let value = try evaluate(operand, row: row, groupRows: groupRows)
      guard value != .null else { return .null }
      var found = false
      for candidate in expressions {
        let candidateValue = try evaluate(candidate, row: row, groupRows: groupRows)
        if candidateValue != .null, try sqlEquals(value, candidateValue) {
          found = true
          break
        }
      }
      return .boolean(negated ? !found : found)
    case .like(let operand, let patternExpression, let negated):
      let value = try evaluate(operand, row: row, groupRows: groupRows)
      let pattern = try evaluate(patternExpression, row: row, groupRows: groupRows)
      guard value != .null, pattern != .null else { return .null }
      guard case .text(let text) = value, case .text(let patternText) = pattern else {
        throw SqlExecutionError.typeMismatch("LIKE requires text operands")
      }
      let match = like(text, pattern: patternText)
      return .boolean(negated ? !match : match)
    case .function(let name, let arguments):
      return try evaluateFunction(name, arguments: arguments, row: row, groupRows: groupRows)
    }
  }

  private static func evaluateBinary(
    _ operation: String,
    left leftExpression: Expression,
    right rightExpression: Expression,
    row: RowContext,
    groupRows: [RowContext]?
  ) throws -> SqlValue {
    let left = try evaluate(leftExpression, row: row, groupRows: groupRows)
    if operation == "AND" {
      if truth(left) == false { return .boolean(false) }
      let right = try evaluate(rightExpression, row: row, groupRows: groupRows)
      if truth(right) == false { return .boolean(false) }
      return truth(left) == nil || truth(right) == nil ? .null : .boolean(true)
    }
    if operation == "OR" {
      if truth(left) == true { return .boolean(true) }
      let right = try evaluate(rightExpression, row: row, groupRows: groupRows)
      if truth(right) == true { return .boolean(true) }
      return truth(left) == nil || truth(right) == nil ? .null : .boolean(false)
    }

    let right = try evaluate(rightExpression, row: row, groupRows: groupRows)
    guard left != .null, right != .null else { return .null }
    switch operation {
    case "+": return .real(try number(left) + number(right))
    case "-": return .real(try number(left) - number(right))
    case "*": return .real(try number(left) * number(right))
    case "/":
      let divisor = try number(right)
      guard divisor != 0 else { throw SqlExecutionError.divisionByZero }
      return .real(try number(left) / divisor)
    case "%":
      let divisor = try number(right)
      guard divisor != 0 else { throw SqlExecutionError.divisionByZero }
      return .real(try number(left).truncatingRemainder(dividingBy: divisor))
    case "=": return .boolean(try sqlEquals(left, right))
    case "!=", "<>": return .boolean(try !sqlEquals(left, right))
    case "<": return .boolean(try compare(left, right) < 0)
    case ">": return .boolean(try compare(left, right) > 0)
    case "<=": return .boolean(try compare(left, right) <= 0)
    case ">=": return .boolean(try compare(left, right) >= 0)
    default: throw SqlExecutionError.typeMismatch("unknown operator \(operation)")
    }
  }

  private static func evaluateFunction(
    _ rawName: String,
    arguments: [Expression],
    row: RowContext,
    groupRows: [RowContext]?
  ) throws -> SqlValue {
    let name = rawName.uppercased()
    if ["COUNT", "SUM", "AVG", "MIN", "MAX"].contains(name) {
      guard let groupRows else {
        throw SqlExecutionError.typeMismatch("aggregate \(name) used outside grouped context")
      }
      if name == "COUNT", arguments.count == 1, case .star = arguments[0] {
        return .integer(groupRows.count)
      }
      guard arguments.count == 1 else {
        throw SqlExecutionError.typeMismatch("\(name) expects one argument")
      }
      let values = try groupRows.map {
        try evaluate(arguments[0], row: $0, groupRows: nil)
      }.filter { $0 != .null }
      if name == "COUNT" { return .integer(values.count) }
      guard let first = values.first else { return .null }
      switch name {
      case "SUM": return .real(try values.reduce(0) { try $0 + number($1) })
      case "AVG": return .real(try values.reduce(0) { try $0 + number($1) } / Double(values.count))
      case "MIN":
        return try values.dropFirst().reduce(first) { try compare($1, $0) < 0 ? $1 : $0 }
      case "MAX":
        return try values.dropFirst().reduce(first) { try compare($1, $0) > 0 ? $1 : $0 }
      default: fatalError("unreachable aggregate")
      }
    }

    guard arguments.count == 1 else {
      throw SqlExecutionError.typeMismatch("\(name) expects one argument")
    }
    let value = try evaluate(arguments[0], row: row, groupRows: groupRows)
    guard value != .null else { return .null }
    guard case .text(let text) = value else {
      throw SqlExecutionError.typeMismatch("\(name) expects text")
    }
    switch name {
    case "UPPER": return .text(text.uppercased())
    case "LOWER": return .text(text.lowercased())
    case "LENGTH": return .integer(text.count)
    default: throw SqlExecutionError.unknownFunction(name)
    }
  }

  private static func truth(_ value: SqlValue) -> Bool? {
    switch value {
    case .null: nil
    case .boolean(let value): value
    default: nil
    }
  }

  private static func number(_ value: SqlValue) throws -> Double {
    switch value {
    case .integer(let value): Double(value)
    case .real(let value): value
    default: throw SqlExecutionError.typeMismatch("expected number, got \(value)")
    }
  }

  private static func sqlEquals(_ left: SqlValue, _ right: SqlValue) throws -> Bool {
    if isNumber(left), isNumber(right) { return try number(left) == number(right) }
    switch (left, right) {
    case (.text(let lhs), .text(let rhs)): return lhs == rhs
    case (.boolean(let lhs), .boolean(let rhs)): return lhs == rhs
    default: throw SqlExecutionError.typeMismatch("cannot compare \(left) and \(right)")
    }
  }

  private static func compare(_ left: SqlValue, _ right: SqlValue) throws -> Int {
    if left == .null { return right == .null ? 0 : 1 }
    if right == .null { return -1 }
    if isNumber(left), isNumber(right) {
      let lhs = try number(left)
      let rhs = try number(right)
      return lhs == rhs ? 0 : (lhs < rhs ? -1 : 1)
    }
    switch (left, right) {
    case (.text(let lhs), .text(let rhs)): return lhs == rhs ? 0 : (lhs < rhs ? -1 : 1)
    case (.boolean(let lhs), .boolean(let rhs)): return lhs == rhs ? 0 : (lhs ? 1 : -1)
    default: throw SqlExecutionError.typeMismatch("cannot order \(left) and \(right)")
    }
  }

  private static func isNumber(_ value: SqlValue) -> Bool {
    if case .integer = value { return true }
    if case .real = value { return true }
    return false
  }

  private static func like(_ text: String, pattern: String) -> Bool {
    let input = Array(text)
    let wildcard = Array(pattern)
    var table = Array(
      repeating: Array(repeating: false, count: wildcard.count + 1),
      count: input.count + 1
    )
    table[0][0] = true
    if !wildcard.isEmpty {
      for j in 1...wildcard.count where wildcard[j - 1] == "%" {
        table[0][j] = table[0][j - 1]
      }
    }
    guard !input.isEmpty, !wildcard.isEmpty else { return table[input.count][wildcard.count] }
    for i in 1...input.count {
      for j in 1...wildcard.count {
        if wildcard[j - 1] == "%" {
          table[i][j] = table[i][j - 1] || table[i - 1][j]
        } else if wildcard[j - 1] == "_" || wildcard[j - 1] == input[i - 1] {
          table[i][j] = table[i - 1][j - 1]
        }
      }
    }
    return table[input.count][wildcard.count]
  }

  private static func hasAggregate(_ expression: Expression) -> Bool {
    switch expression {
    case .function(let name, let arguments):
      ["COUNT", "SUM", "AVG", "MIN", "MAX"].contains(name.uppercased())
        || arguments.contains(where: hasAggregate)
    case .unary(_, let operand), .isNull(let operand, _):
      hasAggregate(operand)
    case .binary(_, let left, let right):
      hasAggregate(left) || hasAggregate(right)
    case .between(let operand, let lower, let upper, _):
      hasAggregate(operand) || hasAggregate(lower) || hasAggregate(upper)
    case .inList(let operand, let values, _):
      hasAggregate(operand) || values.contains(where: hasAggregate)
    case .like(let operand, let pattern, _):
      hasAggregate(operand) || hasAggregate(pattern)
    default:
      false
    }
  }

  private static func expressionLabel(_ expression: Expression) -> String {
    switch expression {
    case .column(_, let name): name
    case .function(let name, _): name.uppercased()
    case .star: "*"
    default: "expression"
    }
  }

  private static func stableSort(_ frames: inout [RowFrame], orderBy: [OrderItem]) throws {
    guard frames.count > 1 else { return }
    for index in 1..<frames.count {
      var cursor = index
      while cursor > 0, try ordered(frames[cursor], before: frames[cursor - 1], orderBy: orderBy) {
        frames.swapAt(cursor, cursor - 1)
        cursor -= 1
      }
    }
  }

  private static func ordered(
    _ left: RowFrame,
    before right: RowFrame,
    orderBy: [OrderItem]
  ) throws -> Bool {
    for item in orderBy {
      let lhs = try evaluate(item.expression, row: left.row, groupRows: left.groupRows)
      let rhs = try evaluate(item.expression, row: right.row, groupRows: right.groupRows)
      let result = try compare(lhs, rhs)
      if result != 0 { return item.descending ? result > 0 : result < 0 }
    }
    return false
  }
}
