public enum TokenKind: Equatable, Sendable {
  case endOfFlags
  case longFlag
  case longFlagWithValue
  case singleDashLong
  case shortFlag
  case shortFlagWithValue
  case stackedFlags
  case positional
  case unknownFlag
}

public struct TokenEvent: Equatable, Sendable {
  public let kind: TokenKind
  public let name: String?
  public let value: String?
  public let characters: [String]
  public let raw: String
}

public struct TokenClassifier: Sendable {
  private let longFlags: [String: FlagDefinition]
  private let shortFlags: [String: FlagDefinition]
  private let singleDashLongs: [String: FlagDefinition]

  public init(_ flags: [FlagDefinition]) {
    self.longFlags = Dictionary(
      flags.compactMap { flag in
        flag.longName.map { ($0, flag) }
      }, uniquingKeysWith: { first, _ in first })
    self.shortFlags = Dictionary(
      flags.compactMap { flag in
        flag.shortName.map { ($0, flag) }
      }, uniquingKeysWith: { first, _ in first })
    self.singleDashLongs = Dictionary(
      flags.compactMap { flag in
        flag.singleDashLong.map { ($0, flag) }
      }, uniquingKeysWith: { first, _ in first })
  }

  public func classify(_ token: String) -> TokenEvent {
    if token == "-" { return event(.positional, name: token, raw: token) }
    if token == "--" { return event(.endOfFlags, raw: token) }
    if token.hasPrefix("--") {
      let rest = String(token.dropFirst(2))
      if let equals = rest.firstIndex(of: "=") {
        return event(
          .longFlagWithValue,
          name: String(rest[..<equals]),
          value: String(rest[rest.index(after: equals)...]),
          raw: token
        )
      }
      return longFlags[rest] == nil
        ? event(.unknownFlag, name: rest, raw: token)
        : event(.longFlag, name: rest, raw: token)
    }
    if token.hasPrefix("-") && token.count >= 2 {
      let rest = String(token.dropFirst())
      if singleDashLongs[rest] != nil {
        return event(.singleDashLong, name: rest, raw: token)
      }
      let first = String(rest.prefix(1))
      if let flag = shortFlags[first] {
        if !flag.type.takesValue {
          return rest.count == 1
            ? event(.shortFlag, name: first, raw: token)
            : classifyStack(rest, raw: token)
        }
        if rest.count == 1 { return event(.shortFlag, name: first, raw: token) }
        let suffix = String(rest.dropFirst())
        if suffix.allSatisfy({ shortFlags[String($0)] != nil }) {
          return event(.unknownFlag, name: first, raw: token)
        }
        return event(.shortFlagWithValue, name: first, value: suffix, raw: token)
      }
      return rest.count > 1
        ? classifyStack(rest, raw: token)
        : event(.unknownFlag, name: rest, raw: token)
    }
    return event(.positional, name: token, raw: token)
  }

  public func classifyTraditional(_ token: String, knownSubcommands: Set<String>) -> TokenEvent {
    if token.hasPrefix("-") || knownSubcommands.contains(token) { return classify(token) }
    let stacked = classifyStack(token, raw: token)
    return stacked.kind == .stackedFlags ? stacked : event(.positional, name: token, raw: token)
  }

  public func flag(long name: String) -> FlagDefinition? { longFlags[name] }
  public func flag(short name: String) -> FlagDefinition? { shortFlags[name] }
  public func flag(singleDashLong name: String) -> FlagDefinition? { singleDashLongs[name] }
  public var knownLongNames: [String] { longFlags.keys.map { "--\($0)" }.sorted() }
  public var knownShortNames: [String] { shortFlags.keys.map { "-\($0)" }.sorted() }

  private func classifyStack(_ text: String, raw: String) -> TokenEvent {
    var result: [String] = []
    let characters = text.map(String.init)
    for (index, character) in characters.enumerated() {
      guard let flag = shortFlags[character] else {
        return event(.unknownFlag, name: character, raw: raw)
      }
      if flag.type.takesValue && index < characters.count - 1 {
        return event(.unknownFlag, name: character, raw: raw)
      }
      result.append(character)
    }
    return event(.stackedFlags, characters: result, raw: raw)
  }
}

private func event(
  _ kind: TokenKind,
  name: String? = nil,
  value: String? = nil,
  characters: [String] = [],
  raw: String
) -> TokenEvent {
  TokenEvent(kind: kind, name: name, value: value, characters: characters, raw: raw)
}
