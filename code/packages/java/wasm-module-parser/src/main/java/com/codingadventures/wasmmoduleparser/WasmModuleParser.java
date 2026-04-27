package com.codingadventures.wasmmoduleparser;

import com.codingadventures.wasmleb128.WasmLeb128;
import com.codingadventures.wasmtypes.WasmModule;
import com.codingadventures.wasmtypes.WasmTypes;
import com.codingadventures.wasmtypes.WasmTypes.CustomSection;
import com.codingadventures.wasmtypes.WasmTypes.DataSegment;
import com.codingadventures.wasmtypes.WasmTypes.Element;
import com.codingadventures.wasmtypes.WasmTypes.Export;
import com.codingadventures.wasmtypes.WasmTypes.ExternalKind;
import com.codingadventures.wasmtypes.WasmTypes.FuncType;
import com.codingadventures.wasmtypes.WasmTypes.FunctionBody;
import com.codingadventures.wasmtypes.WasmTypes.Global;
import com.codingadventures.wasmtypes.WasmTypes.GlobalType;
import com.codingadventures.wasmtypes.WasmTypes.Import;
import com.codingadventures.wasmtypes.WasmTypes.Limits;
import com.codingadventures.wasmtypes.WasmTypes.MemoryType;
import com.codingadventures.wasmtypes.WasmTypes.TableType;
import com.codingadventures.wasmtypes.WasmTypes.ValueType;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;

public final class WasmModuleParser {
    public static final String VERSION = "0.1.0";

    private static final byte[] WASM_MAGIC = new byte[]{0x00, 0x61, 0x73, 0x6D};
    private static final byte[] WASM_VERSION = new byte[]{0x01, 0x00, 0x00, 0x00};

    private static final int SECTION_CUSTOM = 0;
    private static final int SECTION_TYPE = 1;
    private static final int SECTION_IMPORT = 2;
    private static final int SECTION_FUNCTION = 3;
    private static final int SECTION_TABLE = 4;
    private static final int SECTION_MEMORY = 5;
    private static final int SECTION_GLOBAL = 6;
    private static final int SECTION_EXPORT = 7;
    private static final int SECTION_START = 8;
    private static final int SECTION_ELEMENT = 9;
    private static final int SECTION_CODE = 10;
    private static final int SECTION_DATA = 11;

    private static final int FUNC_TYPE_PREFIX = 0x60;
    private static final int END_OPCODE = 0x0B;

    public WasmModule parse(byte[] data) {
        return new BinaryReader(data).parseModule();
    }

    public static final class WasmParseError extends RuntimeException {
        private final int offset;

        public WasmParseError(String message, int offset) {
            super(message);
            this.offset = offset;
        }

        public int offset() {
            return offset;
        }
    }

    private static final class BinaryReader {
        private final byte[] data;
        private int pos;

        private BinaryReader(byte[] data) {
            this.data = Arrays.copyOf(data, data.length);
            this.pos = 0;
        }

        private int readByte() {
            if (pos >= data.length) {
                throw new WasmParseError("Unexpected end of data: expected 1 byte at offset " + pos, pos);
            }
            return Byte.toUnsignedInt(data[pos++]);
        }

        private byte[] readBytes(int count) {
            if (pos + count > data.length) {
                throw new WasmParseError(
                        "Unexpected end of data: expected " + count + " bytes at offset " + pos,
                        pos
                );
            }
            byte[] slice = Arrays.copyOfRange(data, pos, pos + count);
            pos += count;
            return slice;
        }

        private int readU32() {
            int offset = pos;
            try {
                WasmLeb128.UnsignedDecoding decoded = WasmLeb128.decodeUnsigned(data, pos);
                pos += decoded.bytesConsumed();
                return Math.toIntExact(decoded.value());
            } catch (RuntimeException exception) {
                throw new WasmParseError("Invalid LEB128 at offset " + offset + ": " + exception.getMessage(), offset);
            }
        }

        private String readString() {
            int length = readU32();
            return new String(readBytes(length), StandardCharsets.UTF_8);
        }

        private boolean atEnd() {
            return pos >= data.length;
        }

        private int offset() {
            return pos;
        }

        private Limits readLimits() {
            int flagsOffset = pos;
            int flags = readByte();
            int min = readU32();
            Integer max = null;
            if ((flags & 1) != 0) {
                max = readU32();
            } else if (flags != 0) {
                throw new WasmParseError(
                        "Unknown limits flags byte 0x" + Integer.toHexString(flags) + " at offset " + flagsOffset,
                        flagsOffset
                );
            }
            return new Limits(min, max);
        }

        private GlobalType readGlobalType() {
            int typeOffset = pos;
            int valueTypeByte = readByte();
            if (!isValidValueType(valueTypeByte)) {
                throw new WasmParseError(
                        "Unknown value type byte 0x" + Integer.toHexString(valueTypeByte) + " at offset " + typeOffset,
                        typeOffset
                );
            }
            return new GlobalType(ValueType.fromByte(valueTypeByte), readByte() != 0);
        }

        private byte[] readInitExpr() {
            int start = pos;
            while (pos < data.length) {
                int current = Byte.toUnsignedInt(data[pos++]);
                if (current == END_OPCODE) {
                    return Arrays.copyOfRange(data, start, pos);
                }
            }
            throw new WasmParseError(
                    "Init expression at offset " + start + " never terminated with 0x0B (end opcode)",
                    start
            );
        }

        private List<ValueType> readValueTypeVec() {
            int count = readU32();
            List<ValueType> types = new ArrayList<>(count);
            for (int index = 0; index < count; index++) {
                int typeOffset = pos;
                int valueTypeByte = readByte();
                if (!isValidValueType(valueTypeByte)) {
                    throw new WasmParseError(
                            "Unknown value type byte 0x" + Integer.toHexString(valueTypeByte) + " at offset " + typeOffset,
                            typeOffset
                    );
                }
                types.add(ValueType.fromByte(valueTypeByte));
            }
            return types;
        }

        private void parseTypeSection(WasmModule module) {
            int count = readU32();
            for (int index = 0; index < count; index++) {
                int prefixOffset = pos;
                int prefix = readByte();
                if (prefix != FUNC_TYPE_PREFIX) {
                    throw new WasmParseError(
                            "Expected function type prefix 0x60 at offset " + prefixOffset + ", got 0x"
                                    + Integer.toHexString(prefix),
                            prefixOffset
                    );
                }
                FuncType type = WasmTypes.makeFuncType(readValueTypeVec(), readValueTypeVec());
                module.types.add(type);
            }
        }

        private void parseImportSection(WasmModule module) {
            int count = readU32();
            for (int index = 0; index < count; index++) {
                String moduleName = readString();
                String name = readString();
                int kindOffset = pos;
                int kindByte = readByte();
                ExternalKind kind;
                Object typeInfo;

                switch (kindByte) {
                    case 0x00 -> {
                        kind = ExternalKind.FUNCTION;
                        typeInfo = readU32();
                    }
                    case 0x01 -> {
                        kind = ExternalKind.TABLE;
                        int elementTypeOffset = pos;
                        int elementType = readByte();
                        if (elementType != WasmTypes.FUNCREF) {
                            throw new WasmParseError(
                                    "Unknown table element type 0x" + Integer.toHexString(elementType)
                                            + " at offset " + elementTypeOffset,
                                    elementTypeOffset
                            );
                        }
                        typeInfo = new TableType(elementType, readLimits());
                    }
                    case 0x02 -> {
                        kind = ExternalKind.MEMORY;
                        typeInfo = new MemoryType(readLimits());
                    }
                    case 0x03 -> {
                        kind = ExternalKind.GLOBAL;
                        typeInfo = readGlobalType();
                    }
                    default -> throw new WasmParseError(
                            "Unknown import kind 0x" + Integer.toHexString(kindByte) + " at offset " + kindOffset,
                            kindOffset
                    );
                }

                module.imports.add(new Import(moduleName, name, kind, typeInfo));
            }
        }

        private void parseFunctionSection(WasmModule module) {
            int count = readU32();
            for (int index = 0; index < count; index++) {
                module.functions.add(readU32());
            }
        }

        private void parseTableSection(WasmModule module) {
            int count = readU32();
            for (int index = 0; index < count; index++) {
                int elementTypeOffset = pos;
                int elementType = readByte();
                if (elementType != WasmTypes.FUNCREF) {
                    throw new WasmParseError(
                            "Unknown table element type 0x" + Integer.toHexString(elementType)
                                    + " at offset " + elementTypeOffset,
                            elementTypeOffset
                    );
                }
                module.tables.add(new TableType(elementType, readLimits()));
            }
        }

        private void parseMemorySection(WasmModule module) {
            int count = readU32();
            for (int index = 0; index < count; index++) {
                module.memories.add(new MemoryType(readLimits()));
            }
        }

        private void parseGlobalSection(WasmModule module) {
            int count = readU32();
            for (int index = 0; index < count; index++) {
                module.globals.add(new Global(readGlobalType(), readInitExpr()));
            }
        }

        private void parseExportSection(WasmModule module) {
            int count = readU32();
            for (int index = 0; index < count; index++) {
                String name = readString();
                int kindOffset = pos;
                int kindByte = readByte();
                ExternalKind kind;
                try {
                    kind = ExternalKind.fromByte(kindByte);
                } catch (IllegalArgumentException exception) {
                    throw new WasmParseError(
                            "Unknown export kind 0x" + Integer.toHexString(kindByte) + " at offset " + kindOffset,
                            kindOffset
                    );
                }
                module.exports.add(new Export(name, kind, readU32()));
            }
        }

        private void parseStartSection(WasmModule module) {
            module.start = readU32();
        }

        private void parseElementSection(WasmModule module) {
            int count = readU32();
            for (int index = 0; index < count; index++) {
                int tableIndex = readU32();
                byte[] offsetExpr = readInitExpr();
                int functionCount = readU32();
                List<Integer> functionIndices = new ArrayList<>(functionCount);
                for (int functionIndex = 0; functionIndex < functionCount; functionIndex++) {
                    functionIndices.add(readU32());
                }
                module.elements.add(new Element(tableIndex, offsetExpr, functionIndices));
            }
        }

        private void parseCodeSection(WasmModule module) {
            int count = readU32();
            for (int index = 0; index < count; index++) {
                int bodySize = readU32();
                int bodyStart = pos;
                int bodyEnd = bodyStart + bodySize;

                if (bodyEnd > data.length) {
                    throw new WasmParseError(
                            "Code body " + index + " extends beyond end of data (offset " + bodyStart + ", size " + bodySize + ")",
                            bodyStart
                    );
                }

                int localDeclCount = readU32();
                List<ValueType> locals = new ArrayList<>();
                for (int localIndex = 0; localIndex < localDeclCount; localIndex++) {
                    int groupCount = readU32();
                    int typeOffset = pos;
                    int typeByte = readByte();
                    if (!isValidValueType(typeByte)) {
                        throw new WasmParseError(
                                "Unknown local type byte 0x" + Integer.toHexString(typeByte) + " at offset " + typeOffset,
                                typeOffset
                        );
                    }
                    ValueType localType = ValueType.fromByte(typeByte);
                    for (int repeat = 0; repeat < groupCount; repeat++) {
                        locals.add(localType);
                    }
                }

                int codeLength = bodyEnd - pos;
                if (codeLength < 0) {
                    throw new WasmParseError(
                            "Code body " + index + " local declarations exceeded body size at offset " + pos,
                            pos
                    );
                }

                module.code.add(new FunctionBody(locals, readBytes(codeLength)));
            }
        }

        private void parseDataSection(WasmModule module) {
            int count = readU32();
            for (int index = 0; index < count; index++) {
                int memoryIndex = readU32();
                byte[] offsetExpr = readInitExpr();
                module.data.add(new DataSegment(memoryIndex, offsetExpr, readBytes(readU32())));
            }
        }

        private void parseCustomSection(WasmModule module, byte[] payload) {
            BinaryReader subReader = new BinaryReader(payload);
            String name = subReader.readString();
            module.customs.add(new CustomSection(name, subReader.readBytes(payload.length - subReader.offset())));
        }

        private WasmModule parseModule() {
            validateHeader();
            WasmModule module = new WasmModule();
            int lastSectionId = 0;

            while (!atEnd()) {
                int sectionIdOffset = pos;
                int sectionId = readByte();
                int payloadSize = readU32();
                int payloadStart = pos;
                int payloadEnd = payloadStart + payloadSize;

                if (payloadEnd > data.length) {
                    throw new WasmParseError(
                            "Section " + sectionId + " payload extends beyond end of data (offset " + payloadStart
                                    + ", size " + payloadSize + ")",
                            payloadStart
                    );
                }

                if (sectionId != SECTION_CUSTOM) {
                    if (sectionId < lastSectionId) {
                        throw new WasmParseError(
                                "Section " + sectionId + " appears out of order: already saw section " + lastSectionId,
                                sectionIdOffset
                        );
                    }
                    lastSectionId = sectionId;
                }

                byte[] payload = Arrays.copyOfRange(data, payloadStart, payloadEnd);

                switch (sectionId) {
                    case SECTION_TYPE -> parseTypeSection(module);
                    case SECTION_IMPORT -> parseImportSection(module);
                    case SECTION_FUNCTION -> parseFunctionSection(module);
                    case SECTION_TABLE -> parseTableSection(module);
                    case SECTION_MEMORY -> parseMemorySection(module);
                    case SECTION_GLOBAL -> parseGlobalSection(module);
                    case SECTION_EXPORT -> parseExportSection(module);
                    case SECTION_START -> parseStartSection(module);
                    case SECTION_ELEMENT -> parseElementSection(module);
                    case SECTION_CODE -> parseCodeSection(module);
                    case SECTION_DATA -> parseDataSection(module);
                    case SECTION_CUSTOM -> parseCustomSection(module, payload);
                    default -> {
                    }
                }

                pos = payloadEnd;
            }

            return module;
        }

        private void validateHeader() {
            if (data.length < 8) {
                throw new WasmParseError("File too short: " + data.length + " bytes (need at least 8 for the header)", 0);
            }

            for (int index = 0; index < 4; index++) {
                if (data[index] != WASM_MAGIC[index]) {
                    throw new WasmParseError(
                            "Invalid magic bytes at offset " + index,
                            index
                    );
                }
            }
            pos = 4;

            for (int index = 0; index < 4; index++) {
                if (data[4 + index] != WASM_VERSION[index]) {
                    throw new WasmParseError(
                            "Unsupported WASM version at offset " + (4 + index),
                            4 + index
                    );
                }
            }
            pos = 8;
        }

        private boolean isValidValueType(int valueType) {
            return valueType == ValueType.I32.code()
                    || valueType == ValueType.I64.code()
                    || valueType == ValueType.F32.code()
                    || valueType == ValueType.F64.code();
        }
    }
}
