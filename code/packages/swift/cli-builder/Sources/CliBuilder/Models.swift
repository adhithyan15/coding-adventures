import Foundation

public enum ValueType: String, Sendable, CaseIterable {
  case boolean
  case count
  case string
  case integer
  case float
  case path
  case file
  case directory
  case enumeration = "enum"

  var takesValue: Bool { self != .boolean && self != .count }
}

public enum ParsingMode: String, Sendable {
  case gnu
  case posix
  case subcommandFirst = "subcommand_first"
  case traditional
}

public indirect enum CliValue: Equatable, Sendable, CustomStringConvertible {
  case null
  case bool(Bool)
  case int(Int)
  case double(Double)
  case string(String)
  case array([CliValue])

  public var description: String {
    switch self {
    case .null: "null"
    case .bool(let value): String(value)
    case .int(let value): String(value)
    case .double(let value): String(value)
    case .string(let value): value
    case .array(let values): "[\(values.map(\.description).joined(separator: ", "))]"
    }
  }

  var isPresent: Bool {
    switch self {
    case .null: false
    case .bool(let value): value
    case .int(let value): value != 0
    case .array(let values): !values.isEmpty
    default: true
    }
  }

  static func fromJSON(_ value: Any?) -> CliValue {
    guard let value else { return .null }
    if let value = value as? Bool { return .bool(value) }
    if let value = value as? Int { return .int(value) }
    if let value = value as? Double { return .double(value) }
    if let value = value as? String { return .string(value) }
    if let value = value as? [Any] { return .array(value.map(CliValue.fromJSON)) }
    return .string(String(describing: value))
  }
}

public struct BuiltinFlags: Equatable, Sendable {
  public let help: Bool
  public let version: Bool
}

public struct FlagDefinition: Equatable, Sendable {
  public let id: String
  public let shortName: String?
  public let longName: String?
  public let singleDashLong: String?
  public let description: String
  public let type: ValueType
  public let required: Bool
  public let defaultValue: CliValue
  public let valueName: String?
  public let enumValues: [String]
  public let defaultWhenPresent: String?
  public let conflictsWith: [String]
  public let requires: [String]
  public let requiredUnless: [String]
  public let repeatable: Bool

  public init(
    id: String,
    shortName: String? = nil,
    longName: String? = nil,
    singleDashLong: String? = nil,
    description: String = "",
    type: ValueType = .string,
    required: Bool = false,
    defaultValue: CliValue = .null,
    valueName: String? = nil,
    enumValues: [String] = [],
    defaultWhenPresent: String? = nil,
    conflictsWith: [String] = [],
    requires: [String] = [],
    requiredUnless: [String] = [],
    repeatable: Bool = false
  ) {
    self.id = id
    self.shortName = shortName
    self.longName = longName
    self.singleDashLong = singleDashLong
    self.description = description
    self.type = type
    self.required = required
    self.defaultValue = defaultValue
    self.valueName = valueName
    self.enumValues = enumValues
    self.defaultWhenPresent = defaultWhenPresent
    self.conflictsWith = conflictsWith
    self.requires = requires
    self.requiredUnless = requiredUnless
    self.repeatable = repeatable
  }
}

public struct ArgumentDefinition: Equatable, Sendable {
  public let id: String
  public let displayName: String
  public let description: String
  public let type: ValueType
  public let required: Bool
  public let variadic: Bool
  public let variadicMin: Int
  public let variadicMax: Int?
  public let defaultValue: CliValue
  public let enumValues: [String]
  public let requiredUnlessFlag: [String]
}

public struct ExclusiveGroup: Equatable, Sendable {
  public let id: String
  public let flagIds: [String]
  public let required: Bool
}

public struct CommandDefinition: Equatable, Sendable {
  public let id: String
  public let name: String
  public let aliases: [String]
  public let description: String
  public let inheritGlobalFlags: Bool
  public let flags: [FlagDefinition]
  public let arguments: [ArgumentDefinition]
  public let commands: [CommandDefinition]
  public let mutuallyExclusiveGroups: [ExclusiveGroup]

  func findCommand(_ token: String) -> CommandDefinition? {
    commands.first { $0.name == token || $0.aliases.contains(token) }
  }
}

public struct CliSpec: Equatable, Sendable {
  public let specVersion: String
  public let name: String
  public let displayName: String?
  public let description: String
  public let version: String?
  public let parsingMode: ParsingMode
  public let builtinFlags: BuiltinFlags
  public let globalFlags: [FlagDefinition]
  public let flags: [FlagDefinition]
  public let arguments: [ArgumentDefinition]
  public let commands: [CommandDefinition]
  public let mutuallyExclusiveGroups: [ExclusiveGroup]

  func findCommand(path: [String]) -> CommandDefinition? {
    var scope = commands
    var current: CommandDefinition?
    for token in path {
      guard
        let command = scope.first(where: {
          $0.name == token || $0.aliases.contains(token)
        })
      else { return nil }
      current = command
      scope = command.commands
    }
    return current
  }

  func commandChain(path: [String]) -> [CommandDefinition] {
    var scope = commands
    var result: [CommandDefinition] = []
    for token in path {
      guard
        let command = scope.first(where: {
          $0.name == token || $0.aliases.contains(token)
        })
      else { break }
      result.append(command)
      scope = command.commands
    }
    return result
  }

  func flagsForPath(_ path: [String]) -> [FlagDefinition] {
    guard !path.isEmpty else { return flags + globalFlags }
    let chain = commandChain(path: path)
    guard let leaf = chain.last else { return flags + globalFlags }
    var result = chain.flatMap(\.flags)
    if leaf.inheritGlobalFlags { result += globalFlags }
    return deduplicatedFlags(result)
  }

  func argumentsForPath(_ path: [String]) -> [ArgumentDefinition] {
    path.isEmpty ? arguments : findCommand(path: path)?.arguments ?? arguments
  }

  func groupsForPath(_ path: [String]) -> [ExclusiveGroup] {
    path.isEmpty ? mutuallyExclusiveGroups : findCommand(path: path)?.mutuallyExclusiveGroups ?? []
  }
}

private func deduplicatedFlags(_ flags: [FlagDefinition]) -> [FlagDefinition] {
  var seen: Set<String> = []
  return flags.filter { seen.insert($0.id).inserted }
}

public struct ParseResult: Equatable, Sendable {
  public let program: String
  public let commandPath: [String]
  public let flags: [String: CliValue]
  public let arguments: [String: CliValue]
  public let explicitFlags: [String]
}

public struct HelpResult: Equatable, Sendable {
  public let text: String
  public let commandPath: [String]
}

public struct VersionResult: Equatable, Sendable {
  public let version: String
}

public enum ParseOutcome: Equatable, Sendable {
  case parsed(ParseResult)
  case help(HelpResult)
  case version(VersionResult)
}

public struct ValidationResult: Equatable, Sendable {
  public let errors: [String]
  public let warnings: [String]

  public init(errors: [String], warnings: [String] = []) {
    self.errors = errors
    self.warnings = warnings
  }

  public var isValid: Bool { errors.isEmpty }
}

public struct SpecError: Error, Equatable, Sendable, CustomStringConvertible {
  public let message: String
  public init(_ message: String) { self.message = message }
  public var description: String { message }
}

public struct ParseIssue: Error, Equatable, Sendable {
  public let errorType: String
  public let message: String
  public let suggestion: String?
  public let context: [String]

  public init(
    errorType: String,
    message: String,
    suggestion: String? = nil,
    context: [String] = []
  ) {
    self.errorType = errorType
    self.message = message
    self.suggestion = suggestion
    self.context = context
  }
}

public struct ParseErrors: Error, Equatable, Sendable, CustomStringConvertible {
  public let errors: [ParseIssue]
  public init(_ errors: [ParseIssue]) { self.errors = errors }

  public var description: String {
    if errors.count == 1 { return "parse error: \(errors[0].message)" }
    return "\(errors.count) parse errors:\n"
      + errors.map { "  - \($0.message)" }.joined(separator: "\n")
  }
}
