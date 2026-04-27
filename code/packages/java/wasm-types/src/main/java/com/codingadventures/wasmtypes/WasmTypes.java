package com.codingadventures.wasmtypes;

import java.util.Arrays;
import java.util.List;

public final class WasmTypes {
    public static final int BLOCK_TYPE_EMPTY = 0x40;
    public static final int FUNCREF = 0x70;

    private WasmTypes() {}

    public enum ValueType {
        I32(0x7F),
        I64(0x7E),
        F32(0x7D),
        F64(0x7C);

        private final int code;

        ValueType(int code) {
            this.code = code;
        }

        public int code() {
            return code;
        }

        public static ValueType fromByte(int code) {
            for (ValueType value : values()) {
                if (value.code == code) {
                    return value;
                }
            }
            throw new IllegalArgumentException("Unknown value type byte 0x" + Integer.toHexString(code));
        }
    }

    public enum ExternalKind {
        FUNCTION(0x00),
        TABLE(0x01),
        MEMORY(0x02),
        GLOBAL(0x03);

        private final int code;

        ExternalKind(int code) {
            this.code = code;
        }

        public int code() {
            return code;
        }

        public static ExternalKind fromByte(int code) {
            for (ExternalKind kind : values()) {
                if (kind.code == code) {
                    return kind;
                }
            }
            throw new IllegalArgumentException("Unknown external kind byte 0x" + Integer.toHexString(code));
        }
    }

    public record FuncType(List<ValueType> params, List<ValueType> results) {
        public FuncType {
            params = List.copyOf(params);
            results = List.copyOf(results);
        }
    }

    public record Limits(int min, Integer max) {}

    public record MemoryType(Limits limits) {}

    public record TableType(int elementType, Limits limits) {}

    public record GlobalType(ValueType valueType, boolean mutable) {}

    public record Import(String moduleName, String name, ExternalKind kind, Object typeInfo) {}

    public record Export(String name, ExternalKind kind, int index) {}

    public record Global(GlobalType globalType, byte[] initExpr) {
        public Global {
            initExpr = Arrays.copyOf(initExpr, initExpr.length);
        }
    }

    public record Element(int tableIndex, byte[] offsetExpr, List<Integer> functionIndices) {
        public Element {
            offsetExpr = Arrays.copyOf(offsetExpr, offsetExpr.length);
            functionIndices = List.copyOf(functionIndices);
        }
    }

    public record DataSegment(int memoryIndex, byte[] offsetExpr, byte[] data) {
        public DataSegment {
            offsetExpr = Arrays.copyOf(offsetExpr, offsetExpr.length);
            data = Arrays.copyOf(data, data.length);
        }
    }

    public record FunctionBody(List<ValueType> locals, byte[] code) {
        public FunctionBody {
            locals = List.copyOf(locals);
            code = Arrays.copyOf(code, code.length);
        }
    }

    public record CustomSection(String name, byte[] data) {
        public CustomSection {
            data = Arrays.copyOf(data, data.length);
        }
    }

    public static FuncType makeFuncType(List<ValueType> params, List<ValueType> results) {
        return new FuncType(params, results);
    }
}
