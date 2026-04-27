using System.Text;
using CodingAdventures.WasmExecution;
using CodingAdventures.WasmModuleParser;
using CodingAdventures.WasmTypes;
using CodingAdventures.WasmValidator;
using WasmValueType = CodingAdventures.WasmTypes.ValueType;

namespace CodingAdventures.WasmRuntime;

public static class WasmRuntimeVersion
{
    public const string VERSION = "0.1.0";
}

public sealed class ProcExitError : Exception
{
    public ProcExitError(int exitCode) : base($"proc_exit({exitCode})")
    {
        ExitCode = exitCode;
    }

    public int ExitCode { get; }
}

public interface IWasiClock
{
    long RealtimeNanoseconds();

    long MonotonicNanoseconds();
}

public interface IWasiRandom
{
    void FillBytes(byte[] buffer);
}

public sealed class SystemClock : IWasiClock
{
    public long RealtimeNanoseconds() => DateTimeOffset.UtcNow.ToUnixTimeMilliseconds() * 1_000_000;

    public long MonotonicNanoseconds() => DateTime.UtcNow.Ticks * 100;
}

public sealed class SystemRandom : IWasiRandom
{
    public void FillBytes(byte[] buffer) => Random.Shared.NextBytes(buffer);
}

public sealed class WasiConfig
{
    public IReadOnlyList<string> Args { get; init; } = Array.Empty<string>();

    public IReadOnlyDictionary<string, string> Env { get; init; } = new Dictionary<string, string>();

    public Action<string> Stdout { get; init; } = static _ => { };

    public Action<string> Stderr { get; init; } = static _ => { };

    public IWasiClock Clock { get; init; } = new SystemClock();

    public IWasiRandom Random { get; init; } = new SystemRandom();
}

public sealed class WasiStub : IHostInterface
{
    private const int Esuccess = 0;
    private const int Enosys = 52;
    private readonly WasiConfig _config;
    private LinearMemory? _memory;

    public WasiStub(WasiConfig? config = null)
    {
        _config = config ?? new WasiConfig();
    }

    public void SetMemory(LinearMemory? memory) => _memory = memory;

    public IHostFunction? ResolveFunction(string moduleName, string name)
    {
        if (!string.Equals(moduleName, "wasi_snapshot_preview1", StringComparison.Ordinal))
        {
            return null;
        }

        return name switch
        {
            "proc_exit" => new HostFunction(
                WasmTypeFactory.MakeFuncType([WasmValueType.I32], Array.Empty<WasmValueType>()),
                args => throw new ProcExitError(args[0].AsI32())),
            "fd_write" => new HostFunction(
                WasmTypeFactory.MakeFuncType([WasmValueType.I32, WasmValueType.I32, WasmValueType.I32, WasmValueType.I32], [WasmValueType.I32]),
                HandleFdWrite),
            _ => new HostFunction(
                WasmTypeFactory.MakeFuncType(Array.Empty<WasmValueType>(), [WasmValueType.I32]),
                _ => [WasmValue.I32(Enosys)]),
        };
    }

    private IReadOnlyList<WasmValue> HandleFdWrite(IReadOnlyList<WasmValue> args)
    {
        if (_memory is null)
        {
            return [WasmValue.I32(Enosys)];
        }

        var fd = args[0].AsI32();
        var iovsPtr = args[1].AsI32();
        var iovsLen = args[2].AsI32();
        var nwrittenPtr = args[3].AsI32();
        var totalBytes = 0;

        for (var i = 0; i < iovsLen; i++)
        {
            var bufPtr = _memory.LoadI32(iovsPtr + i * 8);
            var bufLen = _memory.LoadI32(iovsPtr + i * 8 + 4);
            var bytes = _memory.ReadBytes(bufPtr, bufLen);
            var text = Encoding.UTF8.GetString(bytes);
            totalBytes += bufLen;
            if (fd == 1)
            {
                _config.Stdout(text);
            }
            else if (fd == 2)
            {
                _config.Stderr(text);
            }
        }

        _memory.StoreI32(nwrittenPtr, totalBytes);
        return [WasmValue.I32(Esuccess)];
    }
}

public sealed class WasmInstance
{
    public required ValidatedModule ValidatedModule { get; init; }

    public required WasmExecutionEngine Engine { get; init; }

    public required Dictionary<string, Export> Exports { get; init; }

    public LinearMemory? Memory => Engine.Memory;
}

public sealed class WasmRuntime
{
    private readonly IHostInterface? _hostInterface;
    private readonly WasmModuleParser.WasmModuleParser _parser = new();

    public WasmRuntime(IHostInterface? hostInterface = null)
    {
        _hostInterface = hostInterface;
    }

    public WasmModule Load(byte[] wasmBytes) => _parser.Parse(wasmBytes);

    public ValidatedModule Validate(WasmModule module) => WasmValidator.WasmValidator.Validate(module);

    public WasmInstance Instantiate(byte[] wasmBytes) => Instantiate(Load(wasmBytes));

    public WasmInstance Instantiate(WasmModule module)
    {
        var validated = Validate(module);
        var importedFunctions = new List<IHostFunction?>();
        var importedGlobals = new List<WasmValue>();
        var globalTypes = new List<GlobalType>();

        foreach (var importEntry in module.Imports)
        {
            switch (importEntry.Kind)
            {
                case ExternalKind.FUNCTION:
                {
                    var hostFunction = _hostInterface?.ResolveFunction(importEntry.ModuleName, importEntry.Name)
                        ?? throw new TrapError($"Missing host function import {importEntry.ModuleName}.{importEntry.Name}");
                    importedFunctions.Add(hostFunction);
                    break;
                }
                default:
                    throw new TrapError($"Imported {importEntry.Kind} values are not implemented in the C# runtime yet");
            }
        }

        var memory = module.Memories.Count > 0 ? new LinearMemory(module.Memories[0].Limits.Min, module.Memories[0].Limits.Max) : null;
        if (_hostInterface is WasiStub wasi)
        {
            wasi.SetMemory(memory);
        }

        foreach (var importedFunction in importedFunctions)
        {
            if (importedFunction is null)
            {
                throw new TrapError("Null host function import");
            }
        }

        globalTypes.AddRange(importedGlobals.Select(_ => new GlobalType(WasmValueType.I32, false)));
        foreach (var global in module.Globals)
        {
            var value = WasmExecution.WasmExecution.EvaluateConstExpr(global.InitExpr, importedGlobals).Single();
            importedGlobals.Add(value);
            globalTypes.Add(global.GlobalType);
        }

        var tables = module.Tables.Select(table => new Table(table.Limits.Min)).ToArray();
        foreach (var element in module.Elements)
        {
            if (tables.Length == 0)
            {
                throw new TrapError("Element segment found but the module has no table");
            }

            var tableOffset = WasmExecution.WasmExecution.EvaluateConstExpr(element.OffsetExpr, Array.Empty<WasmValue>()).Single().AsI32();
            for (var i = 0; i < element.FunctionIndices.Count; i++)
            {
                tables[element.TableIndex][tableOffset + i] = element.FunctionIndices[i];
            }
        }

        if (memory is not null)
        {
            foreach (var dataSegment in module.Data)
            {
                var offset = WasmExecution.WasmExecution.EvaluateConstExpr(dataSegment.OffsetExpr, Array.Empty<WasmValue>()).Single().AsI32();
                memory.WriteBytes(offset, dataSegment.Data);
            }
        }

        var funcBodies = new List<FunctionBody?>(validated.FuncTypes.Count);
        var hostFunctions = new List<IHostFunction?>(validated.FuncTypes.Count);
        for (var i = 0; i < importedFunctions.Count; i++)
        {
            funcBodies.Add(null);
            hostFunctions.Add(importedFunctions[i]);
        }

        foreach (var body in module.Code)
        {
            funcBodies.Add(body);
            hostFunctions.Add(null);
        }

        var engine = new WasmExecutionEngine(new WasmExecutionEngineOptions
        {
            Memory = memory,
            Tables = tables,
            Globals = importedGlobals,
            GlobalTypes = globalTypes,
            FuncTypes = validated.FuncTypes,
            FuncBodies = funcBodies,
            HostFunctions = hostFunctions,
        });

        var exports = module.Exports.ToDictionary(entry => entry.Name, entry => entry, StringComparer.Ordinal);
        var instance = new WasmInstance
        {
            ValidatedModule = validated,
            Engine = engine,
            Exports = exports,
        };

        if (module.Start.HasValue)
        {
            engine.CallFunction(module.Start.Value, Array.Empty<WasmValue>());
        }

        return instance;
    }

    public IReadOnlyList<object> Call(WasmInstance instance, string exportName, params int[] args)
    {
        if (!instance.Exports.TryGetValue(exportName, out var exportEntry))
        {
            throw new TrapError($"Module does not export '{exportName}'");
        }

        if (exportEntry.Kind != ExternalKind.FUNCTION)
        {
            throw new TrapError($"Export '{exportName}' is not a function");
        }

        var funcType = instance.ValidatedModule.FuncTypes[exportEntry.Index];
        var wasmArgs = args.Select((value, index) =>
        {
            if (funcType.Params[index] != WasmValueType.I32)
            {
                throw new TrapError("The convenience Call overload currently supports only i32 arguments");
            }

            return WasmValue.I32(value);
        }).ToArray();

        var rawResults = instance.Engine.CallFunction(exportEntry.Index, wasmArgs);
        return rawResults.Select((result, index) => funcType.Results[index] switch
        {
            WasmValueType.I32 => (object)result.AsI32(),
            WasmValueType.I64 => result.AsI64(),
            WasmValueType.F32 => result.AsF32(),
            WasmValueType.F64 => result.AsF64(),
            _ => throw new TrapError($"Unsupported result type {funcType.Results[index]}"),
        }).ToArray();
    }

    public IReadOnlyList<object> LoadAndRun(byte[] wasmBytes, string exportName, params int[] args)
    {
        var instance = Instantiate(wasmBytes);
        return Call(instance, exportName, args);
    }
}
