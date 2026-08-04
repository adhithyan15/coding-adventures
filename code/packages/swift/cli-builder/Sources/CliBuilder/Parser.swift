import DirectedGraph
import Foundation
import StateMachine

public struct Parser: Sendable {
  public let spec: CliSpec
  public let argv: [String]

  public init(spec: CliSpec, argv: [String]) {
    self.spec = spec
    self.argv = argv
  }

  public init(specPath: String, argv: [String]) throws {
    self.init(spec: try SpecLoader().load(fromFile: specPath), argv: argv)
  }

  public func parse() throws -> ParseOutcome {
    guard let program = argv.first else {
      throw ParseErrors([
        ParseIssue(
          errorType: "missing_required_argument",
          message: "argv is empty (no program name)"
        )
      ])
    }

    var tokens = Array(argv.dropFirst())
    if spec.parsingMode == .traditional,
      let first = tokens.first,
      !first.hasPrefix("-"),
      !knownRootCommandNames.contains(first)
    {
      tokens = first.map { "-\($0)" } + tokens.dropFirst()
    }

    let routed = route(tokens)
    let activeFlags = deduplicate(spec.flagsForPath(routed.commandPath) + builtinFlags)
    let scanned = try scan(
      routed.remainingTokens,
      commandPath: routed.commandPath,
      activeFlags: activeFlags
    )

    if scanned.helpRequested {
      return .help(
        HelpResult(
          text: generateHelp(commandPath: routed.commandPath),
          commandPath: [program] + routed.commandPath
        ))
    }
    if scanned.versionRequested {
      return .version(VersionResult(version: spec.version ?? "(unknown)"))
    }

    var errors = routed.errors + scanned.errors
    let arguments = resolvePositionals(
      definitions: spec.argumentsForPath(routed.commandPath),
      tokens: scanned.positionals,
      parsedFlags: scanned.flags,
      errors: &errors
    )
    validateFlags(
      activeFlags,
      groups: spec.groupsForPath(routed.commandPath),
      parsedFlags: scanned.flags,
      errors: &errors
    )
    guard errors.isEmpty else { throw ParseErrors(errors) }

    return .parsed(
      ParseResult(
        program: program,
        commandPath: [program] + routed.commandPath,
        flags: applyDefaults(activeFlags, to: scanned.flags),
        arguments: arguments,
        explicitFlags: scanned.explicitFlags
      ))
  }

  private func route(_ tokens: [String]) -> RoutingResult {
    var commandPath: [String] = []
    var consumedIndices: Set<Int> = []
    var errors: [ParseIssue] = []
    var scope = spec.commands
    var index = 0

    while index < tokens.count {
      let token = tokens[index]
      if token == "--" { break }
      if token.hasPrefix("-") {
        let routingFlags = deduplicate(spec.flagsForPath(commandPath) + builtinFlags)
        index += flagConsumesNextValue(token, flags: routingFlags) ? 2 : 1
        continue
      }
      guard
        let command = scope.first(where: {
          $0.name == token || $0.aliases.contains(token)
        })
      else {
        if spec.parsingMode == .subcommandFirst && commandPath.isEmpty {
          let suggestion = fuzzyMatch(token, candidates: Array(knownRootCommandNames))
          errors.append(
            ParseIssue(
              errorType: "unknown_command",
              message: "unknown command \"\(token)\"",
              suggestion: suggestion.map { "Did you mean \"\($0)\"?" }
            ))
        }
        break
      }
      commandPath.append(command.name)
      consumedIndices.insert(index)
      scope = command.commands
      index += 1
    }

    return RoutingResult(
      commandPath: commandPath,
      remainingTokens: tokens.enumerated().compactMap {
        consumedIndices.contains($0.offset) ? nil : $0.element
      },
      errors: errors
    )
  }

  private func scan(
    _ tokens: [String],
    commandPath: [String],
    activeFlags: [FlagDefinition]
  ) throws -> ScanningResult {
    var flags: [String: CliValue] = [:]
    var positionals: [String] = []
    var errors: [ParseIssue] = []
    var explicitFlags: [String] = []
    let classifier = TokenClassifier(activeFlags)
    let machine = try scannerMachine()
    var pendingFlag: FlagDefinition?
    var index = 0

    while index < tokens.count {
      let token = tokens[index]
      if machine.currentState == "END_OF_FLAGS" {
        positionals.append(token)
        try machine.process("token")
        index += 1
        continue
      }
      if machine.currentState == "FLAG_VALUE" {
        if let flag = pendingFlag {
          if flag.type == .enumeration,
            let defaultValue = flag.defaultWhenPresent,
            token.hasPrefix("-") || !flag.enumValues.contains(token)
          {
            setFlagValue(.string(defaultValue), for: flag, flags: &flags, errors: &errors)
            explicitFlags.append(flag.id)
            pendingFlag = nil
            try machine.process("value_consumed")
            continue
          }
          if let value = coerce(token, type: flag.type, enumValues: flag.enumValues) {
            setFlagValue(value, for: flag, flags: &flags, errors: &errors)
            explicitFlags.append(flag.id)
          } else {
            errors.append(invalidValueIssue(token, flag: flag))
          }
          pendingFlag = nil
        }
        try machine.process("value_consumed")
        index += 1
        continue
      }

      let event = classifier.classify(token)
      switch event.kind {
      case .endOfFlags:
        try machine.process("end_flags")

      case .longFlag:
        guard let name = event.name, let flag = classifier.flag(long: name) else {
          appendUnknownFlag(token, classifier: classifier, errors: &errors)
          try machine.process("token")
          break
        }
        let action = applyFlag(
          flag,
          inlineValue: nil,
          flags: &flags,
          errors: &errors,
          explicitFlags: &explicitFlags
        )
        switch action {
        case .help: return scanResult(flags, positionals, errors, explicitFlags, help: true)
        case .version: return scanResult(flags, positionals, errors, explicitFlags, version: true)
        case .awaitValue:
          pendingFlag = flag
          try machine.process("await_value")
        case .done: try machine.process("token")
        }

      case .shortFlag:
        guard let name = event.name, let flag = classifier.flag(short: name) else {
          appendUnknownFlag(token, classifier: classifier, errors: &errors)
          try machine.process("token")
          break
        }
        let action = applyFlag(
          flag,
          inlineValue: nil,
          flags: &flags,
          errors: &errors,
          explicitFlags: &explicitFlags
        )
        switch action {
        case .help: return scanResult(flags, positionals, errors, explicitFlags, help: true)
        case .version: return scanResult(flags, positionals, errors, explicitFlags, version: true)
        case .awaitValue:
          pendingFlag = flag
          try machine.process("await_value")
        case .done: try machine.process("token")
        }

      case .singleDashLong:
        guard let name = event.name, let flag = classifier.flag(singleDashLong: name) else {
          appendUnknownFlag(token, classifier: classifier, errors: &errors)
          try machine.process("token")
          break
        }
        let action = applyFlag(
          flag,
          inlineValue: nil,
          flags: &flags,
          errors: &errors,
          explicitFlags: &explicitFlags
        )
        if action == .awaitValue {
          pendingFlag = flag
          try machine.process("await_value")
        } else {
          try machine.process("token")
        }

      case .longFlagWithValue, .shortFlagWithValue:
        let flag: FlagDefinition?
        if event.kind == .longFlagWithValue {
          flag = event.name.flatMap(classifier.flag(long:))
        } else {
          flag = event.name.flatMap(classifier.flag(short:))
        }
        guard let flag, let value = event.value else {
          appendUnknownFlag(token, classifier: classifier, errors: &errors)
          try machine.process("token")
          break
        }
        _ = applyFlag(
          flag,
          inlineValue: value,
          flags: &flags,
          errors: &errors,
          explicitFlags: &explicitFlags
        )
        try machine.process("token")

      case .stackedFlags:
        var awaitsValue = false
        for character in event.characters {
          guard let flag = classifier.flag(short: character) else {
            appendUnknownFlag("-\(character)", classifier: classifier, errors: &errors)
            continue
          }
          let action = applyFlag(
            flag,
            inlineValue: nil,
            flags: &flags,
            errors: &errors,
            explicitFlags: &explicitFlags
          )
          if action == .help {
            return scanResult(flags, positionals, errors, explicitFlags, help: true)
          }
          if action == .version {
            return scanResult(flags, positionals, errors, explicitFlags, version: true)
          }
          if action == .awaitValue {
            pendingFlag = flag
            awaitsValue = true
          }
        }
        try machine.process(awaitsValue ? "await_value" : "token")

      case .positional:
        positionals.append(event.name ?? token)
        try machine.process(spec.parsingMode == .posix ? "end_flags" : "token")

      case .unknownFlag:
        appendUnknownFlag(token, classifier: classifier, errors: &errors)
        try machine.process("token")
      }
      index += 1
    }

    if let flag = pendingFlag {
      if flag.type == .enumeration, let defaultValue = flag.defaultWhenPresent {
        setFlagValue(.string(defaultValue), for: flag, flags: &flags, errors: &errors)
        explicitFlags.append(flag.id)
      } else {
        errors.append(
          ParseIssue(
            errorType: "missing_required_argument",
            message: "\(flagLabel(flag)) expects a value",
            context: commandPath
          ))
      }
    }
    return scanResult(flags, positionals, errors, explicitFlags)
  }

  private func applyFlag(
    _ flag: FlagDefinition,
    inlineValue: String?,
    flags: inout [String: CliValue],
    errors: inout [ParseIssue],
    explicitFlags: inout [String]
  ) -> FlagAction {
    if flag.id == "help" { return .help }
    if flag.id == "version" { return .version }
    if let inlineValue {
      if let value = coerce(inlineValue, type: flag.type, enumValues: flag.enumValues) {
        setFlagValue(value, for: flag, flags: &flags, errors: &errors)
        explicitFlags.append(flag.id)
      } else {
        errors.append(invalidValueIssue(inlineValue, flag: flag))
      }
      return .done
    }
    switch flag.type {
    case .boolean:
      setFlagValue(.bool(true), for: flag, flags: &flags, errors: &errors)
      explicitFlags.append(flag.id)
      return .done
    case .count:
      incrementCount(flag.id, flags: &flags)
      explicitFlags.append(flag.id)
      return .done
    default:
      return .awaitValue
    }
  }

  private func resolvePositionals(
    definitions: [ArgumentDefinition],
    tokens: [String],
    parsedFlags: [String: CliValue],
    errors: inout [ParseIssue]
  ) -> [String: CliValue] {
    guard !definitions.isEmpty else {
      if !tokens.isEmpty {
        errors.append(
          ParseIssue(
            errorType: "too_many_arguments",
            message: "unexpected positional argument(s): \(tokens)"
          ))
      }
      return [:]
    }

    guard let variadicIndex = definitions.firstIndex(where: \.variadic) else {
      var result: [String: CliValue] = [:]
      for (index, definition) in definitions.enumerated() {
        assignArgument(
          definition,
          token: index < tokens.count ? tokens[index] : nil,
          parsedFlags: parsedFlags,
          result: &result,
          errors: &errors
        )
      }
      if tokens.count > definitions.count {
        errors.append(
          ParseIssue(
            errorType: "too_many_arguments",
            message: "unexpected argument(s): \(Array(tokens.dropFirst(definitions.count)))"
          ))
      }
      return result
    }

    var result: [String: CliValue] = [:]
    let leading = Array(definitions[..<variadicIndex])
    let variadic = definitions[variadicIndex]
    let trailing = Array(definitions[(variadicIndex + 1)...])

    for (index, definition) in leading.enumerated() {
      assignArgument(
        definition,
        token: index < tokens.count ? tokens[index] : nil,
        parsedFlags: parsedFlags,
        result: &result,
        errors: &errors
      )
    }

    let variadicStart = min(leading.count, tokens.count)
    let trailingStart = max(tokens.count - trailing.count, variadicStart)
    for (index, definition) in trailing.enumerated() {
      let tokenIndex = trailingStart + index
      assignArgument(
        definition,
        token: tokenIndex < tokens.count ? tokens[tokenIndex] : nil,
        parsedFlags: parsedFlags,
        result: &result,
        errors: &errors
      )
    }

    let variadicTokens = Array(tokens[variadicStart..<trailingStart])
    if variadicTokens.count < variadic.variadicMin {
      errors.append(
        ParseIssue(
          errorType: "too_few_arguments",
          message:
            "expected at least \(variadic.variadicMin) <\(variadic.displayName)>, got \(variadicTokens.count)"
        ))
    }
    if let maximum = variadic.variadicMax, variadicTokens.count > maximum {
      errors.append(
        ParseIssue(
          errorType: "too_many_arguments",
          message:
            "expected at most \(maximum) <\(variadic.displayName)>, got \(variadicTokens.count)"
        ))
    }
    result[variadic.id] = .array(
      variadicTokens.compactMap { token in
        guard let value = coerce(token, type: variadic.type, enumValues: variadic.enumValues) else {
          errors.append(invalidArgumentIssue(token, definition: variadic))
          return nil
        }
        return value
      })
    return result
  }

  private func assignArgument(
    _ definition: ArgumentDefinition,
    token: String?,
    parsedFlags: [String: CliValue],
    result: inout [String: CliValue],
    errors: inout [ParseIssue]
  ) {
    if let token {
      if let value = coerce(token, type: definition.type, enumValues: definition.enumValues) {
        result[definition.id] = value
      } else {
        errors.append(invalidArgumentIssue(token, definition: definition))
      }
      return
    }
    let exempt = definition.requiredUnlessFlag.contains {
      parsedFlags[$0]?.isPresent == true
    }
    if definition.required && !exempt {
      errors.append(
        ParseIssue(
          errorType: "missing_required_argument",
          message: "missing required argument: <\(definition.displayName)>"
        ))
    } else {
      result[definition.id] = definition.defaultValue
    }
  }

  private func validateFlags(
    _ activeFlags: [FlagDefinition],
    groups: [ExclusiveGroup],
    parsedFlags: [String: CliValue],
    errors: inout [ParseIssue]
  ) {
    var graph = Graph()
    let byID = Dictionary(activeFlags.map { ($0.id, $0) }, uniquingKeysWith: { first, _ in first })
    for flag in activeFlags { graph.addNode(flag.id) }
    for flag in activeFlags {
      for required in flag.requires where byID[required] != nil {
        try? graph.addEdge(from: flag.id, to: required)
      }
    }

    var emittedConflicts: Set<String> = []
    for flag in activeFlags {
      let present = parsedFlags[flag.id]?.isPresent == true
      if !present {
        let exempt = flag.requiredUnless.contains {
          parsedFlags[$0]?.isPresent == true
        }
        if flag.required && !exempt {
          errors.append(
            ParseIssue(
              errorType: "missing_required_flag",
              message: "\(flagLabel(flag)) is required"
            ))
        }
        continue
      }
      for otherID in flag.conflictsWith
      where parsedFlags[otherID]?.isPresent == true {
        guard let other = byID[otherID] else { continue }
        let key = [flag.id, otherID].sorted().joined(separator: "\0")
        if emittedConflicts.insert(key).inserted {
          errors.append(
            ParseIssue(
              errorType: "conflicting_flags",
              message: "\(flagLabel(flag)) and \(flagLabel(other)) cannot be used together"
            ))
        }
      }
      if let requiredIDs = try? graph.transitiveClosure(of: flag.id) {
        for requiredID in requiredIDs.sorted()
        where parsedFlags[requiredID]?.isPresent != true {
          guard let required = byID[requiredID] else { continue }
          errors.append(
            ParseIssue(
              errorType: "missing_dependency_flag",
              message: "\(flagLabel(flag)) requires \(flagLabel(required))"
            ))
        }
      }
    }

    for group in groups {
      let present = group.flagIds.filter { parsedFlags[$0]?.isPresent == true }
      if present.count > 1 {
        errors.append(
          ParseIssue(
            errorType: "exclusive_group_violation",
            message:
              "only one of \(present.compactMap { byID[$0].map(flagLabel) }.joined(separator: ", ")) may be used"
          ))
      } else if group.required && present.isEmpty {
        errors.append(
          ParseIssue(
            errorType: "missing_exclusive_group",
            message:
              "one of \(group.flagIds.compactMap { byID[$0].map(flagLabel) }.joined(separator: ", ")) is required"
          ))
      }
    }
  }

  private func applyDefaults(
    _ activeFlags: [FlagDefinition],
    to parsedFlags: [String: CliValue]
  ) -> [String: CliValue] {
    var result = parsedFlags
    for flag in activeFlags where result[flag.id] == nil {
      switch flag.type {
      case .boolean: result[flag.id] = .bool(false)
      case .count: result[flag.id] = .int(0)
      default: result[flag.id] = flag.defaultValue
      }
    }
    return result
  }

  private func generateHelp(commandPath: [String]) -> String {
    let command = spec.findCommand(path: commandPath)
    let isRoot = commandPath.isEmpty
    let description = isRoot ? spec.description : command?.description ?? spec.description
    let commands = isRoot ? spec.commands : command?.commands ?? []
    let localFlags = isRoot ? spec.flags : command?.flags ?? []
    let arguments = isRoot ? spec.arguments : command?.arguments ?? []
    let globalFlags = spec.globalFlags + builtinFlags
    var lines = [
      "USAGE",
      "  \(usageLine(commandPath, flags: localFlags, commands: commands, arguments: arguments))",
      "",
      "DESCRIPTION",
      "  \(description)",
    ]
    if !commands.isEmpty {
      lines += ["", "COMMANDS"]
      lines += commands.map {
        "  \($0.name.padding(toLength: 16, withPad: " ", startingAt: 0))\($0.description)"
      }
    }
    if !localFlags.isEmpty {
      lines += ["", "OPTIONS"]
      lines += localFlags.map {
        "  \(flagSignature($0).padding(toLength: 28, withPad: " ", startingAt: 0))\(flagDescription($0))"
      }
    }
    if !globalFlags.isEmpty {
      lines += ["", "GLOBAL OPTIONS"]
      lines += globalFlags.map {
        "  \(flagSignature($0).padding(toLength: 28, withPad: " ", startingAt: 0))\(flagDescription($0))"
      }
    }
    if !arguments.isEmpty {
      lines += ["", "ARGUMENTS"]
      lines += arguments.map {
        "  \(argumentUsage($0).padding(toLength: 16, withPad: " ", startingAt: 0))\($0.description)\($0.required ? " Required." : "")"
      }
    }
    return lines.joined(separator: "\n")
  }

  private func usageLine(
    _ commandPath: [String],
    flags: [FlagDefinition],
    commands: [CommandDefinition],
    arguments: [ArgumentDefinition]
  ) -> String {
    var parts = [spec.name] + commandPath
    if !flags.isEmpty || !spec.globalFlags.isEmpty { parts.append("[OPTIONS]") }
    if !commands.isEmpty { parts.append("[COMMAND]") }
    parts += arguments.map(argumentUsage)
    return parts.joined(separator: " ")
  }

  private var builtinFlags: [FlagDefinition] {
    var result: [FlagDefinition] = []
    if spec.builtinFlags.help {
      result.append(
        FlagDefinition(
          id: "help",
          shortName: "h",
          longName: "help",
          description: "Show this help message and exit.",
          type: .boolean,
          defaultValue: .bool(false)
        ))
    }
    if spec.builtinFlags.version {
      result.append(
        FlagDefinition(
          id: "version",
          longName: "version",
          description: "Show version and exit.",
          type: .boolean,
          defaultValue: .bool(false)
        ))
    }
    return result
  }

  private var knownRootCommandNames: Set<String> {
    Set(spec.commands.flatMap { [$0.name] + $0.aliases })
  }

  private func flagConsumesNextValue(_ token: String, flags: [FlagDefinition]) -> Bool {
    let classifier = TokenClassifier(flags)
    let event = classifier.classify(token)
    switch event.kind {
    case .longFlag: return event.name.flatMap(classifier.flag(long:))?.type.takesValue == true
    case .shortFlag: return event.name.flatMap(classifier.flag(short:))?.type.takesValue == true
    case .singleDashLong:
      return event.name.flatMap(classifier.flag(singleDashLong:))?.type.takesValue == true
    default: return false
    }
  }
}

private enum FlagAction: Equatable {
  case done
  case awaitValue
  case help
  case version
}

private struct RoutingResult {
  let commandPath: [String]
  let remainingTokens: [String]
  let errors: [ParseIssue]
}

private struct ScanningResult {
  let flags: [String: CliValue]
  let positionals: [String]
  let errors: [ParseIssue]
  let explicitFlags: [String]
  let helpRequested: Bool
  let versionRequested: Bool
}

private func scanResult(
  _ flags: [String: CliValue],
  _ positionals: [String],
  _ errors: [ParseIssue],
  _ explicitFlags: [String],
  help: Bool = false,
  version: Bool = false
) -> ScanningResult {
  ScanningResult(
    flags: flags,
    positionals: positionals,
    errors: errors,
    explicitFlags: explicitFlags,
    helpRequested: help,
    versionRequested: version
  )
}

private func scannerMachine() throws -> DFA {
  try DFA(
    states: ["SCANNING", "FLAG_VALUE", "END_OF_FLAGS"],
    alphabet: ["token", "await_value", "value_consumed", "end_flags"],
    transitions: [
      .init(from: "SCANNING", on: "token", to: "SCANNING"),
      .init(from: "SCANNING", on: "await_value", to: "FLAG_VALUE"),
      .init(from: "SCANNING", on: "end_flags", to: "END_OF_FLAGS"),
      .init(from: "FLAG_VALUE", on: "value_consumed", to: "SCANNING"),
      .init(from: "END_OF_FLAGS", on: "token", to: "END_OF_FLAGS"),
    ],
    initial: "SCANNING",
    accepting: ["SCANNING", "END_OF_FLAGS"]
  )
}

private func setFlagValue(
  _ value: CliValue,
  for flag: FlagDefinition,
  flags: inout [String: CliValue],
  errors: inout [ParseIssue]
) {
  if flag.repeatable {
    switch flags[flag.id] {
    case .array(let values): flags[flag.id] = .array(values + [value])
    case .some(let existing): flags[flag.id] = .array([existing, value])
    case .none: flags[flag.id] = .array([value])
    }
    return
  }
  if flags[flag.id] != nil {
    errors.append(
      ParseIssue(
        errorType: "duplicate_flag",
        message: "\(flagLabel(flag)) specified more than once"
      ))
    return
  }
  flags[flag.id] = value
}

private func incrementCount(_ id: String, flags: inout [String: CliValue]) {
  if case .int(let current) = flags[id] {
    flags[id] = .int(current + 1)
  } else {
    flags[id] = .int(1)
  }
}

private func coerce(_ raw: String, type: ValueType, enumValues: [String]) -> CliValue? {
  switch type {
  case .boolean: return .bool(raw == "true")
  case .count, .integer: return Int(raw).map(CliValue.int)
  case .float: return Double(raw).map(CliValue.double)
  case .string, .path: return .string(raw)
  case .file:
    var isDirectory: ObjCBool = false
    return FileManager.default.fileExists(atPath: raw, isDirectory: &isDirectory)
      && !isDirectory.boolValue
      ? .string(raw) : nil
  case .directory:
    var isDirectory: ObjCBool = false
    return FileManager.default.fileExists(atPath: raw, isDirectory: &isDirectory)
      && isDirectory.boolValue
      ? .string(raw) : nil
  case .enumeration: return enumValues.contains(raw) ? .string(raw) : nil
  }
}

private func invalidValueIssue(_ raw: String, flag: FlagDefinition) -> ParseIssue {
  ParseIssue(
    errorType: flag.type == .enumeration ? "invalid_enum_value" : "invalid_value",
    message: "invalid \(flag.type.rawValue) for \(flagLabel(flag)): \"\(raw)\""
  )
}

private func invalidArgumentIssue(
  _ raw: String,
  definition: ArgumentDefinition
) -> ParseIssue {
  ParseIssue(
    errorType: definition.type == .enumeration ? "invalid_enum_value" : "invalid_value",
    message:
      "invalid \(definition.type.rawValue) for argument <\(definition.displayName)>: \"\(raw)\""
  )
}

private func appendUnknownFlag(
  _ token: String,
  classifier: TokenClassifier,
  errors: inout [ParseIssue]
) {
  let suggestion = fuzzyMatch(
    token, candidates: classifier.knownLongNames + classifier.knownShortNames)
  errors.append(
    ParseIssue(
      errorType: "unknown_flag",
      message: "unknown flag \"\(token)\"",
      suggestion: suggestion.map { "Did you mean \"\($0)\"?" }
    ))
}

private func argumentUsage(_ argument: ArgumentDefinition) -> String {
  if argument.required && argument.variadic { return "<\(argument.displayName)>..." }
  if !argument.required && argument.variadic { return "[\(argument.displayName)...]" }
  return argument.required ? "<\(argument.displayName)>" : "[\(argument.displayName)]"
}

private func flagSignature(_ flag: FlagDefinition) -> String {
  var parts: [String] = []
  if let short = flag.shortName { parts.append("-\(short)") }
  if let long = flag.longName { parts.append("--\(long)") }
  if let single = flag.singleDashLong { parts.append("-\(single)") }
  var result = parts.joined(separator: ", ")
  if flag.type.takesValue {
    let valueName = flag.valueName ?? flag.type.rawValue.uppercased()
    result +=
      flag.type == .enumeration && flag.defaultWhenPresent != nil
      ? "[=\(valueName)]" : " <\(valueName)>"
  }
  return result
}

private func flagDescription(_ flag: FlagDefinition) -> String {
  if flag.required { return "\(flag.description) (required)" }
  if flag.defaultValue != .null { return "\(flag.description) [default: \(flag.defaultValue)]" }
  return flag.description
}

private func flagLabel(_ flag: FlagDefinition) -> String {
  var parts: [String] = []
  if let short = flag.shortName { parts.append("-\(short)") }
  if let long = flag.longName { parts.append("--\(long)") }
  if let single = flag.singleDashLong { parts.append("-\(single)") }
  return parts.isEmpty ? flag.id : parts.joined(separator: "/")
}

private func deduplicate(_ flags: [FlagDefinition]) -> [FlagDefinition] {
  var seen: Set<String> = []
  return flags.filter { seen.insert($0.id).inserted }
}

private func fuzzyMatch(_ value: String, candidates: [String]) -> String? {
  var best: String?
  var bestDistance = 3
  for candidate in candidates {
    let distance = levenshtein(value, candidate)
    if distance < bestDistance {
      bestDistance = distance
      best = candidate
    }
  }
  return best
}

private func levenshtein(_ left: String, _ right: String) -> Int {
  let a = Array(left)
  let b = Array(right)
  if a.isEmpty { return b.count }
  if b.isEmpty { return a.count }
  var previous = Array(0...b.count)
  var current = Array(repeating: 0, count: b.count + 1)
  for i in 1...a.count {
    current[0] = i
    for j in 1...b.count {
      if a[i - 1] == b[j - 1] {
        current[j] = previous[j - 1]
      } else {
        current[j] = min(current[j - 1], previous[j], previous[j - 1]) + 1
      }
    }
    swap(&previous, &current)
  }
  return previous[b.count]
}
