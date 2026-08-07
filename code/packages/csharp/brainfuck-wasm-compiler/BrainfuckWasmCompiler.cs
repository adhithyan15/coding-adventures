using CodingAdventures.WasmLeb128;
using CodingAdventures.WasmTypes;
using WasmValueType = CodingAdventures.WasmTypes.ValueType;

namespace CodingAdventures.BrainfuckWasmCompiler;

public static class BrainfuckWasmCompilerVersion
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
}

public sealed class PackageResult
{
    private readonly byte[] _wasmBytes;

    internal PackageResult(string source, IEnumerable<char> operations, byte[] wasmBytes, string? wasmPath)
    {
        Source = source;
        Operations = Array.AsReadOnly(operations.ToArray());
        _wasmBytes = wasmBytes.ToArray();
        WasmPath = wasmPath;
    }

    public string Source { get; }

    public IReadOnlyList<char> Operations { get; }

    public byte[] WasmBytes => _wasmBytes.ToArray();

    public string? WasmPath { get; }
}

public static class BrainfuckWasmCompiler
{
    private const int MaxSourceLength = 1_000_000;
    private const int MaxLoopNesting = 512;
    private const string WasiModule = "wasi_snapshot_preview1";

    public static PackageResult CompileSource(string source)
    {
        ArgumentNullException.ThrowIfNull(source);
        var program = Parse(source);
        return new PackageResult(source, program.Operations, EmitModule(program), null);
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

        return new PackageResult(result.Source, result.Operations, result.WasmBytes, path);
    }

    private static ParsedProgram Parse(string source)
    {
        if (source.Length > MaxSourceLength)
        {
            throw new PackageError("parse", $"source exceeds {MaxSourceLength} characters");
        }

        var operations = new List<char>();
        var stack = new Stack<int>();
        var loopEnds = new Dictionary<int, int>();

        for (var sourceIndex = 0; sourceIndex < source.Length; sourceIndex++)
        {
            var operation = source[sourceIndex];
            if (!"><+-.,[]".Contains(operation, StringComparison.Ordinal))
            {
                continue;
            }

            var operationIndex = operations.Count;
            if (operation == '[')
            {
                stack.Push(operationIndex);
                if (stack.Count > MaxLoopNesting)
                {
                    throw new PackageError("parse", $"loop nesting exceeds {MaxLoopNesting}");
                }
            }
            else if (operation == ']')
            {
                if (stack.Count == 0)
                {
                    throw new PackageError("parse", $"unmatched ] at byte {sourceIndex}");
                }

                loopEnds[stack.Pop()] = operationIndex;
            }

            operations.Add(operation);
        }

        if (stack.Count > 0)
        {
            throw new PackageError("parse", "unmatched [");
        }

        return new ParsedProgram(operations, loopEnds);
    }

    private static byte[] EmitModule(ParsedProgram program)
    {
        var needsWrite = program.Operations.Contains('.');
        var needsRead = program.Operations.Contains(',');
        var importCount = (needsWrite ? 1 : 0) + (needsRead ? 1 : 0);
        var writeIndex = needsWrite ? 0 : -1;
        var readIndex = needsRead ? (needsWrite ? 1 : 0) : -1;
        var module = new WasmModule();
        var wasiType = new FuncType(
            [WasmValueType.I32, WasmValueType.I32, WasmValueType.I32, WasmValueType.I32],
            [WasmValueType.I32]);

        if (needsWrite)
        {
            module.Types.Add(wasiType);
            module.Imports.Add(new Import(
                WasiModule,
                "fd_write",
                ExternalKind.FUNCTION,
                new FunctionImportDescriptor(writeIndex)));
        }

        if (needsRead)
        {
            module.Types.Add(wasiType);
            module.Imports.Add(new Import(
                WasiModule,
                "fd_read",
                ExternalKind.FUNCTION,
                new FunctionImportDescriptor(readIndex)));
        }

        module.Types.Add(new FuncType([], []));
        module.Functions.Add(importCount);
        module.Memories.Add(new MemoryType(new Limits(1, null)));
        module.Exports.Add(new Export("_start", ExternalKind.FUNCTION, importCount));
        module.Exports.Add(new Export("memory", ExternalKind.MEMORY, 0));
        module.Code.Add(new FunctionBody(
            [WasmValueType.I32, WasmValueType.I32, WasmValueType.I32],
            FunctionBody(program, writeIndex, readIndex)));

        return WasmModuleEncoder.WasmModuleEncoder.EncodeModule(module);
    }

    private static byte[] FunctionBody(ParsedProgram program, int writeIndex, int readIndex)
    {
        var body = new List<byte>();
        EmitOperations(body, program, 0, program.Operations.Count, writeIndex, readIndex);
        body.Add(0x0B);
        return body.ToArray();
    }

    private static void EmitOperations(
        List<byte> output,
        ParsedProgram program,
        int start,
        int end,
        int writeIndex,
        int readIndex)
    {
        var index = start;
        while (index < end)
        {
            switch (program.Operations[index])
            {
                case '>':
                    AddToLocal(output, 0, 1);
                    break;
                case '<':
                    AddToLocal(output, 0, -1);
                    break;
                case '+':
                    MutateCell(output, 1);
                    break;
                case '-':
                    MutateCell(output, -1);
                    break;
                case '.':
                    EmitWrite(output, writeIndex);
                    break;
                case ',':
                    EmitRead(output, readIndex);
                    break;
                case '[':
                    var close = program.LoopEnds[index];
                    output.AddRange([0x02, 0x40, 0x03, 0x40]);
                    LoadCell(output);
                    output.Add(0x45);
                    output.Add(0x0D);
                    U32(output, 1);
                    EmitOperations(output, program, index + 1, close, writeIndex, readIndex);
                    output.Add(0x0C);
                    U32(output, 0);
                    output.AddRange([0x0B, 0x0B]);
                    index = close;
                    break;
            }

            index++;
        }
    }

    private static void LoadCell(List<byte> output)
    {
        output.Add(0x20);
        U32(output, 0);
        output.Add(0x2D);
        U32(output, 0);
        U32(output, 0);
    }

    private static void MutateCell(List<byte> output, int delta)
    {
        LoadCell(output);
        I32(output, delta);
        output.Add(0x6A);
        output.Add(0x21);
        U32(output, 1);
        output.Add(0x20);
        U32(output, 0);
        output.Add(0x20);
        U32(output, 1);
        output.Add(0x3A);
        U32(output, 0);
        U32(output, 0);
    }

    private static void AddToLocal(List<byte> output, int local, int delta)
    {
        output.Add(0x20);
        U32(output, local);
        I32(output, delta);
        output.Add(0x6A);
        output.Add(0x21);
        U32(output, local);
    }

    private static void EmitWrite(List<byte> output, int writeIndex)
    {
        LoadCell(output);
        output.Add(0x21);
        U32(output, 1);
        StoreByteConstAddress(output, 30012, 1);
        StoreI32Const(output, 30000, 30012);
        StoreI32Const(output, 30004, 1);
        I32(output, 1);
        I32(output, 30000);
        I32(output, 1);
        I32(output, 30008);
        output.Add(0x10);
        U32(output, writeIndex);
        output.Add(0x21);
        U32(output, 2);
    }

    private static void EmitRead(List<byte> output, int readIndex)
    {
        StoreByteConstAddress(output, 30012, 0);
        StoreI32Const(output, 30000, 30012);
        StoreI32Const(output, 30004, 1);
        I32(output, 0);
        I32(output, 30000);
        I32(output, 1);
        I32(output, 30008);
        output.Add(0x10);
        U32(output, readIndex);
        output.Add(0x21);
        U32(output, 2);
        I32(output, 30012);
        output.Add(0x2D);
        U32(output, 0);
        U32(output, 0);
        output.Add(0x21);
        U32(output, 1);
        output.Add(0x20);
        U32(output, 0);
        output.Add(0x20);
        U32(output, 1);
        output.Add(0x3A);
        U32(output, 0);
        U32(output, 0);
    }

    private static void StoreByteConstAddress(List<byte> output, int address, int local)
    {
        I32(output, address);
        output.Add(0x20);
        U32(output, local);
        output.Add(0x3A);
        U32(output, 0);
        U32(output, 0);
    }

    private static void StoreI32Const(List<byte> output, int address, int value)
    {
        I32(output, address);
        I32(output, value);
        output.Add(0x36);
        U32(output, 2);
        U32(output, 0);
    }

    private static void I32(List<byte> output, int value)
    {
        output.Add(0x41);
        output.AddRange(WasmLeb128.WasmLeb128.EncodeSigned(value));
    }

    private static void U32(List<byte> output, int value) =>
        output.AddRange(WasmLeb128.WasmLeb128.EncodeUnsigned(value));

    private sealed record ParsedProgram(IReadOnlyList<char> Operations, IReadOnlyDictionary<int, int> LoopEnds);
}
