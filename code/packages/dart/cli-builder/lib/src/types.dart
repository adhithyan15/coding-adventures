enum ValueType {
  boolean,
  count,
  string,
  integer,
  float,
  path,
  file,
  directory,
  enumType,
}

enum ParsingMode {
  gnu,
  posix,
  subcommandFirst,
  traditional,
}

ValueType valueTypeFromString(String value) {
  return switch (value) {
    'boolean' => ValueType.boolean,
    'count' => ValueType.count,
    'string' => ValueType.string,
    'integer' => ValueType.integer,
    'float' => ValueType.float,
    'path' => ValueType.path,
    'file' => ValueType.file,
    'directory' => ValueType.directory,
    'enum' => ValueType.enumType,
    _ => throw ArgumentError('Unknown value type: $value'),
  };
}

String valueTypeName(ValueType valueType) {
  return switch (valueType) {
    ValueType.boolean => 'boolean',
    ValueType.count => 'count',
    ValueType.string => 'string',
    ValueType.integer => 'integer',
    ValueType.float => 'float',
    ValueType.path => 'path',
    ValueType.file => 'file',
    ValueType.directory => 'directory',
    ValueType.enumType => 'enum',
  };
}

ParsingMode parsingModeFromString(String value) {
  return switch (value) {
    'gnu' => ParsingMode.gnu,
    'posix' => ParsingMode.posix,
    'subcommand_first' => ParsingMode.subcommandFirst,
    'traditional' => ParsingMode.traditional,
    _ => ParsingMode.gnu,
  };
}

class BuiltinFlags {
  const BuiltinFlags({
    required this.help,
    required this.version,
  });

  final bool help;
  final bool version;
}

class FlagDef {
  const FlagDef({
    required this.id,
    required this.description,
    required this.type,
    required this.required,
    required this.defaultValue,
    required this.enumValues,
    required this.conflictsWith,
    required this.requires,
    required this.requiredUnless,
    required this.repeatable,
    this.shortName,
    this.longName,
    this.singleDashLong,
    this.valueName,
    this.defaultWhenPresent,
  });

  final String id;
  final String? shortName;
  final String? longName;
  final String? singleDashLong;
  final String description;
  final ValueType type;
  final bool required;
  final Object? defaultValue;
  final String? valueName;
  final List<String> enumValues;
  final String? defaultWhenPresent;
  final List<String> conflictsWith;
  final List<String> requires;
  final List<String> requiredUnless;
  final bool repeatable;

  factory FlagDef.fromJson(Map<String, dynamic> json) {
    return FlagDef(
      id: json['id'] as String,
      shortName: json['short'] as String?,
      longName: json['long'] as String?,
      singleDashLong: json['single_dash_long'] as String?,
      description: (json['description'] as String?) ?? '',
      type: valueTypeFromString((json['type'] as String?) ?? 'string'),
      required: (json['required'] as bool?) ?? false,
      defaultValue: json['default'],
      valueName: json['value_name'] as String?,
      enumValues: _stringList(json['enum_values']),
      defaultWhenPresent: json['default_when_present'] as String?,
      conflictsWith: _stringList(json['conflicts_with']),
      requires: _stringList(json['requires']),
      requiredUnless: _stringList(json['required_unless']),
      repeatable: (json['repeatable'] as bool?) ?? false,
    );
  }
}

class ArgDef {
  const ArgDef({
    required this.id,
    required this.displayName,
    required this.description,
    required this.type,
    required this.required,
    required this.variadic,
    required this.variadicMin,
    required this.variadicMax,
    required this.defaultValue,
    required this.enumValues,
    required this.requiredUnlessFlag,
  });

  final String id;
  final String displayName;
  final String description;
  final ValueType type;
  final bool required;
  final bool variadic;
  final int variadicMin;
  final int? variadicMax;
  final Object? defaultValue;
  final List<String> enumValues;
  final List<String> requiredUnlessFlag;

  factory ArgDef.fromJson(Map<String, dynamic> json) {
    return ArgDef(
      id: json['id'] as String,
      displayName: (json['display_name'] as String?) ?? (json['name'] as String?) ?? '',
      description: (json['description'] as String?) ?? '',
      type: valueTypeFromString((json['type'] as String?) ?? 'string'),
      required: (json['required'] as bool?) ?? true,
      variadic: (json['variadic'] as bool?) ?? false,
      variadicMin: ((json['variadic_min'] as num?)?.toInt()) ??
          (((json['required'] as bool?) ?? true) ? 1 : 0),
      variadicMax: (json['variadic_max'] as num?)?.toInt(),
      defaultValue: json['default'],
      enumValues: _stringList(json['enum_values']),
      requiredUnlessFlag: _stringList(json['required_unless_flag']),
    );
  }
}

class ExclusiveGroup {
  const ExclusiveGroup({
    required this.id,
    required this.flagIds,
    required this.required,
  });

  final String id;
  final List<String> flagIds;
  final bool required;

  factory ExclusiveGroup.fromJson(Map<String, dynamic> json) {
    return ExclusiveGroup(
      id: json['id'] as String,
      flagIds: _stringList(json['flag_ids']),
      required: (json['required'] as bool?) ?? false,
    );
  }
}

class CommandDef {
  const CommandDef({
    required this.id,
    required this.name,
    required this.aliases,
    required this.description,
    required this.inheritGlobalFlags,
    required this.flags,
    required this.arguments,
    required this.commands,
    required this.mutuallyExclusiveGroups,
  });

  final String id;
  final String name;
  final List<String> aliases;
  final String description;
  final bool inheritGlobalFlags;
  final List<FlagDef> flags;
  final List<ArgDef> arguments;
  final List<CommandDef> commands;
  final List<ExclusiveGroup> mutuallyExclusiveGroups;

  factory CommandDef.fromJson(Map<String, dynamic> json) {
    return CommandDef(
      id: json['id'] as String,
      name: json['name'] as String,
      aliases: _stringList(json['aliases']),
      description: (json['description'] as String?) ?? '',
      inheritGlobalFlags: (json['inherit_global_flags'] as bool?) ?? true,
      flags: _mapList(json['flags'], FlagDef.fromJson),
      arguments: _mapList(json['arguments'], ArgDef.fromJson),
      commands: _mapList(json['commands'], CommandDef.fromJson),
      mutuallyExclusiveGroups:
          _mapList(json['mutually_exclusive_groups'], ExclusiveGroup.fromJson),
    );
  }

  CommandDef? findCommand(String token) {
    for (final command in commands) {
      if (command.name == token || command.aliases.contains(token)) {
        return command;
      }
    }
    return null;
  }
}

class CliSpec {
  const CliSpec({
    required this.specVersion,
    required this.name,
    required this.description,
    required this.parsingMode,
    required this.builtinFlags,
    required this.globalFlags,
    required this.flags,
    required this.arguments,
    required this.commands,
    required this.mutuallyExclusiveGroups,
    this.displayName,
    this.version,
  });

  final String specVersion;
  final String name;
  final String? displayName;
  final String description;
  final String? version;
  final ParsingMode parsingMode;
  final BuiltinFlags builtinFlags;
  final List<FlagDef> globalFlags;
  final List<FlagDef> flags;
  final List<ArgDef> arguments;
  final List<CommandDef> commands;
  final List<ExclusiveGroup> mutuallyExclusiveGroups;

  factory CliSpec.fromJson(Map<String, dynamic> json) {
    return CliSpec(
      specVersion:
          (json['cli_builder_spec_version'] as String?) ??
          (json['spec_version'] as String?) ??
          '1.0',
      name: json['name'] as String,
      displayName: json['display_name'] as String?,
      description: (json['description'] as String?) ?? '',
      version: json['version'] as String?,
      parsingMode: parsingModeFromString((json['parsing_mode'] as String?) ?? 'gnu'),
      builtinFlags: BuiltinFlags(
        help: (json['builtin_flags'] as Map<String, dynamic>?)?['help'] as bool? ?? true,
        version:
            (json['builtin_flags'] as Map<String, dynamic>?)?['version'] as bool? ??
            (json['version'] != null),
      ),
      globalFlags: _mapList(json['global_flags'], FlagDef.fromJson),
      flags: _mapList(json['flags'], FlagDef.fromJson),
      arguments: _mapList(json['arguments'], ArgDef.fromJson),
      commands: _mapList(json['commands'], CommandDef.fromJson),
      mutuallyExclusiveGroups:
          _mapList(json['mutually_exclusive_groups'], ExclusiveGroup.fromJson),
    );
  }

  CommandDef? findCommand(List<String> path) {
    if (path.isEmpty) {
      return null;
    }
    List<CommandDef> scope = commands;
    CommandDef? current;
    for (final token in path) {
      current = scope.where((command) {
        return command.name == token || command.aliases.contains(token);
      }).cast<CommandDef?>().firstWhere(
            (command) => command != null,
            orElse: () => null,
          );
      if (current == null) {
        return null;
      }
      scope = current.commands;
    }
    return current;
  }

  List<FlagDef> flagsForPath(List<String> path) {
    if (path.isEmpty) {
      return List<FlagDef>.unmodifiable(<FlagDef>[...flags, ...globalFlags]);
    }
    final command = findCommand(path);
    if (command == null) {
      return List<FlagDef>.unmodifiable(<FlagDef>[...flags, ...globalFlags]);
    }
    final result = <FlagDef>[...command.flags];
    if (command.inheritGlobalFlags) {
      result.addAll(globalFlags);
    }
    return List<FlagDef>.unmodifiable(result);
  }

  List<ArgDef> argumentsForPath(List<String> path) {
    if (path.isEmpty) {
      return arguments;
    }
    return findCommand(path)?.arguments ?? arguments;
  }

  List<ExclusiveGroup> exclusiveGroupsForPath(List<String> path) {
    if (path.isEmpty) {
      return mutuallyExclusiveGroups;
    }
    return findCommand(path)?.mutuallyExclusiveGroups ?? mutuallyExclusiveGroups;
  }
}

class ParseResult {
  const ParseResult({
    required this.program,
    required this.commandPath,
    required this.flags,
    required this.arguments,
    required this.explicitFlags,
  });

  final String program;
  final List<String> commandPath;
  final Map<String, Object?> flags;
  final Map<String, Object?> arguments;
  final List<String> explicitFlags;
}

class HelpResult {
  const HelpResult({
    required this.text,
    required this.commandPath,
  });

  final String text;
  final List<String> commandPath;
}

class VersionResult {
  const VersionResult(this.version);

  final String version;
}

typedef ParserResult = Object;

List<T> _mapList<T>(
  Object? value,
  T Function(Map<String, dynamic>) mapper,
) {
  final raw = value as List<dynamic>? ?? const <dynamic>[];
  return List<T>.unmodifiable(
    raw
        .map((item) => mapper(Map<String, dynamic>.from(item as Map)))
        .toList(),
  );
}

List<String> _stringList(Object? value) {
  final raw = value as List<dynamic>? ?? const <dynamic>[];
  return List<String>.unmodifiable(raw.cast<String>());
}
