package com.codingadventures.wasmvalidator;

import com.codingadventures.wasmleb128.WasmLeb128;
import com.codingadventures.wasmtypes.WasmModule;
import com.codingadventures.wasmtypes.WasmTypes;
import com.codingadventures.wasmtypes.WasmTypes.ExternalKind;
import com.codingadventures.wasmtypes.WasmTypes.FuncType;
import com.codingadventures.wasmtypes.WasmTypes.GlobalType;
import com.codingadventures.wasmtypes.WasmTypes.Limits;
import com.codingadventures.wasmtypes.WasmTypes.MemoryType;
import com.codingadventures.wasmtypes.WasmTypes.TableType;
import com.codingadventures.wasmtypes.WasmTypes.ValueType;
import java.util.ArrayList;
import java.util.HashSet;
import java.util.List;
import java.util.Objects;
import java.util.Set;

public final class WasmValidator {
    public static final String VERSION = "0.1.0";
    private static final int MAX_MEMORY_PAGES = 65_536;

    private WasmValidator() {}

    public enum ValidationErrorKind {
        INVALID_TYPE_INDEX,
        INVALID_FUNC_INDEX,
        INVALID_TABLE_INDEX,
        INVALID_MEMORY_INDEX,
        MULTIPLE_MEMORIES,
        MULTIPLE_TABLES,
        MEMORY_LIMIT_EXCEEDED,
        MEMORY_LIMIT_ORDER,
        TABLE_LIMIT_ORDER,
        DUPLICATE_EXPORT_NAME,
        EXPORT_INDEX_OUT_OF_RANGE,
        START_FUNCTION_BAD_TYPE,
        INIT_EXPR_INVALID
    }

    public static final class ValidationError extends RuntimeException {
        private final ValidationErrorKind kind;

        public ValidationError(ValidationErrorKind kind, String message) {
            super(message);
            this.kind = kind;
        }

        public ValidationErrorKind kind() {
            return kind;
        }
    }

    public record ValidatedModule(WasmModule module, List<FuncType> funcTypes, List<List<ValueType>> funcLocals) {
        public ValidatedModule {
            Objects.requireNonNull(module, "module");
            funcTypes = List.copyOf(funcTypes);
            funcLocals = funcLocals.stream().map(List::copyOf).toList();
        }
    }

    public record IndexSpaces(
            List<FuncType> funcTypes,
            int numImportedFuncs,
            List<TableType> tableTypes,
            List<MemoryType> memoryTypes,
            List<GlobalType> globalTypes,
            int numImportedGlobals,
            int numTypes
    ) {
        public IndexSpaces {
            funcTypes = List.copyOf(funcTypes);
            tableTypes = List.copyOf(tableTypes);
            memoryTypes = List.copyOf(memoryTypes);
            globalTypes = List.copyOf(globalTypes);
        }
    }

    public static ValidatedModule validate(WasmModule module) {
        IndexSpaces indexSpaces = validateStructure(module);
        List<List<ValueType>> funcLocals = new ArrayList<>(module.code.size());
        for (int index = 0; index < module.code.size(); index++) {
            int funcIndex = indexSpaces.numImportedFuncs + index;
            FuncType type = indexSpaces.funcTypes.get(funcIndex);
            List<ValueType> locals = new ArrayList<>(type.params());
            locals.addAll(module.code.get(index).locals());
            funcLocals.add(List.copyOf(locals));
        }
        return new ValidatedModule(module, indexSpaces.funcTypes, funcLocals);
    }

    public static IndexSpaces validateStructure(WasmModule module) {
        IndexSpaces indexSpaces = buildIndexSpaces(module);

        if (indexSpaces.tableTypes.size() > 1) {
            throw new ValidationError(ValidationErrorKind.MULTIPLE_TABLES, "WASM 1.0 allows at most one table");
        }
        if (indexSpaces.memoryTypes.size() > 1) {
            throw new ValidationError(ValidationErrorKind.MULTIPLE_MEMORIES, "WASM 1.0 allows at most one memory");
        }

        for (MemoryType memoryType : indexSpaces.memoryTypes) {
            validateMemoryLimits(memoryType.limits());
        }
        for (TableType tableType : indexSpaces.tableTypes) {
            validateTableLimits(tableType.limits());
        }

        validateExports(module, indexSpaces);
        validateStartFunction(module, indexSpaces);

        for (WasmTypes.Global global : module.globals) {
            validateConstExpr(global.initExpr(), global.globalType().valueType(), indexSpaces);
        }
        for (WasmTypes.Element element : module.elements) {
            if (element.tableIndex() != 0 || element.tableIndex() >= indexSpaces.tableTypes.size()) {
                throw new ValidationError(ValidationErrorKind.INVALID_TABLE_INDEX, "Invalid element table index");
            }
            validateConstExpr(element.offsetExpr(), ValueType.I32, indexSpaces);
            for (int funcIndex : element.functionIndices()) {
                ensureIndex(funcIndex, indexSpaces.funcTypes.size(), ValidationErrorKind.INVALID_FUNC_INDEX, "Invalid element function index");
            }
        }
        for (WasmTypes.DataSegment segment : module.data) {
            if (segment.memoryIndex() != 0 || segment.memoryIndex() >= indexSpaces.memoryTypes.size()) {
                throw new ValidationError(ValidationErrorKind.INVALID_MEMORY_INDEX, "Invalid data memory index");
            }
            validateConstExpr(segment.offsetExpr(), ValueType.I32, indexSpaces);
        }

        return indexSpaces;
    }

    public static void validateConstExpr(byte[] expr, ValueType expectedType, IndexSpaces indexSpaces) {
        if (expr.length < 2 || Byte.toUnsignedInt(expr[expr.length - 1]) != 0x0B) {
            throw new ValidationError(ValidationErrorKind.INIT_EXPR_INVALID, "Constant expression must end with 'end'");
        }
        int opcode = Byte.toUnsignedInt(expr[0]);
        ValueType actualType;
        switch (opcode) {
            case 0x41 -> actualType = ValueType.I32;
            case 0x42 -> actualType = ValueType.I64;
            case 0x43 -> actualType = ValueType.F32;
            case 0x44 -> actualType = ValueType.F64;
            case 0x23 -> {
                int index = Math.toIntExact(WasmLeb128.decodeUnsigned(expr, 1).value());
                if (index < 0 || index >= indexSpaces.numImportedGlobals) {
                    throw new ValidationError(
                            ValidationErrorKind.INIT_EXPR_INVALID,
                            "Constant expressions may only reference imported globals"
                    );
                }
                actualType = indexSpaces.globalTypes.get(index).valueType();
            }
            default -> throw new ValidationError(
                    ValidationErrorKind.INIT_EXPR_INVALID,
                    "Opcode 0x" + Integer.toHexString(opcode) + " is not allowed in a constant expression"
            );
        }
        if (actualType != expectedType) {
            throw new ValidationError(
                    ValidationErrorKind.INIT_EXPR_INVALID,
                    "Constant expression has type " + actualType + " but expected " + expectedType
            );
        }
    }

    private static IndexSpaces buildIndexSpaces(WasmModule module) {
        if (module.functions.size() != module.code.size()) {
            throw new ValidationError(ValidationErrorKind.INVALID_FUNC_INDEX, "Function and code section sizes differ");
        }

        List<FuncType> funcTypes = new ArrayList<>();
        List<TableType> tableTypes = new ArrayList<>();
        List<MemoryType> memoryTypes = new ArrayList<>();
        List<GlobalType> globalTypes = new ArrayList<>();
        int numImportedFuncs = 0;
        int numImportedGlobals = 0;

        for (WasmTypes.Import entry : module.imports) {
            switch (entry.kind()) {
                case FUNCTION -> {
                    if (!(entry.typeInfo() instanceof Integer typeIndex)) {
                        throw new ValidationError(ValidationErrorKind.INVALID_TYPE_INDEX, "Imported function missing type index");
                    }
                    ensureIndex(typeIndex, module.types.size(), ValidationErrorKind.INVALID_TYPE_INDEX, "Invalid imported function type index");
                    funcTypes.add(module.types.get(typeIndex));
                    numImportedFuncs += 1;
                }
                case TABLE -> tableTypes.add((TableType) entry.typeInfo());
                case MEMORY -> memoryTypes.add((MemoryType) entry.typeInfo());
                case GLOBAL -> {
                    globalTypes.add((GlobalType) entry.typeInfo());
                    numImportedGlobals += 1;
                }
            }
        }

        for (int typeIndex : module.functions) {
            ensureIndex(typeIndex, module.types.size(), ValidationErrorKind.INVALID_TYPE_INDEX, "Invalid function type index");
            funcTypes.add(module.types.get(typeIndex));
        }

        tableTypes.addAll(module.tables);
        memoryTypes.addAll(module.memories);
        for (WasmTypes.Global global : module.globals) {
            globalTypes.add(global.globalType());
        }

        return new IndexSpaces(funcTypes, numImportedFuncs, tableTypes, memoryTypes, globalTypes, numImportedGlobals, module.types.size());
    }

    private static void validateExports(WasmModule module, IndexSpaces indexSpaces) {
        Set<String> names = new HashSet<>();
        for (WasmTypes.Export exportEntry : module.exports) {
            if (!names.add(exportEntry.name())) {
                throw new ValidationError(ValidationErrorKind.DUPLICATE_EXPORT_NAME, "Duplicate export name");
            }
            int upperBound = switch (exportEntry.kind()) {
                case FUNCTION -> indexSpaces.funcTypes.size();
                case TABLE -> indexSpaces.tableTypes.size();
                case MEMORY -> indexSpaces.memoryTypes.size();
                case GLOBAL -> indexSpaces.globalTypes.size();
            };
            if (exportEntry.index() < 0 || exportEntry.index() >= upperBound) {
                throw new ValidationError(ValidationErrorKind.EXPORT_INDEX_OUT_OF_RANGE, "Export index out of range");
            }
        }
    }

    private static void validateStartFunction(WasmModule module, IndexSpaces indexSpaces) {
        if (module.start == null) {
            return;
        }
        ensureIndex(module.start, indexSpaces.funcTypes.size(), ValidationErrorKind.INVALID_FUNC_INDEX, "Invalid start function index");
        FuncType startType = indexSpaces.funcTypes.get(module.start);
        if (!startType.params().isEmpty() || !startType.results().isEmpty()) {
            throw new ValidationError(ValidationErrorKind.START_FUNCTION_BAD_TYPE, "Start function must have type () -> ()");
        }
    }

    private static void validateMemoryLimits(Limits limits) {
        if (limits.max() != null && limits.max() > MAX_MEMORY_PAGES) {
            throw new ValidationError(ValidationErrorKind.MEMORY_LIMIT_EXCEEDED, "Memory limit exceeds WASM 1.0 maximum");
        }
        if (limits.max() != null && limits.min() > limits.max()) {
            throw new ValidationError(ValidationErrorKind.MEMORY_LIMIT_ORDER, "Memory min exceeds max");
        }
    }

    private static void validateTableLimits(Limits limits) {
        if (limits.max() != null && limits.min() > limits.max()) {
            throw new ValidationError(ValidationErrorKind.TABLE_LIMIT_ORDER, "Table min exceeds max");
        }
    }

    private static void ensureIndex(int index, int length, ValidationErrorKind kind, String message) {
        if (index < 0 || index >= length) {
            throw new ValidationError(kind, message);
        }
    }
}
