namespace CodingAdventures.WasmSimulator;

public static class WasmSimulatorVersion
{
    public const string VERSION = "0.1.0";
}

public static class WasmOp
{
    public const byte END = 0x0B;
    public const byte LOCAL_GET = 0x20;
    public const byte LOCAL_SET = 0x21;
    public const byte I32_CONST = 0x41;
    public const byte I32_ADD = 0x6A;
    public const byte I32_SUB = 0x6B;
}

public sealed record WasmInstruction(byte Opcode, string Mnemonic, int? Operand, int Size);

public sealed record WasmStepTrace(int Pc, WasmInstruction Instruction, IReadOnlyList<int> StackBefore, IReadOnlyList<int> StackAfter, IReadOnlyList<int> LocalsSnapshot, string Description, bool Halted);

public sealed class WasmDecoder
{
    public WasmInstruction Decode(byte[] bytecode, int pc)
    {
        return bytecode[pc] switch
        {
            WasmOp.I32_CONST => new WasmInstruction(bytecode[pc], "i32.const", BitConverter.ToInt32(bytecode, pc + 1), 5),
            WasmOp.I32_ADD => new WasmInstruction(bytecode[pc], "i32.add", null, 1),
            WasmOp.I32_SUB => new WasmInstruction(bytecode[pc], "i32.sub", null, 1),
            WasmOp.LOCAL_GET => new WasmInstruction(bytecode[pc], "local.get", bytecode[pc + 1], 2),
            WasmOp.LOCAL_SET => new WasmInstruction(bytecode[pc], "local.set", bytecode[pc + 1], 2),
            WasmOp.END => new WasmInstruction(bytecode[pc], "end", null, 1),
            _ => throw new InvalidOperationException($"Unknown WASM opcode 0x{bytecode[pc]:X2} at PC={pc}"),
        };
    }
}

public sealed class WasmExecutor
{
    public WasmStepTrace Execute(WasmInstruction instruction, List<int> stack, int[] locals, int pc)
    {
        var before = stack.ToArray();

        switch (instruction.Mnemonic)
        {
            case "i32.const":
                stack.Add(instruction.Operand!.Value);
                break;
            case "i32.add":
            {
                var right = Pop(stack);
                var left = Pop(stack);
                stack.Add(unchecked(left + right));
                break;
            }
            case "i32.sub":
            {
                var right = Pop(stack);
                var left = Pop(stack);
                stack.Add(unchecked(left - right));
                break;
            }
            case "local.get":
                stack.Add(locals[instruction.Operand!.Value]);
                break;
            case "local.set":
                locals[instruction.Operand!.Value] = Pop(stack);
                break;
            case "end":
                return new WasmStepTrace(pc, instruction, before, stack.ToArray(), locals.ToArray(), "halt", true);
            default:
                throw new InvalidOperationException($"Cannot execute {instruction.Mnemonic}");
        }

        return new WasmStepTrace(pc, instruction, before, stack.ToArray(), locals.ToArray(), instruction.Mnemonic, false);
    }

    private static int Pop(List<int> stack)
    {
        if (stack.Count == 0)
        {
            throw new InvalidOperationException("Stack underflow");
        }

        var value = stack[^1];
        stack.RemoveAt(stack.Count - 1);
        return value;
    }
}

public sealed class WasmSimulator
{
    private readonly WasmDecoder _decoder = new();
    private readonly WasmExecutor _executor = new();

    public WasmSimulator(int numLocals)
    {
        Locals = new int[numLocals];
    }

    public List<int> Stack { get; } = [];

    public int[] Locals { get; }

    public int Pc { get; private set; }

    public bool Halted { get; private set; }

    public byte[] Bytecode { get; private set; } = [];

    public void Load(byte[] bytecode)
    {
        Bytecode = bytecode.ToArray();
        Pc = 0;
        Halted = false;
        Stack.Clear();
        Array.Clear(Locals, 0, Locals.Length);
    }

    public WasmStepTrace Step()
    {
        if (Halted)
        {
            throw new InvalidOperationException("WASM simulator has halted");
        }

        var instruction = _decoder.Decode(Bytecode, Pc);
        var trace = _executor.Execute(instruction, Stack, Locals, Pc);
        Pc += instruction.Size;
        Halted = trace.Halted;
        return trace;
    }

    public IReadOnlyList<WasmStepTrace> Run(byte[] program, int maxSteps = 1000)
    {
        Load(program);
        var traces = new List<WasmStepTrace>();
        for (var i = 0; i < maxSteps && !Halted; i++)
        {
            traces.Add(Step());
        }

        return traces;
    }

    public static byte[] EncodeI32Const(int value) => [WasmOp.I32_CONST, .. BitConverter.GetBytes(value)];

    public static byte[] EncodeI32Add() => [WasmOp.I32_ADD];

    public static byte[] EncodeI32Sub() => [WasmOp.I32_SUB];

    public static byte[] EncodeLocalGet(byte index) => [WasmOp.LOCAL_GET, index];

    public static byte[] EncodeLocalSet(byte index) => [WasmOp.LOCAL_SET, index];

    public static byte[] EncodeEnd() => [WasmOp.END];

    public static byte[] AssembleWasm(IEnumerable<byte[]> instructions) => instructions.SelectMany(bytes => bytes).ToArray();
}
