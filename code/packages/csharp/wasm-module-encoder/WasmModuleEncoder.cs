using System.Text;
using CodingAdventures.WasmLeb128;
using CodingAdventures.WasmTypes;
using WasmValueType = CodingAdventures.WasmTypes.ValueType;

namespace CodingAdventures.WasmModuleEncoder;

public static class WasmModuleEncoderVersion
{
    public const string VERSION = "0.1.0";
}

public sealed class WasmEncodeError : Exception
{
    public WasmEncodeError(string message) : base(message)
    {
    }
}

public static class WasmModuleEncoder
{
    public static readonly byte[] WASM_MAGIC = [0x00, 0x61, 0x73, 0x6D];
    public static readonly byte[] WASM_VERSION = [0x01, 0x00, 0x00, 0x00];

    public static byte[] EncodeModule(WasmModule module)
    {
        ArgumentNullException.ThrowIfNull(module);

        var result = new List<byte>(WASM_MAGIC.Length + WASM_VERSION.Length);
        result.AddRange(WASM_MAGIC);
        result.AddRange(WASM_VERSION);

        foreach (var custom in module.Customs)
        {
            result.AddRange(Section(0, EncodeCustom(custom)));
        }

        AddVectorSection(result, 1, module.Types, EncodeFuncType);
        AddVectorSection(result, 2, module.Imports, EncodeImport);
        AddVectorSection(result, 3, module.Functions, U32);
        AddVectorSection(result, 4, module.Tables, EncodeTableType);
        AddVectorSection(result, 5, module.Memories, EncodeMemoryType);
        AddVectorSection(result, 6, module.Globals, EncodeGlobal);
        AddVectorSection(result, 7, module.Exports, EncodeExport);

        if (module.Start is int start)
        {
            result.AddRange(Section(8, U32(start)));
        }

        AddVectorSection(result, 9, module.Elements, EncodeElement);
        AddVectorSection(result, 10, module.Code, EncodeFunctionBody);
        AddVectorSection(result, 11, module.Data, EncodeDataSegment);
        return result.ToArray();
    }

    private static void AddVectorSection<T>(
        List<byte> destination,
        byte sectionId,
        IReadOnlyCollection<T> values,
        Func<T, byte[]> encoder)
    {
        if (values.Count > 0)
        {
            destination.AddRange(Section(sectionId, Vector(values, encoder)));
        }
    }

    private static byte[] Section(byte sectionId, byte[] payload)
    {
        var result = new List<byte>(payload.Length + 6) { sectionId };
        result.AddRange(U32(payload.Length));
        result.AddRange(payload);
        return result.ToArray();
    }

    private static byte[] U32(int value) => WasmLeb128.WasmLeb128.EncodeUnsigned(value);

    private static byte[] Name(string text)
    {
        var data = Encoding.UTF8.GetBytes(text);
        var result = new List<byte>(data.Length + 5);
        result.AddRange(U32(data.Length));
        result.AddRange(data);
        return result.ToArray();
    }

    private static byte[] Vector<T>(IEnumerable<T> values, Func<T, byte[]> encoder)
    {
        var materialized = values as IReadOnlyCollection<T> ?? values.ToArray();
        var result = new List<byte>();
        result.AddRange(U32(materialized.Count));
        foreach (var value in materialized)
        {
            result.AddRange(encoder(value));
        }

        return result.ToArray();
    }

    private static byte[] ValueTypes(IEnumerable<WasmValueType> valueTypes)
    {
        var materialized = valueTypes as IReadOnlyCollection<WasmValueType> ?? valueTypes.ToArray();
        var result = new List<byte>();
        result.AddRange(U32(materialized.Count));
        result.AddRange(materialized.Select(valueType => (byte)valueType));
        return result.ToArray();
    }

    private static byte[] EncodeFuncType(FuncType funcType)
    {
        var result = new List<byte> { 0x60 };
        result.AddRange(ValueTypes(funcType.Params));
        result.AddRange(ValueTypes(funcType.Results));
        return result.ToArray();
    }

    private static byte[] EncodeLimits(Limits limits)
    {
        var result = new List<byte>();
        if (limits.Max is int maximum)
        {
            result.Add(0x01);
            result.AddRange(U32(limits.Min));
            result.AddRange(U32(maximum));
        }
        else
        {
            result.Add(0x00);
            result.AddRange(U32(limits.Min));
        }

        return result.ToArray();
    }

    private static byte[] EncodeMemoryType(MemoryType memoryType) => EncodeLimits(memoryType.Limits);

    private static byte[] EncodeTableType(TableType tableType)
    {
        var result = new List<byte> { tableType.ElementType };
        result.AddRange(EncodeLimits(tableType.Limits));
        return result.ToArray();
    }

    private static byte[] EncodeGlobalType(GlobalType globalType) =>
        [(byte)globalType.ValueType, globalType.Mutable ? (byte)0x01 : (byte)0x00];

    private static byte[] EncodeImport(Import importValue)
    {
        var result = new List<byte>();
        result.AddRange(Name(importValue.ModuleName));
        result.AddRange(Name(importValue.Name));
        result.Add((byte)importValue.Kind);

        switch (importValue.Kind, importValue.Descriptor)
        {
            case (ExternalKind.FUNCTION, FunctionImportDescriptor functionDescriptor):
                result.AddRange(U32(functionDescriptor.TypeIndex));
                break;
            case (ExternalKind.FUNCTION, _):
                throw new WasmEncodeError("function imports require a FunctionImportDescriptor");
            case (ExternalKind.TABLE, TableImportDescriptor tableDescriptor):
                result.AddRange(EncodeTableType(tableDescriptor.TableType));
                break;
            case (ExternalKind.TABLE, _):
                throw new WasmEncodeError("table imports require a TableImportDescriptor");
            case (ExternalKind.MEMORY, MemoryImportDescriptor memoryDescriptor):
                result.AddRange(EncodeMemoryType(memoryDescriptor.MemoryType));
                break;
            case (ExternalKind.MEMORY, _):
                throw new WasmEncodeError("memory imports require a MemoryImportDescriptor");
            case (ExternalKind.GLOBAL, GlobalImportDescriptor globalDescriptor):
                result.AddRange(EncodeGlobalType(globalDescriptor.GlobalType));
                break;
            case (ExternalKind.GLOBAL, _):
                throw new WasmEncodeError("global imports require a GlobalImportDescriptor");
            default:
                throw new WasmEncodeError($"unsupported import kind: {(byte)importValue.Kind}");
        }

        return result.ToArray();
    }

    private static byte[] EncodeExport(Export exportValue)
    {
        var result = new List<byte>();
        result.AddRange(Name(exportValue.Name));
        result.Add((byte)exportValue.Kind);
        result.AddRange(U32(exportValue.Index));
        return result.ToArray();
    }

    private static byte[] EncodeGlobal(Global globalValue)
    {
        var result = new List<byte>();
        result.AddRange(EncodeGlobalType(globalValue.GlobalType));
        result.AddRange(globalValue.InitExpr);
        return result.ToArray();
    }

    private static byte[] EncodeElement(Element element)
    {
        var result = new List<byte>();
        result.AddRange(U32(element.TableIndex));
        result.AddRange(element.OffsetExpr);
        result.AddRange(U32(element.FunctionIndices.Count));
        foreach (var functionIndex in element.FunctionIndices)
        {
            result.AddRange(U32(functionIndex));
        }

        return result.ToArray();
    }

    private static byte[] EncodeDataSegment(DataSegment segment)
    {
        var result = new List<byte>();
        result.AddRange(U32(segment.MemoryIndex));
        result.AddRange(segment.OffsetExpr);
        result.AddRange(U32(segment.Data.Length));
        result.AddRange(segment.Data);
        return result.ToArray();
    }

    private static byte[] EncodeFunctionBody(FunctionBody body)
    {
        var localGroups = GroupLocals(body.Locals);
        var payload = new List<byte>();
        payload.AddRange(U32(localGroups.Count));
        foreach (var (count, valueType) in localGroups)
        {
            payload.AddRange(U32(count));
            payload.Add((byte)valueType);
        }

        payload.AddRange(body.Code);
        var result = new List<byte>();
        result.AddRange(U32(payload.Count));
        result.AddRange(payload);
        return result.ToArray();
    }

    private static List<(int Count, WasmValueType ValueType)> GroupLocals(IReadOnlyList<WasmValueType> locals)
    {
        var groups = new List<(int, WasmValueType)>();
        if (locals.Count == 0)
        {
            return groups;
        }

        var currentType = locals[0];
        var count = 1;
        for (var index = 1; index < locals.Count; index++)
        {
            if (locals[index] == currentType)
            {
                count++;
            }
            else
            {
                groups.Add((count, currentType));
                currentType = locals[index];
                count = 1;
            }
        }

        groups.Add((count, currentType));
        return groups;
    }

    private static byte[] EncodeCustom(CustomSection custom)
    {
        var result = new List<byte>();
        result.AddRange(Name(custom.Name));
        result.AddRange(custom.Data);
        return result.ToArray();
    }
}
