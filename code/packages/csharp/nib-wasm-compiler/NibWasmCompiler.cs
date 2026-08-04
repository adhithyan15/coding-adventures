using System.Collections.ObjectModel;
using System.Text.RegularExpressions;
using CodingAdventures.WasmLeb128;
using CodingAdventures.WasmTypes;
using WasmValueType = CodingAdventures.WasmTypes.ValueType;

namespace CodingAdventures.NibWasmCompiler;

public static class NibWasmCompilerVersion
{
    public const string VERSION = "0.1.0";
}

public sealed class PackageError : Exception
{
    public PackageError(string stage, string message) : base(message)
    {
        Stage = stage;
    }

    public string Stage { get; }

    public override string ToString() => $"[{Stage}] {Message}";
}

public sealed class NibFunction
{
    internal NibFunction(string name, IEnumerable<string> parameters, string expression)
    {
        Name = name;
        Parameters = Array.AsReadOnly(parameters.ToArray());
        Expression = expression.Trim();
    }

    public string Name { get; }
    public IReadOnlyList<string> Parameters { get; }
    public string Expression { get; }
}

public sealed class PackageResult
{
    private readonly byte[] _wasmBytes;

    internal PackageResult(string source, IEnumerable<NibFunction> functions, byte[] wasmBytes, string? wasmPath)
    {
        Source = source;
        Functions = new ReadOnlyCollection<NibFunction>(functions.ToArray());
        _wasmBytes = wasmBytes.ToArray();
        WasmPath = wasmPath;
    }

    public string Source { get; }
    public IReadOnlyList<NibFunction> Functions { get; }
    public byte[] WasmBytes => _wasmBytes.ToArray();
    public string? WasmPath { get; }
}

public static class NibWasmCompiler
{
    private const int MaxSourceLength = 1_000_000;
    private const int MaxExpressionNesting = 256;
    private static readonly Regex FunctionPattern = new(
        @"fn\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(([^)]*)\)\s*->\s*u4\s*\{\s*return\s+([^;]+);\s*\}",
        RegexOptions.Singleline | RegexOptions.CultureInvariant);
    private static readonly Regex IdentifierPattern = new(
        @"\A[A-Za-z_][A-Za-z0-9_]*\z",
        RegexOptions.CultureInvariant);
    private static readonly Regex CallPattern = new(
        @"\A([A-Za-z_][A-Za-z0-9_]*)\s*\((.*)\)\z",
        RegexOptions.Singleline | RegexOptions.CultureInvariant);

    public static PackageResult CompileSource(string source)
    {
        ArgumentNullException.ThrowIfNull(source);
        var functions = Parse(source);
        var index = IndexFunctions(functions);
        foreach (var function in functions)
        {
            EmitExpression([], function.Expression, index, ParameterMap(function), false, 0);
        }

        return new PackageResult(source, functions, EmitModule(functions, index), null);
    }

    public static PackageResult PackSource(string source) => CompileSource(source);

    public static PackageResult WriteWasmFile(string source, string path)
    {
        ArgumentNullException.ThrowIfNull(path);
        var result = CompileSource(source);
        try
        {
            File.WriteAllBytes(path, result.WasmBytes);
        }
        catch (Exception error) when (error is IOException or UnauthorizedAccessException)
        {
            throw new PackageError("write", error.Message);
        }

        return new PackageResult(result.Source, result.Functions, result.WasmBytes, path);
    }

    private static IReadOnlyList<NibFunction> Parse(string source)
    {
        if (source.Length > MaxSourceLength)
        {
            throw new PackageError("parse", $"source exceeds {MaxSourceLength} characters");
        }

        var functions = new List<NibFunction>();
        var cursor = 0;
        foreach (Match match in FunctionPattern.Matches(source))
        {
            if (!string.IsNullOrWhiteSpace(source[cursor..match.Index]))
            {
                throw new PackageError("parse", "unexpected text before function");
            }

            functions.Add(new NibFunction(match.Groups[1].Value, ParseParameters(match.Groups[2].Value), match.Groups[3].Value));
            cursor = match.Index + match.Length;
        }

        if (functions.Count == 0 || !string.IsNullOrWhiteSpace(source[cursor..]))
        {
            throw new PackageError("parse", "expected one or more Nib functions");
        }

        return functions;
    }

    private static IReadOnlyList<string> ParseParameters(string text)
    {
        if (string.IsNullOrWhiteSpace(text))
        {
            return [];
        }

        var parameters = new List<string>();
        foreach (var piece in text.Split(','))
        {
            var parts = Regex.Split(piece.Trim(), @"\s*:\s*");
            if (parts.Length != 2 || parts[1] != "u4" || !IdentifierPattern.IsMatch(parts[0]))
            {
                throw new PackageError("parse", "parameters must be `name: u4`");
            }

            if (parameters.Contains(parts[0], StringComparer.Ordinal))
            {
                throw new PackageError("validate", $"duplicate parameter `{parts[0]}`");
            }

            parameters.Add(parts[0]);
        }

        return parameters;
    }

    private static IReadOnlyDictionary<string, (NibFunction Function, int Index)> IndexFunctions(IReadOnlyList<NibFunction> functions)
    {
        var result = new Dictionary<string, (NibFunction, int)>(StringComparer.Ordinal);
        for (var index = 0; index < functions.Count; index++)
        {
            if (!result.TryAdd(functions[index].Name, (functions[index], index)))
            {
                throw new PackageError("validate", $"duplicate function `{functions[index].Name}`");
            }
        }

        return result;
    }

    private static IReadOnlyDictionary<string, int> ParameterMap(NibFunction function) =>
        function.Parameters.Select((name, index) => (name, index)).ToDictionary(item => item.name, item => item.index, StringComparer.Ordinal);

    private static byte[] EmitModule(
        IReadOnlyList<NibFunction> functions,
        IReadOnlyDictionary<string, (NibFunction Function, int Index)> index)
    {
        var module = new WasmModule();
        for (var functionIndex = 0; functionIndex < functions.Count; functionIndex++)
        {
            var function = functions[functionIndex];
            module.Types.Add(new FuncType(
                Enumerable.Repeat(WasmValueType.I32, function.Parameters.Count),
                [WasmValueType.I32]));
            module.Functions.Add(functionIndex);
            module.Exports.Add(new Export(function.Name, ExternalKind.FUNCTION, functionIndex));

            var body = new List<byte>();
            EmitExpression(body, function.Expression, index, ParameterMap(function), true, 0);
            body.Add(0x0B);
            module.Code.Add(new FunctionBody([], body.ToArray()));
        }

        return WasmModuleEncoder.WasmModuleEncoder.EncodeModule(module);
    }

    private static void EmitExpression(
        List<byte> output,
        string expression,
        IReadOnlyDictionary<string, (NibFunction Function, int Index)> functions,
        IReadOnlyDictionary<string, int> parameters,
        bool emit,
        int depth)
    {
        if (depth > MaxExpressionNesting)
        {
            throw new PackageError("validate", $"expression nesting exceeds {MaxExpressionNesting}");
        }

        var addition = SplitTopLevel(expression, "+%");
        if (addition.Count > 1)
        {
            EmitExpression(output, addition[0], functions, parameters, emit, depth + 1);
            foreach (var part in addition.Skip(1))
            {
                EmitExpression(output, part, functions, parameters, emit, depth + 1);
                if (emit)
                {
                    output.Add(0x6A);
                    I32(output, 15);
                    output.Add(0x71);
                }
            }

            return;
        }

        var trimmed = expression.Trim();
        if (int.TryParse(trimmed, out var literal))
        {
            if (literal is < 0 or > 15)
            {
                throw new PackageError("validate", $"u4 literal out of range: {literal}");
            }

            if (emit)
            {
                I32(output, literal);
            }

            return;
        }

        var call = CallPattern.Match(trimmed);
        if (call.Success)
        {
            if (!functions.TryGetValue(call.Groups[1].Value, out var target))
            {
                throw new PackageError("validate", $"unknown function `{call.Groups[1].Value}`");
            }

            var arguments = SplitArguments(call.Groups[2].Value);
            if (arguments.Count != target.Function.Parameters.Count)
            {
                throw new PackageError("validate", $"wrong arity for `{target.Function.Name}`");
            }

            foreach (var argument in arguments)
            {
                EmitExpression(output, argument, functions, parameters, emit, depth + 1);
            }

            if (emit)
            {
                output.Add(0x10);
                U32(output, target.Index);
            }

            return;
        }

        if (parameters.TryGetValue(trimmed, out var parameterIndex))
        {
            if (emit)
            {
                output.Add(0x20);
                U32(output, parameterIndex);
            }

            return;
        }

        throw new PackageError("validate", $"unsupported expression `{expression}`");
    }

    private static IReadOnlyList<string> SplitArguments(string text) =>
        string.IsNullOrWhiteSpace(text) ? [] : SplitTopLevel(text, ",");

    private static IReadOnlyList<string> SplitTopLevel(string text, string delimiter)
    {
        var parts = new List<string>();
        var nesting = 0;
        var start = 0;
        for (var index = 0; index < text.Length; index++)
        {
            nesting += text[index] switch { '(' => 1, ')' => -1, _ => 0 };
            if (nesting == 0 && text.AsSpan(index).StartsWith(delimiter, StringComparison.Ordinal))
            {
                parts.Add(text[start..index].Trim());
                index += delimiter.Length - 1;
                start = index + 1;
            }
        }

        parts.Add(text[start..].Trim());
        return parts;
    }

    private static void I32(List<byte> output, int value)
    {
        output.Add(0x41);
        output.AddRange(WasmLeb128.WasmLeb128.EncodeSigned(value));
    }

    private static void U32(List<byte> output, int value) =>
        output.AddRange(WasmLeb128.WasmLeb128.EncodeUnsigned(value));
}
