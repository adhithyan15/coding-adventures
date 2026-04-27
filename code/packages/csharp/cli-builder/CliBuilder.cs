namespace CodingAdventures.CliBuilder;

using System.Globalization;
using System.Text;
using CodingAdventures.DirectedGraph;
using JsonNode = CodingAdventures.JsonValue.JsonValue;

public enum ValueType
{
    Boolean,
    Count,
    String,
    Integer,
    Float,
    Path,
    File,
    Directory,
    Enum,
}

public enum ParsingMode
{
    Gnu,
    Posix,
    SubcommandFirst,
    Traditional,
}

public sealed record BuiltinFlags(bool Help, bool Version);

public sealed record FlagDef(
    string Id,
    string? Short,
    string? Long,
    string? SingleDashLong,
    string Description,
    ValueType Type,
    bool Required,
    object? Default,
    string? ValueName,
    IReadOnlyList<string> EnumValues,
    string? DefaultWhenPresent,
    IReadOnlyList<string> ConflictsWith,
    IReadOnlyList<string> Requires,
    IReadOnlyList<string> RequiredUnless,
    bool Repeatable);

public sealed record ArgDef(
    string Id,
    string DisplayName,
    string Description,
    ValueType Type,
    bool Required,
    bool Variadic,
    int VariadicMin,
    int? VariadicMax,
    object? Default,
    IReadOnlyList<string> EnumValues,
    IReadOnlyList<string> RequiredUnlessFlag);

public sealed record ExclusiveGroup(string Id, IReadOnlyList<string> FlagIds, bool Required);

public sealed record CommandDef(
    string Id,
    string Name,
    IReadOnlyList<string> Aliases,
    string Description,
    bool InheritGlobalFlags,
    IReadOnlyList<FlagDef> Flags,
    IReadOnlyList<ArgDef> Arguments,
    IReadOnlyList<CommandDef> Commands,
    IReadOnlyList<ExclusiveGroup> MutuallyExclusiveGroups);

public sealed record CliSpec(
    string SpecVersion,
    string Name,
    string? DisplayName,
    string Description,
    string? Version,
    ParsingMode ParsingMode,
    BuiltinFlags BuiltinFlags,
    IReadOnlyList<FlagDef> GlobalFlags,
    IReadOnlyList<FlagDef> Flags,
    IReadOnlyList<ArgDef> Arguments,
    IReadOnlyList<CommandDef> Commands,
    IReadOnlyList<ExclusiveGroup> MutuallyExclusiveGroups);

public sealed record ValidationResult(bool IsValid, IReadOnlyList<string> Errors);

public sealed record ParseError(string ErrorType, string Message, string? Suggestion, IReadOnlyList<string> Context);

public abstract record ParserResult;

public sealed record ParseResult(
    string Program,
    IReadOnlyList<string> CommandPath,
    IReadOnlyDictionary<string, object?> Flags,
    IReadOnlyDictionary<string, object?> Arguments,
    IReadOnlyList<string> ExplicitFlags) : ParserResult;

public sealed record HelpResult(string Text, IReadOnlyList<string> CommandPath) : ParserResult;

public sealed record VersionResult(string Version) : ParserResult;

public class CliBuilderError : Exception
{
    public CliBuilderError(string message) : base(message)
    {
    }
}

public sealed class SpecError : CliBuilderError
{
    public SpecError(string message) : base(message)
    {
    }
}

public sealed class ParseErrors : CliBuilderError
{
    public ParseErrors(IReadOnlyList<ParseError> errors)
        : base(errors.Count == 1 ? errors[0].Message : $"{errors.Count} parse errors:\n  - {string.Join("\n  - ", errors.Select(error => error.Message))}")
    {
        Errors = errors;
    }

    public IReadOnlyList<ParseError> Errors { get; }
}

public enum TokenEventType
{
    EndOfFlags,
    LongFlag,
    LongFlagWithValue,
    SingleDashLong,
    ShortFlag,
    ShortFlagWithValue,
    StackedFlags,
    Positional,
    UnknownFlag,
}

public abstract record TokenEvent(TokenEventType Type);

public sealed record EndOfFlagsToken() : TokenEvent(TokenEventType.EndOfFlags);

public sealed record LongFlagToken(string Name) : TokenEvent(TokenEventType.LongFlag);

public sealed record LongFlagWithValueToken(string Name, string Value) : TokenEvent(TokenEventType.LongFlagWithValue);

public sealed record SingleDashLongToken(string Name) : TokenEvent(TokenEventType.SingleDashLong);

public sealed record ShortFlagToken(string Char) : TokenEvent(TokenEventType.ShortFlag);

public sealed record ShortFlagWithValueToken(string Char, string Value) : TokenEvent(TokenEventType.ShortFlagWithValue);

public sealed record StackedFlagsToken(IReadOnlyList<string> Chars) : TokenEvent(TokenEventType.StackedFlags);

public sealed record PositionalToken(string Value) : TokenEvent(TokenEventType.Positional);

public sealed record UnknownFlagToken(string Raw) : TokenEvent(TokenEventType.UnknownFlag);

public sealed class TokenClassifier
{
    private readonly Dictionary<string, FlagDef> _shortMap = new(StringComparer.Ordinal);
    private readonly Dictionary<string, FlagDef> _longMap = new(StringComparer.Ordinal);
    private readonly Dictionary<string, FlagDef> _singleDashLongMap = new(StringComparer.Ordinal);

    public TokenClassifier(IEnumerable<FlagDef> activeFlags)
    {
        foreach (var flag in activeFlags)
        {
            if (!string.IsNullOrEmpty(flag.Short) && !_shortMap.ContainsKey(flag.Short))
            {
                _shortMap[flag.Short] = flag;
            }

            if (!string.IsNullOrEmpty(flag.Long) && !_longMap.ContainsKey(flag.Long))
            {
                _longMap[flag.Long] = flag;
            }

            if (!string.IsNullOrEmpty(flag.SingleDashLong) && !_singleDashLongMap.ContainsKey(flag.SingleDashLong))
            {
                _singleDashLongMap[flag.SingleDashLong] = flag;
            }
        }
    }

    public TokenEvent Classify(string token)
    {
        if (token == "--")
        {
            return new EndOfFlagsToken();
        }

        if (token.StartsWith("--", StringComparison.Ordinal))
        {
            var rest = token[2..];
            var separatorIndex = rest.IndexOf('=');
            return separatorIndex >= 0
                ? new LongFlagWithValueToken(rest[..separatorIndex], rest[(separatorIndex + 1)..])
                : new LongFlagToken(rest);
        }

        if (token == "-")
        {
            return new PositionalToken(token);
        }

        if (token.StartsWith("-", StringComparison.Ordinal) && token.Length > 1)
        {
            return ClassifySingleDash(token);
        }

        return new PositionalToken(token);
    }

    private TokenEvent ClassifySingleDash(string token)
    {
        var rest = token[1..];
        if (_singleDashLongMap.ContainsKey(rest))
        {
            return new SingleDashLongToken(rest);
        }

        var first = rest[..1];
        if (_shortMap.TryGetValue(first, out var flag))
        {
            var remainder = rest[1..];
            var consumesNoValue = flag.Type is ValueType.Boolean or ValueType.Count;
            if (consumesNoValue)
            {
                if (remainder.Length == 0)
                {
                    return new ShortFlagToken(first);
                }

                return ClassifyStack(rest);
            }

            return remainder.Length == 0
                ? new ShortFlagToken(first)
                : new ShortFlagWithValueToken(first, remainder);
        }

        return ClassifyStack(rest);
    }

    private TokenEvent ClassifyStack(string chars)
    {
        var values = new List<string>();
        for (var index = 0; index < chars.Length; index++)
        {
            var flagChar = chars[index].ToString();
            if (!_shortMap.TryGetValue(flagChar, out var flag))
            {
                return new UnknownFlagToken($"-{chars}");
            }

            if (flag.Type is not (ValueType.Boolean or ValueType.Count))
            {
                if (index == chars.Length - 1)
                {
                    values.Add(flagChar);
                    return new StackedFlagsToken(values);
                }

                return new UnknownFlagToken($"-{chars}");
            }

            values.Add(flagChar);
        }

        return new StackedFlagsToken(values);
    }
}

public sealed class SpecLoader
{
    private readonly string? _filePath;
    private CliSpec? _cached;

    public SpecLoader(string specFilePath)
    {
        _filePath = specFilePath;
    }

    public CliSpec Load()
    {
        if (_cached is not null)
        {
            return _cached;
        }

        if (string.IsNullOrWhiteSpace(_filePath))
        {
            throw new SpecError("No spec file path was provided.");
        }

        try
        {
            var text = File.ReadAllText(_filePath);
            var raw = AsObject(JsonNode.ParseNative(text), "root");
            _cached = ParseSpec(raw);
            return _cached;
        }
        catch (SpecError)
        {
            throw;
        }
        catch (Exception exception)
        {
            throw new SpecError($"Failed to read spec file '{_filePath}': {exception.Message}");
        }
    }

    public CliSpec LoadFromObject(IDictionary<string, object?> raw)
    {
        if (_cached is null)
        {
            _cached = ParseSpec(raw);
        }

        return _cached;
    }

    public static ValidationResult ValidateSpec(string specFilePath)
    {
        try
        {
            _ = new SpecLoader(specFilePath).Load();
            return new ValidationResult(true, []);
        }
        catch (SpecError error)
        {
            return new ValidationResult(false, [error.Message]);
        }
    }

    public static ValidationResult ValidateSpecObject(IDictionary<string, object?> raw)
    {
        try
        {
            _ = new SpecLoader("<memory>").LoadFromObject(raw);
            return new ValidationResult(true, []);
        }
        catch (SpecError error)
        {
            return new ValidationResult(false, [error.Message]);
        }
    }

    private static CliSpec ParseSpec(IDictionary<string, object?> raw)
    {
        var specVersion = RequireString(raw, "cli_builder_spec_version", "root");
        if (!string.Equals(specVersion, "1.0", StringComparison.Ordinal))
        {
            throw new SpecError($"cli_builder_spec_version must be \"1.0\", got: {specVersion}");
        }

        var name = RequireString(raw, "name", "root");
        var description = RequireString(raw, "description", "root");
        var displayName = OptionalString(raw, "display_name");
        var version = OptionalString(raw, "version");
        var parsingMode = ParseParsingMode(raw.TryGetValue("parsing_mode", out var parsingModeRaw) ? parsingModeRaw : null, "root");

        var builtinFlags = ParseBuiltinFlags(raw.TryGetValue("builtin_flags", out var builtinRaw) ? builtinRaw : null);
        var globalFlags = ParseFlagArray(raw.TryGetValue("global_flags", out var globalFlagsRaw) ? globalFlagsRaw : null, "global_flags", []);
        var flags = ParseFlagArray(raw.TryGetValue("flags", out var flagsRaw) ? flagsRaw : null, "flags", globalFlags);
        var arguments = ParseArgArray(raw.TryGetValue("arguments", out var argsRaw) ? argsRaw : null, "arguments");
        var commands = ParseCommandArray(raw.TryGetValue("commands", out var commandsRaw) ? commandsRaw : null, "commands", globalFlags);
        var groups = ParseExclusiveGroups(raw.TryGetValue("mutually_exclusive_groups", out var groupsRaw) ? groupsRaw : null, "mutually_exclusive_groups", flags, globalFlags);

        CheckVariadicCount(arguments, "root");
        CheckFlagRequiresGraph(globalFlags.Concat(flags).ToList(), "root");

        return new CliSpec(
            "1.0",
            name,
            displayName,
            description,
            version,
            parsingMode,
            builtinFlags,
            globalFlags,
            flags,
            arguments,
            commands,
            groups);
    }

    private static BuiltinFlags ParseBuiltinFlags(object? raw)
    {
        if (raw is null)
        {
            return new BuiltinFlags(true, true);
        }

        var map = AsObject(raw, "builtin_flags");
        return new BuiltinFlags(
            map.TryGetValue("help", out var help) && help is not null ? ConvertToBoolean(help, "builtin_flags.help") : true,
            map.TryGetValue("version", out var version) && version is not null ? ConvertToBoolean(version, "builtin_flags.version") : true);
    }

    private static List<FlagDef> ParseFlagArray(object? raw, string fieldPath, IReadOnlyList<FlagDef> globalFlags)
    {
        if (raw is null)
        {
            return [];
        }

        var items = AsArray(raw, fieldPath);
        var flags = new List<FlagDef>();
        var seenIds = new HashSet<string>(StringComparer.Ordinal);

        for (var index = 0; index < items.Count; index++)
        {
            var flag = ParseFlag(AsObject(items[index], $"{fieldPath}[{index}]"), $"{fieldPath}[{index}]");
            if (!seenIds.Add(flag.Id))
            {
                throw new SpecError($"Duplicate flag id \"{flag.Id}\" in {fieldPath}");
            }

            flags.Add(flag);
        }

        var validFlagIds = globalFlags.Select(flag => flag.Id).Concat(flags.Select(flag => flag.Id)).ToHashSet(StringComparer.Ordinal);
        foreach (var flag in flags)
        {
            foreach (var reference in flag.ConflictsWith.Concat(flag.Requires).Concat(flag.RequiredUnless))
            {
                if (!validFlagIds.Contains(reference))
                {
                    throw new SpecError($"Flag \"{flag.Id}\" references unknown flag id \"{reference}\" in {fieldPath}");
                }
            }
        }

        return flags;
    }

    private static FlagDef ParseFlag(IDictionary<string, object?> raw, string path)
    {
        var id = RequireString(raw, "id", path);
        var description = RequireString(raw, "description", path);
        var type = ParseValueType(RequireString(raw, "type", path), path);
        var shortName = OptionalString(raw, "short");
        var longName = OptionalString(raw, "long");
        var singleDashLong = OptionalString(raw, "single_dash_long");
        if (shortName is null && longName is null && singleDashLong is null)
        {
            throw new SpecError($"Flag \"{id}\" at {path} must have at least one of short, long, or single_dash_long.");
        }

        var enumValues = OptionalStringArray(raw, "enum_values");
        if (type == ValueType.Enum && enumValues.Count == 0)
        {
            throw new SpecError($"Flag \"{id}\" at {path} has type enum but enum_values is empty.");
        }

        var defaultWhenPresent = OptionalString(raw, "default_when_present");
        if (defaultWhenPresent is not null)
        {
            if (type != ValueType.Enum)
            {
                throw new SpecError($"Flag \"{id}\" at {path} uses default_when_present but is not an enum.");
            }

            if (!enumValues.Contains(defaultWhenPresent, StringComparer.Ordinal))
            {
                throw new SpecError($"Flag \"{id}\" at {path} has default_when_present \"{defaultWhenPresent}\" outside enum_values.");
            }
        }

        return new FlagDef(
            id,
            shortName,
            longName,
            singleDashLong,
            description,
            type,
            OptionalBoolean(raw, "required") ?? false,
            raw.TryGetValue("default", out var defaultValue) ? defaultValue : null,
            OptionalString(raw, "value_name"),
            enumValues,
            defaultWhenPresent,
            OptionalStringArray(raw, "conflicts_with"),
            OptionalStringArray(raw, "requires"),
            OptionalStringArray(raw, "required_unless"),
            OptionalBoolean(raw, "repeatable") ?? false);
    }

    private static List<ArgDef> ParseArgArray(object? raw, string fieldPath)
    {
        if (raw is null)
        {
            return [];
        }

        var items = AsArray(raw, fieldPath);
        var args = new List<ArgDef>();
        var seenIds = new HashSet<string>(StringComparer.Ordinal);

        for (var index = 0; index < items.Count; index++)
        {
            var arg = ParseArg(AsObject(items[index], $"{fieldPath}[{index}]"), $"{fieldPath}[{index}]");
            if (!seenIds.Add(arg.Id))
            {
                throw new SpecError($"Duplicate argument id \"{arg.Id}\" in {fieldPath}");
            }

            args.Add(arg);
        }

        return args;
    }

    private static ArgDef ParseArg(IDictionary<string, object?> raw, string path)
    {
        var id = RequireString(raw, "id", path);
        var displayName = OptionalString(raw, "display_name") ?? OptionalString(raw, "name") ?? throw new SpecError($"Argument \"{id}\" at {path} is missing display_name.");
        var description = RequireString(raw, "description", path);
        var type = ParseValueType(RequireString(raw, "type", path), path);
        var required = OptionalBoolean(raw, "required") ?? true;
        var variadic = OptionalBoolean(raw, "variadic") ?? false;
        var variadicMin = OptionalInteger(raw, "variadic_min") ?? (variadic ? (required ? 1 : 0) : 0);
        var variadicMax = raw.TryGetValue("variadic_max", out var variadicMaxRaw) && variadicMaxRaw is not null ? ConvertToInteger(variadicMaxRaw, $"{path}.variadic_max") : (int?)null;
        var enumValues = OptionalStringArray(raw, "enum_values");
        if (type == ValueType.Enum && enumValues.Count == 0)
        {
            throw new SpecError($"Argument \"{id}\" at {path} has type enum but enum_values is empty.");
        }

        return new ArgDef(
            id,
            displayName,
            description,
            type,
            required,
            variadic,
            variadicMin,
            variadicMax,
            raw.TryGetValue("default", out var defaultValue) ? defaultValue : null,
            enumValues,
            OptionalStringArray(raw, "required_unless_flag"));
    }

    private static List<CommandDef> ParseCommandArray(object? raw, string fieldPath, IReadOnlyList<FlagDef> globalFlags)
    {
        if (raw is null)
        {
            return [];
        }

        var items = AsArray(raw, fieldPath);
        var commands = new List<CommandDef>();
        var seenIds = new HashSet<string>(StringComparer.Ordinal);

        for (var index = 0; index < items.Count; index++)
        {
            var command = ParseCommand(AsObject(items[index], $"{fieldPath}[{index}]"), $"{fieldPath}[{index}]", globalFlags);
            if (!seenIds.Add(command.Id))
            {
                throw new SpecError($"Duplicate command id \"{command.Id}\" in {fieldPath}");
            }

            commands.Add(command);
        }

        return commands;
    }

    private static CommandDef ParseCommand(IDictionary<string, object?> raw, string path, IReadOnlyList<FlagDef> globalFlags)
    {
        var id = RequireString(raw, "id", path);
        var name = RequireString(raw, "name", path);
        var description = RequireString(raw, "description", path);
        var inheritGlobalFlags = OptionalBoolean(raw, "inherit_global_flags") ?? true;
        var visibleGlobalFlags = inheritGlobalFlags ? globalFlags : [];
        var flags = ParseFlagArray(raw.TryGetValue("flags", out var flagsRaw) ? flagsRaw : null, $"{path}.flags", visibleGlobalFlags);
        var arguments = ParseArgArray(raw.TryGetValue("arguments", out var argsRaw) ? argsRaw : null, $"{path}.arguments");
        var commands = ParseCommandArray(raw.TryGetValue("commands", out var commandsRaw) ? commandsRaw : null, $"{path}.commands", globalFlags);
        var groups = ParseExclusiveGroups(raw.TryGetValue("mutually_exclusive_groups", out var groupsRaw) ? groupsRaw : null, $"{path}.mutually_exclusive_groups", flags, visibleGlobalFlags);
        var aliases = OptionalStringArray(raw, "aliases");

        CheckVariadicCount(arguments, path);
        CheckFlagRequiresGraph(visibleGlobalFlags.Concat(flags).ToList(), path);

        return new CommandDef(id, name, aliases, description, inheritGlobalFlags, flags, arguments, commands, groups);
    }

    private static List<ExclusiveGroup> ParseExclusiveGroups(object? raw, string fieldPath, IReadOnlyList<FlagDef> localFlags, IReadOnlyList<FlagDef> globalFlags)
    {
        if (raw is null)
        {
            return [];
        }

        var validFlagIds = localFlags.Select(flag => flag.Id).Concat(globalFlags.Select(flag => flag.Id)).ToHashSet(StringComparer.Ordinal);
        var groups = new List<ExclusiveGroup>();
        foreach (var (entry, index) in AsArray(raw, fieldPath).Select((entry, index) => (entry, index)))
        {
            var map = AsObject(entry, $"{fieldPath}[{index}]");
            var id = RequireString(map, "id", $"{fieldPath}[{index}]");
            var ids = OptionalStringArray(map, "flag_ids");
            foreach (var flagId in ids)
            {
                if (!validFlagIds.Contains(flagId))
                {
                    throw new SpecError($"Exclusive group \"{id}\" references unknown flag id \"{flagId}\" in {fieldPath}");
                }
            }

            groups.Add(new ExclusiveGroup(id, ids, OptionalBoolean(map, "required") ?? false));
        }

        return groups;
    }

    private static void CheckVariadicCount(IReadOnlyList<ArgDef> arguments, string path)
    {
        if (arguments.Count(argument => argument.Variadic) > 1)
        {
            throw new SpecError($"At most one variadic argument is allowed in {path}.");
        }
    }

    private static void CheckFlagRequiresGraph(IReadOnlyList<FlagDef> flags, string path)
    {
        var graph = new Graph();
        foreach (var flag in flags)
        {
            graph.AddNode(flag.Id);
        }

        foreach (var flag in flags)
        {
            foreach (var requiredFlagId in flag.Requires)
            {
                if (graph.HasNode(requiredFlagId))
                {
                    graph.AddEdge(flag.Id, requiredFlagId);
                }
            }
        }

        try
        {
            _ = graph.TopologicalSort();
        }
        catch (CycleError error)
        {
            throw new SpecError($"Circular requires dependency detected in {path}: {string.Join(" -> ", error.Cycle)}");
        }
    }

    private static ParsingMode ParseParsingMode(object? raw, string path)
    {
        if (raw is null)
        {
            return ParsingMode.Gnu;
        }

        return raw switch
        {
            "gnu" => ParsingMode.Gnu,
            "posix" => ParsingMode.Posix,
            "subcommand_first" => ParsingMode.SubcommandFirst,
            "traditional" => ParsingMode.Traditional,
            _ => throw new SpecError($"parsing_mode at {path} must be one of gnu, posix, subcommand_first, or traditional."),
        };
    }

    private static ValueType ParseValueType(string raw, string path)
    {
        return raw switch
        {
            "boolean" => ValueType.Boolean,
            "count" => ValueType.Count,
            "string" => ValueType.String,
            "integer" => ValueType.Integer,
            "float" => ValueType.Float,
            "path" => ValueType.Path,
            "file" => ValueType.File,
            "directory" => ValueType.Directory,
            "enum" => ValueType.Enum,
            _ => throw new SpecError($"Unsupported value type \"{raw}\" at {path}."),
        };
    }

    private static IDictionary<string, object?> AsObject(object? raw, string path)
    {
        return raw as IDictionary<string, object?> ?? throw new SpecError($"{path} must be an object.");
    }

    private static IReadOnlyList<object?> AsArray(object? raw, string path)
    {
        return raw as IReadOnlyList<object?> ?? throw new SpecError($"{path} must be an array.");
    }

    private static string RequireString(IDictionary<string, object?> raw, string fieldName, string path)
    {
        return OptionalString(raw, fieldName) ?? throw new SpecError($"{path}.{fieldName} must be a string.");
    }

    private static string? OptionalString(IDictionary<string, object?> raw, string fieldName)
    {
        return raw.TryGetValue(fieldName, out var value) ? value as string : null;
    }

    private static bool? OptionalBoolean(IDictionary<string, object?> raw, string fieldName)
    {
        return raw.TryGetValue(fieldName, out var value) && value is not null ? ConvertToBoolean(value, fieldName) : null;
    }

    private static int? OptionalInteger(IDictionary<string, object?> raw, string fieldName)
    {
        return raw.TryGetValue(fieldName, out var value) && value is not null ? ConvertToInteger(value, fieldName) : null;
    }

    private static List<string> OptionalStringArray(IDictionary<string, object?> raw, string fieldName)
    {
        if (!raw.TryGetValue(fieldName, out var value) || value is null)
        {
            return [];
        }

        return AsArray(value, fieldName)
            .Select(item => item as string ?? throw new SpecError($"{fieldName} entries must be strings."))
            .ToList();
    }

    private static bool ConvertToBoolean(object raw, string path)
    {
        return raw switch
        {
            bool value => value,
            _ => throw new SpecError($"{path} must be a boolean."),
        };
    }

    private static int ConvertToInteger(object raw, string path)
    {
        return raw switch
        {
            int value => value,
            long value when value is >= int.MinValue and <= int.MaxValue => (int)value,
            double value when double.IsInteger(value) && value is >= int.MinValue and <= int.MaxValue => (int)value,
            _ => throw new SpecError($"{path} must be an integer."),
        };
    }
}

public sealed class PositionalResolver
{
    private readonly IReadOnlyList<ArgDef> _argumentDefinitions;

    public PositionalResolver(IReadOnlyList<ArgDef> argumentDefinitions)
    {
        _argumentDefinitions = argumentDefinitions;
    }

    public (Dictionary<string, object?> Result, List<ParseError> Errors) Resolve(
        IReadOnlyList<string> tokens,
        IReadOnlyDictionary<string, object?> parsedFlags,
        IReadOnlyList<string> context)
    {
        var result = new Dictionary<string, object?>(StringComparer.Ordinal);
        var errors = new List<ParseError>();
        var minimumSuffix = BuildMinimumSuffixRequirements(parsedFlags);
        var index = 0;

        for (var argumentIndex = 0; argumentIndex < _argumentDefinitions.Count; argumentIndex++)
        {
            var argument = _argumentDefinitions[argumentIndex];
            if (argument.Variadic)
            {
                var remaining = tokens.Count - index;
                var mustReserveForSuffix = minimumSuffix[argumentIndex + 1];
                var available = Math.Max(0, remaining - mustReserveForSuffix);
                var maxCount = argument.VariadicMax ?? int.MaxValue;
                var take = Math.Min(available, maxCount);
                var requiredMinimum = IsArgumentRequired(argument, parsedFlags) ? argument.VariadicMin : 0;
                if (take < requiredMinimum)
                {
                    errors.Add(new ParseError("too_few_arguments", $"Argument \"{argument.DisplayName}\" expects at least {requiredMinimum} values.", null, context));
                    take = Math.Max(0, take);
                }

                var values = new List<object?>();
                for (var count = 0; count < take; count++)
                {
                    var coercion = CoerceValue(tokens[index++], argument.Type, argument.Id, context, argument.EnumValues);
                    if (coercion.Error is not null)
                    {
                        errors.Add(coercion.Error);
                    }
                    else
                    {
                        values.Add(coercion.Value);
                    }
                }

                result[argument.Id] = values;
                continue;
            }

            var shouldConsume = tokens.Count - index > minimumSuffix[argumentIndex + 1];
            if (shouldConsume)
            {
                var coercion = CoerceValue(tokens[index++], argument.Type, argument.Id, context, argument.EnumValues);
                if (coercion.Error is not null)
                {
                    errors.Add(coercion.Error);
                    result[argument.Id] = argument.Default;
                }
                else
                {
                    result[argument.Id] = coercion.Value;
                }
            }
            else if (IsArgumentRequired(argument, parsedFlags))
            {
                errors.Add(new ParseError("missing_required_argument", $"Argument \"{argument.DisplayName}\" is required.", null, context));
                result[argument.Id] = argument.Default;
            }
            else
            {
                result[argument.Id] = argument.Default;
            }
        }

        if (index < tokens.Count)
        {
            errors.Add(new ParseError("too_many_arguments", $"Received too many positional arguments: {string.Join(" ", tokens.Skip(index))}", null, context));
        }

        foreach (var argument in _argumentDefinitions.Where(argument => !result.ContainsKey(argument.Id)))
        {
            result[argument.Id] = argument.Variadic ? new List<object?>() : argument.Default;
        }

        return (result, errors);
    }

    public static (object? Value, ParseError? Error) CoerceValue(
        string raw,
        ValueType type,
        string argumentId,
        IReadOnlyList<string> context,
        IReadOnlyList<string>? enumValues = null)
    {
        switch (type)
        {
            case ValueType.Boolean:
            case ValueType.Count:
            case ValueType.String:
            case ValueType.Path:
                return (raw, null);

            case ValueType.Integer:
                if (long.TryParse(raw, NumberStyles.Integer, CultureInfo.InvariantCulture, out var integerValue))
                {
                    return (integerValue, null);
                }

                return (null, new ParseError("invalid_value", $"Invalid integer for \"{argumentId}\": '{raw}'", null, context));

            case ValueType.Float:
                if (double.TryParse(raw, NumberStyles.Float | NumberStyles.AllowThousands, CultureInfo.InvariantCulture, out var floatValue))
                {
                    return (floatValue, null);
                }

                return (null, new ParseError("invalid_value", $"Invalid float for \"{argumentId}\": '{raw}'", null, context));

            case ValueType.File:
                if (!System.IO.File.Exists(raw))
                {
                    return (null, new ParseError("invalid_value", $"File not found: \"{raw}\"", null, context));
                }

                return (raw, null);

            case ValueType.Directory:
                if (!System.IO.Directory.Exists(raw))
                {
                    return (null, new ParseError("invalid_value", $"Directory not found: \"{raw}\"", null, context));
                }

                return (raw, null);

            case ValueType.Enum:
                if (enumValues is not null && enumValues.Contains(raw, StringComparer.Ordinal))
                {
                    return (raw, null);
                }

                return (null, new ParseError("invalid_enum_value", $"Invalid value '{raw}' for \"{argumentId}\". Must be one of: {string.Join(", ", enumValues ?? [])}", null, context));

            default:
                return (raw, null);
        }
    }

    private int[] BuildMinimumSuffixRequirements(IReadOnlyDictionary<string, object?> parsedFlags)
    {
        var suffix = new int[_argumentDefinitions.Count + 1];
        for (var index = _argumentDefinitions.Count - 1; index >= 0; index--)
        {
            var argument = _argumentDefinitions[index];
            var requirement = IsArgumentRequired(argument, parsedFlags)
                ? (argument.Variadic ? argument.VariadicMin : 1)
                : 0;
            suffix[index] = suffix[index + 1] + requirement;
        }

        return suffix;
    }

    private static bool IsArgumentRequired(ArgDef argument, IReadOnlyDictionary<string, object?> parsedFlags)
    {
        if (!argument.Required)
        {
            return false;
        }

        if (argument.RequiredUnlessFlag.Count == 0)
        {
            return true;
        }

        return !argument.RequiredUnlessFlag.Any(flagId => FlagPresence.IsPresent(parsedFlags.GetValueOrDefault(flagId)));
    }
}

public sealed class FlagValidator
{
    private readonly IReadOnlyList<FlagDef> _activeFlags;
    private readonly IReadOnlyList<ExclusiveGroup> _exclusiveGroups;
    private readonly Dictionary<string, FlagDef> _byId;
    private readonly Graph _requiresGraph = new();

    public FlagValidator(IReadOnlyList<FlagDef> activeFlags, IReadOnlyList<ExclusiveGroup> exclusiveGroups)
    {
        _activeFlags = activeFlags;
        _exclusiveGroups = exclusiveGroups;
        _byId = activeFlags.ToDictionary(flag => flag.Id, StringComparer.Ordinal);

        foreach (var flag in activeFlags)
        {
            _requiresGraph.AddNode(flag.Id);
        }

        foreach (var flag in activeFlags)
        {
            foreach (var requiredFlag in flag.Requires.Where(_requiresGraph.HasNode))
            {
                _requiresGraph.AddEdge(flag.Id, requiredFlag);
            }
        }
    }

    public List<ParseError> Validate(IReadOnlyDictionary<string, object?> parsedFlags, IReadOnlyList<string> context)
    {
        var errors = new List<ParseError>();
        var presentFlags = _activeFlags.Where(flag => FlagPresence.IsPresent(flag, parsedFlags.GetValueOrDefault(flag.Id))).Select(flag => flag.Id).ToHashSet(StringComparer.Ordinal);
        var reportedConflicts = new HashSet<string>(StringComparer.Ordinal);

        foreach (var flagId in presentFlags)
        {
            var flag = _byId[flagId];
            foreach (var otherId in flag.ConflictsWith.Where(presentFlags.Contains))
            {
                var key = string.Join("\0", new[] { flagId, otherId }.OrderBy(value => value, StringComparer.Ordinal));
                if (reportedConflicts.Add(key))
                {
                    errors.Add(new ParseError("conflicting_flags", $"{DisplayFlag(flag)} and {DisplayFlag(_byId[otherId])} cannot be used together.", null, context));
                }
            }

            foreach (var requiredFlag in _requiresGraph.TransitiveClosure(flagId))
            {
                if (!presentFlags.Contains(requiredFlag))
                {
                    errors.Add(new ParseError("missing_dependency_flag", $"{DisplayFlag(flag)} requires {DisplayFlag(_byId[requiredFlag])}.", null, context));
                }
            }
        }

        foreach (var flag in _activeFlags.Where(flag => flag.Required && !presentFlags.Contains(flag.Id)))
        {
            if (!flag.RequiredUnless.Any(presentFlags.Contains))
            {
                errors.Add(new ParseError("missing_required_flag", $"{DisplayFlag(flag)} is required.", null, context));
            }
        }

        foreach (var group in _exclusiveGroups)
        {
            var presentInGroup = group.FlagIds.Where(presentFlags.Contains).ToList();
            if (presentInGroup.Count > 1)
            {
                errors.Add(new ParseError("exclusive_group_violation", $"Only one of {string.Join(", ", presentInGroup.Select(id => DisplayFlag(_byId[id])))} may be used.", null, context));
            }

            if (group.Required && presentInGroup.Count == 0)
            {
                errors.Add(new ParseError("missing_exclusive_group", $"One of {string.Join(", ", group.FlagIds.Select(id => DisplayFlag(_byId[id])))} is required.", null, context));
            }
        }

        return errors;
    }

    private static string DisplayFlag(FlagDef flag)
    {
        var parts = new List<string>();
        if (!string.IsNullOrWhiteSpace(flag.Short))
        {
            parts.Add($"-{flag.Short}");
        }

        if (!string.IsNullOrWhiteSpace(flag.Long))
        {
            parts.Add($"--{flag.Long}");
        }

        if (!string.IsNullOrWhiteSpace(flag.SingleDashLong))
        {
            parts.Add($"-{flag.SingleDashLong}");
        }

        return string.Join("/", parts);
    }
}

public sealed class HelpGenerator
{
    private readonly CliSpec _spec;
    private readonly IReadOnlyList<string> _commandSegments;

    public HelpGenerator(CliSpec spec, IReadOnlyList<string> commandSegments)
    {
        _spec = spec;
        _commandSegments = commandSegments;
    }

    public string Generate()
    {
        var command = ResolveCommand(_commandSegments);
        var lines = new List<string>();
        lines.Add("USAGE");
        lines.Add($"  {BuildUsageLine(command)}");
        lines.Add(string.Empty);
        lines.Add("DESCRIPTION");
        lines.Add($"  {(command?.Description ?? _spec.Description)}");

        var commands = command?.Commands ?? _spec.Commands;
        if (commands.Count > 0)
        {
            lines.Add(string.Empty);
            lines.Add("COMMANDS");
            var width = commands.Max(item => item.Name.Length);
            foreach (var child in commands)
            {
                lines.Add($"  {child.Name.PadRight(width + 2)}{child.Description}");
            }
        }

        var localFlags = command?.Flags ?? _spec.Flags;
        if (localFlags.Count > 0)
        {
            lines.Add(string.Empty);
            lines.Add("OPTIONS");
            foreach (var line in BuildFlagLines(localFlags))
            {
                lines.Add($"  {line}");
            }
        }

        var arguments = command?.Arguments ?? _spec.Arguments;
        if (arguments.Count > 0)
        {
            lines.Add(string.Empty);
            lines.Add("ARGUMENTS");
            foreach (var argument in arguments)
            {
                var suffix = argument.Required ? "Required." : "Optional.";
                if (argument.Variadic)
                {
                    suffix += " Repeatable.";
                }

                lines.Add($"  {DisplayArgument(argument).PadRight(18)}{argument.Description}. {suffix}");
            }
        }

        var globalFlags = new List<FlagDef>(_spec.GlobalFlags);
        if (_spec.BuiltinFlags.Help)
        {
            globalFlags.Add(BuiltinFlag("help", "h", "help", "Show this help message and exit."));
        }

        if (_spec.BuiltinFlags.Version && !string.IsNullOrWhiteSpace(_spec.Version))
        {
            globalFlags.Add(BuiltinFlag("version", null, "version", "Show version and exit."));
        }

        if (globalFlags.Count > 0)
        {
            lines.Add(string.Empty);
            lines.Add("GLOBAL OPTIONS");
            foreach (var line in BuildFlagLines(globalFlags))
            {
                lines.Add($"  {line}");
            }
        }

        return string.Join("\n", lines);
    }

    private string BuildUsageLine(CommandDef? command)
    {
        var parts = new List<string> { _spec.Name };
        parts.AddRange(_commandSegments);

        var flags = (command?.Flags ?? _spec.Flags).Count + _spec.GlobalFlags.Count;
        if (flags > 0 || _spec.BuiltinFlags.Help)
        {
            parts.Add("[OPTIONS]");
        }

        if ((command?.Commands ?? _spec.Commands).Count > 0)
        {
            parts.Add("[COMMAND]");
        }

        parts.AddRange((command?.Arguments ?? _spec.Arguments).Select(DisplayArgument));
        return string.Join(" ", parts);
    }

    private IEnumerable<string> BuildFlagLines(IEnumerable<FlagDef> flags)
    {
        var entries = flags
            .Select(flag => (Signature: BuildSignature(flag), Description: BuildDescription(flag)))
            .ToList();

        var width = entries.Count == 0 ? 0 : entries.Max(entry => entry.Signature.Length);
        foreach (var entry in entries)
        {
            yield return $"{entry.Signature.PadRight(width + 4)}{entry.Description}";
        }
    }

    private static string BuildSignature(FlagDef flag)
    {
        var parts = new List<string>();
        var usesValue = flag.Type is not (ValueType.Boolean or ValueType.Count);
        if (!string.IsNullOrWhiteSpace(flag.Short))
        {
            parts.Add($"-{flag.Short}");
        }

        if (!string.IsNullOrWhiteSpace(flag.Long))
        {
            parts.Add(usesValue ? $"--{flag.Long} <{flag.ValueName ?? flag.Type.ToString().ToUpperInvariant()}>" : $"--{flag.Long}");
        }

        if (!string.IsNullOrWhiteSpace(flag.SingleDashLong))
        {
            parts.Add(usesValue ? $"-{flag.SingleDashLong} <{flag.ValueName ?? flag.Type.ToString().ToUpperInvariant()}>" : $"-{flag.SingleDashLong}");
        }

        return string.Join(", ", parts);
    }

    private static string BuildDescription(FlagDef flag)
    {
        var description = flag.Description;
        if (flag.Default is not null && !flag.Required && flag.Type is not (ValueType.Boolean or ValueType.Count))
        {
            description += $" [default: {flag.Default}]";
        }

        return description;
    }

    private static string DisplayArgument(ArgDef argument)
    {
        var label = argument.Variadic ? $"{argument.DisplayName}..." : argument.DisplayName;
        return argument.Required ? $"<{label}>" : $"[{label}]";
    }

    private CommandDef? ResolveCommand(IReadOnlyList<string> commandSegments)
    {
        var commands = _spec.Commands;
        CommandDef? current = null;
        foreach (var segment in commandSegments)
        {
            current = commands.FirstOrDefault(command => command.Name == segment || command.Aliases.Contains(segment, StringComparer.Ordinal));
            if (current is null)
            {
                break;
            }

            commands = current.Commands;
        }

        return current;
    }

    private static FlagDef BuiltinFlag(string id, string? shortName, string? longName, string description)
    {
        return new FlagDef(id, shortName, longName, null, description, ValueType.Boolean, false, null, null, [], null, [], [], [], false);
    }
}

public sealed class Parser
{
    private readonly IReadOnlyList<string> _argv;
    private readonly CliSpec? _spec;
    private readonly SpecLoader? _loader;

    public Parser(string specFilePath, IReadOnlyList<string> argv)
    {
        _argv = argv;
        _loader = new SpecLoader(specFilePath);
    }

    public Parser(CliSpec spec, IReadOnlyList<string> argv)
    {
        _argv = argv;
        _spec = spec;
    }

    public ParserResult Parse()
    {
        var spec = _spec ?? _loader?.Load() ?? throw new SpecError("No CLI spec is available.");
        var program = _argv.Count > 0 ? _argv[0] : spec.Name;
        var commandSegments = new List<string>();
        CommandDef? currentCommand = null;
        var currentFlags = BuildActiveFlags(spec, currentCommand);
        var parsedFlags = InitializeFlagValues(currentFlags);
        var positionalTokens = new List<string>();
        var explicitFlags = new List<string>();
        var routeFinalized = false;
        var endOfFlags = false;
        FlagDef? pendingValueFlag = null;
        var pendingValueContext = new List<string> { program };

        for (var index = 1; index < _argv.Count; index++)
        {
            var token = _argv[index];
            if (spec.ParsingMode == ParsingMode.Traditional
                && index == 1
                && !token.StartsWith("-", StringComparison.Ordinal)
                && !TryResolveCommand(token, currentCommand?.Commands ?? spec.Commands, out _))
            {
                token = "-" + token;
            }

            if (pendingValueFlag is not null)
            {
                ApplyFlagValue(pendingValueFlag, token, parsedFlags, explicitFlags, pendingValueContext, duplicateAsError: false);
                pendingValueFlag = null;
                continue;
            }

            if (!endOfFlags
                && !routeFinalized
                && !token.StartsWith("-", StringComparison.Ordinal)
                && TryResolveCommand(token, currentCommand?.Commands ?? spec.Commands, out var resolvedCommand))
            {
                commandSegments.Add(resolvedCommand.Name);
                currentCommand = resolvedCommand;
                currentFlags = BuildActiveFlags(spec, currentCommand);
                parsedFlags = PreserveMatchingFlagValues(parsedFlags, currentFlags);
                continue;
            }

            var classifier = new TokenClassifier(currentFlags.Concat(BuiltinFlagsFor(spec)).ToList());
            var tokenEvent = endOfFlags ? new PositionalToken(token) : classifier.Classify(token);
            switch (tokenEvent)
            {
                case EndOfFlagsToken:
                    endOfFlags = true;
                    routeFinalized = true;
                    break;

                case PositionalToken positionalToken:
                    routeFinalized = true;
                    positionalTokens.Add(positionalToken.Value);
                    if (spec.ParsingMode == ParsingMode.Posix)
                    {
                        endOfFlags = true;
                    }

                    if (spec.ParsingMode == ParsingMode.SubcommandFirst
                        && commandSegments.Count == 0
                        && (spec.Commands.Count > 0 || currentCommand?.Commands.Count > 0))
                    {
                        throw new ParseErrors(
                        [
                            new ParseError("unknown_command", $"Unknown command \"{positionalToken.Value}\".", null, [program]),
                        ]);
                    }

                    break;

                case LongFlagToken { Name: "help" } or ShortFlagToken { Char: "h" }:
                    if (spec.BuiltinFlags.Help)
                    {
                        return new HelpResult(new HelpGenerator(spec, commandSegments).Generate(), [program, .. commandSegments]);
                    }

                    positionalTokens.Add(token);
                    break;

                case LongFlagToken { Name: "version" }:
                    if (spec.BuiltinFlags.Version && !string.IsNullOrWhiteSpace(spec.Version))
                    {
                        return new VersionResult(spec.Version);
                    }

                    positionalTokens.Add(token);
                    break;

                case LongFlagWithValueToken longFlagWithValue:
                    if (!TryFindFlag(flag => string.Equals(flag.Long, longFlagWithValue.Name, StringComparison.Ordinal), currentFlags, out var longFlag))
                    {
                        throw UnknownFlag(program, commandSegments, $"--{longFlagWithValue.Name}");
                    }

                    ApplyFlagValue(longFlag, longFlagWithValue.Value, parsedFlags, explicitFlags, [program, .. commandSegments], duplicateAsError: true);
                    routeFinalized = true;
                    break;

                case LongFlagToken longFlagToken:
                    if (!TryFindFlag(flag => string.Equals(flag.Long, longFlagToken.Name, StringComparison.Ordinal), currentFlags, out var longFlagDefinition))
                    {
                        throw UnknownFlag(program, commandSegments, $"--{longFlagToken.Name}");
                    }

                    if (longFlagDefinition.Type is ValueType.Boolean or ValueType.Count)
                    {
                        ApplyFlagPresence(longFlagDefinition, parsedFlags, explicitFlags, [program, .. commandSegments], duplicateAsError: true);
                    }
                    else
                    {
                        var defaultWhenPresent = longFlagDefinition.DefaultWhenPresent;
                        if (defaultWhenPresent is not null && (index == _argv.Count - 1 || _argv[index + 1].StartsWith("-", StringComparison.Ordinal)))
                        {
                            ApplyFlagValue(longFlagDefinition, defaultWhenPresent, parsedFlags, explicitFlags, [program, .. commandSegments], duplicateAsError: true);
                        }
                        else
                        {
                            pendingValueFlag = longFlagDefinition;
                            pendingValueContext = [program, .. commandSegments];
                        }
                    }

                    routeFinalized = true;
                    break;

                case SingleDashLongToken singleDashLongToken:
                    if (!TryFindFlag(flag => string.Equals(flag.SingleDashLong, singleDashLongToken.Name, StringComparison.Ordinal), currentFlags, out var singleDashLongFlag))
                    {
                        throw UnknownFlag(program, commandSegments, $"-{singleDashLongToken.Name}");
                    }

                    if (singleDashLongFlag.Type is ValueType.Boolean or ValueType.Count)
                    {
                        ApplyFlagPresence(singleDashLongFlag, parsedFlags, explicitFlags, [program, .. commandSegments], duplicateAsError: true);
                    }
                    else
                    {
                        pendingValueFlag = singleDashLongFlag;
                        pendingValueContext = [program, .. commandSegments];
                    }

                    routeFinalized = true;
                    break;

                case ShortFlagWithValueToken shortFlagWithValue:
                    if (!TryFindFlag(flag => string.Equals(flag.Short, shortFlagWithValue.Char, StringComparison.Ordinal), currentFlags, out var shortFlag))
                    {
                        throw UnknownFlag(program, commandSegments, $"-{shortFlagWithValue.Char}");
                    }

                    ApplyFlagValue(shortFlag, shortFlagWithValue.Value, parsedFlags, explicitFlags, [program, .. commandSegments], duplicateAsError: true);
                    routeFinalized = true;
                    break;

                case ShortFlagToken shortFlagToken:
                    if (!TryFindFlag(flag => string.Equals(flag.Short, shortFlagToken.Char, StringComparison.Ordinal), currentFlags, out var shortFlagDefinition))
                    {
                        throw UnknownFlag(program, commandSegments, $"-{shortFlagToken.Char}");
                    }

                    if (shortFlagDefinition.Type is ValueType.Boolean or ValueType.Count)
                    {
                        ApplyFlagPresence(shortFlagDefinition, parsedFlags, explicitFlags, [program, .. commandSegments], duplicateAsError: true);
                    }
                    else
                    {
                        pendingValueFlag = shortFlagDefinition;
                        pendingValueContext = [program, .. commandSegments];
                    }

                    routeFinalized = true;
                    break;

                case StackedFlagsToken stackedFlagsToken:
                    foreach (var shortName in stackedFlagsToken.Chars)
                    {
                        if (!TryFindFlag(flag => string.Equals(flag.Short, shortName, StringComparison.Ordinal), currentFlags, out var stackedFlag))
                        {
                            throw UnknownFlag(program, commandSegments, $"-{shortName}");
                        }

                        if (stackedFlag.Type is not (ValueType.Boolean or ValueType.Count))
                        {
                            throw new ParseErrors([new ParseError("invalid_stack", $"Flag -{shortName} cannot appear inside a stacked flag token.", null, [program, .. commandSegments])]);
                        }

                        ApplyFlagPresence(stackedFlag, parsedFlags, explicitFlags, [program, .. commandSegments], duplicateAsError: false);
                    }

                    routeFinalized = true;
                    break;

                case UnknownFlagToken unknownFlagToken:
                    throw UnknownFlag(program, commandSegments, unknownFlagToken.Raw);
            }
        }

        if (pendingValueFlag is not null)
        {
            throw new ParseErrors([new ParseError("invalid_value", $"Flag {DisplayFlag(pendingValueFlag)} expects a value.", null, pendingValueContext)]);
        }

        var context = new List<string> { program };
        context.AddRange(commandSegments);
        var activeArguments = currentCommand?.Arguments ?? spec.Arguments;
        var activeGroups = currentCommand?.MutuallyExclusiveGroups ?? spec.MutuallyExclusiveGroups;
        var positionalResolution = new PositionalResolver(activeArguments).Resolve(positionalTokens, parsedFlags, context);
        var validationErrors = new FlagValidator(currentFlags, activeGroups).Validate(parsedFlags, context);
        var allErrors = positionalResolution.Errors.Concat(validationErrors).ToList();
        if (allErrors.Count > 0)
        {
            throw new ParseErrors(allErrors);
        }

        return new ParseResult(program, [program, .. commandSegments], parsedFlags, positionalResolution.Result, explicitFlags);
    }

    private static IReadOnlyList<FlagDef> BuildActiveFlags(CliSpec spec, CommandDef? command)
    {
        var global = command is null || command.InheritGlobalFlags ? spec.GlobalFlags : [];
        var local = command?.Flags ?? spec.Flags;
        return global.Concat(local).ToList();
    }

    private static Dictionary<string, object?> InitializeFlagValues(IEnumerable<FlagDef> flags)
    {
        var values = new Dictionary<string, object?>(StringComparer.Ordinal);
        foreach (var flag in flags)
        {
            values[flag.Id] = flag.Repeatable
                ? new List<object?>()
                : flag.Type switch
                {
                    ValueType.Boolean => false,
                    ValueType.Count => 0L,
                    _ => flag.Default,
                };
        }

        return values;
    }

    private static Dictionary<string, object?> PreserveMatchingFlagValues(IReadOnlyDictionary<string, object?> previousValues, IEnumerable<FlagDef> activeFlags)
    {
        var next = InitializeFlagValues(activeFlags);
        foreach (var flag in activeFlags)
        {
            if (previousValues.TryGetValue(flag.Id, out var value))
            {
                next[flag.Id] = value;
            }
        }

        return next;
    }

    private static bool TryResolveCommand(string token, IReadOnlyList<CommandDef> commands, out CommandDef command)
    {
        command = commands.FirstOrDefault(candidate => candidate.Name == token || candidate.Aliases.Contains(token, StringComparer.Ordinal))!;
        return command is not null;
    }

    private static bool TryFindFlag(Func<FlagDef, bool> predicate, IReadOnlyList<FlagDef> flags, out FlagDef flag)
    {
        flag = flags.FirstOrDefault(predicate)!;
        return flag is not null;
    }

    private static IReadOnlyList<FlagDef> BuiltinFlagsFor(CliSpec spec)
    {
        var flags = new List<FlagDef>();
        if (spec.BuiltinFlags.Help)
        {
            flags.Add(new FlagDef("__builtin_help", "h", "help", null, "Show help.", ValueType.Boolean, false, null, null, [], null, [], [], [], false));
        }

        if (spec.BuiltinFlags.Version && !string.IsNullOrWhiteSpace(spec.Version))
        {
            flags.Add(new FlagDef("__builtin_version", null, "version", null, "Show version.", ValueType.Boolean, false, null, null, [], null, [], [], [], false));
        }

        return flags;
    }

    private static void ApplyFlagPresence(
        FlagDef flag,
        IDictionary<string, object?> parsedFlags,
        IList<string> explicitFlags,
        IReadOnlyList<string> context,
        bool duplicateAsError)
    {
        if (flag.Type == ValueType.Count)
        {
            var current = parsedFlags.TryGetValue(flag.Id, out var value) && value is long currentCount ? currentCount : 0L;
            parsedFlags[flag.Id] = current + 1;
            explicitFlags.Add(flag.Id);
            return;
        }

        if (flag.Repeatable)
        {
            var values = parsedFlags.TryGetValue(flag.Id, out var existing) && existing is List<object?> list ? list : [];
            values.Add(true);
            parsedFlags[flag.Id] = values;
            explicitFlags.Add(flag.Id);
            return;
        }

        if (duplicateAsError && FlagPresence.IsPresent(flag, TryGetValue(parsedFlags, flag.Id)))
        {
            throw new ParseErrors([new ParseError("duplicate_flag", $"Flag {DisplayFlag(flag)} was provided more than once.", null, context)]);
        }

        parsedFlags[flag.Id] = true;
        explicitFlags.Add(flag.Id);
    }

    private static void ApplyFlagValue(
        FlagDef flag,
        string rawValue,
        IDictionary<string, object?> parsedFlags,
        IList<string> explicitFlags,
        IReadOnlyList<string> context,
        bool duplicateAsError)
    {
        var coercion = PositionalResolver.CoerceValue(rawValue, flag.Type, flag.Id, context, flag.EnumValues);
        if (coercion.Error is not null)
        {
            throw new ParseErrors([coercion.Error]);
        }

        if (flag.Repeatable)
        {
            var values = parsedFlags.TryGetValue(flag.Id, out var existing) && existing is List<object?> list ? list : [];
            values.Add(coercion.Value);
            parsedFlags[flag.Id] = values;
            explicitFlags.Add(flag.Id);
            return;
        }

        if (duplicateAsError && FlagPresence.IsPresent(flag, TryGetValue(parsedFlags, flag.Id)))
        {
            throw new ParseErrors([new ParseError("duplicate_flag", $"Flag {DisplayFlag(flag)} was provided more than once.", null, context)]);
        }

        parsedFlags[flag.Id] = coercion.Value;
        explicitFlags.Add(flag.Id);
    }

    private static ParseErrors UnknownFlag(string program, IReadOnlyList<string> commandSegments, string rawFlag)
    {
        return new ParseErrors([new ParseError("unknown_flag", $"Unknown flag '{rawFlag}'.", null, [program, .. commandSegments])]);
    }

    private static object? TryGetValue(IDictionary<string, object?> values, string key)
    {
        return values.TryGetValue(key, out var value) ? value : null;
    }

    private static string DisplayFlag(FlagDef flag)
    {
        if (!string.IsNullOrWhiteSpace(flag.Long))
        {
            return $"--{flag.Long}";
        }

        if (!string.IsNullOrWhiteSpace(flag.Short))
        {
            return $"-{flag.Short}";
        }

        return $"-{flag.SingleDashLong}";
    }
}

internal static class FlagPresence
{
    public static bool IsPresent(FlagDef flag, object? value)
    {
        if (flag.Repeatable)
        {
            return value is IList<object?> list && list.Count > 0;
        }

        return flag.Type switch
        {
            ValueType.Boolean => value is true,
            ValueType.Count => value is long count && count > 0,
            _ => IsPresent(value),
        };
    }

    public static bool IsPresent(object? value)
    {
        return value switch
        {
            null => false,
            bool boolValue => boolValue,
            string stringValue => !string.IsNullOrEmpty(stringValue),
            IList<object?> list => list.Count > 0,
            long integerValue => integerValue > 0,
            _ => true,
        };
    }
}
