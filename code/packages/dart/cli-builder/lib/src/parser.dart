import 'dart:convert';
import 'dart:io';

import 'package:coding_adventures_directed_graph/directed_graph.dart';
import 'package:coding_adventures_state_machine/state_machine.dart' as sm;

import 'errors.dart';
import 'token_classifier.dart';
import 'types.dart';

ValidationResult validateSpec(String jsonString) {
  final decoded = jsonDecode(jsonString) as Map<String, dynamic>;
  return validateSpecObject(decoded);
}

ValidationResult validateSpecObject(Map<String, dynamic> json) {
  final errors = <String>[];
  if (((json['cli_builder_spec_version'] as String?) ?? '1.0') != '1.0') {
    errors.add('unsupported cli_builder_spec_version "${json['cli_builder_spec_version']}"');
  }
  final name = json['name'] as String?;
  if (name == null || name.isEmpty) {
    errors.add('required field "name" is missing or empty');
  }
  final description = json['description'] as String?;
  if (description == null || description.isEmpty) {
    errors.add('required field "description" is missing or empty');
  }

  final spec = CliSpec.fromJson(json);
  final globalFlagIds = spec.globalFlags.map((flag) => flag.id).toSet();
  _validateScope(
    scopeName: 'root',
    flags: <FlagDef>[...spec.flags, ...spec.globalFlags],
    arguments: spec.arguments,
    groups: spec.mutuallyExclusiveGroups,
    additionalVisibleIds: globalFlagIds,
    errors: errors,
  );
  for (final command in spec.commands) {
    _validateCommand(command, globalFlagIds, errors);
  }

  return ValidationResult(errors: List<String>.unmodifiable(errors));
}

class SpecLoader {
  CliSpec loadFromFile(String path) {
    final contents = File(path).readAsStringSync();
    return loadFromString(contents);
  }

  CliSpec loadFromString(String contents) {
    final decoded = jsonDecode(contents) as Map<String, dynamic>;
    return loadFromMap(decoded);
  }

  CliSpec loadFromMap(Map<String, dynamic> json) {
    final validation = validateSpecObject(json);
    if (!validation.isValid) {
      throw SpecError(validation.errors.join('\n'));
    }
    return CliSpec.fromJson(json);
  }
}

class Parser {
  Parser.fromSpec(this.spec, this.argv);

  factory Parser.fromPath(String specPath, List<String> argv) {
    return Parser.fromSpec(SpecLoader().loadFromFile(specPath), argv);
  }

  final CliSpec spec;
  final List<String> argv;

  Object parse() {
    if (argv.isEmpty) {
      throw ParseErrors(<ParseError>[
        ParseError(
          errorType: 'missing_required_argument',
          message: 'argv is empty (no program name)',
        ),
      ]);
    }

    final program = argv.first;
    var tokens = argv.sublist(1);
    if (spec.parsingMode == ParsingMode.traditional &&
        tokens.isNotEmpty &&
        !tokens.first.startsWith('-') &&
        !_knownRootCommandNames().contains(tokens.first)) {
      final expanded = tokens.first.split('').map((char) => '-$char');
      tokens = <String>[...expanded, ...tokens.skip(1)];
    }

    final routed = _phaseRouting(tokens);
    final activeFlags = <FlagDef>[
      ...spec.flagsForPath(routed.commandPath),
      ..._builtinFlags(),
    ];
    final classifier = TokenClassifier(activeFlags);
    final scanned = _phaseScanning(
      tokens: routed.remainingTokens,
      commandPath: routed.commandPath,
      activeFlags: activeFlags,
      classifier: classifier,
    );

    if (scanned.helpRequested) {
      return HelpResult(
        text: _generateHelp(routed.commandPath),
        commandPath: <String>[program, ...routed.commandPath],
      );
    }
    if (scanned.versionRequested) {
      return VersionResult(spec.version ?? '(unknown)');
    }

    final allErrors = <ParseError>[
      ...routed.errors,
      ...scanned.errors,
    ];
    final resolvedArguments = _resolvePositionals(
      spec.argumentsForPath(routed.commandPath),
      scanned.positionals,
      scanned.flags,
      allErrors,
    );
    _validateFlags(
      activeFlags,
      spec.exclusiveGroupsForPath(routed.commandPath),
      scanned.flags,
      allErrors,
    );

    if (allErrors.isNotEmpty) {
      throw ParseErrors(allErrors);
    }

    return ParseResult(
      program: program,
      commandPath: <String>[program, ...routed.commandPath],
      flags: _applyFlagDefaults(activeFlags, scanned.flags),
      arguments: resolvedArguments,
      explicitFlags: scanned.explicitFlags,
    );
  }

  _RoutingResult _phaseRouting(List<String> tokens) {
    final commandPath = <String>[];
    final consumedIndices = <int>{};
    final errors = <ParseError>[];
    List<CommandDef> scope = spec.commands;
    var index = 0;

    while (index < tokens.length) {
      final token = tokens[index];
      if (token == '--') {
        break;
      }
      if (token.startsWith('-')) {
        if (_flagConsumesNextValue(token, spec.flagsForPath(commandPath))) {
          index += 2;
        } else {
          index += 1;
        }
        continue;
      }

      final matched = scope.cast<CommandDef?>().firstWhere(
            (command) => command != null &&
                (command.name == token || command.aliases.contains(token)),
            orElse: () => null,
          );
      if (matched == null) {
        if (spec.parsingMode == ParsingMode.subcommandFirst && commandPath.isEmpty) {
          final suggestion = _fuzzyMatch(token, _knownRootCommandNames().toList());
          errors.add(
            ParseError(
              errorType: 'unknown_command',
              message: 'unknown command "$token"',
              suggestion: suggestion == null ? null : 'Did you mean "$suggestion"?',
            ),
          );
        }
        break;
      }
      commandPath.add(matched.name);
      consumedIndices.add(index);
      scope = matched.commands;
      index += 1;
    }

    final remaining = <String>[];
    for (var i = 0; i < tokens.length; i++) {
      if (!consumedIndices.contains(i)) {
        remaining.add(tokens[i]);
      }
    }

    return _RoutingResult(
      commandPath: List<String>.unmodifiable(commandPath),
      remainingTokens: List<String>.unmodifiable(remaining),
      errors: List<ParseError>.unmodifiable(errors),
    );
  }

  _ScanningResult _phaseScanning({
    required List<String> tokens,
    required List<String> commandPath,
    required List<FlagDef> activeFlags,
    required TokenClassifier classifier,
  }) {
    final flags = <String, Object?>{};
    final positionals = <String>[];
    final errors = <ParseError>[];
    final explicitFlags = <String>[];
    final modeMachine = _newScannerMachine();

    FlagDef? pendingFlag;
    var helpRequested = false;
    var versionRequested = false;
    var index = 0;

    while (index < tokens.length) {
      final mode = modeMachine.currentMode;
      modeMachine.process('token');
      final token = tokens[index];

      if (mode == 'END_OF_FLAGS') {
        positionals.add(token);
        index += 1;
        continue;
      }

      if (mode == 'FLAG_VALUE') {
        if (pendingFlag != null) {
          final flag = pendingFlag;
          if (flag.type == ValueType.enumType &&
              flag.defaultWhenPresent != null &&
              (token.startsWith('-') || !flag.enumValues.contains(token))) {
            _setFlagValue(flags, flag, flag.defaultWhenPresent!, errors);
            explicitFlags.add(flag.id);
            pendingFlag = null;
            modeMachine.switchMode('to_scanning');
            continue;
          }

          final coerced = _coerceValue(token, flag, errors, isFlag: true);
          if (!identical(coerced, _coerceFailed)) {
            _setFlagValue(flags, flag, coerced, errors);
            explicitFlags.add(flag.id);
          }
          pendingFlag = null;
        }
        modeMachine.switchMode('to_scanning');
        index += 1;
        continue;
      }

      final event = classifier.classify(token);
      switch (event.kind) {
        case TokenKind.endOfFlags:
          modeMachine.switchMode('to_end_of_flags');
          break;
        case TokenKind.longFlag:
          final flag = classifier.lookupByLong(event.name!);
          if (flag == null) {
            _unknownFlagError(token, classifier, errors);
            break;
          }
          if (flag.id == 'help') {
            helpRequested = true;
            return _ScanningResult(
              flags: flags,
              positionals: positionals,
              errors: errors,
              explicitFlags: explicitFlags,
              helpRequested: true,
              versionRequested: false,
            );
          }
          if (flag.id == 'version') {
            versionRequested = true;
            return _ScanningResult(
              flags: flags,
              positionals: positionals,
              errors: errors,
              explicitFlags: explicitFlags,
              helpRequested: false,
              versionRequested: true,
            );
          }
          if (flag.type == ValueType.boolean) {
            _setFlagValue(flags, flag, true, errors);
            explicitFlags.add(flag.id);
          } else if (flag.type == ValueType.count) {
            _incrementCount(flags, flag.id);
            explicitFlags.add(flag.id);
          } else {
            pendingFlag = flag;
            modeMachine.switchMode('to_flag_value');
          }
          break;
        case TokenKind.longFlagWithValue:
          final flag = classifier.lookupByLong(event.name!);
          if (flag == null) {
            _unknownFlagError(token, classifier, errors);
            break;
          }
          final coerced = _coerceValue(event.value!, flag, errors, isFlag: true);
          if (!identical(coerced, _coerceFailed)) {
            _setFlagValue(flags, flag, coerced, errors);
            explicitFlags.add(flag.id);
          }
          break;
        case TokenKind.singleDashLong:
          final flag = classifier.lookupBySingleDashLong(event.name!);
          if (flag == null) {
            _unknownFlagError(token, classifier, errors);
            break;
          }
          if (flag.type == ValueType.boolean) {
            _setFlagValue(flags, flag, true, errors);
            explicitFlags.add(flag.id);
          } else if (flag.type == ValueType.count) {
            _incrementCount(flags, flag.id);
            explicitFlags.add(flag.id);
          } else {
            pendingFlag = flag;
            modeMachine.switchMode('to_flag_value');
          }
          break;
        case TokenKind.shortFlag:
          final flag = classifier.lookupByShort(event.name!);
          if (flag == null) {
            _unknownFlagError(token, classifier, errors);
            break;
          }
          if (flag.id == 'help') {
            helpRequested = true;
            return _ScanningResult(
              flags: flags,
              positionals: positionals,
              errors: errors,
              explicitFlags: explicitFlags,
              helpRequested: true,
              versionRequested: false,
            );
          }
          if (flag.type == ValueType.boolean) {
            _setFlagValue(flags, flag, true, errors);
            explicitFlags.add(flag.id);
          } else if (flag.type == ValueType.count) {
            _incrementCount(flags, flag.id);
            explicitFlags.add(flag.id);
          } else {
            pendingFlag = flag;
            modeMachine.switchMode('to_flag_value');
          }
          break;
        case TokenKind.shortFlagWithValue:
          final flag = classifier.lookupByShort(event.name!);
          if (flag == null) {
            _unknownFlagError(token, classifier, errors);
            break;
          }
          final coerced = _coerceValue(event.value!, flag, errors, isFlag: true);
          if (!identical(coerced, _coerceFailed)) {
            _setFlagValue(flags, flag, coerced, errors);
            explicitFlags.add(flag.id);
          }
          break;
        case TokenKind.stackedFlags:
          for (final char in event.chars) {
            final flag = classifier.lookupByShort(char);
            if (flag == null) {
              _unknownFlagError('-$char', classifier, errors);
              continue;
            }
            if (flag.type == ValueType.boolean) {
              _setFlagValue(flags, flag, true, errors);
              explicitFlags.add(flag.id);
            } else if (flag.type == ValueType.count) {
              _incrementCount(flags, flag.id);
              explicitFlags.add(flag.id);
            } else {
              pendingFlag = flag;
              modeMachine.switchMode('to_flag_value');
            }
          }
          break;
        case TokenKind.positional:
          if (spec.parsingMode == ParsingMode.posix) {
            modeMachine.switchMode('to_end_of_flags');
          }
          positionals.add(event.name!);
          break;
        case TokenKind.unknownFlag:
          _unknownFlagError(token, classifier, errors);
          break;
      }

      index += 1;
    }

    if (pendingFlag != null &&
        pendingFlag.type == ValueType.enumType &&
        pendingFlag.defaultWhenPresent != null) {
      _setFlagValue(flags, pendingFlag, pendingFlag.defaultWhenPresent!, errors);
      explicitFlags.add(pendingFlag.id);
    } else if (pendingFlag != null) {
      errors.add(
        ParseError(
          errorType: 'missing_required_argument',
          message: '${_flagLabel(pendingFlag)} expects a value',
          context: commandPath,
        ),
      );
    }

    return _ScanningResult(
      flags: flags,
      positionals: List<String>.unmodifiable(positionals),
      errors: List<ParseError>.unmodifiable(errors),
      explicitFlags: List<String>.unmodifiable(explicitFlags),
      helpRequested: helpRequested,
      versionRequested: versionRequested,
    );
  }

  Map<String, Object?> _resolvePositionals(
    List<ArgDef> argumentDefs,
    List<String> tokens,
    Map<String, Object?> parsedFlags,
    List<ParseError> errors,
  ) {
    final result = <String, Object?>{};
    if (argumentDefs.isEmpty) {
      if (tokens.isNotEmpty) {
        errors.add(
          ParseError(
            errorType: 'too_many_arguments',
            message: 'unexpected positional argument(s): $tokens',
          ),
        );
      }
      return result;
    }

    final variadicIndex = argumentDefs.indexWhere((argument) => argument.variadic);
    if (variadicIndex < 0) {
      for (var index = 0; index < argumentDefs.length; index++) {
        final argument = argumentDefs[index];
        final required = !_isArgumentExempted(argument, parsedFlags) && argument.required;
        if (index < tokens.length) {
          final coerced = _coerceArgument(tokens[index], argument, errors);
          if (!identical(coerced, _coerceFailed)) {
            result[argument.id] = coerced;
          }
        } else if (required) {
          errors.add(
            ParseError(
              errorType: 'missing_required_argument',
              message: 'missing required argument: <${argument.displayName}>',
            ),
          );
        } else {
          result[argument.id] = argument.defaultValue;
        }
      }
      if (tokens.length > argumentDefs.length) {
        errors.add(
          ParseError(
            errorType: 'too_many_arguments',
            message: 'unexpected argument(s): ${tokens.sublist(argumentDefs.length)}',
          ),
        );
      }
      return result;
    }

    final leading = argumentDefs.sublist(0, variadicIndex);
    final variadic = argumentDefs[variadicIndex];
    final trailing = argumentDefs.sublist(variadicIndex + 1);

    for (var index = 0; index < leading.length; index++) {
      final argument = leading[index];
      final required = !_isArgumentExempted(argument, parsedFlags) && argument.required;
      if (index < tokens.length) {
        final coerced = _coerceArgument(tokens[index], argument, errors);
        if (!identical(coerced, _coerceFailed)) {
          result[argument.id] = coerced;
        }
      } else if (required) {
        errors.add(
          ParseError(
            errorType: 'missing_required_argument',
            message: 'missing required argument: <${argument.displayName}>',
          ),
        );
      } else {
        result[argument.id] = argument.defaultValue;
      }
    }

    var trailingStart = tokens.length - trailing.length;
    if (trailingStart < leading.length) {
      trailingStart = leading.length;
    }
    for (var index = 0; index < trailing.length; index++) {
      final argument = trailing[index];
      final required = !_isArgumentExempted(argument, parsedFlags) && argument.required;
      final tokenIndex = trailingStart + index;
      if (tokenIndex < tokens.length) {
        final coerced = _coerceArgument(tokens[tokenIndex], argument, errors);
        if (!identical(coerced, _coerceFailed)) {
          result[argument.id] = coerced;
        }
      } else if (required) {
        errors.add(
          ParseError(
            errorType: 'missing_required_argument',
            message: 'missing required argument: <${argument.displayName}>',
          ),
        );
      } else {
        result[argument.id] = argument.defaultValue;
      }
    }

    final variadicTokens = tokens.sublist(leading.length, trailingStart);
    if (variadicTokens.length < variadic.variadicMin) {
      errors.add(
        ParseError(
          errorType: 'too_few_arguments',
          message: 'expected at least ${variadic.variadicMin} <${variadic.displayName}>, '
              'got ${variadicTokens.length}',
        ),
      );
    }
    if (variadic.variadicMax != null && variadicTokens.length > variadic.variadicMax!) {
      errors.add(
        ParseError(
          errorType: 'too_many_arguments',
          message: 'expected at most ${variadic.variadicMax} <${variadic.displayName}>, '
              'got ${variadicTokens.length}',
        ),
      );
    }

    final values = <Object?>[];
    for (final token in variadicTokens) {
      final coerced = _coerceArgument(token, variadic, errors);
      if (!identical(coerced, _coerceFailed)) {
        values.add(coerced);
      }
    }
    result[variadic.id] = List<Object?>.unmodifiable(values);
    return result;
  }

  void _validateFlags(
    List<FlagDef> activeFlags,
    List<ExclusiveGroup> groups,
    Map<String, Object?> parsedFlags,
    List<ParseError> errors,
  ) {
    final dependencyGraph = Graph();
    final flagById = <String, FlagDef>{};
    for (final flag in activeFlags) {
      dependencyGraph.addNode(flag.id);
      flagById[flag.id] = flag;
    }
    for (final flag in activeFlags) {
      for (final required in flag.requires) {
        if (flagById.containsKey(required)) {
          dependencyGraph.addEdge(flag.id, required);
        }
      }
    }

    for (final flag in activeFlags) {
      final present = _isPresent(parsedFlags[flag.id]);
      if (!present) {
        final exempted = flag.requiredUnless.any((id) => _isPresent(parsedFlags[id]));
        if (flag.required && !exempted) {
          errors.add(
            ParseError(
              errorType: 'missing_required_flag',
              message: '${_flagLabel(flag)} is required',
            ),
          );
        }
        continue;
      }

      for (final otherId in flag.conflictsWith) {
        if (_isPresent(parsedFlags[otherId])) {
          errors.add(
            ParseError(
              errorType: 'conflicting_flags',
              message: '${_flagLabel(flag)} and ${_flagLabel(flagById[otherId]!)} cannot be used together',
            ),
          );
        }
      }

      if (dependencyGraph.hasNode(flag.id)) {
        for (final required in dependencyGraph.transitiveClosure(flag.id)) {
          if (!_isPresent(parsedFlags[required])) {
            errors.add(
              ParseError(
                errorType: 'missing_dependency_flag',
                message: '${_flagLabel(flag)} requires ${_flagLabel(flagById[required]!)}',
              ),
            );
          }
        }
      }
    }

    for (final group in groups) {
      final present = group.flagIds.where((id) => _isPresent(parsedFlags[id])).toList();
      if (present.length > 1) {
        errors.add(
          ParseError(
            errorType: 'exclusive_group_violation',
            message: 'only one of '
                '${present.map((id) => _flagLabel(flagById[id]!)).join(", ")} may be used',
          ),
        );
      } else if (group.required && present.isEmpty) {
        errors.add(
          ParseError(
            errorType: 'missing_exclusive_group',
            message: 'one of ${group.flagIds.map((id) => _flagLabel(flagById[id]!)).join(", ")} is required',
          ),
        );
      }
    }
  }

  Map<String, Object?> _applyFlagDefaults(
    List<FlagDef> activeFlags,
    Map<String, Object?> parsedFlags,
  ) {
    final result = Map<String, Object?>.from(parsedFlags);
    for (final flag in activeFlags) {
      result.putIfAbsent(flag.id, () {
        return switch (flag.type) {
          ValueType.boolean => false,
          ValueType.count => 0,
          _ => flag.defaultValue,
        };
      });
    }
    return Map<String, Object?>.unmodifiable(result);
  }

  String _generateHelp(List<String> commandPath) {
    final node = spec.findCommand(commandPath);
    final isRoot = commandPath.isEmpty;
    final description = isRoot ? spec.description : node?.description ?? spec.description;
    final commands = isRoot ? spec.commands : node?.commands ?? const <CommandDef>[];
    final localFlags = isRoot ? spec.flags : node?.flags ?? const <FlagDef>[];
    final arguments = isRoot ? spec.arguments : node?.arguments ?? const <ArgDef>[];
    final globalFlags = <FlagDef>[...spec.globalFlags, ..._builtinFlags()];

    final buffer = StringBuffer();
    buffer.writeln('USAGE');
    buffer.writeln('  ${_usageLine(commandPath, localFlags, commands, arguments)}');
    buffer.writeln();
    buffer.writeln('DESCRIPTION');
    buffer.writeln('  $description');
    if (commands.isNotEmpty) {
      buffer.writeln();
      buffer.writeln('COMMANDS');
      for (final command in commands) {
        buffer.writeln('  ${command.name.padRight(16)}${command.description}');
      }
    }
    if (localFlags.isNotEmpty) {
      buffer.writeln();
      buffer.writeln('OPTIONS');
      for (final flag in localFlags) {
        buffer.writeln('  ${_flagSignature(flag).padRight(28)}${_flagDescription(flag)}');
      }
    }
    if (globalFlags.isNotEmpty) {
      buffer.writeln();
      buffer.writeln('GLOBAL OPTIONS');
      for (final flag in globalFlags) {
        buffer.writeln('  ${_flagSignature(flag).padRight(28)}${_flagDescription(flag)}');
      }
    }
    if (arguments.isNotEmpty) {
      buffer.writeln();
      buffer.writeln('ARGUMENTS');
      for (final argument in arguments) {
        buffer.writeln(
          '  ${_argumentUsageToken(argument).padRight(16)}'
          '${argument.description}${argument.required ? ' Required.' : ''}',
        );
      }
    }

    return buffer.toString().trimRight();
  }

  String _usageLine(
    List<String> commandPath,
    List<FlagDef> localFlags,
    List<CommandDef> commands,
    List<ArgDef> arguments,
  ) {
    final parts = <String>[spec.name, ...commandPath];
    if (localFlags.isNotEmpty || spec.globalFlags.isNotEmpty) {
      parts.add('[OPTIONS]');
    }
    if (commands.isNotEmpty) {
      parts.add('[COMMAND]');
    }
    parts.addAll(arguments.map(_argumentUsageToken));
    return parts.join(' ');
  }

  List<FlagDef> _builtinFlags() {
    final result = <FlagDef>[];
    if (spec.builtinFlags.help) {
      result.add(
        const FlagDef(
          id: 'help',
          shortName: 'h',
          longName: 'help',
          singleDashLong: null,
          description: 'Show this help message and exit.',
          type: ValueType.boolean,
          required: false,
          defaultValue: false,
          valueName: null,
          enumValues: <String>[],
          defaultWhenPresent: null,
          conflictsWith: <String>[],
          requires: <String>[],
          requiredUnless: <String>[],
          repeatable: false,
        ),
      );
    }
    if (spec.builtinFlags.version) {
      result.add(
        const FlagDef(
          id: 'version',
          shortName: null,
          longName: 'version',
          singleDashLong: null,
          description: 'Show version and exit.',
          type: ValueType.boolean,
          required: false,
          defaultValue: false,
          valueName: null,
          enumValues: <String>[],
          defaultWhenPresent: null,
          conflictsWith: <String>[],
          requires: <String>[],
          requiredUnless: <String>[],
          repeatable: false,
        ),
      );
    }
    return result;
  }

  sm.ModalStateMachine _newScannerMachine() {
    sm.DFA makeMode(String name) {
      return sm.DFA(
        <String>{name},
        <String>{'token'},
        <String, String>{sm.transitionKey(name, 'token'): name},
        name,
        <String>{name},
      );
    }

    return sm.ModalStateMachine(
      <String, sm.DFA>{
        'SCANNING': makeMode('SCANNING'),
        'FLAG_VALUE': makeMode('FLAG_VALUE'),
        'END_OF_FLAGS': makeMode('END_OF_FLAGS'),
      },
      <String, String>{
        sm.transitionKey('SCANNING', 'to_flag_value'): 'FLAG_VALUE',
        sm.transitionKey('SCANNING', 'to_end_of_flags'): 'END_OF_FLAGS',
        sm.transitionKey('FLAG_VALUE', 'to_scanning'): 'SCANNING',
      },
      'SCANNING',
    );
  }

  void _unknownFlagError(String token, TokenClassifier classifier, List<ParseError> errors) {
    final known = <String>[
      ...classifier.knownLongNames(),
      ...classifier.knownShortNames(),
    ];
    final suggestion = _fuzzyMatch(token, known);
    errors.add(
      ParseError(
        errorType: 'unknown_flag',
        message: 'unknown flag "$token"',
        suggestion: suggestion == null ? null : 'Did you mean "$suggestion"?',
      ),
    );
  }

  bool _flagConsumesNextValue(String token, List<FlagDef> flags) {
    final classifier = TokenClassifier(flags);
    final event = classifier.classify(token);
    return switch (event.kind) {
      TokenKind.longFlag => classifier.lookupByLong(event.name!)?.type != ValueType.boolean &&
          classifier.lookupByLong(event.name!)?.type != ValueType.count,
      TokenKind.shortFlag => classifier.lookupByShort(event.name!)?.type != ValueType.boolean &&
          classifier.lookupByShort(event.name!)?.type != ValueType.count,
      TokenKind.singleDashLong =>
        classifier.lookupBySingleDashLong(event.name!)?.type != ValueType.boolean &&
            classifier.lookupBySingleDashLong(event.name!)?.type != ValueType.count,
      _ => false,
    };
  }

  Set<String> _knownRootCommandNames() {
    return <String>{
      for (final command in spec.commands) command.name,
      for (final command in spec.commands) ...command.aliases,
    };
  }
}

void _validateCommand(CommandDef command, Set<String> globalFlagIds, List<String> errors) {
  _validateScope(
    scopeName: 'command "${command.id}"',
    flags: <FlagDef>[...command.flags],
    arguments: command.arguments,
    groups: command.mutuallyExclusiveGroups,
    additionalVisibleIds: globalFlagIds,
    errors: errors,
  );
  final names = <String>{};
  for (final nested in command.commands) {
    if (!names.add(nested.name)) {
      errors.add('duplicate command name "${nested.name}"');
    }
    for (final alias in nested.aliases) {
      if (!names.add(alias)) {
        errors.add('duplicate command alias "$alias"');
      }
    }
    _validateCommand(nested, globalFlagIds, errors);
  }
}

void _validateScope({
  required String scopeName,
  required List<FlagDef> flags,
  required List<ArgDef> arguments,
  required List<ExclusiveGroup> groups,
  required Set<String> additionalVisibleIds,
  required List<String> errors,
}) {
  final localIds = <String>{};
  final visibleIds = <String>{...additionalVisibleIds};
  for (final flag in flags) {
    if (!localIds.add(flag.id)) {
      errors.add('in $scopeName: duplicate flag id "${flag.id}"');
    }
    visibleIds.add(flag.id);
    if (flag.shortName == null && flag.longName == null && flag.singleDashLong == null) {
      errors.add('in $scopeName: flag "${flag.id}" must declare short, long, or single_dash_long');
    }
    if (flag.type == ValueType.enumType && flag.enumValues.isEmpty) {
      errors.add('in $scopeName: flag "${flag.id}" has type enum but no enum_values');
    }
    if (flag.defaultWhenPresent != null && flag.type != ValueType.enumType) {
      errors.add('in $scopeName: flag "${flag.id}" has default_when_present but is not an enum');
    }
  }
  for (final flag in flags) {
    for (final other in <String>[...flag.conflictsWith, ...flag.requires]) {
      if (!visibleIds.contains(other)) {
        errors.add('in $scopeName: flag "${flag.id}" references unknown flag id "$other"');
      }
    }
  }

  final graph = Graph();
  for (final flag in flags) {
    graph.addNode(flag.id);
  }
  for (final flag in flags) {
    for (final required in flag.requires) {
      if (localIds.contains(required) || additionalVisibleIds.contains(required)) {
        graph.addEdge(flag.id, required);
      }
    }
  }
  if (graph.hasCycle()) {
    errors.add('in $scopeName: circular requires dependency detected');
  }

  final argIds = <String>{};
  var variadicCount = 0;
  for (final argument in arguments) {
    if (!argIds.add(argument.id)) {
      errors.add('in $scopeName: duplicate argument id "${argument.id}"');
    }
    if (argument.type == ValueType.enumType && argument.enumValues.isEmpty) {
      errors.add('in $scopeName: argument "${argument.id}" has type enum but no enum_values');
    }
    if (argument.variadic) {
      variadicCount += 1;
    }
  }
  if (variadicCount > 1) {
    errors.add('in $scopeName: at most one argument may be variadic');
  }

  for (final group in groups) {
    for (final id in group.flagIds) {
      if (!visibleIds.contains(id)) {
        errors.add('in $scopeName: mutually exclusive group "${group.id}" references unknown flag id "$id"');
      }
    }
  }
}

Object _coerceValue(String raw, FlagDef flag, List<ParseError> errors, {required bool isFlag}) {
  try {
    return switch (flag.type) {
      ValueType.boolean => raw == 'true',
      ValueType.count => int.parse(raw),
      ValueType.string => raw,
      ValueType.integer => int.parse(raw),
      ValueType.float => double.parse(raw),
      ValueType.path => raw,
      ValueType.file => File(raw).existsSync() ? raw : throw SpecError('file does not exist: $raw'),
      ValueType.directory => Directory(raw).existsSync()
          ? raw
          : throw SpecError('directory does not exist: $raw'),
      ValueType.enumType => flag.enumValues.contains(raw) ? raw : throw SpecError('invalid enum value'),
    };
  } catch (_) {
    errors.add(
      ParseError(
        errorType: flag.type == ValueType.enumType ? 'invalid_enum_value' : 'invalid_value',
        message: isFlag
            ? 'invalid ${valueTypeName(flag.type)} for ${_flagLabel(flag)}: "$raw"'
            : 'invalid ${valueTypeName(flag.type)} value "$raw"',
      ),
    );
    return _coerceFailed;
  }
}

Object _coerceArgument(String raw, ArgDef argument, List<ParseError> errors) {
  try {
    return switch (argument.type) {
      ValueType.boolean => raw == 'true',
      ValueType.count => int.parse(raw),
      ValueType.string => raw,
      ValueType.integer => int.parse(raw),
      ValueType.float => double.parse(raw),
      ValueType.path => raw,
      ValueType.file => File(raw).existsSync() ? raw : throw SpecError('file does not exist: $raw'),
      ValueType.directory => Directory(raw).existsSync()
          ? raw
          : throw SpecError('directory does not exist: $raw'),
      ValueType.enumType =>
        argument.enumValues.contains(raw) ? raw : throw SpecError('invalid enum value'),
    };
  } catch (_) {
    errors.add(
      ParseError(
        errorType: argument.type == ValueType.enumType ? 'invalid_enum_value' : 'invalid_value',
        message: 'invalid ${valueTypeName(argument.type)} for argument <${argument.displayName}>: "$raw"',
      ),
    );
    return _coerceFailed;
  }
}

void _setFlagValue(Map<String, Object?> flags, FlagDef flag, Object value, List<ParseError> errors) {
  if (flag.repeatable) {
    final existing = flags[flag.id];
    if (existing is List<Object?>) {
      existing.add(value);
    } else if (existing != null) {
      flags[flag.id] = <Object?>[existing, value];
    } else {
      flags[flag.id] = <Object?>[value];
    }
    return;
  }
  if (flags.containsKey(flag.id)) {
    errors.add(
      ParseError(
        errorType: 'duplicate_flag',
        message: '${_flagLabel(flag)} specified more than once',
      ),
    );
    return;
  }
  flags[flag.id] = value;
}

void _incrementCount(Map<String, Object?> flags, String id) {
  flags[id] = ((flags[id] as int?) ?? 0) + 1;
}

bool _isPresent(Object? value) {
  if (value == null) return false;
  if (value is bool) return value;
  if (value is int) return value != 0;
  if (value is List) return value.isNotEmpty;
  return true;
}

bool _isArgumentExempted(ArgDef argument, Map<String, Object?> flags) {
  return argument.requiredUnlessFlag.any((id) => _isPresent(flags[id]));
}

String _argumentUsageToken(ArgDef argument) {
  if (argument.required && argument.variadic) return '<${argument.displayName}>...';
  if (!argument.required && argument.variadic) return '[${argument.displayName}...]';
  if (argument.required) return '<${argument.displayName}>';
  return '[${argument.displayName}]';
}

String _flagSignature(FlagDef flag) {
  final parts = <String>[];
  if (flag.shortName != null) parts.add('-${flag.shortName}');
  if (flag.longName != null) parts.add('--${flag.longName}');
  if (flag.singleDashLong != null) parts.add('-${flag.singleDashLong}');
  var result = parts.join(', ');
  if (flag.type != ValueType.boolean && flag.type != ValueType.count) {
    final valueName = flag.valueName ?? valueTypeName(flag.type).toUpperCase();
    result += flag.type == ValueType.enumType && flag.defaultWhenPresent != null
        ? '[=$valueName]'
        : ' <$valueName>';
  }
  return result;
}

String _flagDescription(FlagDef flag) {
  final buffer = StringBuffer(flag.description);
  if (flag.required) {
    buffer.write(' (required)');
  } else if (flag.defaultValue != null) {
    buffer.write(' [default: ${flag.defaultValue}]');
  }
  return buffer.toString();
}

String _flagLabel(FlagDef flag) {
  final parts = <String>[];
  if (flag.shortName != null) parts.add('-${flag.shortName}');
  if (flag.longName != null) parts.add('--${flag.longName}');
  if (flag.singleDashLong != null) parts.add('-${flag.singleDashLong}');
  return parts.isEmpty ? flag.id : parts.join('/');
}

String? _fuzzyMatch(String unknown, List<String> candidates) {
  String? best;
  var bestDistance = 3;
  for (final candidate in candidates) {
    final distance = _levenshtein(unknown, candidate);
    if (distance < bestDistance) {
      bestDistance = distance;
      best = candidate;
    }
  }
  return best;
}

int _levenshtein(String left, String right) {
  final a = left.runes.toList();
  final b = right.runes.toList();
  if (a.isEmpty) return b.length;
  if (b.isEmpty) return a.length;
  var previous = List<int>.generate(b.length + 1, (index) => index);
  var current = List<int>.filled(b.length + 1, 0);
  for (var i = 1; i <= a.length; i++) {
    current[0] = i;
    for (var j = 1; j <= b.length; j++) {
      if (a[i - 1] == b[j - 1]) {
        current[j] = previous[j - 1];
      } else {
        final insert = current[j - 1] + 1;
        final delete = previous[j] + 1;
        final replace = previous[j - 1] + 1;
        current[j] = [insert, delete, replace].reduce((x, y) => x < y ? x : y);
      }
    }
    final swap = previous;
    previous = current;
    current = swap;
  }
  return previous.last;
}

class _RoutingResult {
  const _RoutingResult({
    required this.commandPath,
    required this.remainingTokens,
    required this.errors,
  });

  final List<String> commandPath;
  final List<String> remainingTokens;
  final List<ParseError> errors;
}

class _ScanningResult {
  const _ScanningResult({
    required this.flags,
    required this.positionals,
    required this.errors,
    required this.explicitFlags,
    required this.helpRequested,
    required this.versionRequested,
  });

  final Map<String, Object?> flags;
  final List<String> positionals;
  final List<ParseError> errors;
  final List<String> explicitFlags;
  final bool helpRequested;
  final bool versionRequested;
}

const Object _coerceFailed = Object();
