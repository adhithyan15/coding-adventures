using CodingAdventures.WasmLeb128;
using CodingAdventures.WasmOpcodes;
using CodingAdventures.WasmTypes;
using WasmValueType = CodingAdventures.WasmTypes.ValueType;

namespace CodingAdventures.WasmValidator;

public static class WasmValidatorVersion
{
    public const string VERSION = "0.1.0";
}

public enum ValidationErrorKind
{
    InvalidTypeIndex,
    InvalidFuncIndex,
    InvalidTableIndex,
    InvalidMemoryIndex,
    InvalidGlobalIndex,
    InvalidLocalIndex,
    MultipleMemories,
    MultipleTables,
    MemoryLimitExceeded,
    MemoryLimitOrder,
    TableLimitOrder,
    DuplicateExportName,
    ExportIndexOutOfRange,
    StartFunctionBadType,
    ImmutableGlobalWrite,
    InitExprInvalid,
    TypeMismatch,
    InvalidFunctionShape,
}

public sealed class ValidationError : Exception
{
    public ValidationError(ValidationErrorKind kind, string message) : base(message)
    {
        Kind = kind;
    }

    public ValidationErrorKind Kind { get; }
}

public sealed class ValidatedModule
{
    public required WasmModule Module { get; init; }
    public required IReadOnlyList<FuncType> FuncTypes { get; init; }
    public required IReadOnlyList<IReadOnlyList<WasmValueType>> FuncLocals { get; init; }
}

public sealed class IndexSpaces
{
    public required IReadOnlyList<FuncType> FuncTypes { get; init; }
    public required int NumImportedFuncs { get; init; }
    public required IReadOnlyList<TableType> TableTypes { get; init; }
    public required int NumImportedTables { get; init; }
    public required IReadOnlyList<MemoryType> MemoryTypes { get; init; }
    public required int NumImportedMemories { get; init; }
    public required IReadOnlyList<GlobalType> GlobalTypes { get; init; }
    public required int NumImportedGlobals { get; init; }
    public required int NumTypes { get; init; }
}

public static class WasmValidator
{
    private const int MaxMemoryPages = 65536;

    public static ValidatedModule Validate(WasmModule module)
    {
        var indexSpaces = ValidateStructure(module);
        var funcLocals = new List<IReadOnlyList<WasmValueType>>(module.Code.Count);

        for (var i = 0; i < module.Code.Count; i++)
        {
            var funcIndex = indexSpaces.NumImportedFuncs + i;
            funcLocals.Add(ValidateFunction(funcIndex, indexSpaces.FuncTypes[funcIndex], module.Code[i], indexSpaces, module));
        }

        return new ValidatedModule
        {
            Module = module,
            FuncTypes = indexSpaces.FuncTypes,
            FuncLocals = funcLocals,
        };
    }

    public static IndexSpaces ValidateStructure(WasmModule module)
    {
        var indexSpaces = BuildIndexSpaces(module);

        if (indexSpaces.TableTypes.Count > 1)
        {
            throw new ValidationError(ValidationErrorKind.MultipleTables, $"WASM 1.0 allows at most one table, found {indexSpaces.TableTypes.Count}");
        }

        if (indexSpaces.MemoryTypes.Count > 1)
        {
            throw new ValidationError(ValidationErrorKind.MultipleMemories, $"WASM 1.0 allows at most one memory, found {indexSpaces.MemoryTypes.Count}");
        }

        foreach (var memoryType in indexSpaces.MemoryTypes)
        {
            ValidateMemoryLimits(memoryType.Limits);
        }

        foreach (var tableType in indexSpaces.TableTypes)
        {
            ValidateTableLimits(tableType.Limits);
        }

        ValidateExports(module, indexSpaces);
        ValidateStartFunction(module, indexSpaces);

        foreach (var global in module.Globals)
        {
            ValidateConstExpr(global.InitExpr, global.GlobalType.ValueType, indexSpaces);
        }

        foreach (var element in module.Elements)
        {
            if (element.TableIndex != 0 || element.TableIndex >= indexSpaces.TableTypes.Count)
            {
                throw new ValidationError(ValidationErrorKind.InvalidTableIndex, $"Element segment references table index {element.TableIndex}, but only {indexSpaces.TableTypes.Count} table(s) exist");
            }

            ValidateConstExpr(element.OffsetExpr, WasmValueType.I32, indexSpaces);

            foreach (var funcIndex in element.FunctionIndices)
            {
                EnsureIndex(funcIndex, indexSpaces.FuncTypes.Count, ValidationErrorKind.InvalidFuncIndex, $"Element segment references function index {funcIndex}, but only {indexSpaces.FuncTypes.Count} function(s) exist");
            }
        }

        foreach (var dataSegment in module.Data)
        {
            if (dataSegment.MemoryIndex != 0 || dataSegment.MemoryIndex >= indexSpaces.MemoryTypes.Count)
            {
                throw new ValidationError(ValidationErrorKind.InvalidMemoryIndex, $"Data segment references memory index {dataSegment.MemoryIndex}, but only {indexSpaces.MemoryTypes.Count} memory/memories exist");
            }

            ValidateConstExpr(dataSegment.OffsetExpr, WasmValueType.I32, indexSpaces);
        }

        return indexSpaces;
    }

    public static IReadOnlyList<WasmValueType> ValidateFunction(int funcIndex, FuncType funcType, FunctionBody body, IndexSpaces indexSpaces, WasmModule module)
    {
        var locals = funcType.Params.Concat(body.Locals).ToArray();
        var reader = new CodeReader(body.Code, $"function {funcIndex}");

        while (!reader.Eof)
        {
            var instruction = reader.ReadInstruction();
            switch (instruction.Info.Name)
            {
                case "local.get":
                case "local.set":
                case "local.tee":
                    EnsureIndex(instruction.LocalIndex ?? -1, locals.Length, ValidationErrorKind.InvalidLocalIndex, $"Local index {instruction.LocalIndex ?? -1} is out of range for {locals.Length} local(s)");
                    break;
                case "global.get":
                    EnsureIndex(instruction.GlobalIndex ?? -1, indexSpaces.GlobalTypes.Count, ValidationErrorKind.InvalidGlobalIndex, $"Global index {instruction.GlobalIndex ?? -1} is out of range for {indexSpaces.GlobalTypes.Count} global(s)");
                    break;
                case "global.set":
                {
                    EnsureIndex(instruction.GlobalIndex ?? -1, indexSpaces.GlobalTypes.Count, ValidationErrorKind.InvalidGlobalIndex, $"Global index {instruction.GlobalIndex ?? -1} is out of range for {indexSpaces.GlobalTypes.Count} global(s)");
                    var globalType = indexSpaces.GlobalTypes[instruction.GlobalIndex!.Value];
                    if (!globalType.Mutable)
                    {
                        throw new ValidationError(ValidationErrorKind.ImmutableGlobalWrite, $"global.set references immutable global {instruction.GlobalIndex.Value}");
                    }
                    break;
                }
                case "call":
                    EnsureIndex(instruction.FuncIndex ?? -1, indexSpaces.FuncTypes.Count, ValidationErrorKind.InvalidFuncIndex, $"Function index {instruction.FuncIndex ?? -1} is out of range for {indexSpaces.FuncTypes.Count} function(s)");
                    break;
                case "call_indirect":
                    EnsureIndex(instruction.TypeIndex ?? -1, indexSpaces.NumTypes, ValidationErrorKind.InvalidTypeIndex, $"Type index {instruction.TypeIndex ?? -1} is out of range for {indexSpaces.NumTypes} type(s)");
                    if (indexSpaces.TableTypes.Count == 0)
                    {
                        throw new ValidationError(ValidationErrorKind.InvalidTableIndex, "call_indirect requires a table, but the module declares none");
                    }
                    break;
                case "i32.const":
                case "i64.const":
                case "f32.const":
                case "f64.const":
                case "drop":
                case "select":
                case "end":
                case "return":
                case "nop":
                case "unreachable":
                    break;
                default:
                    if (instruction.Info.Category == "memory" && indexSpaces.MemoryTypes.Count == 0)
                    {
                        throw new ValidationError(ValidationErrorKind.InvalidMemoryIndex, $"Instruction '{instruction.Info.Name}' requires a memory, but the module declares none");
                    }
                    break;
            }
        }

        if (body.Code.Length == 0 || body.Code[^1] != 0x0B)
        {
            throw new ValidationError(ValidationErrorKind.InvalidFunctionShape, $"Function {funcIndex} ended without a final 'end' opcode");
        }

        return locals;
    }

    public static void ValidateConstExpr(byte[] expr, WasmValueType expectedType, IndexSpaces indexSpaces)
    {
        var reader = new CodeReader(expr, "constant expression");
        var stack = new Stack<WasmValueType>();

        while (!reader.Eof)
        {
            var instruction = reader.ReadInstruction();
            switch (instruction.Info.Name)
            {
                case "i32.const":
                    stack.Push(WasmValueType.I32);
                    break;
                case "i64.const":
                    stack.Push(WasmValueType.I64);
                    break;
                case "f32.const":
                    stack.Push(WasmValueType.F32);
                    break;
                case "f64.const":
                    stack.Push(WasmValueType.F64);
                    break;
                case "global.get":
                {
                    var globalIndex = instruction.GlobalIndex ?? -1;
                    if (globalIndex < 0 || globalIndex >= indexSpaces.NumImportedGlobals)
                    {
                        throw new ValidationError(ValidationErrorKind.InitExprInvalid, $"Constant expressions may only reference imported globals, but saw global {globalIndex}");
                    }

                    stack.Push(indexSpaces.GlobalTypes[globalIndex].ValueType);
                    break;
                }
                case "end":
                    if (!reader.Eof)
                    {
                        throw new ValidationError(ValidationErrorKind.InitExprInvalid, "Constant expression terminated before the end of its byte sequence");
                    }

                    if (stack.Count != 1 || stack.Peek() != expectedType)
                    {
                        throw new ValidationError(ValidationErrorKind.InitExprInvalid, $"Constant expression must leave exactly {expectedType} on the stack");
                    }

                    return;
                default:
                    throw new ValidationError(ValidationErrorKind.InitExprInvalid, $"Opcode '{instruction.Info.Name}' is not allowed in a constant expression");
            }
        }

        throw new ValidationError(ValidationErrorKind.InitExprInvalid, "Constant expression did not terminate with 'end'");
    }

    private static IndexSpaces BuildIndexSpaces(WasmModule module)
    {
        if (module.Functions.Count != module.Code.Count)
        {
            throw new ValidationError(ValidationErrorKind.InvalidFuncIndex, $"Function section declares {module.Functions.Count} local function(s), but code section contains {module.Code.Count} body/bodies");
        }

        var funcTypes = new List<FuncType>();
        var tableTypes = new List<TableType>();
        var memoryTypes = new List<MemoryType>();
        var globalTypes = new List<GlobalType>();
        var numImportedFuncs = 0;
        var numImportedTables = 0;
        var numImportedMemories = 0;
        var numImportedGlobals = 0;

        foreach (var entry in module.Imports)
        {
            switch (entry.Kind)
            {
                case ExternalKind.FUNCTION:
                {
                    if (entry.Descriptor is not FunctionImportDescriptor functionImport)
                    {
                        throw new ValidationError(ValidationErrorKind.InvalidTypeIndex, $"Import '{entry.ModuleName}.{entry.Name}' is not carrying a function type index");
                    }

                    EnsureIndex(functionImport.TypeIndex, module.Types.Count, ValidationErrorKind.InvalidTypeIndex, $"Imported function '{entry.ModuleName}.{entry.Name}' references type index {functionImport.TypeIndex}, but only {module.Types.Count} type(s) exist");
                    funcTypes.Add(module.Types[functionImport.TypeIndex]);
                    numImportedFuncs++;
                    break;
                }
                case ExternalKind.TABLE:
                    if (entry.Descriptor is TableImportDescriptor tableImport)
                    {
                        tableTypes.Add(tableImport.TableType);
                        numImportedTables++;
                    }
                    break;
                case ExternalKind.MEMORY:
                    if (entry.Descriptor is MemoryImportDescriptor memoryImport)
                    {
                        memoryTypes.Add(memoryImport.MemoryType);
                        numImportedMemories++;
                    }
                    break;
                case ExternalKind.GLOBAL:
                    if (entry.Descriptor is GlobalImportDescriptor globalImport)
                    {
                        globalTypes.Add(globalImport.GlobalType);
                        numImportedGlobals++;
                    }
                    break;
            }
        }

        foreach (var typeIndex in module.Functions)
        {
            EnsureIndex(typeIndex, module.Types.Count, ValidationErrorKind.InvalidTypeIndex, $"Local function references type index {typeIndex}, but only {module.Types.Count} type(s) exist");
            funcTypes.Add(module.Types[typeIndex]);
        }

        tableTypes.AddRange(module.Tables);
        memoryTypes.AddRange(module.Memories);
        globalTypes.AddRange(module.Globals.Select(global => global.GlobalType));

        return new IndexSpaces
        {
            FuncTypes = funcTypes,
            NumImportedFuncs = numImportedFuncs,
            TableTypes = tableTypes,
            NumImportedTables = numImportedTables,
            MemoryTypes = memoryTypes,
            NumImportedMemories = numImportedMemories,
            GlobalTypes = globalTypes,
            NumImportedGlobals = numImportedGlobals,
            NumTypes = module.Types.Count,
        };
    }

    private static void ValidateExports(WasmModule module, IndexSpaces indexSpaces)
    {
        var seen = new HashSet<string>(StringComparer.Ordinal);
        foreach (var exportEntry in module.Exports)
        {
            if (!seen.Add(exportEntry.Name))
            {
                throw new ValidationError(ValidationErrorKind.DuplicateExportName, $"Duplicate export name '{exportEntry.Name}'");
            }

            var upperBound = exportEntry.Kind switch
            {
                ExternalKind.FUNCTION => indexSpaces.FuncTypes.Count,
                ExternalKind.TABLE => indexSpaces.TableTypes.Count,
                ExternalKind.MEMORY => indexSpaces.MemoryTypes.Count,
                ExternalKind.GLOBAL => indexSpaces.GlobalTypes.Count,
                _ => 0,
            };

            if (exportEntry.Index < 0 || exportEntry.Index >= upperBound)
            {
                throw new ValidationError(ValidationErrorKind.ExportIndexOutOfRange, $"Export '{exportEntry.Name}' references index {exportEntry.Index}, but only {upperBound} definition(s) exist for kind {exportEntry.Kind}");
            }
        }
    }

    private static void ValidateStartFunction(WasmModule module, IndexSpaces indexSpaces)
    {
        if (!module.Start.HasValue)
        {
            return;
        }

        EnsureIndex(module.Start.Value, indexSpaces.FuncTypes.Count, ValidationErrorKind.InvalidFuncIndex, $"Start function index {module.Start.Value} is out of range for {indexSpaces.FuncTypes.Count} function(s)");
        var startType = indexSpaces.FuncTypes[module.Start.Value];
        if (startType.Params.Count != 0 || startType.Results.Count != 0)
        {
            throw new ValidationError(ValidationErrorKind.StartFunctionBadType, "Start function must have type () -> ()");
        }
    }

    private static void ValidateMemoryLimits(Limits limits)
    {
        if (limits.Max.HasValue && limits.Max.Value > MaxMemoryPages)
        {
            throw new ValidationError(ValidationErrorKind.MemoryLimitExceeded, $"Memory maximum {limits.Max.Value} exceeds the WASM 1.0 limit of {MaxMemoryPages} pages");
        }

        if (limits.Max.HasValue && limits.Min > limits.Max.Value)
        {
            throw new ValidationError(ValidationErrorKind.MemoryLimitOrder, $"Memory minimum {limits.Min} exceeds maximum {limits.Max.Value}");
        }
    }

    private static void ValidateTableLimits(Limits limits)
    {
        if (limits.Max.HasValue && limits.Min > limits.Max.Value)
        {
            throw new ValidationError(ValidationErrorKind.TableLimitOrder, $"Table minimum {limits.Min} exceeds maximum {limits.Max.Value}");
        }
    }

    private static void EnsureIndex(int index, int length, ValidationErrorKind kind, string message)
    {
        if (index < 0 || index >= length)
        {
            throw new ValidationError(kind, message);
        }
    }

    private sealed class ValidationInstruction
    {
        public required OpcodeInfo Info { get; init; }
        public int? FuncIndex { get; init; }
        public int? TypeIndex { get; init; }
        public int? LocalIndex { get; init; }
        public int? GlobalIndex { get; init; }
    }

    private sealed class CodeReader
    {
        private readonly byte[] _bytes;
        private readonly string _context;
        private int _offset;

        public CodeReader(byte[] bytes, string context)
        {
            _bytes = bytes;
            _context = context;
        }

        public bool Eof => _offset >= _bytes.Length;

        public ValidationInstruction ReadInstruction()
        {
            var opcode = ReadByte();
            var info = WasmOpcodes.WasmOpcodes.GetOpcode(opcode)
                ?? throw new ValidationError(ValidationErrorKind.TypeMismatch, $"Unknown opcode 0x{opcode:x2} in {_context} at byte {_offset - 1}");

            int? funcIndex = null;
            int? typeIndex = null;
            int? localIndex = null;
            int? globalIndex = null;

            foreach (var immediate in info.Immediates)
            {
                switch (immediate)
                {
                    case "blocktype":
                        ReadBlockType();
                        break;
                    case "labelidx":
                        ReadU32();
                        break;
                    case "vec_labelidx":
                    {
                        var count = ReadU32();
                        for (var i = 0; i < count; i++)
                        {
                            ReadU32();
                        }
                        ReadU32();
                        break;
                    }
                    case "funcidx":
                        funcIndex = ReadU32();
                        break;
                    case "typeidx":
                        typeIndex = ReadU32();
                        break;
                    case "tableidx":
                        ReadU32();
                        break;
                    case "localidx":
                        localIndex = ReadU32();
                        break;
                    case "globalidx":
                        globalIndex = ReadU32();
                        break;
                    case "memarg":
                        ReadU32();
                        ReadU32();
                        break;
                    case "memidx":
                        ReadU32();
                        break;
                    case "i32":
                    case "i64":
                        ReadSigned();
                        break;
                    case "f32":
                        ReadBytes(4);
                        break;
                    case "f64":
                        ReadBytes(8);
                        break;
                    default:
                        throw new ValidationError(ValidationErrorKind.TypeMismatch, $"Unsupported immediate '{immediate}' in {_context}");
                }
            }

            return new ValidationInstruction
            {
                Info = info,
                FuncIndex = funcIndex,
                TypeIndex = typeIndex,
                LocalIndex = localIndex,
                GlobalIndex = globalIndex,
            };
        }

        private void ReadBlockType()
        {
            var value = ReadByte();
            if (value == BlockType.EMPTY || value == (byte)WasmValueType.I32 || value == (byte)WasmValueType.I64 || value == (byte)WasmValueType.F32 || value == (byte)WasmValueType.F64)
            {
                return;
            }

            throw new ValidationError(ValidationErrorKind.TypeMismatch, $"Unsupported blocktype byte 0x{value:x2} in {_context}");
        }

        private byte ReadByte()
        {
            if (_offset >= _bytes.Length)
            {
                throw new ValidationError(ValidationErrorKind.TypeMismatch, $"Unexpected end of {_context} at byte {_offset}");
            }

            return _bytes[_offset++];
        }

        private byte[] ReadBytes(int length)
        {
            if (_offset + length > _bytes.Length)
            {
                throw new ValidationError(ValidationErrorKind.TypeMismatch, $"Unexpected end of {_context} at byte {_offset}");
            }

            var slice = _bytes[_offset..(_offset + length)];
            _offset += length;
            return slice;
        }

        private int ReadU32()
        {
            try
            {
                var (value, consumed) = WasmLeb128.WasmLeb128.DecodeUnsigned(_bytes, _offset);
                _offset += consumed;
                return checked((int)value);
            }
            catch (Exception ex)
            {
                throw new ValidationError(ValidationErrorKind.TypeMismatch, $"Invalid unsigned LEB128 in {_context} at byte {_offset}: {ex.Message}");
            }
        }

        private int ReadSigned()
        {
            try
            {
                var (value, consumed) = WasmLeb128.WasmLeb128.DecodeSigned(_bytes, _offset);
                _offset += consumed;
                return value;
            }
            catch (Exception ex)
            {
                throw new ValidationError(ValidationErrorKind.TypeMismatch, $"Invalid signed LEB128 in {_context} at byte {_offset}: {ex.Message}");
            }
        }
    }
}
