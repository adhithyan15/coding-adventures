using System.Collections.Generic;
using System.Collections.ObjectModel;

namespace CodingAdventures.WasmTypes;

public static class WasmTypesVersion
{
    public const string VERSION = "0.1.0";
}

public enum ValueType : byte
{
    I32 = 0x7F,
    I64 = 0x7E,
    F32 = 0x7D,
    F64 = 0x7C,
}

public static class BlockType
{
    public const byte EMPTY = 0x40;
}

public enum ExternalKind : byte
{
    FUNCTION = 0x00,
    TABLE = 0x01,
    MEMORY = 0x02,
    GLOBAL = 0x03,
}

public static class ReferenceType
{
    public const byte FUNCREF = 0x70;
}

public sealed class FuncType
{
    public FuncType(IEnumerable<ValueType> parameters, IEnumerable<ValueType> results)
    {
        Params = Array.AsReadOnly(parameters.ToArray());
        Results = Array.AsReadOnly(results.ToArray());
    }

    public ReadOnlyCollection<ValueType> Params { get; }

    public ReadOnlyCollection<ValueType> Results { get; }
}

public static class WasmTypeFactory
{
    public static FuncType MakeFuncType(IEnumerable<ValueType> parameters, IEnumerable<ValueType> results) =>
        new(parameters, results);
}

public readonly record struct Limits(int Min, int? Max);

public readonly record struct MemoryType(Limits Limits);

public readonly record struct TableType(byte ElementType, Limits Limits);

public readonly record struct GlobalType(ValueType ValueType, bool Mutable);

public abstract record ImportDescriptor;

public sealed record FunctionImportDescriptor(int TypeIndex) : ImportDescriptor;

public sealed record TableImportDescriptor(TableType TableType) : ImportDescriptor;

public sealed record MemoryImportDescriptor(MemoryType MemoryType) : ImportDescriptor;

public sealed record GlobalImportDescriptor(GlobalType GlobalType) : ImportDescriptor;

public sealed record Import(string ModuleName, string Name, ExternalKind Kind, ImportDescriptor Descriptor);

public sealed record Export(string Name, ExternalKind Kind, int Index);

public sealed class Global
{
    public Global(GlobalType globalType, byte[] initExpr)
    {
        GlobalType = globalType;
        InitExpr = initExpr.ToArray();
    }

    public GlobalType GlobalType { get; }

    public byte[] InitExpr { get; }
}

public sealed class Element
{
    public Element(int tableIndex, byte[] offsetExpr, IEnumerable<int> functionIndices)
    {
        TableIndex = tableIndex;
        OffsetExpr = offsetExpr.ToArray();
        FunctionIndices = Array.AsReadOnly(functionIndices.ToArray());
    }

    public int TableIndex { get; }

    public byte[] OffsetExpr { get; }

    public ReadOnlyCollection<int> FunctionIndices { get; }
}

public sealed class DataSegment
{
    public DataSegment(int memoryIndex, byte[] offsetExpr, byte[] data)
    {
        MemoryIndex = memoryIndex;
        OffsetExpr = offsetExpr.ToArray();
        Data = data.ToArray();
    }

    public int MemoryIndex { get; }

    public byte[] OffsetExpr { get; }

    public byte[] Data { get; }
}

public sealed class FunctionBody
{
    public FunctionBody(IEnumerable<ValueType> locals, byte[] code)
    {
        Locals = Array.AsReadOnly(locals.ToArray());
        Code = code.ToArray();
    }

    public ReadOnlyCollection<ValueType> Locals { get; }

    public byte[] Code { get; }
}

public sealed class CustomSection
{
    public CustomSection(string name, byte[] data)
    {
        Name = name;
        Data = data.ToArray();
    }

    public string Name { get; }

    public byte[] Data { get; }
}

public sealed class WasmModule
{
    public List<FuncType> Types { get; } = [];

    public List<Import> Imports { get; } = [];

    public List<int> Functions { get; } = [];

    public List<TableType> Tables { get; } = [];

    public List<MemoryType> Memories { get; } = [];

    public List<Global> Globals { get; } = [];

    public List<Export> Exports { get; } = [];

    public int? Start { get; set; }

    public List<Element> Elements { get; } = [];

    public List<FunctionBody> Code { get; } = [];

    public List<DataSegment> Data { get; } = [];

    public List<CustomSection> Customs { get; } = [];
}
