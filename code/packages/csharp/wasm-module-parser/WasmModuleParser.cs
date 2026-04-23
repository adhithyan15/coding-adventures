using System.Text;
using CodingAdventures.WasmLeb128;
using CodingAdventures.WasmTypes;
using WasmValueType = CodingAdventures.WasmTypes.ValueType;

namespace CodingAdventures.WasmModuleParser;

public static class WasmModuleParserVersion
{
    public const string VERSION = "0.1.0";
}

public sealed class WasmParseError : Exception
{
    public WasmParseError(string message, int offset) : base(message)
    {
        Offset = offset;
    }

    public int Offset { get; }
}

public sealed class WasmModuleParser
{
    public WasmModule Parse(byte[] data)
    {
        var reader = new BinaryReader(data);
        return reader.ParseModule();
    }
}

internal sealed class BinaryReader
{
    private static readonly byte[] WasmMagic = [0x00, 0x61, 0x73, 0x6D];
    private static readonly byte[] WasmVersion = [0x01, 0x00, 0x00, 0x00];

    private const byte SectionCustom = 0;
    private const byte SectionType = 1;
    private const byte SectionImport = 2;
    private const byte SectionFunction = 3;
    private const byte SectionTable = 4;
    private const byte SectionMemory = 5;
    private const byte SectionGlobal = 6;
    private const byte SectionExport = 7;
    private const byte SectionStart = 8;
    private const byte SectionElement = 9;
    private const byte SectionCode = 10;
    private const byte SectionData = 11;
    private const byte FuncTypePrefix = 0x60;
    private const byte EndOpcode = 0x0B;

    private readonly byte[] _data;
    private int _pos;

    public BinaryReader(byte[] data)
    {
        _data = data;
    }

    public int Offset => _pos;

    public byte ReadByte()
    {
        if (_pos >= _data.Length)
        {
            throw new WasmParseError($"Unexpected end of data: expected 1 byte at offset {_pos}", _pos);
        }

        return _data[_pos++];
    }

    public byte[] ReadBytes(int count)
    {
        if (_pos + count > _data.Length)
        {
            throw new WasmParseError(
                $"Unexpected end of data: expected {count} bytes at offset {_pos}, but only {_data.Length - _pos} remain",
                _pos);
        }

        var slice = _data[_pos..(_pos + count)];
        _pos += count;
        return slice;
    }

    public uint ReadU32()
    {
        var offset = _pos;
        try
        {
            var (value, consumed) = WasmLeb128.WasmLeb128.DecodeUnsigned(_data, _pos);
            _pos += consumed;
            return value;
        }
        catch (Exception ex)
        {
            throw new WasmParseError($"Invalid LEB128 at offset {offset}: {ex.Message}", offset);
        }
    }

    public string ReadString()
    {
        var length = checked((int)ReadU32());
        return Encoding.UTF8.GetString(ReadBytes(length));
    }

    public bool AtEnd() => _pos >= _data.Length;

    public Limits ReadLimits()
    {
        var flagsOffset = _pos;
        var flags = ReadByte();
        var min = checked((int)ReadU32());
        int? max = null;
        if ((flags & 1) != 0)
        {
            max = checked((int)ReadU32());
        }
        else if (flags != 0)
        {
            throw new WasmParseError($"Unknown limits flags byte 0x{flags:x2} at offset {flagsOffset}", flagsOffset);
        }

        return new Limits(min, max);
    }

    public GlobalType ReadGlobalType()
    {
        var valueTypeOffset = _pos;
        var valueTypeByte = ReadByte();
        if (!IsValidValueType(valueTypeByte))
        {
            throw new WasmParseError($"Unknown value type byte 0x{valueTypeByte:x2} at offset {valueTypeOffset}", valueTypeOffset);
        }

        var mutableByte = ReadByte();
        return new GlobalType((WasmValueType)valueTypeByte, mutableByte != 0);
    }

    public byte[] ReadInitExpr()
    {
        var start = _pos;
        while (_pos < _data.Length)
        {
            var current = _data[_pos++];
            if (current == EndOpcode)
            {
                return _data[start.._pos];
            }
        }

        throw new WasmParseError($"Init expression at offset {start} never terminated with 0x0B (end opcode)", start);
    }

    public List<WasmValueType> ReadValueTypeVec()
    {
        var count = checked((int)ReadU32());
        var result = new List<WasmValueType>(count);
        for (var i = 0; i < count; i++)
        {
            var offset = _pos;
            var current = ReadByte();
            if (!IsValidValueType(current))
            {
                throw new WasmParseError($"Unknown value type byte 0x{current:x2} at offset {offset}", offset);
            }

            result.Add((WasmValueType)current);
        }

        return result;
    }

    public void ParseTypeSection(WasmModule module)
    {
        var count = checked((int)ReadU32());
        for (var i = 0; i < count; i++)
        {
            var prefixOffset = _pos;
            var prefix = ReadByte();
            if (prefix != FuncTypePrefix)
            {
                throw new WasmParseError($"Expected function type prefix 0x60 at offset {prefixOffset}, got 0x{prefix:x2}", prefixOffset);
            }

            module.Types.Add(WasmTypeFactory.MakeFuncType(ReadValueTypeVec(), ReadValueTypeVec()));
        }
    }

    public void ParseImportSection(WasmModule module)
    {
        var count = checked((int)ReadU32());
        for (var i = 0; i < count; i++)
        {
            var moduleName = ReadString();
            var name = ReadString();
            var kindOffset = _pos;
            var kind = ReadByte();

            ImportDescriptor descriptor = kind switch
            {
                (byte)ExternalKind.FUNCTION => new FunctionImportDescriptor(checked((int)ReadU32())),
                (byte)ExternalKind.TABLE => ReadTableImport(kindOffset),
                (byte)ExternalKind.MEMORY => new MemoryImportDescriptor(new MemoryType(ReadLimits())),
                (byte)ExternalKind.GLOBAL => new GlobalImportDescriptor(ReadGlobalType()),
                _ => throw new WasmParseError($"Unknown import kind 0x{kind:x2} at offset {kindOffset}", kindOffset),
            };

            module.Imports.Add(new Import(moduleName, name, (ExternalKind)kind, descriptor));
        }
    }

    private TableImportDescriptor ReadTableImport(int kindOffset)
    {
        var elementTypeOffset = _pos;
        var elementType = ReadByte();
        if (elementType != ReferenceType.FUNCREF)
        {
            throw new WasmParseError($"Unknown table element type 0x{elementType:x2} at offset {elementTypeOffset}", kindOffset);
        }

        return new TableImportDescriptor(new TableType(elementType, ReadLimits()));
    }

    public void ParseFunctionSection(WasmModule module)
    {
        var count = checked((int)ReadU32());
        for (var i = 0; i < count; i++)
        {
            module.Functions.Add(checked((int)ReadU32()));
        }
    }

    public void ParseTableSection(WasmModule module)
    {
        var count = checked((int)ReadU32());
        for (var i = 0; i < count; i++)
        {
            var elementTypeOffset = _pos;
            var elementType = ReadByte();
            if (elementType != ReferenceType.FUNCREF)
            {
                throw new WasmParseError($"Unknown table element type 0x{elementType:x2} at offset {elementTypeOffset}", elementTypeOffset);
            }

            module.Tables.Add(new TableType(elementType, ReadLimits()));
        }
    }

    public void ParseMemorySection(WasmModule module)
    {
        var count = checked((int)ReadU32());
        for (var i = 0; i < count; i++)
        {
            module.Memories.Add(new MemoryType(ReadLimits()));
        }
    }

    public void ParseGlobalSection(WasmModule module)
    {
        var count = checked((int)ReadU32());
        for (var i = 0; i < count; i++)
        {
            module.Globals.Add(new Global(ReadGlobalType(), ReadInitExpr()));
        }
    }

    public void ParseExportSection(WasmModule module)
    {
        var count = checked((int)ReadU32());
        for (var i = 0; i < count; i++)
        {
            var name = ReadString();
            var kindOffset = _pos;
            var kind = ReadByte();
            if (kind != (byte)ExternalKind.FUNCTION
                && kind != (byte)ExternalKind.TABLE
                && kind != (byte)ExternalKind.MEMORY
                && kind != (byte)ExternalKind.GLOBAL)
            {
                throw new WasmParseError($"Unknown export kind 0x{kind:x2} at offset {kindOffset}", kindOffset);
            }

            module.Exports.Add(new Export(name, (ExternalKind)kind, checked((int)ReadU32())));
        }
    }

    public void ParseStartSection(WasmModule module)
    {
        module.Start = checked((int)ReadU32());
    }

    public void ParseElementSection(WasmModule module)
    {
        var count = checked((int)ReadU32());
        for (var i = 0; i < count; i++)
        {
            var tableIndex = checked((int)ReadU32());
            var offsetExpr = ReadInitExpr();
            var functionCount = checked((int)ReadU32());
            var functionIndices = new List<int>(functionCount);
            for (var j = 0; j < functionCount; j++)
            {
                functionIndices.Add(checked((int)ReadU32()));
            }

            module.Elements.Add(new Element(tableIndex, offsetExpr, functionIndices));
        }
    }

    public void ParseCodeSection(WasmModule module)
    {
        var count = checked((int)ReadU32());
        for (var i = 0; i < count; i++)
        {
            var bodySize = checked((int)ReadU32());
            var bodyStart = _pos;
            var bodyEnd = bodyStart + bodySize;
            if (bodyEnd > _data.Length)
            {
                throw new WasmParseError($"Code body {i} extends beyond end of data (offset {bodyStart}, size {bodySize})", bodyStart);
            }

            var localDeclCount = checked((int)ReadU32());
            var locals = new List<WasmValueType>();
            for (var j = 0; j < localDeclCount; j++)
            {
                var groupCount = checked((int)ReadU32());
                var typeOffset = _pos;
                var typeByte = ReadByte();
                if (!IsValidValueType(typeByte))
                {
                    throw new WasmParseError($"Unknown local type byte 0x{typeByte:x2} at offset {typeOffset}", typeOffset);
                }

                for (var k = 0; k < groupCount; k++)
                {
                    locals.Add((WasmValueType)typeByte);
                }
            }

            var codeLength = bodyEnd - _pos;
            if (codeLength < 0)
            {
                throw new WasmParseError($"Code body {i} local declarations exceeded body size at offset {_pos}", _pos);
            }

            module.Code.Add(new FunctionBody(locals, ReadBytes(codeLength)));
        }
    }

    public void ParseDataSection(WasmModule module)
    {
        var count = checked((int)ReadU32());
        for (var i = 0; i < count; i++)
        {
            var memoryIndex = checked((int)ReadU32());
            var offsetExpr = ReadInitExpr();
            var byteCount = checked((int)ReadU32());
            module.Data.Add(new DataSegment(memoryIndex, offsetExpr, ReadBytes(byteCount)));
        }
    }

    public void ParseCustomSection(WasmModule module, byte[] payload)
    {
        var subReader = new BinaryReader(payload);
        var name = subReader.ReadString();
        module.Customs.Add(new CustomSection(name, subReader.ReadBytes(payload.Length - subReader.Offset)));
    }

    public WasmModule ParseModule()
    {
        ValidateHeader();
        var module = new WasmModule();
        byte lastSectionId = 0;

        while (!AtEnd())
        {
            var sectionIdOffset = _pos;
            var sectionId = ReadByte();
            var payloadSize = checked((int)ReadU32());
            var payloadStart = _pos;
            var payloadEnd = payloadStart + payloadSize;

            if (payloadEnd > _data.Length)
            {
                throw new WasmParseError(
                    $"Section {sectionId} payload extends beyond end of data (offset {payloadStart}, size {payloadSize})",
                    payloadStart);
            }

            if (sectionId != SectionCustom)
            {
                if (sectionId < lastSectionId)
                {
                    throw new WasmParseError(
                        $"Section {sectionId} appears out of order: already saw section {lastSectionId}",
                        sectionIdOffset);
                }

                lastSectionId = sectionId;
            }

            var payload = _data[payloadStart..payloadEnd];

            switch (sectionId)
            {
                case SectionType:
                    ParseTypeSection(module);
                    break;
                case SectionImport:
                    ParseImportSection(module);
                    break;
                case SectionFunction:
                    ParseFunctionSection(module);
                    break;
                case SectionTable:
                    ParseTableSection(module);
                    break;
                case SectionMemory:
                    ParseMemorySection(module);
                    break;
                case SectionGlobal:
                    ParseGlobalSection(module);
                    break;
                case SectionExport:
                    ParseExportSection(module);
                    break;
                case SectionStart:
                    ParseStartSection(module);
                    break;
                case SectionElement:
                    ParseElementSection(module);
                    break;
                case SectionCode:
                    ParseCodeSection(module);
                    break;
                case SectionData:
                    ParseDataSection(module);
                    break;
                case SectionCustom:
                    ParseCustomSection(module, payload);
                    break;
            }

            _pos = payloadEnd;
        }

        return module;
    }

    private void ValidateHeader()
    {
        if (_data.Length < 8)
        {
            throw new WasmParseError($"File too short: {_data.Length} bytes (need at least 8 for the header)", 0);
        }

        for (var i = 0; i < 4; i++)
        {
            if (_data[i] != WasmMagic[i])
            {
                throw new WasmParseError(
                    $"Invalid magic bytes at offset {i}: expected 0x{WasmMagic[i]:x2}, got 0x{_data[i]:x2}",
                    i);
            }
        }

        _pos = 4;

        for (var i = 0; i < 4; i++)
        {
            if (_data[4 + i] != WasmVersion[i])
            {
                throw new WasmParseError(
                    $"Unsupported WASM version at offset {4 + i}: expected 0x{WasmVersion[i]:x2}, got 0x{_data[4 + i]:x2}",
                    4 + i);
            }
        }

        _pos = 8;
    }

    private static bool IsValidValueType(byte current) =>
        current == (byte)WasmValueType.I32
        || current == (byte)WasmValueType.I64
        || current == (byte)WasmValueType.F32
        || current == (byte)WasmValueType.F64;
}
