private enum TokenKind: Equatable {
  case identifier
  case keyword
  case number
  case string
  case symbol
  case end
}

private struct Token: Equatable {
  let kind: TokenKind
  let value: String
  let offset: Int
}

private let sqlKeywords: Set<String> = [
  "SELECT", "FROM", "WHERE", "GROUP", "BY", "HAVING", "ORDER", "LIMIT", "OFFSET",
  "DISTINCT", "ALL", "JOIN", "INNER", "LEFT", "RIGHT", "FULL", "OUTER", "CROSS",
  "ON", "AS", "AND", "OR", "NOT", "IS", "NULL", "IN", "BETWEEN", "LIKE", "TRUE",
  "FALSE", "ASC", "DESC",
]

private struct Lexer {
  private let characters: [Character]
  private var position = 0

  init(_ source: String) {
    characters = Array(source)
  }

  mutating func tokenize() throws -> [Token] {
    var tokens: [Token] = []
    while position < characters.count {
      try skipTrivia()
      guard position < characters.count else { break }
      let start = position
      let character = characters[position]

      if character.isLetter || character == "_" {
        let value = readIdentifier()
        let upper = value.uppercased()
        tokens.append(
          Token(
            kind: sqlKeywords.contains(upper) ? .keyword : .identifier,
            value: sqlKeywords.contains(upper) ? upper : value,
            offset: start
          ))
      } else if character.isNumber {
        tokens.append(Token(kind: .number, value: readNumber(), offset: start))
      } else if character == "'" {
        tokens.append(Token(kind: .string, value: try readString(), offset: start))
      } else if character == "`" || character == "\"" {
        tokens.append(Token(kind: .identifier, value: try readQuotedIdentifier(), offset: start))
      } else {
        let pair =
          position + 1 < characters.count
          ? String([characters[position], characters[position + 1]])
          : ""
        if ["!=", "<>", "<=", ">="].contains(pair) {
          tokens.append(Token(kind: .symbol, value: pair, offset: start))
          position += 2
        } else if "=<>+-*/%(),.;".contains(character) {
          tokens.append(Token(kind: .symbol, value: String(character), offset: start))
          position += 1
        } else {
          throw SqlExecutionError.parse("unexpected character '\(character)' at offset \(start)")
        }
      }
    }
    tokens.append(Token(kind: .end, value: "", offset: position))
    return tokens
  }

  private mutating func skipTrivia() throws {
    while position < characters.count {
      if characters[position].isWhitespace {
        position += 1
      } else if hasPrefix("--") {
        position += 2
        while position < characters.count, characters[position] != "\n" { position += 1 }
      } else if hasPrefix("/*") {
        position += 2
        while position + 1 < characters.count, !hasPrefix("*/") { position += 1 }
        guard position + 1 < characters.count else {
          throw SqlExecutionError.parse("unterminated block comment")
        }
        position += 2
      } else {
        return
      }
    }
  }

  private func hasPrefix(_ prefix: String) -> Bool {
    let expected = Array(prefix)
    guard position + expected.count <= characters.count else { return false }
    return Array(characters[position..<position + expected.count]) == expected
  }

  private mutating func readIdentifier() -> String {
    let start = position
    while position < characters.count,
      characters[position].isLetter || characters[position].isNumber || characters[position] == "_"
    {
      position += 1
    }
    return String(characters[start..<position])
  }

  private mutating func readNumber() -> String {
    let start = position
    while position < characters.count, characters[position].isNumber { position += 1 }
    if position + 1 < characters.count,
      characters[position] == ".", characters[position + 1].isNumber
    {
      position += 1
      while position < characters.count, characters[position].isNumber { position += 1 }
    }
    return String(characters[start..<position])
  }

  private mutating func readString() throws -> String {
    position += 1
    var value = ""
    while position < characters.count {
      let character = characters[position]
      if character == "'" {
        if position + 1 < characters.count, characters[position + 1] == "'" {
          value.append("'")
          position += 2
        } else {
          position += 1
          return value
        }
      } else if character == "\\", position + 1 < characters.count {
        position += 1
        value.append(characters[position])
        position += 1
      } else {
        value.append(character)
        position += 1
      }
    }
    throw SqlExecutionError.parse("unterminated string literal")
  }

  private mutating func readQuotedIdentifier() throws -> String {
    let quote = characters[position]
    position += 1
    let start = position
    while position < characters.count, characters[position] != quote { position += 1 }
    guard position < characters.count else {
      throw SqlExecutionError.parse("unterminated quoted identifier")
    }
    let value = String(characters[start..<position])
    position += 1
    return value
  }
}

indirect enum Expression {
  case literal(SqlValue)
  case column(table: String?, name: String)
  case star
  case unary(String, Expression)
  case binary(String, Expression, Expression)
  case isNull(Expression, negated: Bool)
  case between(Expression, lower: Expression, upper: Expression, negated: Bool)
  case inList(Expression, values: [Expression], negated: Bool)
  case like(Expression, pattern: Expression, negated: Bool)
  case function(String, [Expression])
}

struct SelectItem {
  let expression: Expression
  let alias: String?
}

struct TableReference {
  let name: String
  let alias: String
}

enum JoinType {
  case inner
  case left
  case right
  case full
  case cross
}

struct JoinClause {
  let type: JoinType
  let table: TableReference
  let condition: Expression?
}

struct OrderItem {
  let expression: Expression
  let descending: Bool
}

struct SelectStatement {
  let distinct: Bool
  let selectItems: [SelectItem]
  let from: TableReference
  let joins: [JoinClause]
  let whereExpression: Expression?
  let groupBy: [Expression]
  let having: Expression?
  let orderBy: [OrderItem]
  let limit: Int?
  let offset: Int?
}

struct SelectParser {
  private let tokens: [Token]
  private var position = 0

  init(_ source: String) throws {
    var lexer = Lexer(source)
    tokens = try lexer.tokenize()
  }

  mutating func parse() throws -> SelectStatement {
    try expectKeyword("SELECT")
    let distinct = matchKeyword("DISTINCT")
    _ = matchKeyword("ALL")
    let selectItems = try parseSelectList()
    try expectKeyword("FROM")
    let from = try parseTableReference()
    let joins = try parseJoins()
    let whereExpression = matchKeyword("WHERE") ? try parseExpression() : nil

    var groupBy: [Expression] = []
    if matchKeyword("GROUP") {
      try expectKeyword("BY")
      groupBy = try parseExpressionList()
    }
    let having = matchKeyword("HAVING") ? try parseExpression() : nil

    var orderBy: [OrderItem] = []
    if matchKeyword("ORDER") {
      try expectKeyword("BY")
      orderBy = try parseOrderList()
    }
    let limit = matchKeyword("LIMIT") ? try expectInteger() : nil
    let offset = matchKeyword("OFFSET") ? try expectInteger() : nil
    _ = matchSymbol(";")
    guard peek().kind == .end else { throw error("unexpected trailing token '\(peek().value)'") }

    return SelectStatement(
      distinct: distinct,
      selectItems: selectItems,
      from: from,
      joins: joins,
      whereExpression: whereExpression,
      groupBy: groupBy,
      having: having,
      orderBy: orderBy,
      limit: limit,
      offset: offset
    )
  }

  private mutating func parseSelectList() throws -> [SelectItem] {
    var result: [SelectItem] = []
    repeat {
      if matchSymbol("*") {
        result.append(SelectItem(expression: .star, alias: nil))
      } else {
        let expression = try parseExpression()
        let alias: String?
        if matchKeyword("AS") {
          alias = try expectIdentifier()
        } else if peek().kind == .identifier {
          alias = advance().value
        } else {
          alias = nil
        }
        result.append(SelectItem(expression: expression, alias: alias))
      }
    } while matchSymbol(",")
    return result
  }

  private mutating func parseTableReference() throws -> TableReference {
    let name = try expectIdentifier()
    let alias: String
    if matchKeyword("AS") {
      alias = try expectIdentifier()
    } else if peek().kind == .identifier {
      alias = advance().value
    } else {
      alias = name
    }
    return TableReference(name: name, alias: alias)
  }

  private mutating func parseJoins() throws -> [JoinClause] {
    var result: [JoinClause] = []
    while true {
      let type: JoinType?
      if matchKeyword("INNER") {
        try expectKeyword("JOIN")
        type = .inner
      } else if matchKeyword("LEFT") {
        _ = matchKeyword("OUTER")
        try expectKeyword("JOIN")
        type = .left
      } else if matchKeyword("RIGHT") {
        _ = matchKeyword("OUTER")
        try expectKeyword("JOIN")
        type = .right
      } else if matchKeyword("FULL") {
        _ = matchKeyword("OUTER")
        try expectKeyword("JOIN")
        type = .full
      } else if matchKeyword("CROSS") {
        try expectKeyword("JOIN")
        type = .cross
      } else if matchKeyword("JOIN") {
        type = .inner
      } else {
        type = nil
      }
      guard let type else { break }
      let table = try parseTableReference()
      let condition: Expression?
      if type == .cross {
        condition = nil
      } else {
        try expectKeyword("ON")
        condition = try parseExpression()
      }
      result.append(JoinClause(type: type, table: table, condition: condition))
    }
    return result
  }

  private mutating func parseExpressionList() throws -> [Expression] {
    var result = [try parseExpression()]
    while matchSymbol(",") { result.append(try parseExpression()) }
    return result
  }

  private mutating func parseOrderList() throws -> [OrderItem] {
    var result: [OrderItem] = []
    repeat {
      let expression = try parseExpression()
      let descending = matchKeyword("DESC")
      if !descending { _ = matchKeyword("ASC") }
      result.append(OrderItem(expression: expression, descending: descending))
    } while matchSymbol(",")
    return result
  }

  private mutating func parseExpression() throws -> Expression { try parseOr() }

  private mutating func parseOr() throws -> Expression {
    var left = try parseAnd()
    while matchKeyword("OR") { left = .binary("OR", left, try parseAnd()) }
    return left
  }

  private mutating func parseAnd() throws -> Expression {
    var left = try parseNot()
    while matchKeyword("AND") { left = .binary("AND", left, try parseNot()) }
    return left
  }

  private mutating func parseNot() throws -> Expression {
    matchKeyword("NOT") ? .unary("NOT", try parseNot()) : try parseComparison()
  }

  private mutating func parseComparison() throws -> Expression {
    let left = try parseAdditive()
    if matchKeyword("IS") {
      let negated = matchKeyword("NOT")
      try expectKeyword("NULL")
      return .isNull(left, negated: negated)
    }
    if matchKeyword("NOT") {
      if matchKeyword("BETWEEN") {
        let lower = try parseAdditive()
        try expectKeyword("AND")
        return .between(left, lower: lower, upper: try parseAdditive(), negated: true)
      }
      if matchKeyword("IN") {
        return .inList(left, values: try parseParenthesizedList(), negated: true)
      }
      if matchKeyword("LIKE") {
        return .like(left, pattern: try parseAdditive(), negated: true)
      }
      throw error("expected BETWEEN, IN, or LIKE after NOT")
    }
    if matchKeyword("BETWEEN") {
      let lower = try parseAdditive()
      try expectKeyword("AND")
      return .between(left, lower: lower, upper: try parseAdditive(), negated: false)
    }
    if matchKeyword("IN") {
      return .inList(left, values: try parseParenthesizedList(), negated: false)
    }
    if matchKeyword("LIKE") {
      return .like(left, pattern: try parseAdditive(), negated: false)
    }
    if peek().kind == .symbol, ["=", "!=", "<>", "<", ">", "<=", ">="].contains(peek().value) {
      let operation = advance().value
      return .binary(operation, left, try parseAdditive())
    }
    return left
  }

  private mutating func parseParenthesizedList() throws -> [Expression] {
    try expectSymbol("(")
    let values = try parseExpressionList()
    try expectSymbol(")")
    return values
  }

  private mutating func parseAdditive() throws -> Expression {
    var left = try parseMultiplicative()
    while peek().kind == .symbol, ["+", "-"].contains(peek().value) {
      let operation = advance().value
      left = .binary(operation, left, try parseMultiplicative())
    }
    return left
  }

  private mutating func parseMultiplicative() throws -> Expression {
    var left = try parseUnary()
    while peek().kind == .symbol, ["*", "/", "%"].contains(peek().value) {
      let operation = advance().value
      left = .binary(operation, left, try parseUnary())
    }
    return left
  }

  private mutating func parseUnary() throws -> Expression {
    matchSymbol("-") ? .unary("-", try parseUnary()) : try parsePrimary()
  }

  private mutating func parsePrimary() throws -> Expression {
    let token = peek()
    if matchSymbol("(") {
      let expression = try parseExpression()
      try expectSymbol(")")
      return expression
    }
    if token.kind == .number {
      _ = advance()
      if token.value.contains("."), let value = Double(token.value) {
        return .literal(.real(value))
      }
      guard let value = Int(token.value) else { throw error("invalid number '\(token.value)'") }
      return .literal(.integer(value))
    }
    if token.kind == .string {
      _ = advance()
      return .literal(.text(token.value))
    }
    if matchKeyword("NULL") { return .literal(.null) }
    if matchKeyword("TRUE") { return .literal(.boolean(true)) }
    if matchKeyword("FALSE") { return .literal(.boolean(false)) }
    if matchSymbol("*") { return .star }
    if token.kind == .identifier || token.kind == .keyword {
      let name = advance().value
      if matchSymbol("(") {
        var arguments: [Expression] = []
        if !matchSymbol(")") {
          if matchSymbol("*") {
            arguments.append(.star)
          } else {
            arguments.append(try parseExpression())
            while matchSymbol(",") { arguments.append(try parseExpression()) }
          }
          try expectSymbol(")")
        }
        return .function(name, arguments)
      }
      if matchSymbol(".") {
        return .column(table: name, name: try expectIdentifier())
      }
      return .column(table: nil, name: name)
    }
    throw error("unexpected token '\(token.value)'")
  }

  private func peek() -> Token { tokens[position] }

  @discardableResult
  private mutating func advance() -> Token {
    let token = tokens[position]
    if token.kind != .end { position += 1 }
    return token
  }

  private mutating func expectIdentifier() throws -> String {
    let token = advance()
    guard token.kind == .identifier || token.kind == .keyword else {
      throw error("expected identifier")
    }
    return token.value
  }

  private mutating func expectInteger() throws -> Int {
    let token = advance()
    guard token.kind == .number, !token.value.contains("."), let value = Int(token.value) else {
      throw error("expected integer")
    }
    return value
  }

  private mutating func expectKeyword(_ value: String) throws {
    guard matchKeyword(value) else { throw error("expected \(value)") }
  }

  private mutating func matchKeyword(_ value: String) -> Bool {
    guard peek().kind == .keyword, peek().value == value else { return false }
    _ = advance()
    return true
  }

  private mutating func expectSymbol(_ value: String) throws {
    guard matchSymbol(value) else { throw error("expected '\(value)'") }
  }

  private mutating func matchSymbol(_ value: String) -> Bool {
    guard peek().kind == .symbol, peek().value == value else { return false }
    _ = advance()
    return true
  }

  private func error(_ message: String) -> SqlExecutionError {
    .parse("\(message) at offset \(peek().offset)")
  }
}
