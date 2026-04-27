package com.codingadventures.wasmexecution;

import com.codingadventures.wasmtypes.WasmTypes;
import com.codingadventures.wasmtypes.WasmTypes.FuncType;
import com.codingadventures.wasmtypes.WasmTypes.FunctionBody;
import com.codingadventures.wasmtypes.WasmTypes.GlobalType;
import com.codingadventures.wasmtypes.WasmTypes.ValueType;
import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.util.ArrayDeque;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import java.util.Objects;

public final class WasmExecution {
    public static final String VERSION = "0.1.0";
    public static final int PAGE_SIZE = 65_536;

    private WasmExecution() {}

    public record WasmValue(ValueType type, Object value) {}

    public record ImportedGlobal(GlobalType type, WasmValue value) {}

    public interface HostFunction {
        FuncType type();

        List<WasmValue> call(List<WasmValue> args);
    }

    public interface HostInterface {
        default HostFunction resolveFunction(String moduleName, String name) {
            return null;
        }

        default ImportedGlobal resolveGlobal(String moduleName, String name) {
            return null;
        }

        default LinearMemory resolveMemory(String moduleName, String name) {
            return null;
        }

        default Table resolveTable(String moduleName, String name) {
            return null;
        }
    }

    public static final class TrapError extends RuntimeException {
        public TrapError(String message) {
            super(message);
        }
    }

    public static final class LinearMemory {
        private final Integer maxPages;
        private byte[] data;
        private ByteBuffer view;

        public LinearMemory(int minPages) {
            this(minPages, null);
        }

        public LinearMemory(int minPages, Integer maxPages) {
            this.maxPages = maxPages;
            this.data = new byte[Math.max(minPages, 0) * PAGE_SIZE];
            this.view = ByteBuffer.wrap(this.data).order(ByteOrder.LITTLE_ENDIAN);
        }

        public int size() {
            return data.length / PAGE_SIZE;
        }

        public int grow(int pages) {
            int previousSize = size();
            int nextSize = previousSize + pages;
            if (maxPages != null && nextSize > maxPages) {
                return -1;
            }
            if (nextSize > 65_536) {
                return -1;
            }
            data = Arrays.copyOf(data, nextSize * PAGE_SIZE);
            view = ByteBuffer.wrap(data).order(ByteOrder.LITTLE_ENDIAN);
            return previousSize;
        }

        public int byteLength() {
            return data.length;
        }

        public int loadI32(int address) {
            ensureAddress(address, 4);
            return view.getInt(address);
        }

        public long loadI64(int address) {
            ensureAddress(address, 8);
            return view.getLong(address);
        }

        public float loadF32(int address) {
            ensureAddress(address, 4);
            return view.getFloat(address);
        }

        public double loadF64(int address) {
            ensureAddress(address, 8);
            return view.getDouble(address);
        }

        public int loadI32_8s(int address) {
            ensureAddress(address, 1);
            return data[address];
        }

        public int loadI32_8u(int address) {
            ensureAddress(address, 1);
            return Byte.toUnsignedInt(data[address]);
        }

        public int loadI32_16s(int address) {
            ensureAddress(address, 2);
            return view.getShort(address);
        }

        public int loadI32_16u(int address) {
            ensureAddress(address, 2);
            return Short.toUnsignedInt(view.getShort(address));
        }

        public long loadI64_8s(int address) {
            return loadI32_8s(address);
        }

        public long loadI64_8u(int address) {
            return loadI32_8u(address);
        }

        public long loadI64_16s(int address) {
            return loadI32_16s(address);
        }

        public long loadI64_16u(int address) {
            return loadI32_16u(address);
        }

        public long loadI64_32s(int address) {
            return loadI32(address);
        }

        public long loadI64_32u(int address) {
            return Integer.toUnsignedLong(loadI32(address));
        }

        public void storeI32(int address, int value) {
            ensureAddress(address, 4);
            view.putInt(address, value);
        }

        public void storeI64(int address, long value) {
            ensureAddress(address, 8);
            view.putLong(address, value);
        }

        public void storeF32(int address, float value) {
            ensureAddress(address, 4);
            view.putFloat(address, value);
        }

        public void storeF64(int address, double value) {
            ensureAddress(address, 8);
            view.putDouble(address, value);
        }

        public void storeByte(int address, int value) {
            ensureAddress(address, 1);
            data[address] = (byte) value;
        }

        public void storeI32_8(int address, int value) {
            storeByte(address, value);
        }

        public void storeI32_16(int address, int value) {
            ensureAddress(address, 2);
            view.putShort(address, (short) value);
        }

        public void storeI64_8(int address, long value) {
            storeByte(address, (int) value);
        }

        public void storeI64_16(int address, long value) {
            ensureAddress(address, 2);
            view.putShort(address, (short) value);
        }

        public void storeI64_32(int address, long value) {
            storeI32(address, (int) value);
        }

        public void storeBytes(int address, byte[] values) {
            ensureAddress(address, values.length);
            System.arraycopy(values, 0, data, address, values.length);
        }

        public void writeBytes(int address, byte[] values) {
            storeBytes(address, values);
        }

        private void ensureAddress(int address, int width) {
            if (address < 0 || address + width > data.length) {
                throw new TrapError("memory access out of bounds at address " + address);
            }
        }
    }

    public static final class Table {
        private final Integer maxSize;
        private final List<Integer> entries;

        public Table(int minSize) {
            this(minSize, null);
        }

        public Table(int minSize, Integer maxSize) {
            this.maxSize = maxSize;
            this.entries = new ArrayList<>(minSize);
            for (int index = 0; index < minSize; index++) {
                this.entries.add(null);
            }
        }

        public Integer get(int index) {
            ensureIndex(index);
            return entries.get(index);
        }

        public void set(int index, Integer value) {
            ensureIndex(index);
            entries.set(index, value);
        }

        public int size() {
            return entries.size();
        }

        public Integer maxSize() {
            return maxSize;
        }

        public int grow(int delta) {
            int oldSize = entries.size();
            int newSize = oldSize + delta;
            if (maxSize != null && newSize > maxSize) {
                return -1;
            }
            for (int index = 0; index < delta; index++) {
                entries.add(null);
            }
            return oldSize;
        }

        private void ensureIndex(int index) {
            if (index < 0 || index >= entries.size()) {
                throw new TrapError("table index out of bounds: " + index);
            }
        }
    }

    public static WasmValue i32(int value) {
        return new WasmValue(ValueType.I32, value);
    }

    public static WasmValue i64(long value) {
        return new WasmValue(ValueType.I64, value);
    }

    public static WasmValue f32(float value) {
        return new WasmValue(ValueType.F32, value);
    }

    public static WasmValue f64(double value) {
        return new WasmValue(ValueType.F64, value);
    }

    public static WasmValue defaultValue(ValueType type) {
        return switch (type) {
            case I32 -> i32(0);
            case I64 -> i64(0L);
            case F32 -> f32(0.0f);
            case F64 -> f64(0.0d);
        };
    }

    public static WasmValue coerceValue(Object rawValue, ValueType type) {
        if (rawValue instanceof WasmValue wasmValue) {
            if (wasmValue.type() != type) {
                throw new TrapError("expected " + type + " argument but received " + wasmValue.type());
            }
            return wasmValue;
        }

        if (!(rawValue instanceof Number number)) {
            throw new TrapError("unsupported host value: " + rawValue);
        }

        return switch (type) {
            case I32 -> i32(number.intValue());
            case I64 -> i64(number.longValue());
            case F32 -> f32(number.floatValue());
            case F64 -> f64(number.doubleValue());
        };
    }

    public static Object unwrapValue(WasmValue value) {
        return value.value();
    }

    public static WasmValue evaluateConstExpr(byte[] expression, List<WasmValue> globals) {
        if (expression.length < 2 || Byte.toUnsignedInt(expression[expression.length - 1]) != 0x0B) {
            throw new TrapError("constant expression must end with opcode 0x0B");
        }

        int opcode = Byte.toUnsignedInt(expression[0]);
        int offset = 1;

        return switch (opcode) {
            case 0x41 -> i32(readSignedLeb32(expression, offset).value());
            case 0x42 -> i64(readSignedLeb64(expression, offset).value());
            case 0x43 -> {
                float value = ByteBuffer.wrap(expression, offset, 4).order(ByteOrder.LITTLE_ENDIAN).getFloat();
                yield f32(value);
            }
            case 0x44 -> {
                double value = ByteBuffer.wrap(expression, offset, 8).order(ByteOrder.LITTLE_ENDIAN).getDouble();
                yield f64(value);
            }
            case 0x23 -> {
                int index = Math.toIntExact(readUnsignedLeb(expression, offset).value());
                if (index < 0 || index >= globals.size()) {
                    throw new TrapError("undefined global index " + index);
                }
                yield globals.get(index);
            }
            default -> throw new TrapError("unsupported const expression opcode 0x" + Integer.toHexString(opcode));
        };
    }

    private record UnsignedLeb(long value, int bytesConsumed) {}

    private record SignedLeb32(int value, int bytesConsumed) {}

    private record SignedLeb64(long value, int bytesConsumed) {}

    private record MemArg(int offset, int bytesConsumed) {}

    private record BlockType(int value, int bytesConsumed) {}

    private record BlockBounds(Integer elsePc, int endPc) {}

    private record Label(int stackHeight, int branchArity) {}

    private static final class BranchSignal extends RuntimeException {
        private final int depth;

        private BranchSignal(int depth) {
            this.depth = depth;
        }

        @Override
        public synchronized Throwable fillInStackTrace() {
            return this;
        }
    }

    private static final class ReturnSignal extends RuntimeException {
        @Override
        public synchronized Throwable fillInStackTrace() {
            return this;
        }
    }

    public static final class WasmExecutionEngine {
        private final LinearMemory memory;
        private final List<Table> tables;
        private final List<WasmValue> globals;
        private final List<GlobalType> globalTypes;
        private final List<FuncType> funcTypes;
        private final List<FunctionBody> funcBodies;
        private final List<HostFunction> hostFunctions;

        public WasmExecutionEngine(
                LinearMemory memory,
                List<Table> tables,
                List<WasmValue> globals,
                List<GlobalType> globalTypes,
                List<FuncType> funcTypes,
                List<FunctionBody> funcBodies,
                List<HostFunction> hostFunctions
        ) {
            this.memory = memory;
            this.tables = List.copyOf(tables);
            this.globals = globals;
            this.globalTypes = globalTypes;
            this.funcTypes = List.copyOf(funcTypes);
            this.funcBodies = funcBodies;
            this.hostFunctions = hostFunctions;
        }

        public List<WasmValue> callFunction(int funcIndex, List<WasmValue> args) {
            FuncType funcType = requireFunctionType(funcIndex);
            if (args.size() != funcType.params().size()) {
                throw new TrapError(
                        "function " + funcIndex + " expects " + funcType.params().size() + " arguments, got " + args.size()
                );
            }

            HostFunction hostFunction = funcIndex < hostFunctions.size() ? hostFunctions.get(funcIndex) : null;
            if (hostFunction != null) {
                return hostFunction.call(args);
            }

            FunctionBody body = funcIndex < funcBodies.size() ? funcBodies.get(funcIndex) : null;
            if (body == null) {
                throw new TrapError("no body for function " + funcIndex);
            }

            List<WasmValue> locals = new ArrayList<>(args);
            for (ValueType localType : body.locals()) {
                locals.add(defaultValue(localType));
            }

            ArrayDeque<WasmValue> stack = new ArrayDeque<>();
            List<Label> labels = new ArrayList<>();
            try {
                executeRange(body.code(), 0, body.code().length, stack, locals, labels);
            } catch (ReturnSignal ignored) {
                // Early return preserves the current stack as the return-value source.
            }

            return collectResults(stack, funcType.results().size());
        }

        private void executeRange(
                byte[] code,
                int startPc,
                int endPc,
                ArrayDeque<WasmValue> stack,
                List<WasmValue> locals,
                List<Label> labels
        ) {
            int pc = startPc;
            while (pc < endPc) {
                int opcode = Byte.toUnsignedInt(code[pc++]);
                switch (opcode) {
                    case 0x00 -> throw new TrapError("unreachable instruction executed");
                    case 0x01 -> {
                        // nop
                    }
                    case 0x02 -> {
                        BlockType blockType = readBlockType(code, pc);
                        int bodyStart = pc + blockType.bytesConsumed();
                        BlockBounds bounds = findBlockBounds(code, bodyStart);
                        labels.add(new Label(stack.size(), blockResultArity(blockType.value())));
                        try {
                            executeRange(code, bodyStart, bounds.endPc(), stack, locals, labels);
                        } catch (BranchSignal signal) {
                            if (signal.depth != 0) {
                                throw new BranchSignal(signal.depth - 1);
                            }
                        } finally {
                            labels.remove(labels.size() - 1);
                        }
                        pc = bounds.endPc() + 1;
                    }
                    case 0x03 -> {
                        BlockType blockType = readBlockType(code, pc);
                        int bodyStart = pc + blockType.bytesConsumed();
                        BlockBounds bounds = findBlockBounds(code, bodyStart);
                        while (true) {
                            labels.add(new Label(stack.size(), blockParamArity(blockType.value())));
                            try {
                                executeRange(code, bodyStart, bounds.endPc(), stack, locals, labels);
                                break;
                            } catch (BranchSignal signal) {
                                if (signal.depth != 0) {
                                    throw new BranchSignal(signal.depth - 1);
                                }
                            } finally {
                                labels.remove(labels.size() - 1);
                            }
                        }
                        pc = bounds.endPc() + 1;
                    }
                    case 0x04 -> {
                        BlockType blockType = readBlockType(code, pc);
                        int bodyStart = pc + blockType.bytesConsumed();
                        BlockBounds bounds = findBlockBounds(code, bodyStart);
                        boolean condition = asI32(pop(stack)) != 0;
                        int branchStart = condition ? bodyStart : (bounds.elsePc() == null ? bounds.endPc() : bounds.elsePc() + 1);
                        int branchEnd = condition ? (bounds.elsePc() == null ? bounds.endPc() : bounds.elsePc()) : bounds.endPc();

                        labels.add(new Label(stack.size(), blockResultArity(blockType.value())));
                        try {
                            executeRange(code, branchStart, branchEnd, stack, locals, labels);
                        } catch (BranchSignal signal) {
                            if (signal.depth != 0) {
                                throw new BranchSignal(signal.depth - 1);
                            }
                        } finally {
                            labels.remove(labels.size() - 1);
                        }
                        pc = bounds.endPc() + 1;
                    }
                    case 0x05 -> throw new TrapError("unexpected else");
                    case 0x0B -> {
                        return;
                    }
                    case 0x0C -> {
                        UnsignedLeb decoded = readUnsignedLeb(code, pc);
                        pc += decoded.bytesConsumed();
                        branchTo(Math.toIntExact(decoded.value()), stack, labels);
                    }
                    case 0x0D -> {
                        UnsignedLeb decoded = readUnsignedLeb(code, pc);
                        pc += decoded.bytesConsumed();
                        if (asI32(pop(stack)) != 0) {
                            branchTo(Math.toIntExact(decoded.value()), stack, labels);
                        }
                    }
                    case 0x0F -> throw new ReturnSignal();
                    case 0x10 -> {
                        UnsignedLeb decoded = readUnsignedLeb(code, pc);
                        pc += decoded.bytesConsumed();
                        pushAll(stack, callDirect(Math.toIntExact(decoded.value()), stack));
                    }
                    case 0x11 -> {
                        UnsignedLeb typeIndex = readUnsignedLeb(code, pc);
                        UnsignedLeb tableIndex = readUnsignedLeb(code, pc + typeIndex.bytesConsumed());
                        pc += typeIndex.bytesConsumed() + tableIndex.bytesConsumed();
                        pushAll(stack, callIndirect(Math.toIntExact(typeIndex.value()), Math.toIntExact(tableIndex.value()), stack));
                    }
                    case 0x1A -> pop(stack);
                    case 0x1B -> {
                        int condition = asI32(pop(stack));
                        WasmValue second = pop(stack);
                        WasmValue first = pop(stack);
                        stack.push(condition != 0 ? first : second);
                    }
                    case 0x20 -> {
                        UnsignedLeb decoded = readUnsignedLeb(code, pc);
                        pc += decoded.bytesConsumed();
                        int index = Math.toIntExact(decoded.value());
                        ensureIndex(index, locals.size(), "local");
                        stack.push(locals.get(index));
                    }
                    case 0x21 -> {
                        UnsignedLeb decoded = readUnsignedLeb(code, pc);
                        pc += decoded.bytesConsumed();
                        int index = Math.toIntExact(decoded.value());
                        ensureIndex(index, locals.size(), "local");
                        locals.set(index, pop(stack));
                    }
                    case 0x22 -> {
                        UnsignedLeb decoded = readUnsignedLeb(code, pc);
                        pc += decoded.bytesConsumed();
                        int index = Math.toIntExact(decoded.value());
                        ensureIndex(index, locals.size(), "local");
                        WasmValue value = pop(stack);
                        locals.set(index, value);
                        stack.push(value);
                    }
                    case 0x23 -> {
                        UnsignedLeb decoded = readUnsignedLeb(code, pc);
                        pc += decoded.bytesConsumed();
                        int index = Math.toIntExact(decoded.value());
                        ensureIndex(index, globals.size(), "global");
                        stack.push(globals.get(index));
                    }
                    case 0x24 -> {
                        UnsignedLeb decoded = readUnsignedLeb(code, pc);
                        pc += decoded.bytesConsumed();
                        int index = Math.toIntExact(decoded.value());
                        ensureIndex(index, globals.size(), "global");
                        if (!globalTypes.get(index).mutable()) {
                            throw new TrapError("global " + index + " is immutable");
                        }
                        globals.set(index, pop(stack));
                    }
                    default -> pc = executeNonControlOpcode(opcode, code, pc, stack);
                }
            }
        }

        private int executeNonControlOpcode(int opcode, byte[] code, int pc, ArrayDeque<WasmValue> stack) {
            switch (opcode) {
                case 0x28 -> {
                    MemArg memArg = readMemArg(code, pc);
                    requireMemory();
                    stack.push(i32(memory.loadI32(effectiveAddress(asI32(pop(stack)), memArg.offset()))));
                    return pc + memArg.bytesConsumed();
                }
                case 0x29 -> {
                    MemArg memArg = readMemArg(code, pc);
                    requireMemory();
                    stack.push(i64(memory.loadI64(effectiveAddress(asI32(pop(stack)), memArg.offset()))));
                    return pc + memArg.bytesConsumed();
                }
                case 0x2A -> {
                    MemArg memArg = readMemArg(code, pc);
                    requireMemory();
                    stack.push(f32(memory.loadF32(effectiveAddress(asI32(pop(stack)), memArg.offset()))));
                    return pc + memArg.bytesConsumed();
                }
                case 0x2B -> {
                    MemArg memArg = readMemArg(code, pc);
                    requireMemory();
                    stack.push(f64(memory.loadF64(effectiveAddress(asI32(pop(stack)), memArg.offset()))));
                    return pc + memArg.bytesConsumed();
                }
                case 0x2C -> {
                    MemArg memArg = readMemArg(code, pc);
                    requireMemory();
                    stack.push(i32(memory.loadI32_8s(effectiveAddress(asI32(pop(stack)), memArg.offset()))));
                    return pc + memArg.bytesConsumed();
                }
                case 0x2D -> {
                    MemArg memArg = readMemArg(code, pc);
                    requireMemory();
                    stack.push(i32(memory.loadI32_8u(effectiveAddress(asI32(pop(stack)), memArg.offset()))));
                    return pc + memArg.bytesConsumed();
                }
                case 0x2E -> {
                    MemArg memArg = readMemArg(code, pc);
                    requireMemory();
                    stack.push(i32(memory.loadI32_16s(effectiveAddress(asI32(pop(stack)), memArg.offset()))));
                    return pc + memArg.bytesConsumed();
                }
                case 0x2F -> {
                    MemArg memArg = readMemArg(code, pc);
                    requireMemory();
                    stack.push(i32(memory.loadI32_16u(effectiveAddress(asI32(pop(stack)), memArg.offset()))));
                    return pc + memArg.bytesConsumed();
                }
                case 0x30 -> {
                    MemArg memArg = readMemArg(code, pc);
                    requireMemory();
                    stack.push(i64(memory.loadI64_8s(effectiveAddress(asI32(pop(stack)), memArg.offset()))));
                    return pc + memArg.bytesConsumed();
                }
                case 0x31 -> {
                    MemArg memArg = readMemArg(code, pc);
                    requireMemory();
                    stack.push(i64(memory.loadI64_8u(effectiveAddress(asI32(pop(stack)), memArg.offset()))));
                    return pc + memArg.bytesConsumed();
                }
                case 0x32 -> {
                    MemArg memArg = readMemArg(code, pc);
                    requireMemory();
                    stack.push(i64(memory.loadI64_16s(effectiveAddress(asI32(pop(stack)), memArg.offset()))));
                    return pc + memArg.bytesConsumed();
                }
                case 0x33 -> {
                    MemArg memArg = readMemArg(code, pc);
                    requireMemory();
                    stack.push(i64(memory.loadI64_16u(effectiveAddress(asI32(pop(stack)), memArg.offset()))));
                    return pc + memArg.bytesConsumed();
                }
                case 0x34 -> {
                    MemArg memArg = readMemArg(code, pc);
                    requireMemory();
                    stack.push(i64(memory.loadI64_32s(effectiveAddress(asI32(pop(stack)), memArg.offset()))));
                    return pc + memArg.bytesConsumed();
                }
                case 0x35 -> {
                    MemArg memArg = readMemArg(code, pc);
                    requireMemory();
                    stack.push(i64(memory.loadI64_32u(effectiveAddress(asI32(pop(stack)), memArg.offset()))));
                    return pc + memArg.bytesConsumed();
                }
                case 0x36 -> {
                    MemArg memArg = readMemArg(code, pc);
                    requireMemory();
                    int value = asI32(pop(stack));
                    memory.storeI32(effectiveAddress(asI32(pop(stack)), memArg.offset()), value);
                    return pc + memArg.bytesConsumed();
                }
                case 0x37 -> {
                    MemArg memArg = readMemArg(code, pc);
                    requireMemory();
                    long value = asI64(pop(stack));
                    memory.storeI64(effectiveAddress(asI32(pop(stack)), memArg.offset()), value);
                    return pc + memArg.bytesConsumed();
                }
                case 0x38 -> {
                    MemArg memArg = readMemArg(code, pc);
                    requireMemory();
                    float value = asF32(pop(stack));
                    memory.storeF32(effectiveAddress(asI32(pop(stack)), memArg.offset()), value);
                    return pc + memArg.bytesConsumed();
                }
                case 0x39 -> {
                    MemArg memArg = readMemArg(code, pc);
                    requireMemory();
                    double value = asF64(pop(stack));
                    memory.storeF64(effectiveAddress(asI32(pop(stack)), memArg.offset()), value);
                    return pc + memArg.bytesConsumed();
                }
                default -> {
                    return executeNumericOpcode(opcode, code, pc, stack);
                }
            }
        }

        private int executeNumericOpcode(int opcode, byte[] code, int pc, ArrayDeque<WasmValue> stack) {
            switch (opcode) {
                case 0x3A -> {
                    MemArg memArg = readMemArg(code, pc);
                    requireMemory();
                    int value = asI32(pop(stack));
                    memory.storeI32_8(effectiveAddress(asI32(pop(stack)), memArg.offset()), value);
                    return pc + memArg.bytesConsumed();
                }
                case 0x3B -> {
                    MemArg memArg = readMemArg(code, pc);
                    requireMemory();
                    int value = asI32(pop(stack));
                    memory.storeI32_16(effectiveAddress(asI32(pop(stack)), memArg.offset()), value);
                    return pc + memArg.bytesConsumed();
                }
                case 0x3C -> {
                    MemArg memArg = readMemArg(code, pc);
                    requireMemory();
                    long value = asI64(pop(stack));
                    memory.storeI64_8(effectiveAddress(asI32(pop(stack)), memArg.offset()), value);
                    return pc + memArg.bytesConsumed();
                }
                case 0x3D -> {
                    MemArg memArg = readMemArg(code, pc);
                    requireMemory();
                    long value = asI64(pop(stack));
                    memory.storeI64_16(effectiveAddress(asI32(pop(stack)), memArg.offset()), value);
                    return pc + memArg.bytesConsumed();
                }
                case 0x3E -> {
                    MemArg memArg = readMemArg(code, pc);
                    requireMemory();
                    long value = asI64(pop(stack));
                    memory.storeI64_32(effectiveAddress(asI32(pop(stack)), memArg.offset()), value);
                    return pc + memArg.bytesConsumed();
                }
                case 0x3F -> {
                    return executeMemorySizeGrow(code, pc, stack, true);
                }
                case 0x40 -> {
                    return executeMemorySizeGrow(code, pc, stack, false);
                }
                case 0x41 -> {
                    SignedLeb32 decoded = readSignedLeb32(code, pc);
                    stack.push(i32(decoded.value()));
                    return pc + decoded.bytesConsumed();
                }
                case 0x42 -> {
                    SignedLeb64 decoded = readSignedLeb64(code, pc);
                    stack.push(i64(decoded.value()));
                    return pc + decoded.bytesConsumed();
                }
                case 0x43 -> {
                    ensureRemaining(code, pc, 4);
                    stack.push(f32(ByteBuffer.wrap(code, pc, 4).order(ByteOrder.LITTLE_ENDIAN).getFloat()));
                    return pc + 4;
                }
                case 0x44 -> {
                    ensureRemaining(code, pc, 8);
                    stack.push(f64(ByteBuffer.wrap(code, pc, 8).order(ByteOrder.LITTLE_ENDIAN).getDouble()));
                    return pc + 8;
                }
                case 0x45 -> {
                    stack.push(i32(asI32(pop(stack)) == 0 ? 1 : 0));
                    return pc;
                }
                case 0x46 -> {
                    compareI32(stack, (left, right) -> left == right);
                    return pc;
                }
                case 0x47 -> {
                    compareI32(stack, (left, right) -> left != right);
                    return pc;
                }
                case 0x48 -> {
                    compareI32(stack, (left, right) -> left < right);
                    return pc;
                }
                case 0x49 -> {
                    compareI32(stack, (left, right) -> Integer.compareUnsigned(left, right) < 0);
                    return pc;
                }
                case 0x4A -> {
                    compareI32(stack, (left, right) -> left > right);
                    return pc;
                }
                case 0x4B -> {
                    compareI32(stack, (left, right) -> Integer.compareUnsigned(left, right) > 0);
                    return pc;
                }
                case 0x4C -> {
                    compareI32(stack, (left, right) -> left <= right);
                    return pc;
                }
                case 0x4D -> {
                    compareI32(stack, (left, right) -> Integer.compareUnsigned(left, right) <= 0);
                    return pc;
                }
                case 0x4E -> {
                    compareI32(stack, (left, right) -> left >= right);
                    return pc;
                }
                case 0x4F -> {
                    compareI32(stack, (left, right) -> Integer.compareUnsigned(left, right) >= 0);
                    return pc;
                }
                case 0x67 -> {
                    stack.push(i32(Integer.numberOfLeadingZeros(asI32(pop(stack)))));
                    return pc;
                }
                case 0x68 -> {
                    stack.push(i32(Integer.numberOfTrailingZeros(asI32(pop(stack)))));
                    return pc;
                }
                case 0x69 -> {
                    stack.push(i32(Integer.bitCount(asI32(pop(stack)))));
                    return pc;
                }
                case 0x6A -> {
                    binaryI32(stack, Integer::sum);
                    return pc;
                }
                case 0x6B -> {
                    binaryI32(stack, (left, right) -> left - right);
                    return pc;
                }
                case 0x6C -> {
                    binaryI32(stack, (left, right) -> left * right);
                    return pc;
                }
                case 0x6D -> {
                    int right = asI32(pop(stack));
                    int left = asI32(pop(stack));
                    if (right == 0) {
                        throw new TrapError("integer divide by zero");
                    }
                    if (left == Integer.MIN_VALUE && right == -1) {
                        throw new TrapError("integer overflow");
                    }
                    stack.push(i32(left / right));
                    return pc;
                }
                case 0x6E -> {
                    int right = asI32(pop(stack));
                    int left = asI32(pop(stack));
                    if (right == 0) {
                        throw new TrapError("integer divide by zero");
                    }
                    stack.push(i32(Integer.divideUnsigned(left, right)));
                    return pc;
                }
                case 0x6F -> {
                    int right = asI32(pop(stack));
                    int left = asI32(pop(stack));
                    if (right == 0) {
                        throw new TrapError("integer divide by zero");
                    }
                    stack.push(i32(left % right));
                    return pc;
                }
                case 0x70 -> {
                    int right = asI32(pop(stack));
                    int left = asI32(pop(stack));
                    if (right == 0) {
                        throw new TrapError("integer divide by zero");
                    }
                    stack.push(i32(Integer.remainderUnsigned(left, right)));
                    return pc;
                }
                case 0x71 -> {
                    binaryI32(stack, (left, right) -> left & right);
                    return pc;
                }
                case 0x72 -> {
                    binaryI32(stack, (left, right) -> left | right);
                    return pc;
                }
                case 0x73 -> {
                    binaryI32(stack, (left, right) -> left ^ right);
                    return pc;
                }
                case 0x74 -> {
                    binaryI32(stack, (left, right) -> left << (right & 31));
                    return pc;
                }
                case 0x75 -> {
                    binaryI32(stack, (left, right) -> left >> (right & 31));
                    return pc;
                }
                case 0x76 -> {
                    binaryI32(stack, (left, right) -> left >>> (right & 31));
                    return pc;
                }
                case 0x77 -> {
                    binaryI32(stack, Integer::rotateLeft);
                    return pc;
                }
                case 0x78 -> {
                    binaryI32(stack, Integer::rotateRight);
                    return pc;
                }
                default -> throw new TrapError("unsupported opcode 0x" + Integer.toHexString(opcode));
            }
        }

        private int executeMemorySizeGrow(byte[] code, int pc, ArrayDeque<WasmValue> stack, boolean sizeOnly) {
            requireMemory();
            int bytesConsumed = readZeroByteImmediate(code, pc);
            if (sizeOnly) {
                stack.push(i32(memory.size()));
            } else {
                stack.push(i32(memory.grow(asI32(pop(stack)))));
            }
            return pc + bytesConsumed;
        }

        private List<WasmValue> callDirect(int funcIndex, ArrayDeque<WasmValue> stack) {
            FuncType funcType = requireFunctionType(funcIndex);
            List<WasmValue> args = new ArrayList<>(funcType.params().size());
            for (int index = 0; index < funcType.params().size(); index++) {
                args.add(0, pop(stack));
            }
            return callFunction(funcIndex, args);
        }

        private List<WasmValue> callIndirect(int expectedTypeIndex, int tableIndex, ArrayDeque<WasmValue> stack) {
            ensureIndex(tableIndex, tables.size(), "table");
            int elementIndex = asI32(pop(stack));
            Table table = tables.get(tableIndex);
            Integer funcIndex = table.get(elementIndex);
            if (funcIndex == null) {
                throw new TrapError("uninitialized table element");
            }

            FuncType expected = requireType(expectedTypeIndex);
            FuncType actual = requireFunctionType(funcIndex);
            if (!sameType(expected, actual)) {
                throw new TrapError("indirect call type mismatch");
            }
            return callDirect(funcIndex, stack);
        }

        private FuncType requireFunctionType(int funcIndex) {
            if (funcIndex < 0 || funcIndex >= funcTypes.size()) {
                throw new TrapError("undefined function index " + funcIndex);
            }
            return funcTypes.get(funcIndex);
        }

        private FuncType requireType(int typeIndex) {
            if (typeIndex < 0 || typeIndex >= funcTypes.size()) {
                throw new TrapError("undefined type");
            }
            return funcTypes.get(typeIndex);
        }

        private void requireMemory() {
            if (memory == null) {
                throw new TrapError("no linear memory");
            }
        }

        private void branchTo(int depth, ArrayDeque<WasmValue> stack, List<Label> labels) {
            int labelIndex = labels.size() - 1 - depth;
            if (labelIndex < 0) {
                throw new TrapError("branch target " + depth + " out of range");
            }

            Label target = labels.get(labelIndex);
            List<WasmValue> carried = new ArrayList<>(target.branchArity());
            for (int index = 0; index < target.branchArity(); index++) {
                carried.add(0, pop(stack));
            }
            while (stack.size() > target.stackHeight()) {
                pop(stack);
            }
            pushAll(stack, carried);
            throw new BranchSignal(depth);
        }

        private static void binaryI32(ArrayDeque<WasmValue> stack, IntBinaryOperation operation) {
            int right = asI32(pop(stack));
            int left = asI32(pop(stack));
            stack.push(i32(operation.apply(left, right)));
        }

        private static void compareI32(ArrayDeque<WasmValue> stack, IntComparison comparison) {
            int right = asI32(pop(stack));
            int left = asI32(pop(stack));
            stack.push(i32(comparison.test(left, right) ? 1 : 0));
        }

        private static WasmValue pop(ArrayDeque<WasmValue> stack) {
            if (stack.isEmpty()) {
                throw new TrapError("operand stack underflow");
            }
            return Objects.requireNonNull(stack.pop());
        }

        private static void pushAll(ArrayDeque<WasmValue> stack, List<WasmValue> values) {
            for (WasmValue value : values) {
                stack.push(value);
            }
        }

        private static boolean sameType(FuncType expected, FuncType actual) {
            return expected.params().equals(actual.params()) && expected.results().equals(actual.results());
        }

        private static void ensureIndex(int index, int size, String kind) {
            if (index < 0 || index >= size) {
                throw new TrapError("undefined " + kind + " index " + index);
            }
        }

        private static List<WasmValue> collectResults(ArrayDeque<WasmValue> stack, int resultCount) {
            List<WasmValue> results = new ArrayList<>(resultCount);
            for (int index = 0; index < resultCount; index++) {
                results.add(0, pop(stack));
            }
            return results;
        }
    }

    @FunctionalInterface
    private interface IntBinaryOperation {
        int apply(int left, int right);
    }

    @FunctionalInterface
    private interface IntComparison {
        boolean test(int left, int right);
    }

    private static int asI32(WasmValue value) {
        if (value.type() != ValueType.I32) {
            throw new TrapError("expected i32 but found " + value.type());
        }
        return ((Number) value.value()).intValue();
    }

    private static long asI64(WasmValue value) {
        if (value.type() != ValueType.I64) {
            throw new TrapError("expected i64 but found " + value.type());
        }
        return ((Number) value.value()).longValue();
    }

    private static float asF32(WasmValue value) {
        if (value.type() != ValueType.F32) {
            throw new TrapError("expected f32 but found " + value.type());
        }
        return ((Number) value.value()).floatValue();
    }

    private static double asF64(WasmValue value) {
        if (value.type() != ValueType.F64) {
            throw new TrapError("expected f64 but found " + value.type());
        }
        return ((Number) value.value()).doubleValue();
    }

    private static UnsignedLeb readUnsignedLeb(byte[] code, int offset) {
        long result = 0;
        int shift = 0;
        int bytesConsumed = 0;
        while (offset + bytesConsumed < code.length) {
            int current = Byte.toUnsignedInt(code[offset + bytesConsumed]);
            result |= (long) (current & 0x7F) << shift;
            bytesConsumed++;
            if ((current & 0x80) == 0) {
                return new UnsignedLeb(result, bytesConsumed);
            }
            shift += 7;
        }
        throw new TrapError("unterminated unsigned LEB128 immediate");
    }

    private static SignedLeb32 readSignedLeb32(byte[] code, int offset) {
        int result = 0;
        int shift = 0;
        int bytesConsumed = 0;
        int current;
        do {
            if (offset + bytesConsumed >= code.length) {
                throw new TrapError("unterminated signed LEB128 immediate");
            }
            current = Byte.toUnsignedInt(code[offset + bytesConsumed]);
            result |= (current & 0x7F) << shift;
            shift += 7;
            bytesConsumed++;
        } while ((current & 0x80) != 0);

        if (shift < 32 && (current & 0x40) != 0) {
            result |= -1 << shift;
        }
        return new SignedLeb32(result, bytesConsumed);
    }

    private static SignedLeb64 readSignedLeb64(byte[] code, int offset) {
        long result = 0L;
        int shift = 0;
        int bytesConsumed = 0;
        int current;
        do {
            if (offset + bytesConsumed >= code.length) {
                throw new TrapError("unterminated signed LEB128 immediate");
            }
            current = Byte.toUnsignedInt(code[offset + bytesConsumed]);
            result |= (long) (current & 0x7F) << shift;
            shift += 7;
            bytesConsumed++;
        } while ((current & 0x80) != 0);

        if (shift < 64 && (current & 0x40) != 0) {
            result |= -1L << shift;
        }
        return new SignedLeb64(result, bytesConsumed);
    }

    private static MemArg readMemArg(byte[] code, int offset) {
        UnsignedLeb align = readUnsignedLeb(code, offset);
        UnsignedLeb memOffset = readUnsignedLeb(code, offset + align.bytesConsumed());
        return new MemArg(Math.toIntExact(memOffset.value()), align.bytesConsumed() + memOffset.bytesConsumed());
    }

    private static BlockType readBlockType(byte[] code, int offset) {
        ensureRemaining(code, offset, 1);
        int first = Byte.toUnsignedInt(code[offset]);
        if (first == WasmTypes.BLOCK_TYPE_EMPTY || isValueTypeByte(first)) {
            return new BlockType(first, 1);
        }
        SignedLeb32 decoded = readSignedLeb32(code, offset);
        return new BlockType(decoded.value(), decoded.bytesConsumed());
    }

    private static BlockBounds findBlockBounds(byte[] code, int offset) {
        int depth = 1;
        Integer elsePc = null;
        int pc = offset;
        while (pc < code.length) {
            int opcode = Byte.toUnsignedInt(code[pc++]);
            switch (opcode) {
                case 0x02, 0x03, 0x04 -> {
                    BlockType blockType = readBlockType(code, pc);
                    pc += blockType.bytesConsumed();
                    depth++;
                }
                case 0x05 -> {
                    if (depth == 1 && elsePc == null) {
                        elsePc = pc - 1;
                    }
                }
                case 0x0B -> {
                    depth--;
                    if (depth == 0) {
                        return new BlockBounds(elsePc, pc - 1);
                    }
                }
                default -> pc = skipImmediate(code, pc, opcode);
            }
        }
        throw new TrapError("unterminated structured control block");
    }

    private static int skipImmediate(byte[] code, int offset, int opcode) {
        return switch (opcode) {
            case 0x0C, 0x0D, 0x10, 0x20, 0x21, 0x22, 0x23, 0x24 -> offset + readUnsignedLeb(code, offset).bytesConsumed();
            case 0x11 -> {
                UnsignedLeb typeIndex = readUnsignedLeb(code, offset);
                UnsignedLeb tableIndex = readUnsignedLeb(code, offset + typeIndex.bytesConsumed());
                yield offset + typeIndex.bytesConsumed() + tableIndex.bytesConsumed();
            }
            case 0x28, 0x29, 0x2A, 0x2B, 0x2C, 0x2D, 0x2E, 0x2F,
                    0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37,
                    0x38, 0x39, 0x3A, 0x3B, 0x3C, 0x3D, 0x3E -> offset + readMemArg(code, offset).bytesConsumed();
            case 0x3F, 0x40 -> offset + readZeroByteImmediate(code, offset);
            case 0x41 -> offset + readSignedLeb32(code, offset).bytesConsumed();
            case 0x42 -> offset + readSignedLeb64(code, offset).bytesConsumed();
            case 0x43 -> offset + 4;
            case 0x44 -> offset + 8;
            default -> offset;
        };
    }

    private static int readZeroByteImmediate(byte[] code, int offset) {
        ensureRemaining(code, offset, 1);
        if (Byte.toUnsignedInt(code[offset]) != 0) {
            throw new TrapError("expected zero-byte memory immediate");
        }
        return 1;
    }

    private static int effectiveAddress(int base, int offset) {
        long address = Integer.toUnsignedLong(base) + Integer.toUnsignedLong(offset);
        if (address > Integer.MAX_VALUE) {
            throw new TrapError("memory access out of bounds at address " + address);
        }
        return (int) address;
    }

    private static int blockResultArity(int blockType) {
        if (blockType == WasmTypes.BLOCK_TYPE_EMPTY) {
            return 0;
        }
        if (isValueTypeByte(blockType)) {
            return 1;
        }
        return 0;
    }

    private static int blockParamArity(int blockType) {
        if (blockType == WasmTypes.BLOCK_TYPE_EMPTY || isValueTypeByte(blockType)) {
            return 0;
        }
        return 0;
    }

    private static boolean isValueTypeByte(int byteValue) {
        return byteValue == ValueType.I32.code()
                || byteValue == ValueType.I64.code()
                || byteValue == ValueType.F32.code()
                || byteValue == ValueType.F64.code();
    }

    private static void ensureRemaining(byte[] code, int offset, int length) {
        if (offset < 0 || offset + length > code.length) {
            throw new TrapError("unexpected end of bytecode");
        }
    }
}
