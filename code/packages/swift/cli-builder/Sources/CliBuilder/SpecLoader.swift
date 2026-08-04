import DirectedGraph
import Foundation

public struct SpecLoader: Sendable {
  public init() {}

  public func load(fromFile path: String) throws -> CliSpec {
    guard let data = FileManager.default.contents(atPath: path) else {
      throw SpecError("unable to read CLI spec: \(path)")
    }
    return try load(from: data)
  }

  public func load(from json: String) throws -> CliSpec {
    guard let data = json.data(using: .utf8) else {
      throw SpecError("CLI spec is not valid UTF-8")
    }
    return try load(from: data)
  }

  public func load(from data: Data) throws -> CliSpec {
    let object = try jsonObject(data)
    let validation = validateSpecObject(object)
    guard validation.isValid else { throw SpecError(validation.errors.joined(separator: "\n")) }
    return try decodeSpec(object)
  }
}

public func validateSpec(_ json: String) throws -> ValidationResult {
  guard let data = json.data(using: .utf8) else {
    throw SpecError("CLI spec is not valid UTF-8")
  }
  return validateSpecObject(try jsonObject(data))
}

public func validateSpecFile(_ path: String) throws -> ValidationResult {
  guard let data = FileManager.default.contents(atPath: path) else {
    throw SpecError("unable to read CLI spec: \(path)")
  }
  return validateSpecObject(try jsonObject(data))
}

private func jsonObject(_ data: Data) throws -> [String: Any] {
  do {
    guard let object = try JSONSerialization.jsonObject(with: data) as? [String: Any] else {
      throw SpecError("CLI spec root must be a JSON object")
    }
    return object
  } catch let error as SpecError {
    throw error
  } catch {
    throw SpecError("invalid CLI spec JSON: \(error.localizedDescription)")
  }
}

private func validateSpecObject(_ object: [String: Any]) -> ValidationResult {
  var errors: [String] = []
  let version =
    string(object, "cli_builder_spec_version")
    ?? string(object, "spec_version")
    ?? "1.0"
  if version != "1.0" {
    errors.append("unsupported cli_builder_spec_version \"\(version)\"")
  }
  if (string(object, "name") ?? "").isEmpty {
    errors.append("required field \"name\" is missing or empty")
  }
  if (string(object, "description") ?? "").isEmpty {
    errors.append("required field \"description\" is missing or empty")
  }

  do {
    let spec = try decodeSpec(object)
    let globalIDs = Set(spec.globalFlags.map(\.id))
    validateScope(
      name: "root",
      flags: spec.flags + spec.globalFlags,
      arguments: spec.arguments,
      groups: spec.mutuallyExclusiveGroups,
      visibleIDs: globalIDs,
      errors: &errors
    )
    validateCommandNames(spec.commands, scope: "root", errors: &errors)
    for command in spec.commands {
      validateCommand(command, inheritedIDs: globalIDs, errors: &errors)
    }
  } catch let error as SpecError {
    errors.append(error.message)
  } catch {
    errors.append(String(describing: error))
  }
  return ValidationResult(errors: errors)
}

private func validateCommand(
  _ command: CommandDefinition,
  inheritedIDs: Set<String>,
  errors: inout [String]
) {
  validateScope(
    name: "command \"\(command.id)\"",
    flags: command.flags,
    arguments: command.arguments,
    groups: command.mutuallyExclusiveGroups,
    visibleIDs: inheritedIDs,
    errors: &errors
  )
  validateCommandNames(command.commands, scope: "command \"\(command.id)\"", errors: &errors)
  let nextIDs = inheritedIDs.union(command.flags.map(\.id))
  for nested in command.commands {
    validateCommand(nested, inheritedIDs: nextIDs, errors: &errors)
  }
}

private func validateCommandNames(
  _ commands: [CommandDefinition],
  scope: String,
  errors: inout [String]
) {
  var names: Set<String> = []
  for command in commands {
    if !names.insert(command.name).inserted {
      errors.append("in \(scope): duplicate command name \"\(command.name)\"")
    }
    for alias in command.aliases where !names.insert(alias).inserted {
      errors.append("in \(scope): duplicate command alias \"\(alias)\"")
    }
  }
}

private func validateScope(
  name: String,
  flags: [FlagDefinition],
  arguments: [ArgumentDefinition],
  groups: [ExclusiveGroup],
  visibleIDs inheritedIDs: Set<String>,
  errors: inout [String]
) {
  var localIDs: Set<String> = []
  var visibleIDs = inheritedIDs
  var shortNames: Set<String> = []
  var longNames: Set<String> = []
  var singleDashLongs: Set<String> = []

  for flag in flags {
    if !localIDs.insert(flag.id).inserted {
      errors.append("in \(name): duplicate flag id \"\(flag.id)\"")
    }
    visibleIDs.insert(flag.id)
    if flag.shortName == nil && flag.longName == nil && flag.singleDashLong == nil {
      errors.append("in \(name): flag \"\(flag.id)\" must declare short, long, or single_dash_long")
    }
    if let shortName = flag.shortName, !shortNames.insert(shortName).inserted {
      errors.append("in \(name): duplicate short flag \"-\(shortName)\"")
    }
    if let longName = flag.longName, !longNames.insert(longName).inserted {
      errors.append("in \(name): duplicate long flag \"--\(longName)\"")
    }
    if let single = flag.singleDashLong, !singleDashLongs.insert(single).inserted {
      errors.append("in \(name): duplicate single-dash-long flag \"-\(single)\"")
    }
    if flag.type == .enumeration && flag.enumValues.isEmpty {
      errors.append("in \(name): flag \"\(flag.id)\" has type enum but no enum_values")
    }
    if flag.defaultWhenPresent != nil && flag.type != .enumeration {
      errors.append("in \(name): flag \"\(flag.id)\" has default_when_present but is not an enum")
    }
  }

  for flag in flags {
    for reference in flag.conflictsWith + flag.requires + flag.requiredUnless
    where !visibleIDs.contains(reference) {
      errors.append("in \(name): flag \"\(flag.id)\" references unknown flag id \"\(reference)\"")
    }
  }

  var graph = Graph()
  for id in visibleIDs { graph.addNode(id) }
  var graphConstructionFailed = false
  for flag in flags {
    for required in flag.requires where visibleIDs.contains(required) {
      do { try graph.addEdge(from: flag.id, to: required) } catch { graphConstructionFailed = true }
    }
  }
  if graphConstructionFailed || graph.hasCycle() {
    errors.append("in \(name): circular requires dependency detected")
  }

  var argumentIDs: Set<String> = []
  var variadicCount = 0
  for argument in arguments {
    if !argumentIDs.insert(argument.id).inserted {
      errors.append("in \(name): duplicate argument id \"\(argument.id)\"")
    }
    if argument.type == .enumeration && argument.enumValues.isEmpty {
      errors.append("in \(name): argument \"\(argument.id)\" has type enum but no enum_values")
    }
    if argument.variadic { variadicCount += 1 }
    if let maximum = argument.variadicMax, maximum < argument.variadicMin {
      errors.append("in \(name): argument \"\(argument.id)\" has variadic_max below variadic_min")
    }
    for id in argument.requiredUnlessFlag where !visibleIDs.contains(id) {
      errors.append("in \(name): argument \"\(argument.id)\" references unknown flag id \"\(id)\"")
    }
  }
  if variadicCount > 1 {
    errors.append("in \(name): at most one argument may be variadic")
  }
  for group in groups {
    for id in group.flagIds where !visibleIDs.contains(id) {
      errors.append(
        "in \(name): mutually exclusive group \"\(group.id)\" references unknown flag id \"\(id)\"")
    }
  }
}

private func decodeSpec(_ object: [String: Any]) throws -> CliSpec {
  let builtinObject = object["builtin_flags"] as? [String: Any] ?? [:]
  let parsedVersion = string(object, "version")
  return CliSpec(
    specVersion: string(object, "cli_builder_spec_version")
      ?? string(object, "spec_version")
      ?? "1.0",
    name: string(object, "name") ?? "",
    displayName: string(object, "display_name"),
    description: string(object, "description") ?? "",
    version: parsedVersion,
    parsingMode: ParsingMode(rawValue: string(object, "parsing_mode") ?? "gnu") ?? .gnu,
    builtinFlags: BuiltinFlags(
      help: boolean(builtinObject, "help") ?? true,
      version: boolean(builtinObject, "version") ?? (parsedVersion != nil)
    ),
    globalFlags: try objectArray(object, "global_flags").map(decodeFlag),
    flags: try objectArray(object, "flags").map(decodeFlag),
    arguments: try objectArray(object, "arguments").map(decodeArgument),
    commands: try objectArray(object, "commands").map(decodeCommand),
    mutuallyExclusiveGroups: try objectArray(object, "mutually_exclusive_groups").map(decodeGroup)
  )
}

private func decodeFlag(_ object: [String: Any]) throws -> FlagDefinition {
  guard let id = string(object, "id"), !id.isEmpty else {
    throw SpecError("flag is missing required field \"id\"")
  }
  let typeName = string(object, "type") ?? "string"
  guard let type = ValueType(rawValue: typeName) else {
    throw SpecError("flag \"\(id)\" has unknown type \"\(typeName)\"")
  }
  return FlagDefinition(
    id: id,
    shortName: string(object, "short"),
    longName: string(object, "long"),
    singleDashLong: string(object, "single_dash_long"),
    description: string(object, "description") ?? "",
    type: type,
    required: boolean(object, "required") ?? false,
    defaultValue: CliValue.fromJSON(object["default"]),
    valueName: string(object, "value_name"),
    enumValues: stringArray(object, "enum_values"),
    defaultWhenPresent: string(object, "default_when_present"),
    conflictsWith: stringArray(object, "conflicts_with"),
    requires: stringArray(object, "requires"),
    requiredUnless: stringArray(object, "required_unless"),
    repeatable: boolean(object, "repeatable") ?? false
  )
}

private func decodeArgument(_ object: [String: Any]) throws -> ArgumentDefinition {
  guard let id = string(object, "id"), !id.isEmpty else {
    throw SpecError("argument is missing required field \"id\"")
  }
  let required = boolean(object, "required") ?? true
  let typeName = string(object, "type") ?? "string"
  guard let type = ValueType(rawValue: typeName) else {
    throw SpecError("argument \"\(id)\" has unknown type \"\(typeName)\"")
  }
  return ArgumentDefinition(
    id: id,
    displayName: string(object, "display_name") ?? string(object, "name") ?? "",
    description: string(object, "description") ?? "",
    type: type,
    required: required,
    variadic: boolean(object, "variadic") ?? false,
    variadicMin: integer(object, "variadic_min") ?? (required ? 1 : 0),
    variadicMax: integer(object, "variadic_max"),
    defaultValue: CliValue.fromJSON(object["default"]),
    enumValues: stringArray(object, "enum_values"),
    requiredUnlessFlag: stringArray(object, "required_unless_flag")
  )
}

private func decodeGroup(_ object: [String: Any]) throws -> ExclusiveGroup {
  guard let id = string(object, "id"), !id.isEmpty else {
    throw SpecError("mutually exclusive group is missing required field \"id\"")
  }
  return ExclusiveGroup(
    id: id,
    flagIds: stringArray(object, "flag_ids"),
    required: boolean(object, "required") ?? false
  )
}

private func decodeCommand(_ object: [String: Any]) throws -> CommandDefinition {
  guard let name = string(object, "name"), !name.isEmpty else {
    throw SpecError("command is missing required field \"name\"")
  }
  return CommandDefinition(
    id: string(object, "id") ?? name,
    name: name,
    aliases: stringArray(object, "aliases"),
    description: string(object, "description") ?? "",
    inheritGlobalFlags: boolean(object, "inherit_global_flags") ?? true,
    flags: try objectArray(object, "flags").map(decodeFlag),
    arguments: try objectArray(object, "arguments").map(decodeArgument),
    commands: try objectArray(object, "commands").map(decodeCommand),
    mutuallyExclusiveGroups: try objectArray(object, "mutually_exclusive_groups").map(decodeGroup)
  )
}

private func objectArray(_ object: [String: Any], _ key: String) throws -> [[String: Any]] {
  guard let value = object[key] else { return [] }
  guard let array = value as? [Any] else { throw SpecError("field \"\(key)\" must be an array") }
  return try array.map { value in
    guard let dictionary = value as? [String: Any] else {
      throw SpecError("field \"\(key)\" must contain objects")
    }
    return dictionary
  }
}

private func string(_ object: [String: Any], _ key: String) -> String? {
  object[key] as? String
}

private func boolean(_ object: [String: Any], _ key: String) -> Bool? {
  object[key] as? Bool
}

private func integer(_ object: [String: Any], _ key: String) -> Int? {
  if let value = object[key] as? Int { return value }
  if let value = object[key] as? NSNumber { return value.intValue }
  return nil
}

private func stringArray(_ object: [String: Any], _ key: String) -> [String] {
  (object[key] as? [Any] ?? []).compactMap { $0 as? String }
}
