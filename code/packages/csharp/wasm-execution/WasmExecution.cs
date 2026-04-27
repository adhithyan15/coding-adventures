using System.Buffers.Binary;
using CodingAdventures.WasmLeb128;
using CodingAdventures.WasmOpcodes;
using CodingAdventures.WasmTypes;
using WasmValueType = CodingAdventures.WasmTypes.ValueType;

namespace CodingAdventures.WasmExecution;

public static class WasmExecutionVersion
{
    public const string VERSION = "0.1.0";
}

public sealed class TrapError : Exception
{
    public TrapError(string message) : base(message)
    {
    }
}

public readonly struct WasmValue : IEquatable<WasmValue>
{
    private readonly long _bits;

    private WasmValue(WasmValueType type, long bits)
    {
        Type = type;
        _bits = bits;
    }

    public WasmValueType Type { get; }

    public object Value => Type switch
    {
        WasmValueType.I32 => AsI32(),
        WasmValueType.I64 => AsI64(),
        WasmValueType.F32 => AsF32(),
        WasmValueType.F64 => AsF64(),
        _ => throw new TrapError($"Unsupported wasm value type: {Type}"),
    };

    public static WasmValue I32(int value) => new(WasmValueType.I32, value);

    public static WasmValue I64(long value) => new(WasmValueType.I64, value);

    public static WasmValue F32(float value) => new(WasmValueType.F32, BitConverter.SingleToInt32Bits(value));

    public static WasmValue F64(double value) => new(WasmValueType.F64, BitConverter.DoubleToInt64Bits(value));

    public static WasmValue DefaultFor(WasmValueType type) => type switch
    {
        WasmValueType.I32 => I32(0),
        WasmValueType.I64 => I64(0),
        WasmValueType.F32 => F32(0),
        WasmValueType.F64 => F64(0),
        _ => throw new TrapError($"Unsupported wasm value type: {type}"),
    };

    public int AsI32()
    {
        EnsureType(WasmValueType.I32);
        return unchecked((int)_bits);
    }

    public long AsI64()
    {
        EnsureType(WasmValueType.I64);
        return _bits;
    }

    public float AsF32()
    {
        EnsureType(WasmValueType.F32);
        return BitConverter.Int32BitsToSingle(unchecked((int)_bits));
    }

    public double AsF64()
    {
        EnsureType(WasmValueType.F64);
        return BitConverter.Int64BitsToDouble(_bits);
    }

    public bool Equals(WasmValue other) => Type == other.Type && _bits == other._bits;

    public override bool Equals(object? obj) => obj is WasmValue other && Equals(other);

    public override int GetHashCode() => HashCode.Combine((int)Type, _bits);

    public override string ToString() => $"{Type}:{Value}";

    private void EnsureType(WasmValueType expected)
    {
        if (Type != expected)
        {
            throw new TrapError($"Type mismatch: expected {expected}, got {Type}");
        }
    }
}

public sealed class LinearMemory
{
    public const int PAGE_SIZE = 65536;

    private byte[] _data;
    private readonly int? _maxPages;

    public LinearMemory(int initialPages, int? maxPages = null)
    {
        if (initialPages < 0)
        {
            throw new ArgumentOutOfRangeException(nameof(initialPages));
        }

        _data = new byte[checked(initialPages * PAGE_SIZE)];
        CurrentPages = initialPages;
        _maxPages = maxPages;
    }

    public int CurrentPages { get; private set; }

    public int Size() => CurrentPages;

    public int ByteLength() => _data.Length;

    public int Grow(int deltaPages)
    {
        if (deltaPages < 0)
        {
            throw new TrapError("Memory cannot grow by a negative number of pages");
        }

        var oldPages = CurrentPages;
        var newPages = CurrentPages + deltaPages;
        if (_maxPages.HasValue && newPages > _maxPages.Value)
        {
            return -1;
        }

        if (newPages == CurrentPages)
        {
            return oldPages;
        }

        Array.Resize(ref _data, checked(newPages * PAGE_SIZE));
        CurrentPages = newPages;
        return oldPages;
    }

    public void WriteBytes(int offset, byte[] data)
    {
        EnsureBounds(offset, data.Length);
        Array.Copy(data, 0, _data, offset, data.Length);
    }

    public byte[] ReadBytes(int offset, int length)
    {
        EnsureBounds(offset, length);
        return _data[offset..(offset + length)];
    }

    public int LoadI32(int offset)
    {
        EnsureBounds(offset, 4);
        return BinaryPrimitives.ReadInt32LittleEndian(_data.AsSpan(offset, 4));
    }

    public void StoreI32(int offset, int value)
    {
        EnsureBounds(offset, 4);
        BinaryPrimitives.WriteInt32LittleEndian(_data.AsSpan(offset, 4), value);
    }

    public long LoadI64(int offset)
    {
        EnsureBounds(offset, 8);
        return BinaryPrimitives.ReadInt64LittleEndian(_data.AsSpan(offset, 8));
    }

    public void StoreI64(int offset, long value)
    {
        EnsureBounds(offset, 8);
        BinaryPrimitives.WriteInt64LittleEndian(_data.AsSpan(offset, 8), value);
    }

    public float LoadF32(int offset) => BitConverter.Int32BitsToSingle(LoadI32(offset));

    public void StoreF32(int offset, float value) => StoreI32(offset, BitConverter.SingleToInt32Bits(value));

    public double LoadF64(int offset) => BitConverter.Int64BitsToDouble(LoadI64(offset));

    public void StoreF64(int offset, double value) => StoreI64(offset, BitConverter.DoubleToInt64Bits(value));

    public int LoadI32_8s(int offset)
    {
        EnsureBounds(offset, 1);
        return unchecked((sbyte)_data[offset]);
    }

    public int LoadI32_8u(int offset)
    {
        EnsureBounds(offset, 1);
        return _data[offset];
    }

    public int LoadI32_16s(int offset)
    {
        EnsureBounds(offset, 2);
        return BinaryPrimitives.ReadInt16LittleEndian(_data.AsSpan(offset, 2));
    }

    public int LoadI32_16u(int offset)
    {
        EnsureBounds(offset, 2);
        return BinaryPrimitives.ReadUInt16LittleEndian(_data.AsSpan(offset, 2));
    }

    public long LoadI64_8s(int offset) => LoadI32_8s(offset);

    public long LoadI64_8u(int offset) => unchecked((byte)LoadI32_8u(offset));

    public long LoadI64_16s(int offset) => LoadI32_16s(offset);

    public long LoadI64_16u(int offset) => unchecked((ushort)LoadI32_16u(offset));

    public long LoadI64_32s(int offset) => LoadI32(offset);

    public long LoadI64_32u(int offset) => unchecked((uint)LoadI32(offset));

    public void StoreI32_8(int offset, int value)
    {
        EnsureBounds(offset, 1);
        _data[offset] = unchecked((byte)value);
    }

    public void StoreI32_16(int offset, int value)
    {
        EnsureBounds(offset, 2);
        BinaryPrimitives.WriteInt16LittleEndian(_data.AsSpan(offset, 2), unchecked((short)value));
    }

    public void StoreI64_8(int offset, long value) => StoreI32_8(offset, unchecked((int)value));

    public void StoreI64_16(int offset, long value) => StoreI32_16(offset, unchecked((int)value));

    public void StoreI64_32(int offset, long value) => StoreI32(offset, unchecked((int)value));

    private void EnsureBounds(int offset, int size)
    {
        if (offset < 0 || size < 0 || offset + size > _data.Length)
        {
            throw new TrapError($"Memory access out of bounds at offset {offset} for {size} bytes");
        }
    }
}

public sealed class Table
{
    private readonly int?[] _elements;

    public Table(int size)
    {
        _elements = new int?[size];
    }

    public int Length => _elements.Length;

    public int? this[int index]
    {
        get => _elements[index];
        set => _elements[index] = value;
    }
}

public interface IHostFunction
{
    FuncType Type { get; }

    IReadOnlyList<WasmValue> Call(IReadOnlyList<WasmValue> args);
}

public interface IHostInterface
{
    IHostFunction? ResolveFunction(string moduleName, string name);
}

public sealed class HostFunction : IHostFunction
{
    private readonly Func<IReadOnlyList<WasmValue>, IReadOnlyList<WasmValue>> _callback;

    public HostFunction(FuncType type, Func<IReadOnlyList<WasmValue>, IReadOnlyList<WasmValue>> callback)
    {
        Type = type;
        _callback = callback;
    }

    public FuncType Type { get; }

    public IReadOnlyList<WasmValue> Call(IReadOnlyList<WasmValue> args) => _callback(args);
}

public sealed record DecodedInstruction(
    OpcodeInfo Info,
    int Offset,
    int Size,
    int? Index = null,
    int? SecondaryIndex = null,
    int? Align = null,
    int? MemoryOffset = null,
    WasmValue? Constant = null);

public sealed class WasmExecutionEngineOptions
{
    public LinearMemory? Memory { get; init; }

    public required IReadOnlyList<Table> Tables { get; init; }

    public required IReadOnlyList<WasmValue> Globals { get; init; }

    public required IReadOnlyList<GlobalType> GlobalTypes { get; init; }

    public required IReadOnlyList<FuncType> FuncTypes { get; init; }

    public required IReadOnlyList<FunctionBody?> FuncBodies { get; init; }

    public required IReadOnlyList<IHostFunction?> HostFunctions { get; init; }
}

public static class WasmExecution
{
    public static IReadOnlyList<WasmValue> EvaluateConstExpr(byte[] expr, IReadOnlyList<WasmValue> importedGlobals)
    {
        var stack = new Stack<WasmValue>();
        foreach (var instruction in DecodeBytes(expr))
        {
            switch (instruction.Info.Name)
            {
                case "i32.const":
                case "i64.const":
                case "f32.const":
                case "f64.const":
                    stack.Push(instruction.Constant!.Value);
                    break;
                case "global.get":
                    if (!instruction.Index.HasValue || instruction.Index.Value < 0 || instruction.Index.Value >= importedGlobals.Count)
                    {
                        throw new TrapError($"Constant expression references unavailable imported global {instruction.Index}");
                    }

                    stack.Push(importedGlobals[instruction.Index.Value]);
                    break;
                case "end":
                    return stack.Reverse().ToArray();
                default:
                    throw new TrapError($"Opcode '{instruction.Info.Name}' is not allowed in constant expressions");
            }
        }

        throw new TrapError("Constant expression did not terminate with end");
    }

    public static IReadOnlyList<DecodedInstruction> DecodeFunctionBody(FunctionBody body) => DecodeBytes(body.Code);

    public static IReadOnlyDictionary<int, int> BuildControlFlowMap(FunctionBody body)
    {
        var instructions = DecodeFunctionBody(body);
        var pending = new Stack<int>();
        var map = new Dictionary<int, int>();
        foreach (var instruction in instructions)
        {
            switch (instruction.Info.Name)
            {
                case "block":
                case "loop":
                case "if":
                    pending.Push(instruction.Offset);
                    break;
                case "end":
                    if (pending.Count > 0)
                    {
                        var start = pending.Pop();
                        map[start] = instruction.Offset;
                        map[instruction.Offset] = start;
                    }
                    break;
            }
        }

        return map;
    }

    internal static IReadOnlyList<DecodedInstruction> DecodeBytes(byte[] code)
    {
        var instructions = new List<DecodedInstruction>();
        var offset = 0;
        while (offset < code.Length)
        {
            var start = offset;
            var opcode = code[offset++];
            var info = WasmOpcodes.WasmOpcodes.GetOpcode(opcode) ?? throw new TrapError($"Unknown opcode 0x{opcode:x2} at byte {start}");
            int? index = null;
            int? secondaryIndex = null;
            int? align = null;
            int? memoryOffset = null;
            WasmValue? constant = null;

            foreach (var immediate in info.Immediates)
            {
                switch (immediate)
                {
                    case "blocktype":
                        offset += 1;
                        break;
                    case "labelidx":
                    case "funcidx":
                    case "typeidx":
                    case "localidx":
                    case "globalidx":
                    case "tableidx":
                    case "memidx":
                    {
                        var (value, consumed) = WasmLeb128.WasmLeb128.DecodeUnsigned(code, offset);
                        offset += consumed;
                        if (!index.HasValue)
                        {
                            index = checked((int)value);
                        }
                        else
                        {
                            secondaryIndex = checked((int)value);
                        }
                        break;
                    }
                    case "memarg":
                    {
                        var (alignValue, alignConsumed) = WasmLeb128.WasmLeb128.DecodeUnsigned(code, offset);
                        offset += alignConsumed;
                        var (offsetValue, offsetConsumed) = WasmLeb128.WasmLeb128.DecodeUnsigned(code, offset);
                        offset += offsetConsumed;
                        align = checked((int)alignValue);
                        memoryOffset = checked((int)offsetValue);
                        break;
                    }
                    case "i32":
                    {
                        var (value, consumed) = WasmLeb128.WasmLeb128.DecodeSigned(code, offset);
                        offset += consumed;
                        constant = WasmValue.I32(value);
                        break;
                    }
                    case "i64":
                    {
                        var (value, consumed) = WasmLeb128.WasmLeb128.DecodeSigned(code, offset);
                        offset += consumed;
                        constant = WasmValue.I64(value);
                        break;
                    }
                    case "f32":
                        constant = WasmValue.F32(BitConverter.Int32BitsToSingle(BinaryPrimitives.ReadInt32LittleEndian(code.AsSpan(offset, 4))));
                        offset += 4;
                        break;
                    case "f64":
                        constant = WasmValue.F64(BitConverter.Int64BitsToDouble(BinaryPrimitives.ReadInt64LittleEndian(code.AsSpan(offset, 8))));
                        offset += 8;
                        break;
                    case "vec_labelidx":
                    {
                        var (count, consumed) = WasmLeb128.WasmLeb128.DecodeUnsigned(code, offset);
                        offset += consumed;
                        for (var i = 0; i < count + 1; i++)
                        {
                            var (_, size) = WasmLeb128.WasmLeb128.DecodeUnsigned(code, offset);
                            offset += size;
                        }
                        break;
                    }
                    default:
                        throw new TrapError($"Unsupported immediate kind '{immediate}'");
                }
            }

            instructions.Add(new DecodedInstruction(info, start, offset - start, index, secondaryIndex, align, memoryOffset, constant));
        }

        return instructions;
    }
}

public sealed class WasmExecutionEngine
{
    private readonly LinearMemory? _memory;
    private readonly IReadOnlyList<Table> _tables;
    private readonly WasmValue[] _globals;
    private readonly IReadOnlyList<GlobalType> _globalTypes;
    private readonly IReadOnlyList<FuncType> _funcTypes;
    private readonly IReadOnlyList<FunctionBody?> _funcBodies;
    private readonly IReadOnlyList<IHostFunction?> _hostFunctions;

    public WasmExecutionEngine(WasmExecutionEngineOptions options)
    {
        _memory = options.Memory;
        _tables = options.Tables;
        _globals = options.Globals.ToArray();
        _globalTypes = options.GlobalTypes;
        _funcTypes = options.FuncTypes;
        _funcBodies = options.FuncBodies;
        _hostFunctions = options.HostFunctions;

        if (_funcTypes.Count != _funcBodies.Count || _funcTypes.Count != _hostFunctions.Count)
        {
            throw new ArgumentException("FuncTypes, FuncBodies, and HostFunctions must have the same length");
        }
    }

    public IReadOnlyList<WasmValue> Globals => _globals;

    public LinearMemory? Memory => _memory;

    public IReadOnlyList<WasmValue> CallFunction(int functionIndex, IReadOnlyList<WasmValue> args)
    {
        EnsureFunctionIndex(functionIndex);
        var funcType = _funcTypes[functionIndex];
        if (funcType.Params.Count != args.Count)
        {
            throw new TrapError($"Function {functionIndex} expects {funcType.Params.Count} argument(s), got {args.Count}");
        }

        for (var i = 0; i < args.Count; i++)
        {
            if (args[i].Type != funcType.Params[i])
            {
                throw new TrapError($"Function {functionIndex} argument {i} expects {funcType.Params[i]}, got {args[i].Type}");
            }
        }

        if (_hostFunctions[functionIndex] is { } hostFunction)
        {
            return hostFunction.Call(args);
        }

        var body = _funcBodies[functionIndex] ?? throw new TrapError($"Function {functionIndex} has neither a body nor a host binding");
        return ExecuteBody(functionIndex, funcType, body, args);
    }

    private IReadOnlyList<WasmValue> ExecuteBody(int functionIndex, FuncType funcType, FunctionBody body, IReadOnlyList<WasmValue> args)
    {
        var instructions = WasmExecution.DecodeFunctionBody(body);
        var locals = new WasmValue[funcType.Params.Count + body.Locals.Count];
        for (var i = 0; i < funcType.Params.Count; i++)
        {
            locals[i] = args[i];
        }

        for (var i = 0; i < body.Locals.Count; i++)
        {
            locals[funcType.Params.Count + i] = WasmValue.DefaultFor(body.Locals[i]);
        }

        var stack = new Stack<WasmValue>();
        for (var ip = 0; ip < instructions.Count; ip++)
        {
            var instruction = instructions[ip];
            switch (instruction.Info.Name)
            {
                case "nop":
                    break;
                case "unreachable":
                    throw new TrapError("unreachable");
                case "drop":
                    Pop(stack);
                    break;
                case "local.get":
                    stack.Push(locals[EnsureIndex(instruction.Index, locals.Length, "local")]);
                    break;
                case "local.set":
                    locals[EnsureIndex(instruction.Index, locals.Length, "local")] = Pop(stack);
                    break;
                case "local.tee":
                {
                    var value = Pop(stack);
                    locals[EnsureIndex(instruction.Index, locals.Length, "local")] = value;
                    stack.Push(value);
                    break;
                }
                case "global.get":
                    stack.Push(_globals[EnsureIndex(instruction.Index, _globals.Length, "global")]);
                    break;
                case "global.set":
                {
                    var globalIndex = EnsureIndex(instruction.Index, _globals.Length, "global");
                    if (!_globalTypes[globalIndex].Mutable)
                    {
                        throw new TrapError($"Global {globalIndex} is immutable");
                    }

                    _globals[globalIndex] = Pop(stack);
                    break;
                }
                case "i32.const":
                case "i64.const":
                case "f32.const":
                case "f64.const":
                    stack.Push(instruction.Constant!.Value);
                    break;
                case "i32.add":
                {
                    var right = Pop(stack).AsI32();
                    var left = Pop(stack).AsI32();
                    stack.Push(WasmValue.I32(unchecked(left + right)));
                    break;
                }
                case "i32.sub":
                {
                    var right = Pop(stack).AsI32();
                    var left = Pop(stack).AsI32();
                    stack.Push(WasmValue.I32(unchecked(left - right)));
                    break;
                }
                case "i32.mul":
                {
                    var right = Pop(stack).AsI32();
                    var left = Pop(stack).AsI32();
                    stack.Push(WasmValue.I32(unchecked(left * right)));
                    break;
                }
                case "i64.add":
                {
                    var right = Pop(stack).AsI64();
                    var left = Pop(stack).AsI64();
                    stack.Push(WasmValue.I64(unchecked(left + right)));
                    break;
                }
                case "i64.sub":
                {
                    var right = Pop(stack).AsI64();
                    var left = Pop(stack).AsI64();
                    stack.Push(WasmValue.I64(unchecked(left - right)));
                    break;
                }
                case "i64.mul":
                {
                    var right = Pop(stack).AsI64();
                    var left = Pop(stack).AsI64();
                    stack.Push(WasmValue.I64(unchecked(left * right)));
                    break;
                }
                case "call":
                {
                    var target = EnsureIndex(instruction.Index, _funcTypes.Count, "function");
                    var targetType = _funcTypes[target];
                    var callArgs = new WasmValue[targetType.Params.Count];
                    for (var i = targetType.Params.Count - 1; i >= 0; i--)
                    {
                        callArgs[i] = Pop(stack);
                    }

                    var results = CallFunction(target, callArgs);
                    foreach (var result in results)
                    {
                        stack.Push(result);
                    }
                    break;
                }
                case "i32.load":
                    stack.Push(WasmValue.I32(RequireMemory().LoadI32(ResolveAddress(stack, instruction))));
                    break;
                case "i64.load":
                    stack.Push(WasmValue.I64(RequireMemory().LoadI64(ResolveAddress(stack, instruction))));
                    break;
                case "f32.load":
                    stack.Push(WasmValue.F32(RequireMemory().LoadF32(ResolveAddress(stack, instruction))));
                    break;
                case "f64.load":
                    stack.Push(WasmValue.F64(RequireMemory().LoadF64(ResolveAddress(stack, instruction))));
                    break;
                case "i32.load8_u":
                    stack.Push(WasmValue.I32(RequireMemory().LoadI32_8u(ResolveAddress(stack, instruction))));
                    break;
                case "i32.store":
                {
                    var value = Pop(stack).AsI32();
                    RequireMemory().StoreI32(ResolveAddress(stack, instruction), value);
                    break;
                }
                case "i64.store":
                {
                    var value = Pop(stack).AsI64();
                    RequireMemory().StoreI64(ResolveAddress(stack, instruction), value);
                    break;
                }
                case "f32.store":
                {
                    var value = Pop(stack).AsF32();
                    RequireMemory().StoreF32(ResolveAddress(stack, instruction), value);
                    break;
                }
                case "f64.store":
                {
                    var value = Pop(stack).AsF64();
                    RequireMemory().StoreF64(ResolveAddress(stack, instruction), value);
                    break;
                }
                case "i32.store8":
                {
                    var value = Pop(stack).AsI32();
                    RequireMemory().StoreI32_8(ResolveAddress(stack, instruction), value);
                    break;
                }
                case "i32.store16":
                {
                    var value = Pop(stack).AsI32();
                    RequireMemory().StoreI32_16(ResolveAddress(stack, instruction), value);
                    break;
                }
                case "i64.store8":
                {
                    var value = Pop(stack).AsI64();
                    RequireMemory().StoreI64_8(ResolveAddress(stack, instruction), value);
                    break;
                }
                case "i64.store16":
                {
                    var value = Pop(stack).AsI64();
                    RequireMemory().StoreI64_16(ResolveAddress(stack, instruction), value);
                    break;
                }
                case "i64.store32":
                {
                    var value = Pop(stack).AsI64();
                    RequireMemory().StoreI64_32(ResolveAddress(stack, instruction), value);
                    break;
                }
                case "memory.size":
                    stack.Push(WasmValue.I32(RequireMemory().Size()));
                    break;
                case "memory.grow":
                {
                    var deltaPages = Pop(stack).AsI32();
                    stack.Push(WasmValue.I32(RequireMemory().Grow(deltaPages)));
                    break;
                }
                case "return":
                case "end":
                    return CollectResults(stack, funcType, functionIndex);
                default:
                    throw new TrapError($"Instruction '{instruction.Info.Name}' is not implemented in the C# wasm execution engine yet");
            }
        }

        throw new TrapError($"Function {functionIndex} terminated without end");
    }

    private IReadOnlyList<WasmValue> CollectResults(Stack<WasmValue> stack, FuncType funcType, int functionIndex)
    {
        var results = new WasmValue[funcType.Results.Count];
        for (var i = results.Length - 1; i >= 0; i--)
        {
            results[i] = Pop(stack);
            if (results[i].Type != funcType.Results[i])
            {
                throw new TrapError($"Function {functionIndex} result {i} expects {funcType.Results[i]}, got {results[i].Type}");
            }
        }

        return results;
    }

    private static WasmValue Pop(Stack<WasmValue> stack)
    {
        if (stack.Count == 0)
        {
            throw new TrapError("Operand stack underflow");
        }

        return stack.Pop();
    }

    private static int EnsureIndex(int? index, int length, string kind)
    {
        if (!index.HasValue || index.Value < 0 || index.Value >= length)
        {
            throw new TrapError($"Invalid {kind} index {index}");
        }

        return index.Value;
    }

    private int ResolveAddress(Stack<WasmValue> stack, DecodedInstruction instruction)
    {
        var baseAddress = Pop(stack).AsI32();
        return checked(baseAddress + (instruction.MemoryOffset ?? 0));
    }

    private LinearMemory RequireMemory() => _memory ?? throw new TrapError("Instruction requires linear memory, but no memory is configured");

    private void EnsureFunctionIndex(int functionIndex)
    {
        if (functionIndex < 0 || functionIndex >= _funcTypes.Count)
        {
            throw new TrapError($"Undefined function index {functionIndex}");
        }
    }
}
